use super::super::*;
use super::FastJsonParser;

impl<'a, 'ctx> FastJsonParser<'a, 'ctx> {
    pub(in super::super) fn parse_value(&mut self) -> Option<VmValue> {
        match self.peek()? {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => self.parse_string(),
            b't' => self.parse_true(),
            b'f' => self.parse_false(),
            b'n' => self.parse_null(),
            b'0'..=b'9' | b'-' => self.parse_number(),
            _ => None,
        }
    }

    #[inline(always)]
    pub(in super::super) fn parse_true(&mut self) -> Option<VmValue> {
        if self.bytes.get(self.pos..self.pos + 4) == Some(b"true") {
            self.pos += 4;
            Some(VmValue::bool_true())
        } else {
            None
        }
    }

    #[inline(always)]
    pub(in super::super) fn parse_false(&mut self) -> Option<VmValue> {
        if self.bytes.get(self.pos..self.pos + 5) == Some(b"false") {
            self.pos += 5;
            Some(VmValue::bool_false())
        } else {
            None
        }
    }

    #[inline(always)]
    pub(in super::super) fn parse_null(&mut self) -> Option<VmValue> {
        if self.bytes.get(self.pos..self.pos + 4) == Some(b"null") {
            self.pos += 4;
            Some(VmValue::null())
        } else {
            None
        }
    }
}
