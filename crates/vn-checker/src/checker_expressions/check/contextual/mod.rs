use crate::checker::recorder::Recorder;
use crate::checker::Checker;
use vn_core::ast::{ExprId, Param, PropKey};
use vn_core::TypeKind;
use vn_sem::bind::BindResult;
use vn_sem::infer::pattern_lead_name;
use vn_sem::types::{FunctionType, Type};

mod array;
mod object;

impl<'r> Checker<'r> {
    pub(super) fn apply_contextual_arrow_params(
        &mut self,
        rec: &mut Recorder,
        params: &[Param],
        expected_fn: &FunctionType,
        bind: &BindResult,
    ) {
        for (ap, ep) in params.iter().zip(expected_fn.params.iter()) {
            let has_ann = ap.type_ann.is_some();
            let ep_ty = Type::resolved(ep.ty);
            if has_ann || ep_ty.is_unresolved() {
                continue;
            }
            if !matches!(ap.pattern, vn_core::ast::Pattern::Identifier { .. }) {
                self.check_pattern(rec, &ap.pattern, &ep_ty, bind);
                continue;
            }

            let name = pattern_lead_name(&ap.pattern, &bind.interner);
            let scope = bind.scopes.get(self.current_scope);
            if let Some(sym_id) = bind
                .interner
                .get(name)
                .and_then(|atom| scope.resolve(atom, &bind.scopes))
            {
                rec.symbol_types.insert(sym_id, ep_ty);
                self.mark_infer_env_dirty();
            }
        }
    }

    pub(crate) fn fallback_expected(
        &mut self,
        rec: &mut Recorder,
        left: ExprId,
        right: ExprId,
        bind: &BindResult,
    ) -> Option<Type> {
        if let Some(given) = self.expected_for(right) {
            return Some(given);
        }
        let present = self.infer_type(rec, left, bind);
        let present = present.non_nullified(&mut *std::sync::Arc::make_mut(&mut self.ty_table));
        (!present.is_unresolved()).then_some(present)
    }

    pub(crate) fn expected_for(&self, expr: ExprId) -> Option<Type> {
        self.expected_type.filter(|_| {
            self.expected_owner == Some(expr.index())
                || self.expected_forwarded.contains(&expr.index())
        })
    }

    pub(crate) fn forwarded_operands(&self, expr: ExprId) -> Vec<u32> {
        use vn_core::ast::ExprKind;
        let mut forwarded = Vec::new();
        let mut pending = vec![expr];
        while let Some(current) = pending.pop() {
            let kind = &self.ast_arena.expr(current).kind;
            let operands: Vec<ExprId> = if let ExprKind::Paren { expression } = kind {
                vec![*expression]
            } else if let ExprKind::Logical { left, right, .. } = kind {
                vec![*left, *right]
            } else if let ExprKind::Conditional {
                consequent,
                alternate,
                ..
            } = kind
            {
                vec![*consequent, *alternate]
            } else if let ExprKind::Sequence { expressions } = kind {
                expressions.last().copied().into_iter().collect()
            } else {
                Vec::new()
            };
            for operand in operands {
                forwarded.push(operand.index());
                pending.push(operand);
            }
        }
        forwarded
    }

    pub(super) fn expected_fn_type(&self) -> Option<FunctionType> {
        self.expected_type.and_then(|t| {
            if let TypeKind::Fn(fid) = self.ty_table.get(t.0) {
                Some(self.ty_table.get_function(fid).clone())
            } else {
                None
            }
        })
    }

    pub(super) fn expected_return_from_fn_type(&self) -> Option<Type> {
        self.expected_fn_type()
            .map(|ft| Type::resolved(ft.return_type))
            .filter(|t| !t.is_unresolved())
    }
}

pub(super) fn prop_key_str(key: &PropKey) -> Option<&str> {
    match key {
        PropKey::Identifier(s) | PropKey::Str(s) => Some(s.as_str()),
        PropKey::Int(_) | PropKey::Computed(_) => None,
    }
}
