crate::named_enum! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum RuntimeOp {
        Conform => "conform",
        Decode => "decode",
        VerifyCall => "verify-call",
        VerifySpread => "verify-spread",
        VerifyMethod => "verify-method",
        VerifyMethodSpread => "verify-method-spread",
        VerifySuper => "verify-super",
        VerifySuperSpread => "verify-super-spread",
        StrIndex => "str-index",
        Inherited => "inherited",
        ParseTyped => "parse-typed",
        Meta => "meta",
        Copy => "copy",
    }
}

impl RuntimeOp {
    pub const fn id(self) -> u64 {
        self as u64
    }
}
