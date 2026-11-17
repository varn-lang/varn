use super::foreign_classes::OwnerRef;
use super::Checker;
use std::collections::BTreeMap;
use std::sync::Arc;
use vn_core::TokenKind;
use vn_core::TypeKind;
use vn_sem::bind::BindResult;
use vn_sem::output::{ForeignClass, ForeignStatic, ForeignVtableEntry, InheritedField};
use vn_sem::types::{ClassMemberKind, ObjectTypeMember, Type};

impl Checker<'_> {
    pub(super) fn collect_foreign_classes<'t>(
        &mut self,
        bind: &BindResult,
        types: impl Iterator<Item = &'t Type>,
    ) -> Vec<ForeignClass> {
        let mut found: BTreeMap<(Arc<str>, Arc<str>), ForeignClass> = BTreeMap::new();
        let mut seeds: Vec<Type> = types.copied().collect();
        let mut keys: Vec<(Arc<str>, Arc<str>)> = Vec::new();
        loop {
            while let Some(ty) = seeds.pop() {
                keys.extend(self.foreign_refs_in(bind, ty));
            }
            let Some(key) = keys.pop() else { break };
            if found.contains_key(&key) {
                continue;
            }
            let Some((entry, field_tys)) = self.build_foreign_entry(bind, &key.0, &key.1) else {
                continue;
            };
            seeds.extend(field_tys);
            keys.extend(entry.parent.clone());
            found.insert(key, entry);
        }
        found.into_values().collect()
    }

    pub(super) fn foreign_refs_in(
        &mut self,
        bind: &BindResult,
        ty: Type,
    ) -> Vec<(Arc<str>, Arc<str>)> {
        let mut out = Vec::new();
        self.collect_refs_in(bind, ty, &mut out);
        out
    }

    fn collect_refs_in(
        &mut self,
        bind: &BindResult,
        ty: Type,
        out: &mut Vec<(Arc<str>, Arc<str>)>,
    ) {
        match self.ty_table.get(ty.0) {
            TypeKind::Union(list) => {
                for id in self.ty_table.get_list(list).to_vec() {
                    self.collect_refs_in(bind, Type::resolved(id), out);
                }
            }
            TypeKind::Generic(_, args, _) => {
                for id in self.ty_table.get_list(args).to_vec() {
                    self.collect_refs_in(bind, Type::resolved(id), out);
                }
                self.collect_named_ref(bind, ty, out);
            }
            TypeKind::Array(el) => {
                self.collect_refs_in(bind, Type::resolved(el), out);
            }
            TypeKind::Tuple(els) => {
                for id in self.ty_table.get_list(els).to_vec() {
                    self.collect_refs_in(bind, Type::resolved(id), out);
                }
            }
            TypeKind::Named(..) => {
                self.collect_named_ref(bind, ty, out);
            }
            TypeKind::Intersection(list) => {
                for id in self.ty_table.get_list(list).to_vec() {
                    self.collect_refs_in(bind, Type::resolved(id), out);
                }
            }
            TypeKind::Fn(fid) => {
                let f = self.ty_table.get_function(fid);
                let (params, ret) = (f.params.clone(), f.return_type);
                for p in params {
                    self.collect_refs_in(bind, Type::resolved(p.ty), out);
                }
                self.collect_refs_in(bind, Type::resolved(ret), out);
            }
            TypeKind::Record(members) | TypeKind::Interface(members) => {
                for member in self.ty_table.get_object_members(members).to_vec() {
                    match member {
                        ObjectTypeMember::Property { ty, .. } => {
                            self.collect_refs_in(bind, Type::resolved(ty), out);
                        }
                        ObjectTypeMember::Method {
                            params,
                            return_type,
                            ..
                        }
                        | ObjectTypeMember::Callable {
                            params,
                            return_type,
                            ..
                        } => {
                            for p in params {
                                self.collect_refs_in(bind, Type::resolved(p.ty), out);
                            }
                            self.collect_refs_in(bind, Type::resolved(return_type), out);
                        }
                    }
                }
            }
            TypeKind::Primitive(_)
            | TypeKind::Literal(_)
            | TypeKind::This
            | TypeKind::TemplateLiteral(_)
            | TypeKind::Typeof(_)
            | TypeKind::KeyOf(_)
            | TypeKind::IndexedAccess { .. }
            | TypeKind::Mapped { .. }
            | TypeKind::Conditional { .. }
            | TypeKind::Infer(_)
            | TypeKind::EnumVariant { .. }
            | TypeKind::TypePredicate { .. } => {}
        }
    }

    fn collect_named_ref(
        &mut self,
        bind: &BindResult,
        ty: Type,
        out: &mut Vec<(Arc<str>, Arc<str>)>,
    ) {
        let (TypeKind::Named(name, Some(origin)) | TypeKind::Generic(name, _, Some(origin))) =
            self.ty_table.get(ty.0)
        else {
            return;
        };
        if vn_core::is_lang_type(name, Some(origin)) {
            return;
        }
        let origin = self.resolve_bind_atom(bind, origin);
        if origin.as_ref() == bind.source_file.as_ref() {
            return;
        }
        let name = self.resolve_bind_atom(bind, name);
        let key = (name, origin);
        if !out.contains(&key) {
            out.push(key);
        }
    }

    fn build_foreign_entry(
        &mut self,
        bind: &BindResult,
        name: &Arc<str>,
        origin: &Arc<str>,
    ) -> Option<(ForeignClass, Vec<Type>)> {
        let owner = self
            .resolver
            .module_bind(origin)
            .or_else(|| self.resolver.stdlib_bind(origin))?;
        use vn_sem::bind::DeclKind;
        if !matches!(owner.classify_decl(name.as_ref()), Some(DeclKind::Class)) {
            return None;
        }
        let chain = self.foreign_chain(name.as_ref(), OwnerRef::Foreign(owner.clone()));
        let mut fields = Vec::new();
        let mut vtable: Vec<ForeignVtableEntry> = Vec::new();
        let ctor_ty = self.effective_ctor_ty(name.as_ref(), OwnerRef::Foreign(owner.clone()));
        let mut upsert =
            |key: Arc<str>, ty: Type, accessor: Option<vn_core::AccessorKind>| match vtable
                .iter_mut()
                .find(|e| e.key == key)
            {
                Some(slot) => {
                    slot.ty = ty;
                    slot.accessor = accessor;
                }
                None => vtable.push(ForeignVtableEntry { key, ty, accessor }),
            };

        for level in chain.iter().rev() {
            let Some(entry) = level.owner.get().get_class_entry(&level.name) else {
                continue;
            };
            let mut ordered: Vec<_> = entry.members.iter().collect();
            ordered.sort_by_key(|m| m.offset);
            let mut level_has_ctor = false;
            let mut level_primary: Option<Type> = None;
            let mut level_has_init = false;
            for m in &ordered {
                if m.is_static || m.is_abstract {
                    continue;
                }
                match m.kind {
                    ClassMemberKind::Property | ClassMemberKind::Variable => {
                        if m.kind == ClassMemberKind::Property && m.has_init {
                            level_has_init = true;
                        }
                        if fields.iter().any(|f: &InheritedField| f.name == m.name) {
                            continue;
                        }
                        fields.push(InheritedField {
                            name: m.name.clone(),
                            ty: self.reintern_foreign_ty(level.owner.get(), m.ty),
                            optional: m.is_optional,
                        });
                    }
                    ClassMemberKind::Method | ClassMemberKind::Function => {
                        upsert(
                            m.name.clone(),
                            self.reintern_foreign_ty(level.owner.get(), m.ty),
                            None,
                        );
                    }
                    ClassMemberKind::Constructor if m.is_primary => {
                        level_has_ctor = true;
                        level_primary = Some(self.reintern_foreign_ty(level.owner.get(), m.ty));
                    }
                    ClassMemberKind::Constructor => {
                        level_has_ctor = true;
                        upsert(
                            Arc::from(TokenKind::Constructor.as_str()),
                            self.reintern_foreign_ty(level.owner.get(), m.ty),
                            None,
                        );
                    }
                    ClassMemberKind::Getter
                    | ClassMemberKind::Setter
                    | ClassMemberKind::Class
                    | ClassMemberKind::Interface
                    | ClassMemberKind::Namespace
                    | ClassMemberKind::Enum => {}
                }
            }
            if let Some(ty) = level_primary {
                upsert(Arc::from(TokenKind::Constructor.as_str()), ty, None);
            } else if !level_has_ctor && level_has_init {
                upsert(Arc::from(TokenKind::Constructor.as_str()), ctor_ty, None);
            }
            for m in &ordered {
                if m.is_static || m.is_abstract {
                    continue;
                }
                match m.kind {
                    ClassMemberKind::Getter | ClassMemberKind::Setter => {
                        let kind = if m.kind == ClassMemberKind::Getter {
                            vn_core::AccessorKind::Get
                        } else {
                            vn_core::AccessorKind::Set
                        };
                        upsert(
                            Arc::from(kind.key(&m.name)),
                            self.reintern_foreign_ty(level.owner.get(), m.ty),
                            Some(kind),
                        );
                    }
                    _ => {}
                }
            }
        }
        let statics: Vec<ForeignStatic> = chain
            .first()
            .and_then(|own| own.owner.get().get_class_entry(&own.name))
            .map(|entry| {
                entry
                    .members
                    .iter()
                    .filter(|m| m.is_static)
                    .map(|m| ForeignStatic {
                        kind: m.kind,
                        name: m.name.clone(),
                        ty: self.reintern_foreign_ty(owner.as_ref(), m.ty),
                        optional: m.is_optional,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let constructor =
            Some(self.effective_ctor_ty(name.as_ref(), OwnerRef::Foreign(owner.clone())));
        let decorated = owner
            .interner
            .get(name.as_ref())
            .and_then(|atom| {
                owner
                    .scopes
                    .get(owner.global_scope)
                    .resolve(atom, &owner.scopes)
            })
            .is_none_or(|sid| owner.arena.get(sid).decorated);
        let import_binding = bind
            .global_symbols()
            .find(|s| {
                !decorated
                    && s.origin_module
                        .is_some_and(|m| self.resolve_bind_atom(bind, m) == *origin)
                    && s.original_name
                        .is_some_and(|n| self.resolve_bind_atom(bind, n) == *name)
            })
            .map(|s| self.resolve_bind_atom(bind, s.name));
        let is_struct = owner
            .get_class_entry(name.as_ref())
            .is_some_and(|entry| entry.is_struct);
        let parent = owner.get_class_parent(name.as_ref()).map(|p| {
            (
                p.name.clone(),
                p.origin.clone().unwrap_or_else(|| origin.clone()),
            )
        });
        let field_tys = fields
            .iter()
            .map(|f| f.ty)
            .chain(vtable.iter().map(|e| e.ty))
            .chain(statics.iter().map(|s| s.ty))
            .collect();
        Some((
            ForeignClass {
                name: name.clone(),
                origin: origin.clone(),
                fields,
                vtable,
                statics,
                constructor,
                import_binding,
                is_struct,
                parent,
            },
            field_tys,
        ))
    }
}
