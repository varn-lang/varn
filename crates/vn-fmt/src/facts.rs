use crate::tokens::TokenStream;
use std::collections::{BTreeMap, HashSet};
use vn_core::ast::{
    AstArena, ClassDecl, ClassMember, Decl, EnumDecl, ExportDecl, ExprKind, ExtensionDecl,
    ExtensionMember, InterfaceDecl, InterfaceMember, NamespaceDecl, Program, StmtKind,
};
use vn_core::{BinOp, TokenKind};

pub struct Facts {
    pub chains: std::collections::HashMap<usize, Vec<crate::chains::Chain>>,
    pub assigns: std::collections::HashMap<usize, usize>,
    pub containers: BTreeMap<usize, Vec<u32>>,
    pub root_items: Vec<u32>,
    pub ternary: HashSet<usize>,
    pub comparisons: HashSet<usize>,
    pub decorator_ends: HashSet<usize>,
}

struct Collector<'a> {
    stream: &'a TokenStream,
    containers: BTreeMap<usize, Vec<u32>>,
    decorators: Vec<(u32, u32)>,
    comparisons: HashSet<usize>,
}

impl<'a> Collector<'a> {
    fn token_at(&self, offset: u32) -> Option<usize> {
        let index = self.stream.index_at_or_after(offset);
        (index < self.stream.toks.len() && self.stream.toks[index].start == offset).then_some(index)
    }

    fn container(&mut self, close_end: u32, items: Vec<u32>) {
        let Some(close_start) = close_end.checked_sub(1) else {
            return;
        };
        let Some(close) = self.token_at(close_start) else {
            return;
        };
        if self.stream.toks[close].kind != TokenKind::RBrace {
            return;
        }
        let Some(open) = self.stream.open_of[close] else {
            return;
        };
        self.containers.insert(open, items);
    }

    fn decorators(&mut self, list: &[vn_core::ast::Decorator]) {
        for decorator in list {
            self.decorators
                .push((decorator.range.start.offset, decorator.range.end.offset));
        }
    }

    fn class(&mut self, class: &ClassDecl) {
        self.decorators(&class.decorators);
        let mut items = Vec::new();
        for member in &class.body {
            match member {
                ClassMember::Constructor { params, .. } if class.is_struct => {
                    items.extend(params.iter().map(|param| param.range.start.offset));
                }
                ClassMember::Constructor { .. }
                | ClassMember::Method { .. }
                | ClassMember::Property { .. }
                | ClassMember::Getter { .. }
                | ClassMember::Setter { .. }
                | ClassMember::StaticBlock { .. } => items.push(self.member(member)),
            }
        }
        self.container(class.range.end.offset, items);
    }

    fn member(&mut self, member: &ClassMember) -> u32 {
        match member {
            ClassMember::Constructor {
                decorators, range, ..
            }
            | ClassMember::Method {
                decorators, range, ..
            }
            | ClassMember::Property {
                decorators, range, ..
            }
            | ClassMember::Getter {
                decorators, range, ..
            }
            | ClassMember::Setter {
                decorators, range, ..
            } => {
                self.decorators(decorators);
                range.start.offset
            }
            ClassMember::StaticBlock { range, .. } => range.start.offset,
        }
    }

    fn interface(&mut self, interface: &InterfaceDecl) {
        let items = interface
            .body
            .iter()
            .map(|member| match member {
                InterfaceMember::Property { range, .. }
                | InterfaceMember::Method { range, .. }
                | InterfaceMember::Callable { range, .. } => range.start.offset,
            })
            .collect();
        self.container(interface.range.end.offset, items);
    }

    fn enumeration(&mut self, declaration: &EnumDecl) {
        let mut items: Vec<u32> = declaration
            .members
            .iter()
            .map(|member| member.range.start.offset)
            .collect();
        for member in &declaration.body {
            items.push(self.member(member));
        }
        items.sort_unstable();
        self.container(declaration.range.end.offset, items);
    }

    fn namespace(&mut self, namespace: &NamespaceDecl) {
        let items = namespace
            .body
            .iter()
            .map(|decl| decl.range().start.offset)
            .collect();
        self.container(namespace.range.end.offset, items);
        for decl in &namespace.body {
            self.decl(decl);
        }
    }

    fn extension(&mut self, extension: &ExtensionDecl) {
        let items = extension
            .members
            .iter()
            .map(|member| match member {
                ExtensionMember::Method(function) => {
                    self.decorators(&function.decorators);
                    function.range.start.offset
                }
                ExtensionMember::Getter { range, .. } | ExtensionMember::Setter { range, .. } => {
                    range.start.offset
                }
            })
            .collect();
        self.container(extension.range.end.offset, items);
    }

    fn decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Class(class) => self.class(class),
            Decl::Interface(interface) => self.interface(interface),
            Decl::Enum(declaration) => self.enumeration(declaration),
            Decl::Namespace(namespace) => self.namespace(namespace),
            Decl::Extension(extension) => self.extension(extension),
            Decl::Function(function) => self.decorators(&function.decorators),
            Decl::Export(export) => {
                if let ExportDecl::Decl { declaration, .. } = export {
                    self.decl(declaration);
                }
            }
            Decl::Variable(_) | Decl::TypeAlias(_) | Decl::Import(_) => {}
        }
    }
}

