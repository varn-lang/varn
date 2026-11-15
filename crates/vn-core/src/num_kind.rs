#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[repr(u8)]
pub enum IntKind {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[repr(u8)]
pub enum FloatKind {
    F32,
    F64,
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum NumKind {
    Int(IntKind),
    Float(FloatKind),
}

impl IntKind {
    pub const ALL: [Self; 8] = [
        Self::I8,
        Self::I16,
        Self::I32,
        Self::I64,
        Self::U8,
        Self::U16,
        Self::U32,
        Self::U64,
    ];

    pub const fn bits(self) -> u32 {
        match self {
            Self::I8 | Self::U8 => 8,
            Self::I16 | Self::U16 => 16,
            Self::I32 | Self::U32 => 32,
            Self::I64 | Self::U64 => 64,
        }
    }

    pub const fn bytes(self) -> u32 {
        self.bits() / 8
    }

    pub const fn is_signed(self) -> bool {
        match self {
            Self::I8 | Self::I16 | Self::I32 | Self::I64 => true,
            Self::U8 | Self::U16 | Self::U32 | Self::U64 => false,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "int",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
        }
    }

    pub const fn min(self) -> i128 {
        if self.is_signed() {
            -(1i128 << (self.bits() - 1))
        } else {
            0
        }
    }

    pub const fn max(self) -> i128 {
        if self.is_signed() {
            (1i128 << (self.bits() - 1)) - 1
        } else {
            (1i128 << self.bits()) - 1
        }
    }

    pub const fn contains(self, value: i128) -> bool {
        value >= self.min() && value <= self.max()
    }
}

impl FloatKind {
    pub const ALL: [Self; 2] = [Self::F32, Self::F64];

    pub const fn bits(self) -> u32 {
        match self {
            Self::F32 => 32,
            Self::F64 => 64,
        }
    }

    pub const fn bytes(self) -> u32 {
        self.bits() / 8
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::F64 => "float",
        }
    }

    pub const fn round(self, value: f64) -> f64 {
        match self {
            Self::F32 => value as f32 as f64,
            Self::F64 => value,
        }
    }

    pub const fn max_finite(self) -> f64 {
        match self {
            Self::F32 => f32::MAX as f64,
            Self::F64 => f64::MAX,
        }
    }

    pub const fn mantissa_bits(self) -> u32 {
        match self {
            Self::F32 => 24,
            Self::F64 => 53,
        }
    }
}

impl std::fmt::Debug for NumKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl NumKind {
    pub const INT: Self = Self::Int(IntKind::I64);
    pub const FLOAT: Self = Self::Float(FloatKind::F64);

    pub const LANGUAGE: [Self; 9] = [
        Self::Int(IntKind::I64),
        Self::Float(FloatKind::F64),
        Self::Int(IntKind::I8),
        Self::Int(IntKind::I16),
        Self::Int(IntKind::I32),
        Self::Int(IntKind::U8),
        Self::Int(IntKind::U16),
        Self::Int(IntKind::U32),
        Self::Float(FloatKind::F32),
    ];

    pub const ALL: [Self; 10] = [
        Self::Int(IntKind::I64),
        Self::Float(FloatKind::F64),
        Self::Int(IntKind::I8),
        Self::Int(IntKind::I16),
        Self::Int(IntKind::I32),
        Self::Int(IntKind::U8),
        Self::Int(IntKind::U16),
        Self::Int(IntKind::U32),
        Self::Int(IntKind::U64),
        Self::Float(FloatKind::F32),
    ];

    pub const fn bits(self) -> u32 {
        match self {
            Self::Int(k) => k.bits(),
            Self::Float(k) => k.bits(),
        }
    }

    pub const fn bytes(self) -> u32 {
        self.bits() / 8
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Int(k) => k.name(),
            Self::Float(k) => k.name(),
        }
    }

    pub const fn is_int(self) -> bool {
        matches!(self, Self::Int(_))
    }

    pub const fn is_float(self) -> bool {
        matches!(self, Self::Float(_))
    }

    pub const fn is_default_width(self) -> bool {
        matches!(self, Self::Int(IntKind::I64) | Self::Float(FloatKind::F64))
    }

    pub const fn core_module(self) -> &'static str {
        match self {
            Self::Int(IntKind::I8) => "core:types/i8",
            Self::Int(IntKind::I16) => "core:types/i16",
            Self::Int(IntKind::I32) => "core:types/i32",
            Self::Int(IntKind::I64) => "core:types/int",
            Self::Int(IntKind::U8) => "core:types/u8",
            Self::Int(IntKind::U16) => "core:types/u16",
            Self::Int(IntKind::U32) => "core:types/u32",
            Self::Int(IntKind::U64) => "core:types/u64",
            Self::Float(FloatKind::F32) => "core:types/f32",
            Self::Float(FloatKind::F64) => "core:types/float",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        IntKind::ALL
            .into_iter()
            .find(|k| k.name() == name)
            .map(Self::Int)
            .or_else(|| {
                FloatKind::ALL
                    .into_iter()
                    .find(|k| k.name() == name)
                    .map(Self::Float)
            })
    }
}
