use crate::{FieldKind, RuntimeKind};

mod class;
pub use class::{ClassLayout, FieldLayout};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ScalarRepr {
    Bool,

    NullableBool,

    Char,

    NullableChar,

    I64,

    F64,

    Ref,

    Boxed,
}

impl ScalarRepr {
    pub const fn ic_code(self) -> u8 {
        match self {
            Self::Bool => 1,
            Self::I64 => 2,
            Self::F64 => 3,
            Self::Ref => 4,
            Self::Boxed => 5,
            Self::Char => 6,
            Self::NullableBool => 7,
            Self::NullableChar => 8,
        }
    }
}

pub const COMPACT_REF_NULL: u64 = 0;

pub const COMPACT_BOOL_NULL: u8 = 2;

pub const COMPACT_CHAR_NULL: u32 = 0x11_0000;

impl ScalarRepr {
    pub const fn holds_reference(self) -> bool {
        match self {
            ScalarRepr::Ref | ScalarRepr::Boxed => true,
            ScalarRepr::Bool
            | ScalarRepr::NullableBool
            | ScalarRepr::Char
            | ScalarRepr::NullableChar
            | ScalarRepr::I64
            | ScalarRepr::F64 => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TypeLayout {
    pub size: u32,
    pub align: u32,
    pub repr: ScalarRepr,
}

impl TypeLayout {
    const BOXED: Self = Self::new(16, 8, ScalarRepr::Boxed);

    const fn new(size: u32, align: u32, repr: ScalarRepr) -> Self {
        Self { size, align, repr }
    }

    pub const fn of_field(field: Option<FieldKind>) -> Self {
        match field {
            Some(FieldKind {
                kind,
                nullable: false,
            }) => Self::of_kind(kind),
            Some(FieldKind {
                kind,
                nullable: true,
            }) => Self::of_nullable(kind),
            None => Self::BOXED,
        }
    }

    const fn of_nullable(kind: RuntimeKind) -> Self {
        match kind {
            RuntimeKind::Bool => Self::new(1, 1, ScalarRepr::NullableBool),
            RuntimeKind::Char => Self::new(4, 4, ScalarRepr::NullableChar),
            RuntimeKind::Int | RuntimeKind::Float => Self::BOXED,
            RuntimeKind::Null
            | RuntimeKind::Str
            | RuntimeKind::Opaque
            | RuntimeKind::Array
            | RuntimeKind::Map
            | RuntimeKind::Set
            | RuntimeKind::Object
            | RuntimeKind::Class
            | RuntimeKind::Function
            | RuntimeKind::Task
            | RuntimeKind::Bytes
            | RuntimeKind::Generator
            | RuntimeKind::AsyncGenerator
            | RuntimeKind::BigInt
            | RuntimeKind::Decimal
            | RuntimeKind::Tuple
            | RuntimeKind::Range
            | RuntimeKind::Enum
            | RuntimeKind::TaskHandle => Self::of_kind(kind),
        }
    }

    const fn of_kind(kind: RuntimeKind) -> Self {
        match kind {
            RuntimeKind::Bool => Self::new(1, 1, ScalarRepr::Bool),
            RuntimeKind::Char => Self::new(4, 4, ScalarRepr::Char),
            RuntimeKind::Int => Self::new(8, 8, ScalarRepr::I64),
            RuntimeKind::Float => Self::new(8, 8, ScalarRepr::F64),
            RuntimeKind::Array
            | RuntimeKind::Map
            | RuntimeKind::Set
            | RuntimeKind::Object
            | RuntimeKind::Class
            | RuntimeKind::Function
            | RuntimeKind::Task
            | RuntimeKind::Bytes
            | RuntimeKind::Generator
            | RuntimeKind::AsyncGenerator
            | RuntimeKind::BigInt
            | RuntimeKind::Decimal
            | RuntimeKind::Tuple
            | RuntimeKind::Range
            | RuntimeKind::Enum
            | RuntimeKind::TaskHandle => Self::new(8, 8, ScalarRepr::Ref),
            RuntimeKind::Null | RuntimeKind::Str | RuntimeKind::Opaque => Self::BOXED,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GcLayout {
    pub slots: Vec<GcSlot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GcSlot {
    pub offset: u32,

    pub repr: ScalarRepr,
}
