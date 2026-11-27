use crate::checker::recorder::Recorder;
use crate::checker::Checker;
use vn_core::TypeKind;
use vn_sem::bind::BindResult;
use vn_sem::types::Type;

impl<'r> Checker<'r> {
    pub(crate) fn check_pattern_match(
        &mut self,
        rec: &mut Recorder,
        pattern: &vn_core::ast::MatchPattern,
        value_ty: &Type,
        range: vn_core::SourceRange,
        bind: &BindResult,
    ) {
        use vn_core::ast::{MatchPattern, SequenceRest, VariantPayload};
        match pattern {
            MatchPattern::Wildcard => {}
            MatchPattern::Identifier(name) => {
                if bind.interner.resolve(*name) == vn_core::TokenKind::Placeholder.as_str() {
                    return;
                }
                let scope = bind.scopes.get(self.current_scope);
                if let Some(id) = scope.resolve(*name, &bind.scopes) {
                    self.record_type_with_symbol(rec, 0, *value_ty, id);
                }
            }
            MatchPattern::Tuple(items) => {
                let present = self.present_subject(value_ty);
                let shape = self.sequence_shape(&present, true, items.len(), false, range, bind);
                for (at, item) in items.iter().enumerate() {
                    let item_ty = match &shape.fixed {
                        Some(fixed) => fixed.get(at).copied().unwrap_or(Type::Error),
                        None => shape.element,
                    };
                    self.check_pattern_match(rec, item, &item_ty, range, bind);
                }
            }
            MatchPattern::Array { elements, rest } => {
                let present = self.present_subject(value_ty);
                let shape = self.sequence_shape(
                    &present,
                    false,
                    elements.len(),
                    !matches!(rest, SequenceRest::Closed),
                    range,
                    bind,
                );
                for (at, item) in elements.iter().enumerate() {
                    let item_ty = match &shape.fixed {
                        Some(fixed) => fixed.get(at).copied().unwrap_or(Type::Error),
                        None => shape.element,
                    };
                    self.check_pattern_match(rec, item, &item_ty, range, bind);
                }
                if let SequenceRest::Bind(name) = rest {
                    let rest_ty =
                        Type::array(shape.element, std::sync::Arc::make_mut(&mut self.ty_table));
                    let scope = bind.scopes.get(self.current_scope);
                    if let Some(id) = scope.resolve(*name, &bind.scopes) {
                        self.record_type_with_symbol(rec, 0, rest_ty, id);
                    }
                }
            }
            MatchPattern::Record { fields, .. } => {
                let present = self.present_subject(value_ty);
                for field in fields {
                    let name = bind.interner.resolve(field.name);
                    let member_ty = if present.is_unresolved() {
                        Type::Error
                    } else if let Some((found, _)) = self.find_member_info(&present, name, bind) {
                        found
                    } else {
                        self.emit(
                            vn_core::Diagnostic::error(
                                vn_core::ErrorCode::MissingProperty,
                                format!(
                                    "property '{name}' does not exist on type '{}'",
                                    present.display(&self.ty_table, &bind.interner)
                                ),
                            )
                            .with_range(field.range),
                        );
                        Type::Error
                    };
                    self.check_match_field(rec, field, &member_ty, range, bind);
                }
            }
            MatchPattern::EnumVariant {
                enum_name,
                variant_name,
                payload,
            } => {
                let variant_name_str = bind.interner.resolve(*variant_name).to_string();
                if let Some(qualifier) = enum_name
                    && let TypeKind::Generic(actual, ..) | TypeKind::Named(actual, ..) =
                        self.ty_table.get(value_ty.0)
                    && bind.interner.resolve(actual) != bind.interner.resolve(*qualifier)
                {
                    self.emit(
                        vn_core::Diagnostic::error(
                            vn_core::ErrorCode::MatchPatternTypeMismatch,
                            format!(
                                "the pattern names `{}.{variant_name_str}`, but the value is a `{}`",
                                bind.interner.resolve(*qualifier),
                                bind.interner.resolve(actual)
                            ),
                        )
                        .with_range(range),
                    );
                    return;
                }
                let subject_is_enum = self.enum_subject(value_ty, bind);
                let lookup_ty = match (enum_name, subject_is_enum) {
                    (_, true) => *value_ty,
                    (Some(qualifier), false) => Type::named(
                        bind.interner.resolve(*qualifier),
                        &mut *std::sync::Arc::make_mut(&mut self.ty_table),
                    ),
                    (None, false) => {
                        if !value_ty.is_error() {
                            let shown = value_ty.display(&self.ty_table, &bind.interner);
                            self.emit(
                                vn_core::Diagnostic::error(
                                    vn_core::ErrorCode::MatchPatternTypeMismatch,
                                    format!(
                                        "'{shown}' is not an enum, so `{variant_name_str}` does not say which enum it belongs to: write `Enum.{variant_name_str}(...)`"
                                    ),
                                )
                                .with_range(range),
                            );
                        }
                        return;
                    }
                };
                let Some((fields, mapping)) =
                    self.variant_fields_with_subst(&variant_name_str, &lookup_ty, bind)
                else {
                    return;
                };
                let field_ty = |checker: &mut Self, ty: &Type| {
                    if mapping.is_empty() {
                        *ty
                    } else {
                        crate::generic_substitution::map_generics_cached(checker, ty, &mapping)
                    }
                };
                match payload {
                    VariantPayload::Positional(items) => {
                        if items.len() != fields.len() {
                            self.emit(
                                vn_core::Diagnostic::error(
                                    vn_core::ErrorCode::MatchPatternTypeMismatch,
                                    format!(
                                        "variant '{variant_name_str}' carries {} value(s), but the pattern has {}",
                                        fields.len(),
                                        items.len()
                                    ),
                                )
                                .with_range(range),
                            );
                        }
                        for (at, item) in items.iter().enumerate() {
                            let item_ty = match fields.get(at) {
                                Some((_, ty)) => field_ty(self, ty),
                                None => Type::Error,
                            };
                            self.check_pattern_match(rec, item, &item_ty, range, bind);
                        }
                    }
                    VariantPayload::Named { fields: named, .. } => {
                        for field in named {
                            let name = bind.interner.resolve(field.name);
                            let item_ty = match fields.iter().find(|(n, _)| n.as_ref() == name) {
                                Some((_, ty)) => field_ty(self, ty),
                                None => {
                                    self.emit(
                                        vn_core::Diagnostic::error(
                                            vn_core::ErrorCode::MatchPatternTypeMismatch,
                                            format!(
                                                "variant '{variant_name_str}' has no field named '{name}'"
                                            ),
                                        )
                                        .with_range(field.range),
                                    );
                                    Type::Error
                                }
                            };
                            self.check_match_field(rec, field, &item_ty, range, bind);
                        }
                    }
                }
            }
            MatchPattern::Literal(expr) => {
                self.check_expr(rec, *expr, bind);
                let given = self.infer_type(rec, *expr, bind);
                if !value_ty.is_unresolved()
                    && !given.is_unresolved()
                    && !self.value_fits(rec, value_ty, &given, Some(*expr), Some(bind))
                {
                    let given_s = given.display(&self.ty_table, &bind.interner);
                    let value_s = value_ty.display(&self.ty_table, &bind.interner);
                    self.emit(
                        vn_core::Diagnostic::error(
                            vn_core::ErrorCode::MatchPatternTypeMismatch,
                            format!(
                                "this pattern is '{given_s}' and can never match a value of type '{value_s}'"
                            ),
                        )
                        .with_range(self.ast_arena.expr(*expr).range),
                    );
                }
            }
        }
    }

