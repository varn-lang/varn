use cranelift_codegen::ir::{condcodes::IntCC, types, InstBuilder, MemFlagsData, Value};
use cranelift_frontend::FunctionBuilder;
use vn_types::register_meta::SlotKind;

use super::super::super::emit::{call_helper_void, HEAP_KIND, KIND_MASK};
use super::super::super::native_abi::{NativeClass, NativeShape};
use super::super::store::{clif_ty, drop_home_addrs};
use super::super::{heap, load_value, stack_exit, Ctx, Out};
use crate::NATIVE_FRAMELESS;

pub(crate) struct NativeCall<'a> {
    pub callee: Value,
    pub receiver: Value,
    pub args: &'a [u32],
    pub dest: Option<u32>,
}

pub(crate) fn emit(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    values: &[Option<Value>],
    call: NativeCall<'_>,
    slow: impl FnOnce(&mut FunctionBuilder) -> Result<Value, String>,
) -> Result<Out, String> {
    let frame = ctx
        .frame
        .as_ref()
        .ok_or("from_ssa: call in a frame-less body")?;
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

pub(crate) struct DirectCall<'a> {
    pub callee: u32,
    pub args: &'a [u32],
    pub dest: Option<u32>,
    pub site: u16,
}

pub(crate) fn emit_direct(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    values: &[Option<Value>],
    call: DirectCall<'_>,
) -> Result<Out, String> {
    use super::direct::{entry_out_slot, run_entered_or};
    use super::invoke::{boxed_window, emit_invoke};
    use crate::clif::emit::call_helper;

    let frame = ctx
        .frame
        .as_ref()
        .ok_or("from_ssa: direct call in a frame-less body")?;
    let ectx = frame.exec_ctx;
    let m = MemFlagsData::trusted();
    let dest_kind = call
        .dest
        .map(|d| ctx.ssa.value_ty(d))
        .unwrap_or(SlotKind::Dynamic);
    let arg_kinds: Vec<SlotKind> = call.args.iter().map(|v| ctx.ssa.value_ty(*v)).collect();
    let shape = declared_shape(ctx, &arg_kinds, dest_kind)
        .unwrap_or_else(|| NativeShape::new(&arg_kinds, dest_kind));
    let Some(id) = shape.id() else {
        let callee_v = load_value(b, ctx, values, call.callee)?;
        let args = call.args;
        let dest = call.dest;
        let native = NativeCall {
            callee: callee_v,
            receiver: callee_v,
            args,
            dest,
        };
        return emit(b, ctx, values, native, |b| {
            let window = boxed_window(b, ctx, values, callee_v, args)?;
            let (ctag, cpayload) = b.ins().isplit(callee_v);
            emit_invoke(b, ctx, window, (ctag, cpayload), args.len() + 1)
        });
    };

    let merge = b.create_block();
    b.append_block_param(
        merge,
        clif_ty(dest_kind).ok_or("from_ssa: unsupported call destination")?,
    );
    let miss_blk = b.create_block();
    b.set_cold_block(miss_blk);

    let callee_v = load_value(b, ctx, values, call.callee)?;
    let (ctag, cpayload) = b.ins().isplit(callee_v);
    let ic_base = b.ins().load(
        types::I64,
        m,
        frame.closure,
        ctx.helpers.closure_direct_entries_offset as i32,
    );
    let site_base = b.ins().iadd_imm_u(
        ic_base,
        call.site as i64 * ctx.helpers.direct_ic_stride as i64,
    );
    let cached_tag = b.ins().load(types::I64, m, site_base, 0);
    let cached_payload = b.ins().load(types::I64, m, site_base, 8);
    let cached_entry = b.ins().load(types::I64, m, site_base, 16);
    let cached_epoch = b.ins().load(types::I64, m, site_base, 24);
    let cached_closure = b.ins().load(types::I64, m, site_base, 40);
    let hit_tag = b.ins().icmp(IntCC::Equal, ctag, cached_tag);
    let hit_payload = b.ins().icmp(IntCC::Equal, cpayload, cached_payload);
    let baked_epoch = crate::clif::reloc::abs_value(b, crate::clif::reloc::AbsSym::Epoch);
    let hit_epoch = b.ins().icmp(IntCC::Equal, cached_epoch, baked_epoch);
    let hit_entry = b.ins().icmp_imm_u(IntCC::NotEqual, cached_entry, 0);
    let hit = b.ins().band(hit_tag, hit_payload);
    let hit = b.ins().band(hit, hit_epoch);
    let hit = b.ins().band(hit, hit_entry);
    let fast_blk = b.create_block();
    b.ins().brif(hit, fast_blk, &[], miss_blk, &[]);

    b.switch_to_block(fast_blk);
    stack_exit::guard_headroom(b, ctx);
    let cached_sig = b.ins().load(types::I64, m, site_base, 32);
    let frameless_bit = b.ins().band_imm_u(cached_sig, NATIVE_FRAMELESS as i64);
    let frameless = b.ins().icmp_imm_u(IntCC::NotEqual, frameless_bit, 0);
    let push = b.create_block();
    let invoke = b.create_block();
    let push_frameless = b.create_block();
    b.ins().brif(frameless, push_frameless, &[], push, &[]);

    b.switch_to_block(push_frameless);
    stack_exit::enter_frameless(b, ctx);
    b.ins().jump(invoke, &[]);

    b.switch_to_block(push);
    push_frame(b, ctx, ectx, cached_closure)?;
    b.ins().jump(invoke, &[]);

    b.switch_to_block(invoke);
    let argv = stage_argv(
        b,
        ctx,
        values,
        ectx,
        cached_closure,
        callee_v,
        call.args,
        &shape,
    )?;
    let sig_ref = b.import_signature(shape.signature());
    crate::clif::from_ssa::exceptions::clear_error_flag(b, ctx)?;
    let inst = b.ins().call_indirect(sig_ref, cached_entry, &argv);
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

    b.switch_to_block(miss_blk);
    let window = boxed_window(b, ctx, values, callee_v, call.args)?;
    let total = b.ins().iconst(types::I64, (call.args.len() + 1) as i64);
    let shape_imm = b.ins().iconst(types::I64, id as i64);
    let out = entry_out_slot(b);
    let entered = call_helper(
        b,
        ctx.cc,
        ctx.helpers.jit_call_direct_cached,
        &[
            ectx, ctag, cpayload, site_base, window, total, out, shape_imm,
        ],
    );
    drop_home_addrs(ctx);
    let res = run_entered_or(b, ctx, entered, out)?;
    let fallback = heap::unbox_checked(b, ctx, dest_kind, res)?;
    b.ins().jump(merge, &[fallback.into()]);

    b.switch_to_block(merge);
    Ok(Out::Native(b.block_params(merge)[0]))
}

