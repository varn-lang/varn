use super::member_auth::MemberAuth;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use vn_core::TypeKind;
use vn_sem::bind::BindResult;
use vn_sem::types::Type;

pub(crate) fn class_type_params(
    resolver: &dyn vn_sem::resolver::ImportResolver,
    name: &str,
    origin: Option<&Arc<str>>,
    bind: &BindResult,
) -> Vec<Arc<str>> {
    let local = params_in(name, bind);
    let declared_here = origin.is_none_or(|o| o.as_ref() == bind.source_file.as_ref())
        && (bind.type_members.classes.contains_key(name)
            || bind.type_members.interfaces.contains_key(name));
    if declared_here || !local.is_empty() {
        return local;
    }
    if let Some(params) = bind
        .core
        .as_ref()
        .and_then(|b| b.class_type_params.get(name))
    {
        return params.clone();
    }
    let origins: Vec<String> = origin.iter().map(|s| s.to_string()).collect();
    if let Some(b) = resolver.find_bind_for_type(name, &origins) {
        return params_in(name, &b);
    }
    if origin.is_none()
        && let Some(found) = resolver.find_stdlib_bind(&mut |b| !params_in(name, b).is_empty())
    {
        return params_in(name, &found);
    }
    Vec::new()
}

fn params_in(name: &str, bind: &BindResult) -> Vec<Arc<str>> {
    bind.interner
        .get(name)
        .and_then(|atom| {
            bind.scopes
                .get(bind.global_scope)
                .resolve(atom, &bind.scopes)
        })
        .map(|sid| {
            bind.arena
                .get(sid)
                .type_params
                .iter()
                .map(|a| Arc::from(bind.interner.resolve(*a)))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn generic_mapping(
    resolver: &dyn vn_sem::resolver::ImportResolver,
    name: &str,
    args: &[Type],
    origin: Option<&Arc<str>>,
    bind: &BindResult,
) -> FxHashMap<vn_core::Atom, Type> {
    if args.is_empty() {
        return FxHashMap::default();
    }
    class_type_params(resolver, name, origin, bind)
        .into_iter()
        .map(|p| vn_core::Atom::of(&p))
        .zip(args.iter().cloned())
        .collect()
}

fn resolve_extension_symbol_type(bind: &BindResult, mangled: &Arc<str>) -> Option<Type> {
    let scope = bind.scopes.get(bind.global_scope);
    let atom = bind.interner.get(mangled.as_ref())?;
    let sid = scope.resolve(atom, &bind.scopes)?;
    bind.arena.get(sid).ty
}

pub(super) fn extension_method_auth(bind: &BindResult, mangled: &Arc<str>) -> MemberAuth {
    let scope = bind.scopes.get(bind.global_scope);
    let sid = bind
        .interner
        .get(mangled.as_ref())
        .and_then(|atom| scope.resolve(atom, &bind.scopes));
    MemberAuth::of(bind, sid)
}

pub(super) fn extension_signature(
    checker: &mut crate::checker::Checker,
    bind: &BindResult,
    mangled: &Arc<str>,
    receiver: &Type,
) -> Option<vn_sem::types::FunctionType> {
    let sym_ty = resolve_extension_symbol_type(bind, mangled)?;
    let TypeKind::Fn(fid) = checker.ty_table.get(sym_ty.0) else {
        return None;
    };
    let ft = checker.ty_table.get_function(fid).clone();
    let (declared_receiver, params) = match ft.params.split_first() {
        Some((first, rest)) if first.name.as_deref() == Some(vn_core::TokenKind::This.as_str()) => {
            (Some(Type::resolved(first.ty)), rest.to_vec())
        }
        Some(_) | None => (None, ft.params.clone()),
    };
    let mut mapping: FxHashMap<Arc<str>, Type> = FxHashMap::default();
    if let Some(declared) = declared_receiver {
        crate::checker_type_inferences::collect_type_inferences(
            &declared,
            receiver,
            &ft.type_params,
            &mut mapping,
            &mut *std::sync::Arc::make_mut(&mut checker.ty_table),
            &bind.interner,
        );
    }
    let params = params
        .into_iter()
        .map(|p| vn_sem::types::FunctionParam {
            ty: crate::generic_substitution::map_generics_cached(
                checker,
                &Type::resolved(p.ty),
                &mapping,
            )
            .0,
            ..p
        })
        .collect();
    let return_type = crate::generic_substitution::map_generics_cached(
        checker,
        &Type::resolved(ft.return_type),
        &mapping,
    )
    .0;
    let (type_params, type_param_defaults) = ft
        .type_params
        .iter()
        .zip(
            ft.type_param_defaults
                .iter()
                .copied()
                .chain(std::iter::repeat(None)),
        )
        .filter(|(name, _)| !mapping.contains_key(*name))
        .map(|(name, default)| (name.clone(), default))
        .unzip();
    Some(vn_sem::types::FunctionType {
        params,
        return_type,
        is_arrow: ft.is_arrow,
        type_params,
        type_param_defaults,
    })
}

pub(super) fn extension_method_type(
    checker: &mut crate::checker::Checker,
    bind: &BindResult,
    mangled: &Arc<str>,
    receiver: &Type,
) -> Option<Type> {
    let signature = extension_signature(checker, bind, mangled, receiver)?;
    Some(Type::fn_(
        signature,
        &mut *std::sync::Arc::make_mut(&mut checker.ty_table),
    ))
}

pub(super) fn extension_getter_type(
    checker: &mut crate::checker::Checker,
    bind: &BindResult,
    mangled: &Arc<str>,
    receiver: &Type,
) -> Option<Type> {
    extension_signature(checker, bind, mangled, receiver).map(|s| Type::resolved(s.return_type))
}

pub(super) fn intrinsic_member_info(
    bind: &BindResult,
    type_name: &str,
    key: &str,
) -> Option<(Type, super::member_auth::MemberAuth)> {
    bind.core
        .as_ref()
        .and_then(|b| b.class_members.get(type_name))
        .and_then(|members| {
            members
                .members
                .iter()
                .find(|m| m.name.as_ref() == key)
                .map(|m| (m.ty, super::member_auth::MemberAuth::of_info(m)))
        })
}
