use super::*;

impl<'a> Scopes<'a> {
    pub(super) fn stmt(&mut self, id: StmtId) {
        let arena = self.arena;
        match &arena.stmt(id).kind {
            StmtKind::Block { stmts } => {
                self.stack.push(HashMap::new());
                self.hoist(stmts);
                for stmt in stmts {
                    self.stmt(*stmt);
                }
                self.stack.pop();
            }
            StmtKind::Expr { expression } => self.expr(*expression),
            StmtKind::Decl(decl) => self.decl(decl),
            StmtKind::If {
                test,
                consequent,
                alternate,
            } => {
                self.expr(*test);
                self.stmt(*consequent);
                if let Some(alternate) = alternate {
                    self.stmt(*alternate);
                }
            }
            StmtKind::While { test, body } | StmtKind::DoWhile { body, test } => {
                self.expr(*test);
                self.stmt(*body);
            }
            StmtKind::For {
                init,
                test,
                update,
                body,
            } => {
                self.stack.push(HashMap::new());
                if let Some(init) = init {
                    match init.as_ref() {
                        vn_core::ast::stmt::ForInit::Var { kind, declarators } => {
                            self.declarators(*kind, declarators, false)
                        }
                        vn_core::ast::stmt::ForInit::Expr(expression) => self.expr(*expression),
                    }
                }
                if let Some(test) = test {
                    self.expr(*test);
                }
                if let Some(update) = update {
                    self.expr(*update);
                }
                self.stmt(*body);
                self.stack.pop();
            }
            StmtKind::ForIn {
                kind,
                left,
                right,
                body,
                ..
            } => {
                self.expr(*right);
                self.stack.push(HashMap::new());
                let group = self.groups.len();
                self.groups.push(Group {
                    keyword_start: 0,
                    fixable: false,
                    members: Vec::new(),
                });
                self.declare_pattern(
                    left,
                    &Declare {
                        kind: if *kind == VarKind::Let {
                            Kind::Let
                        } else {
                            Kind::Const
                        },
                        exported: false,
                        initialized: true,
                        group: Some(group),
                        param_of: None,
                        renamable: false,
                        import: None,
                    },
                );
                self.stmt(*body);
                self.stack.pop();
            }
            StmtKind::Return { argument } => {
                if let Some(argument) = argument {
                    self.expr(*argument);
                }
            }
            StmtKind::Throw { argument } => self.expr(*argument),
            StmtKind::Try {
                block,
                catches,
                finally,
            } => {
                self.stmt(*block);
                for clause in catches {
                    self.stack.push(HashMap::new());
                    if let Some(ty) = &clause.type_ann {
                        self.ty(ty);
                    }
                    if let Some(param) = &clause.param {
                        self.declare_pattern(
                            param,
                            &Declare {
                                kind: Kind::Exempt,
                                exported: false,
                                initialized: true,
                                group: None,
                                param_of: None,
                                renamable: false,
                                import: None,
                            },
                        );
                    }
                    self.stmt(clause.body);
                    self.stack.pop();
                }
                if let Some(finally) = finally {
                    self.stmt(*finally);
                }
            }
            StmtKind::Using { declarations, .. } => {
                for declarator in declarations {
                    if let Some(init) = declarator.init {
                        self.expr(init);
                    }
                    if let Some(ty) = &declarator.type_ann {
                        self.ty(ty);
                    }
                    self.declare_pattern(
                        &declarator.id,
                        &Declare {
                            kind: Kind::Exempt,
                            exported: false,
                            initialized: true,
                            group: None,
                            param_of: None,
                            renamable: false,
                            import: None,
                        },
                    );
                }
            }
            StmtKind::Labeled { body, .. } => self.stmt(*body),
            StmtKind::Empty
            | StmtKind::Error
            | StmtKind::Break { .. }
            | StmtKind::Continue { .. }
            | StmtKind::Debugger => {}
        }
    }
    pub(super) fn declarators(
        &mut self,
        kind: VarKind,
        declarators: &[vn_core::ast::stmt::VarDeclarator],
        exported: bool,
    ) {
        let group = self.groups.len();
        self.groups.push(Group {
            keyword_start: 0,
            fixable: false,
            members: Vec::new(),
        });
        for declarator in declarators {
            if let Some(init) = declarator.init {
                self.expr(init);
            }
            if let Some(ty) = &declarator.type_ann {
                self.ty(ty);
            }
            self.declare_pattern(
                &declarator.id,
                &Declare {
                    kind: if kind == VarKind::Let {
                        Kind::Let
                    } else {
                        Kind::Const
                    },
                    exported,
                    initialized: declarator.init.is_some(),
                    group: Some(group),
                    param_of: None,
                    renamable: false,
                    import: None,
                },
            );
        }
    }
}
