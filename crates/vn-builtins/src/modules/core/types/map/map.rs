use vn_macros::natives;
use vn_types::value::MapRef;
use vn_types::{NativeCtx, VmValue};


fn get_map(ctx: &dyn NativeCtx, this: VmValue) -> Option<MapRef> {
    ctx.as_map(this)
}

natives! {
    class Map {
        constructor(ctx, _this, entries) => {
            let map = ctx.alloc_map(Vec::new());
            let (Some(entries), Some(target)) = (entries, ctx.as_map(map)) else {
                return map;
            };
            for index in 0..entries.len(ctx) {
                let Some(entry) = entries.get(ctx, index) else {
                    continue;
                };
                let (Some(key), Some(value)) = (ctx.array_get(entry, 0), ctx.array_get(entry, 1)) else {
                    continue;
                };
                let Ok(key) = ctx.map_key(key) else {
                    continue;
                };
                target.write().insert(key, value);
                ctx.collection_write_barrier(map, key.0);
                ctx.collection_write_barrier(map, value);
            }
            map
        },
        get(ctx, this, key) => {
            let Some(m) = get_map(ctx, this) else {
                return Ok(None);
            };
            let k = ctx.map_key(key)?;
            let found = m.read().get(&k).copied();
            Ok(found)
        },
        set(ctx, this, key, value) => {
            if let Some(m) = get_map(ctx, this) {
                let k = ctx.map_key(key)?;
                m.write().insert(k, value);


                ctx.collection_write_barrier(this, k.0);
                ctx.collection_write_barrier(this, value);
            }
            Ok(())
        },
        has(ctx, this, key) => {
            match get_map(ctx, this) {
                Some(m) => {
                    let k = ctx.map_key(key)?;
                    Ok(m.read().contains_key(&k))
                }
                None => Ok(false),
            }
        },
        delete(ctx, this, key) => {
            match get_map(ctx, this) {
                Some(m) => {
                    let k = ctx.map_key(key)?;
                    Ok(m.write().remove(&k).is_some())
                }
                None => Ok(false),
            }
        },
        clear(ctx, this) => {
            if let Some(m) = get_map(ctx, this) {
                m.write().clear();
            }
        },
        keys(ctx, this) => {
            match get_map(ctx, this) {
                Some(m) => m.read().keys().map(|k| k.0).collect(),
                None => Vec::new(),
            }
        },
        values(ctx, this) => {
            match get_map(ctx, this) {
                Some(m) => m.read().values().copied().collect(),
                None => Vec::new(),
            }
        },
        entries(ctx, this) => {
            match get_map(ctx, this) {
                Some(m) => {
                    let pairs: Vec<(VmValue, VmValue)> =
                        m.read().iter().map(|(k, v)| (k.0, *v)).collect();
                    pairs
                        .into_iter()
                        .map(|(k, v)| ctx.alloc_tuple(vec![k, v]))
                        .collect()
                }
                None => Vec::new(),
            }
        },
        forEach(ctx, this, callback) => {
            if let Some(m) = get_map(ctx, this) {
                let pairs: Vec<(VmValue, VmValue)> =
                    m.read().iter().map(|(k, v)| (k.0, *v)).collect();
                for (k, v) in pairs {
                    ctx.call_vm(callback, &[v, k, this])?;
                }
            }
            Ok(())
        },
        size(ctx, this) => get_map(ctx, this).map(|m| m.read().len() as i64).unwrap_or(0),
    }
}
