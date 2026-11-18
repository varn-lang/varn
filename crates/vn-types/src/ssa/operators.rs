use serde::{Deserialize, Serialize};

pub use vn_core::BinOp as DynBinOp;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SsaBinOp {
    IntAdd,
    IntSub,
    IntMul,
    IntDiv,
    IntMod,
    IntPow,
    IntEq,
    IntNe,
    IntLt,
    IntLe,
    IntGt,
    IntGe,
    IntAnd,
    IntOr,
    IntXor,
    IntShl,
    IntShr,
    IntUshr,

    FloatAdd,
    FloatSub,
    FloatMul,
    FloatDiv,
    FloatMod,
    FloatPow,
    FloatEq,
    FloatNe,
    FloatLt,
    FloatLe,
    FloatGt,
    FloatGe,

    StrConcat,

    StrEq,
    StrNe,

    BitsEq,
    BitsNe,

    Dyn(DynBinOp),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DynUnOp {
    Neg,

    Not,

    BitNot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SsaUnOp {
    NegInt,
    NegFloat,

    Not,
    BitNotInt,

    Dyn(DynUnOp),
}
