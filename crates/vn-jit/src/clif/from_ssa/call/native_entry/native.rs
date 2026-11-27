use cranelift_codegen::ir::{condcodes::IntCC, types, InstBuilder, MemFlagsData, Value};
use cranelift_frontend::FunctionBuilder;
use vn_types::register_meta::SlotKind;

use super::super::super::super::emit::{HEAP_KIND, KIND_MASK};
use super::super::super::super::native_abi::NativeShape;
use super::super::super::store::{clif_ty, drop_home_addrs};
use super::super::super::{heap, stack_exit, Ctx, Out};
use crate::NATIVE_FRAMELESS;
use vn_types::ssa::ValueId;

use super::*;
pub(crate) struct NativeCall<'a> {
    pub callee: Value,
    pub receiver: Value,
    pub args: &'a [ValueId],
    pub dest: Option<ValueId>,
}

pub(crate) fn emit(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    values: &[Option<Value>],
    call: NativeCall<'_>,
    slow: impl FnOnce(&mut FunctionBuilder) -> Result<Value, String>,
) -> Result<Out, String> {
    let frame = ctx.frame_for("call")?;
    let ectx = frame.exec_ctx;
    let lay = &ctx.helpers.call_layout;
    let m = MemFlagsData::trusted();
    let dest_kind = call
        .dest
        .map(|d| ctx.ssa.value_ty(d))
        .unwrap_or(SlotKind::Dynamic);
    let arg_kinds: Vec<SlotKind> = call.args.iter().map(|v| ctx.ssa.value_ty(*v)).collect();
    let shape = declared_shape(ctx, &arg_kinds, dest_kind)
        .unwrap_or_else(|| NativeShape::new(&arg_kinds, dest_kind));
    let Some(id) = shape.id() else {
        return Ok(Out::Boxed(slow(b)?));
    };

    let merge = b.create_block();
    b.append_block_param(
        merge,
        clif_ty(dest_kind).ok_or("from_ssa: unsupported call destination")?,
    );
    let slow_blk = b.create_block();
    b.set_cold_block(slow_blk);

    let (ctag, cpayload) = b.ins().isplit(call.callee);
    let kind = b.ins().band_imm_u(ctag, KIND_MASK);
    let is_heap_ref = b.ins().icmp_imm_u(IntCC::Equal, kind, HEAP_KIND);
    let walk = b.create_block();
    b.ins().brif(is_heap_ref, walk, &[], slow_blk, &[]);

    b.switch_to_block(walk);
    let slot = cpayload;
    let slot_tag = b.ins().uload8(
        types::I64,
        m,
        slot,
        ctx.helpers.array_layout.kind_off as i32,
    );
    let is_closure = b
        .ins()
        .icmp_imm_u(IntCC::Equal, slot_tag, lay.closure_tag as i64);
    let read = b.create_block();
    b.ins().brif(is_closure, read, &[], slow_blk, &[]);

    b.switch_to_block(read);
    let control = b
        .ins()
        .load(types::I64, m, slot, lay.closure_payload_off as i32);
    let closure = b.ins().iadd_imm_u(control, lay.rc_value_off as i64);
    let proto_control = b
        .ins()
        .load(types::I64, m, closure, lay.closure_proto_off as i32);
    let proto = b.ins().iadd_imm_u(proto_control, lay.rc_value_off as i64);
    let entry = b
        .ins()
        .load(types::I64, m, proto, lay.proto_native_off as i32);
    let sig = b
        .ins()
        .load(types::I64, m, proto, lay.proto_native_sig_off as i32);
    let epoch = b
        .ins()
        .load(types::I64, m, proto, lay.proto_epoch_off as i32);
    let has_entry = b.ins().icmp_imm_u(IntCC::NotEqual, entry, 0);
    let sig_core = b.ins().band_imm_u(sig, !NATIVE_FRAMELESS as i64);
    let same_sig = b.ins().icmp_imm_u(IntCC::Equal, sig_core, id as i64);
    let baked_epoch = crate::clif::reloc::abs_value(b, crate::clif::reloc::AbsSym::Epoch);
    let same_epoch = b.ins().icmp(IntCC::Equal, epoch, baked_epoch);
    let matched = b.ins().band(same_sig, same_epoch);
    let enter = b.ins().band(has_entry, matched);
    let entered = b.create_block();
    b.ins().brif(enter, entered, &[], slow_blk, &[]);

    b.switch_to_block(entered);
    stack_exit::guard_headroom(b, ctx);
    let frameless_bit = b.ins().band_imm_u(sig, NATIVE_FRAMELESS as i64);
    let frameless = b.ins().icmp_imm_u(IntCC::NotEqual, frameless_bit, 0);
    let push = b.create_block();
    let invoke = b.create_block();
    let push_frameless = b.create_block();
    b.ins().brif(frameless, push_frameless, &[], push, &[]);

    b.switch_to_block(push_frameless);
    stack_exit::enter_frameless(b, ctx);
    b.ins().jump(invoke, &[]);

    b.switch_to_block(push);
    push_frame(b, ctx, ectx, closure)?;
    b.ins().jump(invoke, &[]);

    b.switch_to_block(invoke);
    let argv = stage_argv(
        b,
        ctx,
        values,
        ectx,
        closure,
        call.receiver,
        call.args,
        &shape,
    )?;
    let sig_ref = b.import_signature(shape.signature());
    crate::clif::from_ssa::exceptions::clear_error_flag(b, ctx)?;
    let inst = b.ins().call_indirect(sig_ref, entry, &argv);
    let res = b.inst_results(inst).to_vec();
    emit_pop_for(b, ctx, ectx, frameless);

    stack_exit::publish(b, ctx.helpers, ectx);
    crate::clif::from_ssa::exceptions::check_error_branch(b, ctx)?;
    let direct = box_call_result(
        b,
        ctx,
        &res,
        crate::clif::native_abi::NativeClass::of(shape.ret),
        dest_kind,
    )?;
    drop_home_addrs(ctx);
    b.ins().jump(merge, &[direct.into()]);

    b.switch_to_block(slow_blk);
    let boxed = slow(b)?;
    let fallback = heap::unbox_checked(b, ctx, dest_kind, boxed)?;
    drop_home_addrs(ctx);
    b.ins().jump(merge, &[fallback.into()]);

    b.switch_to_block(merge);
    Ok(Out::Native(b.block_params(merge)[0]))
}
