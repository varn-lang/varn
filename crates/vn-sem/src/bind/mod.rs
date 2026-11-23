use crate::cores::CoreMembers;
use crate::scope::{ScopeArena, ScopeId};
use crate::symbol::{Symbol, SymbolArena, SymbolId};
use crate::types::DeclKind;
use crate::types::{ClassMemberInfo, ClassMemberKind, Type};
use rustc_hash::FxHashMap;
use std::sync::Arc;
use vn_core::ast::{ExprId, StmtId, TypeNode};
use vn_core::Atom;

pub use crate::types::TypeContext;

#[derive(Clone)]
pub enum PendingEnrich {
    Var {
        sym_id: SymbolId,
        init: ExprId,
    },
    Fn {
        sym_id: SymbolId,
        body: StmtId,
        is_async: bool,
    },
    Method {
        class_name: Atom,
        key: Atom,
        body: StmtId,
        is_async: bool,
    },
    Getter {
        class_name: Atom,
        key: Atom,
        body: StmtId,
    },
    Setter {
        class_name: Atom,
        key: Atom,
        body: StmtId,
    },
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TypeMembers {
    #[serde(serialize_with = "crate::portable::canonical::sorted")]
    pub classes: FxHashMap<Arc<str>, ClassMemberInfo>,
    #[serde(serialize_with = "crate::portable::canonical::sorted")]
    pub interfaces: FxHashMap<Arc<str>, Vec<ClassMemberInfo>>,
    #[serde(serialize_with = "crate::portable::canonical::sorted")]
    pub objects: FxHashMap<Atom, Vec<ClassMemberInfo>>,
    #[serde(serialize_with = "crate::portable::canonical::sorted")]
    pub enums: FxHashMap<Arc<str>, Vec<ClassMemberInfo>>,
    #[serde(serialize_with = "crate::portable::canonical::sorted")]
    pub namespaces: FxHashMap<Arc<str>, Vec<ClassMemberInfo>>,
    #[serde(serialize_with = "crate::portable::canonical::sorted")]
    pub flattened: FxHashMap<Arc<str>, Vec<ClassMemberInfo>>,
    #[serde(serialize_with = "crate::portable::canonical::sorted_nested")]
    pub getters: FxHashMap<Arc<str>, FxHashMap<Arc<str>, Type>>,
    #[serde(serialize_with = "crate::portable::canonical::sorted_nested")]
    pub setters: FxHashMap<Arc<str>, FxHashMap<Arc<str>, Type>>,
}

pub type VariantFields = FxHashMap<Arc<str>, FxHashMap<Arc<str>, Vec<(Arc<str>, Type)>>>;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Extensions {
    #[serde(serialize_with = "crate::portable::canonical::sorted_nested")]
    pub methods: FxHashMap<Arc<str>, FxHashMap<Arc<str>, Vec<ExtensionCandidate>>>,
    #[serde(serialize_with = "crate::portable::canonical::sorted_nested")]
    pub getters: FxHashMap<Arc<str>, FxHashMap<Arc<str>, Vec<ExtensionCandidate>>>,
    #[serde(serialize_with = "crate::portable::canonical::sorted_nested")]
    pub setters: FxHashMap<Arc<str>, FxHashMap<Arc<str>, Vec<ExtensionCandidate>>>,
    #[serde(default)]
    pub forwarded: std::collections::BTreeSet<Arc<str>>,
}

type ExtensionBuckets = FxHashMap<Arc<str>, FxHashMap<Arc<str>, Vec<ExtensionCandidate>>>;

impl Extensions {
    fn table(&self, kind: vn_core::extension_names::ExtensionKind) -> &ExtensionBuckets {
        use vn_core::extension_names::ExtensionKind;
        match kind {
            ExtensionKind::Method => &self.methods,
            ExtensionKind::Getter => &self.getters,
            ExtensionKind::Setter => &self.setters,
        }
    }

    fn table_mut(
        &mut self,
        kind: vn_core::extension_names::ExtensionKind,
    ) -> &mut ExtensionBuckets {
        use vn_core::extension_names::ExtensionKind;
        match kind {
            ExtensionKind::Method => &mut self.methods,
            ExtensionKind::Getter => &mut self.getters,
            ExtensionKind::Setter => &mut self.setters,
        }
    }

