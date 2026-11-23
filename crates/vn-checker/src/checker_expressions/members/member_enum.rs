use std::sync::Arc;

use vn_sem::bind::BindResult;

pub(crate) fn is_enum_type(
    resolver: &dyn vn_sem::resolver::ImportResolver,
    bind: &BindResult,
    name: &Arc<str>,
    origin_modules: &[String],
) -> bool {
    if bind
        .members_local(vn_sem::types::DeclKind::Enum, name.as_ref())
        .is_some()
    {
        return true;
    }
    if bind
        .core
        .as_ref()
        .is_some_and(|b| b.types.enums.contains_key(name.as_ref()))
    {
        return true;
    }
    if bind.type_members.classes.contains_key(name)
        || bind.type_members.interfaces.contains_key(name)
    {
        return false;
    }
    resolver
        .find_bind_for_type(name, origin_modules)
        .as_ref()
        .is_some_and(|eb| {
            eb.members_local(vn_sem::types::DeclKind::Enum, name.as_ref())
                .is_some()
        })
}
