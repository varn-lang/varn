use super::arena::ExprId;
use super::decl::InterfaceMember;
use super::expr::AstId;
use crate::kinds::TypeKind;
use crate::source::SourceRange;
use crate::Atom;

pub type AstTypeKind = TypeKind<
    Box<TypeNode>,
    Atom,
    Vec<TypeNode>,
    (Vec<FnTypeParam>, Box<TypeNode>),
    Vec<InterfaceMember>,
    std::convert::Infallible,
    QualifiedName,
    (),
>;

#[derive(Clone, Debug)]
pub struct NameSegment {
    pub name: Atom,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct QualifiedName {
    pub segments: Vec<NameSegment>,
}

impl QualifiedName {
    pub fn single(name: Atom, range: SourceRange) -> Self {
        QualifiedName {
            segments: vec![NameSegment { name, range }],
        }
    }

    pub fn simple(&self) -> Option<Atom> {
        match self.segments.as_slice() {
            [only] => Some(only.name),
            _ => None,
        }
    }

    pub fn root(&self) -> Atom {
        self.segments[0].name
    }

    pub fn last(&self) -> Atom {
        self.segments[self.segments.len() - 1].name
    }

    pub fn range(&self) -> SourceRange {
        SourceRange {
            start: self.segments[0].range.start,
            end: self.segments[self.segments.len() - 1].range.end,
        }
    }

    pub fn joined(&self, interner: &crate::AtomInterner, separator: &str) -> String {
        let mut text = String::new();
        for (at, segment) in self.segments.iter().enumerate() {
            if at > 0 {
                text.push_str(separator);
            }
            text.push_str(interner.resolve(segment.name));
        }
        text
    }

    pub fn dotted(&self, interner: &crate::AtomInterner) -> String {
        self.joined(interner, ".")
    }

    pub fn scoped(&self, interner: &crate::AtomInterner) -> String {
        self.joined(interner, "::")
    }
}

#[derive(Clone, Debug)]
pub struct TypeNode {
    pub id: AstId,
    pub kind: AstTypeKind,
    pub range: SourceRange,
}

impl TypeNode {
    pub fn id(&self) -> AstId {
        self.id
    }

    pub fn range(&self) -> &SourceRange {
        &self.range
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeclaredVariance {
    Out,
    In,
}

#[derive(Clone, Debug)]
pub struct TypeParam {
    pub variance: Option<DeclaredVariance>,
    pub name: Atom,
    pub constraint: Option<TypeNode>,
    pub default: Option<TypeNode>,
    pub is_rest: bool,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct FnTypeParam {
    pub name: Option<Atom>,
    pub ty: TypeNode,
    pub is_rest: bool,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct Decorator {
    pub expression: ExprId,
    pub range: SourceRange,
}
