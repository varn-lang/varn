#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum ArrayStorage {
    Boxed = 0,
    Int = 1,
    Float = 2,
    Char = 3,
    Bool = 4,
    Ref = 5,
}

impl ArrayStorage {
    pub const ALL: [Self; 6] = [
        Self::Boxed,
        Self::Int,
        Self::Float,
        Self::Char,
        Self::Bool,
        Self::Ref,
    ];

    pub const fn bits(self) -> u8 {
        self as u8
    }

    pub fn from_bits(bits: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|storage| storage.bits() == bits)
    }

    pub const fn stores_without_barrier(self) -> bool {
        match self {
            Self::Int | Self::Float | Self::Char | Self::Bool => true,
            Self::Ref | Self::Boxed => false,
        }
    }
}
