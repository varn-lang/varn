use crate::checker::Checker;
use vn_core::ast::operators::UnaryOp;
use vn_core::ast::{ExprId, ExprKind};
use vn_core::BinOp;

impl<'r> Checker<'r> {
    pub(crate) fn can_extract_narrowings(&self, expr: ExprId) -> bool {
        matches!(
            &self.ast_arena.expr(expr).kind,
            ExprKind::Binary {
                op: BinOp::Eq | BinOp::Ne,
                ..
            } | ExprKind::Logical { .. }
                | ExprKind::Is { .. }
                | ExprKind::Paren { .. }
                | ExprKind::Call { .. }
                | ExprKind::Identifier { .. }
                | ExprKind::Unary {
                    op: UnaryOp::Not,
                    ..
                }
        )
    }
}
