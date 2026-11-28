use super::*;

impl<'a> Scopes<'a> {
    pub(super) fn match_binding(&mut self, name: Atom, arm: SourceRange) {
        if self.interner.resolve(name).starts_with(char::is_uppercase) {
            return;
        }
        let (start, end) = self.locate_match_binding(name, arm).unwrap_or((0, 0));
        self.declare_match(name, start, end, false);
    }

    pub(super) fn declare_match(&mut self, name: Atom, start: u32, end: u32, shorthand: bool) {
        self.declare(
            name,
            start,
            end,
            Declare {
                kind: Kind::MatchBinding,
                exported: false,
                initialized: true,
                group: None,
                param_of: None,
                renamable: end > start,
                shorthand,
                import: None,
            },
        );
    }

    fn locate_match_binding(&self, name: Atom, arm: SourceRange) -> Option<(u32, u32)> {
        let text = self.interner.resolve(name);
        let from = arm.start.offset as usize;
        let upto = (arm.end.offset as usize).min(self.text.len());
        let region = self.text.get(from..upto)?;
        let pattern = &region[..region.find("=>").unwrap_or(region.len())];
        let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
        pattern.match_indices(text).find_map(|(at, found)| {
            let before = pattern[..at].chars().next_back();
            let after = pattern[at + found.len()..].chars().next();
            let bounded = !before.is_some_and(|c| is_word(c) || c == '.')
                && !after.is_some_and(|c| is_word(c) || c == '(' || c == '.');
            bounded.then_some(((from + at) as u32, (from + at + found.len()) as u32))
        })
    }
}
