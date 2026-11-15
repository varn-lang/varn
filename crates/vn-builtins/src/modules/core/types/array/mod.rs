#![allow(non_upper_case_globals)]

mod ordering;

use vn_macros::natives;
use vn_types::{NativeCtx, NativeError, VmValue, VnArray};


fn vm_eq(ctx: &dyn NativeCtx, a: VmValue, b: VmValue) -> bool {
    ctx.values_equal(a, b)
}

fn norm_idx(idx: i64, len: i64) -> usize {
    vn_core::clamp_index(idx, usize::try_from(len).unwrap_or(0))
}

fn room_for(ctx: &mut dyn NativeCtx, array: &VnArray, extra: usize) -> Result<(), NativeError> {
    if array.len(ctx).saturating_add(extra) > vn_core::MAX_ARRAY_LEN {
        return Err(NativeError::range_error(format!(
            "an array cannot hold more than {} elements",
            vn_core::MAX_ARRAY_LEN
        )));
    }
    Ok(())
}

fn flatten_one_level(ctx: &mut dyn NativeCtx, arr: VmValue, out: &mut Vec<VmValue>) {
    let len = ctx.array_len(arr);
    for i in 0..len {
        let v = ctx.array_get(arr, i).unwrap_or(VmValue::null());
        if ctx.is_array(v) {
            let inner = ctx.array_len(v);
            for j in 0..inner {
                out.push(ctx.array_get(v, j).unwrap_or(VmValue::null()));
            }
        } else {
            out.push(v);
        }
    }
}

