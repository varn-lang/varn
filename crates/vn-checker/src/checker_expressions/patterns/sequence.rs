use crate::checker::Checker;
use vn_sem::bind::BindResult;
use vn_sem::types::Type;

pub(crate) struct SequenceShape {
    pub fixed: Option<Vec<Type>>,
    pub element: Type,
    pub sequence: bool,
}

impl Checker<'_> {
    pub(crate) fn sequence_shape(
        &mut self,
        value_ty: &Type,
        tuple_pattern: bool,
        element_count: usize,
        has_rest: bool,
        range: vn_core::SourceRange,
        bind: &BindResult,
    ) -> SequenceShape {
        let fixed: Option<Vec<Type>> =
            if let vn_core::TypeKind::Tuple(items) = self.ty_table.get(value_ty.0) {
                Some(
                    self.ty_table
                        .get_list(items)
                        .iter()
                        .map(|item| Type::resolved(*item))
                        .collect(),
                )
            } else {
                None
            };
        let value_is_array = matches!(self.ty_table.get(value_ty.0), vn_core::TypeKind::Array(_));
        let is_readonly_sequence = value_ty.sequence_element(&self.ty_table).is_some();
        let not_sequence = match self.ty_table.get(value_ty.0) {
            vn_core::TypeKind::Primitive(p) => !matches!(
                p,
                vn_core::LangPrimitive::Dynamic
                    | vn_core::LangPrimitive::Error
                    | vn_core::LangPrimitive::Str
            ),
            vn_core::TypeKind::Named(..)
            | vn_core::TypeKind::Generic(..)
            | vn_core::TypeKind::Record(_)
            | vn_core::TypeKind::Interface(_)
            | vn_core::TypeKind::Fn(_)
            | vn_core::TypeKind::EnumVariant { .. } => !is_readonly_sequence,
            vn_core::TypeKind::Literal(_)
            | vn_core::TypeKind::This
            | vn_core::TypeKind::Array(_)
            | vn_core::TypeKind::Union(_)
            | vn_core::TypeKind::Intersection(_)
            | vn_core::TypeKind::Tuple(_)
            | vn_core::TypeKind::TypePredicate { .. } => false,
        };
        if not_sequence {
            self.emit(
                vn_core::Diagnostic::error(
                    vn_core::ErrorCode::TypeMismatch,
                    format!(
                        "'{}' is neither an array nor a tuple, so it cannot be destructured by position",
                        value_ty.display(&self.ty_table, &bind.interner)
                    ),
                )
                .with_range(range),
            );
            return SequenceShape {
                fixed,
                element: Type::Error,
                sequence: false,
            };
        }
        if tuple_pattern && value_is_array {
            self.emit(
                vn_core::Diagnostic::error(
                    vn_core::ErrorCode::TypeMismatch,
                    format!(
                        "'{}' is an array: destructure it with [a, b], not #[a, b]",
                        value_ty.display(&self.ty_table, &bind.interner)
                    ),
                )
                .with_range(range),
            );
        } else if !tuple_pattern && fixed.is_some() {
            self.emit(
                vn_core::Diagnostic::error(
                    vn_core::ErrorCode::TypeMismatch,
                    format!(
                        "'{}' is a tuple: destructure it with #[a, b], not [a, b]",
                        value_ty.display(&self.ty_table, &bind.interner)
                    ),
                )
                .with_range(range),
            );
        }
        let element = value_ty.get_array_element_type(&self.ty_table);
        if let Some(items) = &fixed {
            let arity_ok = if has_rest {
                element_count <= items.len()
            } else {
                element_count == items.len()
            };
            if !arity_ok {
                self.emit(
                    vn_core::Diagnostic::error(
                        vn_core::ErrorCode::TupleIndexOutOfBounds,
                        format!(
                            "this pattern destructures {} elements, but '{}' has {}",
                            element_count,
                            value_ty.display(&self.ty_table, &bind.interner),
                            items.len()
                        ),
                    )
                    .with_range(range),
                );
            }
        }
        SequenceShape {
            fixed,
            element,
            sequence: true,
        }
    }
}
