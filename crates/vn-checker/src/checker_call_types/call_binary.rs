use super::CallTypeCtx;
use vn_core::ast::ExprId;
use vn_core::BinOp;
use vn_sem::types::numeric_ops::{integer_binary_type, numeric_binary_type};
use vn_sem::types::Type;

pub(super) fn infer_binary(
    c: &mut CallTypeCtx,
    left: ExprId,
    right: ExprId,
    op: BinOp,
) -> Option<Type> {
    let l = c.infer(left)?.apparent(c.table);
    let r = c.infer(right)?.apparent(c.table);

    match op {
        BinOp::Add => {
            if l == Type::Str || r == Type::Str {
                Some(Type::Str)
            } else {
                numeric_binary_type(&l, &r, c.table)
            }
        }
        BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
            numeric_binary_type(&l, &r, c.table)
        }
        BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge | BinOp::Eq | BinOp::Ne | BinOp::In => {
            Some(Type::Bool)
        }
        BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr | BinOp::Ushr => {
            integer_binary_type(&l, &r, c.table)
        }
    }
}
