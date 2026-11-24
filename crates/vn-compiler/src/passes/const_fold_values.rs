use super::const_fold::is_constant_kind;
use crate::ssa::ir::InstKind;
use vn_core::{add_int, mul_int, neg_int, pow_int, sub_int};

pub(super) fn fold_convert(conv: vn_core::ValueConv, operand: &InstKind) -> Option<InstKind> {
    use vn_core::ValueConv::*;
    match (conv, operand) {
        (IntToFloat, InstKind::ConstInt(n)) => Some(InstKind::ConstFloat(*n as f64)),
        (FloatToInt, InstKind::ConstFloat(f)) => vn_core::float_to_int(*f).map(InstKind::ConstInt),
        (BigIntToInt, InstKind::ConstBigInt(b)) => {
            let b: num_bigint::BigInt = b.parse().ok()?;
            i64::try_from(&b).ok().map(InstKind::ConstInt)
        }
        _ => None,
    }
}

pub(super) fn fold_unary(op: vn_types::ssa::SsaUnOp, operand: &InstKind) -> Option<InstKind> {
    use vn_types::ssa::SsaUnOp::*;
    match (op, operand) {
        (NegInt, InstKind::ConstInt(x)) => neg_int(*x).map(InstKind::ConstInt),
        (NegFloat, InstKind::ConstFloat(x)) => Some(InstKind::ConstFloat(-x)),
        (Not, InstKind::ConstBool(x)) => Some(InstKind::ConstBool(!x)),
        (BitNotInt, InstKind::ConstInt(x)) => Some(InstKind::ConstInt(!x)),
        (NegInt, _) | (NegFloat, _) | (Not, _) | (BitNotInt, _) | (Dyn(_), _) => None,
    }
}

