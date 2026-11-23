pub const DELEGATED_ITEM: &str = "<yield*>";
pub const BOUND_SOURCE: &str = "<bound>";
const REGEX_LITERAL_BINDING_PREFIX: &str = "<regex ";

pub fn is_regex_literal_binding(name: &str) -> bool {
    name.starts_with(REGEX_LITERAL_BINDING_PREFIX)
}

pub fn is_synthetic_name(name: &str) -> bool {
    name.starts_with('<')
}

pub fn regex_literal_binding(index: usize) -> String {
    format!("{REGEX_LITERAL_BINDING_PREFIX}{index}>")
}

crate::named_enum! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum MemberKey {
        Tag => "__tag",
        Value0 => "value0",
        RawValue => "rawValue",
        Length => "length",
        Size => "size",
        Name => "name",
        VariantName => "__variant_name__",
        Variant => "__variant__",
        Callable => "()",
        ToString => "toString",
        ValueOf => "valueOf",
        Iter => "iter",
        IterNext => "next",
        IterClose => "close",
        Push => "push",
        Pop => "pop",
        Shift => "shift",
        Unshift => "unshift",
        Slice => "slice",
        Join => "join",
        IndexOf => "indexOf",
        Includes => "includes",
        Split => "split",
        Chars => "chars",
        ToArray => "toArray",
        Class => "class",
        Type => "type",
        Fields => "fields",
        Methods => "methods",
        Keys => "keys",
        Values => "values",
        Entries => "entries",
        HasOwn => "hasOwn",
        Repeat => "repeat",
        PadStart => "padStart",
        PadEnd => "padEnd",
        Start => "start",
        End => "end",
        StartsWith => "startsWith",
        EndsWith => "endsWith",
        CharCodeAt => "charCodeAt",
        CodePointAt => "codePointAt",
        Substring => "substring",
        Substr => "substr",
        At => "at",
        LastIndexOf => "lastIndexOf",
        CharCode => "charCode",
        Message => "message",
        Stack => "stack",
        Cause => "cause",
        IsStatic => "isStatic",
        IsPrivate => "isPrivate",
        Inclusive => "inclusive",
        Stride => "stride",
    }
}

crate::named_enum! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum MetaKey {
        Type => MemberKey::Type.name(),
        Name => MemberKey::Name.name(),
        Class => MemberKey::Class.name(),
        Fields => MemberKey::Fields.name(),
        Methods => MemberKey::Methods.name(),
        Keys => MemberKey::Keys.name(),
        Values => MemberKey::Values.name(),
        Entries => MemberKey::Entries.name(),
        HasOwn => MemberKey::HasOwn.name(),
    }
}

impl MetaKey {
    pub const fn member(self) -> MemberKey {
        match self {
            Self::Type => MemberKey::Type,
            Self::Name => MemberKey::Name,
            Self::Class => MemberKey::Class,
            Self::Fields => MemberKey::Fields,
            Self::Methods => MemberKey::Methods,
            Self::Keys => MemberKey::Keys,
            Self::Values => MemberKey::Values,
            Self::Entries => MemberKey::Entries,
            Self::HasOwn => MemberKey::HasOwn,
        }
    }

    pub const fn index(self) -> i64 {
        self as i64
    }

    pub fn from_index(index: i64) -> Option<Self> {
        Self::ALL.into_iter().find(|key| key.index() == index)
    }
}

pub const PLAIN_OBJECT_TYPE_NAME: &str = "Object";
