use super::*;

impl<'a> Scopes<'a> {
    pub(super) fn match_binding(&mut self, name: Atom, range: SourceRange) {
        if self.interner.resolve(name).starts_with(char::is_uppercase) {
            return;
        }
        self.declare_match(name, range.start.offset, range.end.offset, false);
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
}
