use super::super::*;
use super::FastJsonParser;

impl<'a, 'ctx> FastJsonParser<'a, 'ctx> {
    #[inline]
    pub(super) fn parse_array(&mut self) -> Option<VmValue> {
        self.pos += 1;
        let est_capacity = ((self.bytes.len() - self.pos) / 32).clamp(16, 65536);
        let mut items = Vec::with_capacity(est_capacity);
        loop {
            self.skip_whitespace();
            if self.pos < self.bytes.len() && self.bytes[self.pos] == b']' {
                self.pos += 1;
                return Some(self.ctx.alloc_array_inferred(items));
            }
            let elem = self.parse_value()?;
            items.push(elem);
            self.skip_whitespace();
            if self.pos < self.bytes.len() {
                let b = self.bytes[self.pos];
                if b == b',' {
                    self.pos += 1;
                } else if b == b']' {
                    self.pos += 1;
                    return Some(self.ctx.alloc_array_inferred(items));
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
    }

    #[inline]
    pub(super) fn parse_object(&mut self) -> Option<VmValue> {
        self.pos += 1;
        self.skip_whitespace();
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'}' {
            self.pos += 1;
            return Some(self.ctx.alloc_object());
        }
        let depth = self.depth;
        self.depth += 1;
        let parsed = self.parse_object_fields(depth);
        self.depth -= 1;
        parsed
    }

    pub(super) fn remember_shape(&mut self, depth: usize, shape_index: usize) {
        if self.ctx.json_hints.len() <= depth {
            self.ctx.json_hints.resize(depth + 1, usize::MAX);
        }
        self.ctx.json_hints[depth] = shape_index;
    }

    pub(super) fn parse_object_fields(&mut self, depth: usize) -> Option<VmValue> {
        const INLINE_FIELDS: usize = 16;
        let mut inline = [VmValue::null(); INLINE_FIELDS];
        let mut spilled: Vec<VmValue> = Vec::new();
        let mut n = 0usize;
        let mut resume_after_value = false;

        let predicted = self
            .ctx
            .json_hints
            .get(depth)
            .copied()
            .filter(|&h| h < self.ctx.json_shapes.len())
            .filter(|&h| self.ctx.json_shapes[h].tokens.len() <= INLINE_FIELDS);
        if let Some(h) = predicted {
            let entry_len = self.ctx.json_shapes[h].tokens.len();
            while n < entry_len {
                self.skip_whitespace();
                let Some(token) = self.ctx.json_shapes[h].tokens[n].as_deref() else {
                    break;
                };
                if !self.bytes[self.pos..].starts_with(token) {
                    break;
                }
                self.pos += token.len();
                let val = self.parse_value()?;
                inline[n] = val;
                n += 1;
                self.skip_whitespace();
                match self.bytes.get(self.pos) {
                    Some(b',') if n < entry_len => self.pos += 1,
                    Some(b'}') if n == entry_len => {
                        self.pos += 1;
                        let shape = std::rc::Rc::clone(&self.ctx.json_shapes[h].shape);
                        return Some(
                            self.ctx
                                .heap
                                .alloc_object_with_shape_slice(&shape, &inline[..n]),
                        );
                    }
                    _ => {
                        resume_after_value = true;
                        break;
                    }
                }
            }
        }

        let known = self.ctx.json_shapes.len();
        let mut candidates: u64 = if known >= 64 {
            u64::MAX
        } else {
            (1u64 << known) - 1
        };
        if let Some(h) = predicted
            && n > 0
        {
            let shapes = &self.ctx.json_shapes;
            let prefix = &shapes[h].keys[..n];
            candidates = (0..known.min(64))
                .filter(|&i| shapes[i].keys.len() >= n && shapes[i].keys[..n] == *prefix)
                .fold(0u64, |mask, i| mask | (1u64 << i));
        }
        let mut owned_keys: Vec<String> = Vec::new();
        let mut cold = candidates == 0;
        if cold
            && n > 0
            && let Some(h) = predicted
        {
            owned_keys = self.ctx.json_shapes[h].keys[..n].to_vec();
        }

        loop {
            if !resume_after_value {
                self.skip_whitespace();
                if self.pos >= self.bytes.len() || self.bytes[self.pos] != b'"' {
                    return None;
                }
                let key_text = self.read_string_text()?;
                let key_str: &str = &key_text;

                self.skip_whitespace();
                if self.pos >= self.bytes.len() || self.bytes[self.pos] != b':' {
                    return None;
                }
                self.pos += 1;

                let val = self.parse_value()?;
                if n < INLINE_FIELDS {
                    inline[n] = val;
                } else {
                    if spilled.is_empty() {
                        spilled.extend_from_slice(&inline);
                    }
                    spilled.push(val);
                }

                if cold {
                    owned_keys.push(key_str.to_owned());
                } else {
                    let shapes = &self.ctx.json_shapes;
                    let mut next = 0u64;
                    let mut rest = candidates;
                    while rest != 0 {
                        let i = rest.trailing_zeros() as usize;
                        rest &= rest - 1;
                        if shapes[i].keys.get(n).map(String::as_str) == Some(key_str) {
                            next |= 1u64 << i;
                        }
                    }
                    if next == 0 {
                        let pivot = candidates.trailing_zeros() as usize;
                        owned_keys = shapes[pivot].keys[..n].to_vec();
                        owned_keys.push(key_str.to_owned());
                        cold = true;
                    } else {
                        candidates = next;
                    }
                }
                n += 1;
            }
            resume_after_value = false;

            self.skip_whitespace();
            if self.pos < self.bytes.len() {
                let b = self.bytes[self.pos];
                if b == b',' {
                    self.pos += 1;
                } else if b == b'}' {
                    self.pos += 1;
                    break;
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }

        let values: &[VmValue] = if spilled.is_empty() {
            &inline[..n]
        } else {
            &spilled
        };

        if !cold {
            let shapes = &self.ctx.json_shapes;
            let mut rest = candidates;
            while rest != 0 {
                let i = rest.trailing_zeros() as usize;
                rest &= rest - 1;
                if shapes[i].keys.len() == n {
                    let shape = std::rc::Rc::clone(&shapes[i].shape);
                    let obj = self.ctx.heap.alloc_object_with_shape_slice(&shape, values);
                    self.remember_shape(depth, i);
                    return Some(obj);
                }
            }
            let pivot = candidates.trailing_zeros() as usize;
            owned_keys = shapes[pivot].keys[..n].to_vec();
        }
        let obj = self.ctx.alloc_object();
        for (k, v) in owned_keys.iter().zip(values.iter()) {
            self.ctx.set_field(obj, k, *v);
        }
        if self.ctx.json_shapes.len() < JSON_SHAPE_CAP
            && let Some(shape) = self.ctx.get_object_shape(obj)
            && shape.property_names.len() == owned_keys.len()
        {
            self.ctx
                .json_shapes
                .push(CacheEntry::new(owned_keys, shape));
            let index = self.ctx.json_shapes.len() - 1;
            self.remember_shape(depth, index);
        }
        Some(obj)
    }
}
