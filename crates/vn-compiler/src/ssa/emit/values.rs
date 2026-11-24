use vn_tir::BackendTy;
use vn_types::ssa::SsaOp;
#[path = "values/access.rs"]
mod access;
#[path = "values/build.rs"]
mod build;
#[path = "values/calls.rs"]
mod calls;
#[path = "values/closure.rs"]
mod closure;
#[path = "values/scalars.rs"]
mod scalars;
use super::super::ir::{BlockId, Inst};
use crate::OptError;
use std::sync::Arc;
use vn_core::OpCode;
use vn_types::chunk::Chunk;
type Result<T> = std::result::Result<T, OptError>;
pub(super) fn emit_value(
    chunk: &mut Chunk,
    inst: &Inst,
    d: u8,
    value_tys: &[BackendTy],
    reg: &[u8],
    scratch: u8,
    call_base: u8,
    ic_slot: Option<u8>,
    source_file: &Arc<str>,
    nparams: usize,
    fixups: &mut Vec<(usize, BlockId)>,
    closure_const: &mut Option<u16>,
    scope: &crate::from_tir::compile::ModuleScope,
) -> Result<()> {
    let line = inst.line;
    match &inst.kind {
        SsaOp::ConstInt(n) => scalars::emit_const_int(chunk, d, *n, line),
        SsaOp::ConstFloat(f) => scalars::emit_const_float(chunk, d, *f, line),
        SsaOp::ConstBool(b) => scalars::emit_const_bool(chunk, d, *b, line),
        SsaOp::ConstStr(s) => scalars::emit_const_str(chunk, d, s, line),
        SsaOp::ConstChar(c) => scalars::emit_const_char(chunk, d, *c, line),
        SsaOp::ConstDecimal(dec) => scalars::emit_const_decimal(chunk, d, dec, line),
        SsaOp::ConstBigInt(n) => scalars::emit_const_bigint(chunk, d, n, line)?,
        SsaOp::ConstNull => scalars::emit_const_null(chunk, d, line),
        SsaOp::Binary { op, lhs, rhs } => {
            scalars::emit_binary(chunk, d, *op, *lhs, *rhs, reg, line);
        }
        SsaOp::Unary { op, operand } => {
            scalars::emit_unary(chunk, d, *op, *operand, reg, scratch, line);
        }
        SsaOp::KindOf { operand } => {
            chunk.emit_rr(OpCode::KindOf, d, reg[operand.0 as usize], line);
        }
        SsaOp::LoadGlobal(name) => access::emit_load_global(chunk, d, name, line),
        SsaOp::LoadGlobalIdx(slot) => access::emit_load_global_idx(chunk, d, *slot, line)?,
        SsaOp::LoadNativeGlobalIdx(slot) => {
            access::emit_load_native_global_idx(chunk, d, *slot, line)?;
        }
        SsaOp::LoadUpvalue(uv) => access::emit_load_upvalue(chunk, d, *uv, line),

        SsaOp::Call { callee, args, .. } => {
            calls::emit_call(chunk, d, *callee, args, reg, call_base, line);
        }
        SsaOp::CallDirect { callee, args, .. } => {
            calls::emit_call(chunk, d, *callee, args, reg, call_base, line);
        }
        SsaOp::AllocInstance { class, .. } => {
            chunk.write(Chunk::pack_op(OpCode::AllocInstance, d), line);
            chunk.write(Chunk::pack(reg[class.0 as usize], 0), line);
        }

        SsaOp::SelfCall { args } => {
            calls::emit_self_call(chunk, d, args, reg, call_base, line);
        }
        SsaOp::GetProperty { object, name } => {
            access::emit_get_property(chunk, d, *object, name, reg, ic_slot, line)?;
        }
        SsaOp::GetSlotMethod { object, slot } => {
            access::emit_get_slot_method(chunk, d, *object, *slot, reg, line);
        }
        SsaOp::GetFixedField {
            object,
            slot,
            offset,
            access,
        } => {
            access::emit_get_fixed_field(chunk, d, *object, reg, *slot, *offset, *access, line);
        }
        SsaOp::GetIndex { object, index } => {
            access::emit_get_index(chunk, d, *object, *index, reg, line);
        }
        SsaOp::ArrayGetIndex { object, index } => {
            access::emit_array_get_index(chunk, d, *object, *index, reg, line);
        }
        SsaOp::BytesGetIndex { object, index } => {
            access::emit_bytes_get_index(chunk, d, *object, *index, reg, line);
        }

        SsaOp::MethodCall { recv, name, args } => {
            calls::emit_method_call(
                chunk, d, *recv, name, args, value_tys, reg, call_base, ic_slot, line,
            )?;
        }
        SsaOp::IsNull { operand } => {
            scalars::emit_is_null(chunk, d, *operand, reg, line);
        }
        SsaOp::TypeTest { value, class } => {
            scalars::emit_type_test(chunk, d, *value, *class, reg, line);
        }
        SsaOp::Cast { operand, .. } => {
            scalars::emit_cast(chunk, d, *operand, reg, line);
        }
        SsaOp::Convert { operand, conv } => {
            scalars::emit_convert(chunk, d, *operand, reg, *conv, line);
        }

        SsaOp::BuildArray { elements, storage } => {
            build::emit_build_array(chunk, d, elements, *storage, reg, call_base, line);
        }

        SsaOp::BuildTuple { elements } => {
            build::emit_build_tuple(chunk, d, elements, reg, call_base, line);
        }

        SsaOp::BuildObject { pairs } => {
            build::emit_build_object(chunk, d, pairs, reg, call_base, line);
        }

        SsaOp::BuildMap { pairs } => {
            build::emit_build_map(chunk, d, pairs, reg, call_base, line);
        }
        SsaOp::ToString { operand } => {
            scalars::emit_to_string(chunk, d, *operand, reg, line);
        }

        SsaOp::MakeClosure { func, upvalues } => {
            closure::emit_make_closure(
                chunk,
                d,
                *func,
                upvalues,
                source_file,
                nparams,
                closure_const,
                line,
                scope,
            )?;
        }

        SsaOp::IntrinsicCall { object, args, wire }
            if args.len() == 1
                && vn_core::intrinsic_ops::math::is_unary_math(*wire)
                && matches!(
                    value_tys.get(args[0].0 as usize),
                    Some(BackendTy::Num(vn_core::NumKind::Float(_)))
                ) =>
        {
            calls::emit_intrinsic_direct(chunk, d, args, *wire, reg, line);
        }

        SsaOp::IntrinsicCall { object, args, wire } => {
            calls::emit_intrinsic(chunk, d, *object, args, *wire, reg, call_base, line);
        }

        SsaOp::CallNativeOp {
            object,
            args,
            op_id,
        } => {
            calls::emit_call_native_op(chunk, d, *object, args, *op_id, reg, call_base, line);
        }

        SsaOp::BuildStr { parts } => {
            build::emit_build_str(chunk, d, parts, reg, line);
        }
        SsaOp::GetPropertyMaybe { object, name } => {
            access::emit_get_property_maybe(chunk, d, *object, name, reg, line);
        }
        SsaOp::ModuleSlot { object, slot } => {
            access::emit_module_slot(chunk, d, *object, reg, *slot, line);
        }
        SsaOp::GetEnumTag { operand } => {
            scalars::emit_get_enum_tag(chunk, d, *operand, reg, line);
        }
        SsaOp::IsArray { operand } => {
            scalars::emit_is_array(chunk, d, *operand, reg, line);
        }
        SsaOp::StrLength { operand } => {
            scalars::emit_str_length(chunk, d, *operand, reg, line);
        }
        SsaOp::ArrayLength { operand } => {
            scalars::emit_array_length(chunk, d, *operand, reg, line);
        }
        SsaOp::BytesLength { operand } => {
            scalars::emit_bytes_length(chunk, d, *operand, reg, line);
        }

        SsaOp::This => scalars::emit_this(chunk, d, line),

        SsaOp::Range {
            start,
            end,
            inclusive,
        } => {
            access::emit_range(chunk, d, *start, *end, *inclusive, reg, line);
        }
        SsaOp::ObjectKeys { operand } => {
            scalars::emit_object_keys(chunk, d, *operand, reg, line);
        }

        SsaOp::GetSuper { name } => {
            access::emit_get_super(chunk, d, name, line);
        }
        SsaOp::GetSuperSlot { slot } => {
            access::emit_get_super_slot(chunk, d, *slot, line);
        }

        SsaOp::SuperCall { args } => {
            calls::emit_super_call(chunk, d, args, reg, call_base, line);
        }

        SsaOp::SuperMethodCall { name, args } => {
            calls::emit_super_method_call(chunk, d, name, args, reg, call_base, line);
        }

        SsaOp::InvokeSlot {
            recv, slot, args, ..
        } => {
            calls::emit_invoke_slot(chunk, d, *recv, *slot, args, reg, call_base, ic_slot, line)?;
        }

        SsaOp::SuperInvokeSlot { slot, args } => {
            calls::emit_super_invoke_slot(chunk, d, *slot, args, reg, call_base, line);
        }

        SsaOp::ExtensionCall {
            func,
            slot,
            recv,
            args,
        } => {
            calls::emit_extension_call(chunk, d, func, *slot, *recv, args, reg, call_base, line)?;
        }

        SsaOp::CallSpread { callee, args } => {
            calls::emit_call_spread(chunk, d, *callee, args, reg, call_base, line);
        }

        SsaOp::BuildArraySpread { elements } => {
            build::emit_build_array_spread(chunk, d, elements, reg, call_base, line);
        }

        SsaOp::BuildObjectSpread { parts } => {
            build::emit_build_object_spread(chunk, d, parts, reg, ic_slot, line)?;
        }
        SsaOp::ObjectRest { object, skip_keys } => {
            build::emit_object_rest(chunk, d, *object, skip_keys, reg, line);
        }
        SsaOp::LoadCaptured { var } => {
            access::emit_load_captured(chunk, d, *var, nparams, line);
        }
        SsaOp::MakeClass { name, super_class } => {
            closure::emit_make_class(chunk, d, name, *super_class, reg, line);
        }
        SsaOp::MakeEnumVariant { def } => {
            closure::emit_make_enum_variant(chunk, d, def, line);
        }
        SsaOp::Try { handler } => {
            closure::emit_try(chunk, d, *handler, fixups, line);
        }
        SsaOp::CatchParam { try_val } => {
            closure::emit_catch_param(chunk, d, *try_val, reg, line);
        }
        SsaOp::LoadModule { source } => {
            access::emit_load_module(chunk, d, source, line);
        }
        SsaOp::Await { operand } => {
            closure::emit_await(chunk, d, *operand, reg, line);
        }
        SsaOp::Spawn { operand } => {
            closure::emit_spawn(chunk, d, *operand, reg, line);
        }
        SsaOp::Yield { operand } => {
            closure::emit_yield(chunk, d, *operand, reg, line);
        }

        SsaOp::SetProperty { .. }
        | SsaOp::SetFixedField { .. }
        | SsaOp::SetIndex { .. }
        | SsaOp::ArrayPush { .. }
        | SsaOp::ArraySetIndex { .. }
        | SsaOp::BytesSetIndex { .. }
        | SsaOp::ObjectMerge { .. }
        | SsaOp::AssertNotNull { .. }
        | SsaOp::StoreGlobal { .. }
        | SsaOp::StoreGlobalIdx { .. }
        | SsaOp::StoreUpvalue { .. }
        | SsaOp::StoreCaptured { .. }
        | SsaOp::StoreModuleSlot { .. }
        | SsaOp::CloseUpvalues { .. }
        | SsaOp::PopTry
        | SsaOp::DeclareLayout { .. }
        | SsaOp::FreezeClass { .. }
        | SsaOp::DefineMember { .. } => {
            return Err(OptError::Internal(
                "ssa-emit: effect op has no value to define".to_owned(),
            ))
        }
    }
    Ok(())
}
