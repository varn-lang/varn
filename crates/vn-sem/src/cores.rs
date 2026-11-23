use crate::bind::ClassParent;
use crate::symbol::Symbol;
use crate::types::CheckerTyTable;
use rustc_hash::FxHashMap;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct CoreMembers {
    pub class_methods: FxHashMap<Arc<str>, FxHashMap<Arc<str>, crate::types::Type>>,
    pub types: crate::bind::TypeMembers,
    pub class_parents: FxHashMap<Arc<str>, ClassParent>,
    pub class_type_params: FxHashMap<Arc<str>, Vec<Arc<str>>>,
    pub table: Arc<CheckerTyTable>,
}

#[derive(Default)]
pub struct CoreExports {
    pub symbols: FxHashMap<Arc<str>, Symbol>,
    pub extensions: crate::bind::Extensions,
    pub table: Arc<CheckerTyTable>,
}
