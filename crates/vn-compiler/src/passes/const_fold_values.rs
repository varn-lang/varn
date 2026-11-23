use super::const_fold::is_constant_kind;
use crate::ssa::ir::InstKind;
use vn_core::{add_int, mul_int, neg_int, pow_int, sub_int};
use vn_tir::BackendTy;

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

pub(super) fn fold_unary(
    op: vn_core::UnOp,
    operand: &InstKind,
    _ty: BackendTy,
) -> Option<InstKind> {
    match (op, operand) {
        (vn_core::UnOp::Neg, InstKind::ConstInt(x)) => neg_int(*x).map(InstKind::ConstInt),
        (vn_core::UnOp::Neg, InstKind::ConstFloat(x)) => Some(InstKind::ConstFloat(-x)),
        (vn_core::UnOp::Not, InstKind::ConstBool(x)) => Some(InstKind::ConstBool(!x)),
        (vn_core::UnOp::BitNot, InstKind::ConstInt(x)) => Some(InstKind::ConstInt(!x)),
        _ => None,
    }
}

pub(super) fn fold_binary(
    op: vn_core::BinOp,
    lhs: &InstKind,
    rhs: &InstKind,
    _ty: BackendTy,
) -> Option<InstKind> {
    use vn_core::BinOp::*;
    match (lhs, rhs) {
        (InstKind::ConstInt(x), InstKind::ConstInt(y)) => match op {
            Add => add_int(*x, *y).map(InstKind::ConstInt),
            Sub => sub_int(*x, *y).map(InstKind::ConstInt),
            Mul => mul_int(*x, *y).map(InstKind::ConstInt),
            Div => vn_core::div_int(*x, *y).ok().map(InstKind::ConstInt),
            Mod => vn_core::rem_int(*x, *y).ok().map(InstKind::ConstInt),

            Pow => {
                if *y >= 0 && *y <= 30 {
                    pow_int(*x, *y as u64).map(InstKind::ConstInt)
                } else {
                    None
                }
            }
            Eq => Some(InstKind::ConstBool(x == y)),
            Ne => Some(InstKind::ConstBool(x != y)),
            Lt => Some(InstKind::ConstBool(x < y)),
            Le => Some(InstKind::ConstBool(x <= y)),
            Gt => Some(InstKind::ConstBool(x > y)),
            Ge => Some(InstKind::ConstBool(x >= y)),
            BitAnd => Some(InstKind::ConstInt(x & y)),
            BitOr => Some(InstKind::ConstInt(x | y)),
            BitXor => Some(InstKind::ConstInt(x ^ y)),
            Shl => vn_core::shl_int(*x, *y).map(InstKind::ConstInt),
            Shr => vn_core::shr_int(*x, *y).map(InstKind::ConstInt),
            Ushr => vn_core::ushr_int(*x, *y).map(InstKind::ConstInt),
            In => None,
        },
        (InstKind::ConstFloat(x), InstKind::ConstFloat(y)) => match op {
            Add => Some(InstKind::ConstFloat(x + y)),
            Sub => Some(InstKind::ConstFloat(x - y)),
            Mul => Some(InstKind::ConstFloat(x * y)),
            Div => Some(InstKind::ConstFloat(x / y)),
            Mod => Some(InstKind::ConstFloat(x % y)),
            Pow => Some(InstKind::ConstFloat(x.powf(*y))),
            Eq => Some(InstKind::ConstBool(x == y)),
            Ne => Some(InstKind::ConstBool(x != y)),
            Lt => Some(InstKind::ConstBool(x < y)),
            Le => Some(InstKind::ConstBool(x <= y)),
            Gt => Some(InstKind::ConstBool(x > y)),
            Ge => Some(InstKind::ConstBool(x >= y)),
            BitAnd | BitOr | BitXor | Shl | Shr | Ushr | In => None,
        },
        (InstKind::ConstBool(x), InstKind::ConstBool(y)) => match op {
            Eq => Some(InstKind::ConstBool(x == y)),
            Ne => Some(InstKind::ConstBool(x != y)),
            Add | Sub | Mul | Div | Mod | Pow | Lt | Le | Gt | Ge | BitAnd | BitOr | BitXor
            | Shl | Shr | Ushr | In => None,
        },
        (InstKind::ConstStr(x), InstKind::ConstStr(y)) => match op {
            Eq => Some(InstKind::ConstBool(x == y)),
            Ne => Some(InstKind::ConstBool(x != y)),
            Add | Sub | Mul | Div | Mod | Pow | Lt | Le | Gt | Ge | BitAnd | BitOr | BitXor
            | Shl | Shr | Ushr | In => None,
        },
        (InstKind::ConstChar(x), InstKind::ConstChar(y)) => match op {
            Eq => Some(InstKind::ConstBool(x == y)),
            Ne => Some(InstKind::ConstBool(x != y)),
            Add | Sub | Mul | Div | Mod | Pow | Lt | Le | Gt | Ge | BitAnd | BitOr | BitXor
            | Shl | Shr | Ushr | In => None,
        },
        (InstKind::ConstDecimal(x), InstKind::ConstDecimal(y)) => match op {
            Add => Some(InstKind::ConstDecimal(x + y)),
            Sub => Some(InstKind::ConstDecimal(x - y)),
            Mul => Some(InstKind::ConstDecimal(vn_core::numeric_big::mul_decimal(
                x, y,
            ))),
            Div => vn_core::numeric_big::div_decimal(x, y)
                .ok()
                .map(InstKind::ConstDecimal),
            Eq => Some(InstKind::ConstBool(x == y)),
            Ne => Some(InstKind::ConstBool(x != y)),
            Lt => Some(InstKind::ConstBool(x < y)),
            Le => Some(InstKind::ConstBool(x <= y)),
            Gt => Some(InstKind::ConstBool(x > y)),
            Ge => Some(InstKind::ConstBool(x >= y)),
            Mod | Pow | BitAnd | BitOr | BitXor | Shl | Shr | Ushr | In => None,
        },
        (InstKind::ConstBigInt(x), InstKind::ConstBigInt(y)) => fold_bigint(op, x, y),
        (InstKind::ConstNull, InstKind::ConstNull) => match op {
            Eq => Some(InstKind::ConstBool(true)),
            Ne => Some(InstKind::ConstBool(false)),
            Add | Sub | Mul | Div | Mod | Pow | Lt | Le | Gt | Ge | BitAnd | BitOr | BitXor
            | Shl | Shr | Ushr | In => None,
        },
        (InstKind::ConstNull, other) | (other, InstKind::ConstNull) => {
            if is_constant_kind(other) {
                match op {
                    Eq => Some(InstKind::ConstBool(false)),
                    Ne => Some(InstKind::ConstBool(true)),
                    Add | Sub | Mul | Div | Mod | Pow | Lt | Le | Gt | Ge | BitAnd | BitOr
                    | BitXor | Shl | Shr | Ushr | In => None,
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
