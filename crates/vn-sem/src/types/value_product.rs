use crate::bind::{BindResult, BindView};
use crate::resolver::ImportResolver;
use crate::types::{CheckerTyTable, Type, TypeContext};
use std::sync::Arc;
use vn_core::TypeKind;

fn name_of(bind: &BindResult, table: &CheckerTyTable, atom: vn_core::Atom) -> Arc<str> {
    if let Some(text) = bind.interner.try_resolve(atom) {
        return Arc::from(text);
    }
    if let Some(text) = table.name(atom) {
        return Arc::from(text);
    }
    Arc::from(format!("<stale:{atom:?}>"))
}

pub fn is_value_product(
    ty: &Type,
    table: &mut CheckerTyTable,
    bind: &BindResult,
    resolver: &dyn ImportResolver,
) -> bool {
    let ty = ty.non_nullified(table);
    match table.get(ty.0) {
        TypeKind::Record(_) => true,
        TypeKind::Named(name, origin) | TypeKind::Generic(name, _, origin) => {
            let name = name_of(bind, table, name);
            let origin = origin.map(|o| name_of(bind, table, o));
            BindView::new(bind, resolver)
                .class_entry(&name, origin.as_deref())
                .is_some_and(|entry| entry.is_struct)
        }
        TypeKind::Primitive(_)
        | TypeKind::Literal(_)
        | TypeKind::This
        | TypeKind::Union(_)
        | TypeKind::Intersection(_)
        | TypeKind::Tuple(_)
        | TypeKind::Array(_)
        | TypeKind::Fn(_)
        | TypeKind::Interface(_)
        | TypeKind::EnumVariant { .. }
        | TypeKind::TypePredicate { .. } => false,
    }
}

pub fn is_reference_object(
    ty: &Type,
    table: &mut CheckerTyTable,
    bind: &BindResult,
    resolver: &dyn ImportResolver,
) -> bool {
    let ty = ty.non_nullified(table);
    match table.get(ty.0) {
        TypeKind::Array(_) => true,
        TypeKind::Named(name, origin) | TypeKind::Generic(name, _, origin) => {
            let name = name_of(bind, table, name);
            let origin = origin.map(|o| name_of(bind, table, o));
            BindView::new(bind, resolver)
                .class_entry(&name, origin.as_deref())
                .is_some_and(|entry| !entry.is_struct)
        }
        TypeKind::Record(_)
        | TypeKind::Primitive(_)
        | TypeKind::Literal(_)
        | TypeKind::This
        | TypeKind::Union(_)
        | TypeKind::Intersection(_)
        | TypeKind::Tuple(_)
        | TypeKind::Fn(_)
        | TypeKind::Interface(_)
        | TypeKind::EnumVariant { .. }
        | TypeKind::TypePredicate { .. } => false,
    }
}
