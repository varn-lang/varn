use super::ctx::ExecCtx;
use crate::error::{RuntimeError, VmResult};
use crate::heap::{Heap, HeapObj};
use crate::value::VmValue;
use vn_types::NativeCtx;
impl ExecCtx {
    pub(crate) fn bound_method(&mut self, recv: VmValue, name: &str) -> Option<VmValue> {
        let method = super::props::get_property(recv, name, &mut self.heap).ok()?;
        let callable = method.is_heap()
            && matches!(
                self.heap.get(method.as_heap()),
                Some(HeapObj::BoundMethod(_) | HeapObj::VmClosure(_) | HeapObj::NativeFn(..))
            );
        callable.then_some(method)
    }
}

pub(crate) fn bind_method(
    receiver: VmValue,
    method: VmValue,
    heap: &mut Heap,
) -> VmResult<VmValue> {
    if let Some((f, name)) = heap.native_of(method) {
        return Ok(heap.alloc_bound_native(receiver, f, name));
    }
    if heap.closure_of(method).is_some() && !receiver.is_null() {
        return Ok(heap.alloc_bound_vm(receiver, method, None));
    }
    Ok(method)
}

pub(crate) fn slot_method_bound(
    receiver: VmValue,
    slot: usize,
    heap: &mut Heap,
) -> VmResult<VmValue> {
    let cls = super::props::get_class(receiver, heap)
        .ok_or_else(|| RuntimeError::new("GetSlotMethod: value has no class"))?;
    let vtable = cls.vtable.borrow();
    let method = vtable.get(slot).copied().ok_or_else(|| {
        RuntimeError::new(format!(
            "GetSlotMethod: slot {slot} out of range for class '{}' (vtable len {})",
            cls.name,
            vtable.len()
        ))
    })?;
    let owner = cls.vtable_owners.borrow().get(slot).cloned().flatten();
    drop(vtable);
    Ok(super::props::bind_method_to_receiver(
        heap, receiver, method, owner,
    ))
}

pub(crate) fn invoke_runtime_static(
    name: &str,
    stack: &mut Vec<VmValue>,
    heap: &mut Heap,
    flag: u16,
) -> VmResult<VmValue> {
    match name {
        vn_core::well_known::RUNTIME_RANGE => {
            let end = stack
                .pop()
                .ok_or_else(|| RuntimeError::new("range: stack empty"))?;
            let start = stack
                .pop()
                .ok_or_else(|| RuntimeError::new("range: stack empty"))?;
            let r = match (start.as_char(), end.as_char()) {
                (Some(a), Some(b)) => vn_types::value::RangeData {
                    elem: vn_types::value::RangeElem::Char,
                    ..vn_types::value::RangeData::int(a as i64, b as i64, flag != 0)
                },
                _ => {
                    vn_types::value::RangeData::int(heap.as_int(start), heap.as_int(end), flag != 0)
                }
            };
            Ok(heap.alloc_range_data(r))
        }
        _ => Err(RuntimeError::new(format!(
            "OpInvokeRuntimeStatic: method '{}' not supported",
            name
        ))),
    }
}
