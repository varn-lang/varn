crate::named_enum! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum FfiKind {
        Void => "Void",
        Int => "Int",
        Float => "Float",
        Str => "Str",
        Ptr => "Ptr",
        Bool => "Bool",
        Int32 => "Int32",
        Float32 => "Float32",
    }
}

impl FfiKind {
    pub const fn raw(self) -> i64 {
        match self {
            Self::Void => 0,
            Self::Int => 1,
            Self::Float => 2,
            Self::Str => 3,
            Self::Ptr => 4,
            Self::Bool => 5,
            Self::Int32 => 6,
            Self::Float32 => 7,
        }
    }

    pub fn of_raw(raw: i64) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.raw() == raw)
    }
}
