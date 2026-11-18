use crate::hir::{HirUnOp, HirUpvalueSrc, LocalId};
use vn_tir::{BackendTy, TirUnOp};

pub(super) fn un_op(op: TirUnOp) -> HirUnOp {
    match op {
        TirUnOp::Neg => HirUnOp::Neg,
        TirUnOp::Not => HirUnOp::Not,
        TirUnOp::BitNot => HirUnOp::BitNot,
        TirUnOp::KindOf => HirUnOp::KindOf,
        TirUnOp::IsNull | TirUnOp::Present | TirUnOp::ToStr => {
            unreachable!("handled by lower_expr")
        }
    }
}

pub(super) fn upvalue_src(u: vn_tir::TirUpvalue) -> HirUpvalueSrc {
    match u {
        vn_tir::TirUpvalue::ParentLocal(i) => HirUpvalueSrc::ParentLocal(LocalId(i)),
        vn_tir::TirUpvalue::ParentParam(i) => HirUpvalueSrc::ParentParam(i),
        vn_tir::TirUpvalue::ParentUpvalue(i) => HirUpvalueSrc::ParentUpvalue(i),
    }
}

pub(super) fn is_free_fn_name(name: &str) -> bool {
    name.split("::").all(|part| {
        let mut cs = part.chars();
        matches!(cs.next(), Some(c) if c == '_' || c.is_ascii_alphabetic())
            && cs.all(|c| c == '_' || c.is_ascii_alphanumeric())
    })
}

pub(super) fn numeric_domain(bt: BackendTy) -> Option<vn_core::NumericDomain> {
    use vn_core::NumericDomain as D;
    Some(match bt {
        BackendTy::Literal(_) => return numeric_domain(bt.representation()),
        BackendTy::Num(vn_core::NumKind::Int(_)) => D::Int,
        BackendTy::Num(vn_core::NumKind::Float(_)) => D::Float,
        BackendTy::BigInt => D::BigInt,
        BackendTy::Decimal => D::Decimal,
        BackendTy::Bool
        | BackendTy::Char
        | BackendTy::Str
        | BackendTy::Bytes
        | BackendTy::Array(_)
        | BackendTy::Map(..)
        | BackendTy::Set(_)
        | BackendTy::Tuple(_)
        | BackendTy::Class(_)
        | BackendTy::Enum(_)
        | BackendTy::Fn(_)
        | BackendTy::Nullable(_)
        | BackendTy::Void
        | BackendTy::Never
        | BackendTy::Dynamic(_) => return None,
    })
}
