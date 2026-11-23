use vn_macros::natives;
use vn_types::str_util::{
    char_len, char_range_to_bytes, find_bytes,
};
use crate::modules::types::regex::regex_argument;
use vn_types::{NativeCtx, NativeFnResult, VmValue};



natives! {
    class str {
        EMPTY => String::new(),
        fromCharCode(codes) => {
            let mut res = String::new();
            for &v in codes {
                if v.is_int()
                    && let Some(c) = std::char::from_u32(v.as_int() as u32) {
                        res.push(c);
                    }
            }
            res
        },
        join(arr, sep) => {
            let sep = sep.unwrap_or("");
            let mut out = String::with_capacity(arr.len().saturating_mul(sep.len() + 12));
            for (i, part) in arr.iter().enumerate() {
                if i > 0 {
                    out.push_str(sep);
                }
                out.push_str(part.as_str());
            }
            out
        },
        length(this) => char_len(this, this.is_ascii()) as i64,
        toString(this) => this.to_string(),
        toStr(this) => this.to_string(),
        valueOf(this) => this.to_string(),
        toLowerCase(this) => {
            if this.is_ascii() {
                let mut bytes = this.as_bytes().to_vec();
                bytes.make_ascii_lowercase();
                unsafe { String::from_utf8_unchecked(bytes) }
            } else {
                this.to_lowercase()
            }
        },
        toUpperCase(this) => {
            if this.is_ascii() {
                let mut bytes = this.as_bytes().to_vec();
                bytes.make_ascii_uppercase();
                unsafe { String::from_utf8_unchecked(bytes) }
            } else {
                this.to_uppercase()
            }
        },
        trim(ctx, this) => {
            let s = this.as_str();
            let t = s.trim();
            ctx.str_sub(this, t.as_ptr() as usize - s.as_ptr() as usize, t.len())
        },
        trimStart(ctx, this) => {
            let s = this.as_str();
            let t = s.trim_start();
            ctx.str_sub(this, t.as_ptr() as usize - s.as_ptr() as usize, t.len())
        },
        trimEnd(ctx, this) => {
            let s = this.as_str();
            let t = s.trim_end();
            ctx.str_sub(this, t.as_ptr() as usize - s.as_ptr() as usize, t.len())
        },
        includes(this, search) => find_bytes(this, search).is_some(),
        contains(this, search) => find_bytes(this, search).is_some(),
        compare(this, other) => match this.cmp(other) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        },
        startsWith(this, search) => this.starts_with(search),
        endsWith(this, search) => this.ends_with(search),
        indexOf(this, search) => vn_types::str_util::char_index_of(this, search).map(|at| at as i64),
        lastIndexOf(this, search) => vn_types::str_util::char_last_index_of(this, search).map(|at| at as i64),
        substring(ctx, this, start, end) => {
            let ascii = this.is_ascii();
            let len = char_len(this, ascii);
            let (s, e) = vn_types::str_util::substring_bounds(len, start, end).map_err(|message| {
                vn_types::NativeError::rethrow(vn_core::RuntimeErrorKind::RangeError, message, None)
            })?;
            let (bs, be) = char_range_to_bytes(this, ascii, s, e);
            Ok(ctx.str_sub(this, bs, be - bs))
        },
        slice(ctx, this, start, end) => {
            let b = this.as_bytes();
            if this.is_ascii() {
                let len = b.len() as i64;
                let si = vn_types::str_util::normalize_index(start, len).min(b.len());
                let ei = vn_types::str_util::normalize_index(end.unwrap_or(len), len).min(b.len()).max(si);
                return ctx.str_sub(this, si, ei - si);
            }
            let len = char_len(this, false);
            let si = vn_types::str_util::normalize_index(start, len as i64).min(len);
            let ei = vn_types::str_util::normalize_index(end.unwrap_or(len as i64), len as i64).min(len).max(si);
            let (bs, be) = char_range_to_bytes(this, false, si, ei);
            ctx.str_sub(this, bs, be - bs)
        },
        at(ctx, this, index) => {
            let b = this.as_bytes();
            if this.is_ascii() {
                let len = b.len() as i64;
                let idx = if index < 0 { len + index } else { index };
                if idx < 0 || idx >= len {
                    return None;
                }
                let idx = idx as usize;
                return Some(ctx.str_sub(this, idx, 1));
            }
            let len = char_len(this, false) as i64;
            let idx = if index < 0 { len + index } else { index };
            if idx < 0 || idx >= len {
                return None;
            }
            let (bs, be) = char_range_to_bytes(this, false, idx as usize, idx as usize + 1);
            Some(ctx.str_sub(this, bs, be - bs))
        },
        substr(ctx, this, start, length) => {
            let b = this.as_bytes();
            if this.is_ascii() {
                let len = b.len() as i64;
                let st = if start < 0 { (len + start).max(0) as usize } else { (start as usize).min(b.len()) };
                let count = length.map(|c| c.max(0) as usize).unwrap_or(b.len() - st);
                let end = (st + count).min(b.len());
                return ctx.str_sub(this, st, end - st);
            }
            let len = char_len(this, false);
            let st = if start < 0 { (len as i64 + start).max(0) as usize } else { (start as usize).min(len) };
            let count = length.map(|c| c.max(0) as usize).unwrap_or(len - st);
            let end = (st + count).min(len);
            let (bs, be) = char_range_to_bytes(this, false, st, end);
            ctx.str_sub(this, bs, be - bs)
        },
        replace(ctx, this, from, to) => {
            if let Some(re) = regex_argument(ctx, from) {
                return re.replacen(this.as_str(), 1, to).into_owned();
            }
            let from = ctx.str_repr_borrowed(from);
            this.replacen(from.as_ref(), to, 1)
        },
        replaceAll(ctx, this, from, to) => {
            if let Some(re) = regex_argument(ctx, from) {
                return re.replace_all(this.as_str(), to).into_owned();
            }
            let from = ctx.str_repr_borrowed(from);
            if from.is_empty() {
                return this.to_string();
            }
            this.replace(from.as_ref(), to)
        },
        chars(this) => this.as_str().chars().map(VmValue::from_char).collect::<Vec<_>>(),
        split(ctx, this, separator) => {
            if let Some(sep) = separator
                && let Some(re) = regex_argument(ctx, sep)
            {
                let base = this.as_str().as_ptr() as usize;
                let parts: Vec<(usize, usize)> = re
                    .split(this.as_str())
                    .map(|p| (p.as_ptr() as usize - base, p.len()))
                    .collect();
                return parts.into_iter().map(|(off, len)| ctx.str_sub(this, off, len)).collect::<Vec<_>>();
            }
            let separator = separator.map(|s| ctx.str_repr_borrowed(s).into_owned());
            let mut ranges = Vec::new();
            vn_types::str_util::split_ranges(this.as_str(), separator.as_deref(), &mut ranges);
            ranges.into_iter().map(|(off, len)| ctx.str_sub(this, off, len)).collect()
        },
        lines(ctx, this) => {
            let s = this.as_str();
            let base = s.as_ptr() as usize;
            s.lines().map(|l| ctx.str_sub(this, l.as_ptr() as usize - base, l.len())).collect::<Vec<_>>()
        },
        words(ctx, this) => {
            let s = this.as_str();
            let base = s.as_ptr() as usize;
            s.split_whitespace().map(|w| ctx.str_sub(this, w.as_ptr() as usize - base, w.len())).collect::<Vec<_>>()
        },
        charCodeAt(this, pos) => this.char_code_at(pos),
        charCode(this) => this.chars().next().map(|c| c as i64).unwrap_or(-1),
        codePointAt(this, pos) => this.char_code_at(pos),
        repeat(this, n) => {
            let count = usize::try_from(n).map_err(|_| {
                vn_types::NativeError::rethrow(
                    vn_core::RuntimeErrorKind::RangeError,
                    format!("repeat: count {n} is negative"),
                    None,
                )
            })?;
            if this.len().checked_mul(count).is_none_or(|bytes| bytes > vn_core::MAX_STR_BYTES) {
                return Err(vn_types::NativeError::range_error(format!(
                    "repeat: the result would exceed {} bytes",
                    vn_core::MAX_STR_BYTES
                )));
            }
            Ok(this.repeat(count))
        },
        padStart(this, target, pad) => {
            pad_target(target)?;
            Ok(pad_start(this, target, pad))
        },
        padEnd(this, target, pad) => {
            pad_target(target)?;
            Ok(pad_end(this, target, pad))
        },
        concat(this, other) => format!("{this}{other}"),
        isEmpty(this) => this.is_empty(),
        isBlank(this) => this.trim().is_empty(),
        isDigit(this) => !this.is_empty() && this.chars().all(|c| c.is_ascii_digit()),
        isLetter(this) => !this.is_empty() && this.chars().all(|c| c.is_alphabetic()),
        isWhitespace(this) => !this.is_empty() && this.chars().all(|c| c.is_whitespace()),
        reverse(this) => this.chars().rev().collect::<String>(),
        capitalize(this) => {
            let mut chars = this.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        },
        toInt(this) => this.trim().parse::<i64>().unwrap_or(0),
        toFloat(this) => this.trim().parse::<f64>().unwrap_or(0.0),
    }
}

