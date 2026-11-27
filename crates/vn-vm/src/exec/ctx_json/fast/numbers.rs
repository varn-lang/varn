use super::super::*;
use super::FastJsonParser;

use super::super::number::fast_parse_f64;

impl<'a, 'ctx> FastJsonParser<'a, 'ctx> {
    #[inline]
    pub(in super::super) fn parse_number(&mut self) -> Option<VmValue> {
        let start = self.pos;
        let mut i = start;
        let negative = self.bytes.get(i) == Some(&b'-');
        if negative {
            i += 1;
        }
        let digits_start = i;
        while self.bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        let int_end = i;
        if int_end == digits_start
            || (self.bytes[digits_start] == b'0' && int_end - digits_start > 1)
        {
            return None;
        }
        let mut is_float = false;
        if self.bytes.get(i) == Some(&b'.') {
            is_float = true;
            i += 1;
            let frac = i;
            while self.bytes.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            if i == frac {
                return None;
            }
        }
        if matches!(self.bytes.get(i), Some(b'e' | b'E')) {
            is_float = true;
            i += 1;
            if matches!(self.bytes.get(i), Some(b'+' | b'-')) {
                i += 1;
            }
            let exp = i;
            while self.bytes.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            if i == exp {
                return None;
            }
        }
        self.pos = i;
        if !is_float {
            let mut value: i64 = 0;
            let mut fits = true;
            for &b in &self.bytes[digits_start..int_end] {
                let d = i64::from(b - b'0');
                let next = value.checked_mul(10).and_then(|v| {
                    if negative {
                        v.checked_sub(d)
                    } else {
                        v.checked_add(d)
                    }
                });
                match next {
                    Some(n) => value = n,
                    None => {
                        fits = false;
                        break;
                    }
                }
            }
            if fits {
                return Some(VmValue::from_int(value));
            }
        }
        let f = fast_parse_f64(&self.bytes[start..i])?;
        Some(VmValue::from_f64(f))
    }
}
