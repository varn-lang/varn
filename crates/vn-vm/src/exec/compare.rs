use crate::heap::{Heap, HeapObj};
use crate::value::VmValue;

#[inline(always)]
pub(crate) fn eq(a: VmValue, b: VmValue, heap: &Heap) -> bool {
    if heap.is_int(a) && heap.is_int(b) {
        return heap.as_int(a) == heap.as_int(b);
    }
    if a.is_f64() && b.is_f64() {
        return a.as_f64() == b.as_f64();
    }
    if a.is_bool() && b.is_bool() {
        return a.as_bool() == b.as_bool();
    }
    if a.is_null() && b.is_null() {
        return true;
    }
    if a.is_null() || b.is_null() {
        return false;
    }
    if a.is_char() || b.is_char() {
        return a.bits_eq(b);
    }
    if heap.is_int(a) && b.is_f64() {
        return (heap.as_int(a) as f64) == b.as_f64();
    }
    if a.is_f64() && heap.is_int(b) {
        return a.as_f64() == (heap.as_int(b) as f64);
    }

    if a.is_sso() && b.is_sso() {
        return a.bits_eq(b);
    }

    if a.is_sso() && b.is_heap() {
        if let Some(HeapObj::Str(s)) = heap.get(b.as_heap()) {
            if s.byte_len() != a.sso_len() {
                return false;
            }
            return a.sso_eq_bytes(s.as_str().as_bytes());
        }
        return false;
    }
    if b.is_sso() && a.is_heap() {
        if let Some(HeapObj::Str(s)) = heap.get(a.as_heap()) {
            if s.byte_len() != b.sso_len() {
                return false;
            }
            return b.sso_eq_bytes(s.as_str().as_bytes());
        }
        return false;
    }
    if a.is_heap() && b.is_heap() {
        let ai = a.as_heap();
        let bi = b.as_heap();
        if ai == bi {
            return true;
        }
        if is_value_instance(a, heap) && is_value_instance(b, heap) {
            return composite_eq(a, b, heap);
        }
        let (Some(x), Some(y)) = (heap.get(ai), heap.get(bi)) else {
            return false;
        };
        return match x {
            HeapObj::Str(sa) => matches!(y, HeapObj::Str(sb)
                if sa.byte_len() == sb.byte_len() && sa.as_str() == sb.as_str()),
            HeapObj::BigInt(p) => matches!(y, HeapObj::BigInt(q) if p == q),
            HeapObj::Decimal(p) => matches!(y, HeapObj::Decimal(q) if p == q),
            HeapObj::EnumVariant(_) | HeapObj::Object(_) | HeapObj::Tuple(_) => {
                std::mem::discriminant(x) == std::mem::discriminant(y) && composite_eq(a, b, heap)
            }
            HeapObj::Array(_)
            | HeapObj::Buffer(_)
            | HeapObj::Module(_)
            | HeapObj::FrozenModule(_)
            | HeapObj::VmClosure(_)
            | HeapObj::Class(_)
            | HeapObj::NativeFn(..)
            | HeapObj::BoundMethod(_)
            | HeapObj::Map(_)
            | HeapObj::Set(_)
            | HeapObj::Task(_)
            | HeapObj::TaskHandle(_)
            | HeapObj::Range(_)
            | HeapObj::Generator(_)
            | HeapObj::Spread(_) => false,
        };
    }

    if a.is_heap() && heap.is_int(b) {
        match heap.get(a.as_heap()) {
            Some(HeapObj::BigInt(av)) => {
                let rhs = num_bigint::BigInt::from(heap.as_int(b));
                return **av == rhs;
            }
            Some(HeapObj::Decimal(da)) => {
                let rhs = bigdecimal::BigDecimal::from(heap.as_int(b));
                return **da == rhs;
            }
            Some(
                HeapObj::EnumVariant(_)
                | HeapObj::Str(_)
                | HeapObj::Array(_)
                | HeapObj::Tuple(_)
                | HeapObj::Object(_)
                | HeapObj::Buffer(_)
                | HeapObj::Module(_)
                | HeapObj::FrozenModule(_)
                | HeapObj::VmClosure(_)
                | HeapObj::Class(_)
                | HeapObj::NativeFn(..)
                | HeapObj::BoundMethod(_)
                | HeapObj::Map(_)
                | HeapObj::Set(_)
                | HeapObj::Task(_)
                | HeapObj::TaskHandle(_)
                | HeapObj::Range(_)
                | HeapObj::Generator(_)
                | HeapObj::Spread(_),
            )
            | None => return false,
        }
    }
    if heap.is_int(a) && b.is_heap() {
        match heap.get(b.as_heap()) {
            Some(HeapObj::BigInt(bv)) => {
                let lhs = num_bigint::BigInt::from(heap.as_int(a));
                return **bv == lhs;
            }
            Some(HeapObj::Decimal(db)) => {
                let lhs = bigdecimal::BigDecimal::from(heap.as_int(a));
                return lhs == **db;
            }
            Some(HeapObj::EnumVariant(_)) => return false,
            Some(
                HeapObj::Str(_)
                | HeapObj::Array(_)
                | HeapObj::Tuple(_)
                | HeapObj::Object(_)
                | HeapObj::Buffer(_)
                | HeapObj::Module(_)
                | HeapObj::FrozenModule(_)
                | HeapObj::VmClosure(_)
                | HeapObj::Class(_)
                | HeapObj::NativeFn(..)
                | HeapObj::BoundMethod(_)
                | HeapObj::Map(_)
                | HeapObj::Set(_)
                | HeapObj::Task(_)
                | HeapObj::TaskHandle(_)
                | HeapObj::Range(_)
                | HeapObj::Generator(_)
                | HeapObj::Spread(_),
            )
            | None => return false,
        }
    }
    false
}

