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
pub(super) fn emit_pop_for(b: &mut FunctionBuilder, ctx: &Ctx<'_>, ectx: Value, frameless: Value) {
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

pub(super) fn push_frame(
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

pub(super) fn pop_frame(b: &mut FunctionBuilder, ctx: &Ctx<'_>, ectx: Value) {
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
