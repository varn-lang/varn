use super::context::{Builder, Result};
use crate::ssa::ir::Value;
use vn_tir::{BackendTy, DynReason, TirExpr};
use vn_types::ssa::SsaOp;

impl Builder<'_> {
    fn lower_checked_class(
        &mut self,
        v: Value,
        class: vn_tir::ClassId,
        nullable: bool,
        ty: BackendTy,
    ) -> Result<Value> {
        let name =
            self.tir
                .class(class)
                .map(|ci| ci.name.clone())
                .ok_or(crate::OptError::Unsupported(
                    "from_tir: Checked to an unknown class",
                ))?;
        let cls = self.emit(
            self.global_load(&name),
            BackendTy::Dynamic(DynReason::Runtime),
        );
        let is_instance = self.emit(
            SsaOp::TypeTest {
                value: v,
                class: cls,
            },
            BackendTy::Bool,
        );
        let holds = if nullable {
            let is_null = self.emit(SsaOp::IsNull { operand: v }, BackendTy::Bool);
            let not_slot = crate::ssa::emit::slot_kind_of(self.value_ty(is_null));
            let not_sel =
                vn_types::ssa::select_un(vn_core::UnOp::Not, not_slot).ok_or_else(|| {
                    crate::OptError::Internal("from_tir: unsupported unary op".to_owned())
                })?;
            let not_null = self.emit(
                SsaOp::Unary {
                    op: not_sel,
                    operand: is_null,
                },
                BackendTy::Bool,
            );
            let ty_slot = crate::ssa::emit::slot_kind_of(BackendTy::Bool);
            let lhs_slot = crate::ssa::emit::slot_kind_of(self.value_ty(is_instance));
            let rhs_slot = crate::ssa::emit::slot_kind_of(self.value_ty(not_null));
            let sel = vn_types::ssa::select_bin(vn_core::BinOp::Eq, ty_slot, lhs_slot, rhs_slot);
            self.emit(
                SsaOp::Binary {
                    op: sel,
                    lhs: is_instance,
                    rhs: not_null,
                },
                BackendTy::Bool,
            )
        } else {
            is_instance
        };
        let expected = if nullable {
            format!("{name}?")
        } else {
            name.to_string()
        };
        self.throw_type_mismatch_unless(holds, v, &expected)?;
        Ok(self.emit(SsaOp::Cast { operand: v }, ty))
    }

    fn throw_type_mismatch_unless(&mut self, holds: Value, v: Value, expected: &str) -> Result<()> {
        let throw_blk = self.new_block();
        let join_blk = self.new_block();
        let from = self.current;
        self.set_term(crate::ssa::ir::Terminator::Branch {
            cond: holds,
            then_blk: join_blk,
            then_args: vec![],
            else_blk: throw_blk,
            else_args: vec![],
        });
        self.add_pred(join_blk, from);
        self.add_pred(throw_blk, from);
        self.seal_block(join_blk);
        self.seal_block(throw_blk);
        self.current = throw_blk;
        let got = self.emit(SsaOp::KindOf { operand: v }, BackendTy::Str);
        let head = self.emit(
            SsaOp::ConstStr(std::sync::Arc::from(format!(
                "type mismatch: expected '{expected}', got '"
            ))),
            BackendTy::Str,
        );
        let tail = self.emit(SsaOp::ConstStr(std::sync::Arc::from("'")), BackendTy::Str);
        let mut message = head;
        for part in [got, tail] {
            let ty_slot = crate::ssa::emit::slot_kind_of(BackendTy::Str);
            let lhs_slot = crate::ssa::emit::slot_kind_of(self.value_ty(message));
            let rhs_slot = crate::ssa::emit::slot_kind_of(self.value_ty(part));
            let sel = vn_types::ssa::select_bin(vn_core::BinOp::Add, ty_slot, lhs_slot, rhs_slot);
            message = self.emit(
                SsaOp::Binary {
                    op: sel,
                    lhs: message,
                    rhs: part,
                },
                BackendTy::Str,
            );
        }
        let type_error =
            vn_core::abi::native_global_index(vn_core::RuntimeErrorKind::TypeError.name()).ok_or(
                crate::OptError::Unsupported("from_tir: TypeError is not a native global"),
            )?;
        let ctor = self.emit(
            SsaOp::LoadNativeGlobalIdx(type_error),
            BackendTy::Dynamic(DynReason::Runtime),
        );
        let error = self.emit(
            SsaOp::Call {
                callee: ctor,
                args: vec![message],
                shape: None,
            },
            BackendTy::Dynamic(DynReason::Runtime),
        );
        self.set_term(crate::ssa::ir::Terminator::Throw(error));
        self.current = join_blk;
        Ok(())
    }

    fn native_decode_text<'t>(&self, operand: &'t TirExpr) -> Option<(&'t TirExpr, u64)> {
        let vn_tir::Resolution::NativeDecode { typed, .. } = operand.res else {
            return None;
        };
        let vn_tir::TirExprKind::Call { args, .. } = &operand.kind else {
            return None;
        };
        let [vn_tir::TirArg::Expr(text)] = args.as_slice() else {
            return None;
        };
        (text.ty == BackendTy::Str).then_some((text, typed))
    }

    pub(super) fn lower_checked(
        &mut self,
        operand: &TirExpr,
        node_ty: vn_tir::BackendTy,
        ty: BackendTy,
        kind: vn_tir::CheckKind,
    ) -> Result<Value> {
        if kind == vn_tir::CheckKind::Decode
            && super::conform::is_structural(node_ty)
            && let Some((text, typed)) = self.native_decode_text(operand)
        {
            let text = self.lower_expr(text)?;
            return self.lower_conform_op(text, node_ty, ty, typed);
        }
        let v = self.lower_expr(operand)?;
        if kind == vn_tir::CheckKind::Decode
            && matches!(
                node_ty,
                vn_tir::BackendTy::Dynamic(vn_tir::DynReason::Interface(_))
            )
        {
            return self.lower_conform(v, node_ty, ty, kind);
        }
        match node_ty {
            vn_tir::BackendTy::Class(class) => {
                return self.lower_checked_class(v, class, false, ty);
            }
            vn_tir::BackendTy::Nullable(inner) if self.tir.types.contains(inner) => {
                let inner = self.tir.types.get(inner);
                if let vn_tir::BackendTy::Class(class) = inner {
                    return self.lower_checked_class(v, class, true, ty);
                }
                if super::conform::is_structural(inner) {
                    return self.lower_conform(v, node_ty, ty, kind);
                }
            }
            vn_tir::BackendTy::Literal(_)
            | vn_tir::BackendTy::Array(_)
            | vn_tir::BackendTy::Map(..)
            | vn_tir::BackendTy::Set(_)
            | vn_tir::BackendTy::Tuple(_)
            | vn_tir::BackendTy::Enum(_)
            | vn_tir::BackendTy::Fn(_)
            | vn_tir::BackendTy::Char
            | vn_tir::BackendTy::Bytes
            | vn_tir::BackendTy::Decimal
            | vn_tir::BackendTy::BigInt
            | vn_tir::BackendTy::Dynamic(
                vn_tir::DynReason::Callable
                | vn_tir::DynReason::Shape(_)
                | vn_tir::DynReason::Interface(_),
            ) => {
                return self.lower_conform(v, node_ty, ty, kind);
            }
            vn_tir::BackendTy::Num(_)
            | vn_tir::BackendTy::Bool
            | vn_tir::BackendTy::Str
            | vn_tir::BackendTy::Nullable(_)
            | vn_tir::BackendTy::Void
            | vn_tir::BackendTy::Never
            | vn_tir::BackendTy::Dynamic(
                vn_tir::DynReason::HostBoundary
                | vn_tir::DynReason::Union
                | vn_tir::DynReason::IndexSignature
                | vn_tir::DynReason::Declared
                | vn_tir::DynReason::Awaitable(_)
                | vn_tir::DynReason::Yields(_)
                | vn_tir::DynReason::Range(_)
                | vn_tir::DynReason::TaskHandle(_)
                | vn_tir::DynReason::ClassObject(_)
                | vn_tir::DynReason::CoreMember
                | vn_tir::DynReason::Spread
                | vn_tir::DynReason::Unrecorded
                | vn_tir::DynReason::Generic
                | vn_tir::DynReason::Boxed
                | vn_tir::DynReason::Runtime,
            ) => {}
        }
        let conv = match ty {
            BackendTy::Literal(_) => {
                return self.lower_conform(v, node_ty, ty.representation(), kind)
            }
            BackendTy::Str => vn_core::ValueConv::ExpectStr,
            BackendTy::Bool => vn_core::ValueConv::ExpectBool,
            BackendTy::Num(width) => vn_core::ValueConv::expect_num(width),
            BackendTy::Nullable(inner) => {
                let conv = match self.tir.types.get(inner) {
                    BackendTy::Literal(_) => return self.lower_conform(v, node_ty, ty, kind),
                    BackendTy::Num(width) if !width.is_default_width() => {
                        return self.lower_conform(v, node_ty, ty, kind)
                    }
                    BackendTy::Str => vn_core::ValueConv::ExpectStrOrNull,
                    BackendTy::Bool => vn_core::ValueConv::ExpectBoolOrNull,
                    BackendTy::Num(vn_core::NumKind::Int(_)) => vn_core::ValueConv::ExpectIntOrNull,
                    BackendTy::Num(vn_core::NumKind::Float(_)) => {
                        vn_core::ValueConv::ExpectFloatOrNull
                    }
                    BackendTy::Dynamic(_)
                    | BackendTy::Char
                    | BackendTy::Bytes
                    | BackendTy::Enum(_)
                    | BackendTy::Fn(_)
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
                        return Err(crate::OptError::Unsupported(
                            "from_tir: Checked to an unsupported nullable type",
                        ));
                    }
                };
                let checked = self.emit(
                    SsaOp::Convert { operand: v, conv },
                    BackendTy::Dynamic(DynReason::Boxed),
                );
                return Ok(self.emit(SsaOp::Cast { operand: checked }, ty));
            }
            BackendTy::Dynamic(_)
            | BackendTy::Char
            | BackendTy::Bytes
            | BackendTy::Enum(_)
            | BackendTy::Fn(_)
            | BackendTy::Decimal
            | BackendTy::BigInt
            | BackendTy::Void
            | BackendTy::Never
            | BackendTy::Array(_)
            | BackendTy::Map(..)
            | BackendTy::Set(_)
            | BackendTy::Tuple(_)
            | BackendTy::Class(_) => {
                return Err(crate::OptError::Unsupported(
                    "from_tir: Checked to a non-scalar type",
                ));
            }
        };
        let result_ty = crate::ssa::verify::convert_result_ty(conv);
        Ok(self.emit(SsaOp::Convert { operand: v, conv }, result_ty))
    }
}
