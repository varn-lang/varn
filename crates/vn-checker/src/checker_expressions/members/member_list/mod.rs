use std::sync::Arc;

use vn_core::TypeKind;
use vn_sem::bind::BindResult;
use vn_sem::types::{CheckerTyTable, ObjectTypeMember, Type};

mod declared;
mod nominal;
mod primitive;

use nominal::list_nominal;
use primitive::list_primitive;

pub fn get_members_of_type(
    resolver: &dyn vn_sem::resolver::ImportResolver,
    ty: &Type,
    bind: &BindResult,
    table: &mut CheckerTyTable,
) -> Vec<vn_sem::semantic_info::ResolvedMemberSummary> {
    let mut results: Vec<vn_sem::semantic_info::ResolvedMemberSummary> = Vec::new();
    let mut seen = rustc_hash::FxHashSet::default();

    let add_member = |results: &mut Vec<vn_sem::semantic_info::ResolvedMemberSummary>,
                      seen: &mut rustc_hash::FxHashSet<Arc<str>>,
                      name: Arc<str>,
                      ty: Type,
                      kind: vn_sem::semantic_info::ResolvedMemberKind,
                      is_static: bool,
                      optional: bool,
                      readonly: bool| {
        if seen.insert(name.clone()) {
            results.push(vn_sem::semantic_info::ResolvedMemberSummary {
                name,
                ty,
                kind,
                is_static,
                optional,
                readonly,
                def_line: None,
                def_col: 0,
                is_async: false,
                is_generator: false,
                doc: None,
                visibility: None,
                owner: None,
            });
        }
    };

    let view = vn_sem::bind::BindView::new(bind, resolver);
    let resolved = vn_sem::resolve::expand_alias(*ty, Some(&view), table);
    let ty = &resolved;
    let ty_kind = table.get(ty.0);
    match ty_kind {
        TypeKind::Record(mid) | TypeKind::Interface(mid) => {
            for m in table.get_object_members(mid).to_vec() {
                match m {
                    ObjectTypeMember::Property {
                        name,
                        ty,
                        optional,
                        readonly,
                    } => {
                        add_member(
                            &mut results,
                            &mut seen,
                            name.clone(),
                            Type::resolved(ty),
                            vn_sem::semantic_info::ResolvedMemberKind::Property,
                            false,
                            optional,
                            readonly,
                        );
                    }
                    ObjectTypeMember::Method {
                        name,
                        params,
                        return_type,
                        is_arrow,
                        type_params,
                        ..
                    } => {
                        let fn_ty = Type::fn_(
                            vn_sem::types::FunctionType {
                                params: params.clone(),
                                return_type,
                                is_arrow,
                                type_params: type_params.clone(),
                                type_param_defaults: Vec::new(),
                            },
                            table,
                        );
                        add_member(
                            &mut results,
                            &mut seen,
                            name.clone(),
                            fn_ty,
                            vn_sem::semantic_info::ResolvedMemberKind::Method,
                            false,
                            false,
                            true,
                        );
                    }
                    ObjectTypeMember::Callable { .. } => {}
                }
            }
        }
        TypeKind::Tuple(list) => {
            let elems = table.get_list(list).to_vec();
            for (idx, elem) in elems.iter().enumerate() {
                add_member(
                    &mut results,
                    &mut seen,
                    Arc::from(idx.to_string()),
                    Type::resolved(*elem),
                    vn_sem::semantic_info::ResolvedMemberKind::Property,
                    false,
                    false,
                    false,
                );
            }
            add_member(
                &mut results,
                &mut seen,
                Arc::from(vn_core::MemberKey::Length.name()),
                Type::Int,
                vn_sem::semantic_info::ResolvedMemberKind::Property,
                false,
                false,
                true,
            );
        }
        TypeKind::Array(inner) => list_nominal(
            resolver,
            bind,
            table,
            Arc::from(vn_core::BuiltinType::Array.name()),
            Some(Arc::from(vn_core::BuiltinType::Array.core_module())),
            &[Type::resolved(inner)],
            &mut results,
            &mut seen,
        ),
        TypeKind::Named(cn_atom, origin_atom) | TypeKind::Generic(cn_atom, _, origin_atom) => {
            let cn: Arc<str> = Arc::from(super::member_atom::resolve_atom_text(bind, cn_atom));
            let origin: Option<Arc<str>> =
                origin_atom.map(|o| Arc::from(super::member_atom::resolve_atom_text(bind, o)));
            let args: Vec<Type> = if let TypeKind::Generic(_, args_list, _) = ty_kind {
                table
                    .get_list(args_list)
                    .iter()
                    .map(|id| Type::resolved(*id))
                    .collect()
            } else {
                Vec::new()
            };
            let origin_modules: Vec<String> = origin.iter().map(|o| o.to_string()).collect();
            let is_enum = super::is_enum_type(resolver, bind, &cn, &origin_modules);
            list_nominal(
                resolver,
                bind,
                table,
                cn,
                origin,
                &args,
                &mut results,
                &mut seen,
            );
            if is_enum {
                for (key, ty) in [
                    (vn_core::MemberKey::Name, Type::Str),
                    (vn_core::MemberKey::VariantName, Type::Str),
                    (vn_core::MemberKey::Tag, Type::Int),
                    (vn_core::MemberKey::RawValue, Type::Int),
                ] {
                    add_member(
                        &mut results,
                        &mut seen,
                        Arc::from(key.name()),
                        ty,
                        vn_sem::semantic_info::ResolvedMemberKind::Property,
                        false,
                        false,
                        true,
                    );
                }
            }
        }
        TypeKind::Primitive(vn_core::LangPrimitive::Str) => {
            add_member(
                &mut results,
                &mut seen,
                Arc::from(vn_core::MemberKey::Length.name()),
                Type::Int,
                vn_sem::semantic_info::ResolvedMemberKind::Property,
                false,
                false,
                true,
            );
            list_primitive(
                resolver,
                bind,
                table,
                vn_core::LangPrimitive::Str,
                &mut results,
                &mut seen,
            );
        }
        TypeKind::Primitive(p) => {
            list_primitive(resolver, bind, table, p, &mut results, &mut seen);
        }
        TypeKind::Literal(l) => {
            list_primitive(resolver, bind, table, l.base(), &mut results, &mut seen);
        }
        TypeKind::Union(list) | TypeKind::Intersection(list) => {
            let variants: Vec<Type> = table
                .get_list(list)
                .to_vec()
                .iter()
                .map(|id| Type::resolved(*id))
                .collect();
            let mut per_variant = Vec::with_capacity(variants.len());
            for v in &variants {
                if matches!(
                    table.get(v.0),
                    TypeKind::Primitive(
                        vn_core::LangPrimitive::Dynamic | vn_core::LangPrimitive::Error
                    )
                ) {
                    continue;
                }
                per_variant.push(get_members_of_type(resolver, v, bind, table));
            }
            if let Some((first, rest)) = per_variant.split_first() {
                for m in first {
                    if rest.iter().all(|o| o.iter().any(|x| x.name == m.name))
                        && seen.insert(m.name.clone())
                    {
                        results.push(m.clone());
                    }
                }
            }
        }
        TypeKind::This
        | TypeKind::Fn(_)
        | TypeKind::EnumVariant { .. }
        | TypeKind::TypePredicate { .. } => {}
    }

    super::member_extension::collect_extension_members(
        &mut results,
        &mut seen,
        ty,
        resolver,
        bind,
        table,
    );
    results
}
