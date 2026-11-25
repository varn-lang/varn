use vn_core::{Atom, LangPrimitive, TypeKind};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct CheckerTyId(pub(super) u128);

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct TyListId(pub(super) u128);

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct FunctionTypeId(pub(super) u128);

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ObjectMembersId(pub(super) u128);

pub type InternedTypeKind =
    TypeKind<CheckerTyId, Atom, TyListId, FunctionTypeId, ObjectMembersId, ObjectMembersId>;

impl CheckerTyId {
    pub const INT: CheckerTyId = CheckerTyId(0);
    pub const FLOAT: CheckerTyId = CheckerTyId(1);
    pub const DECIMAL: CheckerTyId = CheckerTyId(2);
    pub const BIGINT: CheckerTyId = CheckerTyId(3);
    pub const STR: CheckerTyId = CheckerTyId(4);
    pub const CHAR: CheckerTyId = CheckerTyId(5);
    pub const BOOL: CheckerTyId = CheckerTyId(6);
    pub const VOID: CheckerTyId = CheckerTyId(7);
    pub const NULL: CheckerTyId = CheckerTyId(8);
    pub const NEVER: CheckerTyId = CheckerTyId(9);
    pub const DYNAMIC: CheckerTyId = CheckerTyId(10);
    pub const THIS: CheckerTyId = CheckerTyId(11);
    pub const ERROR: CheckerTyId = CheckerTyId(12);

    pub const fn num_kind(self) -> Option<vn_core::NumKind> {
        use vn_core::{FloatKind, IntKind, NumKind};
        match self.0 {
            0 => Some(NumKind::INT),
            1 => Some(NumKind::FLOAT),
            13 => Some(NumKind::Int(IntKind::I8)),
            14 => Some(NumKind::Int(IntKind::I16)),
            15 => Some(NumKind::Int(IntKind::I32)),
            16 => Some(NumKind::Int(IntKind::U8)),
            17 => Some(NumKind::Int(IntKind::U16)),
            18 => Some(NumKind::Int(IntKind::U32)),
            19 => Some(NumKind::Int(IntKind::U64)),
            20 => Some(NumKind::Float(FloatKind::F32)),
            _ => None,
        }
    }

    pub const fn num(kind: vn_core::NumKind) -> CheckerTyId {
        use vn_core::{FloatKind, IntKind, NumKind};
        match kind {
            NumKind::Int(IntKind::I64) => Self::INT,
            NumKind::Float(FloatKind::F64) => Self::FLOAT,
            NumKind::Int(IntKind::I8) => CheckerTyId(13),
            NumKind::Int(IntKind::I16) => CheckerTyId(14),
            NumKind::Int(IntKind::I32) => CheckerTyId(15),
            NumKind::Int(IntKind::U8) => CheckerTyId(16),
            NumKind::Int(IntKind::U16) => CheckerTyId(17),
            NumKind::Int(IntKind::U32) => CheckerTyId(18),
            NumKind::Int(IntKind::U64) => CheckerTyId(19),
            NumKind::Float(FloatKind::F32) => CheckerTyId(20),
        }
    }

    pub const fn is_numeric(self) -> bool {
        self.num_kind().is_some() || matches!(self, Self::DECIMAL | Self::BIGINT)
    }
}

impl CheckerTyId {
    pub const fn primitive(p: LangPrimitive) -> CheckerTyId {
        match p {
            LangPrimitive::Num(k) => CheckerTyId::num(k),
            LangPrimitive::Decimal => CheckerTyId::DECIMAL,
            LangPrimitive::BigInt => CheckerTyId::BIGINT,
            LangPrimitive::Str => CheckerTyId::STR,
            LangPrimitive::Char => CheckerTyId::CHAR,
            LangPrimitive::Bool => CheckerTyId::BOOL,
            LangPrimitive::Void => CheckerTyId::VOID,
            LangPrimitive::Null => CheckerTyId::NULL,
            LangPrimitive::Never => CheckerTyId::NEVER,
            LangPrimitive::Dynamic => CheckerTyId::DYNAMIC,
            LangPrimitive::Error => CheckerTyId::ERROR,
        }
    }
}

pub(super) fn seeded_id(kind: &InternedTypeKind) -> Option<CheckerTyId> {
    match kind {
        TypeKind::Primitive(p) => Some(CheckerTyId::primitive(*p)),
        TypeKind::This => Some(CheckerTyId::THIS),
        TypeKind::Literal(_)
        | TypeKind::Array(_)
        | TypeKind::Union(_)
        | TypeKind::Intersection(_)
        | TypeKind::Tuple(_)
        | TypeKind::Named(..)
        | TypeKind::Generic(..)
        | TypeKind::Fn(_)
        | TypeKind::Record(_)
        | TypeKind::Interface(_)
        | TypeKind::EnumVariant { .. }
        | TypeKind::TypePredicate { .. } => None,
    }
}
