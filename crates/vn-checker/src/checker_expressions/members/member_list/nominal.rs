use super::declared::list_declared;
use std::sync::Arc;

use vn_sem::bind::BindResult;
use vn_sem::types::{CheckerTyTable, Type};

#[allow(clippy::too_many_arguments)]
pub(super) fn list_nominal(
    resolver: &dyn vn_sem::resolver::ImportResolver,
    bind: &BindResult,
    table: &mut CheckerTyTable,
    cn: Arc<str>,
    origin: Option<Arc<str>>,
    args: &[Type],
    results: &mut Vec<vn_sem::semantic_info::ResolvedMemberSummary>,
    seen: &mut rustc_hash::FxHashSet<Arc<str>>,
) {
    list_declared(
        resolver,
        bind,
        table,
        cn.clone(),
        origin.clone(),
        args,
        results,
        seen,
    );

    let mut visited = vec![cn.clone()];
    let mut current = parent_of(resolver, bind, &cn, origin.as_ref());
    while let Some(parent) = current {
        if visited.contains(&parent.name) {
            break;
        }
        visited.push(parent.name.clone());
        list_declared(
            resolver,
            bind,
            table,
            parent.name.clone(),
            parent.origin.clone(),
            &[],
            results,
            seen,
        );
        current = parent_of(resolver, bind, &parent.name, parent.origin.as_ref());
    }
}

pub(super) fn parent_of(
    resolver: &dyn vn_sem::resolver::ImportResolver,
    bind: &BindResult,
    class: &Arc<str>,
    origin: Option<&Arc<str>>,
) -> Option<vn_sem::bind::ClassParent> {
    if let Some(parent) = bind.get_class_parent(class) {
        return Some(parent.clone());
    }
    let origin_modules: Vec<String> = origin.iter().map(|s| s.to_string()).collect();
    if let Some(ext_bind) = resolver.find_bind_for_type(class, &origin_modules)
        && let Some(parent) = ext_bind.get_class_parent(class)
    {
        return Some(vn_sem::bind::ClassParent {
            name: parent.name.clone(),
            origin: parent.origin.clone().or_else(|| origin.cloned()),
            args: Vec::new(),
        });
    }
    let core = bind.core.as_ref()?;
    let parent = core.class_parents.get(class.as_ref())?;
    Some(vn_sem::bind::ClassParent {
        name: parent.name.clone(),
        origin: parent.origin.clone(),
        args: Vec::new(),
    })
}