    pub fn absorb(&mut self, other: &Extensions) {
        use vn_core::extension_names::ExtensionKind;
        self.forwarded.extend(other.forwarded.iter().cloned());
        for kind in ExtensionKind::ALL {
            for (head, members) in other.table(kind) {
                for (member, candidates) in members {
                    for candidate in candidates {
                        let _ = self.record(kind, head, member, candidate.clone());
                    }
                }
            }
        }
    }

    pub fn record(
        &mut self,
        kind: vn_core::extension_names::ExtensionKind,
        head: &str,
        member: &str,
        candidate: ExtensionCandidate,
    ) -> Result<(), Arc<str>> {
        let slot = self
            .table_mut(kind)
            .entry(Arc::from(head))
            .or_default()
            .entry(Arc::from(member))
            .or_default();
        if slot.iter().any(|c| c.mangled == candidate.mangled) {
            return Ok(());
        }
        let rank = candidate.pattern.specificity();
        if let Some(clash) = slot
            .iter()
            .find(|c| c.pattern.specificity() == rank && c.pattern.overlaps(&candidate.pattern))
        {
            return Err(clash.mangled.clone());
        }
        slot.push(candidate);
        slot.sort_by(|a, b| a.mangled.cmp(&b.mangled));
        Ok(())
    }

    pub fn matching(
        &self,
        kind: vn_core::extension_names::ExtensionKind,
        receiver: Type,
        member: &str,
        table: &crate::types::CheckerTyTable,
        name_of: receiver_pattern::NameOf,
    ) -> Vec<Arc<str>> {
        let Some(concrete) = ReceiverPattern::of(receiver, table, name_of, &[]) else {
            return Vec::new();
        };
        concrete
            .head()
            .and_then(|head| self.table(kind).get(&head))
            .and_then(|bucket| bucket.get(member))
            .map(|candidates| by_specificity(candidates, &concrete))
            .unwrap_or_default()
    }

    pub fn candidates_named(
        &self,
        kind: vn_core::extension_names::ExtensionKind,
        receiver: Type,
        member: &str,
        table: &crate::types::CheckerTyTable,
        name_of: receiver_pattern::NameOf,
    ) -> Vec<Arc<str>> {
        ReceiverPattern::of(receiver, table, name_of, &[])
            .and_then(|concrete| concrete.head())
            .and_then(|head| self.table(kind).get(&head))
            .and_then(|bucket| bucket.get(member))
            .map(|candidates| candidates.iter().map(|c| c.mangled.clone()).collect())
            .unwrap_or_default()
    }

    pub fn matching_members(
        &self,
        kind: vn_core::extension_names::ExtensionKind,
        receiver: Type,
        table: &crate::types::CheckerTyTable,
        name_of: receiver_pattern::NameOf,
    ) -> Vec<(Arc<str>, Vec<Arc<str>>)> {
        let Some(concrete) = ReceiverPattern::of(receiver, table, name_of, &[]) else {
            return Vec::new();
        };
        let Some(bucket) = concrete.head().and_then(|head| self.table(kind).get(&head)) else {
            return Vec::new();
        };
        let mut out: Vec<(Arc<str>, Vec<Arc<str>>)> = bucket
            .iter()
            .map(|(member, candidates)| (member.clone(), by_specificity(candidates, &concrete)))
            .filter(|(_, found)| !found.is_empty())
            .collect();
        out.sort();
        out
    }

