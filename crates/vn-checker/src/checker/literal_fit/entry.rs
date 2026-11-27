use super::super::recorder::Recorder;
use super::super::Checker;
use vn_core::ast::{ExprId, ExprKind};
use vn_sem::bind::BindResult;
use vn_sem::types::Type;

use super::MAX_DEPTH;
impl Checker<'_> {
    pub(crate) fn value_fits(
        &mut self,
        rec: &mut Recorder,
        target: &Type,
        given: &Type,
        expr: Option<ExprId>,
        bind: Option<&BindResult>,
    ) -> bool {
        if self.value_assignable_to(target, given, expr, bind) {
            if let Some(expr) = expr {
                self.record_literal_adoption(rec, target, expr);
            }
            return true;
        }
        match (expr, bind) {
            (Some(expr), Some(bind)) => {
                self.literal_fits(rec, target, expr, bind, 0)
                    || self.variant_fits(rec, target, expr, bind, 0)
            }
            (Some(_), None) | (None, _) => false,
        }
    }

    pub(crate) fn record_literal_adoption(
        &mut self,
        rec: &mut Recorder,
        target: &Type,
        expr: ExprId,
    ) {
        use vn_sem::types::numeric_literal::{
            const_float_literal, const_int_literal, float_literal_adoption, int_literal_adoption,
        };
        let mut nodes = Vec::new();
        let adopted = if let Some(value) = const_int_literal(self.ast_arena, expr, &mut nodes) {
            int_literal_adoption(target, value, &self.ty_table)
        } else {
            nodes.clear();
            const_float_literal(self.ast_arena, expr, &mut nodes)
                .and_then(|value| float_literal_adoption(target, value, &self.ty_table))
        };
        let Some(adopted) = adopted else {
            return;
        };
        for node in nodes {
            rec.adopt_literal(node.index(), adopted);
        }
    }

    pub(super) fn literal_fits(
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
        let literal = match &arena.expr(expr).kind {
            ExprKind::Paren { expression } => {
                return self.literal_fits(rec, target, *expression, bind, depth + 1)
            }
            ExprKind::Array { .. } | ExprKind::Object { .. } | ExprKind::Tuple { .. } => expr,
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
            | ExprKind::MetaAccess { .. } => return false,
        };
        let shapes = self.shapes_of(target, bind);
        shapes
            .into_iter()
            .any(|shape| self.literal_fits_shape(rec, &shape, literal, bind, depth))
    }
}
