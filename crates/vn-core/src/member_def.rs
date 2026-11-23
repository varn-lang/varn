use crate::{AccessorKind, OpCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MemberDef {
    Method { is_static: bool },
    Accessor { kind: AccessorKind, is_static: bool },
}

impl MemberDef {
    pub const ALL: [Self; 6] = [
        Self::Method { is_static: false },
        Self::Method { is_static: true },
        Self::Accessor {
            kind: AccessorKind::Get,
            is_static: false,
        },
        Self::Accessor {
            kind: AccessorKind::Set,
            is_static: false,
        },
        Self::Accessor {
            kind: AccessorKind::Get,
            is_static: true,
        },
        Self::Accessor {
            kind: AccessorKind::Set,
            is_static: true,
        },
    ];

    pub const fn is_static(self) -> bool {
        match self {
            Self::Method { is_static } | Self::Accessor { is_static, .. } => is_static,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Method { is_static: false } => "method",
            Self::Method { is_static: true } => "static",
            Self::Accessor {
                kind: AccessorKind::Get,
                is_static: false,
            } => "getter",
            Self::Accessor {
                kind: AccessorKind::Set,
                is_static: false,
            } => "setter",
            Self::Accessor {
                kind: AccessorKind::Get,
                is_static: true,
            } => "static getter",
            Self::Accessor {
                kind: AccessorKind::Set,
                is_static: true,
            } => "static setter",
        }
    }

    pub const fn opcode(self) -> OpCode {
        match self {
            Self::Method { is_static: false } => OpCode::Method,
            Self::Method { is_static: true } => OpCode::DefineStatic,
            Self::Accessor {
                kind: AccessorKind::Get,
                is_static: false,
            } => OpCode::DefineGetter,
            Self::Accessor {
                kind: AccessorKind::Set,
                is_static: false,
            } => OpCode::DefineSetter,
            Self::Accessor {
                kind: AccessorKind::Get,
                is_static: true,
            } => OpCode::DefineStaticGetter,
            Self::Accessor {
                kind: AccessorKind::Set,
                is_static: true,
            } => OpCode::DefineStaticSetter,
        }
    }

    pub fn of_opcode(op: OpCode) -> Option<Self> {
        Self::ALL.into_iter().find(|def| def.opcode() == op)
    }

    pub fn wire(self) -> u8 {
        Self::ALL.iter().position(|def| *def == self).unwrap_or(0) as u8
    }

    pub fn of_wire(wire: u8) -> Option<Self> {
        Self::ALL.get(usize::from(wire)).copied()
    }
}