    pub fn all_mangled(&self) -> Vec<Arc<str>> {
        use vn_core::extension_names::ExtensionKind;
        let mut out: Vec<Arc<str>> = ExtensionKind::ALL
            .into_iter()
            .flat_map(|kind| self.table(kind).values())
            .flat_map(|members| members.values())
            .flatten()
            .map(|c| c.mangled.clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

fn by_specificity(candidates: &[ExtensionCandidate], concrete: &ReceiverPattern) -> Vec<Arc<str>> {
    let mut found: Vec<&ExtensionCandidate> = candidates
        .iter()
        .filter(|c| c.pattern.matches(concrete, &mut Vec::new()))
        .collect();
    found.sort_by(|a, b| {
        b.pattern
            .specificity()
            .cmp(&a.pattern.specificity())
            .then_with(|| a.mangled.cmp(&b.mangled))
    });
    found.into_iter().map(|c| c.mangled.clone()).collect()
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClassParent {
    pub name: Arc<str>,
    pub origin: Option<Arc<str>>,
    pub args: Vec<Type>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct BindResult {
    pub arena: SymbolArena,
    pub scopes: ScopeArena,
    pub global_scope: ScopeId,
    #[serde(skip)]
    pub diagnostics: vn_core::DiagnosticBag,
    #[serde(skip)]
    pub interner: vn_core::AtomInterner,
    #[serde(skip, default)]
    pub ty_table: std::sync::Arc<crate::types::CheckerTyTable>,
    pub deps: Vec<Arc<str>>,
    #[serde(skip, default)]
    pub annotation_types: FxHashMap<vn_core::ast::AstId, Type>,
    #[serde(skip, default)]
    pub match_arm_scopes: FxHashMap<vn_core::ast::AstId, Vec<ScopeId>>,
    #[serde(serialize_with = "crate::portable::canonical::sorted_nested")]
    pub class_methods: FxHashMap<Arc<str>, FxHashMap<Arc<str>, Type>>,
    pub type_members: TypeMembers,
    #[serde(serialize_with = "crate::portable::canonical::sorted")]
    pub class_parents: FxHashMap<Arc<str>, ClassParent>,
    #[serde(serialize_with = "crate::portable::canonical::sorted")]
    pub class_interfaces: FxHashMap<Arc<str>, Vec<ClassParent>>,
    #[serde(serialize_with = "crate::portable::canonical::sorted")]
    pub type_param_variance: FxHashMap<Arc<str>, Vec<crate::compat::variance::Variance>>,
    pub source_file: Arc<str>,
    #[serde(serialize_with = "crate::portable::canonical::sorted")]
    pub decl_kinds: FxHashMap<Arc<str>, DeclKind>,
    #[serde(serialize_with = "crate::portable::canonical::sorted")]
    pub sum_type_variants: FxHashMap<Arc<str>, Vec<Arc<str>>>,
    #[serde(serialize_with = "crate::portable::canonical::sorted")]
    pub enum_variant_tags: FxHashMap<Arc<str>, Vec<i64>>,
    #[serde(serialize_with = "crate::portable::canonical::sorted")]
    pub sum_variant_parent: FxHashMap<Arc<str>, Arc<str>>,
    #[serde(serialize_with = "crate::portable::canonical::sorted_nested")]
    pub sum_variant_fields: VariantFields,
    pub extensions: Extensions,
    #[serde(skip)]
    pub core: Option<Arc<CoreMembers>>,
    #[serde(skip)]
    pub pending_enrich: Vec<PendingEnrich>,
    #[serde(skip, default)]
    pub namespace_imports: Vec<NamespaceImport>,
    #[serde(skip, default)]
    pub imported_forwards: FxHashMap<Arc<str>, Arc<crate::symbol::Forward>>,
    #[serde(skip, default)]
    pub declared_type_names: FxHashMap<vn_core::ast::AstId, Arc<str>>,
    #[serde(skip, default)]
    pub generic_type_uses: Vec<GenericTypeUse>,
}

#[derive(Clone, Debug)]
pub struct GenericTypeUse {
    pub range: vn_core::SourceRange,
    pub scope: ScopeId,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct NamespaceImport {
    pub binding: Arc<str>,
    pub hang: Arc<str>,
    pub exported: Arc<str>,
    pub members: Vec<Arc<str>>,
}

mod extension_labels;
pub mod receiver_pattern;
mod settle;
mod view;
pub use extension_labels::{extension_label, extension_names, extension_pattern, ExtensionName};
pub use receiver_pattern::{ExtensionCandidate, ReceiverPattern};
pub use settle::settle_members;
pub use view::BindView;

impl BindResult {
    pub fn classify_decl(&self, name: &str) -> Option<DeclKind> {
        self.decl_kinds.get(name).copied()
    }

    pub fn declares_type(&self, name: &str) -> bool {
        self.type_members.classes.contains_key(name)
            || self.type_members.interfaces.contains_key(name)
            || self.type_members.enums.contains_key(name)
            || self.type_members.namespaces.contains_key(name)
            || self.sum_type_variants.contains_key(name)
    }

    pub fn variant_fields(&self, enum_name: &str, variant: &str) -> Option<&Vec<(Arc<str>, Type)>> {
        self.sum_variant_fields.get(enum_name)?.get(variant)
    }

    pub fn enum_layout(&self, name: &str) -> Option<Vec<VariantLayout>> {
        let fields = |variant: &Arc<str>| {
            self.variant_fields(name, variant)
                .cloned()
                .unwrap_or_default()
        };
        let tag = |at: usize| {
            self.enum_variant_tags
                .get(name)
                .and_then(|tags| tags.get(at).copied())
                .unwrap_or(at as i64)
        };
        if let Some(variants) = self.sum_type_variants.get(name) {
            return Some(
                variants
                    .iter()
                    .enumerate()
                    .map(|(at, v)| VariantLayout {
                        name: v.clone(),
                        tag: tag(at),
                        fields: fields(v),
                    })
                    .collect(),
            );
        }
        let members = self.type_members.enums.get(name)?;
        Some(
            members
                .iter()
                .filter(|m| {
                    !m.is_static
                        && !matches!(
                            m.kind,
                            ClassMemberKind::Method
                                | ClassMemberKind::Getter
                                | ClassMemberKind::Setter
                                | ClassMemberKind::Constructor
                        )
                })
                .enumerate()
                .map(|(at, m)| VariantLayout {
                    name: m.name.clone(),
                    tag: tag(at),
                    fields: fields(&m.name),
                })
                .collect(),
        )
    }
}

#[derive(Clone, Debug)]
pub struct VariantLayout {
    pub name: Arc<str>,
    pub tag: i64,
    pub fields: Vec<(Arc<str>, Type)>,
}

impl BindResult {
    pub fn global_symbols(&self) -> impl Iterator<Item = &Symbol> {
        let scope = self.scopes.get(self.global_scope);
        scope.ordered.iter().map(|&id| self.arena.get(id))
    }

    pub fn get_class_entry(&self, name: &str) -> Option<&ClassMemberInfo> {
        self.type_members
            .classes
            .get(name)
            .or_else(|| self.core.as_ref().and_then(|b| b.class_members.get(name)))
    }

    pub fn get_interface_members_local(&self, name: &str) -> Option<&Vec<ClassMemberInfo>> {
        self.type_members.interfaces.get(name).or_else(|| {
            self.core
                .as_ref()
                .and_then(|b| b.interface_members.get(name))
        })
    }

    pub fn get_namespace_members_local(&self, name: &str) -> Option<&Vec<ClassMemberInfo>> {
        self.type_members.namespaces.get(name).or_else(|| {
            self.core
                .as_ref()
                .and_then(|b| b.namespace_members.get(name))
        })
    }

    pub fn get_enum_members_local(&self, name: &str) -> Option<&Vec<ClassMemberInfo>> {
        self.type_members
            .enums
            .get(name)
            .or_else(|| self.core.as_ref().and_then(|b| b.enum_members.get(name)))
    }

    pub fn get_class_methods_for(&self, name: &str) -> Option<&FxHashMap<Arc<str>, Type>> {
        self.class_methods
            .get(name)
            .or_else(|| self.core.as_ref().and_then(|b| b.class_methods.get(name)))
    }

    pub fn get_class_parent(&self, name: &str) -> Option<&ClassParent> {
        self.class_parents
            .get(name)
            .or_else(|| self.core.as_ref().and_then(|b| b.class_parents.get(name)))
    }

    pub fn get_class_interfaces(&self, name: &str) -> &[ClassParent] {
        self.class_interfaces
            .get(name)
            .map_or(&[][..], Vec::as_slice)
    }

    pub fn get_alias_node_local(&self, name: &str) -> Option<(Vec<String>, TypeNode)> {
        let scope = self.scopes.get(self.global_scope);
        let atom = self.interner.get(name)?;
        let id = scope.resolve(atom, &self.scopes)?;
        self.arena.get(id).alias_signature(&self.interner)
    }

    pub fn has_named_type(&self, name: &str) -> bool {
        self.type_members.classes.contains_key(name)
            || self.type_members.interfaces.contains_key(name)
            || self.type_members.namespaces.contains_key(name)
            || self.type_members.enums.contains_key(name)
            || self
                .core
                .as_ref()
                .map(|b| {
                    b.class_members.contains_key(name)
                        || b.interface_members.contains_key(name)
                        || b.namespace_members.contains_key(name)
                        || b.enum_members.contains_key(name)
                })
                .unwrap_or(false)
    }
}
