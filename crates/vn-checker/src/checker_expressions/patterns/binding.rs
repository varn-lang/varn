use crate::checker::recorder::Recorder;
use crate::checker::Checker;
use vn_sem::bind::BindResult;
use vn_sem::types::Type;

impl<'r> Checker<'r> {
    pub(crate) fn check_pattern(
        &mut self,
        rec: &mut Recorder,
        pattern: &vn_core::ast::Pattern,
        value_ty: &Type,
        bind: &BindResult,
    ) {
        use vn_core::ast::Pattern;
        match pattern {
            Pattern::Identifier { name, range, .. } => {
                if bind.interner.resolve(*name) == vn_core::TokenKind::Placeholder.as_str() {
                    return;
                }
                let scope = bind.scopes.get(self.current_scope);
                if let Some(id) = scope.resolve(*name, &bind.scopes) {
                    self.record_type_with_symbol(rec, range.start.offset, *value_ty, id);
                } else {
                    self.record_type(rec, range.start.offset, *value_ty);
                }
            }
            Pattern::Array { elements, rest, .. } | Pattern::Tuple { elements, rest, .. } => {
                let shape = self.sequence_shape(
                    value_ty,
                    matches!(pattern, Pattern::Tuple { .. }),
                    elements.len(),
                    rest.is_some(),
                    *pattern.range(),
                    bind,
                );
                let (fixed, elem_ty) = (shape.fixed, shape.element);
                if !shape.sequence {
                    return;
                }
                for (at, el) in elements.iter().enumerate() {
                    let Some(el) = el else { continue };
                    let item_ty = match &fixed {
                        Some(items) => items.get(at).copied().unwrap_or(Type::Error),
                        None => elem_ty,
                    };
                    self.check_pattern(rec, &el.pattern, &item_ty, bind);
                }
                if let Some(r) = rest {
                    self.check_pattern(rec, r, value_ty, bind);
                }
            }
            Pattern::Object {
                properties,
                rest,
                is_hash,
                ..
            } => {
                let scrutinee_is_nominal = if value_ty.is_unresolved() {
                    None
                } else {
                    let scrutinee =
                        value_ty.non_nullified(&mut *std::sync::Arc::make_mut(&mut self.ty_table));
                    Some(matches!(
                        self.ty_table.get(scrutinee.0),
                        vn_core::TypeKind::Named(..) | vn_core::TypeKind::Generic(..)
                    ))
                };
                if !is_hash && scrutinee_is_nominal == Some(false) {
                    self.emit(
                        vn_core::Diagnostic::error(
                            vn_core::ErrorCode::RecordRequiresHash,
                            "a record pattern is written `#{ x }`: `{ ... }` against a non-nominal type is a struct pattern, write it with `#`",
                        )
                        .with_range(*pattern.range()),
                    );
                }
                if *is_hash && scrutinee_is_nominal == Some(true) {
                    self.emit(
                        vn_core::Diagnostic::error(
                            vn_core::ErrorCode::RecordRequiresHash,
                            "a struct pattern is written `{ x }`: `#{ ... }` is always a record pattern, write it without `#`",
                        )
                        .with_range(*pattern.range()),
                    );
                }
                for prop in properties {
                    let key = bind.interner.resolve(prop.key);
                    let defaulted = matches!(prop.value, Pattern::Assignment { .. });
                    if !defaulted
                        && !value_ty.is_unresolved()
                        && !self.member_exists_cached(value_ty, key, bind)
                    {
                        self.emit(
                            vn_core::Diagnostic::error(
                                vn_core::ErrorCode::MissingProperty,
                                format!(
                                    "property '{key}' does not exist on type '{}'",
                                    value_ty.display(&self.ty_table, &bind.interner)
                                ),
                            )
                            .with_range(*prop.value.range()),
                        );
                    }
                    let default_ty = match &prop.value {
                        Pattern::Assignment { right, .. } => {
                            Some(self.infer_type(rec, *right, bind))
                        }
                        Pattern::Identifier { .. }
                        | Pattern::Array { .. }
                        | Pattern::Tuple { .. }
                        | Pattern::Object { .. }
                        | Pattern::Rest { .. } => None,
                    };
                    let prop_ty = self
                        .find_member_info(value_ty, key, bind)
                        .map(|(t, _)| t)
                        .or(default_ty)
                        .unwrap_or(Type::Error);
                    self.check_pattern(rec, &prop.value, &prop_ty, bind);
                }
                if let Some(r) = rest {
                    let extracted: Vec<&str> = properties
                        .iter()
                        .map(|prop| bind.interner.resolve(prop.key))
                        .collect();
                    let rest_ty = self.object_rest_type(value_ty, &extracted, *r.range(), bind);
                    self.check_pattern(rec, r, &rest_ty, bind);
                }
            }
            Pattern::Rest { argument, .. } => self.check_pattern(rec, argument, value_ty, bind),
            Pattern::Assignment { left, .. } => {
                self.check_pattern(rec, left, value_ty, bind);
            }
        }
    }

    pub(super) fn object_rest_type(
        &mut self,
        value_ty: &Type,
        extracted: &[&str],
        range: vn_core::SourceRange,
        bind: &BindResult,
    ) -> Type {
        if value_ty.is_unresolved() {
            return *value_ty;
        }
        if let vn_core::TypeKind::Record(id) | vn_core::TypeKind::Interface(id) =
            self.ty_table.get(value_ty.0)
        {
            let remaining: Vec<vn_sem::types::ObjectTypeMember> = self
                .ty_table
                .get_object_members(id)
                .iter()
                .filter(|m| match m {
                    vn_sem::types::ObjectTypeMember::Property { name, .. }
                    | vn_sem::types::ObjectTypeMember::Method { name, .. } => {
                        !extracted.contains(&name.as_ref())
                    }
                    vn_sem::types::ObjectTypeMember::Callable { .. } => true,
                })
                .cloned()
                .collect();
            return Type::record(remaining, std::sync::Arc::make_mut(&mut self.ty_table));
        }
        self.emit(
            vn_core::Diagnostic::error(
                vn_core::ErrorCode::TypeMismatch,
                format!(
                    "object rest needs an object type, found '{}'",
                    value_ty.display(&self.ty_table, &bind.interner)
                ),
            )
            .with_range(range),
        );
        Type::Error
    }
}
