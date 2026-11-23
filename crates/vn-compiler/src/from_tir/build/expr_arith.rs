use super::context::{Builder, Result};
use crate::ssa::ir::{InstKind, Value};
use vn_tir::{BackendTy, DynReason};
use vn_tir::{TirBinOp, TirExpr, TirUnOp};

impl<'m> Builder<'m> {
    pub(super) fn lower_binary(
        &mut self,
        op: TirBinOp,
        lhs: &TirExpr,
        rhs: &TirExpr,
        ty: BackendTy,
    ) -> Result<Value> {
        let mut l = self.lower_expr(lhs)?;
        let mut r = self.lower_expr(rhs)?;
        let is_cmp = op.is_comparison();
        if !is_cmp && matches!(ty, BackendTy::Num(_) | BackendTy::Str) {
            l = self.coerce(l, ty);
            r = self.coerce(r, ty);
        }
        let (lt, rt) = (self.value_ty(l), self.value_ty(r));
        let unsigned_64 = BackendTy::Num(vn_core::NumKind::Int(vn_core::IntKind::U64));
        if lt == unsigned_64 || rt == unsigned_64 {
            return Ok(self.lower_u64_binary(op, l, r, unsigned_64));
        }
        let op_ty = if lt == rt
            && matches!(
                lt,
                BackendTy::Num(_) | BackendTy::Bool | BackendTy::Str | BackendTy::Char
            ) {
            lt
        } else {
            BackendTy::Dynamic(DynReason::Boxed)
        };
        let result_ty = if is_cmp && op_ty != BackendTy::Dynamic(DynReason::Boxed) {
            BackendTy::Bool
        } else if op_ty != BackendTy::Dynamic(DynReason::Boxed) {
            op_ty
        } else {
            ty
        };
        let result = self.emit(
            InstKind::Binary {
                op,
                lhs: l,
                rhs: r,
                ty: op_ty,
            },
            result_ty,
        );
        Ok(if is_cmp {
            result
        } else {
            self.narrow_to(result, result_ty)
        })
    }

    fn u64_convert(&mut self, v: Value, conv: vn_core::ValueConv) -> Value {
        let ty = crate::ssa::verify::convert_result_ty(conv);
        self.emit(InstKind::Convert { operand: v, conv }, ty)
    }

    fn u64_binary(&mut self, op: TirBinOp, l: Value, r: Value, ty: BackendTy) -> Value {
        self.emit(
            InstKind::Binary {
                op,
                lhs: l,
                rhs: r,
                ty,
            },
            ty,
        )
    }

    fn lower_u64_binary(&mut self, op: TirBinOp, l: Value, r: Value, ty: BackendTy) -> Value {
        use vn_core::ValueConv as C;
        let boxed = BackendTy::Dynamic(DynReason::Boxed);
        match op {
            TirBinOp::BitAnd
            | TirBinOp::BitOr
            | TirBinOp::BitXor
            | TirBinOp::Shr
            | TirBinOp::Ushr => {
                let (lb, rb) = (
                    self.u64_convert(l, C::U64ToBits),
                    self.u64_convert(r, C::U64ToBits),
                );
                let op = if op == TirBinOp::Shr {
                    TirBinOp::Ushr
                } else {
                    op
                };
                let bits = self.u64_binary(op, lb, rb, BackendTy::INT);
                self.u64_convert(bits, C::U64FromBits)
            }
            TirBinOp::Shl => {
                let (lb, rb) = (
                    self.u64_convert(l, C::U64ToBigInt),
                    self.u64_convert(r, C::U64ToBigInt),
                );
                let two = self.emit(InstKind::ConstBigInt(std::sync::Arc::from("2")), boxed);
                let scale = self.u64_binary(TirBinOp::Pow, two, rb, boxed);
                let product = self.u64_binary(TirBinOp::Mul, lb, scale, boxed);
                self.narrow_to(product, ty)
            }
            TirBinOp::Eq
            | TirBinOp::Ne
            | TirBinOp::Lt
            | TirBinOp::Le
            | TirBinOp::Gt
            | TirBinOp::Ge => {
                let (lb, rb) = (
                    self.u64_convert(l, C::U64ToBigInt),
                    self.u64_convert(r, C::U64ToBigInt),
                );
                self.emit(
                    InstKind::Binary {
                        op,
                        lhs: lb,
                        rhs: rb,
                        ty: boxed,
                    },
                    BackendTy::Bool,
                )
            }
            TirBinOp::Add
            | TirBinOp::Sub
            | TirBinOp::Mul
            | TirBinOp::Div
            | TirBinOp::Mod
            | TirBinOp::Pow
            | TirBinOp::In => {
                let (lb, rb) = (
                    self.u64_convert(l, C::U64ToBigInt),
                    self.u64_convert(r, C::U64ToBigInt),
                );
                let result = self.u64_binary(op, lb, rb, boxed);
                self.narrow_to(result, ty)
            }
        }
    }

