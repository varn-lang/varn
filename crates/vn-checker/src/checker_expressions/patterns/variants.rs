use crate::checker::Checker;
use vn_core::TypeKind;
use vn_sem::bind::BindResult;
use vn_sem::types::Type;

use super::VariantSubst;
impl<'r> Checker<'r> {
    pub(crate) fn variant_fields_with_subst(
        &mut self,
        variant: &str,
        value_ty: &Type,
        bind: &BindResult,
    ) -> Option<VariantSubst> {
        let (parent, args, origin): (
            Option<std::sync::Arc<str>>,
            Vec<Type>,
            Option<std::sync::Arc<str>>,
        ) = match self.ty_table.get(value_ty.0) {
            TypeKind::Generic(n, a, o) => (
                Some(std::sync::Arc::from(bind.interner.resolve(n))),
                self.ty_table
                    .get_list(a)
                    .iter()
                    .map(|id| Type::resolved(*id))
                    .collect(),
                o.map(|o| std::sync::Arc::from(bind.interner.resolve(o))),
            ),
            TypeKind::Named(n, o) => (
                Some(std::sync::Arc::from(bind.interner.resolve(n))),
                Vec::new(),
                o.map(|o| std::sync::Arc::from(bind.interner.resolve(o))),
            ),
            TypeKind::Primitive(_)
            | TypeKind::Literal(_)
            | TypeKind::This
            | TypeKind::Array(_)
            | TypeKind::Union(_)
            | TypeKind::Intersection(_)
            | TypeKind::Tuple(_)
            | TypeKind::Fn(_)
            | TypeKind::Record(_)
            | TypeKind::Interface(_)
            | TypeKind::EnumVariant { .. }
            | TypeKind::TypePredicate { .. } => (None, Vec::new(), None),
        };

        let owner = parent.clone()?;
        let mut fields = bind.variant_fields(&owner, variant).cloned();
        let mut def_bind: Option<std::sync::Arc<BindResult>> = None;
        if fields.is_none()
            && let Some(o) = &origin
        {
            let mb = self
                .resolver
                .module_bind(o.as_ref())
                .or_else(|| self.resolver.stdlib_bind(o.as_ref()));
            if let Some(mb) = mb {
                fields = mb.variant_fields(&owner, variant).cloned();
                def_bind = Some(mb);
            }
        }
        let fields = fields?;

        let mut mapping = rustc_hash::FxHashMap::default();
        if !args.is_empty()
            && let Some(p) = &parent
        {
            let mut params = self.symbol_type_params_any(p.as_ref(), bind);
            if params.is_empty()
                && let Some(db) = &def_bind
            {
                params = db
                    .interner
                    .get(p.as_ref())
                    .and_then(|atom| db.scopes.get(db.global_scope).resolve(atom, &db.scopes))
                    .map(|sid| {
                        db.arena
                            .get(sid)
                            .type_params
                            .iter()
                            .map(|a| std::sync::Arc::from(db.interner.resolve(*a)))
                            .collect()
                    })
                    .unwrap_or_default();
            }
            for (name, arg) in params.iter().zip(args.iter()) {
                mapping.insert(name.clone(), *arg);
            }
        }
        Some((fields, mapping))
    }
}
