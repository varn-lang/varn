use super::*;

pub(crate) fn build_with_shape_slice(
    shape: Rc<vn_types::Shape>,
    vals: &[VmValue],
    heap: &mut Heap,
) -> VmValue {
    heap.alloc_object_cell(shape, vals.len(), vals)
}
