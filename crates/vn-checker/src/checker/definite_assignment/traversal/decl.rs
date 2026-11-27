use super::super::flow::Flow;
use super::super::hoist::block_bindings;
use crate::checker::Checker;
use vn_core::ast::{ClassMember, Decl, ExportDecl, StmtId, StmtKind};
use vn_core::{Atom, Diagnostic, ErrorCode};
use vn_sem::bind::BindResult;

impl<'r> Checker<'r> {
    pub(super) fn reject_param_redeclaration(
        &mut self,
        params: &[vn_core::ast::Param],
        body: StmtId,
        bind: &BindResult,
    ) {
        let StmtKind::Block { stmts } = &self.ast_arena.stmt(body).kind else {
            return;
        };
        let names: Vec<Atom> = params
            .iter()
            .flat_map(|p| p.pattern.bound_names())
            .collect();
        for hoisted in block_bindings(self.ast_arena, stmts) {
            if hoisted.is_binding && names.contains(&hoisted.name) {
                let text = bind.interner.resolve(hoisted.name);
                self.emit(
                    Diagnostic::error(
                        ErrorCode::DuplicateDeclaration,
                        format!("'{text}' is already declared as a parameter of this function"),
                    )
                    .with_range(hoisted.range),
                );
            }
        }
    }

    pub(super) fn da_declared(&mut self, decl: &Decl, flow: &mut Flow, bind: &BindResult) {
        match decl {
            Decl::Class(c) => {
                if let Some(id) = c.id {
                    flow.declare(&id, true);
                }
                for member in &c.body {
                    match member {
                        ClassMember::Property {
                            init: Some(init),
                            modifiers,
                            ..
                        } if modifiers.is_static => self.da_expr(*init, flow, bind),
                        ClassMember::StaticBlock { body, .. } => {
                            let mut inner = flow.clone();
                            self.da_stmt(*body, &mut inner, bind);
                        }
                        ClassMember::Constructor { .. }
                        | ClassMember::Method { .. }
                        | ClassMember::Property { .. }
                        | ClassMember::Getter { .. }
                        | ClassMember::Setter { .. } => {}
                    }
                }
            }
            Decl::Enum(e) => flow.declare(&e.id, true),
            Decl::Export(ExportDecl::Decl { declaration, .. }) => {
                self.da_declared(declaration, flow, bind)
            }
            Decl::Variable(_)
            | Decl::Function(_)
            | Decl::Interface(_)
            | Decl::TypeAlias(_)
            | Decl::Namespace(_)
            | Decl::Import(_)
            | Decl::Export(_)
            | Decl::Extension(_) => {}
        }
    }

    pub(super) fn da_nested_decl(&mut self, decl: &Decl, bind: &BindResult) {
        match decl {
            Decl::Function(f) => {
                self.reject_param_redeclaration(&f.params, f.body, bind);
                let mut flow = Flow::default();
                self.da_stmt(f.body, &mut flow, bind);
            }
            Decl::Class(c) => {
                self.check_class_fields(c, bind);
                for m in &c.body {
                    use vn_core::ast::ClassMember;
                    let body = match m {
                        ClassMember::Method { body: Some(b), .. }
                        | ClassMember::Getter { body: Some(b), .. }
                        | ClassMember::Setter { body: Some(b), .. } => Some(*b),
                        ClassMember::Constructor { body, .. }
                        | ClassMember::StaticBlock { body, .. } => Some(*body),
                        ClassMember::Method { .. }
                        | ClassMember::Property { .. }
                        | ClassMember::Getter { .. }
                        | ClassMember::Setter { .. } => None,
                    };
                    if let Some(b) = body {
                        if let ClassMember::Method { params, .. }
                        | ClassMember::Constructor { params, .. } = m
                        {
                            self.reject_param_redeclaration(params, b, bind);
                        }
                        let mut flow = Flow::default();
                        self.da_stmt(b, &mut flow, bind);
                    }
                }
            }
            Decl::Export(vn_core::ast::ExportDecl::Decl { declaration, .. }) => {
                self.da_nested_decl(declaration, bind);
            }
            Decl::Namespace(ns) => {
                for inner in &ns.body {
                    self.da_nested_decl(inner, bind);
                }
            }
            Decl::Variable(_)
            | Decl::Interface(_)
            | Decl::TypeAlias(_)
            | Decl::Enum(_)
            | Decl::Import(_)
            | Decl::Export(_)
            | Decl::Extension(_) => {}
        }
    }
}
