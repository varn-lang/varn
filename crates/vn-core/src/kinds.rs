use crate::lang_type::LangPrimitive;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum TypeLiteral<N> {
    Int(i64),
    Str(N),
    Bool(bool),
    Char(char),
}

impl<N> TypeLiteral<N> {
    pub const fn base(&self) -> LangPrimitive {
        match self {
            TypeLiteral::Int(_) => LangPrimitive::INT,
            TypeLiteral::Str(_) => LangPrimitive::Str,
            TypeLiteral::Bool(_) => LangPrimitive::Bool,
            TypeLiteral::Char(_) => LangPrimitive::Char,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]

pub enum TypeKind<T, N, C, F, O, I, M> {
    Primitive(LangPrimitive),
    Literal(TypeLiteral<N>),
    This,
    Array(T),
    Union(C),
    Intersection(C),
    Tuple(C),
    Named(N, M),
    Generic(N, C, M),
    Fn(F),
    Record(O),
    Interface(I),

    EnumVariant {
        enum_name: N,
        variant_name: N,
        type_args: C,
        payload_ty: T,
    },

    TypePredicate {
        parameter_name: N,
        target_type: T,
    },
}

impl<T, N, C, F, O, I, M> TypeKind<T, N, C, F, O, I, M> {
    pub fn lang_name(&self) -> Option<&'static str> {
        match self {
            TypeKind::Primitive(p) => Some(p.name()),
            TypeKind::Literal(l) => Some(l.base().name()),
            TypeKind::This
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

    pub fn of_primitive_name(name: &str) -> Option<Self> {
        LangPrimitive::of(name).map(TypeKind::Primitive)
    }
}