fn stage_argv(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    values: &[Option<Value>],
    ectx: Value,
    closure: Value,
    receiver: Value,
    args: &[u32],
    shape: &NativeShape,
) -> Result<Vec<Value>, String> {
    let (rtag, rpayload) = b.ins().isplit(receiver);
    let mut argv = vec![ectx, closure, rtag, rpayload];
    for (v, class) in args.iter().zip(&shape.params[1..]) {
        match crate::clif::native_abi::NativeClass::of(*class) {
            NativeClass::Word | NativeClass::Float => argv.push(load_value(b, ctx, values, *v)?),
            NativeClass::Boxed => {
                let boxed = heap::boxed_value(b, ctx, values, *v)?;
                let (t, p) = b.ins().isplit(boxed);
                argv.push(t);
                argv.push(p);
            }
        }
    }
    Ok(argv)
}

fn box_call_result(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    res: &[Value],
    shape_ret: NativeClass,
    dest_kind: SlotKind,
) -> Result<Value, String> {
    Ok(match shape_ret {
        NativeClass::Word | NativeClass::Float => res[0],
        NativeClass::Boxed => {
            let boxed = b.ins().iconcat(res[0], res[1]);
            if NativeClass::of(dest_kind) == NativeClass::Boxed {
                boxed
            } else {
                heap::unbox_checked(b, ctx, dest_kind, boxed)?
            }
        }
    })
}

fn emit_pop_for(b: &mut FunctionBuilder, ctx: &Ctx<'_>, ectx: Value, frameless: Value) {
    let pop = b.create_block();
    let after = b.create_block();
    let pop_frameless = b.create_block();
    b.ins().brif(frameless, pop_frameless, &[], pop, &[]);

    b.switch_to_block(pop_frameless);
    stack_exit::leave_frameless(b, ctx);
    b.ins().jump(after, &[]);

    b.switch_to_block(pop);
    pop_frame(b, ctx, ectx);
    b.ins().jump(after, &[]);

    b.switch_to_block(after);
}

fn declared_shape(
    ctx: &Ctx<'_>,
    arg_kinds: &[SlotKind],
    dest_kind: SlotKind,
) -> Option<NativeShape> {
    let declared = ctx.ssa.call_shape_at(ctx.site_ip.get())?;
    if declared.params.len() != arg_kinds.len() {
        return None;
    }
    let same_class = |a: SlotKind, b: SlotKind| NativeClass::of(a) == NativeClass::of(b);
    let args_fit = declared.params.iter().zip(arg_kinds).all(|(param, arg)| {
        NativeClass::of(*param) == NativeClass::Boxed || same_class(*param, *arg)
    });
    let ret_fits =
        same_class(declared.ret, dest_kind) || NativeClass::of(declared.ret) == NativeClass::Boxed;
    (args_fit && ret_fits).then(|| NativeShape::new(&declared.params, declared.ret))
}

