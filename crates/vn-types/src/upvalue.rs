#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpvalueSource {
    Local(u32),
    Inherited(u32),
}

const BYTECODE_LOCAL: u16 = 1 << 8;
const BYTECODE_INDEX: u16 = 0xFF;
const WORD_LOCAL: u64 = 1 << 32;
const WORD_INDEX: u64 = 0xFFFF_FFFF;

impl UpvalueSource {
    pub fn to_bytecode(self) -> Option<u16> {
        let (flag, index) = match self {
            Self::Local(index) => (BYTECODE_LOCAL, index),
            Self::Inherited(index) => (0, index),
        };
        let index = u8::try_from(index).ok()?;
        Some(flag | u16::from(index))
    }

    pub const fn from_bytecode(word: u16) -> Self {
        let index = (word & BYTECODE_INDEX) as u32;
        if word & BYTECODE_LOCAL != 0 {
            Self::Local(index)
        } else {
            Self::Inherited(index)
        }
    }

    pub const fn to_word(self) -> u64 {
        match self {
            Self::Local(index) => WORD_LOCAL | index as u64,
            Self::Inherited(index) => index as u64,
        }
    }

    pub const fn from_word(word: u64) -> Self {
        let index = (word & WORD_INDEX) as u32;
        if word & WORD_LOCAL != 0 {
            Self::Local(index)
        } else {
            Self::Inherited(index)
        }
    }
}
