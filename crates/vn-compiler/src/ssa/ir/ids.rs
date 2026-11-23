use vn_tir::BackendTy;
pub use vn_types::ssa::BlockId;
pub use vn_types::ssa::SsaVar;
pub use vn_types::ssa::ValueId as Value;

#[derive(Debug, Clone, Copy)]
pub struct ValueDef {
    pub ty: BackendTy,
}
