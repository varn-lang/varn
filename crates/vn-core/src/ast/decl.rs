use super::arena::{ExprId, StmtId};
use super::expr::AstId;
use super::module_decl::{ExportDecl, ImportDecl};
use super::operators::{Modifiers, Visibility};
use super::pattern::Param;
use super::stmt::VariableDecl;
use super::types::{Decorator, TypeNode, TypeParam};
use crate::source::SourceRange;

#[derive(Clone, Debug)]
pub enum Decl {
    Variable(VariableDecl),
    Function(FunctionDecl),
    Class(ClassDecl),
    Interface(InterfaceDecl),
    TypeAlias(TypeAliasDecl),
    Enum(EnumDecl),
    Namespace(NamespaceDecl),
    Import(ImportDecl),
    Export(ExportDecl),
    Extension(ExtensionDecl),
}

impl Decl {
    pub fn id(&self) -> AstId {
        match self {
            Decl::Variable(d) => d.ast_id,
            Decl::Function(d) => d.ast_id,
            Decl::Class(d) => d.ast_id,
            Decl::Interface(d) => d.ast_id,
            Decl::TypeAlias(d) => d.ast_id,
            Decl::Enum(d) => d.ast_id,
            Decl::Namespace(d) => d.ast_id,
            Decl::Import(d) => d.ast_id,
            Decl::Export(d) => d.id(),
            Decl::Extension(d) => d.ast_id,
        }
    }

    pub fn set_range(&mut self, range: SourceRange) {
        match self {
            Decl::Variable(d) => d.range = range,
            Decl::Function(d) => d.range = range,
            Decl::Class(d) => d.range = range,
            Decl::Interface(d) => d.range = range,
            Decl::TypeAlias(d) => d.range = range,
            Decl::Enum(d) => d.range = range,
            Decl::Namespace(d) => d.range = range,
            Decl::Import(d) => d.range = range,
            Decl::Export(d) => d.set_range(range),
            Decl::Extension(d) => d.range = range,
        }
    }

    pub fn range(&self) -> &SourceRange {
        match self {
            Decl::Variable(d) => &d.range,
            Decl::Function(d) => &d.range,
            Decl::Class(d) => &d.range,
            Decl::Interface(d) => &d.range,
            Decl::TypeAlias(d) => &d.range,
            Decl::Enum(d) => &d.range,
            Decl::Namespace(d) => &d.range,
            Decl::Import(d) => &d.range,
            Decl::Export(d) => d.range(),
            Decl::Extension(d) => &d.range,
        }
    }
}

use crate::Atom;

#[derive(Clone, Debug)]
pub struct FunctionDecl {
    pub id: Atom,
    pub ast_id: AstId,

