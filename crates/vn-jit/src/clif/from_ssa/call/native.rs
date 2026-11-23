use cranelift_codegen::ir::{types, InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;

use super::super::super::emit::{box_bool, box_found_index, box_int};
use super::super::{heap, Ctx};
use super::invoke::boxed_window;
use vn_types::ssa::ValueId;

pub(crate) fn emit_call_native_op(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    values: &[Option<Value>],
    object: ValueId,
    args: &[ValueId],
    op_id: u64,
) -> Result<Value, String> {
    let frame = ctx.frame_for("native call")?;
    if args.is_empty() && op_id == vn_core::op_id::KnownOp::IntToString.id() {
        return heap::emit_unary_boxed(b, ctx, values, object, ctx.helpers.to_string);
    }
    let receiver = heap::boxed_value(b, ctx, values, object)?;
    if args.len() <= 1 && op_id == vn_core::op_id::KnownOp::GeneratorNext.id() {
        return emit_generator_next(b, ctx, receiver);
    }
    if args.len() == 1
        && (op_id == vn_core::op_id::KnownOp::StrCharCodeAt.id()
            || op_id == vn_core::op_id::KnownOp::StrCodePointAt.id())
    {
        let (recv_tag, recv_payload) = b.ins().isplit(receiver);
        let (pos_tag, pos_payload) = heap::boxed_parts(b, ctx, values, args[0])?;
        return emit_char_code_at(b, ctx, recv_tag, recv_payload, pos_tag, pos_payload);
    }
    if args.len() == 1 && op_id == vn_core::runtime_ops::RuntimeOp::StrIndex.id() {
        let (recv_tag, recv_payload) = b.ins().isplit(receiver);
        let (pos_tag, pos_payload) = heap::boxed_parts(b, ctx, values, args[0])?;
        crate::clif::from_ssa::exceptions::call_checked(
            b,
            ctx,
            ctx.helpers.str_char_at,
            &[frame.exec_ctx, recv_tag, recv_payload, pos_tag, pos_payload],
        )?;
        return Ok(crate::clif::emit::load_native_result(
            b,
            ctx.helpers,
            frame.exec_ctx,
        ));
    }
    if args.len() == 1 {
        let (recv_tag, recv_payload) = b.ins().isplit(receiver);
        let (arg_tag, arg_payload) = heap::boxed_parts(b, ctx, values, args[0])?;
        if op_id == vn_core::op_id::KnownOp::StrStartsWith.id() {
            let r = crate::clif::from_ssa::exceptions::call_checked_value(
                b,
                ctx,
                ctx.helpers.str_starts_with,
                &[frame.exec_ctx, recv_tag, recv_payload, arg_tag, arg_payload],
            )?;
            return Ok(box_bool(b, r));
        }
        if op_id == vn_core::op_id::KnownOp::StrEndsWith.id() {
            let r = crate::clif::from_ssa::exceptions::call_checked_value(
                b,
                ctx,
                ctx.helpers.str_ends_with,
                &[frame.exec_ctx, recv_tag, recv_payload, arg_tag, arg_payload],
            )?;
            return Ok(box_bool(b, r));
        }
        if op_id == vn_core::op_id::KnownOp::StrIndexOf.id() {
            let r = crate::clif::from_ssa::exceptions::call_checked_value(
                b,
                ctx,
                ctx.helpers.str_index_of,
                &[frame.exec_ctx, recv_tag, recv_payload, arg_tag, arg_payload],
            )?;
            return Ok(box_found_index(b, r));
        }
        if op_id == vn_core::op_id::KnownOp::StrIncludes.id() {
            let r = crate::clif::from_ssa::exceptions::call_checked_value(
                b,
                ctx,
                ctx.helpers.str_includes,
                &[frame.exec_ctx, recv_tag, recv_payload, arg_tag, arg_payload],
            )?;
            return Ok(box_bool(b, r));
        }
    }
    if op_id == vn_core::op_id::KnownOp::StrSplit.id() && args.len() <= 1 {
        let (recv_tag, recv_payload) = b.ins().isplit(receiver);
        let argc_v = b.ins().iconst(types::I64, args.len() as i64);
        let (sep_tag, sep_payload) = if args.len() == 1 {
            heap::boxed_parts(b, ctx, values, args[0])?
        } else {
            (b.ins().iconst(types::I64, 0), b.ins().iconst(types::I64, 0))
        };
        crate::clif::from_ssa::exceptions::call_checked(
            b,
            ctx,
            ctx.helpers.str_split,
            &[
                frame.exec_ctx,
                recv_tag,
                recv_payload,
                argc_v,
                sep_tag,
                sep_payload,
            ],
        )?;
        return Ok(crate::clif::emit::load_native_result(
            b,
            ctx.helpers,
            frame.exec_ctx,
        ));
    }
    if op_id == vn_core::op_id::KnownOp::StrSubstring.id() && (args.len() == 1 || args.len() == 2) {
        let (recv_tag, recv_payload) = b.ins().isplit(receiver);
        let (start_tag, start_payload) = heap::boxed_parts(b, ctx, values, args[0])?;
        let has_end_v = b.ins().iconst(types::I64, (args.len() - 1) as i64);
        let (end_tag, end_payload) = if args.len() == 2 {
            heap::boxed_parts(b, ctx, values, args[1])?
        } else {
            (b.ins().iconst(types::I64, 0), b.ins().iconst(types::I64, 0))
        };
        crate::clif::from_ssa::exceptions::call_checked(
            b,
            ctx,
            ctx.helpers.str_substring,
            &[
                frame.exec_ctx,
                recv_tag,
                recv_payload,
                start_tag,
                start_payload,
                has_end_v,
                end_tag,
                end_payload,
            ],
        )?;
        return Ok(crate::clif::emit::load_native_result(
            b,
            ctx.helpers,
            frame.exec_ctx,
        ));
    }
    if op_id == vn_core::op_id::KnownOp::StrSlice.id() && (args.len() == 1 || args.len() == 2) {
        let (recv_tag, recv_payload) = b.ins().isplit(receiver);
        let (start_tag, start_payload) = heap::boxed_parts(b, ctx, values, args[0])?;
        let has_end_v = b.ins().iconst(types::I64, (args.len() - 1) as i64);
        let (end_tag, end_payload) = if args.len() == 2 {
            heap::boxed_parts(b, ctx, values, args[1])?
        } else {
            (b.ins().iconst(types::I64, 0), b.ins().iconst(types::I64, 0))
        };
        crate::clif::from_ssa::exceptions::call_checked(
            b,
            ctx,
            ctx.helpers.str_slice_range,
            &[
                frame.exec_ctx,
                recv_tag,
                recv_payload,
                start_tag,
                start_payload,
                has_end_v,
                end_tag,
                end_payload,
            ],
        )?;
        return Ok(crate::clif::emit::load_native_result(
            b,
            ctx.helpers,
            frame.exec_ctx,
        ));
    }
    let window = boxed_window(b, ctx, values, receiver, args)?;
    let slot = {
        let mut natives = ctx.natives.borrow_mut();
        natives.push(op_id);
        natives.len() - 1
    };
    let fn_v = crate::clif::reloc::abs_value(b, crate::clif::reloc::AbsSym::Native(slot as u32));
    let op_v = b.ins().iconst(types::I64, op_id as i64);
    let total = b.ins().iconst(types::I64, (args.len() + 1) as i64);
    crate::clif::from_ssa::exceptions::call_checked(
        b,
        ctx,
        ctx.helpers.jit_call_native_window,
        &[frame.exec_ctx, fn_v, op_v, window, total],
    )?;
    Ok(crate::clif::emit::load_native_result(
        b,
        ctx.helpers,
        frame.exec_ctx,
    ))
}

fn emit_char_code_at(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    recv_tag: Value,
    recv_payload: Value,
    pos_tag: Value,
    pos_payload: Value,
) -> Result<Value, String> {
    use cranelift_codegen::ir::{condcodes::IntCC, MemFlagsData};
    let m = MemFlagsData::trusted();
    let frame = ctx.frame_for("charCodeAt")?;
    let zero = b.ins().iconst(types::I64, 0);
    let minus_one = b.ins().iconst(types::I64, -1);

    let merge = b.create_block();
    b.append_block_param(merge, types::I64);
    let slow = b.create_block();
    b.set_cold_block(slow);

    let answer = |b: &mut FunctionBuilder, v: Value| {
        b.ins().jump(merge, &[v.into()]);
    };

    let kind_of = |b: &mut FunctionBuilder, tag: Value| {
        b.ins()
            .band_imm_u(tag, vn_types::vm_value::KIND_MASK as i64)
    };
    let pos_kind = kind_of(b, pos_tag);
    let pos_is_int =
        b.ins()
            .icmp_imm_u(IntCC::Equal, pos_kind, vn_types::vm_value::KIND_INT as i64);
    let int_path = b.create_block();
    b.ins().brif(pos_is_int, int_path, &[], slow, &[]);

    b.switch_to_block(int_path);
    let negative = b.ins().icmp(IntCC::SignedLessThan, pos_payload, zero);
    let nonneg = b.create_block();
    b.ins()
        .brif(negative, merge, &[minus_one.into()], nonneg, &[]);
    let _ = nonneg;

    b.switch_to_block(nonneg);
    let recv_kind = kind_of(b, recv_tag);
    let is_sso = b
        .ins()
        .icmp_imm_u(IntCC::Equal, recv_kind, vn_types::vm_value::KIND_SSO as i64);
    let sso_path = b.create_block();
    let heap_path = b.create_block();
    b.ins().brif(is_sso, sso_path, &[], heap_path, &[]);

    b.switch_to_block(sso_path);
    let shifted = b.ins().ushr_imm_u(recv_tag, 8);
    let sso_len = b.ins().band_imm_u(shifted, 0xFF);
    let past_end = b
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, pos_payload, sso_len);
    let sso_in_range = b.create_block();
    b.ins()
        .brif(past_end, merge, &[minus_one.into()], sso_in_range, &[]);

    b.switch_to_block(sso_in_range);
    let high = b
        .ins()
        .band_imm_u(recv_payload, vn_types::vm_value::SSO_HIGH_BITS as i64);
    let non_ascii = b.ins().icmp_imm_u(IntCC::NotEqual, high, 0);
    let sso_ascii = b.create_block();
    b.ins().brif(non_ascii, slow, &[], sso_ascii, &[]);

    b.switch_to_block(sso_ascii);
    let three = b.ins().iconst(types::I64, 3);
    let bit = b.ins().ishl(pos_payload, three);
    let moved = b.ins().ushr(recv_payload, bit);
    let byte = b.ins().band_imm_u(moved, 0xFF);
    answer(b, byte);

    b.switch_to_block(heap_path);
    let lay = &ctx.helpers.array_layout;
    let str_layout = &ctx.helpers.str_layout;
    let cell_kind = b
        .ins()
        .uload8(types::I64, m, recv_payload, lay.kind_off as i32);
    let is_str = b
        .ins()
        .icmp_imm_u(IntCC::Equal, cell_kind, lay.str_tag as i64);
    let str_cell = b.create_block();
    b.ins().brif(is_str, str_cell, &[], slow, &[]);

    b.switch_to_block(str_cell);
    let cell_base = b.ins().iadd_imm_u(recv_payload, lay.payload_off as i64);
    let disc = b.ins().uload8(types::I64, m, cell_base, 0);
    let is_shared = b
        .ins()
        .icmp_imm_u(IntCC::Equal, disc, str_layout.shared_disc as i64);
    let shared_cell = b.create_block();
    b.ins().brif(is_shared, shared_cell, &[], slow, &[]);

    b.switch_to_block(shared_cell);
    let ascii = b
        .ins()
        .uload8(types::I64, m, cell_base, str_layout.shared_ascii_off as i32);
    let is_ascii = b
        .ins()
        .icmp_imm_u(IntCC::Equal, ascii, str_layout.ascii_yes as i64);
    let ascii_cell = b.create_block();
    b.ins().brif(is_ascii, ascii_cell, &[], slow, &[]);

    b.switch_to_block(ascii_cell);
    let arc_ptr = b
        .ins()
        .load(types::I64, m, cell_base, str_layout.shared_arc_off as i32);
    let arc_len = b.ins().load(
        types::I64,
        m,
        cell_base,
        (str_layout.shared_arc_off + 8) as i32,
    );
    let past_heap_end = b
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, pos_payload, arc_len);
    let heap_in_range = b.create_block();
    b.ins().brif(
        past_heap_end,
        merge,
        &[minus_one.into()],
        heap_in_range,
        &[],
    );

    b.switch_to_block(heap_in_range);
    let data = b.ins().iadd_imm_u(arc_ptr, str_layout.arc_data_off as i64);
    let at = b.ins().iadd(data, pos_payload);
    let heap_byte = b.ins().uload8(types::I64, m, at, 0);
    answer(b, heap_byte);

    b.switch_to_block(slow);
    let code = crate::clif::from_ssa::exceptions::call_checked_value(
        b,
        ctx,
        ctx.helpers.str_char_code_at,
        &[frame.exec_ctx, recv_tag, recv_payload, pos_tag, pos_payload],
    )?;
    b.ins().jump(merge, &[code.into()]);

    b.switch_to_block(merge);
    Ok(box_int(b, b.block_params(merge)[0]))
}