fn push_frame(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    ectx: Value,
    closure: Value,
) -> Result<(), String> {
    let lay = &ctx.helpers.call_layout;
    let m = MemFlagsData::trusted();
    let len = b.ins().load(types::I64, m, ectx, lay.frames_len_off as i32);
    let cap = b.ins().load(types::I64, m, ectx, lay.frames_cap_off as i32);
    let full = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, len, cap);
    let frameless = b
        .ins()
        .load(types::I64, m, ectx, lay.frameless_depth_off as i32);
    let depth = b.ins().iadd(len, frameless);
    let deep = b.ins().icmp_imm_u(
        IntCC::UnsignedGreaterThanOrEqual,
        depth,
        lay.max_call_depth as i64,
    );
    let out_of_line = b.ins().bor(full, deep);
    let inline = b.create_block();
    let grow = b.create_block();
    let done = b.create_block();
    b.set_cold_block(grow);
    b.ins().brif(out_of_line, grow, &[], inline, &[]);

    b.switch_to_block(grow);
    crate::clif::from_ssa::exceptions::call_checked(
        b,
        ctx,
        ctx.helpers.jit_push_native_frame,
        &[ectx, closure],
    )?;
    b.ins().jump(done, &[]);

    b.switch_to_block(inline);
    let base = b.ins().load(types::I64, m, ectx, lay.frames_ptr_off as i32);
    let off = b.ins().imul_imm_u(len, lay.frame_size as i64);
    let slot = b.ins().iadd(base, off);
    let caller = b.ins().iadd_imm_s(slot, -(lay.frame_size as i64));
    let pending = b
        .ins()
        .load(types::I64, m, ectx, ctx.helpers.jit_site_ip_offset as i32);
    let known = b.ins().load(types::I64, m, caller, lay.frame_ip_off as i32);
    let noted = b.ins().icmp_imm_u(IntCC::NotEqual, pending, 0);
    let settled = b.ins().select(noted, pending, known);
    b.ins().store(m, settled, caller, lay.frame_ip_off as i32);
    let none = b.ins().iconst(types::I64, 0);
    b.ins()
        .store(m, none, ectx, ctx.helpers.jit_site_ip_offset as i32);
    let control = b.ins().iadd_imm_s(closure, -(lay.rc_value_off as i64));
    let strong = b
        .ins()
        .load(types::I64, m, control, lay.rc_strong_off as i32);
    let strong = b.ins().iadd_imm_u(strong, 1);
    b.ins().store(m, strong, control, lay.rc_strong_off as i32);
    b.ins()
        .store(m, closure, slot, lay.frame_closure_ptr_off as i32);
    b.ins().store(m, control, slot, lay.frame_owned_off as i32);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().store(m, zero, slot, lay.frame_ip_off as i32);
    b.ins().store(m, zero, slot, lay.frame_class_off as i32);
    let no_activation = b.ins().iconst(types::I64, lay.no_activation as i64);
    b.ins()
        .store(m, no_activation, slot, lay.frame_base_off as i32);
    let no_return = b.ins().iconst(types::I64, lay.no_return_reg as i64);
    b.ins()
        .istore16(m, no_return, slot, lay.frame_return_reg_off as i32);
    let len = b.ins().iadd_imm_u(len, 1);
    b.ins().store(m, len, ectx, lay.frames_len_off as i32);
    b.ins().jump(done, &[]);

    b.switch_to_block(done);
    Ok(())
}

fn pop_frame(b: &mut FunctionBuilder, ctx: &Ctx<'_>, ectx: Value) {
    let lay = &ctx.helpers.call_layout;
    let m = MemFlagsData::trusted();
    let none = b.ins().iconst(types::I64, 0);
    b.ins()
        .store(m, none, ectx, ctx.helpers.jit_site_ip_offset as i32);
    let len = b.ins().load(types::I64, m, ectx, lay.frames_len_off as i32);
    let len = b.ins().iadd_imm_s(len, -1);
    b.ins().store(m, len, ectx, lay.frames_len_off as i32);
    let base = b.ins().load(types::I64, m, ectx, lay.frames_ptr_off as i32);
    let off = b.ins().imul_imm_u(len, lay.frame_size as i64);
    let slot = b.ins().iadd(base, off);
    let closure = b
        .ins()
        .load(types::I64, m, slot, lay.frame_closure_ptr_off as i32);
    let control = b.ins().iadd_imm_s(closure, -(lay.rc_value_off as i64));
    let strong = b
        .ins()
        .load(types::I64, m, control, lay.rc_strong_off as i32);
    let last = b.ins().icmp_imm_u(IntCC::Equal, strong, 1);
    let release = b.create_block();
    let keep = b.create_block();
    let done = b.create_block();
    b.set_cold_block(release);
    b.ins().brif(last, release, &[], keep, &[]);

    b.switch_to_block(release);
    call_helper_void(b, ctx.cc, ctx.helpers.jit_release_closure, &[closure]);
    b.ins().jump(done, &[]);

    b.switch_to_block(keep);
    let strong = b.ins().iadd_imm_s(strong, -1);
    b.ins().store(m, strong, control, lay.rc_strong_off as i32);
    b.ins().jump(done, &[]);

    b.switch_to_block(done);
}
