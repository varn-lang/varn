use vn_macros::natives;
use vn_types::{NativeCtx, NativeError, VmValue};


fn byte_value(value: i64, what: &str) -> Result<u8, NativeError> {
    u8::try_from(value)
        .map_err(|_| NativeError::range_error(format!("{what}: {value} is not a byte (0..=255)")))
}

fn byte_index(ctx: &mut dyn NativeCtx, this: VmValue, index: i64) -> Result<usize, NativeError> {
    let len = ctx.buffer_len(this);
    usize::try_from(index)
        .ok()
        .filter(|i| *i < len)
        .ok_or_else(|| NativeError::range_error(format!("index {index} is out of range for length {len}")))
}

natives! {
    class Bytes {
        length(ctx, this) => ctx.buffer_len(this) as i64,
        getByte(ctx, this, index) => {
            let at = byte_index(ctx, this, index)?;
            ctx.buffer_get_byte(this, at)
                .map(i64::from)
                .ok_or_else(|| NativeError::type_error("getByte: not a Bytes"))
        },
        setByte(ctx, this, index, value) => {
            let at = byte_index(ctx, this, index)?;
            let byte = byte_value(value, "setByte")?;
            ctx.buffer_set_byte(this, at, byte);
            Ok(())
        },
        slice(ctx, this, start, end) => {
            let (from, to) = vn_core::slice_bounds(ctx.buffer_len(this), start, end);
            ctx.buffer_slice(this, from, to)
                .ok_or_else(|| NativeError::type_error("slice: not a Bytes"))
        },
        toString(ctx, this) => ctx.buffer_to_string(this).unwrap_or_default(),
        toHex(ctx, this) => {
            if let Some(bytes) = ctx.buffer_to_bytes(this) {
                let mut out = String::with_capacity(bytes.len() * 2);
                for b in bytes {
                    use std::fmt::Write;
                    let _ = write!(&mut out, "{:02x}", b);
                }
                out
            } else {
                String::new()
            }
        },
        toBase64(ctx, this) => {
            if let Some(bytes) = ctx.buffer_to_bytes(this) {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(&bytes)
            } else {
                String::new()
            }
        },
        fill(ctx, this, value, start, end) => {
            let byte = byte_value(value, "fill")?;
            let (from, to) = vn_core::slice_bounds(ctx.buffer_len(this), start, end);
            for idx in from..to {
                ctx.buffer_set_byte(this, idx, byte);
            }
            Ok(this)
        },
        copy(ctx, this, target, target_start, source_start, source_end) => {
            let target_len = ctx.buffer_len(target);
            let (from, to) = vn_core::slice_bounds(ctx.buffer_len(this), source_start, source_end);
            let at = vn_core::clamp_index(target_start.unwrap_or(0), target_len);
            let count = (to - from).min(target_len - at);
            for offset in 0..count {
                if let Some(byte) = ctx.buffer_get_byte(this, from + offset) {
                    ctx.buffer_set_byte(target, at + offset, byte);
                }
            }
            count as i64
        },
        alloc(ctx, size) => {
            let bytes = usize::try_from(size)
                .ok()
                .filter(|n| *n <= vn_core::MAX_ALLOC_BYTES)
                .ok_or_else(|| {
                    NativeError::range_error(format!(
                        "alloc: size {size} must be within 0..={}",
                        vn_core::MAX_ALLOC_BYTES
                    ))
                })?;
            Ok(ctx.alloc_buffer(bytes))
        },
        fromString(ctx, s) => ctx.alloc_buffer_from_bytes(s.as_bytes()),
        fromBytes(ctx, bytes) => {
            let len = bytes.len(ctx);
            let mut raw = Vec::with_capacity(len);
            for i in 0..len {
                let item = bytes
                    .get(ctx, i)
                    .filter(|v| ctx.is_int(*v))
                    .ok_or_else(|| NativeError::type_error(format!("fromBytes: element {i} is not an int")))?;
                let value = ctx.as_int(item);
                raw.push(byte_value(value, &format!("fromBytes: element {i}"))?);
            }
            Ok(ctx.alloc_buffer_from_bytes(&raw))
        },
    }
}
