use super::context::FnEmitter;
use std::sync::Arc;
use vn_core::ast::ExprId;
use vn_tir::{BackendTy, CheckKind, TirExpr, TirExprKind};

impl<'a> FnEmitter<'a> {
    pub(super) fn lower_cond(&mut self, e: ExprId) -> TirExpr {
        let lowered = self.lower_expr(e);
        match lowered.ty {
            BackendTy::Bool | BackendTy::Dynamic(_) => lowered,
            BackendTy::Literal(_)
            | BackendTy::Num(_)
            | BackendTy::Char
            | BackendTy::Str
            | BackendTy::Bytes
            | BackendTy::Decimal
            | BackendTy::BigInt
            | BackendTy::Array(_)
            | BackendTy::Map(..)
            | BackendTy::Set(_)
            | BackendTy::Tuple(_)
            | BackendTy::Class(_)
            | BackendTy::Enum(_)
            | BackendTy::Fn(_)
            | BackendTy::Nullable(_)
            | BackendTy::Void
            | BackendTy::Never => self.cast_to(lowered, BackendTy::Bool),
        }
    }

    pub(super) fn coerce_to_str(&mut self, e: TirExpr) -> TirExpr {
        if e.ty.representation() == BackendTy::Str {
            return self.cast_to(e, BackendTy::Str);
        }
        if let BackendTy::Class(id) = e.ty
            && self.m.classes.get(id.0 as usize).is_some_and(|info| {
                info.method_slot(vn_core::MemberKey::ToString.name())
                    .is_some()
            })
        {
            let span = e.span;
            return self.method_call(
                e,
                Arc::from(vn_core::MemberKey::ToString.name()),
                Vec::new(),
                BackendTy::Str,
                None,
                span,
            );
        }
        let span = e.span;
        TirExpr {
            kind: TirExprKind::Unary {
                op: vn_tir::TirUnOp::ToStr,
                operand: Box::new(e),
            },
            ty: BackendTy::Str,
            res: vn_tir::Resolution::None,
            span,
        }
    }

    pub(super) fn assert_type(&mut self, e: TirExpr, ty: BackendTy) -> TirExpr {
        let e = match self.struct_from_literal(e, ty) {
            Ok(built) => return built,
            Err(unchanged) => unchanged,
        };
        let numeric = matches!(ty.non_nullable(self.tt), BackendTy::Num(_));
        if !numeric && self.crosses_dynamic_boundary(&e, ty) {
            self.guard_dynamic(e, ty)
        } else {
            self.cast_to(e, ty)
        }
    }

    fn crosses_dynamic_boundary(&self, e: &TirExpr, ty: BackendTy) -> bool {
        let BackendTy::Dynamic(reason) = e.ty else {
            return false;
        };
        let untyped = untyped_reason(reason);
        match ty.non_nullable(self.tt) {
            BackendTy::Literal(_)
            | BackendTy::Str
            | BackendTy::Bool
            | BackendTy::Num(_)
            | BackendTy::Class(_) => true,
            BackendTy::Array(_)
            | BackendTy::Map(..)
            | BackendTy::Set(_)
            | BackendTy::Tuple(_)
            | BackendTy::Enum(_)
            | BackendTy::Fn(_)
            | BackendTy::Char
            | BackendTy::Bytes
            | BackendTy::Decimal
            | BackendTy::BigInt
            | BackendTy::Dynamic(vn_tir::DynReason::Callable | vn_tir::DynReason::Shape(_)) => {
                untyped
                    || matches!(
                        reason,
                        vn_tir::DynReason::Unrecorded
                            | vn_tir::DynReason::Generic
                            | vn_tir::DynReason::Interface(_)
                            | vn_tir::DynReason::TaskHandle(_)
                            | vn_tir::DynReason::ClassObject(_)
                            | vn_tir::DynReason::CoreMember
                            | vn_tir::DynReason::Spread
                    )
            }
            BackendTy::Dynamic(
                vn_tir::DynReason::HostBoundary
                | vn_tir::DynReason::Union
                | vn_tir::DynReason::IndexSignature
                | vn_tir::DynReason::Declared
                | vn_tir::DynReason::Awaitable(_)
                | vn_tir::DynReason::Yields(_)
                | vn_tir::DynReason::Range(_)
                | vn_tir::DynReason::Interface(_)
                | vn_tir::DynReason::TaskHandle(_)
                | vn_tir::DynReason::ClassObject(_)
                | vn_tir::DynReason::CoreMember
                | vn_tir::DynReason::Spread
                | vn_tir::DynReason::Unrecorded
                | vn_tir::DynReason::Generic
                | vn_tir::DynReason::Boxed
                | vn_tir::DynReason::Runtime,
            )
            | BackendTy::Nullable(_)
            | BackendTy::Void
            | BackendTy::Never => false,
        }
    }

    pub(super) fn guard_dynamic(&mut self, e: TirExpr, ty: BackendTy) -> TirExpr {
        self.guard_with(e, ty, CheckKind::Conform)
    }

