use super::super::recorder::Recorder;
use super::super::Checker;
use vn_core::ast::{ExprId, ExprKind};
use vn_core::TypeKind;
use vn_sem::bind::BindResult;
use vn_sem::types::Type;

use super::MAX_DEPTH;
impl Checker<'_> {
    pub(super) fn variant_fits(
        &mut self,
        rec: &mut Recorder,
        target: &Type,
        expr: ExprId,
        bind: &BindResult,
        depth: u8,
    ) -> bool {
        if depth > MAX_DEPTH {
            return false;
        }
        let arena = self.ast_arena;
        match &arena.expr(expr).kind {
            ExprKind::Paren { expression } => {
                self.variant_fits(rec, target, *expression, bind, depth + 1)
            }
            ExprKind::Conditional {
                consequent,
                alternate,
                ..
            } => {
                self.variant_fits(rec, target, *consequent, bind, depth + 1)
                    && self.variant_fits(rec, target, *alternate, bind, depth + 1)
            }
            ExprKind::Member { .. } | ExprKind::Call { .. } => {
                if !self.is_variant_construction(expr, bind) {
                    return false;
                }
                let given = self.infer_type(rec, expr, bind);
                self.unresolved_arguments_fit(target, &given)
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
            | ExprKind::ImportMeta(_)
            | ExprKind::Super
            | ExprKind::Array { .. }
            | ExprKind::Object { .. }
            | ExprKind::Tuple { .. }
            | ExprKind::Unary { .. }
            | ExprKind::Update { .. }
            | ExprKind::Binary { .. }
            | ExprKind::Logical { .. }
            | ExprKind::Assign { .. }
            | ExprKind::New { .. }
            | ExprKind::Function { .. }
            | ExprKind::Arrow { .. }
            | ExprKind::Sequence { .. }
            | ExprKind::Await { .. }
            | ExprKind::Yield { .. }
            | ExprKind::Spread { .. }
            | ExprKind::Pipeline { .. }
            | ExprKind::Range { .. }
            | ExprKind::NonNull { .. }
            | ExprKind::Try { .. }
            | ExprKind::As { .. }
            | ExprKind::Satisfies { .. }
            | ExprKind::ClassExpr { .. }
            | ExprKind::Match { .. }
            | ExprKind::Is { .. }
            | ExprKind::With { .. }
            | ExprKind::MetaAccess { .. } => false,
        }
    }

    pub(super) fn is_variant_construction(&self, expr: ExprId, bind: &BindResult) -> bool {
        let arena = self.ast_arena;
        let member = if let ExprKind::Call { callee, .. } = &arena.expr(expr).kind {
            *callee
        } else {
            expr
        };
        let ExprKind::Member { object, .. } = &arena.expr(member).kind else {
            return false;
        };
        let ExprKind::Identifier { name } = &arena.expr(*object).kind else {
            return false;
        };
        bind.scopes
            .get(self.current_scope)
            .resolve(*name, &bind.scopes)
            .is_some_and(|sid| bind.arena.get(sid).kind == vn_sem::symbol::SymbolKind::Enum)
    }

    pub(super) fn unresolved_arguments_fit(&mut self, target: &Type, given: &Type) -> bool {
        let target = target.non_nullified(&mut *std::sync::Arc::make_mut(&mut self.ty_table));
        let (
            TypeKind::Generic(target_name, target_args, target_origin),
            TypeKind::Generic(given_name, given_args, given_origin),
        ) = (self.ty_table.get(target.0), self.ty_table.get(given.0))
        else {
            return false;
        };
        let target_args = self.ty_table.get_list(target_args).to_vec();
        let given_args = self.ty_table.get_list(given_args).to_vec();
        target_name == given_name
            && target_origin == given_origin
            && target_args.len() == given_args.len()
            && target_args
                .iter()
                .zip(&given_args)
                .all(|(wanted, got)| wanted == got || *got == Type::Never.0)
    }
}