fn is_value_instance(v: VmValue, heap: &Heap) -> bool {
    heap.instance(v.as_heap())
        .and_then(|inst| vn_types::ClassObj::find_by_id(inst.class_id))
        .is_some_and(|class| class.layout().is_value)
}

fn expand_instances(
    a: VmValue,
    b: VmValue,
    heap: &Heap,
    pending: &mut Vec<(VmValue, VmValue, bool)>,
) -> Option<bool> {
    let ia = heap.instance(a.as_heap())?;
    let ib = heap.instance(b.as_heap())?;
    if ia.class_id != ib.class_id {
        return Some(false);
    }
    let class = vn_types::ClassObj::find_by_id(ia.class_id)?;
    let layout = class.layout();
    if !layout.is_value {
        return None;
    }
    let null = VmValue::null();
    for field in &layout.fields {
        pending.push((
            ia.read_field(field).unwrap_or(null),
            ib.read_field(field).unwrap_or(null),
            true,
        ));
    }
    Some(true)
}

fn composite_eq(a: VmValue, b: VmValue, heap: &Heap) -> bool {
    let mut pending = vec![(a, b, false)];
    while let Some((x, y, member)) = pending.pop() {
        match expand(x, y, member, heap, &mut pending) {
            Some(true) => {}
            Some(false) => return false,
            None => {
                if !eq(x, y, heap) {
                    return false;
                }
            }
        }
    }
    true
}

fn expand(
    a: VmValue,
    b: VmValue,
    member: bool,
    heap: &Heap,
    pending: &mut Vec<(VmValue, VmValue, bool)>,
) -> Option<bool> {
    if !(a.is_heap() && b.is_heap()) {
        return None;
    }
    if let Some(decided) = expand_instances(a, b, heap, pending) {
        return Some(decided);
    }
    let null = VmValue::null();
    let (x, y) = (heap.get(a.as_heap())?, heap.get(b.as_heap())?);
    match x {
        HeapObj::Tuple(x) => {
            let HeapObj::Tuple(y) = y else {
                return None;
            };
            Some(push_elements(x, y, pending, null))
        }
        HeapObj::Object(x) => {
            let HeapObj::Object(y) = y else {
                return None;
            };
            let (x, y) = (x.borrow(), y.borrow());
            let names = x.shape().ordered_names();
            if names.len() != y.shape().ordered_names().len() {
                return Some(false);
            }
            for (slot, name) in names.iter().enumerate() {
                let (Some(mine), Some(theirs)) = (x.field_at(slot), y.get(name)) else {
                    return Some(false);
                };
                pending.push((mine, theirs, true));
            }
            Some(true)
        }
        HeapObj::Array(x) => {
            let (true, HeapObj::Array(y)) = (member, y) else {
                return None;
            };
            Some(push_elements(x, y, pending, null))
        }
        HeapObj::EnumVariant(x) => {
            let HeapObj::EnumVariant(y) = y else {
                return None;
            };
            let same_enum = match (x.enum_class_id, y.enum_class_id) {
                (Some(p), Some(q)) => p == q,
                (Some(_), None) | (None, Some(_)) | (None, None) => {
                    x.def.enum_name == y.def.enum_name
                }
            };
            if !(same_enum && x.tag() == y.tag() && x.slots.len() == y.slots.len()) {
                return Some(false);
            }
            pending.extend(
                x.slots
                    .iter()
                    .zip(y.slots.iter())
                    .map(|(p, q)| (*p, *q, true)),
            );
            Some(true)
        }
        HeapObj::Str(_)
        | HeapObj::Buffer(_)
        | HeapObj::Module(_)
        | HeapObj::FrozenModule(_)
        | HeapObj::VmClosure(_)
        | HeapObj::Class(_)
        | HeapObj::NativeFn(..)
        | HeapObj::BoundMethod(_)
        | HeapObj::Map(_)
        | HeapObj::Set(_)
        | HeapObj::Task(_)
        | HeapObj::TaskHandle(_)
        | HeapObj::Range(_)
        | HeapObj::BigInt(_)
        | HeapObj::Decimal(_)
        | HeapObj::Generator(_)
        | HeapObj::Spread(_) => None,
    }
}

