use crate::checker::recorder::Recorder;
use crate::checker::Checker;
use vn_core::ast::{ExprId, ExprKind};
use vn_core::source::SourceRange;
use vn_sem::bind::BindResult;

use super::super::members::resolve_extension;

impl<'r> Checker<'r> {
    pub(super) fn record_extension_call(
        &mut self,
        rec: &mut Recorder,
        callee: ExprId,
        range: &SourceRange,
        bind: &BindResult,
    ) {
        let arena = self.ast_arena;
        let ExprKind::Member {
            object,
            property,
            computed: false,
            ..
        } = &arena.expr(callee).kind
        else {
            return;
        };
        let (object, property) = (*object, *property);
        let ExprKind::Identifier { name: method_name } = &arena.expr(property).kind else {
            return;
        };
        let obj_ty_raw = self.infer_type(rec, object, bind);
        let obj_ty = obj_ty_raw.non_nullified(&mut *std::sync::Arc::make_mut(&mut self.ty_table));
        if let Some(mangled) = resolve_extension(
            self,
            vn_core::extension_names::ExtensionKind::Method,
            &obj_ty,
            bind.interner.resolve(*method_name),
            bind,
        ) && !bind.extensions.forwarded.contains(&mangled)
        {
            rec.desugar
                .extension_calls
                .insert(range.start.offset, mangled);
        }
    }
}
