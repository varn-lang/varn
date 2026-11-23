use super::context::FnEmitter;
use super::small_utils::{bool_lit, int_lit};
use std::sync::Arc;
use vn_core::ast::{ExprId, ExprKind, Pattern, StmtId};
use vn_core::BinOp;
use vn_core::MemberKey;
use vn_sem::output::ForOfPlan;
use vn_tir::{BackendTy, Resolution, Span, TirExpr, TirExprKind, TirStmt};

pub(super) fn local_var(local: vn_tir::LocalId, ty: BackendTy) -> TirExpr {
    TirExpr {
        kind: TirExprKind::Var,
        ty,
        res: Resolution::Local(local),
        span: Span::EMPTY,
    }
}

pub(super) fn binary(op: BinOp, lhs: TirExpr, rhs: TirExpr, ty: BackendTy) -> TirExpr {
    TirExpr {
        kind: TirExprKind::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        ty,
        res: Resolution::None,
        span: Span::EMPTY,
    }
}

pub(super) fn assign(target: TirExpr, value: TirExpr) -> TirStmt {
    TirStmt::Expr(TirExpr {
        kind: TirExprKind::Assign {
            target: Box::new(target),
            value: Box::new(value),
        },
        ty: BackendTy::Void,
        res: Resolution::None,
        span: Span::EMPTY,
    })
}

impl<'a> FnEmitter<'a> {
    pub(super) fn lower_for_of(
        &mut self,
        left: &Pattern,
        right: ExprId,
        body: StmtId,
        is_await: bool,
    ) -> Vec<TirStmt> {
        if is_await && self.top_level {
            self.saw_await = true;
        }
        if !is_await
            && let ExprKind::Range {
                start,
                end,
                inclusive,
            } = &self.ast_arena.expr(right).kind
            && matches!(self.expr_ty(*start).representation(), BackendTy::INT)
            && matches!(self.expr_ty(*end).representation(), BackendTy::INT)
        {
            let (start, end, inclusive) = (*start, *end, *inclusive);
            let lo = self.lower_expr(start);
            let hi = self.lower_expr(end);
            let hi = if inclusive {
                binary(BinOp::Add, hi, int_lit(1), BackendTy::INT)
            } else {
                hi
            };
            let mut out = std::mem::take(&mut self.pending);
            out.extend(self.int_loop(left, lo, hi, body, |i| i));
            return out;
        }

        let plan = self.m.desugar.for_of_plans.get(&right.index()).copied();
        let source = self.lower_expr(right);
        let mut out = std::mem::take(&mut self.pending);
        match plan {
            Some(ForOfPlan::Collection { method, elem }) => {
                let elem = self.lower_checked(&elem);
                let items = match method {
                    None => source,
                    Some(key) => {
                        let ty = BackendTy::Array(self.tt.intern(elem));
                        self.member_call(
                            source,
                            Arc::from(key.as_str()),
                            vec![],
                            ty,
                            None,
                            Span::EMPTY,
                        )
                    }
                };
                let items = self.hoist(items);
                out.extend(std::mem::take(&mut self.pending));
                out.extend(self.for_of_over_array(left, items, elem, body));
            }
            Some(ForOfPlan::IntRange) => {
                let range = self.hoist(source);
                out.extend(std::mem::take(&mut self.pending));
                let field = |me: &mut Self, key: MemberKey| {
                    me.field_access(
                        range.clone(),
                        Arc::from(key.as_str()),
                        BackendTy::INT,
                        Span::EMPTY,
                    )
                };
                let start = field(self, MemberKey::Start);
                let count = field(self, MemberKey::Length);
                let stride = field(self, MemberKey::Stride);
                let start = self.hoist(start);
                let stride = self.hoist(stride);
                out.extend(std::mem::take(&mut self.pending));
                out.extend(self.int_loop(left, int_lit(0), count, body, move |i| {
                    let offset = binary(BinOp::Mul, i, stride, BackendTy::INT);
                    binary(BinOp::Add, start, offset, BackendTy::INT)
                }));
            }
            Some(ForOfPlan::Protocol {
                iterator,
                next,
                step,
            }) => {
                let plan = super::stmt_for_of_protocol::Protocol {
                    iterator,
                    next,
                    step,
                    is_await,
                };
                out.extend(self.for_of_protocol(left, source, &plan, body));
            }
            None => {}
        }
        out
    }

