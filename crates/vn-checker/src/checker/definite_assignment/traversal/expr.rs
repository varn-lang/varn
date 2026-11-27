use super::super::flow::Flow;
use super::super::patterns::walk_expr_children;
use crate::checker::Checker;
use vn_core::ast::destructure::{destructure_parts, is_destructuring_assign};
use vn_core::ast::{ExprId, ExprKind};
use vn_core::{Atom, Diagnostic, ErrorCode};
use vn_sem::bind::BindResult;

impl<'r> Checker<'r> {
    pub(super) fn da_expr(&mut self, e: ExprId, flow: &mut Flow, bind: &BindResult) {
        let arena = self.ast_arena;
        match &arena.expr(e).kind {
            ExprKind::Identifier { name } => self.da_read(*name, e, flow, bind),

            ExprKind::Assign { op, target, value } => {
                let (op, target, value) = (*op, *target, *value);
                self.da_expr(value, flow, bind);
                if is_destructuring_assign(arena, op, target) {
                    let parts = destructure_parts(arena, target);
                    for read in parts.reads {
                        self.da_expr(read, flow, bind);
                    }
                    for leaf in parts.leaves {
                        if let ExprKind::Identifier { name } = &arena.expr(leaf).kind {
                            flow.assign(name);
                        } else {
                            self.da_expr(leaf, flow, bind);
                        }
                    }
                } else if let ExprKind::Identifier { name } = &arena.expr(target).kind {
                    if flow.undeclared.contains(name) {
                        self.da_read(*name, target, flow, bind);
                    }
                    flow.assign(name);
                } else {
                    self.da_expr(target, flow, bind);
                }
            }

            ExprKind::Member {
                object,
                property,
                computed,
                ..
            } => {
                self.da_expr(*object, flow, bind);
                if *computed {
                    self.da_expr(*property, flow, bind);
                }
            }

            ExprKind::Call { callee, .. } => {
                if let ExprKind::Identifier { name } = &arena.expr(*callee).kind {
                    self.da_call(*name, e, flow, bind);
                }
                walk_expr_children(e, arena, &mut |c| self.da_expr(c, flow, bind))
            }

            ExprKind::Function { .. } | ExprKind::Arrow { .. } | ExprKind::ClassExpr { .. } => {}

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
            | ExprKind::Missing
            | ExprKind::This
            | ExprKind::ImportMeta(_)
            | ExprKind::Super
            | ExprKind::Array { .. }
            | ExprKind::Object { .. }
            | ExprKind::Tuple { .. }
            | ExprKind::Unary { .. }
            | ExprKind::Update { .. }
            | ExprKind::Binary { .. }
            | ExprKind::Logical { .. }
            | ExprKind::Conditional { .. }
            | ExprKind::New { .. }
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
            | ExprKind::Match { .. }
            | ExprKind::Is { .. }
            | ExprKind::With { .. }
            | ExprKind::MetaAccess { .. } => {
                walk_expr_children(e, arena, &mut |c| self.da_expr(c, flow, bind))
            }
        }
    }

    pub(super) fn da_read(&mut self, name: Atom, at: ExprId, flow: &mut Flow, bind: &BindResult) {
        let range = self.ast_arena.expr(at).range;
        let text = bind.interner.resolve(name);
        if vn_core::intrinsics::is_regex_literal_binding(text) {
            return;
        }
        if flow.undeclared.remove(&name) {
            flow.assigned.insert(name);
            self.emit(
                Diagnostic::error(
                    ErrorCode::UseBeforeDeclaration,
                    format!("'{text}' is used before its declaration"),
                )
                .with_range(range),
            );
        } else if flow.pending.contains(&name) && !flow.assigned.contains(&name) {
            flow.assigned.insert(name);
            self.emit(
                Diagnostic::error(
                    ErrorCode::UseBeforeAssignment,
                    format!("'{text}' is used before it is assigned a value"),
                )
                .with_range(range),
            );
        }
    }

    pub(super) fn da_call(&mut self, callee: Atom, at: ExprId, flow: &Flow, bind: &BindResult) {
        let Some(reads) = flow.callee_reads(&callee) else {
            return;
        };
        let mut unready: Vec<Atom> = reads.into_iter().filter(|n| flow.unready(n)).collect();
        unready.sort_by_key(|n| bind.interner.resolve(*n).to_string());
        if let Some(first) = unready.first() {
            let (callee, first) = (bind.interner.resolve(callee), bind.interner.resolve(*first));
            self.emit(
                Diagnostic::error(
                    ErrorCode::UseBeforeDeclaration,
                    format!("calling '{callee}' reads '{first}' before it is declared"),
                )
                .with_range(self.ast_arena.expr(at).range),
            );
        }
    }
}
