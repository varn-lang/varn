use std::sync::Arc;

use vn_sem::bind::BindResult;
use vn_sem::types::{CheckerTyTable, Type};

#[allow(clippy::too_many_arguments)]
pub(super) fn list_declared(
    resolver: &dyn vn_sem::resolver::ImportResolver,
    bind: &BindResult,
    table: &mut CheckerTyTable,
    cn: Arc<str>,
    origin: Option<Arc<str>>,
    args: &[Type],
    results: &mut Vec<vn_sem::semantic_info::ResolvedMemberSummary>,
    seen: &mut rustc_hash::FxHashSet<Arc<str>>,
) {
    let mapping = if args.is_empty() {
        rustc_hash::FxHashMap::default()
    } else {
        super::super::member_util::generic_mapping(
            resolver,
            cn.as_ref(),
            args,
            origin.as_ref(),
            bind,
        )
    };

    let map_ty = |t: &Type, table: &mut CheckerTyTable| {
        if mapping.is_empty() {
            *t
        } else {
            t.map_generics(&mapping, table)
        }
    };

    if let Some(entry) = bind.type_members.classes.get(&cn) {
        for m in &entry.members {
            let kind = super::super::member_kind::map_class_member_kind(m.kind);
            let mapped = map_ty(&m.ty, table);
            add_declared(results, seen, &cn, m, mapped, kind);
        }
    }
    if let Some(entry) = bind.type_members.interfaces.get(&cn) {
        for m in entry {
            let kind = super::super::member_kind::map_class_member_kind(m.kind);
            let mapped = map_ty(&m.ty, table);
            add_declared(results, seen, &cn, m, mapped, kind);
        }
    }
    if let Some(entry) = bind.type_members.enums.get(&cn) {
        for m in entry {
            let kind = super::super::member_kind::map_class_member_kind(m.kind);
            let mapped = map_ty(&m.ty, table);
            add_declared(results, seen, &cn, m, mapped, kind);
        }
    }
    if let Some(entry) = bind.type_members.namespaces.get(&cn) {
        for m in entry {
            let kind = super::super::member_kind::map_class_member_kind(m.kind);
            let mapped = map_ty(&m.ty, table);
            add_declared(results, seen, &cn, m, mapped, kind);
        }
    }

    if let Some(b) = &bind.core {
        if let Some(entry) = b.types.classes.get(cn.as_ref()) {
            for m in &entry.members {
                let kind = super::super::member_kind::map_class_member_kind(m.kind);
                let mapped = map_ty(&m.ty, table);
                add_declared(results, seen, &cn, m, mapped, kind);
            }
        }
        if let Some(members) = b.types.flattened.get(cn.as_ref()) {
            for m in members {
                let kind = super::super::member_kind::map_class_member_kind(m.kind);
                let mapped = map_ty(&m.ty, table);
                add_declared(results, seen, &cn, m, mapped, kind);
            }
        }
    }

    if let Some(members) = bind.type_members.flattened.get(&cn) {
        for m in members {
            let kind = super::super::member_kind::map_class_member_kind(m.kind);
            let mapped = map_ty(&m.ty, table);
            add_declared(results, seen, &cn, m, mapped, kind);
        }
    }

    let origin_modules: Vec<String> = origin.iter().map(|s| s.to_string()).collect();
    if let Some(ext_bind) = resolver.find_bind_for_type(&cn, &origin_modules) {
        table.absorb(&ext_bind.ty_table);
        let external = &ext_bind.type_members;
        let declared = external
            .classes
            .get(&cn)
            .map(|entry| entry.members.as_slice())
            .into_iter()
            .chain(external.interfaces.get(&cn).map(Vec::as_slice))
            .chain(external.enums.get(&cn).map(Vec::as_slice))
            .chain(external.namespaces.get(&cn).map(Vec::as_slice))
            .chain(external.flattened.get(&cn).map(Vec::as_slice));
        for members in declared {
            for m in members {
                let kind = super::super::member_kind::map_class_member_kind(m.kind);
                let mapped = map_ty(&m.ty, table);
                add_declared(results, seen, &cn, m, mapped, kind);
            }
        }
    }
}

pub(super) fn add_declared(
    results: &mut Vec<vn_sem::semantic_info::ResolvedMemberSummary>,
    seen: &mut rustc_hash::FxHashSet<Arc<str>>,
    owner: &Arc<str>,
    m: &vn_sem::types::ClassMemberInfo,
    ty: Type,
    kind: vn_sem::semantic_info::ResolvedMemberKind,
) {
    if seen.insert(m.name.clone()) {
        results.push(vn_sem::semantic_info::ResolvedMemberSummary {
            name: m.name.clone(),
            ty,
            kind,
            is_static: m.is_static,
            optional: m.is_optional,
            readonly: m.is_readonly,
            def_line: (m.line > 0).then_some(m.line),
            def_col: m.col,
            is_async: m.is_async,
            is_generator: m.is_generator,
            doc: m.doc.clone(),
            visibility: m.visibility,
            owner: Some(owner.clone()),
        });
    }
}
