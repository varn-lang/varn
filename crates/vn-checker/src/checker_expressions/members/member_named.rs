use super::Checker;
use std::sync::Arc;
use vn_sem::bind::BindResult;
use vn_sem::types::Type;

impl<'r> Checker<'r> {
    pub(super) fn member_enum_variant(
        &mut self,
        enum_name: vn_core::Atom,
        payload_ty: vn_sem::types::CheckerTyId,
        key: &str,
        bind: &BindResult,
    ) -> Option<(Type, super::member_auth::MemberAuth)> {
        if key == vn_core::MemberKey::Name.name() || key == vn_core::MemberKey::VariantName.name() {
            return Some((Type::Str, super::member_auth::MemberAuth::none()));
        }
        if key == vn_core::MemberKey::Tag.name() || key == vn_core::MemberKey::RawValue.name() {
            return Some((Type::Int, super::member_auth::MemberAuth::none()));
        }
        if let Some(res) = self.find_member_info_uncached(&Type::resolved(payload_ty), key, bind) {
            return Some(res);
        }
        let enum_name_str = self.resolve_bind_atom(bind, enum_name).to_string();
        let named = Type::named(
            enum_name_str,
            &mut *std::sync::Arc::make_mut(&mut self.ty_table),
        );
        self.find_member_info_uncached(&named, key, bind)
    }

    pub(super) fn member_named(
        &mut self,
        ty: &Type,
        name_atom: vn_core::Atom,
        origin_atom: Option<vn_core::Atom>,
        key: &str,
        bind: &BindResult,
    ) -> Option<(Type, super::member_auth::MemberAuth)> {
        let name: Arc<str> = self.resolve_bind_atom(bind, name_atom);
        let origin: Option<Arc<str>> = origin_atom.map(|o| self.resolve_bind_atom(bind, o));
        if name.as_ref() == "*" {
            if let Some(origin_path) = &origin {
                let exports = if vn_binder::paths::is_known_module(origin_path) {
                    Some(self.resolver.stdlib_exports(origin_path))
                } else {
                    let mut visiting = Vec::new();
                    Some(self.resolver.module_exports(origin_path, &mut visiting))
                };
                if let Some(exports) = exports
                    && let Some(sym) = exports.get(key)
                {
                    let mut sym_ty = sym.ty.unwrap_or(Type::Dynamic);
                    if let Some(origin) = &sym.origin_module {
                        let origin_str = self.resolve_bind_atom(bind, *origin).to_string();
                        let table = std::sync::Arc::make_mut(&mut self.ty_table);
                        let origin_atom = table.intern_name(&origin_str);
                        sym_ty = sym_ty.with_origin(origin_atom, table);
                    }
                    return Some((sym_ty, super::member_auth::MemberAuth::of_symbol(sym, None)));
                }
            }
            return None;
        }

        if let Some(owner) = self.foreign_owner(bind, &name, origin.as_deref()) {
            self.owner_lookup_depth += 1;
            let found = self.find_member_info_uncached(ty, key, &owner);
            self.owner_lookup_depth -= 1;
            return found.map(|(t, sym)| (self.reintern_foreign_ty(&owner, t), sym));
        }

        let origin_modules: Vec<String> = origin.iter().map(|s| s.to_string()).collect();
        let is_enum = super::is_enum_type(self.resolver, bind, &name, &origin_modules);

        if is_enum {
            if key == vn_core::MemberKey::RawValue.name() || key == vn_core::MemberKey::Tag.name() {
                return Some((Type::Int, super::member_auth::MemberAuth::none()));
            }
            if key == vn_core::MemberKey::Name.name()
                || key == vn_core::MemberKey::VariantName.name()
            {
                return Some((Type::Str, super::member_auth::MemberAuth::none()));
            }

            let mut variants = Vec::new();
            if let Some(members) = bind.get_enum_members_local(name.as_ref()) {
                variants.extend(members.iter().map(|m| m.name.clone()));
            }
            if let Some(ext_bind) = self.resolver.find_bind_for_type(&name, &origin_modules)
                && let Some(members) = ext_bind.get_enum_members_local(name.as_ref())
            {
                variants.extend(members.iter().map(|m| m.name.clone()));
            }
            let mut found_tys = Vec::new();
            for v in &variants {
                if let Some(fields) = bind.variant_fields(name.as_ref(), v)
                    && let Some((_, ty)) = fields.iter().find(|(fname, _)| fname.as_ref() == key)
                {
                    found_tys.push(*ty);
                }
                if let Some(ext_bind) = self.resolver.find_bind_for_type(&name, &origin_modules)
                    && let Some(fields) = ext_bind.variant_fields(name.as_ref(), v)
                    && let Some((_, ty)) = fields.iter().find(|(fname, _)| fname.as_ref() == key)
                {
                    let ty = *ty;
                    found_tys.push(self.reintern_foreign_ty(&ext_bind, ty));
                }
            }
            if !found_tys.is_empty() {
                return Some((
                    Type::union(
                        found_tys,
                        &mut *std::sync::Arc::make_mut(&mut self.ty_table),
                    ),
                    super::member_auth::MemberAuth::none(),
                ));
            }
        }
        if let Some(members) = bind.type_members.classes.get(&name)
            && let Some(m) = members.members.iter().find(|m| m.name.as_ref() == key)
        {
            let ty = if m.is_optional {
                Type::make_nullable(m.ty, &mut *std::sync::Arc::make_mut(&mut self.ty_table))
            } else {
                m.ty
            };
            return Some((ty, super::member_auth::MemberAuth::of_member(bind, m)));
        }
        if let Some(members) = bind.type_members.interfaces.get(&name)
            && let Some(m) = members.iter().find(|m| m.name.as_ref() == key)
        {
            let ty = if m.is_optional {
                Type::make_nullable(m.ty, &mut *std::sync::Arc::make_mut(&mut self.ty_table))
            } else {
                m.ty
            };
            return Some((ty, super::member_auth::MemberAuth::of_member(bind, m)));
        }
        if let Some(members) = bind.type_members.namespaces.get(&name)
            && let Some(m) = members.iter().find(|m| m.name.as_ref() == key)
        {
            return Some((m.ty, super::member_auth::MemberAuth::of_member(bind, m)));
        }
        if let Some(members) = bind.get_enum_members_local(name.as_ref())
            && let Some(m) = members.iter().find(|m| m.name.as_ref() == key)
        {
            return Some((m.ty, super::member_auth::MemberAuth::of_member(bind, m)));
        }

        if let Some(ty) = bind
            .get_class_methods_for(name.as_ref())
            .and_then(|m| m.get(key))
        {
            return Some((*ty, super::member_auth::MemberAuth::none()));
        }
        if let Some(b) = &bind.core {
            if let Some(members) = b.types.classes.get(name.as_ref())
                && let Some(m) = members.members.iter().find(|m| m.name.as_ref() == key)
            {
                return Some((m.ty, super::member_auth::MemberAuth::of_info(m)));
            }
            if let Some(members) = b.types.flattened.get(name.as_ref())
                && let Some(m) = members.iter().find(|m| m.name.as_ref() == key)
            {
                return Some((m.ty, super::member_auth::MemberAuth::of_info(m)));
            }
            if let Some(methods) = b.class_methods.get(name.as_ref())
                && let Some(ty) = methods.get(key)
            {
                return Some((*ty, super::member_auth::MemberAuth::none()));
            }
        }
        if let Some(parent) = bind.class_parents.get(&name) {
            let parent = parent.clone();
            let named = self.parent_type(&parent);
            return self.find_member_info_uncached(&named, key, bind);
        }

        let ext_bind_opt = self.resolver.find_bind_for_type(&name, &origin_modules);
        if let Some(ext_bind) = ext_bind_opt {
            if let Some(found) = self.search_named_in(&ext_bind, &name, key) {
                return Some(found);
            }
        } else if origin.is_none() {
            let resolver = self.resolver;
            let mut found = None;
            resolver.find_stdlib_bind(&mut |ext_bind| {
                found = self.search_named_in(ext_bind, &name, key);
                found.is_some()
            });
            if let Some(found) = found {
                return Some(found);
            }
        }
        None
    }

