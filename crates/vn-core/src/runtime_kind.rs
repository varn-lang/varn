#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[repr(u8)]
pub enum RuntimeKind {
    Null = 0,
    Bool,
    Int,
    Float,
    Str,
    BigInt,
    Decimal,
    Char,
    Array,
    Map,
    Set,
    Tuple,
    Object,
    Class,
    Function,
    Generator,
    Task,
    Range,
    Enum,
    Opaque,
    TaskHandle,
    Bytes,
    AsyncGenerator,
}

impl RuntimeKind {
    pub const fn of_num(kind: crate::NumKind) -> Self {
        match kind {
            crate::NumKind::Int(_) => Self::Int,
            crate::NumKind::Float(_) => Self::Float,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Float => "float",
            Self::Str => "str",
            Self::BigInt => "bigint",
            Self::Decimal => "decimal",
            Self::Char => "char",
            Self::Array => "Array",
            Self::Map => "Map",
            Self::Set => "Set",
            Self::Tuple => "Tuple",
            Self::Object => "object",
            Self::Class => "class",
            Self::Function => "function",
            Self::Generator => "Generator",
            Self::Task => "Task",
            Self::Range => "Range",
            Self::Enum => "enum",
            Self::Opaque => "opaque",
            Self::TaskHandle => "TaskHandle",
            Self::Bytes => "Bytes",
            Self::AsyncGenerator => "AsyncGenerator",
        }
    }
}

impl RuntimeKind {
    pub const fn from_u8(raw: u8) -> Option<Self> {
        if raw <= Self::AsyncGenerator as u8 {
            Some(unsafe { std::mem::transmute::<u8, Self>(raw) })
        } else {
            None
        }
    }

    pub const COUNT: usize = Self::AsyncGenerator as usize + 1;

    pub fn named(name: &str) -> Option<Self> {
        (0..Self::COUNT as u8)
            .filter_map(Self::from_u8)
            .find(|k| k.name() == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FieldKind {
    pub kind: RuntimeKind,
    pub nullable: bool,
}

impl FieldKind {
    const NULLABLE_BIT: u8 = 0x80;

    pub const fn exact(kind: RuntimeKind) -> Self {
        Self {
            kind,
            nullable: false,
        }
    }

    pub const fn nullable(kind: RuntimeKind) -> Self {
        Self {
            kind,
            nullable: true,
        }
    }

    pub const fn encode(field: Option<Self>) -> u8 {
        match field {
            Some(Self {
                kind,
                nullable: false,
            }) => kind as u8,
            Some(Self {
                kind,
                nullable: true,
            }) => kind as u8 | Self::NULLABLE_BIT,
            None => u8::MAX,
        }
    }

    pub const fn decode(raw: u8) -> Option<Self> {
        match RuntimeKind::from_u8(raw & !Self::NULLABLE_BIT) {
            Some(kind) => Some(Self {
                kind,
                nullable: raw & Self::NULLABLE_BIT != 0,
            }),
            None => None,
        }
    }
}

impl std::fmt::Display for FieldKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.kind.name())?;
        if self.nullable {
            f.write_str("?")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FieldAccess {
    Slot,
    Compact(Option<FieldKind>),
}

impl FieldAccess {
    pub fn encode(self) -> u8 {
        match self {
            Self::Slot => 0,
            Self::Compact(kind) => {
                debug_assert!(
                    kind.is_none_or(|field| field.kind != RuntimeKind::Null),
                    "a field is never laid out as null"
                );
                FieldKind::encode(kind)
            }
        }
    }

    pub const fn decode(raw: u8) -> Self {
        if raw == 0 {
            Self::Slot
        } else {
            Self::Compact(FieldKind::decode(raw))
        }
    }
}

impl std::fmt::Display for RuntimeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
