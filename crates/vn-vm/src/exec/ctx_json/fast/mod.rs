use super::*;

mod containers;
mod literals;
mod numbers;
mod strings;

impl<'a, 'ctx> FastJsonParser<'a, 'ctx> {
    pub(super) fn new(ctx: &'ctx mut ExecCtx, text: &'a str) -> Self {
        FastJsonParser {
            text,
            bytes: text.as_bytes(),
            pos: 0,
            depth: 0,
            ctx,
        }
    }

    pub(super) fn at_end(&self) -> bool {
        self.pos == self.bytes.len()
    }

    pub(super) fn advance(&mut self, by: usize) {
        self.pos += by;
    }

    pub(super) fn peek_raw(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    #[inline(always)]
    pub(super) fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    #[inline(always)]
    pub(super) fn peek(&mut self) -> Option<u8> {
        self.skip_whitespace();
        self.bytes.get(self.pos).copied()
    }
}

pub(super) fn fast_parse_json(ctx: &mut ExecCtx, text: &str) -> Option<VmValue> {
    let mut parser = FastJsonParser {
        text,
        bytes: text.as_bytes(),
        pos: 0,
        depth: 0,
        ctx,
    };
    let val = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.pos == parser.bytes.len() {
        Some(val)
    } else {
        None
    }
}

pub(super) struct FastJsonParser<'a, 'ctx> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
    depth: usize,
    pub(super) ctx: &'ctx mut ExecCtx,
}
