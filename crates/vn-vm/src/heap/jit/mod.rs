use super::structs::{Heap, HeapInner};
use std::rc::Rc;

mod array;
mod buffer;
mod instance;
mod object;
mod string;

impl Heap {
    pub(crate) fn young_len_byte_offset_from_rcbox() -> usize {
        super::young::YoungGen::probe_born_layout();
        RCBOX_PREFIX
            + std::mem::offset_of!(HeapInner, young)
            + super::young::YoungGen::born_len_byte_offset()
    }

    pub(crate) fn young_limit_byte_offset_from_rcbox() -> usize {
        RCBOX_PREFIX
            + std::mem::offset_of!(HeapInner, young)
            + super::young::YoungGen::limit_byte_offset()
    }

    pub(crate) fn rcbox_ptr_for_validation(&self) -> *const u8 {
        (Rc::as_ptr(&self.inner) as *const u8).wrapping_sub(2 * std::mem::size_of::<usize>())
    }
}

pub(super) const RCBOX_PREFIX: usize = 2 * std::mem::size_of::<usize>();
