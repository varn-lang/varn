use crate::checker::Checker;
use std::sync::Arc;
use vn_sem::bind::BindResult;

impl Checker<'_> {
    pub(super) fn member_input(
        &self,
        ty: vn_sem::types::Type,
        bind: &BindResult,
    ) -> vn_sem::types::Type {
        let view = vn_sem::bind::BindView::new(bind, self.resolver);
        let view = if bind.source_file.as_ref() == self.source_file.as_ref() {
            view.at(self.current_scope)
        } else {
            view
        };
        let mut current = vn_sem::resolve::expand_alias(ty, Some(&view), &self.ty_table);
        let mut seen = Vec::new();
        while let vn_core::TypeKind::Named(name, None) = self.ty_table.get(current.0)
            && !seen.contains(&current)
        {
            seen.push(current);
            let text = self.resolve_bind_atom(bind, name);
            let Some(bound) =
                vn_sem::types::TypeContext::type_param_constraint(&view, text.as_ref())
            else {
                break;
            };
            current = vn_sem::resolve::expand_alias(bound, Some(&view), &self.ty_table);
        }
        current
    }

    pub(crate) fn parent_type(
        &mut self,
        parent: &vn_sem::bind::ClassParent,
    ) -> vn_sem::types::Type {
        let table = std::sync::Arc::make_mut(&mut self.ty_table);
        if parent.args.is_empty() {
            vn_sem::types::Type::named_with_origin(
                parent.name.clone(),
                parent.origin.clone(),
                table,
            )
        } else {
            vn_sem::types::Type::generic_with_origin(
                parent.name.clone(),
                parent.args.clone(),
                parent.origin.clone(),
                table,
            )
        }
    }

    pub(crate) fn declared_instance(
        &mut self,
        expected: vn_sem::types::Type,
        actual: vn_sem::types::Type,
        bind: &BindResult,
    ) -> vn_sem::types::Type {
        let vn_core::TypeKind::Generic(wanted, _, _) = self.ty_table.get(expected.0) else {
            return actual;
        };
        let vn_core::TypeKind::Named(class, origin) = self.ty_table.get(actual.0) else {
            return actual;
        };
        let wanted = self.resolve_bind_atom(bind, wanted);
        let view = vn_sem::bind::BindView::new(bind, self.resolver);
        let mut pending: Vec<(Arc<str>, Option<Arc<str>>)> = vec![(
            self.resolve_bind_atom(bind, class),
            origin.map(|o| self.resolve_bind_atom(bind, o)),
        )];
        let mut seen: Vec<Arc<str>> = Vec::new();
        while let Some((name, from)) = pending.pop() {
            if seen.contains(&name) {
                continue;
            }
            seen.push(name.clone());
            for parent in view.heritage(&name, from.as_deref()) {
                if parent.name == wanted && !parent.args.is_empty() {
                    return self.parent_type(&parent);
                }
                pending.push((parent.name.clone(), parent.origin.clone()));
            }
        }
        actual
    }

    pub(super) fn foreign_owner(
        &self,
        bind: &BindResult,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Arc<BindResult>> {
        let origin = origin.filter(|o| *o != bind.source_file.as_ref())?;
        let owner = self
            .resolver
            .module_bind(origin)
            .or_else(|| self.resolver.stdlib_bind(origin))?;
        (!std::ptr::eq(owner.as_ref(), bind) && owner.declares_type(name)).then_some(owner)
    }
}
