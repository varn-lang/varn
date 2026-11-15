use vn_core::StdErrorClass;
use vn_macros::natives;
use vn_types::NativeError;


natives! {
    alloc(ctx, size) => {
        if size < 0 {
            return Err(NativeError::range_error("buffer.alloc: size must be non-negative".to_string()));
        }
        Ok(ctx.alloc_buffer(size as usize))
    },
    fromBytes(ctx, bytes) => {
        let len = bytes.len(ctx);
        let mut raw = Vec::with_capacity(len);
        for i in 0..len {
            if let Some(v) = bytes.get(ctx, i) {
                raw.push(ctx.as_int(v) as u8);
            }
        }
        Ok(ctx.alloc_buffer_from_bytes(&raw))
    },
    fromString(ctx, s) => ctx.alloc_buffer_from_bytes(s.as_bytes()),
    len(ctx, buf) => {
        if !ctx.is_buffer(buf) {
            return Err(NativeError::type_error("buffer.len: argument is not a Buffer".to_string()));
        }
        Ok(ctx.buffer_len(buf) as i64)
    },
    getByte(ctx, buf, index) => {
        if !ctx.is_buffer(buf) {
            return Err(NativeError::type_error("buffer.getByte: argument is not a Buffer".to_string()));
        }
        let byte = ctx.buffer_get_byte(buf, index as usize)
            .ok_or_else(|| format!("buffer.getByte: index out of bounds ({index})")).map_err(NativeError::range_error)?;
        Ok(byte as i64)
    },
    setByte(ctx, buf, index, value) => {
        if !ctx.is_buffer(buf) {
            return Err(NativeError::type_error("buffer.setByte: argument is not a Buffer".to_string()));
        }
        if !ctx.buffer_set_byte(buf, index as usize, value as u8) {
            return Err(NativeError::range_error(format!("buffer.setByte: index out of bounds ({index})")));
        }
        Ok(())
    },
    slice(ctx, buf, start, end) => {
        if !ctx.is_buffer(buf) {
            return Err(NativeError::type_error("buffer.slice: argument is not a Buffer".to_string()));
        }
        let sub = ctx.buffer_slice(buf, start as usize, end as usize)
            .ok_or_else(|| "buffer.slice failed".to_string()).map_err(NativeError::range_error)?;
        Ok(sub)
    },
    toString(ctx, buf) => {
        if !ctx.is_buffer(buf) {
            return Err(NativeError::type_error("buffer.toString: argument is not a Buffer".to_string()));
        }
        (ctx.buffer_to_string(buf).ok_or_else(|| "buffer.toString: invalid UTF-8".to_string())).map_err(|e| NativeError::std(StdErrorClass::Parse, e))
    },
    toHex(ctx, buf) => {
        if !ctx.is_buffer(buf) {
            return Err(NativeError::type_error("buffer.toHex: argument is not a Buffer".to_string()));
        }
        let bytes = ctx.buffer_to_bytes(buf).ok_or_else(|| "buffer: could not read the Bytes argument".to_string()).map_err(NativeError::range_error)?;
        let mut out = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            use std::fmt::Write;
            let _ = write!(&mut out, "{:02x}", b);
        }
        Ok(out)
    },
    toBase64(ctx, buf) => {
        if !ctx.is_buffer(buf) {
            return Err(NativeError::type_error("buffer.toBase64: argument is not a Buffer".to_string()));
        }
        let bytes = ctx.buffer_to_bytes(buf).ok_or_else(|| "buffer: could not read the Bytes argument".to_string()).map_err(NativeError::range_error)?;
        use base64::Engine;
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
    },
    fill(ctx, buf, val) => {
        if !ctx.is_buffer(buf) {
            return Err(NativeError::type_error("buffer.fill: argument is not a Buffer".to_string()));
        }
        let len = ctx.buffer_len(buf);
        let b = val as u8;
        for i in 0..len {
            ctx.buffer_set_byte(buf, i, b);
        }
        Ok(())
    },
}