    pub(super) fn narrow_to(&mut self, v: Value, ty: BackendTy) -> Value {
        let BackendTy::Num(kind) = ty else {
            return v;
        };
        match vn_core::ValueConv::narrow(kind) {
            Some(conv) => self.emit(InstKind::Convert { operand: v, conv }, ty),
            None if self.value_ty(v) != ty => self.emit(InstKind::Cast { operand: v, ty }, ty),
            None => v,
        }
    }

    pub(super) fn lower_unary(
        &mut self,
        op: TirUnOp,
        operand: &TirExpr,
        ty: BackendTy,
    ) -> Result<Value> {
        let v = self.lower_expr(operand)?;
        match op {
            TirUnOp::IsNull => match self.value_ty(v) {
                BackendTy::Literal(_) | BackendTy::Num(_) | BackendTy::Bool => {
                    Ok(self.emit(InstKind::ConstBool(false), BackendTy::Bool))
                }
                BackendTy::Str
                | BackendTy::Char
                | BackendTy::Bytes
                | BackendTy::Enum(_)
                | BackendTy::Fn(_)
                | BackendTy::Dynamic(_)
                | BackendTy::Decimal
                | BackendTy::BigInt
                | BackendTy::Void
                | BackendTy::Never
                | BackendTy::Array(_)
                | BackendTy::Map(..)
                | BackendTy::Set(_)
                | BackendTy::Tuple(_)
                | BackendTy::Class(_)
                | BackendTy::Nullable(_) => {
                    Ok(self.emit(InstKind::IsNull { operand: v }, BackendTy::Bool))
                }
            },
            TirUnOp::Present => {
                self.emit_effect(InstKind::AssertNotNull { operand: v });
                Ok(v)
            }
            TirUnOp::ToStr => Ok(self.emit(InstKind::ToString { operand: v }, BackendTy::Str)),
            TirUnOp::Scalar(vn_core::UnOp::BitNot)
                if ty == BackendTy::Num(vn_core::NumKind::Int(vn_core::IntKind::U64)) =>
            {
                let bits = self.u64_convert(v, vn_core::ValueConv::U64ToBits);
                let inverted = self.emit(
                    InstKind::Unary {
                        op: vn_core::UnOp::BitNot,
                        operand: bits,
                        ty: BackendTy::INT,
                    },
                    BackendTy::INT,
                );
                Ok(self.u64_convert(inverted, vn_core::ValueConv::U64FromBits))
            }
            TirUnOp::Scalar(vn_core::UnOp::BitNot) if matches!(ty, BackendTy::Num(vn_core::NumKind::Int(k)) if !k.is_signed() && k != vn_core::IntKind::I64) =>
            {
                let BackendTy::Num(vn_core::NumKind::Int(kind)) = ty else {
                    return Ok(v);
                };
                let mask = self.emit(InstKind::ConstInt(kind.max() as i64), ty);
                Ok(self.emit(
                    InstKind::Binary {
                        op: TirBinOp::BitXor,
                        lhs: v,
                        rhs: mask,
                        ty,
                    },
                    ty,
                ))
            }
            TirUnOp::Scalar(op) => {
                let result = self.emit(InstKind::Unary { op, operand: v, ty }, ty);
                Ok(
                    if matches!(op, vn_core::UnOp::Neg | vn_core::UnOp::BitNot) {
                        self.narrow_to(result, ty)
                    } else {
                        result
                    },
                )
            }
        }
    }

    fn static_truthiness(&mut self, v: Value) -> Option<Value> {
        match self.value_ty(v) {
            BackendTy::Bool => Some(v),
            BackendTy::Literal(_) => None,
            BackendTy::Num(_)
            | BackendTy::Str
            | BackendTy::Array(_)
            | BackendTy::Map(..)
            | BackendTy::Set(_)
            | BackendTy::Tuple(_)
            | BackendTy::Class(_) => Some(self.emit(InstKind::ConstBool(true), BackendTy::Bool)),
            BackendTy::Nullable(inner) => match self.tir.types.get(inner) {
                BackendTy::Literal(_) => None,
                BackendTy::Num(_)
                | BackendTy::Str
                | BackendTy::Array(_)
                | BackendTy::Map(..)
                | BackendTy::Set(_)
                | BackendTy::Tuple(_)
                | BackendTy::Class(_) => {
                    let absent = self.emit(InstKind::IsNull { operand: v }, BackendTy::Bool);
                    Some(self.emit(
                        InstKind::Unary {
                            op: vn_core::UnOp::Not,
                            operand: absent,
                            ty: BackendTy::Bool,
                        },
                        BackendTy::Bool,
                    ))
                }
                BackendTy::Bool
                | BackendTy::Dynamic(_)
                | BackendTy::Char
                | BackendTy::Bytes
                | BackendTy::Enum(_)
                | BackendTy::Fn(_)
                | BackendTy::Decimal
                | BackendTy::BigInt
                | BackendTy::Void
                | BackendTy::Never
                | BackendTy::Nullable(_) => None,
            },
            BackendTy::Dynamic(_)
            | BackendTy::Char
            | BackendTy::Bytes
            | BackendTy::Enum(_)
            | BackendTy::Fn(_)
            | BackendTy::Decimal
            | BackendTy::BigInt
            | BackendTy::Void
            | BackendTy::Never => None,
        }
    }

