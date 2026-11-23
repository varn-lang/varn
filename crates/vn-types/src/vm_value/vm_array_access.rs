use super::{ArrayRepr, BoxedElems, CellVec, HeapRef, VmArray, VmValue};
use crate::memory::ExternalLedger;

#[inline(always)]
fn push_vec<T: Copy>(v: &mut CellVec<T>, val: T) {
    let heap_before = v.heap_capacity();
    v.push(val);
    let heap_after = v.heap_capacity();
    if heap_after > heap_before {
        ExternalLedger::grow((heap_after - heap_before) * std::mem::size_of::<T>());
    }
}

impl VmArray {
    #[inline]
    pub fn get_vm(&self, idx: usize) -> Option<VmValue> {
        match self.repr() {
            ArrayRepr::Boxed(v) => v.get(idx).copied(),
            ArrayRepr::I64(v) => v.get(idx).map(|&n| VmValue::from_int(n)),
            ArrayRepr::F64(v) => v.get(idx).map(|&f| VmValue::from_f64(f)),
            ArrayRepr::Char(v) => v.get(idx).map(|&c| VmValue::from_char(c)),
            ArrayRepr::Bool(v) => v.get(idx).map(|&b| VmValue::from_bool(b)),
            ArrayRepr::Ref(v) => v.get(idx).map(|&r| VmValue::from_heap(r)),
        }
    }

    #[inline]
    pub fn set_vm(&self, idx: usize, val: VmValue) -> bool {
        {
            match self.repr_mut() {
                ArrayRepr::Boxed(v) => {
                    return if idx < v.items.len() {
                        v.items[idx] = val;
                        true
                    } else {
                        false
                    };
                }
                ArrayRepr::I64(v) => {
                    if idx >= v.len() {
                        return false;
                    }
                    if val.is_int() {
                        v[idx] = val.as_int();
                        return true;
                    }
                }
                ArrayRepr::F64(v) => {
                    if idx >= v.len() {
                        return false;
                    }
                    if val.is_f64() {
                        v[idx] = val.as_f64();
                        return true;
                    }
                }
                ArrayRepr::Char(v) => {
                    if idx >= v.len() {
                        return false;
                    }
                    if let Some(c) = val.as_char() {
                        v[idx] = c;
                        return true;
                    }
                }
                ArrayRepr::Bool(v) => {
                    if idx >= v.len() {
                        return false;
                    }
                    if val.is_bool() {
                        v[idx] = val.as_bool();
                        return true;
                    }
                }
                ArrayRepr::Ref(v) => {
                    if idx >= v.len() {
                        return false;
                    }
                    if val.is_heap() {
                        v[idx] = val.as_heap();
                        return true;
                    }
                }
            }
        }

        self.migrate_to_boxed()[idx] = val;
        true
    }

    #[inline]
    pub fn push_vm(&self, val: VmValue) {
        {
            match self.repr_mut() {
                ArrayRepr::Boxed(v) => {
                    if !v.items.is_empty() {
                        push_vec(&mut v.items, val);
                        return;
                    }
                }
                ArrayRepr::I64(v) => {
                    if val.is_int() {
                        push_vec(v, val.as_int());
                        return;
                    }
                }
                ArrayRepr::F64(v) => {
                    if val.is_f64() {
                        push_vec(v, val.as_f64());
                        return;
                    }
                }
                ArrayRepr::Char(v) => {
                    if let Some(c) = val.as_char() {
                        push_vec(v, c);
                        return;
                    }
                }
                ArrayRepr::Bool(v) => {
                    if val.is_bool() {
                        push_vec(v, val.as_bool());
                        return;
                    }
                }
                ArrayRepr::Ref(v) => {
                    if val.is_heap() {
                        push_vec(v, val.as_heap());
                        return;
                    }
                }
            }
        }
        self.push_repr_change(val);
    }

    #[cold]
    fn push_repr_change(&self, val: VmValue) {
        if matches!(self.repr(), ArrayRepr::Boxed(_)) {
            if val.is_int() {
                ExternalLedger::grow(std::mem::size_of::<i64>());
                *self.repr_mut() = ArrayRepr::I64(CellVec::from_vec(vec![val.as_int()]));
            } else if val.is_f64() {
                ExternalLedger::grow(std::mem::size_of::<f64>());
                *self.repr_mut() = ArrayRepr::F64(CellVec::from_vec(vec![val.as_f64()]));
            } else if let Some(c) = val.as_char() {
                ExternalLedger::grow(std::mem::size_of::<char>());
                *self.repr_mut() = ArrayRepr::Char(CellVec::from_vec(vec![c]));
            } else if val.is_bool() {
                ExternalLedger::grow(std::mem::size_of::<bool>());
                *self.repr_mut() = ArrayRepr::Bool(CellVec::from_vec(vec![val.as_bool()]));
            } else if val.is_heap() {
                ExternalLedger::grow(std::mem::size_of::<HeapRef>());
                *self.repr_mut() = ArrayRepr::Ref(CellVec::from_vec(vec![val.as_heap()]));
            } else {
                match self.repr_mut() {
                    ArrayRepr::Boxed(v) => v.items.push(val),
                    ArrayRepr::I64(_)
                    | ArrayRepr::F64(_)
                    | ArrayRepr::Char(_)
                    | ArrayRepr::Bool(_)
                    | ArrayRepr::Ref(_) => {
                        unreachable!("checked Boxed above")
                    }
                }
            }
            return;
        }
        self.migrate_to_boxed().push(val);
    }

    #[inline]
    pub fn pop_vm(&self) -> Option<VmValue> {
        match self.repr_mut() {
            ArrayRepr::Boxed(v) => v.items.pop(),
            ArrayRepr::I64(v) => v.pop().map(VmValue::from_int),
            ArrayRepr::F64(v) => v.pop().map(VmValue::from_f64),
            ArrayRepr::Char(v) => v.pop().map(VmValue::from_char),
            ArrayRepr::Bool(v) => v.pop().map(VmValue::from_bool),
            ArrayRepr::Ref(v) => v.pop().map(VmValue::from_heap),
        }
    }

    #[cold]
    #[allow(clippy::mut_from_ref)]
    fn migrate_to_boxed(&self) -> &mut CellVec<VmValue> {
        {
            let repr = self.repr_mut();
            if !matches!(repr, ArrayRepr::Boxed(_)) {
                let boxed: Vec<VmValue> = match repr {
                    ArrayRepr::I64(v) => v.iter().map(|&n| VmValue::from_int(n)).collect(),
                    ArrayRepr::F64(v) => v.iter().map(|&f| VmValue::from_f64(f)).collect(),
                    ArrayRepr::Char(v) => v.iter().map(|&c| VmValue::from_char(c)).collect(),
                    ArrayRepr::Bool(v) => v.iter().map(|&b| VmValue::from_bool(b)).collect(),
                    ArrayRepr::Ref(v) => v.iter().map(|&r| VmValue::from_heap(r)).collect(),
                    ArrayRepr::Boxed(_) => unreachable!(),
                };
                ExternalLedger::grow(boxed.capacity() * std::mem::size_of::<VmValue>());
                *repr = ArrayRepr::Boxed(BoxedElems::new(CellVec::from_vec(boxed)));
            }
        }
        match self.repr_mut() {
            ArrayRepr::Boxed(v) => &mut v.items,
            ArrayRepr::I64(_)
            | ArrayRepr::F64(_)
            | ArrayRepr::Char(_)
            | ArrayRepr::Bool(_)
            | ArrayRepr::Ref(_) => unreachable!(),
        }
    }
}
