use crate::types::{CheckerTyTable, ObjectTypeMember, Type};
use std::sync::Arc;
use vn_core::{Atom, BuiltinType, TypeKind};

pub const TUPLE_HEAD: &str = "()";
pub const RECORD_HEAD: &str = "{}";

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ReceiverPattern {
    Var(u16),
    Named {
        name: Arc<str>,
        args: Vec<ReceiverPattern>,
    },
    Primitive(Arc<str>),
    Array(Box<ReceiverPattern>),
    Tuple(Vec<ReceiverPattern>),
    Record(Vec<(Arc<str>, bool, ReceiverPattern)>),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExtensionCandidate {
    pub mangled: Arc<str>,
    pub pattern: ReceiverPattern,
}

pub type NameOf<'a> = &'a dyn Fn(Atom) -> Option<Arc<str>>;

impl ReceiverPattern {
    pub fn of(
        ty: Type,
        table: &CheckerTyTable,
        name_of: NameOf,
        type_params: &[Arc<str>],
    ) -> Option<Self> {
        match table.get(ty.0) {
            TypeKind::Named(name, _) => {
                let name = name_of(name)?;
                match type_params.iter().position(|p| *p == name) {
                    Some(at) => Some(Self::Var(u16::try_from(at).ok()?)),
                    None => Some(Self::Named {
                        name,
                        args: Vec::new(),
                    }),
                }
            }
            TypeKind::Generic(name, args, _) => {
                let name = name_of(name)?;
                let args: Option<Vec<Self>> = table
                    .get_list(args)
                    .iter()
                    .map(|id| Self::of(Type::resolved(*id), table, name_of, type_params))
                    .collect();
                let args = args?;
                if name.as_ref() == BuiltinType::Array.name() && args.len() == 1 {
                    return args.into_iter().next().map(|el| Self::Array(Box::new(el)));
                }
                Some(Self::Named { name, args })
            }
            TypeKind::Primitive(primitive) => Some(Self::Primitive(Arc::from(primitive.name()))),
            TypeKind::Literal(literal) => Some(Self::Primitive(Arc::from(literal.base().name()))),
            TypeKind::Array(inner) => Some(Self::Array(Box::new(Self::of(
                Type::resolved(inner),
                table,
                name_of,
                type_params,
            )?))),
            TypeKind::Tuple(list) => {
                let items: Option<Vec<Self>> = table
                    .get_list(list)
                    .iter()
                    .map(|id| Self::of(Type::resolved(*id), table, name_of, type_params))
                    .collect();
                Some(Self::Tuple(items?))
            }
            TypeKind::Record(fields) => {
                let mut out = Vec::new();
                for member in table.get_object_members(fields) {
                    match member {
                        ObjectTypeMember::Property {
                            name, ty, optional, ..
                        } => out.push((
                            name.clone(),
                            *optional,
                            Self::of(Type::resolved(*ty), table, name_of, type_params)?,
                        )),
                        ObjectTypeMember::Method { .. } | ObjectTypeMember::Callable { .. } => {
                            return None
                        }
                    }
                }
                out.sort_by(|a, b| a.0.cmp(&b.0));
                Some(Self::Record(out))
            }
            TypeKind::This
            | TypeKind::Union(_)
            | TypeKind::Intersection(_)
            | TypeKind::Fn(_)
            | TypeKind::Interface(_)
            | TypeKind::EnumVariant { .. }
            | TypeKind::TypePredicate { .. } => None,
        }
    }

    pub fn head(&self) -> Option<Arc<str>> {
        match self {
            Self::Var(_) => None,
            Self::Named { name, .. } | Self::Primitive(name) => Some(name.clone()),
            Self::Array(_) => Some(Arc::from(BuiltinType::Array.name())),
            Self::Tuple(_) => Some(Arc::from(TUPLE_HEAD)),
            Self::Record(_) => Some(Arc::from(RECORD_HEAD)),
        }
    }

    pub fn specificity(&self) -> usize {
        match self {
            Self::Var(_) => 0,
            Self::Primitive(_) => 1,
            Self::Named { args, .. } => 1 + args.iter().map(Self::specificity).sum::<usize>(),
            Self::Array(inner) => 1 + inner.specificity(),
            Self::Tuple(items) => 1 + items.iter().map(Self::specificity).sum::<usize>(),
            Self::Record(fields) => 1 + fields.iter().map(|f| f.2.specificity()).sum::<usize>(),
        }
    }

    pub fn matches(&self, concrete: &Self, bound: &mut Vec<Option<Self>>) -> bool {
        match (self, concrete) {
            (Self::Var(at), _) => {
                let at = usize::from(*at);
                if bound.len() <= at {
                    bound.resize(at + 1, None);
                }
                match &bound[at] {
                    Some(previous) => previous == concrete,
                    None => {
                        bound[at] = Some(concrete.clone());
                        true
                    }
                }
            }
            (Self::Named { name: a, args: x }, Self::Named { name: b, args: y }) => {
                a == b && x.len() == y.len() && x.iter().zip(y).all(|(p, c)| p.matches(c, bound))
            }
            (Self::Primitive(a), Self::Primitive(b)) => a == b,
            (Self::Array(p), Self::Array(c)) => p.matches(c, bound),
            (Self::Tuple(x), Self::Tuple(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(p, c)| p.matches(c, bound))
            }
            (Self::Record(x), Self::Record(y)) => {
                x.len() == y.len()
                    && x.iter()
                        .zip(y)
                        .all(|(p, c)| p.0 == c.0 && p.1 == c.1 && p.2.matches(&c.2, bound))
            }
            (Self::Named { .. }, _)
            | (Self::Primitive(_), _)
            | (Self::Array(_), _)
            | (Self::Tuple(_), _)
            | (Self::Record(_), _) => false,
        }
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Var(_), _) | (_, Self::Var(_)) => true,
            (Self::Named { name: a, args: x }, Self::Named { name: b, args: y }) => {
                a == b && x.len() == y.len() && x.iter().zip(y).all(|(p, q)| p.overlaps(q))
            }
            (Self::Primitive(a), Self::Primitive(b)) => a == b,
            (Self::Array(p), Self::Array(q)) => p.overlaps(q),
            (Self::Tuple(x), Self::Tuple(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(p, q)| p.overlaps(q))
            }
            (Self::Record(x), Self::Record(y)) => {
                x.len() == y.len()
                    && x.iter()
                        .zip(y)
                        .all(|(p, q)| p.0 == q.0 && p.1 == q.1 && p.2.overlaps(&q.2))
            }
            (Self::Named { .. }, _)
            | (Self::Primitive(_), _)
            | (Self::Array(_), _)
            | (Self::Tuple(_), _)
            | (Self::Record(_), _) => false,
        }
    }
}