fn push_elements(
    x: &vn_types::VmArray,
    y: &vn_types::VmArray,
    pending: &mut Vec<(VmValue, VmValue, bool)>,
    null: VmValue,
) -> bool {
    if x.len() != y.len() {
        return false;
    }
    for i in 0..x.len() {
        pending.push((
            x.get_vm(i).unwrap_or(null),
            y.get_vm(i).unwrap_or(null),
            true,
        ));
    }
    true
}

#[inline(always)]
pub(crate) fn neq(a: VmValue, b: VmValue, heap: &Heap) -> bool {
    !eq(a, b, heap)
}

fn text_bytes<'a>(
    v: VmValue,
    heap: &'a Heap,
    buf: &'a mut [u8; vn_types::vm_value::SSO_MAX_LEN],
) -> Option<&'a [u8]> {
    if v.is_sso() {
        return Some(v.sso_as_str(buf).as_bytes());
    }
    if let Some(c) = v.as_char() {
        let (head, _) = buf.split_at_mut(4);
        return Some(c.encode_utf8(head).as_bytes());
    }
    if !v.is_heap() {
        return None;
    }
    match heap.get(v.as_heap()) {
        Some(HeapObj::Str(s)) => Some(s.as_str().as_bytes()),
        Some(
            HeapObj::Array(_)
            | HeapObj::Tuple(_)
            | HeapObj::Object(_)
            | HeapObj::Buffer(_)
            | HeapObj::Module(_)
            | HeapObj::FrozenModule(_)
            | HeapObj::VmClosure(_)
            | HeapObj::Class(_)
            | HeapObj::NativeFn(..)
            | HeapObj::BoundMethod(_)
            | HeapObj::Map(_)
            | HeapObj::Set(_)
            | HeapObj::Task(_)
            | HeapObj::TaskHandle(_)
            | HeapObj::Range(_)
            | HeapObj::Generator(_)
            | HeapObj::Spread(_)
            | HeapObj::BigInt(_)
            | HeapObj::Decimal(_)
            | HeapObj::EnumVariant(_),
        )
        | None => None,
    }
}

fn text_order(a: VmValue, b: VmValue, heap: &Heap) -> Option<std::cmp::Ordering> {
    let (mut left, mut right) = (
        [0u8; vn_types::vm_value::SSO_MAX_LEN],
        [0u8; vn_types::vm_value::SSO_MAX_LEN],
    );
    let left = text_bytes(a, heap, &mut left)?;
    let right = text_bytes(b, heap, &mut right)?;
    Some(left.cmp(right))
}

pub(crate) fn lt_heap(a: VmValue, b: VmValue, heap: &Heap) -> bool {
    if heap.is_int(a) && heap.is_int(b) {
        return heap.as_int(a) < heap.as_int(b);
    }
    if let Some(order) = text_order(a, b, heap) {
        return order == std::cmp::Ordering::Less;
    }
    if a.is_f64() && b.is_f64() {
        return a.as_f64() < b.as_f64();
    }
    if heap.is_int(a) && b.is_f64() {
        return (heap.as_int(a) as f64) < b.as_f64();
    }
    if a.is_f64() && heap.is_int(b) {
        return a.as_f64() < (heap.as_int(b) as f64);
    }
    if a.is_heap() && b.is_heap() {
        let ai = a.as_heap();
        let bi = b.as_heap();
        if let (Some(HeapObj::BigInt(ba)), Some(HeapObj::BigInt(bb))) = (heap.get(ai), heap.get(bi))
        {
            return ba < bb;
        }
        if let (Some(HeapObj::Decimal(da)), Some(HeapObj::Decimal(db))) =
            (heap.get(ai), heap.get(bi))
        {
            return da < db;
        }
    }
    if a.is_heap()
        && heap.is_int(b)
        && let Some(HeapObj::Decimal(da)) = heap.get(a.as_heap())
    {
        let rhs = bigdecimal::BigDecimal::from(heap.as_int(b));
        return **da < rhs;
    }
    if heap.is_int(a)
        && b.is_heap()
        && let Some(HeapObj::Decimal(db)) = heap.get(b.as_heap())
    {
        let lhs = bigdecimal::BigDecimal::from(heap.as_int(a));
        return lhs < **db;
    }
    heap.to_f64_val(a) < heap.to_f64_val(b)
}

pub(crate) fn lte_heap(a: VmValue, b: VmValue, heap: &Heap) -> bool {
    if heap.is_int(a) && heap.is_int(b) {
        return heap.as_int(a) <= heap.as_int(b);
    }
    !gt_heap(a, b, heap)
}

pub(crate) fn gt_heap(a: VmValue, b: VmValue, heap: &Heap) -> bool {
    lt_heap(b, a, heap)
}

pub(crate) fn gte_heap(a: VmValue, b: VmValue, heap: &Heap) -> bool {
    if heap.is_int(a) && heap.is_int(b) {
        return heap.as_int(a) >= heap.as_int(b);
    }
    !lt_heap(a, b, heap)
}

#[inline(always)]
pub(crate) fn logical_not(a: VmValue) -> VmValue {
    VmValue::from_bool(!a.as_bool())
}
