use crate::{Atom, NumKind};

pub const TYPE_ALIASES_MODULE: &str = "core:types/aliases";

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[repr(u8)]
pub enum LangPrimitive {
    Null,
    Bool,
    Num(crate::NumKind),
    BigInt,
    Decimal,
    Char,
    Str,
    Void,
    Never,
    Dynamic,
    Error,
}

impl LangPrimitive {
    pub const INT: Self = Self::Num(crate::NumKind::INT);
    pub const FLOAT: Self = Self::Num(crate::NumKind::FLOAT);

    const COUNT: usize = 9 + NumKind::ALL.len();

    pub const ALL: [Self; Self::COUNT] = {
        const TAIL: [LangPrimitive; 7] = [
            LangPrimitive::BigInt,
            LangPrimitive::Decimal,
            LangPrimitive::Char,
            LangPrimitive::Str,
            LangPrimitive::Void,
            LangPrimitive::Never,
            LangPrimitive::Dynamic,
        ];
        let mut out = [Self::Null; Self::COUNT];
        out[1] = Self::Bool;
        let mut i = 0;
        while i < NumKind::ALL.len() {
            out[2 + i] = Self::Num(NumKind::ALL[i]);
            i += 1;
        }
        let mut j = 0;
        while j < TAIL.len() {
            out[2 + NumKind::ALL.len() + j] = TAIL[j];
            j += 1;
        }
        out
    };

    pub const fn name(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool => "bool",
            Self::Num(k) => k.name(),
            Self::BigInt => "bigint",
            Self::Decimal => "decimal",
            Self::Char => "char",
            Self::Str => "str",
            Self::Void => "void",
            Self::Never => "never",
            Self::Dynamic => "dynamic",
            Self::Error => "error",
        }
    }

    pub fn of(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }

    const IDENTITY: [(Atom, Option<Atom>, Self); Self::COUNT] = {
        let mut out = [(Atom::of_const(""), None, Self::Null); Self::COUNT];
        let mut i = 0;
        while i < Self::ALL.len() {
            let p = Self::ALL[i];
            let module = match p.core_module() {
                Some(m) => Some(Atom::of_const(m)),
                None => None,
            };
            out[i] = (Atom::of_const(p.name()), module, p);
            i += 1;
        }
        out
    };

    pub fn identify(name: Atom, origin: Option<Atom>) -> Option<Self> {
        let origin = origin?;
        Self::IDENTITY
            .iter()
            .find(|(n, m, _)| *n == name && *m == Some(origin))
            .map(|&(_, _, p)| p)
    }

    pub const fn core_module(self) -> Option<&'static str> {
        match self {
            Self::Bool => Some("core:types/bool"),
            Self::Num(k) => Some(k.core_module()),
            Self::BigInt => Some("core:types/bigint"),
            Self::Decimal => Some("core:types/decimal"),
            Self::Char => Some("core:types/char"),
            Self::Str => Some("core:types/str"),
            Self::Null | Self::Void | Self::Never | Self::Dynamic | Self::Error => None,
        }
    }

    pub const fn is_numeric(self) -> bool {
        matches!(self, Self::Num(_) | Self::BigInt | Self::Decimal)
    }
}

impl std::fmt::Display for LangPrimitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

crate::named_enum! {
    #[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
    )]
    #[repr(u8)]
    pub enum BuiltinType {
        Array => "Array",
        Map => "Map",
        Set => "Set",
        Range => "Range",
        Bytes => "Bytes",
        Task => "Task",
        TaskHandle => "TaskHandle",
        Generator => "Generator",
        AsyncGenerator => "AsyncGenerator",
        Class => "Class",
        ReadonlyArray => "ReadonlyArray",
        ReadonlyMap => "ReadonlyMap",
        ReadonlySet => "ReadonlySet",
    }
}

impl BuiltinType {
    const IDENTITY: [(Atom, Atom, Self); 13] = {
        let mut out = [(Atom::of_const(""), Atom::of_const(""), Self::Array); 13];
        let mut i = 0;
        while i < Self::ALL.len() {
            let b = Self::ALL[i];
            out[i] = (Atom::of_const(b.name()), Atom::of_const(b.core_module()), b);
            i += 1;
        }
        out
    };

    pub fn identify(name: Atom, origin: Option<Atom>) -> Option<Self> {
        let origin = origin?;
        Self::IDENTITY
            .iter()
            .find(|(n, m, _)| *n == name && *m == origin)
            .map(|&(_, _, b)| b)
    }

    pub const fn runtime_kind(self) -> crate::RuntimeKind {
        use crate::RuntimeKind as K;
        match self {
            Self::Array => K::Array,
            Self::Map => K::Map,
            Self::Set => K::Set,
            Self::Range => K::Range,
            Self::Bytes => K::Bytes,
            Self::Task => K::Task,
            Self::TaskHandle => K::TaskHandle,
            Self::Generator => K::Generator,
            Self::AsyncGenerator => K::AsyncGenerator,
            Self::Class => K::Class,
            Self::ReadonlyArray => K::Array,
            Self::ReadonlyMap => K::Map,
            Self::ReadonlySet => K::Set,
        }
    }

