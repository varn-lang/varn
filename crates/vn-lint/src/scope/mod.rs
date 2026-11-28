use crate::TypeFacts;
use std::collections::HashMap;
use vn_core::ast::{
    Arg, ArrayEl, ArrowBody, ClassDecl, ClassMember, Decl, EnumDecl, ExportDecl, ExprKind,
    ExtensionDecl, ExtensionMember, FunctionDecl, ImportSpecifier, InterfaceDecl, InterfaceMember,
    MatchBody, MatchPattern, ObjectProp, Param, Pattern, PropKey, StmtKind, TemplatePart,
    TypeAliasDecl, VarKind, VariantPayload,
};
use vn_core::ast::{AstArena, ExprId, StmtId};
use vn_core::ast::{Decorator, TypeNode, TypeParam};
use vn_core::{Atom, AtomInterner, SourceRange, TypeKind};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Let,
    Const,
    Param,
    Import,
    Function,
    Class,
    Enum,
    Interface,
    TypeAlias,
    CatchParam,
    MatchBinding,
    Exempt,
}

pub struct Binding {
    pub name: Atom,
    pub kind: Kind,
    pub start: u32,
    pub end: u32,
    pub reads: u32,
    pub writes: u32,
    pub exported: bool,
    pub initialized: bool,
    pub param_of: Option<(usize, usize)>,
    pub renamable: bool,
    pub shorthand: bool,
    pub import: Option<(usize, usize)>,
}

pub struct ImportInfo {
    pub start: u32,
    pub end: u32,
    pub specifiers: Vec<(u32, u32)>,
}

pub struct Group {
    pub keyword_start: u32,
    pub fixable: bool,
    pub members: Vec<usize>,
}

pub struct Scopes<'a> {
    pub facts: &'a TypeFacts,
    pub arena: &'a AstArena,
    pub interner: &'a AtomInterner,
    pub bindings: Vec<Binding>,
    pub groups: Vec<Group>,
    pub imports: Vec<ImportInfo>,
    pub catch_heads: HashMap<usize, (u32, u32)>,
    pub functions: usize,
    pub(super) stack: Vec<HashMap<Atom, usize>>,
}

#[derive(Clone)]
pub(super) struct Declare {
    kind: Kind,
    exported: bool,
    initialized: bool,
    group: Option<usize>,
    param_of: Option<(usize, usize)>,
    renamable: bool,
    shorthand: bool,
    import: Option<(usize, usize)>,
}

pub(super) fn positional(pattern: &Pattern) -> bool {
    matches!(
        pattern,
        Pattern::Identifier { .. } | Pattern::Array { .. } | Pattern::Tuple { .. }
    )
}

mod decl;
mod expr;
mod sites;
mod stmt;
mod types;

