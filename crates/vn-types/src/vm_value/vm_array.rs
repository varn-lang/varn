use std::cell::UnsafeCell;
use std::ptr::NonNull;

use super::{ArrayRepr, BoxedElems, VmValue};
use crate::register_meta::SlotKind;

#[derive(Clone, Copy, Debug)]
pub struct VmArray(NonNull<UnsafeCell<ArrayRepr>>);

impl ArrayRepr {
    #[inline(always)]
    pub fn boxed(items: Vec<VmValue>) -> Self {
        ArrayRepr::Boxed(BoxedElems::new(items))
    }

    pub fn from_items(items: Vec<VmValue>) -> Self {
        Self::unboxed(&items).unwrap_or_else(|| Self::boxed(items))
    }

    pub fn from_slice(items: &[VmValue]) -> Self {
        Self::unboxed(items).unwrap_or_else(|| Self::boxed(items.to_vec()))
    }

    fn unboxed(items: &[VmValue]) -> Option<Self> {
        let first = *items.first()?;
        if first.is_int() && items.iter().all(|v| v.is_int()) {
            return Some(ArrayRepr::I64(items.iter().map(|v| v.as_int()).collect()));
        }
        if first.is_f64() && items.iter().all(|v| v.is_f64()) {
            return Some(ArrayRepr::F64(items.iter().map(|v| v.as_f64()).collect()));
        }
        if first.is_char() {
            return items
                .iter()
                .map(|v| v.as_char())
                .collect::<Option<Vec<char>>>()
                .map(ArrayRepr::Char);
        }
        if first.is_bool() && items.iter().all(|v| v.is_bool()) {
            return Some(ArrayRepr::Bool(items.iter().map(|v| v.as_bool()).collect()));
        }
        if first.is_heap() && items.iter().all(|v| v.is_heap()) {
            return Some(ArrayRepr::Ref(items.iter().map(|v| v.as_heap()).collect()));
        }
        None
    }
}

impl VmArray {
    #[inline(always)]
    pub unsafe fn init_at(at: *mut ArrayRepr, repr: ArrayRepr) -> Self {
        unsafe {
            std::ptr::write(at, repr);
            Self(NonNull::new_unchecked(at as *mut UnsafeCell<ArrayRepr>))
        }
    }

    pub unsafe fn drop_at(self) {
        unsafe { std::ptr::drop_in_place(self.0.as_ptr()) };
    }

    #[inline(always)]
    pub fn repr(&self) -> &ArrayRepr {
        unsafe { &*(*self.0.as_ptr()).get() }
    }

    #[inline(always)]
    #[allow(clippy::mut_from_ref)]
    pub(super) fn repr_mut(&self) -> &mut ArrayRepr {
        unsafe { &mut *(*self.0.as_ptr()).get() }
    }

    #[inline(always)]
    pub fn discriminant(&self) -> u8 {
        self.repr().discriminant()
    }

    #[inline(always)]
    pub fn element_slotkind(&self) -> SlotKind {
        match self.repr() {
            ArrayRepr::Boxed(_) => SlotKind::Dynamic,
            ArrayRepr::I64(_) => SlotKind::Int,
            ArrayRepr::F64(_) => SlotKind::Float,
            ArrayRepr::Char(_) => SlotKind::Char,
            ArrayRepr::Bool(_) => SlotKind::Bool,
            ArrayRepr::Ref(_) => SlotKind::Ref,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.repr().len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.repr().is_empty()
    }
}

impl PartialEq for VmArray {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for VmArray {}

impl std::hash::Hash for VmArray {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state);
    }
}
