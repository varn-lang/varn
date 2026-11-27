use super::nominal::list_nominal;
use std::sync::Arc;

use vn_sem::bind::BindResult;
use vn_sem::types::CheckerTyTable;

pub(super) fn list_primitive(
    resolver: &dyn vn_sem::resolver::ImportResolver,
    bind: &BindResult,
    table: &mut CheckerTyTable,
    primitive: vn_core::LangPrimitive,
    results: &mut Vec<vn_sem::semantic_info::ResolvedMemberSummary>,
    seen: &mut rustc_hash::FxHashSet<Arc<str>>,
) {
    if let Some(module) = primitive.core_module() {
        let name = Arc::from(primitive.name());
        list_nominal(
            resolver,
            bind,
            table,
            name,
            Some(Arc::from(module)),
            &[],
            results,
            seen,
        );
    }
}
