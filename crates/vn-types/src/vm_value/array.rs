use super::{CellVec, HeapRef, VmValue};

#[repr(C, u8)]
#[derive(Debug)]
pub enum ArrayRepr {
    Boxed(BoxedElems) = 0,

    I64(CellVec<i64>) = 1,

    F64(CellVec<f64>) = 2,

    Char(CellVec<char>) = 3,

    Bool(CellVec<bool>) = 4,

    Ref(CellVec<HeapRef>) = 5,
}

#[repr(C)]
#[derive(Debug)]
pub struct BoxedElems {
    pub(super) items: CellVec<VmValue>,
}

impl BoxedElems {
    #[inline(always)]
    pub fn new(items: CellVec<VmValue>) -> Self {
        Self { items }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[VmValue] {
        &self.items
    }
}

impl std::ops::Deref for BoxedElems {
    type Target = CellVec<VmValue>;

    #[inline(always)]
    fn deref(&self) -> &CellVec<VmValue> {
        &self.items
    }
}

impl ArrayRepr {
    pub const DISC_OFF: usize = 0;

    pub const ELEMS_UNION_OFF: usize = 8;

    #[inline(always)]
    pub fn discriminant(&self) -> u8 {
        match self {
            ArrayRepr::Boxed(_) => 0,
            ArrayRepr::I64(_) => 1,
            ArrayRepr::F64(_) => 2,
            ArrayRepr::Char(_) => 3,
            ArrayRepr::Bool(_) => 4,
            ArrayRepr::Ref(_) => 5,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        match self {
            ArrayRepr::Boxed(v) => v.len(),
            ArrayRepr::I64(v) => v.len(),
            ArrayRepr::F64(v) => v.len(),
            ArrayRepr::Char(v) => v.len(),
            ArrayRepr::Bool(v) => v.len(),
            ArrayRepr::Ref(v) => v.len(),
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn heap_bytes(&self) -> u64 {
        match self {
            ArrayRepr::Boxed(v) => {
                (v.items.heap_capacity() * std::mem::size_of::<VmValue>()) as u64
            }
            ArrayRepr::I64(v) => (v.heap_capacity() * std::mem::size_of::<i64>()) as u64,
            ArrayRepr::F64(v) => (v.heap_capacity() * std::mem::size_of::<f64>()) as u64,
            ArrayRepr::Char(v) => (v.heap_capacity() * std::mem::size_of::<char>()) as u64,
            ArrayRepr::Bool(v) => (v.heap_capacity() * std::mem::size_of::<bool>()) as u64,
            ArrayRepr::Ref(v) => (v.heap_capacity() * std::mem::size_of::<HeapRef>()) as u64,
        }
    }
}

const _: () = {
    assert!(std::mem::align_of::<ArrayRepr>() == 8);
    assert!(ArrayRepr::ELEMS_UNION_OFF == 8);
    assert!(std::mem::size_of::<ArrayRepr>().is_multiple_of(8));
};
