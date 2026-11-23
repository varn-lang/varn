use super::Binder;
use vn_core::ast::TypeNode;
use vn_sem::types::{ClassMemberInfo, Type, TypeContext};

impl TypeContext for Binder<'_> {
    fn resolver(&self) -> Option<&dyn vn_sem::resolver::ImportResolver> {
        Some(self.resolver)
    }

    fn interner(&self) -> Option<&vn_core::AtomInterner> {
        Some(&self.interner)
    }

    fn ty_table(&self) -> Option<&vn_sem::types::CheckerTyTable> {
        Some(&self.ty_table)
    }

    fn ast_arena(&self) -> Option<&vn_core::ast::AstArena> {
        Some(self.ast_arena)
    }

    fn type_members(
        &self,
        kind: vn_sem::types::DeclKind,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<ClassMemberInfo>> {
        if let Some(origin) = origin
            && origin != self.source_file.as_ref()
            && let Some(rb) = self
                .resolver
                .module_bind(origin)
                .or_else(|| self.resolver.stdlib_bind(origin))
        {
            return rb.type_members.members(kind, name).cloned();
        }
        self.type_members.members(kind, name).cloned()
    }

    fn is_type_parameter(&self, name: &str) -> bool {
        let scope = self.scopes.get(self.current);
        self.interner
            .get(name)
            .and_then(|atom| scope.resolve(atom, &self.scopes))
            .is_some_and(|id| self.arena.get(id).kind == vn_sem::symbol::SymbolKind::TypeParameter)
    }

    fn resolve_symbol(&self, name: &str) -> Option<Type> {
        let scope = self.scopes.get(self.current);
        let atom = self.interner.get(name)?;
        let id = scope.resolve(atom, &self.scopes)?;
        self.arena.get(id).ty
    }

    fn generic_defaults(&self, name: &str) -> Option<Vec<Option<Type>>> {
        let scope = self.scopes.get(self.current);
        let atom = self.interner.get(name)?;
        let id = scope.resolve(atom, &self.scopes)?;
        let sym = self.arena.get(id);
        match sym.kind {
            vn_sem::symbol::SymbolKind::Class | vn_sem::symbol::SymbolKind::Interface => {
                (!sym.type_params.is_empty()).then(|| sym.type_param_defaults.clone())
            }
            vn_sem::symbol::SymbolKind::Var
            | vn_sem::symbol::SymbolKind::Let
            | vn_sem::symbol::SymbolKind::Const
            | vn_sem::symbol::SymbolKind::Function
            | vn_sem::symbol::SymbolKind::TypeAlias
            | vn_sem::symbol::SymbolKind::Enum
            | vn_sem::symbol::SymbolKind::Parameter
            | vn_sem::symbol::SymbolKind::Property
            | vn_sem::symbol::SymbolKind::Method
            | vn_sem::symbol::SymbolKind::TypeParameter
            | vn_sem::symbol::SymbolKind::Namespace
            | vn_sem::symbol::SymbolKind::Extension
            | vn_sem::symbol::SymbolKind::EnumMember => None,
        }
    }

    fn symbol_origin(&self, name: &str) -> Option<vn_core::Atom> {
        let scope = self.scopes.get(self.current);
        let atom = self.interner.get(name)?;
        let id = scope.resolve(atom, &self.scopes)?;
        self.arena.get(id).origin_module
    }

    fn symbol_kind(&self, name: &str) -> Option<vn_sem::symbol::SymbolKind> {
        let scope = self.scopes.get(self.current);
        let atom = self.interner.get(name)?;
        let id = scope.resolve(atom, &self.scopes)?;
        Some(self.arena.get(id).kind)
    }

    fn source_file(&self) -> Option<&str> {
        Some(self.source_file.as_ref())
    }

    fn get_alias_node(&self, name: &str) -> Option<(Vec<String>, TypeNode)> {
        let scope = self.scopes.get(self.current);
        let atom = self.interner.get(name)?;
        let id = scope.resolve(atom, &self.scopes)?;
        self.arena.get(id).alias_signature(&self.interner)
    }
}
