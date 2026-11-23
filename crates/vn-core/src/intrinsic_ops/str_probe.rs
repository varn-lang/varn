use crate::MemberKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrProbe {
    StartsWith,
    EndsWith,
    Includes,
    IndexOf,
}

impl StrProbe {
    pub const ALL: [Self; 4] = [
        Self::StartsWith,
        Self::EndsWith,
        Self::Includes,
        Self::IndexOf,
    ];

    pub const fn key(self) -> MemberKey {
        match self {
            Self::StartsWith => MemberKey::StartsWith,
            Self::EndsWith => MemberKey::EndsWith,
            Self::Includes => MemberKey::Includes,
            Self::IndexOf => MemberKey::IndexOf,
        }
    }

    pub fn of(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.key().name() == name)
    }
}
