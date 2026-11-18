pub const CONFORM: &str = "conform";
pub const DECODE: &str = "decode";
pub const VERIFY_CALL: &str = "verify-call";
pub const VERIFY_SPREAD: &str = "verify-spread";
pub const VERIFY_METHOD: &str = "verify-method";
pub const VERIFY_METHOD_SPREAD: &str = "verify-method-spread";
pub const VERIFY_SUPER: &str = "verify-super";
pub const VERIFY_SUPER_SPREAD: &str = "verify-super-spread";
pub const STR_INDEX: &str = "str-index";
pub const INHERITED: &str = "inherited";
pub const PARSE_TYPED: &str = "parse-typed";
pub const META: &str = "meta";
pub const COPY: &str = "copy";

pub const ALL: [&str; 13] = [
    CONFORM,
    DECODE,
    VERIFY_CALL,
    VERIFY_SPREAD,
    VERIFY_METHOD,
    VERIFY_METHOD_SPREAD,
    VERIFY_SUPER,
    VERIFY_SUPER_SPREAD,
    STR_INDEX,
    INHERITED,
    PARSE_TYPED,
    META,
    COPY,
];

pub const COUNT: u64 = ALL.len() as u64;

pub fn op_id(name: &str) -> u64 {
    try_op_id(name).unwrap_or_else(|| panic!("unknown VM runtime op `{name}`"))
}

pub fn try_op_id(name: &str) -> Option<u64> {
    ALL.iter().position(|n| *n == name).map(|i| i as u64)
}
