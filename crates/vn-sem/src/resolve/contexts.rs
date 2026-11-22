use crate::types::{CheckerTyTable, Type, TypeContext};
use vn_core::ast::TypeNode;

pub(super) struct AliasSubstitutionContext<'a> {
    pub(super) inner: Option<&'a dyn TypeContext>,
    pub(super) params: Vec<String>,
    pub(super) args: Vec<Type>,
}

impl AliasSubstitutionContext<'_> {
    fn binds_type_param(&self, name: &str) -> bool {
        self.params.iter().any(|p| p == name)
    }
}

impl TypeContext for AliasSubstitutionContext<'_> {
    fn alias_expansion_depth(&self) -> usize {
        1 + self.inner.map_or(0, |c| c.alias_expansion_depth())
    }

    fn is_type_parameter(&self, name: &str) -> bool {
        !self.binds_type_param(name) && self.inner.is_some_and(|c| c.is_type_parameter(name))
    }

    fn resolver(&self) -> Option<&dyn crate::resolver::ImportResolver> {
        self.inner.and_then(|c| c.resolver())
    }

    fn interner(&self) -> Option<&vn_core::AtomInterner> {
        self.inner.and_then(|c| c.interner())
    }

    fn ty_table(&self) -> Option<&CheckerTyTable> {
        self.inner.and_then(|c| c.ty_table())
    }

    fn ast_arena(&self) -> Option<&vn_core::ast::AstArena> {
        None
    }

    fn resolve_symbol(&self, name: &str) -> Option<Type> {
        if let Some(pos) = self.params.iter().position(|p| p == name) {
            return Some(self.args[pos]);
        }
        self.inner.and_then(|c| c.resolve_symbol(name))
    }

    fn generic_defaults(&self, name: &str) -> Option<Vec<Option<Type>>> {
        self.inner.and_then(|c| c.generic_defaults(name))
    }

    fn symbol_origin(&self, name: &str) -> Option<vn_core::Atom> {
        self.inner.and_then(|c| c.symbol_origin(name))
    }

    fn symbol_kind(&self, name: &str) -> Option<crate::symbol::SymbolKind> {
        if self.binds_type_param(name) {
            return Some(crate::symbol::SymbolKind::TypeParameter);
        }
        self.inner.and_then(|c| c.symbol_kind(name))
    }

    fn get_interface_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<crate::types::ClassMemberInfo>> {
        self.inner
            .and_then(|c| c.get_interface_members(name, origin))
    }

    fn get_class_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<crate::types::ClassMemberInfo>> {
        self.inner.and_then(|c| c.get_class_members(name, origin))
    }

    fn get_namespace_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<crate::types::ClassMemberInfo>> {
        self.inner
            .and_then(|c| c.get_namespace_members(name, origin))
    }

    fn source_file(&self) -> Option<&str> {
        self.inner.and_then(|c| c.source_file())
    }

    fn get_alias_node(&self, name: &str) -> Option<(Vec<String>, TypeNode)> {
        self.inner.and_then(|c| c.get_alias_node(name))
    }
}
