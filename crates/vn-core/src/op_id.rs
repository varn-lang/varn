use crate::runtime_kind::RuntimeKind;

pub const CORE_MODULE: &str = "globals";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnownOp {
    ArrayPush,
    IntToString,
    StrStartsWith,
    StrEndsWith,
    StrSlice,
    StrIndexOf,
    StrIncludes,
    StrSplit,
    StrSubstring,
    StrCharCodeAt,
    StrCodePointAt,
    GeneratorNext,
    ArrayNew,
    MapNew,
    SetNew,
}

impl KnownOp {
    pub const ALL: [KnownOp; 15] = [
        KnownOp::ArrayPush,
        KnownOp::IntToString,
        KnownOp::StrStartsWith,
        KnownOp::StrEndsWith,
        KnownOp::StrSlice,
        KnownOp::StrIndexOf,
        KnownOp::StrIncludes,
        KnownOp::StrSplit,
        KnownOp::StrSubstring,
        KnownOp::StrCharCodeAt,
        KnownOp::StrCodePointAt,
        KnownOp::GeneratorNext,
        KnownOp::ArrayNew,
        KnownOp::MapNew,
        KnownOp::SetNew,
    ];

    pub const fn id(self) -> u64 {
        crate::runtime_ops::RuntimeOp::COUNT as u64 + self as u64
    }

    pub fn names(self) -> (RuntimeKind, &'static str) {
        match self {
            KnownOp::ArrayPush => (RuntimeKind::Array, crate::MemberKey::Push.name()),
            KnownOp::IntToString => (RuntimeKind::Int, crate::MemberKey::ToString.name()),
            KnownOp::StrStartsWith => (RuntimeKind::Str, crate::MemberKey::StartsWith.name()),
            KnownOp::StrEndsWith => (RuntimeKind::Str, crate::MemberKey::EndsWith.name()),
            KnownOp::StrSlice => (RuntimeKind::Str, crate::MemberKey::Slice.name()),
            KnownOp::StrIndexOf => (RuntimeKind::Str, crate::MemberKey::IndexOf.name()),
            KnownOp::StrIncludes => (RuntimeKind::Str, crate::MemberKey::Includes.name()),
            KnownOp::StrSplit => (RuntimeKind::Str, crate::MemberKey::Split.name()),
            KnownOp::StrSubstring => (RuntimeKind::Str, crate::MemberKey::Substring.name()),
            KnownOp::StrCharCodeAt => (RuntimeKind::Str, crate::MemberKey::CharCodeAt.name()),
            KnownOp::StrCodePointAt => (RuntimeKind::Str, crate::MemberKey::CodePointAt.name()),
            KnownOp::GeneratorNext => (RuntimeKind::Generator, crate::MemberKey::IterNext.name()),
            KnownOp::ArrayNew => (RuntimeKind::Array, crate::TokenKind::Constructor.as_str()),
            KnownOp::MapNew => (RuntimeKind::Map, crate::TokenKind::Constructor.as_str()),
            KnownOp::SetNew => (RuntimeKind::Set, crate::TokenKind::Constructor.as_str()),
        }
    }

    pub fn from_names(class: &str, method: &str) -> Option<KnownOp> {
        KnownOp::ALL.into_iter().find(|op| {
            let (kind, name) = op.names();
            kind.name() == class && name == method
        })
    }
}

pub const FIRST_REGISTRY_ID: u64 =
    crate::runtime_ops::RuntimeOp::COUNT as u64 + KnownOp::ALL.len() as u64;

pub const CORE_CLASSES: [RuntimeKind; 14] = [
    RuntimeKind::Array,
    RuntimeKind::Str,
    RuntimeKind::Bytes,
    RuntimeKind::Map,
    RuntimeKind::Set,
    RuntimeKind::Range,
    RuntimeKind::Generator,
    RuntimeKind::AsyncGenerator,
    RuntimeKind::Int,
    RuntimeKind::Float,
    RuntimeKind::Bool,
    RuntimeKind::Char,
    RuntimeKind::Decimal,
    RuntimeKind::BigInt,
];

#[inline]
pub fn is_core_class(tag: RuntimeKind) -> bool {
    CORE_CLASSES.contains(&tag)
}

pub fn core_class_names() -> impl Iterator<Item = &'static str> {
    CORE_CLASSES.into_iter().filter_map(core_class_name).chain(
        crate::NumKind::ALL
            .into_iter()
            .filter(|kind| !kind.is_default_width())
            .map(crate::NumKind::name),
    )
}

pub fn core_class_name(tag: RuntimeKind) -> Option<&'static str> {
    is_core_class(tag).then(|| tag.name())
}
