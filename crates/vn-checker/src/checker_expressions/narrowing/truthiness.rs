use crate::checker::recorder::Recorder;
use crate::checker::Checker;
use vn_sem::bind::BindResult;
use vn_sem::symbol::SymbolId;
use vn_sem::types::Type;

impl<'r> Checker<'r> {
    pub(crate) fn narrowing_base(
        &self,
        rec: &Recorder,
        id: SymbolId,
        bind: &BindResult,
    ) -> Option<Type> {
        self.narrowed_types
            .get(&id)
            .and_then(|stack| stack.last().copied())
            .or_else(|| rec.symbol_types.get(&id).cloned())
            .or_else(|| bind.arena.get(id).ty)
    }
}