pub(super) fn fold_binary(
    op: vn_types::ssa::SsaBinOp,
    lhs: &InstKind,
    rhs: &InstKind,
) -> Option<InstKind> {
    use vn_types::ssa::SsaBinOp::*;
    match (lhs, rhs) {
        (InstKind::ConstInt(x), InstKind::ConstInt(y)) => match op {
            IntAdd => add_int(*x, *y).map(InstKind::ConstInt),
            IntSub => sub_int(*x, *y).map(InstKind::ConstInt),
            IntMul => mul_int(*x, *y).map(InstKind::ConstInt),
            IntDiv => vn_core::div_int(*x, *y).ok().map(InstKind::ConstInt),
            IntMod => vn_core::rem_int(*x, *y).ok().map(InstKind::ConstInt),
            IntPow => {
                if *y >= 0 && *y <= 30 {
                    pow_int(*x, *y as u64).map(InstKind::ConstInt)
                } else {
                    None
                }
            }
            IntEq => Some(InstKind::ConstBool(x == y)),
            IntNe => Some(InstKind::ConstBool(x != y)),
            IntLt => Some(InstKind::ConstBool(x < y)),
            IntLe => Some(InstKind::ConstBool(x <= y)),
            IntGt => Some(InstKind::ConstBool(x > y)),
            IntGe => Some(InstKind::ConstBool(x >= y)),
            IntAnd => Some(InstKind::ConstInt(x & y)),
            IntOr => Some(InstKind::ConstInt(x | y)),
            IntXor => Some(InstKind::ConstInt(x ^ y)),
            IntShl => vn_core::shl_int(*x, *y).map(InstKind::ConstInt),
            IntShr => vn_core::shr_int(*x, *y).map(InstKind::ConstInt),
            IntUshr => vn_core::ushr_int(*x, *y).map(InstKind::ConstInt),
            FloatAdd | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | StrEq | StrNe | BitsEq
            | BitsNe | Dyn(_) => None,
        },
        (InstKind::ConstFloat(x), InstKind::ConstFloat(y)) => match op {
            FloatAdd => Some(InstKind::ConstFloat(x + y)),
            FloatSub => Some(InstKind::ConstFloat(x - y)),
            FloatMul => Some(InstKind::ConstFloat(x * y)),
            FloatDiv => Some(InstKind::ConstFloat(x / y)),
            FloatMod => Some(InstKind::ConstFloat(x % y)),
            FloatPow => Some(InstKind::ConstFloat(x.powf(*y))),
            FloatEq => Some(InstKind::ConstBool(x == y)),
            FloatNe => Some(InstKind::ConstBool(x != y)),
            FloatLt => Some(InstKind::ConstBool(x < y)),
            FloatLe => Some(InstKind::ConstBool(x <= y)),
            FloatGt => Some(InstKind::ConstBool(x > y)),
            FloatGe => Some(InstKind::ConstBool(x >= y)),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | StrConcat
            | StrEq | StrNe | BitsEq | BitsNe | Dyn(_) => None,
        },
        (InstKind::ConstBool(x), InstKind::ConstBool(y)) => match op {
            BitsEq => Some(InstKind::ConstBool(x == y)),
            BitsNe => Some(InstKind::ConstBool(x != y)),
            Dyn(vn_core::BinOp::Eq) => Some(InstKind::ConstBool(x == y)),
            Dyn(vn_core::BinOp::Ne) => Some(InstKind::ConstBool(x != y)),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | FloatAdd
            | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | StrEq | StrNe | Dyn(_) => None,
        },
        (InstKind::ConstStr(x), InstKind::ConstStr(y)) => match op {
            StrEq => Some(InstKind::ConstBool(x == y)),
            StrNe => Some(InstKind::ConstBool(x != y)),
            Dyn(vn_core::BinOp::Eq) => Some(InstKind::ConstBool(x == y)),
            Dyn(vn_core::BinOp::Ne) => Some(InstKind::ConstBool(x != y)),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | FloatAdd
            | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | BitsEq | BitsNe | Dyn(_) => None,
        },
        (InstKind::ConstChar(x), InstKind::ConstChar(y)) => match op {
            BitsEq => Some(InstKind::ConstBool(x == y)),
            BitsNe => Some(InstKind::ConstBool(x != y)),
            Dyn(vn_core::BinOp::Eq) => Some(InstKind::ConstBool(x == y)),
            Dyn(vn_core::BinOp::Ne) => Some(InstKind::ConstBool(x != y)),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | FloatAdd
            | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | StrEq | StrNe | Dyn(_) => None,
        },
        (InstKind::ConstDecimal(x), InstKind::ConstDecimal(y)) => match op {
            Dyn(vn_core::BinOp::Add) => Some(InstKind::ConstDecimal(x + y)),
            Dyn(vn_core::BinOp::Sub) => Some(InstKind::ConstDecimal(x - y)),
            Dyn(vn_core::BinOp::Mul) => Some(InstKind::ConstDecimal(
                vn_core::numeric_big::mul_decimal(x, y),
            )),
            Dyn(vn_core::BinOp::Div) => vn_core::numeric_big::div_decimal(x, y)
                .ok()
                .map(InstKind::ConstDecimal),
            Dyn(vn_core::BinOp::Eq) => Some(InstKind::ConstBool(x == y)),
            Dyn(vn_core::BinOp::Ne) => Some(InstKind::ConstBool(x != y)),
            Dyn(vn_core::BinOp::Lt) => Some(InstKind::ConstBool(x < y)),
            Dyn(vn_core::BinOp::Le) => Some(InstKind::ConstBool(x <= y)),
            Dyn(vn_core::BinOp::Gt) => Some(InstKind::ConstBool(x > y)),
            Dyn(vn_core::BinOp::Ge) => Some(InstKind::ConstBool(x >= y)),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | FloatAdd
            | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | StrEq | StrNe | BitsEq
            | BitsNe | Dyn(_) => None,
        },
        (InstKind::ConstBigInt(x), InstKind::ConstBigInt(y)) => match op {
            Dyn(inner) => fold_bigint(inner, x, y),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | FloatAdd
            | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | StrEq | StrNe | BitsEq
            | BitsNe => None,
        },
        (InstKind::ConstNull, InstKind::ConstNull) => match op {
            BitsEq => Some(InstKind::ConstBool(true)),
            BitsNe => Some(InstKind::ConstBool(false)),
            Dyn(vn_core::BinOp::Eq) => Some(InstKind::ConstBool(true)),
            Dyn(vn_core::BinOp::Ne) => Some(InstKind::ConstBool(false)),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | FloatAdd
            | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | StrEq | StrNe | Dyn(_) => None,
        },
        (InstKind::ConstNull, other) | (other, InstKind::ConstNull) => {
            if is_constant_kind(other) {
                match op {
                    BitsEq | Dyn(vn_core::BinOp::Eq) => Some(InstKind::ConstBool(false)),
                    BitsNe | Dyn(vn_core::BinOp::Ne) => Some(InstKind::ConstBool(true)),
                    IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt
                    | IntLe | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr
                    | IntUshr | FloatAdd | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow
                    | FloatEq | FloatNe | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat
                    | StrEq | StrNe | Dyn(_) => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(super) fn fold_bigint(op: vn_core::BinOp, x: &str, y: &str) -> Option<InstKind> {
    use vn_core::BinOp::*;
    let (a, b): (num_bigint::BigInt, num_bigint::BigInt) = (x.parse().ok()?, y.parse().ok()?);
    let big = |v: num_bigint::BigInt| Some(InstKind::ConstBigInt(v.to_string().into()));
    match op {
        Add => big(a + b),
        Sub => big(a - b),
        Mul => big(a * b),
        Div => vn_core::numeric_big::div_big(&a, &b).ok().and_then(big),
        Mod => vn_core::numeric_big::rem_big(&a, &b).ok().and_then(big),
        Eq => Some(InstKind::ConstBool(a == b)),
        Ne => Some(InstKind::ConstBool(a != b)),
        Lt => Some(InstKind::ConstBool(a < b)),
        Le => Some(InstKind::ConstBool(a <= b)),
        Gt => Some(InstKind::ConstBool(a > b)),
        Ge => Some(InstKind::ConstBool(a >= b)),
        Pow | BitAnd | BitOr | BitXor | Shl | Shr | Ushr | In => None,
    }
}
