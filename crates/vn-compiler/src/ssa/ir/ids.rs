use vn_tir::BackendTy;
pub use vn_types::ssa::SsaVar;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Value(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy)]
pub struct ValueDef {
    pub ty: BackendTy,
}
