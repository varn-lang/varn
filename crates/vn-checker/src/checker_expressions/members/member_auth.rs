use vn_sem::bind::BindResult;
use vn_sem::symbol::SymbolKind;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MemberAuth {
    pub capabilities: Vec<String>,
    pub pure: bool,
    pub kind: Option<SymbolKind>,
    pub deprecated: Option<String>,
    pub sid: Option<usize>,
}

impl MemberAuth {
    pub(crate) fn none() -> Self {
        Self::default()
    }

    pub(crate) fn of(bind: &BindResult, sid: Option<usize>) -> Self {
        let Some(sid) = sid else {
            return Self::none();
        };
        let Some(sym) = bind.arena.all().get(sid) else {
            return Self::none();
        };
        Self {
            capabilities: sym.capabilities.clone(),
            pure: sym.is_pure,
            kind: Some(sym.kind),
            deprecated: sym.deprecated.clone(),
            sid: Some(sid),
        }
    }

    pub(crate) fn of_symbol(sym: &vn_sem::symbol::Symbol, sid: Option<usize>) -> Self {
        Self {
            capabilities: sym.capabilities.clone(),
            pure: sym.is_pure,
            kind: Some(sym.kind),
            deprecated: sym.deprecated.clone(),
            sid,
        }
    }
}
