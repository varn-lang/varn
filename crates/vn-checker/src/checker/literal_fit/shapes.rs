use super::super::recorder::Recorder;
use super::super::Checker;
use vn_core::ast::{ArrayEl, ExprId, ExprKind, ObjectProp, PropKey};
use vn_core::TypeKind;
use vn_sem::bind::BindResult;
use vn_sem::types::{ObjectTypeMember, Type};

impl Checker<'_> {
    pub(super) fn shapes_of(&mut self, target: &Type, bind: &BindResult) -> Vec<Type> {
        use vn_sem::bind::TypeContext;
        match self.ty_table.get(target.0) {
            TypeKind::Union(members) => {
                let members: Vec<Type> = self
                    .ty_table
                    .get_list(members)
                    .iter()
                    .map(|m| Type::resolved(*m))
                    .collect();
                members
                    .iter()
                    .flat_map(|m| self.shapes_of(m, bind))
                    .collect()
            }
            TypeKind::Named(name, origin) => {
                let name_s = self.resolve_bind_atom(bind, name);
                let origin_s = origin.map(|o| self.resolve_bind_atom(bind, o));
                let alias = vn_sem::bind::BindView::new(bind, self.resolver)
                    .resolve_type_alias(&name_s, origin_s.as_deref())
                    .filter(|expanded| expanded.0 != target.0);
                match alias {
                    Some(expanded) => self.shapes_of(&expanded, bind),
                    None => vec![*target],
                }
            }
            TypeKind::Primitive(_)
            | TypeKind::Literal(_)
            | TypeKind::This
            | TypeKind::Array(_)
            | TypeKind::Intersection(_)
            | TypeKind::Tuple(_)
            | TypeKind::Generic(..)
            | TypeKind::Fn(_)
            | TypeKind::Record(_)
            | TypeKind::Interface(_)
            | TypeKind::EnumVariant { .. }
            | TypeKind::TypePredicate { .. } => vec![*target],
        }
    }

    pub(super) fn literal_fits_shape(
        &mut self,
        rec: &mut Recorder,
        shape: &Type,
        literal: ExprId,
        bind: &BindResult,
        depth: u8,
    ) -> bool {
        let arena = self.ast_arena;
        let shape_kind = self.ty_table.get(shape.0);
        if let ExprKind::Array { elements } = &arena.expr(literal).kind {
            if !matches!(shape_kind, TypeKind::Array(_) | TypeKind::Generic(..)) {
                return false;
            }
            {
                let item = shape.get_array_element_type(&self.ty_table);
                if item.is_unresolved() && !matches!(shape_kind, TypeKind::Array(_)) {
                    return false;
                }
                return elements.iter().all(|el| match el {
                    ArrayEl::Expr(e) => self.part_fits(rec, &item, *e, bind, depth),
                    ArrayEl::Spread(e) => self.spread_fits(rec, &item, *e, bind),
                    ArrayEl::Hole => true,
                });
            }
        }
        if let ExprKind::Tuple { elements } = &arena.expr(literal).kind {
            let TypeKind::Tuple(list) = shape_kind else {
                return false;
            };
            let slots: Vec<Type> = self
                .ty_table
                .get_list(list)
                .iter()
                .map(|id| Type::resolved(*id))
                .collect();
            return slots.len() == elements.len()
                && elements
                    .iter()
                    .zip(&slots)
                    .all(|(element, slot)| self.part_fits(rec, slot, *element, bind, depth));
        }
        if let (
            ExprKind::Object { properties, .. },
            TypeKind::Record(mid) | TypeKind::Interface(mid),
        ) = (&arena.expr(literal).kind, shape_kind)
        {
            {
                let members = self.ty_table.get_object_members(mid).to_vec();
                let mut present: Vec<&str> = Vec::with_capacity(properties.len());
                for prop in properties {
                    let ObjectProp::Property {
                        key: PropKey::Identifier(key) | PropKey::Str(key),
                        value,
                        ..
                    } = prop
                    else {
                        return false;
                    };
                    let slot = members.iter().find_map(|m| match m {
                        ObjectTypeMember::Property { name, ty, .. } if name.as_ref() == key => {
                            Some(Type::resolved(*ty))
                        }
                        ObjectTypeMember::Property { .. }
                        | ObjectTypeMember::Method { .. }
                        | ObjectTypeMember::Callable { .. } => None,
                    });
                    let Some(slot) = slot else {
                        return false;
                    };
                    if !self.part_fits(rec, &slot, *value, bind, depth) {
                        return false;
                    }
                    present.push(key.as_str());
                }
                return members.iter().all(|m| match m {
                    ObjectTypeMember::Property {
                        name,
                        optional: false,
                        ..
                    }
                    | ObjectTypeMember::Method {
                        name,
                        optional: false,
                        ..
                    } => present.contains(&name.as_ref()),
                    ObjectTypeMember::Property { .. }
                    | ObjectTypeMember::Method { .. }
                    | ObjectTypeMember::Callable { .. } => true,
                });
            }
        }
        if let (ExprKind::Object { properties, .. }, TypeKind::Named(..) | TypeKind::Generic(..)) =
            (&arena.expr(literal).kind, shape_kind)
        {
            let mut filled: Vec<ObjectTypeMember> = Vec::with_capacity(properties.len());
            for prop in properties {
                let ObjectProp::Property {
                    key: PropKey::Identifier(key) | PropKey::Str(key),
                    value,
                    ..
                } = prop
                else {
                    return false;
                };
                let slot = match self.find_member_info(shape, key, bind) {
                    Some((slot, _)) => {
                        if !self.part_fits(rec, &slot, *value, bind, depth) {
                            return false;
                        }
                        slot
                    }
                    None => self.infer_type(rec, *value, bind),
                };
                filled.push(ObjectTypeMember::Property {
                    name: std::sync::Arc::from(key.as_str()),
                    ty: slot.0,
                    optional: false,
                    readonly: false,
                });
            }
            let as_declared =
                Type::record(filled, &mut *std::sync::Arc::make_mut(&mut self.ty_table));
            return self.types_compatible_cached(shape, &as_declared, Some(bind));
        }
        false
    }

    pub(super) fn part_fits(
        &mut self,
        rec: &mut Recorder,
        slot: &Type,
        part: ExprId,
        bind: &BindResult,
        depth: u8,
    ) -> bool {
        let given = self.infer_type(rec, part, bind);
        self.value_assignable_to(slot, &given, Some(part), Some(bind))
            || self.literal_fits(rec, slot, part, bind, depth + 1)
    }

    pub(super) fn spread_fits(
        &mut self,
        rec: &mut Recorder,
        item: &Type,
        spread: ExprId,
        bind: &BindResult,
    ) -> bool {
        let given = self.infer_type(rec, spread, bind);
        if given.is_unresolved() {
            return true;
        }
        let parts: Vec<Type> = match self.ty_table.get(given.0) {
            TypeKind::Tuple(items) => self
                .ty_table
                .get_list(items)
                .iter()
                .map(|i| Type::resolved(*i))
                .collect(),
            TypeKind::Array(inner) => vec![Type::resolved(inner)],
            TypeKind::Primitive(_)
            | TypeKind::Literal(_)
            | TypeKind::This
            | TypeKind::Union(_)
            | TypeKind::Intersection(_)
            | TypeKind::Named(..)
            | TypeKind::Generic(..)
            | TypeKind::Fn(_)
            | TypeKind::Record(_)
            | TypeKind::Interface(_)
            | TypeKind::EnumVariant { .. }
            | TypeKind::TypePredicate { .. } => return false,
        };
        parts
            .iter()
            .all(|part| self.types_compatible_cached(item, part, Some(bind)))
    }
}
