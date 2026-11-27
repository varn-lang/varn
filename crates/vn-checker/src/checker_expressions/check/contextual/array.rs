use crate::checker::recorder::Recorder;
use crate::checker::Checker;
use vn_core::ast::{ArrayEl, ExprId};
use vn_core::{Diagnostic, ErrorCode, TypeKind};
use vn_sem::bind::BindResult;
use vn_sem::types::Type;

impl<'r> Checker<'r> {
    pub(in super::super) fn check_array_with_context(
        &mut self,
        rec: &mut Recorder,
        expr: ExprId,
        elements: &[ArrayEl],
        bind: &BindResult,
    ) {
        let elem_expected = self
            .expected_for(expr)
            .and_then(|t| match self.ty_table.get(t.0) {
                TypeKind::Array(inner) => Some(Type::resolved(inner)),
                TypeKind::Primitive(_)
                | TypeKind::Literal(_)
                | TypeKind::This
                | TypeKind::Union(_)
                | TypeKind::Intersection(_)
                | TypeKind::Tuple(_)
                | TypeKind::Named(..)
                | TypeKind::Generic(..)
                | TypeKind::Fn(_)
                | TypeKind::Record(_)
                | TypeKind::Interface(_)
                | TypeKind::EnumVariant { .. }
                | TypeKind::TypePredicate { .. } => None,
            });

        for el in elements {
            match el {
                ArrayEl::Expr(e) => {
                    self.with_expected(elem_expected, |c| c.check_expr(rec, *e, bind));
                    if let Some(expected) = &elem_expected {
                        let actual = self.infer_type(rec, *e, bind);

                        if !actual.is_unresolved()
                            && !self.value_fits(rec, expected, &actual, Some(*e), Some(bind))
                        {
                            let actual_s = actual.display(&self.ty_table, &bind.interner);
                            let expected_s = expected.display(&self.ty_table, &bind.interner);
                            self.emit(
                                Diagnostic::error(ErrorCode::TypeMismatch, format!(
                                    "type mismatch: array element is '{actual_s}', expected '{expected_s}'"
                                ))
                                .with_range(self.ast_arena.expr(*e).range),
                            );
                        }
                    }
                }
                ArrayEl::Spread(e) => {
                    self.check_expr(rec, *e, bind);
                    let spread = vn_core::ast::Arg::Spread(*e);
                    if let Err(given) = self.infer_arg_type(rec, &spread, bind) {
                        self.emit(
                            Diagnostic::error(
                                ErrorCode::TypeMismatch,
                                format!(
                                    "spread needs an array, tuple, Set, Map, string or Range, got '{}': lazy iterators use for-in or toArray()",
                                    given.display(&self.ty_table, &bind.interner)
                                ),
                            )
                            .with_range(self.ast_arena.expr(*e).range),
                        );
                    }
                }
                ArrayEl::Hole => {
                    self.emit(
                        Diagnostic::error(
                            ErrorCode::InvalidStatement,
                            vn_core::ForeignForm::ArrayHole.message(),
                        )
                        .with_range(self.ast_arena.expr(expr).range),
                    );
                }
            }
        }
    }
}
