use super::context::FnEmitter;
use super::small_utils::{bin_op, bool_lit};
use vn_core::ast::operators::{BinaryOp, LogicalOp};
use vn_core::ast::ExprId;
use vn_tir::{BackendTy, DynReason, Resolution, Span, TirBinOp, TirExpr, TirExprKind, TirUnOp};

impl<'a> FnEmitter<'a> {
    pub(super) fn lower_binary(
        &mut self,
        op: BinaryOp,
        left: ExprId,
        right: ExprId,
        ty: BackendTy,
        span: Span,
    ) -> TirExpr {
        let lhs = self.lower_expr(left);
        let rhs = self.lower_expr(right);

        match op {
            BinaryOp::Eq => return self.equality(TirBinOp::Eq, lhs, rhs, span),
            BinaryOp::NotEq => return self.equality(TirBinOp::Ne, lhs, rhs, span),
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::Pow
            | BinaryOp::Lt
            | BinaryOp::Gt
            | BinaryOp::LtEq
            | BinaryOp::GtEq
            | BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr
            | BinaryOp::UShr
            | BinaryOp::In => {}
        }

        let Some(top) = bin_op(op) else {
            if op == BinaryOp::In {
                return TirExpr {
                    kind: TirExprKind::Binary {
                        op: TirBinOp::In,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    },
                    ty: BackendTy::Bool,
                    res: Resolution::None,
                    span,
                };
            }
            return self.cast_to(lhs, BackendTy::Bool);
        };

        let (lhs, rhs, node_ty) = self.coerce_binary_operands(top, lhs, rhs, ty);
        TirExpr {
            kind: TirExprKind::Binary {
                op: top,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            ty: node_ty,
            res: Resolution::None,
            span,
        }
    }

    pub(super) fn equality(
        &mut self,
        op: TirBinOp,
        lhs: TirExpr,
        rhs: TirExpr,
        span: Span,
    ) -> TirExpr {
        let is_eq = op == TirBinOp::Eq;
        let is_null_expr = |e: &TirExpr| -> bool {
            matches!(e.kind, TirExprKind::NullLit) || e.ty.non_nullable(self.tt) == BackendTy::Never
        };
        let (l_null, r_null) = (is_null_expr(&lhs), is_null_expr(&rhs));
        if l_null && r_null {
            return bool_lit(is_eq);
        }
        if l_null || r_null {
            let target = if r_null { lhs } else { rhs };
            let is_null = unary_bool(TirUnOp::IsNull, target, span);
            return if is_eq {
                is_null
            } else {
                unary_bool(TirUnOp::Not, is_null, span)
            };
        }
        let literal = |e: &TirExpr| {
            if let TirExprKind::BoolLit(value) = e.kind {
                Some(value)
            } else {
                None
            }
        };
        let bool_side = match (literal(&lhs), literal(&rhs)) {
            (Some(value), None) if rhs.ty == BackendTy::Bool => Some((value, rhs.clone())),
            (None, Some(value)) if lhs.ty == BackendTy::Bool => Some((value, lhs.clone())),
            (Some(_), Some(_)) | (Some(_), None) | (None, Some(_)) | (None, None) => None,
        };
        if let Some((value, other)) = bool_side {
            return if value == is_eq {
                other
            } else {
                unary_bool(TirUnOp::Not, other, span)
            };
        }
        if let (BackendTy::Enum(l), BackendTy::Enum(r)) = (lhs.ty, rhs.ty)
            && l == r
            && self.tags_decide_equality(l, &lhs, &rhs)
        {
            let (lhs, rhs) = (self.enum_tag(lhs), self.enum_tag(rhs));
            return TirExpr {
                kind: TirExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                ty: BackendTy::Bool,
                res: Resolution::None,
                span,
            };
        }
        let (lhs, rhs, ty) = self.coerce_binary_operands(op, lhs, rhs, BackendTy::Bool);
        TirExpr {
            kind: TirExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            ty,
            res: Resolution::None,
            span,
        }
    }

    fn unit_variant_tag(&self, e: &TirExpr) -> Option<i64> {
        let Resolution::EnumVariant { enum_id, tag } = e.res else {
            return None;
        };
        let constant = matches!(&e.kind, TirExprKind::MakeVariant { args } if args.is_empty())
            || matches!(e.kind, TirExprKind::Field { .. });
        let unit = self
            .m
            .enums
            .get(enum_id.0 as usize)
            .and_then(|info| info.variant_at(tag))
            .is_some_and(|variant| variant.payload.is_empty());
        (constant && unit).then_some(tag)
    }

    fn tags_decide_equality(&self, id: vn_tir::EnumId, lhs: &TirExpr, rhs: &TirExpr) -> bool {
        let payload_free = self
            .m
            .enums
            .get(id.0 as usize)
            .is_some_and(|info| info.variants.iter().all(|v| v.payload.is_empty()));
        payload_free || self.unit_variant_tag(lhs).is_some() || self.unit_variant_tag(rhs).is_some()
    }

    fn enum_tag(&self, e: TirExpr) -> TirExpr {
        let span = e.span;
        let kind = match self.unit_variant_tag(&e) {
            Some(tag) => TirExprKind::IntLit(tag),
            None => TirExprKind::Discriminant { value: Box::new(e) },
        };
        TirExpr {
            kind,
            ty: BackendTy::INT,
            res: Resolution::None,
            span,
        }
    }

    pub(super) fn coerce_binary_operands(
        &mut self,
        op: TirBinOp,
        lhs: TirExpr,
        rhs: TirExpr,
        checked: BackendTy,
    ) -> (TirExpr, TirExpr, BackendTy) {
        let is_cmp = op.is_comparison();
        let l = lhs.ty.non_nullable(self.tt).representation();
        let r = rhs.ty.non_nullable(self.tt).representation();

        if matches!(l, BackendTy::Dynamic(_)) || matches!(r, BackendTy::Dynamic(_)) {
            let ty = if is_cmp {
                BackendTy::Bool
            } else {
                BackendTy::Dynamic(DynReason::Declared)
            };
            return (lhs, rhs, ty);
        }
        if is_cmp && (l == BackendTy::Never || r == BackendTy::Never) {
            return (lhs, rhs, BackendTy::Bool);
        }

        if l == r {
            let ty = if is_cmp { BackendTy::Bool } else { l };
            return (self.cast_to(lhs, l), self.cast_to(rhs, r), ty);
        }

        let common = if op == TirBinOp::Add
            && (matches!(l, BackendTy::Str) || matches!(r, BackendTy::Str))
        {
            BackendTy::Str
        } else if l == BackendTy::FLOAT || r == BackendTy::FLOAT {
            BackendTy::FLOAT
        } else if matches!(l, BackendTy::Str) || matches!(r, BackendTy::Str) {
            BackendTy::Str
        } else {
            l
        };
        let (lhs, rhs) = if op == TirBinOp::Add && common == BackendTy::Str {
            (self.coerce_to_str(lhs), self.coerce_to_str(rhs))
        } else {
            (self.cast_to(lhs, common), self.cast_to(rhs, common))
        };
        let ty = if is_cmp { BackendTy::Bool } else { common };
        let _ = checked;
        (lhs, rhs, ty)
    }

    pub(super) fn lower_logical(
        &mut self,
        op: LogicalOp,
        left: ExprId,
        right: ExprId,
        ty: BackendTy,
        span: Span,
    ) -> TirExpr {
        let (cond, then_val, else_val) = match op {
            LogicalOp::And | LogicalOp::Or => {
                let l = self.lower_expr(left);
                let r = self.lower_expr(right);
                if l.ty == BackendTy::Bool && r.ty == BackendTy::Bool {
                    match op {
                        LogicalOp::And => (l, r, bool_lit(false)),
                        LogicalOp::Or | LogicalOp::Nullish => (l, bool_lit(true), r),
                    }
                } else {
                    let l = self.pin(l);
                    let holds = self.cast_to(l.clone(), BackendTy::Bool);
                    match op {
                        LogicalOp::And => (holds, r, l),
                        LogicalOp::Or | LogicalOp::Nullish => (holds, l, r),
                    }
                }
            }
            LogicalOp::Nullish => {
                let l = self.lower_expr(left);
                let l = self.pin(l);
                let r = self.lower_expr(right);
                let is_null = TirExpr {
                    kind: TirExprKind::Unary {
                        op: TirUnOp::IsNull,
                        operand: Box::new(l.clone()),
                    },
                    ty: BackendTy::Bool,
                    res: Resolution::None,
                    span,
                };
                (is_null, r, l)
            }
        };

        let (then_val, else_val) =
            if then_val.ty == else_val.ty || ty == BackendTy::Dynamic(DynReason::Union) {
                (then_val, else_val)
            } else {
                (self.cast_to(then_val, ty), self.cast_to(else_val, ty))
            };
        TirExpr {
            kind: TirExprKind::Select {
                cond: Box::new(cond),
                then_val: Box::new(then_val),
                else_val: Box::new(else_val),
            },
            ty,
            res: Resolution::None,
            span,
        }
    }

    pub(super) fn lower_unary(
        &mut self,
        op: vn_core::ast::operators::UnaryOp,
        operand: ExprId,
        ty: BackendTy,
        span: Span,
    ) -> TirExpr {
        let top = match op {
            vn_core::ast::operators::UnaryOp::Minus => TirUnOp::Neg,
            vn_core::ast::operators::UnaryOp::Not => TirUnOp::Not,
            vn_core::ast::operators::UnaryOp::BitNot => TirUnOp::BitNot,
            vn_core::ast::operators::UnaryOp::Plus => return self.lower_expr(operand),
        };

        let inner = self.lower_expr(operand);
        TirExpr {
            kind: TirExprKind::Unary {
                op: top,
                operand: Box::new(inner),
            },
            ty,
            res: Resolution::None,
            span,
        }
    }
}

fn unary_bool(op: TirUnOp, operand: TirExpr, span: Span) -> TirExpr {
    TirExpr {
        kind: TirExprKind::Unary {
            op,
            operand: Box::new(operand),
        },
        ty: BackendTy::Bool,
        res: Resolution::None,
        span,
    }
}
