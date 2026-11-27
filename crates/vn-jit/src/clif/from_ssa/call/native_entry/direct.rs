use cranelift_codegen::ir::{condcodes::IntCC, types, InstBuilder, MemFlagsData, Value};
use cranelift_frontend::FunctionBuilder;
use vn_types::register_meta::SlotKind;

use super::super::super::super::emit::{call_helper_void, HEAP_KIND, KIND_MASK};
use super::super::super::super::native_abi::{NativeClass, NativeShape};
use super::super::super::store::{clif_ty, drop_home_addrs};
use super::super::super::{heap, load_value, stack_exit, Ctx, Out};
use crate::NATIVE_FRAMELESS;
use vn_types::ssa::ValueId;

use super::*;
pub(crate) struct DirectCall<'a> {
    pub callee: ValueId,
    pub args: &'a [ValueId],
    pub dest: Option<ValueId>,
    pub site: u16,
}

pub(crate) fn emit_direct(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    values: &[Option<Value>],
    call: DirectCall<'_>,
) -> Result<Out, String> {
    use super::super::direct::{entry_out_slot, run_entered_or};
    use super::super::invoke::{boxed_window, emit_invoke};
    use crate::clif::emit::call_helper;

    let frame = ctx.frame_for("direct call")?;
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
