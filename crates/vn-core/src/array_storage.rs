#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum ArrayStorage {
    Boxed,
    Int,
    Float,
    Char,
    Bool,
    Ref,
}

impl ArrayStorage {
    pub const fn bits(self) -> u8 {
        self as u8
    }

    pub const fn from_bits(bits: u8) -> Option<Self> {
        Some(match bits {
            0 => Self::Boxed,
            1 => Self::Int,
            2 => Self::Float,
            3 => Self::Char,
            4 => Self::Bool,
            5 => Self::Ref,
            _ => return None,
        })
    }
}
