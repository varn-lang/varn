use serde::{Deserialize, Serialize};

pub use vn_core::BinOp as DynBinOp;

use crate::register_meta::SlotKind;
use vn_core::{BinOp, OpCode, UnOp};

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

pub fn select_bin(op: BinOp, ty: SlotKind, lhs: SlotKind, rhs: SlotKind) -> SsaBinOp {
    if matches!(op, BinOp::Add) && (matches!(lhs, SlotKind::Str) || matches!(rhs, SlotKind::Str)) {
        return SsaBinOp::StrConcat;
    }
    match ty {
        SlotKind::Int => match op {
            BinOp::Add => SsaBinOp::IntAdd,
            BinOp::Sub => SsaBinOp::IntSub,
            BinOp::Mul => SsaBinOp::IntMul,
            BinOp::Div => SsaBinOp::IntDiv,
            BinOp::Mod => SsaBinOp::IntMod,
            BinOp::Pow => SsaBinOp::IntPow,
            BinOp::Eq => SsaBinOp::IntEq,
            BinOp::Ne => SsaBinOp::IntNe,
            BinOp::Lt => SsaBinOp::IntLt,
            BinOp::Le => SsaBinOp::IntLe,
            BinOp::Gt => SsaBinOp::IntGt,
            BinOp::Ge => SsaBinOp::IntGe,
            BinOp::BitAnd => SsaBinOp::IntAnd,
            BinOp::BitOr => SsaBinOp::IntOr,
            BinOp::BitXor => SsaBinOp::IntXor,
            BinOp::Shl => SsaBinOp::IntShl,
            BinOp::Shr => SsaBinOp::IntShr,
            BinOp::Ushr => SsaBinOp::IntUshr,
            BinOp::In => SsaBinOp::Dyn(BinOp::In),
        },
        SlotKind::Float => match op {
            BinOp::Add => SsaBinOp::FloatAdd,
            BinOp::Sub => SsaBinOp::FloatSub,
            BinOp::Mul => SsaBinOp::FloatMul,
            BinOp::Div => SsaBinOp::FloatDiv,
            BinOp::Mod => SsaBinOp::FloatMod,
            BinOp::Pow => SsaBinOp::FloatPow,
            BinOp::Eq => SsaBinOp::FloatEq,
            BinOp::Ne => SsaBinOp::FloatNe,
            BinOp::Lt => SsaBinOp::FloatLt,
            BinOp::Le => SsaBinOp::FloatLe,
            BinOp::Gt => SsaBinOp::FloatGt,
            BinOp::Ge => SsaBinOp::FloatGe,
            BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::Shl
            | BinOp::Shr
            | BinOp::Ushr
            | BinOp::In => SsaBinOp::Dyn(op),
        },
        SlotKind::Str => match op {
            BinOp::Eq => SsaBinOp::StrEq,
            BinOp::Ne => SsaBinOp::StrNe,
            BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::Mod
            | BinOp::Pow
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Gt
            | BinOp::Ge
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::Shl
            | BinOp::Shr
            | BinOp::Ushr
            | BinOp::In => SsaBinOp::Dyn(op),
        },
        SlotKind::Bool => match op {
            BinOp::Eq => SsaBinOp::BitsEq,
            BinOp::Ne => SsaBinOp::BitsNe,
            BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::Mod
            | BinOp::Pow
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Gt
            | BinOp::Ge
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::Shl
            | BinOp::Shr
            | BinOp::Ushr
            | BinOp::In => SsaBinOp::Dyn(op),
        },
        SlotKind::Char => match op {
            BinOp::Eq => SsaBinOp::BitsEq,
            BinOp::Ne => SsaBinOp::BitsNe,
            BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::Mod
            | BinOp::Pow
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Gt
            | BinOp::Ge
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::Shl
            | BinOp::Shr
            | BinOp::Ushr
            | BinOp::In => SsaBinOp::Dyn(op),
        },
        SlotKind::Ref | SlotKind::Dynamic => SsaBinOp::Dyn(op),
    }
}

