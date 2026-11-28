use std::collections::HashSet;

#[derive(Clone, Debug, Default)]
pub struct TypeFacts {
    references: HashSet<(u32, u32)>,
}

impl TypeFacts {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_reference(&mut self, start: u32, end: u32) {
        self.references.insert((start, end));
    }

    pub fn is_reference(&self, start: u32, end: u32) -> bool {
        self.references.contains(&(start, end))
    }
}
