use super::cells::INLINE_ARRAY_MAX;
use super::structs::HeapInner;
use crate::value::VmValue;
use std::rc::Rc;
use vn_types::vm_value::ArrayRepr;

impl HeapInner {
    pub(crate) fn alloc_array_vm(
        &mut self,
        storage: vn_core::ArrayStorage,
        items: Vec<VmValue>,
    ) -> VmValue {
        self.alloc_items(false, storage, items)
    }

    pub(crate) fn alloc_array_inferred_vm(&mut self, items: Vec<VmValue>) -> VmValue {
        let storage = ArrayRepr::infer_storage(&items);
        self.alloc_items(false, storage, items)
    }

    pub(crate) fn alloc_array_slice_vm(
        &mut self,
        storage: vn_core::ArrayStorage,
        items: &[VmValue],
    ) -> VmValue {
        if items.len() <= INLINE_ARRAY_MAX {
            self.alloc_array_inline_vm(false, storage, items)
        } else {
            self.alloc_items(false, storage, items.to_vec())
        }
    }

    pub(crate) fn alloc_tuple_vm(&mut self, items: Vec<VmValue>) -> VmValue {
        let storage = ArrayRepr::infer_storage(&items);
        self.alloc_items(true, storage, items)
    }

    pub(crate) fn alloc_tuple_slice_vm(&mut self, items: &[VmValue]) -> VmValue {
        let storage = ArrayRepr::infer_storage(items);
        if items.len() <= INLINE_ARRAY_MAX {
            self.alloc_array_inline_vm(true, storage, items)
        } else {
            self.alloc_items(true, storage, items.to_vec())
        }
    }

    fn alloc_items(
        &mut self,
        tuple: bool,
        storage: vn_core::ArrayStorage,
        items: Vec<VmValue>,
    ) -> VmValue {
        if items.len() <= INLINE_ARRAY_MAX {
            self.alloc_array_inline_vm(tuple, storage, &items)
        } else {
            self.alloc_array_repr(tuple, ArrayRepr::from_storage_owned(storage, items))
        }
    }

    pub(crate) fn alloc_object(&mut self) -> VmValue {
        self.alloc_object_cell(vn_types::root_shape(), 0, &[])
    }

    pub(crate) fn alloc_object_with_shape(
        &mut self,
        shape: &Rc<vn_types::Shape>,
        values: Vec<VmValue>,
    ) -> VmValue {
        self.alloc_object_with_shape_slice(shape, &values)
    }

    pub(crate) fn alloc_object_with_shape_slice(
        &mut self,
        shape: &Rc<vn_types::Shape>,
        values: &[VmValue],
    ) -> VmValue {
        self.alloc_object_cell(Rc::clone(shape), values.len(), values)
    }

    #[inline(always)]
    pub(crate) fn alloc_empty_map_vm(&mut self) -> VmValue {
        self.alloc_map_vm(vn_types::value::ValueMap::default())
    }

    #[inline(always)]
    pub(crate) fn alloc_map_vm(&mut self, map: vn_types::value::ValueMap) -> VmValue {
        if let Some(h) = &self.hotspot {
            h.borrow_mut()
                .record_alloc(vn_core::RuntimeKind::Map.name());
        }
        let r = self.cells.alloc_map(map, super::cells::SlotState::Young);
        self.young.born.push(r);
        VmValue::from_heap(r)
    }
}
