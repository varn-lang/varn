use super::super::flow::Flow;
use super::super::hoist::block_bindings;
use crate::checker::Checker;
use vn_core::ast::{Decl, StmtId, StmtKind};
use vn_sem::bind::BindResult;

impl<'r> Checker<'r> {
    pub(super) fn da_block(&mut self, stmts: &[StmtId], flow: &mut Flow, bind: &BindResult) {
        let saved = flow.enter(block_bindings(self.ast_arena, stmts));
        for &s in stmts {
            if flow.diverged {
                break;
            }
            self.da_stmt(s, flow, bind);
        }
        flow.leave(saved);
    }

    pub(super) fn da_stmt(&mut self, stmt: StmtId, flow: &mut Flow, bind: &BindResult) {
        let arena = self.ast_arena;
        match &arena.stmt(stmt).kind {
            StmtKind::Block { stmts } => {
                let stmts = stmts.clone();
                self.da_block(&stmts, flow, bind);
            }
            StmtKind::Expr { expression } => self.da_expr(*expression, flow, bind),

            StmtKind::Decl(decl) => {
                let decl = decl.clone();
                if let Decl::Variable(v) = decl.as_ref()
                    && !Self::is_rejected_regex_literal(v, bind)
                {
                    for d in &v.declarators {
                        if let Some(init) = d.init {
                            self.da_expr(init, flow, bind);
                        }
                        for name in d.id.bound_names() {
                            flow.declare(&name, d.init.is_some());
                        }
                    }
                }

                self.da_nested_decl(&decl, bind);
                self.da_declared(&decl, flow, bind);
            }

            StmtKind::Return { argument } => {
                let argument = *argument;
                if let Some(a) = argument {
                    self.da_expr(a, flow, bind);
                }
                flow.diverged = true;
            }
            StmtKind::Throw { argument } => {
                let argument = *argument;
                self.da_expr(argument, flow, bind);
                flow.diverged = true;
            }
            StmtKind::Break { .. } | StmtKind::Continue { .. } => flow.diverged = true,

            StmtKind::If {
                test,
                consequent,
                alternate,
            } => {
                let (test, consequent, alternate) = (*test, *consequent, *alternate);
                self.da_expr(test, flow, bind);
                let mut a = flow.clone();
                self.da_stmt(consequent, &mut a, bind);
                let mut b = flow.clone();
                if let Some(alt) = alternate {
                    self.da_stmt(alt, &mut b, bind);
                }
                flow.merge(a, b);
            }

            StmtKind::While { test, body } | StmtKind::DoWhile { body, test } => {
                let (test, body) = (*test, *body);
                self.da_expr(test, flow, bind);
                let mut inner = flow.clone();
                inner.diverged = false;
                self.da_stmt(body, &mut inner, bind);
            }
            StmtKind::For {
                init,
                test,
                update,
                body,
            } => {
                let init = init.clone();
                let (test, update, body) = (*test, *update, *body);
                if let Some(fi) = &init {
                    match fi.as_ref() {
                        vn_core::ast::ForInit::Expr(e) => self.da_expr(*e, flow, bind),
                        vn_core::ast::ForInit::Var { declarators, .. } => {
                            for d in declarators {
                                if let Some(e) = d.init {
                                    self.da_expr(e, flow, bind);
                                }
                                for n in d.id.bound_names() {
                                    flow.assigned.insert(n);
                                }
                            }
                        }
                    }
                }
                if let Some(t) = test {
                    self.da_expr(t, flow, bind);
                }
                let mut inner = flow.clone();
                inner.diverged = false;
                self.da_stmt(body, &mut inner, bind);
                if let Some(u) = update {
                    self.da_expr(u, &mut inner, bind);
                }
            }
            StmtKind::ForIn {
                right, body, left, ..
            } => {
                let (right, body, left) = (*right, *body, left.clone());
                self.da_expr(right, flow, bind);
                let mut inner = flow.clone();
                inner.diverged = false;
                for n in left.bound_names() {
                    inner.assigned.insert(n);
                }
                self.da_stmt(body, &mut inner, bind);
            }

            StmtKind::Try {
                block,
                catches,
                finally,
            } => {
                let (block, catches, finally) = (*block, catches.clone(), *finally);
                let mut t = flow.clone();
                self.da_stmt(block, &mut t, bind);
                let mut c = flow.clone();
                if let Some(clause) = catches.first() {
                    self.da_stmt(clause.body, &mut c, bind);
                }
                flow.merge(t, c);
                if let Some(f) = finally {
                    self.da_stmt(f, flow, bind);
                }
            }

            StmtKind::Using { declarations, .. } => {
                let declarations = declarations.clone();
                for d in &declarations {
                    if let Some(e) = d.init {
                        self.da_expr(e, flow, bind);
                    }
                    for n in d.id.bound_names() {
                        flow.assigned.insert(n);
                    }
                }
            }
            StmtKind::Labeled { body, .. } => {
                let body = *body;
                self.da_stmt(body, flow, bind);
            }

            StmtKind::Empty | StmtKind::Debugger | StmtKind::Error => {}
        }
    }
}
