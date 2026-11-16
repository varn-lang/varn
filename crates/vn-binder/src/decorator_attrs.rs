use super::Binder;
use vn_core::ast::decorators::{decorator_head, match_builtin, BuiltinDecorator};
use vn_core::ast::types::Decorator;
use vn_sem::symbol::SymbolId;

pub(super) fn record_decorators(binder: &mut Binder, sym: SymbolId, decorators: &[Decorator]) {
    if decorators.is_empty() {
        return;
    }
    binder
        .pending_decorator_roles
        .push((sym, decorators.to_vec()));
}

impl Binder<'_> {
    pub(super) fn resolve_decorator_roles(&mut self) {
        let pending = std::mem::take(&mut self.pending_decorator_roles);
        for (symid, decorators) in pending {
            for d in &decorators {
                if decorator_head(self.ast_arena, &self.interner, d).is_none() {
                    continue;
                };
                let mut matched =
                    match_builtin(self.ast_arena, &self.interner, std::slice::from_ref(d));
                let Some(m) = matched.pop() else {
                    continue;
                };
                let Ok(kind) = m.result.unwrap_or(Err(String::new())) else {
                    continue;
                };
                let sym = self.arena.get_mut(symid);
                match kind {
                    BuiltinDecorator::Deprecated { message } => {
                        sym.deprecated = Some(message.unwrap_or_default());
                    }
                    BuiltinDecorator::Pure => sym.is_pure = true,
                    BuiltinDecorator::Inline => sym.force_inline = true,
                    BuiltinDecorator::Capability { domains } => {
                        for d in domains {
                            if !sym.capabilities.contains(&d) {
                                sym.capabilities.push(d);
                            }
                        }
                        sym.capabilities.sort();
                    }
                    BuiltinDecorator::Test => sym.is_test = true,
                    BuiltinDecorator::Throws { classes } => {
                        for c in classes {
                            if !sym.throws.contains(&c) {
                                sym.throws.push(c);
                            }
                        }
                    }
                    BuiltinDecorator::Decodes { op } => {
                        sym.decode_op = Some(op.into());
                    }
                }
            }
        }
    }
}
