use super::*;

pub(crate) fn array_length(val: VmValue, heap: &Heap) -> VmResult<VmValue> {
    if val.is_heap() {
        if let Some(HeapObj::Array(a) | HeapObj::Tuple(a)) = heap.get(val.as_heap()) {
            return Ok(VmValue::from_i32(a.len() as i32));
        }
        if let Some(HeapObj::Str(s)) = heap.get(val.as_heap()) {
            return Ok(VmValue::from_i32(s.chars().count() as i32));
        }
    }
    Err(RuntimeError::new("OpArrayLength: not an array"))
}

pub(crate) fn bytes_length(val: VmValue, heap: &Heap) -> VmResult<VmValue> {
    if val.is_heap()
        && let Some(HeapObj::Buffer(b)) = heap.get(val.as_heap())
    {
        return Ok(VmValue::from_i32(b.len() as i32));
    }
    Err(RuntimeError::new("OpBytesLength: not bytes"))
}

#[cold]
fn array_too_long() -> RuntimeError {
    RuntimeError::range_error(format!(
        "an array cannot hold more than {} elements",
        vn_core::MAX_ARRAY_LEN
    ))
}

pub(crate) fn array_push(arr: VmValue, val: VmValue, heap: &mut Heap) -> VmResult<()> {
    if arr.is_heap()
        && let Some(HeapObj::Array(a)) = heap.get(arr.as_heap())
    {
        if a.len() >= vn_core::MAX_ARRAY_LEN {
            return Err(array_too_long());
        }
        a.push_vm(val);
        heap.write_barrier(arr.as_heap(), val);
        return Ok(());
    }
    Err(RuntimeError::new("OpArrayPush: not an array"))
}

pub(crate) fn array_pop(arr: VmValue, heap: &mut Heap) -> VmResult<VmValue> {
    if arr.is_heap()
        && let Some(HeapObj::Array(a)) = heap.get(arr.as_heap())
    {
        let v = a.pop_vm().unwrap_or(VmValue::null());
        return Ok(v);
    }
    Err(RuntimeError::new("OpArrayPop: not an array"))
}

pub(crate) fn array_extend(dst: VmValue, src: VmValue, heap: &mut Heap) -> VmResult<()> {
    if !dst.is_heap() || !src.is_heap() {
        return Err(RuntimeError::new("OpArrayExtend: not arrays"));
    }
    let Some(HeapObj::Array(da)) = heap.get(dst.as_heap()) else {
        return Err(RuntimeError::new("OpArrayExtend: not arrays"));
    };
    let da_len = da.len();
    let mut items: Vec<VmValue> = Vec::new();
    match heap.get(src.as_heap()) {
        Some(HeapObj::Array(sa) | HeapObj::Tuple(sa)) => {
            let n = sa.len();
            if da_len.saturating_add(n) > vn_core::MAX_ARRAY_LEN {
                return Err(array_too_long());
            }
            for i in 0..n {
                items.push(sa.get_vm(i).unwrap_or(VmValue::null()));
            }
        }
        Some(HeapObj::Set(s)) => {
            let vals: Vec<VmValue> = s.read().iter().map(|k| k.0).collect();
            if da_len.saturating_add(vals.len()) > vn_core::MAX_ARRAY_LEN {
                return Err(array_too_long());
            }
            items.extend(vals);
        }
        Some(HeapObj::Map(m)) => {
            let entries: Vec<(VmValue, VmValue)> =
                m.read().iter().map(|(k, v)| (k.0, *v)).collect();
            if da_len.saturating_add(entries.len()) > vn_core::MAX_ARRAY_LEN {
                return Err(array_too_long());
            }
            for (k, v) in entries {
                items.push(heap.alloc_tuple_vm(vec![k, v]));
            }
        }
        Some(HeapObj::Str(s)) => {
            let chars: Vec<VmValue> = s.as_str().chars().map(VmValue::from_char).collect();
            if da_len.saturating_add(chars.len()) > vn_core::MAX_ARRAY_LEN {
                return Err(array_too_long());
            }
            items.extend(chars);
        }
        Some(HeapObj::Range(r)) => {
            let count = r.count();
            if count > vn_core::MAX_ARRAY_LEN as u128
                || da_len.saturating_add(count as usize) > vn_core::MAX_ARRAY_LEN
            {
                return Err(array_too_long());
            }
            let len = r.len().unwrap_or(0);
            for i in 0..len {
                let Some(raw) = r.nth(i) else { continue };
                let v = match r.elem {
                    vn_types::value::RangeElem::Int => VmValue::from_int(raw),
                    vn_types::value::RangeElem::Char => {
                        VmValue::from_char(vn_types::value::RangeData::char_of(raw))
                    }
                };
                items.push(v);
            }
        }
        Some(HeapObj::Buffer(b)) => {
            let bytes: Vec<VmValue> = b
                .as_slice()
                .iter()
                .map(|byte| VmValue::from_int(i64::from(*byte)))
                .collect();
            if da_len.saturating_add(bytes.len()) > vn_core::MAX_ARRAY_LEN {
                return Err(array_too_long());
            }
            items.extend(bytes);
        }
        _ => return Err(RuntimeError::new("OpArrayExtend: not arrays")),
    }
    for &item in &items {
        let Some(HeapObj::Array(da)) = heap.get(dst.as_heap()) else {
            return Err(RuntimeError::new("OpArrayExtend: not arrays"));
        };
        da.push_vm(item);
    }
    for &item in &items {
        heap.write_barrier(dst.as_heap(), item);
    }
    Ok(())
}