    fn search_named_in(
        &mut self,
        ext_bind: &BindResult,
        name: &Arc<str>,
        key: &str,
    ) -> Option<(Type, super::member_auth::MemberAuth)> {
        if let Some(members) = ext_bind.type_members.classes.get(name)
            && let Some(m) = members.members.iter().find(|m| m.name.as_ref() == key)
        {
            let ty = m.ty;
            return Some((
                self.reintern_foreign_ty(ext_bind, ty),
                super::member_auth::MemberAuth::of_member(ext_bind, m),
            ));
        }
        if let Some(entry) = ext_bind.get_class_entry(name)
            && let Some(m) = entry.members.iter().find(|m| m.name.as_ref() == key)
        {
            let ty = m.ty;
            return Some((
                self.reintern_foreign_ty(ext_bind, ty),
                super::member_auth::MemberAuth::of_member(ext_bind, m),
            ));
        }
        if let Some(members) = ext_bind.type_members.interfaces.get(name)
            && let Some(m) = members.iter().find(|m| m.name.as_ref() == key)
        {
            let ty = m.ty;
            return Some((
                self.reintern_foreign_ty(ext_bind, ty),
                super::member_auth::MemberAuth::of_member(ext_bind, m),
            ));
        }
        if let Some(members) = ext_bind.type_members.namespaces.get(name)
            && let Some(m) = members.iter().find(|m| m.name.as_ref() == key)
        {
            let ty = m.ty;
            return Some((
                self.reintern_foreign_ty(ext_bind, ty),
                super::member_auth::MemberAuth::of_member(ext_bind, m),
            ));
        }
        if let Some(members) = ext_bind.get_enum_members_local(name.as_ref())
            && let Some(m) = members.iter().find(|m| m.name.as_ref() == key)
        {
            let ty = m.ty;
            return Some((
                self.reintern_foreign_ty(ext_bind, ty),
                super::member_auth::MemberAuth::of_member(ext_bind, m),
            ));
        }
        if let Some(ty) = ext_bind
            .get_class_methods_for(name.as_ref())
            .and_then(|m| m.get(key))
        {
            let ty = *ty;
            return Some((
                self.reintern_foreign_ty(ext_bind, ty),
                super::member_auth::MemberAuth::none(),
            ));
        }
        None
    }
}