fn emit_generator_next(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    receiver: Value,
) -> Result<Value, String> {
    let exec_ctx = ctx.exec_ctx;
    let (gen_tag, gen_payload) = b.ins().isplit(receiver);
    let out = super::direct::entry_out_slot(b);
    let entry = super::super::super::emit::call_helper(
        b,
        ctx.cc,
        ctx.helpers.generator_enter,
        &[exec_ctx, gen_tag, gen_payload, out],
    );
    let entered = b.create_block();
    let merge = b.create_block();
    let taken = b
        .ins()
        .icmp_imm_u(cranelift_codegen::ir::condcodes::IntCC::NotEqual, entry, 0);
    b.ins().brif(taken, entered, &[], merge, &[]);
    b.switch_to_block(entered);
    super::direct::call_entry(b, ctx, entry, out)?;
    super::super::super::emit::call_helper_void(
        b,
        ctx.cc,
        ctx.helpers.generator_leave,
        &[exec_ctx],
    );
    b.ins().jump(merge, &[]);
    b.switch_to_block(merge);
    super::super::store::drop_home_addrs(ctx);
    crate::clif::from_ssa::exceptions::check_error_branch(b, ctx)?;
    Ok(crate::clif::emit::load_native_result(
        b,
        ctx.helpers,
        exec_ctx,
    ))
}