    pub(super) fn decode_dynamic(&mut self, e: TirExpr, ty: BackendTy) -> TirExpr {
        self.guard_with(e, ty, CheckKind::Decode)
    }

    fn guard_with(&mut self, e: TirExpr, ty: BackendTy, kind: CheckKind) -> TirExpr {
        let e = if kind == CheckKind::Conform {
            match self.struct_from_literal(e, ty) {
                Ok(built) => return built,
                Err(unchanged) => unchanged,
            }
        } else {
            e
        };
        let decodes_shape = kind == CheckKind::Decode
            && matches!(
                ty.non_nullable(self.tt),
                BackendTy::Dynamic(vn_tir::DynReason::Interface(_) | vn_tir::DynReason::Shape(_))
            )
            && matches!(e.ty, BackendTy::Dynamic(_));
        if !decodes_shape && !self.crosses_dynamic_boundary(&e, ty) {
            return e;
        }
        let span = e.span;
        TirExpr {
            kind: TirExprKind::Checked {
                operand: Box::new(e),
                kind,
            },
            ty,
            res: vn_tir::Resolution::None,
            span,
        }
    }

    pub(super) fn cast_to(&mut self, e: TirExpr, ty: BackendTy) -> TirExpr {
        if e.ty == ty {
            return e;
        }
        let e = match self.struct_from_literal(e, ty) {
            Ok(built) => return built,
            Err(unchanged) => unchanged,
        };
        let span = e.span;
        TirExpr {
            kind: TirExprKind::Cast {
                operand: Box::new(e),
            },
            ty,
            res: vn_tir::Resolution::None,
            span,
        }
    }

    fn struct_from_literal(&mut self, e: TirExpr, ty: BackendTy) -> Result<TirExpr, TirExpr> {
        let BackendTy::Class(class) = ty else {
            return Err(e);
        };
        let Some(info) = self.m.classes.get(class.0 as usize) else {
            return Err(e);
        };
        if !info.layout.is_value {
            return Err(e);
        }
        let field_names: Vec<Arc<str>> = info.fields.iter().map(|f| f.name.clone()).collect();
        let field_tys: Vec<BackendTy> = info.fields.iter().map(|f| f.ty).collect();
        let TirExpr {
            kind,
            span,
            ty: lit_ty,
            res,
        } = e;
        let TirExprKind::ObjectLit { entries } = kind else {
            return Err(TirExpr {
                kind,
                span,
                ty: lit_ty,
                res,
            });
        };
        let mut named: Vec<(Arc<str>, TirExpr)> = Vec::with_capacity(entries.len());
        for entry in entries {
            match entry {
                vn_tir::TirObjectEntry::Field { name, value } => named.push((name, value)),
                vn_tir::TirObjectEntry::Spread(spread) => {
                    let mut back: Vec<vn_tir::TirObjectEntry> = named
                        .into_iter()
                        .map(|(name, value)| vn_tir::TirObjectEntry::Field { name, value })
                        .collect();
                    back.push(vn_tir::TirObjectEntry::Spread(spread));
                    return Err(TirExpr {
                        kind: TirExprKind::ObjectLit { entries: back },
                        span,
                        ty: lit_ty,
                        res,
                    });
                }
            }
        }
        let in_order = named.iter().map(|(n, _)| n).eq(field_names
            .iter()
            .filter(|f| named.iter().any(|(n, _)| n == *f)));
        if !in_order {
            named = named
                .into_iter()
                .map(|(name, value)| (name, self.pin(value)))
                .collect();
        }
        let args = field_names
            .iter()
            .zip(field_tys)
            .map(|(field, field_ty)| {
                let value = match named.iter().position(|(n, _)| n == field) {
                    Some(at) => named.swap_remove(at).1,
                    None => TirExpr {
                        kind: TirExprKind::NullLit,
                        ty: BackendTy::Nullable(self.tt.intern(field_ty)),
                        res: vn_tir::Resolution::None,
                        span,
                    },
                };
                vn_tir::TirArg::Expr(value)
            })
            .collect();
        Ok(TirExpr {
            kind: TirExprKind::New { class, args },
            ty,
            res: vn_tir::Resolution::None,
            span,
        })
    }
}

pub(super) fn untyped_reason(reason: vn_tir::DynReason) -> bool {
    match reason {
        vn_tir::DynReason::Declared | vn_tir::DynReason::HostBoundary => true,
        vn_tir::DynReason::Union
        | vn_tir::DynReason::IndexSignature
        | vn_tir::DynReason::Callable
        | vn_tir::DynReason::Awaitable(_)
        | vn_tir::DynReason::Yields(_)
        | vn_tir::DynReason::Range(_)
        | vn_tir::DynReason::Interface(_)
        | vn_tir::DynReason::TaskHandle(_)
        | vn_tir::DynReason::ClassObject(_)
        | vn_tir::DynReason::CoreMember
        | vn_tir::DynReason::Spread
        | vn_tir::DynReason::Shape(_)
        | vn_tir::DynReason::Unrecorded
        | vn_tir::DynReason::Generic
        | vn_tir::DynReason::Boxed
        | vn_tir::DynReason::Runtime => false,
    }
}