    pub const fn mutable_counterpart(self) -> Option<Self> {
        match self {
            Self::ReadonlyArray => Some(Self::Array),
            Self::ReadonlyMap => Some(Self::Map),
            Self::ReadonlySet => Some(Self::Set),
            Self::Array
            | Self::Map
            | Self::Set
            | Self::Range
            | Self::Bytes
            | Self::Task
            | Self::TaskHandle
            | Self::Generator
            | Self::AsyncGenerator
            | Self::Class => None,
        }
    }

    pub const fn is_readonly(self) -> bool {
        match self {
            Self::ReadonlyArray | Self::ReadonlyMap | Self::ReadonlySet => true,
            Self::Array
            | Self::Map
            | Self::Set
            | Self::Range
            | Self::Bytes
            | Self::Task
            | Self::TaskHandle
            | Self::Generator
            | Self::AsyncGenerator
            | Self::Class => false,
        }
    }

    pub const fn unreadonly(self) -> Self {
        match self {
            Self::ReadonlyArray => Self::Array,
            Self::ReadonlyMap => Self::Map,
            Self::ReadonlySet => Self::Set,
            Self::Array
            | Self::Map
            | Self::Set
            | Self::Range
            | Self::Bytes
            | Self::Task
            | Self::TaskHandle
            | Self::Generator
            | Self::AsyncGenerator
            | Self::Class => self,
        }
    }

    pub const fn readonly_counterpart(self) -> Option<Self> {
        match self {
            Self::Array => Some(Self::ReadonlyArray),
            Self::Map => Some(Self::ReadonlyMap),
            Self::Set => Some(Self::ReadonlySet),
            Self::ReadonlyArray | Self::ReadonlyMap | Self::ReadonlySet => Some(self),
            Self::Range
            | Self::Bytes
            | Self::Task
            | Self::TaskHandle
            | Self::Generator
            | Self::AsyncGenerator
            | Self::Class => None,
        }
    }

    pub const fn is_array_family(self) -> bool {
        match self {
            Self::Array | Self::ReadonlyArray => true,
            Self::Map
            | Self::ReadonlyMap
            | Self::Set
            | Self::ReadonlySet
            | Self::Range
            | Self::Bytes
            | Self::Task
            | Self::TaskHandle
            | Self::Generator
            | Self::AsyncGenerator
            | Self::Class => false,
        }
    }

    pub const fn is_map_family(self) -> bool {
        match self {
            Self::Map | Self::ReadonlyMap => true,
            Self::Array
            | Self::ReadonlyArray
            | Self::Set
            | Self::ReadonlySet
            | Self::Range
            | Self::Bytes
            | Self::Task
            | Self::TaskHandle
            | Self::Generator
            | Self::AsyncGenerator
            | Self::Class => false,
        }
    }

    pub const fn is_set_family(self) -> bool {
        match self {
            Self::Set | Self::ReadonlySet => true,
            Self::Array
            | Self::ReadonlyArray
            | Self::Map
            | Self::ReadonlyMap
            | Self::Range
            | Self::Bytes
            | Self::Task
            | Self::TaskHandle
            | Self::Generator
            | Self::AsyncGenerator
            | Self::Class => false,
        }
    }

    pub const fn core_module(self) -> &'static str {
        match self {
            Self::Array | Self::ReadonlyArray => "core:types/array",
            Self::Map | Self::ReadonlyMap => "core:types/map",
            Self::Set | Self::ReadonlySet => "core:types/set",
            Self::Range => "core:types/range",
            Self::Bytes => "core:types/bytes",
            Self::Class => "core:types/class",
            Self::Task | Self::TaskHandle | Self::Generator | Self::AsyncGenerator => {
                "core:types/iteration"
            }
        }
    }
}

impl std::fmt::Display for BuiltinType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

crate::named_enum! {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum CoreSum {
        Result => "Result",
    }
}

impl CoreSum {
    pub const fn module(self) -> &'static str {
        match self {
            Self::Result => "core:types/result",
        }
    }

    pub const fn failure_variant(self) -> &'static str {
        match self {
            Self::Result => "Err",
        }
    }

    pub const fn success_variant(self) -> &'static str {
        match self {
            Self::Result => "Ok",
        }
    }

    pub fn identify(name: Atom, origin: Option<Atom>) -> Option<Self> {
        let origin = origin?;
        Self::ALL
            .into_iter()
            .find(|s| Atom::of_const(s.name()) == name && Atom::of_const(s.module()) == origin)
    }

    pub fn named(name: Atom) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|s| Atom::of_const(s.name()) == name)
    }
}

crate::named_enum! {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum LangConstraint {
        Hashable => "Hashable",
    }
}

impl LangConstraint {
    pub const fn module(self) -> &'static str {
        match self {
            Self::Hashable => "core:types/hashable",
        }
    }

    pub fn identify(name: Atom, origin: Option<Atom>) -> Option<Self> {
        let origin = origin?;
        Self::ALL
            .into_iter()
            .find(|c| Atom::of_const(c.name()) == name && Atom::of_const(c.module()) == origin)
    }
}

pub fn is_lang_type(name: Atom, origin: Option<Atom>) -> bool {
    LangPrimitive::identify(name, origin).is_some() || BuiltinType::identify(name, origin).is_some()
}
