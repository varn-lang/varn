use super::context::FnEmitter;
use vn_core::ast::ExprId;
use vn_tir::{BackendTy, DynReason, Resolution, Span, TirExpr, TirExprKind, TirStmt};

impl<'a> FnEmitter<'a> {
    pub(super) fn lower_await(&mut self, argument: ExprId, ty: BackendTy, span: Span) -> TirExpr {
        if self.top_level {
            self.saw_await = true;
        }
        let fut = self.lower_expr(argument);
        TirExpr {
            kind: TirExprKind::Await {
                future: Box::new(fut),
            },
            ty,
            res: Resolution::None,
            span,
        }
    }

    pub(super) fn lower_yield(&mut self, argument: Option<ExprId>, span: Span) -> TirExpr {
        let value = argument.map(|a| {
            let value = self.lower_expr(a);
            let value = self.widened(value, self.yield_ty);
            Box::new(self.guard_dynamic(value, self.yield_ty))
        });
        TirExpr {
            kind: TirExprKind::Yield { value },
            ty: BackendTy::Dynamic(DynReason::Declared),
            res: Resolution::None,
            span,
        }
    }

    pub(super) fn lower_yield_stmt(
        &mut self,
        argument: Option<ExprId>,
        span: Span,
    ) -> Vec<TirStmt> {
        let yield_expr = self.lower_yield(argument, span);
        let mut out = std::mem::take(&mut self.pending);
        let closing = self.fresh_local(BackendTy::Bool);
        out.push(TirStmt::Let {
            local: closing,
            ty: BackendTy::Bool,
            init: Some(yield_expr),
        });
        out.push(TirStmt::If {
            cond: TirExpr {
                kind: TirExprKind::Var,
                ty: BackendTy::Bool,
                res: Resolution::Local(closing),
                span,
            },
            then_body: vec![TirStmt::Return(None)],
            else_body: vec![],
        });
        out
    }
}
