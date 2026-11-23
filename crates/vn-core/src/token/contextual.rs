crate::named_enum! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ContextualKeyword {
        Keyof => "keyof",
        Infer => "infer",
        Satisfies => "satisfies",
        Meta => "meta",
    }
}
