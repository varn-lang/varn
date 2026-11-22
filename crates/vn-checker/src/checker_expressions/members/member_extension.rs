use std::sync::Arc;

use vn_sem::bind::BindResult;
use vn_sem::types::{CheckerTyTable, Type};

pub(super) fn collect_extension_members(
    results: &mut Vec<vn_sem::semantic_info::ResolvedMemberSummary>,
    seen: &mut rustc_hash::FxHashSet<Arc<str>>,
    ty: &Type,
    bind: &BindResult,
    table: &mut CheckerTyTable,
) {
    let scope = bind.scopes.get(bind.global_scope);

    let strip_this = |ft: &vn_sem::types::FunctionType| {
        let mut params = ft.params.clone();
        if params.first().and_then(|p| p.name.as_deref()) == Some(vn_core::TokenKind::This.as_str())
        {
            params.remove(0);
        }
        vn_sem::types::FunctionType {
            params,
            return_type: ft.return_type,
            is_arrow: ft.is_arrow,
            type_params: ft.type_params.clone(),
            type_param_defaults: ft.type_param_defaults.clone(),
        }
    };

    let push = |name: &Arc<str>,
                mangled: &Arc<str>,
                kind: vn_sem::semantic_info::ResolvedMemberKind,
                as_return: bool,
                results: &mut Vec<vn_sem::semantic_info::ResolvedMemberSummary>,
                seen: &mut rustc_hash::FxHashSet<Arc<str>>,
                table: &mut CheckerTyTable| {
        let Some(sid) = bind
            .interner
            .get(mangled.as_ref())
            .and_then(|atom| scope.resolve(atom, &bind.scopes))
        else {
            return;
        };
        let sym = bind.arena.get(sid);
        let Some(sym_ty) = &sym.ty else {
            return;
        };
        let vn_core::TypeKind::Fn(fid) = table.get(sym_ty.0) else {
            return;
        };
        let ft = table.get_function(fid).clone();
        let member_ty = if as_return {
            Type::resolved(ft.return_type)
        } else {
            Type::fn_(strip_this(&ft), table)
        };
        if seen.insert(name.clone()) {
            results.push(vn_sem::semantic_info::ResolvedMemberSummary {
                name: name.clone(),
                ty: member_ty,
                kind,
                is_static: false,
                optional: false,
                readonly: false,
                def_line: (sym.line > 0).then_some(sym.line),
                def_col: sym.col,
                is_async: sym.is_async,
                is_generator: sym.is_generator,
                doc: sym
                    .doc
                    .and_then(|d| bind.interner.try_resolve(d))
                    .map(std::sync::Arc::from),
                visibility: None,
                owner: None,
            });
        }
    };

    use vn_core::extension_names::ExtensionKind;
    use vn_sem::semantic_info::ResolvedMemberKind as K;
    let name_of = |atom: vn_core::Atom| -> Option<Arc<str>> {
        Some(Arc::from(super::member_atom::resolve_atom_text(bind, atom)))
    };
    for (kind, member_kind, as_return) in [
        (ExtensionKind::Method, K::ExtensionMethod, false),
        (ExtensionKind::Getter, K::ExtensionProperty, true),
        (ExtensionKind::Setter, K::ExtensionProperty, true),
    ] {
        for (name, mangled) in bind.extensions.members_for(kind, *ty, table, &name_of) {
            push(
                &name,
                &mangled,
                member_kind,
                as_return,
                results,
                seen,
                table,
            );
        }
    }
}