    pub(super) fn lower_checked(&mut self, ty: &vn_sem::types::Type) -> BackendTy {
        crate::ty::lower_type(
            ty,
            self.m.checker_table,
            self.m.interner,
            self.tt,
            self.m.names,
        )
    }

    fn int_loop(
        &mut self,
        left: &Pattern,
        lo: TirExpr,
        hi: TirExpr,
        body: StmtId,
        element: impl FnOnce(TirExpr) -> TirExpr,
    ) -> Vec<TirStmt> {
        let mut out = Vec::new();
        let hi = self.hoist(hi);
        out.extend(std::mem::take(&mut self.pending));
        let i = self.fresh_local(BackendTy::INT);
        out.push(TirStmt::Let {
            local: i,
            ty: BackendTy::INT,
            init: Some(binary(BinOp::Sub, lo, int_lit(1), BackendTy::INT)),
        });
        let ivar = || local_var(i, BackendTy::INT);
        let mut loop_body = vec![
            assign(
                ivar(),
                binary(BinOp::Add, ivar(), int_lit(1), BackendTy::INT),
            ),
            TirStmt::If {
                cond: binary(BinOp::Lt, ivar(), hi, BackendTy::Bool),
                then_body: vec![],
                else_body: vec![TirStmt::Break],
            },
        ];
        self.bind_pattern(left, element(ivar()), &mut loop_body);
        loop_body.extend(self.lower_stmt_as_block(body));
        out.push(TirStmt::Loop {
            cond: bool_lit(true),
            body: loop_body,
        });
        out
    }

    pub(super) fn for_of_over_array(
        &mut self,
        left: &Pattern,
        arr: TirExpr,
        elem_ty: BackendTy,
        body: StmtId,
    ) -> Vec<TirStmt> {
        let source = arr.clone();
        self.indexed_loop(left, arr, body, move |idx| {
            index(source.clone(), idx, elem_ty)
        })
    }

    fn indexed_loop(
        &mut self,
        left: &Pattern,
        sized: TirExpr,
        body: StmtId,
        element: impl FnOnce(TirExpr) -> TirExpr,
    ) -> Vec<TirStmt> {
        let mut out = Vec::new();
        let idx = self.fresh_local(BackendTy::INT);
        out.push(TirStmt::Let {
            local: idx,
            ty: BackendTy::INT,
            init: Some(int_lit(-1)),
        });
        let idx_var = || local_var(idx, BackendTy::INT);
        let len = self.field_access(
            sized,
            Arc::from(MemberKey::Length.as_str()),
            BackendTy::INT,
            Span::EMPTY,
        );
        let mut loop_body = vec![
            assign(
                idx_var(),
                binary(BinOp::Add, idx_var(), int_lit(1), BackendTy::INT),
            ),
            TirStmt::If {
                cond: binary(BinOp::Lt, idx_var(), len, BackendTy::Bool),
                then_body: vec![],
                else_body: vec![TirStmt::Break],
            },
        ];
        self.bind_pattern(left, element(idx_var()), &mut loop_body);
        loop_body.extend(self.lower_stmt_as_block(body));
        out.push(TirStmt::Loop {
            cond: bool_lit(true),
            body: loop_body,
        });
        out
    }
}

fn index(object: TirExpr, at: TirExpr, ty: BackendTy) -> TirExpr {
    TirExpr {
        kind: TirExprKind::Index {
            object: Box::new(object),
            index: Box::new(at),
        },
        ty,
        res: Resolution::None,
        span: Span::EMPTY,
    }
}
