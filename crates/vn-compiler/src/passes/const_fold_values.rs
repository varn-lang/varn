use crate::ssa::ir::const_ty;
use vn_core::{add_int, mul_int, neg_int, pow_int, sub_int};
use vn_types::ssa::SsaOp;

pub(super) fn fold_convert(conv: vn_core::ValueConv, operand: &SsaOp) -> Option<SsaOp> {
    use vn_core::ValueConv::*;
    match (conv, operand) {
        (IntToFloat, SsaOp::ConstInt(n)) => Some(SsaOp::ConstFloat(*n as f64)),
        (FloatToInt, SsaOp::ConstFloat(f)) => vn_core::float_to_int(*f).map(SsaOp::ConstInt),
        (BigIntToInt, SsaOp::ConstBigInt(b)) => {
            let b: num_bigint::BigInt = b.parse().ok()?;
            i64::try_from(&b).ok().map(SsaOp::ConstInt)
        }
        _ => None,
    }
}

pub(super) fn fold_unary(op: vn_types::ssa::SsaUnOp, operand: &SsaOp) -> Option<SsaOp> {
    use vn_types::ssa::SsaUnOp::*;
    match (op, operand) {
        (NegInt, SsaOp::ConstInt(x)) => neg_int(*x).map(SsaOp::ConstInt),
        (NegFloat, SsaOp::ConstFloat(x)) => Some(SsaOp::ConstFloat(-x)),
        (Not, SsaOp::ConstBool(x)) => Some(SsaOp::ConstBool(!x)),
        (BitNotInt, SsaOp::ConstInt(x)) => Some(SsaOp::ConstInt(!x)),
        (NegInt, _) | (NegFloat, _) | (Not, _) | (BitNotInt, _) | (Dyn(_), _) => None,
    }
}