fn pad_target(target: i64) -> Result<(), vn_types::NativeError> {
    let limit = (vn_core::MAX_STR_BYTES / 4) as i64;
    if target > limit {
        return Err(vn_types::NativeError::range_error(format!(
            "pad: the target length {target} exceeds {limit}"
        )));
    }
    Ok(())
}

fn pad_start(s: &str, target: i64, pad: Option<&str>) -> String {
    let target = target.max(0) as usize;
    let pad = pad.filter(|p| !p.is_empty()).unwrap_or(" ");
    let len = char_len(s, s.is_ascii());
    if len >= target {
        return s.to_string();
    }
    let needed = target - len;
    let prefix: String = pad.chars().cycle().take(needed).collect();
    format!("{prefix}{s}")
}

fn pad_end(s: &str, target: i64, pad: Option<&str>) -> String {
    let target = target.max(0) as usize;
    let pad = pad.filter(|p| !p.is_empty()).unwrap_or(" ");
    let len = char_len(s, s.is_ascii());
    if len >= target {
        return s.to_string();
    }
    let needed = target - len;
    let suffix: String = pad.chars().cycle().take(needed).collect();
    format!("{s}{suffix}")
}

pub fn str_from_char_code(ctx: &mut dyn NativeCtx, args: &[VmValue]) -> NativeFnResult {
    let mut res = String::new();
    for &v in args {
        if v.is_int()
            && let Some(c) = std::char::from_u32(v.as_int() as u32)
        {
            res.push(c);
        }
    }
    Ok(ctx.alloc_str_owned(res))
}

pub fn str_from_value(ctx: &mut dyn NativeCtx, args: &[VmValue]) -> NativeFnResult {
    if let Some(&v) = args.first() {
        return Ok(ctx.alloc_str_owned(ctx.str_repr(v)));
    }
    Ok(ctx.alloc_str(""))
}