    pub id_offset: u32,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeNode>,
    pub body: StmtId,
    pub modifiers: Modifiers,
    pub decorators: Vec<Decorator>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct ClassDecl {
    pub id: Option<Atom>,
    pub ast_id: AstId,

    pub id_offset: u32,
    pub type_params: Vec<TypeParam>,
    pub super_class: Option<ExprId>,
    pub super_type_args: Vec<TypeNode>,
    pub implements: Vec<TypeNode>,
    pub body: Vec<ClassMember>,
    pub modifiers: Modifiers,
    pub decorators: Vec<Decorator>,
    pub range: SourceRange,
    pub is_struct: bool,
}

#[derive(Clone, Debug)]
pub enum ClassMember {
    Constructor {
        params: Vec<Param>,
        body: StmtId,
        decorators: Vec<Decorator>,
        range: SourceRange,
    },
    Method {
        key: Atom,
        key_range: SourceRange,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        return_type: Option<TypeNode>,
        body: Option<StmtId>,
        modifiers: Modifiers,
        decorators: Vec<Decorator>,
        range: SourceRange,
    },
    Property {
        key: Atom,
        key_range: SourceRange,
        type_ann: Option<TypeNode>,
        init: Option<ExprId>,
        modifiers: Modifiers,
        decorators: Vec<Decorator>,
        range: SourceRange,
    },
    Getter {
        key: Atom,
        key_range: SourceRange,
        return_type: Option<TypeNode>,
        body: Option<StmtId>,
        modifiers: Modifiers,
        decorators: Vec<Decorator>,
        range: SourceRange,
    },
    Setter {
        key: Atom,
        key_range: SourceRange,
        param: Param,
        body: Option<StmtId>,
        modifiers: Modifiers,
        decorators: Vec<Decorator>,
        range: SourceRange,
    },
    StaticBlock {
        body: StmtId,
        range: SourceRange,
    },
    Field {
        key: Atom,
        key_range: SourceRange,
        type_ann: TypeNode,
        default: Option<ExprId>,
        visibility: Option<Visibility>,
        range: SourceRange,
    },
}

#[derive(Clone, Debug)]
pub struct PositionalConstructor {
    pub params: Vec<Param>,
    pub range: SourceRange,
}

impl ClassDecl {
    pub fn positional_constructor(&self) -> Option<PositionalConstructor> {
        if !self.is_struct {
            return None;
        }
        let mut params = Vec::new();
        let mut span: Option<SourceRange> = None;
        for member in &self.body {
            let ClassMember::Field {
                key,
                key_range,
                type_ann,
                default,
                visibility,
                range,
                ..
            } = member
            else {
                continue;
            };
            params.push(Param {
                pattern: super::pattern::Pattern::Identifier {
                    name: *key,
                    range: *key_range,
                    origin: super::pattern::BindingOrigin::Written,
                },
                type_ann: Some(type_ann.clone()),
                default: *default,
                is_rest: false,
                is_optional: false,
                modifiers: Modifiers {
                    visibility: *visibility,
                    is_readonly: true,
                    ..Default::default()
                },
                range: *range,
            });
            span = Some(match span {
                Some(existing) => SourceRange {
                    start: existing.start,
                    end: range.end,
                },
                None => *range,
            });
        }
        Some(PositionalConstructor {
            params,
            range: span.unwrap_or(self.range),
        })
    }
}

#[derive(Clone, Debug)]
pub struct InterfaceDecl {
    pub id: Atom,
    pub id_offset: u32,
    pub ast_id: AstId,
    pub type_params: Vec<TypeParam>,
    pub extends: Vec<TypeNode>,
    pub body: Vec<InterfaceMember>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum InterfaceMember {
    Property {
        key: Atom,
        key_range: SourceRange,
        type_ann: TypeNode,
        optional: bool,
        readonly: bool,
        range: SourceRange,
    },
    Method {
        key: Atom,
        key_range: SourceRange,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        return_type: Option<TypeNode>,
        optional: bool,

        is_async: bool,
        range: SourceRange,
    },
    Callable {
        params: Vec<Param>,
        return_type: TypeNode,
        range: SourceRange,
    },
}

#[derive(Clone, Debug)]
pub struct TypeAliasDecl {
    pub id: Atom,
    pub id_offset: u32,
    pub ast_id: AstId,
    pub type_params: Vec<TypeParam>,
    pub alias: TypeNode,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct EnumDecl {
    pub id: Atom,
    pub id_offset: u32,
    pub ast_id: AstId,
    pub type_params: Vec<TypeParam>,
    pub implements: Vec<TypeNode>,
    pub members: Vec<EnumMember>,
    pub body: Vec<ClassMember>,
    pub is_declare: bool,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct EnumMember {
    pub id: Atom,
    pub id_range: SourceRange,
    pub init: Option<ExprId>,

    pub payload_fields: Vec<EnumField>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct EnumField {
    pub name: Atom,
    pub name_range: SourceRange,
    pub ty: TypeNode,
    pub init: Option<ExprId>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct NamespaceDecl {
    pub id: Atom,
    pub id_offset: u32,
    pub ast_id: AstId,
    pub body: Vec<Decl>,
    pub is_declare: bool,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub struct ExtensionDecl {
    pub id: Option<Atom>,
    pub ast_id: AstId,
    pub type_params: Vec<TypeParam>,
    pub target: TypeNode,
    pub members: Vec<ExtensionMember>,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub enum ExtensionMember {
    Method(FunctionDecl),
    Getter {
        key: Atom,
        key_range: SourceRange,
        return_type: Option<TypeNode>,
        body: StmtId,
        modifiers: Modifiers,
        range: SourceRange,
    },
    Setter {
        key: Atom,
        key_range: SourceRange,
        param: Param,
        body: StmtId,
        modifiers: Modifiers,
        range: SourceRange,
    },
}
