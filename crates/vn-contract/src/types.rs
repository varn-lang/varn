use vn_core::ast::TypeNode;
use vn_core::kinds::TypeKind;
use vn_core::{AtomInterner, BuiltinType, FieldKind, IntKind, LangPrimitive, NumKind, RuntimeKind};

use crate::ModuleContract;

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
            TypeKind::Named(path, ()) => {
                let name = path.dotted(interner);
                LangPrimitive::of(&name)
                    .map(Self::of_primitive)
                    .unwrap_or(Self::Named(name))
            }
            TypeKind::Primitive(p) => Self::of_primitive(*p),
            TypeKind::TypePredicate { .. } => Self::Bool,
            TypeKind::Array(inner) => Self::Array(Box::new(Self::from_node(inner, interner))),
            TypeKind::Tuple(items) => Self::Tuple(Self::all(items, interner)),
            TypeKind::Generic(path, args, ()) => {
                Self::Generic(path.dotted(interner), Self::all(args, interner))
            }
            TypeKind::Union(members) => Self::union(members, interner),
            TypeKind::Fn(_) => Self::Fn,
            TypeKind::Literal(_)
            | TypeKind::This
            | TypeKind::Intersection(_)
            | TypeKind::Record(_)
            | TypeKind::Interface(_)
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

impl ContractTy {
    pub fn field_kind(&self, contract: &ModuleContract) -> Option<FieldKind> {
        match self {
            Self::Opt(inner) => inner.exact_field_kind(contract).map(FieldKind::nullable),
            Self::Num(_)
            | Self::Bool
            | Self::Char
            | Self::Str
            | Self::Void
            | Self::Null
            | Self::Never
            | Self::BigInt
            | Self::Decimal
            | Self::Dynamic
            | Self::Array(_)
            | Self::Tuple(_)
            | Self::Union(_)
            | Self::Generic(..)
            | Self::Named(_)
            | Self::Fn
            | Self::Other => self.exact_field_kind(contract).map(FieldKind::exact),
        }
    }

    fn exact_field_kind(&self, contract: &ModuleContract) -> Option<RuntimeKind> {
        match self {
            Self::Num(NumKind::Int(IntKind::U64)) => None,
            Self::Num(kind) => Some(RuntimeKind::of_num(*kind)),
            Self::Bool => Some(RuntimeKind::Bool),
            Self::Char => Some(RuntimeKind::Char),
            Self::Str => Some(RuntimeKind::Str),
            Self::BigInt => Some(RuntimeKind::BigInt),
            Self::Decimal => Some(RuntimeKind::Decimal),
            Self::Array(_) => Some(RuntimeKind::Array),
            Self::Tuple(_) => Some(RuntimeKind::Tuple),
            Self::Fn => Some(RuntimeKind::Function),
            Self::Generic(name, _) | Self::Named(name) if contract.enums.contains(name) => {
                Some(RuntimeKind::Enum)
            }
            Self::Generic(name, _) | Self::Named(name) if contract.classes.contains_key(name) => {
                Some(RuntimeKind::Class)
            }
            Self::Generic(name, args) => {
                BuiltinType::of(name).and_then(|b| generic_kind(b, args.len()))
            }
            Self::Named(name) => BuiltinType::of(name).and_then(bare_kind),
            Self::Opt(_)
            | Self::Union(_)
            | Self::Void
            | Self::Null
            | Self::Never
            | Self::Dynamic
            | Self::Other => None,
        }
    }
}

fn generic_kind(b: BuiltinType, arity: usize) -> Option<RuntimeKind> {
    match (b, arity) {
        (BuiltinType::Map | BuiltinType::ReadonlyMap, _) => Some(RuntimeKind::Map),
        (BuiltinType::Set | BuiltinType::ReadonlySet, _) => Some(RuntimeKind::Set),
        (BuiltinType::Array | BuiltinType::ReadonlyArray, 1) => Some(RuntimeKind::Array),
        (BuiltinType::TaskHandle, 1) => Some(RuntimeKind::TaskHandle),
        (BuiltinType::Range, 1) => Some(RuntimeKind::Range),
        (BuiltinType::Bytes, _) => Some(RuntimeKind::Bytes),
        (
            BuiltinType::Array
            | BuiltinType::ReadonlyArray
            | BuiltinType::TaskHandle
            | BuiltinType::Range
            | BuiltinType::Task
            | BuiltinType::Generator
            | BuiltinType::AsyncGenerator
            | BuiltinType::Class,
            _,
        ) => None,
    }
}

fn bare_kind(b: BuiltinType) -> Option<RuntimeKind> {
    generic_kind(b, 0)
}
