use base64::Engine;
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256, Sha512};
use vn_core::StdErrorClass;
use vn_macros::natives;
use vn_types::NativeError;

fn random_size(what: &str, size: i64) -> Result<usize, NativeError> {
    usize::try_from(size)
        .ok()
        .filter(|n| *n <= vn_core::MAX_ALLOC_BYTES)
        .ok_or_else(|| {
            NativeError::range_error(format!(
                "{what}: size must be within 0..={}",
                vn_core::MAX_ALLOC_BYTES
            ))
        })
}


natives! {
    sha256(data) => hex::encode(Sha256::digest(data.as_bytes())),
    sha512(data) => hex::encode(Sha512::digest(data.as_bytes())),
    uuid() => uuid::Uuid::new_v4().to_string(),
    uuidV4() => uuid::Uuid::new_v4().to_string(),
    uuidV7() => uuid::Uuid::now_v7().to_string(),
    uuidValidate(s) => uuid::Uuid::parse_str(s).is_ok(),
    randomBytes(size) => {
        let size = random_size("crypto.randomBytes", size)?;
        let mut bytes = vec![0u8; size];
        rand::thread_rng().fill_bytes(&mut bytes);
        Ok(bytes.iter().map(|b| i64::from(*b)).collect::<Vec<i64>>())
    },
    randomHex(size) => {
        let size = random_size("crypto.randomHex", size)?;
        let mut bytes = vec![0u8; size];
        rand::thread_rng().fill_bytes(&mut bytes);
        Ok(hex::encode(bytes))
    },
    base64Enc(data) => base64::engine::general_purpose::STANDARD.encode(data.as_bytes()),
    base64Dec(data) => {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(data.as_bytes())
            .map_err(|e| NativeError::std(StdErrorClass::Parse, format!("crypto.base64_dec: {e}")))?;
        (String::from_utf8(decoded).map_err(|e| format!("crypto.base64_dec: {e}"))).map_err(|e| NativeError::std(StdErrorClass::Parse, e))
    },
    base64EncBytes(ctx, data) => {
        let bytes = ctx
            .buffer_to_bytes(data)
            .ok_or_else(|| NativeError::type_error("crypto.base64EncBytes: expected Bytes"))?;
        Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
    },
    base64DecBytes(ctx, data) => {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(data.as_bytes())
            .map_err(|e| NativeError::std(StdErrorClass::Parse, format!("crypto.base64_dec: {e}")))?;
        Ok(ctx.alloc_buffer_from_bytes(&decoded))
    },
    hmac(algo, key, data) => {
        let digest = match algo.to_lowercase().as_str() {
            "sha1" => {
                let mut mac = Hmac::<sha1::Sha1>::new_from_slice(key.as_bytes())
                    .map_err(|e| format!("crypto.hmac: {e}")).map_err(NativeError::range_error)?;
                mac.update(data.as_bytes());
                mac.finalize().into_bytes().to_vec()
            }
            "sha256" => {
                let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes())
                    .map_err(|e| format!("crypto.hmac: {e}")).map_err(NativeError::range_error)?;
                mac.update(data.as_bytes());
                mac.finalize().into_bytes().to_vec()
            }
            "sha512" => {
                let mut mac = Hmac::<Sha512>::new_from_slice(key.as_bytes())
                    .map_err(|e| format!("crypto.hmac: {e}")).map_err(NativeError::range_error)?;
                mac.update(data.as_bytes());
                mac.finalize().into_bytes().to_vec()
            }
            other => return Err(NativeError::range_error(format!("crypto.hmac: unsupported algorithm '{other}'"))),
        };
        Ok(hex::encode(digest))
    },
    hmacBytes(ctx, algo, key, data) => {
        let key_bytes = if let Some(b) = ctx.buffer_to_bytes(key) {
            b
        } else if let Some(s) = ctx.str_owned(key) {
            s.into_bytes()
        } else {
            ctx.str_repr(key).into_bytes()
        };
        let data_bytes = if let Some(b) = ctx.buffer_to_bytes(data) {
            b
        } else if let Some(s) = ctx.str_owned(data) {
            s.into_bytes()
        } else {
            ctx.str_repr(data).into_bytes()
        };
        let digest = match algo.to_lowercase().as_str() {
            "sha1" => {
                let mut mac = Hmac::<sha1::Sha1>::new_from_slice(&key_bytes)
                    .map_err(|e| format!("crypto.hmac: {e}")).map_err(NativeError::range_error)?;
                mac.update(&data_bytes);
                mac.finalize().into_bytes().to_vec()
            }
            "sha256" => {
                let mut mac = Hmac::<Sha256>::new_from_slice(&key_bytes)
                    .map_err(|e| format!("crypto.hmac: {e}")).map_err(NativeError::range_error)?;
                mac.update(&data_bytes);
                mac.finalize().into_bytes().to_vec()
            }
            "sha512" => {
                let mut mac = Hmac::<Sha512>::new_from_slice(&key_bytes)
                    .map_err(|e| format!("crypto.hmac: {e}")).map_err(NativeError::range_error)?;
                mac.update(&data_bytes);
                mac.finalize().into_bytes().to_vec()
            }
            other => return Err(NativeError::range_error(format!("crypto.hmac: unsupported algorithm '{other}'"))),
        };
        Ok(ctx.alloc_buffer_from_bytes(&digest))
    },
    randomBytesBuffer(ctx, size) => {
        let size = random_size("crypto.randomBytes", size)?;
        let mut bytes = vec![0u8; size];
        rand::thread_rng().fill_bytes(&mut bytes);
        Ok(ctx.alloc_buffer_from_bytes(&bytes))
    },
    pbkdf2(ctx, password, salt, iterations, key_len) => {
        let salt_bytes = if let Some(b) = ctx.buffer_to_bytes(salt) {
            b
        } else if let Some(s) = ctx.str_owned(salt) {
            s.into_bytes()
        } else {
            ctx.str_repr(salt).into_bytes()
        };
        if iterations <= 0 {
            return Err(NativeError::range_error("PBKDF2: iterations must be positive".to_string()));
        }
        let key_len = usize::try_from(key_len)
            .ok()
            .filter(|n| (1..=vn_core::MAX_ALLOC_BYTES).contains(n))
            .ok_or_else(|| NativeError::range_error(format!("PBKDF2: keyLen must be within 1..={}", vn_core::MAX_ALLOC_BYTES)))?;

        let mut out = vec![0u8; key_len];
        let hmac_len = 32usize;
        let num_blocks = key_len.div_ceil(hmac_len);
        let mut block_buf = Vec::with_capacity(salt_bytes.len() + 4);

        for i in 1..=num_blocks {
            let mut mac = Hmac::<Sha256>::new_from_slice(password.as_bytes())
                .map_err(|e| e.to_string()).map_err(NativeError::range_error)?;
            block_buf.clear();
            block_buf.extend_from_slice(&salt_bytes);
            block_buf.extend_from_slice(&(i as u32).to_be_bytes());
            mac.update(&block_buf);
            let mut u = mac.finalize().into_bytes();
            let mut t = u;

            for _ in 1..iterations {
                let mut next_mac = Hmac::<Sha256>::new_from_slice(password.as_bytes())
                    .map_err(|e| e.to_string()).map_err(NativeError::range_error)?;
                next_mac.update(&u);
                u = next_mac.finalize().into_bytes();
                for (tb, ub) in t.iter_mut().zip(u.iter()) {
                    *tb ^= *ub;
                }
            }

            let start = (i - 1) * hmac_len;
            let end = (i * hmac_len).min(key_len);
            let take = end - start;
            out[start..end].copy_from_slice(&t[..take]);
        }
        Ok(ctx.alloc_buffer_from_bytes(&out))
    },
    timingSafeEqual(ctx, a, b) => {
        let bytes_a = if let Some(buf) = ctx.buffer_to_bytes(a) {
            buf
        } else {
            ctx.str_repr(a).into_bytes()
        };
        let bytes_b = if let Some(buf) = ctx.buffer_to_bytes(b) {
            buf
        } else {
            ctx.str_repr(b).into_bytes()
        };
        if bytes_a.len() != bytes_b.len() {
            return Ok(false);
        }
        let mut diff = 0u8;
        for (x, y) in bytes_a.iter().zip(bytes_b.iter()) {
            diff |= x ^ y;
        }
        Ok(diff == 0)
    },
}
