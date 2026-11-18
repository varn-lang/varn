use vn_tir::BackendTy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirUpvalueSrc {
    ParentLocal(LocalId),

    ParentParam(u32),

    ParentUpvalue(u32),
}

pub use vn_core::BinOp as HirBinOp;

pub(crate) fn binary_result_ty(op: HirBinOp, operand_ty: BackendTy) -> BackendTy {
    if op.produces_bool() {
        BackendTy::Bool
    } else {
        operand_ty
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HirUnOp {
    Neg,
    Not,
    BitNot,
    KindOf,
}
