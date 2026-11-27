use super::super::*;
use super::FastJsonParser;

impl<'a, 'ctx> FastJsonParser<'a, 'ctx> {
    pub(in super::super) fn read_string_text(&mut self) -> Option<std::borrow::Cow<'a, str>> {
        let bytes: &'a [u8] = self.bytes;
        let text: &'a str = self.text;
        let start = self.pos + 1;
        let short_end = (start + 32).min(bytes.len());
        let mut at = start;
        while at < short_end {
            match bytes[at] {
                b'"' => {
                    self.pos = at + 1;
                    return Some(std::borrow::Cow::Borrowed(text.get(start..at)?));
                }
                b'\\' | 0..=0x1f => break,
                _ => at += 1,
            }
        }
        let first = memchr::memchr2(b'"', b'\\', &bytes[start..])? + start;
        if bytes[first] == b'"' {
            self.pos = first + 1;
            return Some(std::borrow::Cow::Borrowed(Self::plain_text(
                text, start, first,
            )?));
        }
        let mut out = String::with_capacity(first - start + 16);
        let mut chunk = start;
        let mut at = first;
        loop {
            out.push_str(Self::plain_text(text, chunk, at)?);
            if bytes[at] == b'"' {
                self.pos = at + 1;
                return Some(std::borrow::Cow::Owned(out));
            }
            let mut next = at + 2;
            match *bytes.get(at + 1)? {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'/' => out.push('/'),
                b'b' => out.push('\x08'),
                b'f' => out.push('\x0c'),
                b'n' => out.push('\n'),
                b'r' => out.push('\r'),
                b't' => out.push('\t'),
                b'u' => {
                    let high = Self::hex4(bytes, at + 2)?;
                    next = at + 6;
                    let code = if (0xD800..0xDC00).contains(&high) {
                        if bytes.get(next) != Some(&b'\\') || bytes.get(next + 1) != Some(&b'u') {
                            return None;
                        }
                        let low = Self::hex4(bytes, next + 2)?;
                        if !(0xDC00..0xE000).contains(&low) {
                            return None;
                        }
                        next += 6;
                        0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
                    } else {
                        high
                    };
                    out.push(std::char::from_u32(code)?);
                }
                _ => return None,
            }
            chunk = next;
            at = memchr::memchr2(b'"', b'\\', &bytes[chunk..])? + chunk;
        }
    }

    pub(super) fn plain_text(text: &'a str, from: usize, to: usize) -> Option<&'a str> {
        if text.as_bytes().get(from..to)?.iter().any(|&b| b < 0x20) {
            return None;
        }
        text.get(from..to)
    }

    pub(super) fn hex4(bytes: &[u8], at: usize) -> Option<u32> {
        let digits = bytes.get(at..at + 4)?;
        std::str::from_utf8(digits)
            .ok()
            .filter(|text| text.bytes().all(|b| b.is_ascii_hexdigit()))
            .and_then(|text| u32::from_str_radix(text, 16).ok())
    }

    #[inline]
    pub(in super::super) fn parse_string(&mut self) -> Option<VmValue> {
        let text = self.read_string_text()?;
        if let Some(sso) = VmValue::try_from_sso(&text) {
            return Some(sso);
        }
        Some(self.ctx.heap.alloc_str_dynamic(&text))
    }
}