pub(super) fn fold_binary(op: vn_types::ssa::SsaBinOp, lhs: &SsaOp, rhs: &SsaOp) -> Option<SsaOp> {
    use vn_types::ssa::SsaBinOp::*;
    match (lhs, rhs) {
        (SsaOp::ConstInt(x), SsaOp::ConstInt(y)) => match op {
            IntAdd => add_int(*x, *y).map(SsaOp::ConstInt),
            IntSub => sub_int(*x, *y).map(SsaOp::ConstInt),
            IntMul => mul_int(*x, *y).map(SsaOp::ConstInt),
            IntDiv => vn_core::div_int(*x, *y).ok().map(SsaOp::ConstInt),
            IntMod => vn_core::rem_int(*x, *y).ok().map(SsaOp::ConstInt),
            IntPow => {
                if *y >= 0 && *y <= 30 {
                    pow_int(*x, *y as u64).map(SsaOp::ConstInt)
                } else {
                    None
                }
            }
            IntEq => Some(SsaOp::ConstBool(x == y)),
            IntNe => Some(SsaOp::ConstBool(x != y)),
            IntLt => Some(SsaOp::ConstBool(x < y)),
            IntLe => Some(SsaOp::ConstBool(x <= y)),
            IntGt => Some(SsaOp::ConstBool(x > y)),
            IntGe => Some(SsaOp::ConstBool(x >= y)),
            IntAnd => Some(SsaOp::ConstInt(x & y)),
            IntOr => Some(SsaOp::ConstInt(x | y)),
            IntXor => Some(SsaOp::ConstInt(x ^ y)),
            IntShl => vn_core::shl_int(*x, *y).map(SsaOp::ConstInt),
            IntShr => vn_core::shr_int(*x, *y).map(SsaOp::ConstInt),
            IntUshr => vn_core::ushr_int(*x, *y).map(SsaOp::ConstInt),
            FloatAdd | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | StrEq | StrNe | BitsEq
            | BitsNe | Dyn(_) => None,
        },
        (SsaOp::ConstFloat(x), SsaOp::ConstFloat(y)) => match op {
            FloatAdd => Some(SsaOp::ConstFloat(x + y)),
            FloatSub => Some(SsaOp::ConstFloat(x - y)),
            FloatMul => Some(SsaOp::ConstFloat(x * y)),
            FloatDiv => Some(SsaOp::ConstFloat(x / y)),
            FloatMod => Some(SsaOp::ConstFloat(x % y)),
            FloatPow => Some(SsaOp::ConstFloat(x.powf(*y))),
            FloatEq => Some(SsaOp::ConstBool(x == y)),
            FloatNe => Some(SsaOp::ConstBool(x != y)),
            FloatLt => Some(SsaOp::ConstBool(x < y)),
            FloatLe => Some(SsaOp::ConstBool(x <= y)),
            FloatGt => Some(SsaOp::ConstBool(x > y)),
            FloatGe => Some(SsaOp::ConstBool(x >= y)),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | StrConcat
            | StrEq | StrNe | BitsEq | BitsNe | Dyn(_) => None,
        },
        (SsaOp::ConstBool(x), SsaOp::ConstBool(y)) => match op {
            BitsEq => Some(SsaOp::ConstBool(x == y)),
            BitsNe => Some(SsaOp::ConstBool(x != y)),
            Dyn(vn_core::BinOp::Eq) => Some(SsaOp::ConstBool(x == y)),
            Dyn(vn_core::BinOp::Ne) => Some(SsaOp::ConstBool(x != y)),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | FloatAdd
            | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | StrEq | StrNe | Dyn(_) => None,
        },
        (SsaOp::ConstStr(x), SsaOp::ConstStr(y)) => match op {
            StrEq => Some(SsaOp::ConstBool(x == y)),
            StrNe => Some(SsaOp::ConstBool(x != y)),
            Dyn(vn_core::BinOp::Eq) => Some(SsaOp::ConstBool(x == y)),
            Dyn(vn_core::BinOp::Ne) => Some(SsaOp::ConstBool(x != y)),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | FloatAdd
            | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | BitsEq | BitsNe | Dyn(_) => None,
        },
        (SsaOp::ConstChar(x), SsaOp::ConstChar(y)) => match op {
            BitsEq => Some(SsaOp::ConstBool(x == y)),
            BitsNe => Some(SsaOp::ConstBool(x != y)),
            Dyn(vn_core::BinOp::Eq) => Some(SsaOp::ConstBool(x == y)),
            Dyn(vn_core::BinOp::Ne) => Some(SsaOp::ConstBool(x != y)),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | FloatAdd
            | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | StrEq | StrNe | Dyn(_) => None,
        },
        (SsaOp::ConstDecimal(x), SsaOp::ConstDecimal(y)) => match op {
            Dyn(vn_core::BinOp::Add) => Some(SsaOp::ConstDecimal(x + y)),
            Dyn(vn_core::BinOp::Sub) => Some(SsaOp::ConstDecimal(x - y)),
            Dyn(vn_core::BinOp::Mul) => {
                Some(SsaOp::ConstDecimal(vn_core::numeric_big::mul_decimal(x, y)))
            }
            Dyn(vn_core::BinOp::Div) => vn_core::numeric_big::div_decimal(x, y)
                .ok()
                .map(SsaOp::ConstDecimal),
            Dyn(vn_core::BinOp::Eq) => Some(SsaOp::ConstBool(x == y)),
            Dyn(vn_core::BinOp::Ne) => Some(SsaOp::ConstBool(x != y)),
            Dyn(vn_core::BinOp::Lt) => Some(SsaOp::ConstBool(x < y)),
            Dyn(vn_core::BinOp::Le) => Some(SsaOp::ConstBool(x <= y)),
            Dyn(vn_core::BinOp::Gt) => Some(SsaOp::ConstBool(x > y)),
            Dyn(vn_core::BinOp::Ge) => Some(SsaOp::ConstBool(x >= y)),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | FloatAdd
            | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | StrEq | StrNe | BitsEq
            | BitsNe | Dyn(_) => None,
        },
        (SsaOp::ConstBigInt(x), SsaOp::ConstBigInt(y)) => match op {
            Dyn(inner) => fold_bigint(inner, x, y),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | FloatAdd
            | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | StrEq | StrNe | BitsEq
            | BitsNe => None,
        },
        (SsaOp::ConstNull, SsaOp::ConstNull) => match op {
            BitsEq => Some(SsaOp::ConstBool(true)),
            BitsNe => Some(SsaOp::ConstBool(false)),
            Dyn(vn_core::BinOp::Eq) => Some(SsaOp::ConstBool(true)),
            Dyn(vn_core::BinOp::Ne) => Some(SsaOp::ConstBool(false)),
            IntAdd | IntSub | IntMul | IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe
            | IntGt | IntGe | IntAnd | IntOr | IntXor | IntShl | IntShr | IntUshr | FloatAdd
            | FloatSub | FloatMul | FloatDiv | FloatMod | FloatPow | FloatEq | FloatNe
            | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat | StrEq | StrNe | Dyn(_) => None,
        },
        (SsaOp::ConstNull, other) | (other, SsaOp::ConstNull) => {
            if const_ty(other).is_some() {
                match op {
                    BitsEq | Dyn(vn_core::BinOp::Eq) => Some(SsaOp::ConstBool(false)),
                    BitsNe | Dyn(vn_core::BinOp::Ne) => Some(SsaOp::ConstBool(true)),
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

pub(super) fn fold_bigint(op: vn_core::BinOp, x: &str, y: &str) -> Option<SsaOp> {
    use vn_core::BinOp::*;
    let (a, b): (num_bigint::BigInt, num_bigint::BigInt) = (x.parse().ok()?, y.parse().ok()?);
    let big = |v: num_bigint::BigInt| Some(SsaOp::ConstBigInt(v.to_string().into()));
    match op {
        Add => big(a + b),
        Sub => big(a - b),
        Mul => big(a * b),
        Div => vn_core::numeric_big::div_big(&a, &b).ok().and_then(big),
        Mod => vn_core::numeric_big::rem_big(&a, &b).ok().and_then(big),
        Eq => Some(SsaOp::ConstBool(a == b)),
        Ne => Some(SsaOp::ConstBool(a != b)),
        Lt => Some(SsaOp::ConstBool(a < b)),
        Le => Some(SsaOp::ConstBool(a <= b)),
        Gt => Some(SsaOp::ConstBool(a > b)),
        Ge => Some(SsaOp::ConstBool(a >= b)),
        Pow | BitAnd | BitOr | BitXor | Shl | Shr | Ushr | In => None,
    }
}