    pub(super) fn present_subject(&mut self, value_ty: &Type) -> Type {
        if value_ty.is_unresolved() {
            *value_ty
        } else {
            value_ty.non_nullified(std::sync::Arc::make_mut(&mut self.ty_table))
        }
    }

    pub(super) fn check_match_field(
        &mut self,
        rec: &mut Recorder,
        field: &vn_core::ast::MatchField,
        field_ty: &Type,
        range: vn_core::SourceRange,
        bind: &BindResult,
    ) {
        match &field.pattern {
            Some(sub) => self.check_pattern_match(rec, sub, field_ty, range, bind),
            None => {
                if bind.interner.resolve(field.name) == vn_core::TokenKind::Placeholder.as_str() {
                    return;
                }
                let scope = bind.scopes.get(self.current_scope);
                if let Some(id) = scope.resolve(field.name, &bind.scopes) {
                    self.record_type_with_symbol(rec, field.range.start.offset, *field_ty, id);
                }
            }
        }
    }

    pub(super) fn enum_subject(&mut self, value_ty: &Type, bind: &BindResult) -> bool {
        let ty = value_ty.non_nullified(&mut *std::sync::Arc::make_mut(&mut self.ty_table));
        let (TypeKind::Named(name, origin) | TypeKind::Generic(name, _, origin)) =
            self.ty_table.get(ty.0)
        else {
            return false;
        };
        let name: std::sync::Arc<str> = std::sync::Arc::from(bind.interner.resolve(name));
        let origins: Vec<String> = origin
            .map(|o| bind.interner.resolve(o).to_owned())
            .into_iter()
            .collect();
        crate::checker_expressions::members::is_enum_type(self.resolver, bind, &name, &origins)
    }
}
