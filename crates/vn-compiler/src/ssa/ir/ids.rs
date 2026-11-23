use vn_tir::BackendTy;
use vn_tir::LocalId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarId {
    Param(u32),
    Local(LocalId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Value(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy)]
pub struct ValueDef {
    pub ty: BackendTy,
}