pub fn collect(stream: &TokenStream, program: &Program, arena: &AstArena) -> Facts {
    let mut collector = Collector {
        stream,
        containers: BTreeMap::new(),
        decorators: Vec::new(),
        comparisons: HashSet::new(),
    };

    for node in arena.stmts() {
        match &node.kind {
            StmtKind::Block { stmts } => {
                let items = stmts
                    .iter()
                    .map(|id| arena.stmt(*id).range.start.offset)
                    .collect();
                collector.container(node.range.end.offset, items);
            }
            StmtKind::Decl(decl) => collector.decl(decl),
            StmtKind::Empty
            | StmtKind::Expr { .. }
            | StmtKind::Error
            | StmtKind::If { .. }
            | StmtKind::While { .. }
            | StmtKind::DoWhile { .. }
            | StmtKind::For { .. }
            | StmtKind::ForIn { .. }
            | StmtKind::Return { .. }
            | StmtKind::Break { .. }
            | StmtKind::Continue { .. }
            | StmtKind::Throw { .. }
            | StmtKind::Try { .. }
            | StmtKind::Using { .. }
            | StmtKind::Labeled { .. }
            | StmtKind::Debugger => {}
        }
    }

    for node in arena.exprs() {
        match &node.kind {
            ExprKind::Match { cases, .. } => {
                let items = cases.iter().map(|case| case.range.start.offset).collect();
                collector.container(node.range.end.offset, items);
            }
            ExprKind::ClassExpr { declaration } => collector.class(declaration),
            ExprKind::Binary {
                op: BinOp::Lt | BinOp::Gt,
                left,
                ..
            } => {
                let from = stream.index_at_or_after(arena.expr(*left).range.end.offset);
                if let Some(index) = (from..stream.toks.len()).take(4).find(|index| {
                    matches!(
                        stream.toks[*index].kind,
                        TokenKind::LAngle | TokenKind::RAngle
                    )
                }) {
                    collector.comparisons.insert(index);
                }
            }
            ExprKind::IntLiteral { .. }
            | ExprKind::FloatLiteral { .. }
            | ExprKind::BigIntLiteral { .. }
            | ExprKind::DecimalLiteral { .. }
            | ExprKind::StrLiteral { .. }
            | ExprKind::CharLiteral { .. }
            | ExprKind::BoolLiteral { .. }
            | ExprKind::NullLiteral
            | ExprKind::Template { .. }
            | ExprKind::TaggedTemplate { .. }
            | ExprKind::Identifier { .. }
            | ExprKind::Missing
            | ExprKind::This
            | ExprKind::Super
            | ExprKind::ImportMeta(_)
            | ExprKind::Array { .. }
            | ExprKind::Object { .. }
            | ExprKind::Tuple { .. }
            | ExprKind::Unary { .. }
            | ExprKind::Update { .. }
            | ExprKind::Binary { .. }
            | ExprKind::Logical { .. }
            | ExprKind::Assign { .. }
            | ExprKind::Conditional { .. }
            | ExprKind::Member { .. }
            | ExprKind::Call { .. }
            | ExprKind::New { .. }
            | ExprKind::Function { .. }
            | ExprKind::Arrow { .. }
            | ExprKind::Sequence { .. }
            | ExprKind::Paren { .. }
            | ExprKind::Await { .. }
            | ExprKind::Yield { .. }
            | ExprKind::Spread { .. }
            | ExprKind::Pipeline { .. }
            | ExprKind::Range { .. }
            | ExprKind::NonNull { .. }
            | ExprKind::Try { .. }
            | ExprKind::As { .. }
            | ExprKind::Satisfies { .. }
            | ExprKind::Is { .. }
            | ExprKind::With { .. }
            | ExprKind::MetaAccess { .. } => {}
        }
    }

    let scanner = crate::chains::Scanner { stream, arena };
    let mut shapes = scanner.scan();
    for node in arena.stmts() {
        let StmtKind::Decl(decl) = &node.kind else {
            continue;
        };
        let Decl::Variable(variable) = decl.as_ref() else {
            continue;
        };
        for declarator in &variable.declarators {
            let Some(init) = declarator.init else {
                continue;
            };
            let before = declarator
                .type_ann
                .as_ref()
                .map(|ty| ty.range.end.offset)
                .unwrap_or(declarator.id.range().end.offset);
            if let Some(op) = scanner.operator_after(before) {
                scanner.assignment(arena.expr(init), op, &mut shapes);
            }
        }
    }

    let root_items: Vec<u32> = program
        .body
        .iter()
        .map(|id| arena.stmt(*id).range.start.offset)
        .collect();

    let mut decorator_ends = HashSet::new();
    for (_, end) in &collector.decorators {
        let last = stream.index_at_or_after(*end);
        if last > 0 {
            decorator_ends.insert(last - 1);
        }
    }

    let containers = collector.containers;

    Facts {
        containers,
        root_items,
        ternary: shapes.ternary,
        chains: shapes.chains,
        assigns: shapes.assigns,
        comparisons: collector.comparisons,
        decorator_ends,
    }
}