impl<'a> Scopes<'a> {
    pub fn new(arena: &'a AstArena, interner: &'a AtomInterner, facts: &'a TypeFacts) -> Self {
        Self {
            facts,
            arena,
            interner,
            bindings: Vec::new(),
            groups: Vec::new(),
            imports: Vec::new(),
            catch_heads: HashMap::new(),
            functions: 0,
            stack: Vec::new(),
        }
    }
    pub fn program(&mut self, body: &[StmtId]) {
        self.stack.push(HashMap::new());
        self.hoist(body);
        for stmt in body {
            self.stmt(*stmt);
        }
        self.stack.pop();
    }
    pub(super) fn declare(&mut self, name: Atom, start: u32, end: u32, how: Declare) {
        let index = self.bindings.len();
        self.bindings.push(Binding {
            name,
            kind: how.kind,
            start,
            end,
            reads: 0,
            writes: 0,
            exported: how.exported,
            initialized: how.initialized,
            param_of: how.param_of,
            renamable: how.renamable,
            shorthand: how.shorthand,
            import: how.import,
        });
        if let Some(group) = how.group {
            self.groups[group].members.push(index);
        }
        if let Some(scope) = self.stack.last_mut() {
            scope.insert(name, index);
        }
    }
    pub(super) fn lookup(&self, name: Atom) -> Option<usize> {
        self.stack
            .iter()
            .rev()
            .find_map(|scope| scope.get(&name).copied())
    }
    pub(super) fn read(&mut self, name: Atom) {
        if let Some(index) = self.lookup(name) {
            self.bindings[index].reads += 1;
        }
    }
    pub(super) fn read_qualified(&mut self, name: Atom) {
        let text = self.interner.resolve(name);
        let root = text.split('.').next().unwrap_or(text);
        let interner = self.interner;
        let found = self.stack.iter().rev().find_map(|scope| {
            scope
                .iter()
                .find(|(atom, _)| interner.resolve(**atom) == root)
                .map(|(_, index)| *index)
        });
        if let Some(index) = found {
            self.bindings[index].reads += 1;
        }
    }
    pub(super) fn write(&mut self, name: Atom) {
        if let Some(index) = self.lookup(name) {
            self.bindings[index].writes += 1;
        }
    }
    pub(super) fn exempt(&mut self, name: Atom, start: u32, end: u32) {
        self.declare(
            name,
            start,
            end,
            Declare {
                kind: Kind::Exempt,
                exported: false,
                initialized: true,
                group: None,
                param_of: None,
                renamable: false,
                shorthand: false,
                import: None,
            },
        );
    }
    fn declare_type(&mut self, name: Atom, offset: u32, kind: Kind, exported: bool) {
        let length = self.interner.resolve(name).len() as u32;
        self.declare(
            name,
            offset,
            offset + length,
            Declare {
                kind,
                exported,
                initialized: true,
                group: None,
                param_of: None,
                renamable: false,
                shorthand: false,
                import: None,
            },
        );
    }
    pub(super) fn hoist(&mut self, body: &[StmtId]) {
        for id in body {
            if let StmtKind::Decl(decl) = &self.arena.stmt(*id).kind {
                self.hoist_decl(decl, false);
            }
        }
    }
    pub(super) fn hoist_decls(&mut self, decls: &[Decl]) {
        for decl in decls {
            self.hoist_decl(decl, false);
        }
    }
    pub(super) fn hoist_decl(&mut self, decl: &Decl, exported: bool) {
        match decl {
            Decl::Variable(variable) => {
                let group = self.groups.len();
                self.groups.push(Group {
                    keyword_start: variable.range.start.offset,
                    fixable: variable.kind == VarKind::Let && !exported && !variable.is_declare,
                    members: Vec::new(),
                });
                let kind = if variable.kind == VarKind::Let {
                    Kind::Let
                } else {
                    Kind::Const
                };
                for declarator in &variable.declarators {
                    self.declare_pattern(
                        &declarator.id,
                        &Declare {
                            kind: if variable.is_declare {
                                Kind::Exempt
                            } else {
                                kind
                            },
                            exported,
                            initialized: declarator.init.is_some(),
                            group: Some(group),
                            param_of: None,
                            renamable: positional(&declarator.id),
                            shorthand: false,
                            import: None,
                        },
                    );
                }
            }
            Decl::Function(function) => {
                let how = Declare {
                    kind: if function.decorators.is_empty() && !function.modifiers.is_declare {
                        Kind::Function
                    } else {
                        Kind::Exempt
                    },
                    exported,
                    initialized: true,
                    group: None,
                    param_of: None,
                    renamable: false,
                    shorthand: false,
                    import: None,
                };
                let start = function.id_offset;
                let length = self.interner.resolve(function.id).len() as u32;
                self.declare(function.id, start, start + length, how);
            }
            Decl::Class(class) => {
                if let Some(id) = class.id {
                    let how = Declare {
                        kind: if class.decorators.is_empty() && !class.modifiers.is_declare {
                            Kind::Class
                        } else {
                            Kind::Exempt
                        },
                        exported,
                        initialized: true,
                        group: None,
                        param_of: None,
                        renamable: false,
                        shorthand: false,
                        import: None,
                    };
                    let start = class.id_offset;
                    let length = self.interner.resolve(id).len() as u32;
                    self.declare(id, start, start + length, how);
                }
            }
            Decl::Interface(interface) => {
                self.declare_type(interface.id, interface.id_offset, Kind::Interface, exported)
            }
            Decl::TypeAlias(alias) => {
                self.declare_type(alias.id, alias.id_offset, Kind::TypeAlias, exported)
            }
            Decl::Enum(declaration) => {
                let kind = if declaration.is_declare {
                    Kind::Exempt
                } else {
                    Kind::Enum
                };
                self.declare_type(declaration.id, declaration.id_offset, kind, exported)
            }
            Decl::Namespace(namespace) => self.exempt(namespace.id, 0, 0),
            Decl::Import(import) => {
                let decl = self.imports.len();
                self.imports.push(ImportInfo {
                    start: import.range.start.offset,
                    end: import.range.end.offset,
                    specifiers: import
                        .specifiers
                        .iter()
                        .map(|specifier| {
                            let range = match specifier {
                                ImportSpecifier::Named { range, .. }
                                | ImportSpecifier::Namespace { range, .. } => range,
                            };
                            (range.start.offset, range.end.offset)
                        })
                        .collect(),
                });
                for (position, specifier) in import.specifiers.iter().enumerate() {
                    let (local, range) = match specifier {
                        ImportSpecifier::Named { local, range, .. }
                        | ImportSpecifier::Namespace { local, range } => (*local, range),
                    };
                    self.declare(
                        local,
                        range.start.offset,
                        range.end.offset,
                        Declare {
                            kind: Kind::Import,
                            exported: false,
                            initialized: true,
                            group: None,
                            param_of: None,
                            renamable: false,
                            shorthand: false,
                            import: Some((decl, position)),
                        },
                    );
                }
            }
            Decl::Export(export) => {
                if let ExportDecl::Decl { declaration, .. } = export {
                    self.hoist_decl(declaration, true);
                }
            }
            Decl::Extension(_) => {}
        }
    }
    pub(super) fn declare_pattern(&mut self, pattern: &Pattern, how: &Declare) {
        match pattern {
            Pattern::Identifier { name, range, .. } => self.declare(
                *name,
                range.start.offset,
                range.end.offset,
                Declare {
                    kind: how.kind,
                    exported: how.exported,
                    initialized: how.initialized,
                    group: how.group,
                    param_of: how.param_of,
                    renamable: how.renamable,
                    shorthand: how.shorthand,
                    import: how.import,
                },
            ),
            Pattern::Array { elements, rest, .. } | Pattern::Tuple { elements, rest, .. } => {
                for element in elements.iter().flatten() {
                    self.declare_pattern(&element.pattern, how);
                }
                if let Some(rest) = rest {
                    self.declare_pattern(rest, how);
                }
            }
            Pattern::Object {
                properties, rest, ..
            } => {
                let keyed = Declare {
                    renamable: false,
                    ..how.clone()
                };
                for property in properties {
                    self.declare_pattern(&property.value, &keyed);
                }
                if let Some(rest) = rest {
                    self.declare_pattern(rest, how);
                }
            }
            Pattern::Assignment { left, right, .. } => {
                self.expr(*right);
                self.declare_pattern(left, how);
            }
            Pattern::Rest { argument, .. } => self.declare_pattern(argument, how),
        }
    }
}
