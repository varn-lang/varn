use super::signatures::accessor_signature;
use super::{intern_signature, NameIndex};
use crate::ty::lower_type;
use crate::ty::NameResolver;
use std::sync::Arc;
use vn_core::AtomInterner;
use vn_sem::bind::BindResult;
use vn_sem::types::{CheckerTyTable, ClassMemberKind};
use vn_tir::{EnumInfo, Signature, TyTable, VariantInfo, VtableEntry};

pub(super) fn build_enums(
    bind: &BindResult,
    table: &CheckerTyTable,
    interner: &AtomInterner,
    tt: &mut TyTable,
    names: &NameIndex,
    enum_names: &[Arc<str>],
    signatures: &mut Vec<Signature>,
) -> Vec<EnumInfo> {
    enum_names
        .iter()
        .map(|name| EnumInfo {
            name: name.clone(),
            binding: Some(name.clone()),
            members: enum_members(
                bind.type_members
                    .enums
                    .get(name)
                    .into_iter()
                    .flatten()
                    .map(|m| (m.kind, &m.name, &m.ty, m.offset, m.is_static, m.is_abstract)),
                table,
                interner,
                tt,
                names,
                signatures,
            ),
            variants: bind
                .enum_layout(name)
                .unwrap_or_default()
                .into_iter()
                .map(|variant| VariantInfo {
                    name: variant.name,
                    tag: variant.tag,
                    payload: variant
                        .fields
                        .iter()
                        .map(|(_, ty)| lower_type(ty, table, interner, tt, names))
                        .collect(),
                    field_names: variant
                        .fields
                        .iter()
                        .map(|(field, _)| field.clone())
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

pub(super) fn enum_members<'a>(
    members: impl Iterator<
        Item = (
            ClassMemberKind,
            &'a Arc<str>,
            &'a vn_sem::types::Type,
            u32,
            bool,
            bool,
        ),
    >,
    table: &CheckerTyTable,
    interner: &AtomInterner,
    tt: &mut TyTable,
    names: &dyn NameResolver,
    signatures: &mut Vec<Signature>,
) -> Vec<VtableEntry> {
    use vn_core::AccessorKind;
    let mut ordered: Vec<_> = members.collect();
    ordered.sort_by_key(|m| m.3);
    let mut out: Vec<VtableEntry> = Vec::new();
    let push = |key: Arc<str>, sig: vn_tir::SigId, out: &mut Vec<VtableEntry>| {
        if let Some(entry) = out.iter_mut().find(|e| e.name == key) {
            entry.sig = sig;
        } else {
            out.push(VtableEntry { name: key, sig });
        }
    };
    for (kind, name, ty, _, is_static, is_abstract) in ordered.iter().copied() {
        if is_static || is_abstract {
            continue;
        }
        match kind {
            ClassMemberKind::Method | ClassMemberKind::Function | ClassMemberKind::Constructor => {
                push(
                    name.clone(),
                    intern_signature(ty, table, interner, tt, names, signatures),
                    &mut out,
                );
            }
            ClassMemberKind::Property
            | ClassMemberKind::Variable
            | ClassMemberKind::Getter
            | ClassMemberKind::Setter
            | ClassMemberKind::Type(_) => {}
        }
    }
    for (kind, name, ty, _, is_static, is_abstract) in ordered.iter().copied() {
        if is_static || is_abstract {
            continue;
        }
        match kind {
            ClassMemberKind::Getter => {
                push(
                    Arc::from(AccessorKind::Get.key(name)),
                    accessor_signature(
                        AccessorKind::Get,
                        ty,
                        table,
                        interner,
                        tt,
                        names,
                        signatures,
                    ),
                    &mut out,
                );
            }
            ClassMemberKind::Setter => {
                push(
                    Arc::from(AccessorKind::Set.key(name)),
                    accessor_signature(
                        AccessorKind::Set,
                        ty,
                        table,
                        interner,
                        tt,
                        names,
                        signatures,
                    ),
                    &mut out,
                );
            }
            ClassMemberKind::Method
            | ClassMemberKind::Function
            | ClassMemberKind::Constructor
            | ClassMemberKind::Property
            | ClassMemberKind::Variable
            | ClassMemberKind::Type(_) => {}
        }
    }
    out
}