natives! {
    class Array {
        constructor(ctx, _this) => ctx.alloc_array(Vec::new()),
        filled(length, value) => {
            let len = usize::try_from(length)
                .ok()
                .filter(|&n| n <= vn_core::MAX_ARRAY_LEN)
                .ok_or_else(|| NativeError::range_error(format!(
                    "an array length must be between 0 and {}, got {length}",
                    vn_core::MAX_ARRAY_LEN
                )))?;
            Ok(vec![value; len])
        },
        length(ctx, this) => this.len(ctx) as i64,
        push(ctx, this, item) => {
            room_for(ctx, &this, 1)?;
            this.push(ctx, item);
            Ok(())
        },
        pop(ctx, this) => this.pop(ctx),
        shift(ctx, this) => {
            let len = this.len(ctx);
            if len == 0 {
                return None;
            }
            let first = this.get(ctx, 0).unwrap_or_else(VmValue::null);
            for i in 1..len {
                let v = this.get(ctx, i).unwrap_or_else(VmValue::null);
                this.set(ctx, i - 1, v);
            }
            this.pop(ctx);
            Some(first)
        },
        unshift(ctx, this, item) => {
            room_for(ctx, &this, 1)?;
            let len = this.len(ctx);
            let mut items = Vec::with_capacity(len + 1);
            items.push(item);
            for i in 0..len {
                items.push(this.get(ctx, i).unwrap_or_else(VmValue::null));
            }
            this.push(ctx, VmValue::null());
            for (i, v) in items.into_iter().enumerate() {
                this.set(ctx, i, v);
            }
            Ok(())
        },
        join(ctx, this, separator) => {
            let sep = separator.unwrap_or(",");
            let len = this.len(ctx);
            if len == 0 {
                return String::new();
            }
            let mut out = String::with_capacity(len * (sep.len() + 8));
            let mut first = true;
            let mut buf = [0u8; vn_types::vm_value::SSO_MAX_LEN];
            for i in 0..len {
                if !first {
                    out.push_str(sep);
                } else {
                    first = false;
                }
                if let Some(v) = this.get(ctx, i) {
                    if v.is_sso() {
                        out.push_str(v.sso_as_str(&mut buf));
                    } else {
                        out.push_str(&ctx.str_repr_borrowed(v));
                    }
                }
            }
            out
        },
        toString(ctx, this) => {
            let len = this.len(ctx);
            let mut parts = Vec::with_capacity(len);
            for i in 0..len {
                let v = this.get(ctx, i).unwrap_or_else(VmValue::null);
                parts.push(ctx.str_repr(v));
            }
            parts.join(",")
        },
        slice(ctx, this, start, end) => {
            let len = this.len(ctx) as i64;
            let si = norm_idx(start.unwrap_or(0), len);
            let ei = norm_idx(end.unwrap_or(len), len).max(si);
            let mut items = Vec::with_capacity(ei.saturating_sub(si));
            for i in si..ei {
                if let Some(v) = this.get(ctx, i) {
                    items.push(v);
                }
            }
            items
        },
        at(ctx, this, index) => {
            let len = this.len(ctx) as i64;
            let idx = if index < 0 { len + index } else { index };
            if idx >= 0 && (idx as usize) < this.len(ctx) {
                this.get(ctx, idx as usize)
            } else {
                None
            }
        },
        entries(ctx, this) => {
            this.to_vec(ctx)
                .into_iter()
                .enumerate()
                .map(|(index, item)| {
                    let index = ctx.int_val(index as i64);
                    ctx.alloc_tuple(vec![index, item])
                })
                .collect()
        },
        includes(ctx, this, search) => {
            let len = this.len(ctx);
            for i in 0..len {
                if let Some(v) = this.get(ctx, i)
                    && vm_eq(ctx, v, search) {
                        return true;
                    }
            }
            false
        },
        indexOf(ctx, this, search) => {
            let len = this.len(ctx);
            for i in 0..len {
                if let Some(v) = this.get(ctx, i)
                    && vm_eq(ctx, v, search) {
                        return Some(i as i64);
                    }
            }
            None
        },
        lastIndexOf(ctx, this, search) => {
            let len = this.len(ctx);
            for i in (0..len).rev() {
                if let Some(v) = this.get(ctx, i)
                    && vm_eq(ctx, v, search) {
                        return Some(i as i64);
                    }
            }
            None
        },
        concat(ctx, this, items) => {
            let mut added = 0usize;
            for &other in items {
                added = added.saturating_add(if ctx.is_array(other) { ctx.array_len(other) } else { 1 });
            }
            room_for(ctx, &this, added)?;
            let mut out = this.to_vec(ctx);
            for &other in items {
                if ctx.is_array(other) {
                    let olen = ctx.array_len(other);
                    for j in 0..olen {
                        out.push(ctx.array_get(other, j).unwrap_or_else(VmValue::null));
                    }
                } else {
                    out.push(other);
                }
            }
            Ok(out)
        },
        fill(ctx, this, value, start, end) => {
            let len = this.len(ctx) as i64;
            let si = norm_idx(start.unwrap_or(0), len);
            let ei = norm_idx(end.unwrap_or(len), len);
            for i in si..ei {
                this.set(ctx, i, value);
            }
            this.raw()
        },
        flat(ctx, this) => {
            let mut out = Vec::new();
            flatten_one_level(ctx, this.raw(), &mut out);
            out
        },
        reverse(ctx, this) => {
            let mut items = this.to_vec(ctx);
            items.reverse();
            for (i, v) in items.iter().enumerate() {
                this.set(ctx, i, *v);
            }
            this.raw()
        },
        map(ctx, this, callback) => {
            let arr = this.raw();
            let len = this.len(ctx);
            let mut out = Vec::with_capacity(len);
            for i in 0..len {
                let v = this.get(ctx, i).unwrap_or_else(VmValue::null);
                out.push(ctx.call_vm(callback, &[v, VmValue::from_int(i as i64), arr])?);
            }
            Ok(out)
        },
        filter(ctx, this, predicate) => {
            let arr = this.raw();
            let len = this.len(ctx);
            let mut out = Vec::new();
            for i in 0..len {
                let v = this.get(ctx, i).unwrap_or_else(VmValue::null);
                if ctx.call_vm(predicate, &[v, VmValue::from_int(i as i64), arr])?.is_truthy() {
                    out.push(v);
                }
            }
            Ok(out)
        },
        find(ctx, this, predicate) => {
            let arr = this.raw();
            let len = this.len(ctx);
            for i in 0..len {
                let v = this.get(ctx, i).unwrap_or_else(VmValue::null);
                if ctx.call_vm(predicate, &[v, VmValue::from_int(i as i64), arr])?.is_truthy() {
                    return Ok(Some(v));
                }
            }
            Ok(None)
        },
        findIndex(ctx, this, predicate) => {
            let arr = this.raw();
            let len = this.len(ctx);
            for i in 0..len {
                let v = this.get(ctx, i).unwrap_or_else(VmValue::null);
                if ctx.call_vm(predicate, &[v, VmValue::from_int(i as i64), arr])?.is_truthy() {
                    return Ok(Some(i as i64));
                }
            }
            Ok(None)
        },
        forEach(ctx, this, callback) => {
            let arr = this.raw();
            let len = this.len(ctx);
            for i in 0..len {
                let v = this.get(ctx, i).unwrap_or_else(VmValue::null);
                ctx.call_vm(callback, &[v, VmValue::from_int(i as i64), arr])?;
            }
            Ok(())
        },
        flatMap(ctx, this, callback) => {
            let arr = this.raw();
            let len = this.len(ctx);
            let mut out = Vec::new();
            for i in 0..len {
                let v = this.get(ctx, i).unwrap_or_else(VmValue::null);
                let r = ctx.call_vm(callback, &[v, VmValue::from_int(i as i64), arr])?;
                if ctx.is_array(r) {
                    let rlen = ctx.array_len(r);
                    for j in 0..rlen {
                        out.push(ctx.array_get(r, j).unwrap_or_else(VmValue::null));
                    }
                } else {
                    out.push(r);
                }
            }
            Ok(out)
        },
        reduce(ctx, this, callback, initial) => {
            let arr = this.raw();
            let len = this.len(ctx);
            let (mut acc, start) = match initial {
                Some(init) => (init, 0usize),
                None if len > 0 => (this.get(ctx, 0).unwrap_or_else(VmValue::null), 1usize),
                None => return Err(NativeError::type_error("reduce of an empty array with no initial value")),
            };
            for i in start..len {
                let v = this.get(ctx, i).unwrap_or_else(VmValue::null);
                acc = ctx.call_vm(callback, &[acc, v, VmValue::from_int(i as i64), arr])?;
            }
            Ok(acc)
        },
        sort(ctx, this, compareFn) => {
            let mut items = this.to_vec(ctx);
            match compareFn.filter(|v| !v.is_null()) {
                Some(cb) => {
                    if items.len() > 1 {
                        for &item in &items {
                            ctx.verify_callback(cb, &[item, item])?;
                        }
                    }
                    ordering::merge_sort(&mut items, &mut |a, b| ordering::sign(ctx, cb, &[a, b]))?
                }
                None => ordering::merge_sort(&mut items, &mut |a, b| ordering::natural_cmp(ctx, a, b))?,
            }
            for (i, v) in items.iter().enumerate() {
                this.set(ctx, i, *v);
            }
            Ok(this.raw())
        },
        splice(ctx, this, start, deleteCount, items) => {
            let len = this.len(ctx) as i64;
            let st = if start < 0 { (len + start).max(0) as usize } else { (start as usize).min(len as usize) };
            let del = deleteCount
                .map(|d| d.max(0) as usize)
                .unwrap_or((len as usize) - st)
                .min((len as usize) - st);

            let mut all = this.to_vec(ctx);
            let removed: Vec<VmValue> = all.drain(st..st + del).collect();
            for (i, &v) in items.iter().enumerate() {
                all.insert(st + i, v);
            }

            let new_len = all.len();
            let old_len = len as usize;
            for (i, v) in all.into_iter().enumerate() {
                this.set(ctx, i, v);
            }
            for _ in new_len..old_len {
                this.pop(ctx);
            }
            removed
        },
        every(ctx, this, predicate) => {
            let arr = this.raw();
            let len = this.len(ctx);
            for i in 0..len {
                let v = this.get(ctx, i).unwrap_or_else(VmValue::null);
                if !ctx.call_vm(predicate, &[v, VmValue::from_int(i as i64), arr])?.is_truthy() {
                    return Ok(false);
                }
            }
            Ok(true)
        },
        some(ctx, this, predicate) => {
            let arr = this.raw();
            let len = this.len(ctx);
            for i in 0..len {
                let v = this.get(ctx, i).unwrap_or_else(VmValue::null);
                if ctx.call_vm(predicate, &[v, VmValue::from_int(i as i64), arr])?.is_truthy() {
                    return Ok(true);
                }
            }
            Ok(false)
        },
    }
}
