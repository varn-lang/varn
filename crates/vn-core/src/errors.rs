crate::named_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum RuntimeErrorKind {
        #[default]
        Error => "Error",
        IntegerOverflow => "IntegerOverflow",
        DivisionByZero => "DivisionByZero",
        MatchError => "MatchError",
        TypeError => "TypeError",
        RangeError => "RangeError",
    }
}

impl RuntimeErrorKind {
    pub const MODULE: &'static str = "core:errors";

    pub fn identify(name: crate::Atom, origin: Option<crate::Atom>) -> Option<Self> {
        let origin = origin?;
        if origin != crate::Atom::of_const(Self::MODULE) {
            return None;
        }
        Self::ALL
            .into_iter()
            .find(|k| crate::Atom::of_const(k.name()) == name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorClass {
    Runtime(RuntimeErrorKind),
    Std(crate::StdErrorClass),
}

impl ErrorClass {
    pub fn of(name: &str) -> Option<Self> {
        RuntimeErrorKind::of(name)
            .map(Self::Runtime)
            .or_else(|| crate::StdErrorClass::of(name).map(Self::Std))
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Runtime(kind) => kind.name(),
            Self::Std(class) => class.name(),
        }
    }

    pub fn is_a(self, ancestor: Self) -> bool {
        match (self, ancestor) {
            (_, Self::Runtime(RuntimeErrorKind::Error)) => true,
            (Self::Runtime(a), Self::Runtime(b)) => a == b,
            (Self::Std(a), Self::Std(b)) => {
                let mut current = Some(a);
                while let Some(class) = current {
                    if class == b {
                        return true;
                    }
                    current = class.parent();
                }
                false
            }
            (Self::Runtime(_), Self::Std(_)) | (Self::Std(_), Self::Runtime(_)) => false,
        }
    }
}
