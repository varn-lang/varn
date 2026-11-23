use std::cell::UnsafeCell;
use std::ptr::NonNull;

use super::{ArrayRepr, BoxedElems, CellVec, HeapRef, VmValue};
use vn_core::ArrayStorage;

#[derive(Clone, Copy, Debug)]
pub struct VmArray(NonNull<UnsafeCell<ArrayRepr>>);

impl ArrayRepr {
    #[inline(always)]
    pub fn boxed(items: Vec<VmValue>) -> Self {
        ArrayRepr::Boxed(BoxedElems::new(CellVec::from_vec(items)))
    }

    pub fn infer_storage(items: &[VmValue]) -> ArrayStorage {
        let Some(&first) = items.first() else {
            return ArrayStorage::Boxed;
        };
        if first.is_int() && items.iter().all(|v| v.is_int()) {
            ArrayStorage::Int
        } else if first.is_f64() && items.iter().all(|v| v.is_f64()) {
            ArrayStorage::Float
        } else if first.is_char() && items.iter().all(|v| v.as_char().is_some()) {
            ArrayStorage::Char
        } else if first.is_bool() && items.iter().all(|v| v.is_bool()) {
            ArrayStorage::Bool
        } else if first.is_heap() && items.iter().all(|v| v.is_heap()) {
            ArrayStorage::Ref
        } else {
            ArrayStorage::Boxed
        }
    }

    pub fn effective_storage(storage: ArrayStorage, items: &[VmValue]) -> ArrayStorage {
        match storage {
            ArrayStorage::Boxed | ArrayStorage::Int | ArrayStorage::Float | ArrayStorage::Bool => {
                storage
            }
            ArrayStorage::Char if items.iter().all(|v| v.as_char().is_some()) => storage,
            ArrayStorage::Ref if items.iter().all(|v| v.is_heap()) => storage,
            ArrayStorage::Char | ArrayStorage::Ref => ArrayStorage::Boxed,
        }
    }

    pub const fn elem_bytes(storage: ArrayStorage) -> usize {
        match storage {
            ArrayStorage::Boxed => std::mem::size_of::<VmValue>(),
            ArrayStorage::Int => std::mem::size_of::<i64>(),
            ArrayStorage::Float => std::mem::size_of::<f64>(),
            ArrayStorage::Char => std::mem::size_of::<char>(),
            ArrayStorage::Bool => std::mem::size_of::<bool>(),
            ArrayStorage::Ref => std::mem::size_of::<HeapRef>(),
        }
    }

    pub fn inferred(items: Vec<VmValue>) -> Self {
        Self::from_storage_owned(Self::infer_storage(&items), items)
    }

    pub fn from_storage_owned(storage: ArrayStorage, items: Vec<VmValue>) -> Self {
        match Self::effective_storage(storage, &items) {
            ArrayStorage::Boxed => Self::boxed(items),
            ArrayStorage::Int => ArrayRepr::I64(items.iter().map(|v| v.as_int()).collect()),
            ArrayStorage::Float => ArrayRepr::F64(items.iter().map(|v| v.as_f64()).collect()),
            ArrayStorage::Char => {
                ArrayRepr::Char(items.iter().filter_map(|v| v.as_char()).collect())
            }
            ArrayStorage::Bool => ArrayRepr::Bool(items.iter().map(|v| v.as_bool()).collect()),
            ArrayStorage::Ref => ArrayRepr::Ref(items.iter().map(|v| v.as_heap()).collect()),
        }
    }

    #[inline(always)]
    pub unsafe fn write_inline(
        storage: ArrayStorage,
        items: &[VmValue],
        data: *mut u8,
        cap: usize,
    ) -> Self {
        let len = items.len();
        macro_rules! fill {
            ($ty:ty, $variant:path, $conv:expr) => {{
                let ptr = data as *mut $ty;
                for (at, v) in items.iter().enumerate() {
                    unsafe { ptr.add(at).write($conv(*v)) };
                }
                $variant(unsafe { CellVec::inline(ptr, len, cap) })
            }};
        }
        match storage {
            ArrayStorage::Boxed => {
                let ptr = data as *mut VmValue;
                unsafe { std::ptr::copy_nonoverlapping(items.as_ptr(), ptr, len) };
                ArrayRepr::Boxed(BoxedElems::new(unsafe { CellVec::inline(ptr, len, cap) }))
            }
            ArrayStorage::Int => fill!(i64, ArrayRepr::I64, |v: VmValue| v.as_int()),
            ArrayStorage::Float => fill!(f64, ArrayRepr::F64, |v: VmValue| v.as_f64()),
            ArrayStorage::Char => {
                fill!(char, ArrayRepr::Char, |v: VmValue| v
                    .as_char()
                    .unwrap_or('\0'))
            }
            ArrayStorage::Bool => fill!(bool, ArrayRepr::Bool, |v: VmValue| v.as_bool()),
            ArrayStorage::Ref => fill!(HeapRef, ArrayRepr::Ref, |v: VmValue| v.as_heap()),
        }
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