    pub(super) fn lower_cast(
        &mut self,
        operand: &TirExpr,
        node_ty: vn_tir::BackendTy,
        ty: BackendTy,
    ) -> Result<Value> {
        use vn_core::NumericDomain as D;
        let v = self.lower_expr(operand)?;
        if ty == BackendTy::Bool {
            if let Some(holds) = self.static_truthiness(v) {
                return Ok(holds);
            }
            let absent = self.emit(
                InstKind::Unary {
                    op: vn_core::UnOp::Not,
                    operand: v,
                    ty: BackendTy::Bool,
                },
                BackendTy::Bool,
            );
            return Ok(self.emit(
                InstKind::Unary {
                    op: vn_core::UnOp::Not,
                    operand: absent,
                    ty: BackendTy::Bool,
                },
                BackendTy::Bool,
            ));
        }
        let unsigned_64 = BackendTy::Num(vn_core::NumKind::Int(vn_core::IntKind::U64));
        if node_ty == unsigned_64 {
            let whole = match super::ops::numeric_domain(self.value_ty(v).representation()) {
                Some(D::Float) => self.u64_convert(v, vn_core::ValueConv::FloatToBigInt),
                Some(D::Decimal) => self.u64_convert(v, vn_core::ValueConv::DecimalToBigInt),
                Some(D::Int | D::BigInt) | None => v,
            };
            return Ok(self.narrow_to(whole, node_ty));
        }
        let v = if self.value_ty(v) == BackendTy::Num(vn_core::NumKind::Int(vn_core::IntKind::U64))
            && matches!(node_ty, BackendTy::Num(vn_core::NumKind::Int(k)) if k != vn_core::IntKind::U64)
        {
            self.u64_convert(v, vn_core::ValueConv::DynToInt)
        } else {
            v
        };
        let from = match self.value_ty(v) {
            literal @ BackendTy::Literal(_) => super::ops::numeric_domain(literal.representation()),
            BackendTy::Num(vn_core::NumKind::Int(_)) => Some(D::Int),
            BackendTy::Num(vn_core::NumKind::Float(_)) => Some(D::Float),
            BackendTy::Bool
            | BackendTy::Str
            | BackendTy::Char
            | BackendTy::Bytes
            | BackendTy::Enum(_)
            | BackendTy::Fn(_)
            | BackendTy::Dynamic(_)
            | BackendTy::Decimal
            | BackendTy::BigInt
            | BackendTy::Void
            | BackendTy::Never
            | BackendTy::Array(_)
            | BackendTy::Map(..)
            | BackendTy::Set(_)
            | BackendTy::Tuple(_)
            | BackendTy::Class(_)
            | BackendTy::Nullable(_) => super::ops::numeric_domain(operand.ty)
                .filter(|d| matches!(d, D::BigInt | D::Decimal)),
        };
        let to = super::ops::numeric_domain(node_ty);
        let conv = match (from, to) {
            (Some(f), Some(t)) if f == t => return Ok(self.narrow_to(v, node_ty)),
            (Some(f), Some(t)) => vn_core::ValueConv::between(f, t),
            (None, Some(D::Int)) => Some(vn_core::ValueConv::DynToInt),
            (None, Some(D::Float)) => Some(vn_core::ValueConv::DynToFloat),
            (None, Some(D::BigInt)) | (None, Some(D::Decimal)) | (None, None) | (Some(_), None) => {
                None
            }
        };
        if let Some(conv) = conv {
            let result_ty = crate::ssa::verify::convert_result_ty(conv);
            let converted = self.emit(InstKind::Convert { operand: v, conv }, result_ty);
            return Ok(self.narrow_to(converted, node_ty));
        }
        Ok(self.emit(InstKind::Cast { operand: v, ty }, ty))
    }
}