pub fn select_un(op: UnOp, operand: SlotKind) -> Option<SsaUnOp> {
    match op {
        UnOp::Neg => match operand {
            SlotKind::Int => Some(SsaUnOp::NegInt),
            SlotKind::Float => Some(SsaUnOp::NegFloat),
            SlotKind::Bool | SlotKind::Char | SlotKind::Str | SlotKind::Ref | SlotKind::Dynamic => {
                Some(SsaUnOp::Dyn(DynUnOp::Neg))
            }
        },
        UnOp::Not => match operand {
            SlotKind::Bool => Some(SsaUnOp::Not),
            SlotKind::Int
            | SlotKind::Float
            | SlotKind::Char
            | SlotKind::Str
            | SlotKind::Ref
            | SlotKind::Dynamic => Some(SsaUnOp::Dyn(DynUnOp::Not)),
        },
        UnOp::BitNot => match operand {
            SlotKind::Int => Some(SsaUnOp::BitNotInt),
            SlotKind::Float
            | SlotKind::Bool
            | SlotKind::Char
            | SlotKind::Str
            | SlotKind::Ref
            | SlotKind::Dynamic => Some(SsaUnOp::Dyn(DynUnOp::BitNot)),
        },
        UnOp::KindOf => None,
    }
}

pub fn opcode_of_bin(op: SsaBinOp) -> OpCode {
    match op {
        SsaBinOp::IntAdd => OpCode::AddInt,
        SsaBinOp::IntSub => OpCode::SubInt,
        SsaBinOp::IntMul => OpCode::MulInt,
        SsaBinOp::IntDiv => OpCode::DivInt,
        SsaBinOp::IntMod => OpCode::ModInt,
        SsaBinOp::IntPow => OpCode::PowInt,
        SsaBinOp::IntEq => OpCode::EqInt,
        SsaBinOp::IntNe => OpCode::NeqInt,
        SsaBinOp::IntLt => OpCode::LtInt,
        SsaBinOp::IntLe => OpCode::LteInt,
        SsaBinOp::IntGt => OpCode::GtInt,
        SsaBinOp::IntGe => OpCode::GteInt,
        SsaBinOp::IntAnd => OpCode::BitAnd,
        SsaBinOp::IntOr => OpCode::BitOr,
        SsaBinOp::IntXor => OpCode::BitXor,
        SsaBinOp::IntShl => OpCode::Shl,
        SsaBinOp::IntShr => OpCode::Shr,
        SsaBinOp::IntUshr => OpCode::Ushr,
        SsaBinOp::FloatAdd => OpCode::AddFloat,
        SsaBinOp::FloatSub => OpCode::SubFloat,
        SsaBinOp::FloatMul => OpCode::MulFloat,
        SsaBinOp::FloatDiv => OpCode::DivFloat,
        SsaBinOp::FloatMod => OpCode::ModFloat,
        SsaBinOp::FloatPow => OpCode::PowFloat,
        SsaBinOp::FloatEq => OpCode::EqFloat,
        SsaBinOp::FloatNe => OpCode::NeqFloat,
        SsaBinOp::FloatLt => OpCode::LtFloat,
        SsaBinOp::FloatLe => OpCode::LteFloat,
        SsaBinOp::FloatGt => OpCode::GtFloat,
        SsaBinOp::FloatGe => OpCode::GteFloat,
        SsaBinOp::StrConcat => OpCode::StrConcat,
        SsaBinOp::StrEq => OpCode::EqStr,
        SsaBinOp::StrNe => OpCode::NeqStr,
        SsaBinOp::BitsEq => OpCode::EqBool,
        SsaBinOp::BitsNe => OpCode::NeqBool,
        SsaBinOp::Dyn(op) => match op {
            BinOp::Add => OpCode::Add,
            BinOp::Sub => OpCode::Sub,
            BinOp::Mul => OpCode::Mul,
            BinOp::Div => OpCode::Div,
            BinOp::Mod => OpCode::Mod,
            BinOp::Pow => OpCode::Pow,
            BinOp::Eq => OpCode::Eq,
            BinOp::Ne => OpCode::Neq,
            BinOp::Lt => OpCode::Lt,
            BinOp::Le => OpCode::Lte,
            BinOp::Gt => OpCode::Gt,
            BinOp::Ge => OpCode::Gte,
            BinOp::BitAnd => OpCode::BitAnd,
            BinOp::BitOr => OpCode::BitOr,
            BinOp::BitXor => OpCode::BitXor,
            BinOp::Shl => OpCode::Shl,
            BinOp::Shr => OpCode::Shr,
            BinOp::Ushr => OpCode::Ushr,
            BinOp::In => OpCode::In,
        },
    }
}
