#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RuntimeOp {
    Conform,
    Decode,
    VerifyCall,
    VerifySpread,
    VerifyMethod,
    VerifyMethodSpread,
    VerifySuper,
    VerifySuperSpread,
    StrIndex,
    Inherited,
    ParseTyped,
    Meta,
    Copy,
}

impl RuntimeOp {
    pub const ALL: [Self; 13] = [
        Self::Conform,
        Self::Decode,
        Self::VerifyCall,
        Self::VerifySpread,
        Self::VerifyMethod,
        Self::VerifyMethodSpread,
        Self::VerifySuper,
        Self::VerifySuperSpread,
        Self::StrIndex,
        Self::Inherited,
        Self::ParseTyped,
        Self::Meta,
        Self::Copy,
    ];

    pub const COUNT: u64 = Self::ALL.len() as u64;

    pub const NAMES: [&'static str; 13] = [
        "conform",
        "decode",
        "verify-call",
        "verify-spread",
        "verify-method",
        "verify-method-spread",
        "verify-super",
        "verify-super-spread",
        "str-index",
        "inherited",
        "parse-typed",
        "meta",
        "copy",
    ];

    pub const fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    pub const fn id(self) -> u64 {
        self as u64
    }

    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "conform" => Self::Conform,
            "decode" => Self::Decode,
            "verify-call" => Self::VerifyCall,
            "verify-spread" => Self::VerifySpread,
            "verify-method" => Self::VerifyMethod,
            "verify-method-spread" => Self::VerifyMethodSpread,
            "verify-super" => Self::VerifySuper,
            "verify-super-spread" => Self::VerifySuperSpread,
            "str-index" => Self::StrIndex,
            "inherited" => Self::Inherited,
            "parse-typed" => Self::ParseTyped,
            "meta" => Self::Meta,
            "copy" => Self::Copy,
            _ => return None,
        })
    }
}
