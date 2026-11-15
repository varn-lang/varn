#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericDomain {
    Int,
    Float,
    BigInt,
    Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum ValueConv {
    IntToFloat = 0,
    FloatToInt = 1,
    IntToBigInt = 2,
    BigIntToInt = 3,
    IntToDecimal = 4,
    DecimalToInt = 5,

    DynToInt = 6,

    DynToFloat = 7,
    BigIntToFloat = 8,
    FloatToBigInt = 9,
    DecimalToFloat = 10,
    FloatToDecimal = 11,
    BigIntToDecimal = 12,
    DecimalToBigInt = 13,

    ExpectStr = 14,
    ExpectBool = 15,
    ExpectStrOrNull = 16,
    ExpectBoolOrNull = 17,
    ExpectInt = 18,
    ExpectFloat = 19,
    ExpectIntOrNull = 20,
    ExpectFloatOrNull = 21,
    ExpectRef = 22,
    ExpectChar = 23,

    NarrowI8 = 24,
    NarrowI16 = 25,
    NarrowI32 = 26,
    NarrowU8 = 27,
    NarrowU16 = 28,
    NarrowU32 = 29,
    NarrowF32 = 30,
    NarrowU64 = 31,
    U64ToBigInt = 32,
    U64ToBits = 33,
    U64FromBits = 34,
}

impl ValueConv {
    pub const ALL: [Self; 35] = [
        Self::IntToFloat,
        Self::FloatToInt,
        Self::IntToBigInt,
        Self::BigIntToInt,
        Self::IntToDecimal,
        Self::DecimalToInt,
        Self::DynToInt,
        Self::DynToFloat,
        Self::BigIntToFloat,
        Self::FloatToBigInt,
        Self::DecimalToFloat,
        Self::FloatToDecimal,
        Self::BigIntToDecimal,
        Self::DecimalToBigInt,
        Self::ExpectStr,
        Self::ExpectBool,
        Self::ExpectStrOrNull,
        Self::ExpectBoolOrNull,
        Self::ExpectInt,
        Self::ExpectFloat,
        Self::ExpectIntOrNull,
        Self::ExpectFloatOrNull,
        Self::ExpectRef,
        Self::ExpectChar,
        Self::NarrowI8,
        Self::NarrowI16,
        Self::NarrowI32,
        Self::NarrowU8,
        Self::NarrowU16,
        Self::NarrowU32,
        Self::NarrowF32,
        Self::NarrowU64,
        Self::U64ToBigInt,
        Self::U64ToBits,
        Self::U64FromBits,
    ];

    pub const fn from_u8(raw: u8) -> Option<Self> {
        if (raw as usize) < Self::ALL.len() {
            Some(Self::ALL[raw as usize])
        } else {
            None
        }
    }

    pub const fn narrow(kind: crate::NumKind) -> Option<Self> {
        use crate::{FloatKind, IntKind, NumKind};
        match kind {
            NumKind::Int(IntKind::I8) => Some(Self::NarrowI8),
            NumKind::Int(IntKind::I16) => Some(Self::NarrowI16),
            NumKind::Int(IntKind::I32) => Some(Self::NarrowI32),
            NumKind::Int(IntKind::U8) => Some(Self::NarrowU8),
            NumKind::Int(IntKind::U16) => Some(Self::NarrowU16),
            NumKind::Int(IntKind::U32) => Some(Self::NarrowU32),
            NumKind::Int(IntKind::U64) => Some(Self::NarrowU64),
            NumKind::Float(FloatKind::F32) => Some(Self::NarrowF32),
            NumKind::Int(IntKind::I64) | NumKind::Float(FloatKind::F64) => None,
        }
    }

    pub const fn narrowed(self) -> Option<crate::NumKind> {
        use crate::{FloatKind, IntKind, NumKind};
        match self {
            Self::NarrowI8 => Some(NumKind::Int(IntKind::I8)),
            Self::NarrowI16 => Some(NumKind::Int(IntKind::I16)),
            Self::NarrowI32 => Some(NumKind::Int(IntKind::I32)),
            Self::NarrowU8 => Some(NumKind::Int(IntKind::U8)),
            Self::NarrowU16 => Some(NumKind::Int(IntKind::U16)),
            Self::NarrowU32 => Some(NumKind::Int(IntKind::U32)),
            Self::NarrowU64 => Some(NumKind::Int(IntKind::U64)),
            Self::NarrowF32 => Some(NumKind::Float(FloatKind::F32)),
            Self::IntToFloat
            | Self::FloatToInt
            | Self::IntToBigInt
            | Self::BigIntToInt
            | Self::IntToDecimal
            | Self::DecimalToInt
            | Self::DynToInt
            | Self::DynToFloat
            | Self::BigIntToFloat
            | Self::FloatToBigInt
            | Self::DecimalToFloat
            | Self::FloatToDecimal
            | Self::BigIntToDecimal
            | Self::DecimalToBigInt
            | Self::ExpectStr
            | Self::ExpectBool
            | Self::ExpectStrOrNull
            | Self::ExpectBoolOrNull
            | Self::ExpectInt
            | Self::ExpectFloat
            | Self::ExpectIntOrNull
            | Self::ExpectFloatOrNull
            | Self::ExpectRef
            | Self::ExpectChar
            | Self::U64ToBigInt
            | Self::U64ToBits
            | Self::U64FromBits => None,
        }
    }

    pub const fn between(from: NumericDomain, to: NumericDomain) -> Option<Self> {
        use NumericDomain::*;
        Some(match (from, to) {
            (Int, Float) => Self::IntToFloat,
            (Float, Int) => Self::FloatToInt,
            (Int, BigInt) => Self::IntToBigInt,
            (BigInt, Int) => Self::BigIntToInt,
            (Int, Decimal) => Self::IntToDecimal,
            (Decimal, Int) => Self::DecimalToInt,
            (BigInt, Float) => Self::BigIntToFloat,
            (Float, BigInt) => Self::FloatToBigInt,
            (Decimal, Float) => Self::DecimalToFloat,
            (Float, Decimal) => Self::FloatToDecimal,
            (BigInt, Decimal) => Self::BigIntToDecimal,
            (Decimal, BigInt) => Self::DecimalToBigInt,
            _ => return None,
        })
    }

    pub const fn can_fault(self) -> bool {
        matches!(
            self,
            Self::FloatToInt
                | Self::BigIntToInt
                | Self::DecimalToInt
                | Self::DynToInt
                | Self::DynToFloat
                | Self::ExpectStr
                | Self::ExpectBool
                | Self::ExpectStrOrNull
                | Self::ExpectBoolOrNull
                | Self::ExpectInt
                | Self::ExpectFloat
                | Self::ExpectIntOrNull
                | Self::ExpectFloatOrNull
                | Self::ExpectRef
                | Self::ExpectChar
                | Self::FloatToBigInt
                | Self::FloatToDecimal
                | Self::NarrowI8
                | Self::NarrowI16
                | Self::NarrowI32
                | Self::NarrowU8
                | Self::NarrowU16
                | Self::NarrowU32
                | Self::NarrowU64
                | Self::U64ToBits
        )
    }
}

pub fn float_to_int(f: f64) -> Option<i64> {
    const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;
    let t = f.trunc();
    if t.is_nan() || !(-TWO_POW_63..TWO_POW_63).contains(&t) {
        return None;
    }
    Some(t as i64)
}
