use super::construct::jit_propagate_error;
use crate::error::VmResult;
use crate::exec::ctx::ExecCtx;
use crate::heap::Heap;
use crate::value::VmValue;

#[inline(always)]
pub(super) unsafe fn binary_fallible(
    ctx: *mut ExecCtx,
    a_tag: u64,
    a_payload: u64,
    b_tag: u64,
    b_payload: u64,
    op: impl FnOnce(VmValue, VmValue, &mut Heap) -> VmResult<VmValue>,
) {
    let ctx_ref = unsafe { &mut *ctx };
    let a = VmValue::from_raw_parts(a_tag, a_payload);
    let b = VmValue::from_raw_parts(b_tag, b_payload);
    match op(a, b, &mut ctx_ref.heap) {
        Ok(v) => ctx_ref.jit_native_result = v,
        Err(e) => unsafe { jit_propagate_error(ctx_ref, e) },
    }
}

#[inline(always)]
pub(super) unsafe fn binary(
    ctx: *mut ExecCtx,
    a_tag: u64,
    a_payload: u64,
    b_tag: u64,
    b_payload: u64,
    op: impl FnOnce(VmValue, VmValue, &mut Heap) -> VmValue,
) {
    let ctx_ref = unsafe { &mut *ctx };
    let a = VmValue::from_raw_parts(a_tag, a_payload);
    let b = VmValue::from_raw_parts(b_tag, b_payload);
    ctx_ref.jit_native_result = op(a, b, &mut ctx_ref.heap);
}

#[inline(always)]
pub(super) unsafe fn compare(
    ctx: *mut ExecCtx,
    a_tag: u64,
    a_payload: u64,
    b_tag: u64,
    b_payload: u64,
    op: impl FnOnce(VmValue, VmValue, &Heap) -> bool,
) -> u64 {
    let ctx_ref = unsafe { &*ctx };
    let a = VmValue::from_raw_parts(a_tag, a_payload);
    let b = VmValue::from_raw_parts(b_tag, b_payload);
    u64::from(op(a, b, &ctx_ref.heap))
}
