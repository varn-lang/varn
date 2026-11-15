use vn_core::ast::TypeNode;
use vn_core::kinds::TypeKind;
use vn_core::{AtomInterner, LangPrimitive, NumKind};

#[derive(Clone, Debug, PartialEq)]
pub enum ContractTy {
    Num(NumKind),
    Bool,
    Char,
    Str,
    Void,
    Null,
    Never,
    BigInt,
    Decimal,
    Dynamic,
    Opt(Box<ContractTy>),
    Array(Box<ContractTy>),
    Tuple(Vec<ContractTy>),
    Union(Vec<ContractTy>),
    Generic(String, Vec<ContractTy>),
    Named(String),
    Fn,
    Other,
}

impl ContractTy {
    pub fn from_node(node: &TypeNode, interner: &AtomInterner) -> Self {
        match &node.kind {
            TypeKind::Named(name, _) => {
                let name = interner.resolve(*name);
                LangPrimitive::from_str(name)
                    .map(Self::of_primitive)
                    .unwrap_or_else(|| Self::Named(name.to_string()))
            }
            TypeKind::Primitive(p) => Self::of_primitive(*p),
            TypeKind::TypePredicate { .. } => Self::Bool,
            TypeKind::Array(inner) => Self::Array(Box::new(Self::from_node(inner, interner))),
            TypeKind::Tuple(items) => Self::Tuple(Self::all(items, interner)),
            TypeKind::Generic(name, args, _) => Self::Generic(
                interner.resolve(*name).to_string(),
                Self::all(args, interner),
            ),
            TypeKind::Union(members) => Self::union(members, interner),
            TypeKind::Fn(_) => Self::Fn,
            TypeKind::Literal(_)
            | TypeKind::This
            | TypeKind::Intersection(_)
            | TypeKind::TemplateLiteral(_)
            | TypeKind::Record(_)
            | TypeKind::Interface(_)
            | TypeKind::Typeof(_)
            | TypeKind::KeyOf(_)
            | TypeKind::IndexedAccess { .. }
            | TypeKind::Mapped { .. }
            | TypeKind::Conditional { .. }
            | TypeKind::Infer(_)
            | TypeKind::EnumVariant { .. } => Self::Other,
        }
    }

    fn all(nodes: &[TypeNode], interner: &AtomInterner) -> Vec<Self> {
        nodes.iter().map(|n| Self::from_node(n, interner)).collect()
    }

    fn union(members: &[TypeNode], interner: &AtomInterner) -> Self {
        let is_null = |m: &TypeNode| matches!(m.kind, TypeKind::Primitive(LangPrimitive::Null));
        match members {
            [value, null] if is_null(null) => Self::Opt(Box::new(Self::from_node(value, interner))),
            [null, value] if is_null(null) => Self::Opt(Box::new(Self::from_node(value, interner))),
            _ => Self::Union(Self::all(members, interner)),
        }
    }

    fn of_primitive(p: LangPrimitive) -> Self {
        match p {
            LangPrimitive::Num(kind) => Self::Num(kind),
            LangPrimitive::Bool => Self::Bool,
            LangPrimitive::Char => Self::Char,
            LangPrimitive::Str => Self::Str,
            LangPrimitive::Void => Self::Void,
            LangPrimitive::Null => Self::Null,
            LangPrimitive::Never => Self::Never,
            LangPrimitive::BigInt => Self::BigInt,
            LangPrimitive::Decimal => Self::Decimal,
            LangPrimitive::Dynamic | LangPrimitive::Error => Self::Dynamic,
        }
    }
}
