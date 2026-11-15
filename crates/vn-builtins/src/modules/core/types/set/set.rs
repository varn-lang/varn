use vn_macros::natives;
use vn_types::value::SetRef;
use vn_types::{NativeCtx, VmValue};


fn get_set(ctx: &dyn NativeCtx, this: VmValue) -> Option<SetRef> {
    ctx.as_set(this)
}

natives! {
    class Set {
        constructor(ctx, _this, items) => {
            let set = ctx.alloc_set(Vec::new());
            let (Some(items), Some(target)) = (items, ctx.as_set(set)) else {
                return set;
            };
            for index in 0..items.len(ctx) {
                let Some(item) = items.get(ctx, index) else {
                    continue;
                };
                let Ok(key) = ctx.map_key(item) else {
                    continue;
                };
                target.write().insert(key);
                ctx.collection_write_barrier(set, key.0);
            }
            set
        },
        add(ctx, this, value) => {
            if let Some(s) = get_set(ctx, this) {
                let k = ctx.map_key(value)?;
                s.write().insert(k);


                ctx.collection_write_barrier(this, k.0);
            }
            Ok(())
        },
        has(ctx, this, value) => {
            match get_set(ctx, this) {
                Some(s) => {
                    let k = ctx.map_key(value)?;
                    Ok(s.read().contains(&k))
                }
                None => Ok(false),
            }
        },
        delete(ctx, this, value) => {
            match get_set(ctx, this) {
                Some(s) => {
                    let k = ctx.map_key(value)?;
                    Ok(s.write().remove(&k))
                }
                None => Ok(false),
            }
        },
        clear(ctx, this) => {
            if let Some(s) = get_set(ctx, this) {
                s.write().clear();
            }
        },
        values(ctx, this) => {
            match get_set(ctx, this) {
                Some(s) => s.read().iter().map(|k| k.0).collect(),
                None => Vec::new(),
            }
        },
        forEach(ctx, this, callback) => {
            if let Some(s) = get_set(ctx, this) {
                let items: Vec<VmValue> = s.read().iter().map(|k| k.0).collect();
                for v in items {
                    ctx.call_vm(callback, &[v, v, this])?;
                }
            }
            Ok(())
        },
        size(ctx, this) => get_set(ctx, this).map(|s| s.read().len() as i64).unwrap_or(0),
    }
}
