use super::get_class;
use crate::error::{RuntimeError, VmResult};
use crate::heap::{Heap, HeapObj};
use crate::value::VmValue;

pub(crate) fn copy_value(value: VmValue, heap: &mut Heap) -> VmResult<VmValue> {
    if value.is_heap() {
        let at = value.as_heap();
        if let Some(source) = heap.instance(at) {
            let fields: Vec<(usize, VmValue)> = (0..source.slot_count())
                .filter_map(|slot| source.field_at(slot).map(|field| (slot, field)))
                .collect();
            let class = get_class(value, heap)
                .ok_or_else(|| RuntimeError::type_error("with: an instance without a class"))?;
            let (copy, target) = heap.alloc_instance(&class);
            for (slot, field) in fields {
                target.set_field_at(slot, field);
                heap.write_barrier(copy, field);
            }
            return Ok(VmValue::from_heap(copy));
        }
        if let Some(HeapObj::Object(source)) = heap.get(at) {
            let source = *source;
            let shape = source.shape().clone();
            let class = source.class();
            let mut values = Vec::with_capacity(source.slot_count());
            source.for_each_field(|_, field| values.push(field));
            let copy = heap.alloc_object_cell(shape, values.len(), &values);
            if let Some(class) = class
                && let Some(HeapObj::Object(target)) = heap.get(copy.as_heap())
            {
                target.set_class(class);
            }
            return Ok(copy);
        }
    }
    Err(RuntimeError::type_error(format!(
        "with: a {} has no fields to copy",
        super::meta::type_name(value, heap)
    )))
}
