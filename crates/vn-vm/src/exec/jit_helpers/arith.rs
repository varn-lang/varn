use super::construct::jit_propagate_error;
use crate::exec::ctx::ExecCtx;
use crate::value::VmValue;

#[vn_macros::jit_slow(field = "negate")]
pub(crate) extern "C" fn jit_negate(ctx: *mut ExecCtx, v_tag: u64, v_payload: u64) {
    unsafe {
        let ctx_ref = &mut *ctx;
        let v = VmValue::from_raw_parts(v_tag, v_payload);
        match crate::exec::arith::negate(v, &mut ctx_ref.heap) {
            Ok(r) => ctx_ref.jit_native_result = r,
            Err(e) => jit_propagate_error(ctx_ref, e),
        }
    }
}

#[vn_macros::jit_slow(field = "logical_not")]
pub(crate) extern "C" fn jit_logical_not(_ctx: *mut ExecCtx, v_tag: u64, v_payload: u64) -> u64 {
    let v = VmValue::from_raw_parts(v_tag, v_payload);
    u64::from(!v.as_bool())
}

#[vn_macros::jit_slow(field = "div")]
pub(crate) extern "C" fn jit_div(
    ctx: *mut ExecCtx,
    a_tag: u64,
    a_payload: u64,
    b_tag: u64,
    b_payload: u64,
) {
    unsafe {
        super::shape::binary_fallible(
            ctx,
            a_tag,
            a_payload,
            b_tag,
            b_payload,
            crate::exec::arith::div,
        )
    }
}

#[vn_macros::jit_slow(field = "modulo")]
pub(crate) extern "C" fn jit_modulo(
    ctx: *mut ExecCtx,
    a_tag: u64,
    a_payload: u64,
    b_tag: u64,
    b_payload: u64,
) {
    unsafe {
        super::shape::binary_fallible(
            ctx,
            a_tag,
            a_payload,
            b_tag,
            b_payload,
            crate::exec::arith::modulo,
        )
    }
}

#[vn_macros::jit_slow(field = "pow")]
pub(crate) extern "C" fn jit_pow(
    ctx: *mut ExecCtx,
    a_tag: u64,
    a_payload: u64,
    b_tag: u64,
    b_payload: u64,
) {
    unsafe {
        super::shape::binary_fallible(
            ctx,
            a_tag,
            a_payload,
            b_tag,
            b_payload,
            crate::exec::arith::pow,
        )
    }
}

#[vn_macros::jit_slow(field = "bit_and")]
pub(crate) extern "C" fn jit_bitand(
    ctx: *mut ExecCtx,
    a_tag: u64,
    a_payload: u64,
    b_tag: u64,
    b_payload: u64,
) {
    unsafe {
        super::shape::binary(
            ctx,
            a_tag,
            a_payload,
            b_tag,
            b_payload,
            crate::exec::arith::bit_and,
        )
    }
}

#[vn_macros::jit_slow(field = "bit_or")]
pub(crate) extern "C" fn jit_bitor(
    ctx: *mut ExecCtx,
    a_tag: u64,
    a_payload: u64,
    b_tag: u64,
    b_payload: u64,
) {
    unsafe {
        super::shape::binary(
            ctx,
            a_tag,
            a_payload,
            b_tag,
            b_payload,
            crate::exec::arith::bit_or,
        )
    }
}

#[vn_macros::jit_slow(field = "bit_xor")]
pub(crate) extern "C" fn jit_bitxor(
    ctx: *mut ExecCtx,
    a_tag: u64,
    a_payload: u64,
    b_tag: u64,
    b_payload: u64,
) {
    unsafe {
        super::shape::binary(
            ctx,
            a_tag,
            a_payload,
            b_tag,
            b_payload,
            crate::exec::arith::bit_xor,
        )
    }
}

#[vn_macros::jit_slow(field = "shl")]
pub(crate) extern "C" fn jit_shl(
    ctx: *mut ExecCtx,
    a_tag: u64,
    a_payload: u64,
    b_tag: u64,
    b_payload: u64,
) {
    unsafe {
        super::shape::binary_fallible(
            ctx,
            a_tag,
            a_payload,
            b_tag,
            b_payload,
            crate::exec::arith::shl,
        )
    }
}

#[vn_macros::jit_slow(field = "shr")]
pub(crate) extern "C" fn jit_shr(
    ctx: *mut ExecCtx,
    a_tag: u64,
    a_payload: u64,
    b_tag: u64,
    b_payload: u64,
) {
    unsafe {
        super::shape::binary_fallible(
            ctx,
            a_tag,
            a_payload,
            b_tag,
            b_payload,
            crate::exec::arith::shr,
        )
    }
}

#[vn_macros::jit_slow(field = "ushr")]
pub(crate) extern "C" fn jit_ushr(
    ctx: *mut ExecCtx,
    a_tag: u64,
    a_payload: u64,
    b_tag: u64,
    b_payload: u64,
) {
    unsafe {
        super::shape::binary_fallible(
            ctx,
            a_tag,
            a_payload,
            b_tag,
            b_payload,
            crate::exec::arith::ushr,
        )
    }
}
