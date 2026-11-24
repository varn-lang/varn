use super::context::{Builder, Result};
use crate::ssa::ir::Value;
use crate::OptError;
use vn_tir::BackendTy;
use vn_tir::{Resolution, TirExpr};
use vn_types::ssa::SsaOp;

impl<'m> Builder<'m> {
    pub(super) fn lower_field(
        &mut self,
        object: &TirExpr,
        name: &std::sync::Arc<str>,
        res: &Resolution,
        ty: BackendTy,
    ) -> Result<Value> {
        if let Resolution::EnumVariant { enum_id, tag } = res {
            return self.lower_variant_constant(*enum_id, *tag, ty);
        }
        let obj = self.lower_expr(object)?;
        let kind = match res {
            Resolution::FieldSlot(slot) => match self.compact_field(object.ty, *slot) {
                Some((offset, kind)) => SsaOp::GetFixedField {
                    object: obj,
                    slot: *slot,
                    offset,
                    access: vn_core::FieldAccess::Compact(kind),
                },
                None => SsaOp::GetProperty {
                    object: obj,
                    name: name.clone(),
                },
            },
            Resolution::StaticField(slot) => SsaOp::GetFixedField {
                object: obj,
                slot: *slot,
                offset: 0,
                access: vn_core::FieldAccess::Slot,
            },
            Resolution::None
            | Resolution::Local(_)
            | Resolution::Param(_)
            | Resolution::Upvalue(_)
            | Resolution::GlobalSlot(_)
            | Resolution::NativeGlobal(_)
            | Resolution::ModuleSlot { .. }
            | Resolution::VtableSlot(_)
            | Resolution::DirectFn(_)
            | Resolution::Intrinsic(_)
            | Resolution::NativeOp(_)
            | Resolution::NativeDecode { .. }
            | Resolution::EnumVariant { .. }
            | Resolution::ByName { .. } => SsaOp::GetProperty {
                object: obj,
                name: name.clone(),
            },
        };
        Ok(self.emit(kind, ty))
    }

    fn lower_variant_constant(
        &mut self,
        enum_id: vn_tir::EnumId,
        tag: i64,
        ty: BackendTy,
    ) -> Result<Value> {
        let unsupported = || OptError::Unsupported("from_tir: variant constant without a slot");
        let info = self.tir.enum_info(enum_id).ok_or_else(unsupported)?;
        let global = info
            .variant_at(tag)
            .and_then(|variant| info.variant_global(&variant.name))
            .ok_or_else(unsupported)?;
        let slot = self.gslot(&global).ok_or_else(unsupported)?;
        Ok(self.emit(SsaOp::LoadGlobalIdx(slot), ty))
    }

    pub(super) fn lower_length(&mut self, operand: &TirExpr, ty: BackendTy) -> Result<Value> {
        let obj = self.lower_expr(operand)?;
        if matches!(
            operand.ty.non_nullable(&self.tir.types),
            vn_tir::BackendTy::Bytes
        ) {
            return Ok(self.emit(SsaOp::BytesLength { operand: obj }, ty));
        }
        let value_ty = self.value_ty(obj);
        let inner = if let BackendTy::Nullable(id) = value_ty {
            self.tir.types.get(id)
        } else {
            value_ty
        };
        let kind = match inner {
            BackendTy::Literal(vn_core::TypeLiteral::Str(_)) => SsaOp::StrLength { operand: obj },
            BackendTy::Str => SsaOp::StrLength { operand: obj },
            BackendTy::Array(_) | BackendTy::Tuple(_) => SsaOp::ArrayLength { operand: obj },
            BackendTy::Literal(
                vn_core::TypeLiteral::Int(_)
                | vn_core::TypeLiteral::Bool(_)
                | vn_core::TypeLiteral::Char(_),
            )
            | BackendTy::Num(_)
            | BackendTy::Bool
            | BackendTy::Char
            | BackendTy::Bytes
            | BackendTy::Enum(_)
            | BackendTy::Fn(_)
            | BackendTy::Decimal
            | BackendTy::BigInt
            | BackendTy::Void
            | BackendTy::Never
            | BackendTy::Dynamic(_)
            | BackendTy::Map(..)
            | BackendTy::Set(_)
            | BackendTy::Class(_)
            | BackendTy::Nullable(_) => SsaOp::GetProperty {
                object: obj,
                name: std::sync::Arc::from(vn_core::MemberKey::Length.name()),
            },
        };
        Ok(self.emit(kind, ty))
    }

    pub(super) fn lower_index(
        &mut self,
        object: &TirExpr,
        index: &TirExpr,
        ty: BackendTy,
    ) -> Result<Value> {
        let obj = self.lower_expr(object)?;
        let idx = self.lower_expr(index)?;
        let kind = if self.value_ty(obj) == BackendTy::Str && self.value_ty(idx) == BackendTy::INT {
            SsaOp::CallNativeOp {
                object: obj,
                args: vec![idx],
                op_id: vn_core::runtime_ops::RuntimeOp::StrIndex.id(),
            }
        } else if matches!(object.ty, vn_tir::BackendTy::Bytes)
            && matches!(self.value_ty(idx), BackendTy::INT | BackendTy::Dynamic(_))
        {
            SsaOp::BytesGetIndex {
                object: obj,
                index: idx,
            }
        } else if matches!(
            self.value_ty(obj),
            BackendTy::Array(_) | BackendTy::Tuple(_)
        ) {
            SsaOp::ArrayGetIndex {
                object: obj,
                index: idx,
            }
        } else {
            SsaOp::GetIndex {
                object: obj,
                index: idx,
            }
        };
        Ok(self.emit(kind, ty))
    }
}
