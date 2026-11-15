use std::sync::atomic::{AtomicU64, Ordering};
use vn_macros::natives;
use vn_types::{VmValue};


static META_KEY_COUNTER: AtomicU64 = AtomicU64::new(1);

natives! {
    defineMetadata(ctx, metadataKey, metadataValue, target) => {
        let key = ctx.str_repr(metadataKey);
        ctx.define_metadata(target, &key, metadataValue);
        Ok(())
    },
    getMetadata(ctx, metadataKey, target) => {
        let key = ctx.str_repr(metadataKey);
        Ok(ctx.get_metadata(target, &key).unwrap_or(VmValue::null()))
    },
    hasMetadata(ctx, metadataKey, target) => {
        let key = ctx.str_repr(metadataKey);
        Ok(ctx.has_metadata(target, &key))
    },
    createMetaKey() => {
        let id = META_KEY_COUNTER.fetch_add(1, Ordering::Relaxed);
        Ok(format!("meta_{id}"))
    },
    setMetaKey(ctx, metaId, target, value) => {
        ctx.define_metadata(target, metaId, value);
        Ok(())
    },
    getMetaKey(ctx, metaId, target) => ctx.get_metadata(target, metaId).unwrap_or(VmValue::null()),
    hasMetaKey(ctx, metaId, target) => ctx.has_metadata(target, metaId),
}
