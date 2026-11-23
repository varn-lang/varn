use super::ValueId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SsaSpread<V = ValueId> {
    pub value: V,
    pub spread: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SsaObjectSpreadPart<V = ValueId> {
    pub key: Option<std::sync::Arc<str>>,
    pub value: V,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SsaVar {
    Param(u32),
    Local(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SsaUpvalue {
    Captured(SsaVar),

    Inherited(u32),
}

impl<V: Copy> SsaSpread<V> {
    pub fn map<W>(&self, f: impl Fn(V) -> W) -> SsaSpread<W> {
        SsaSpread {
            value: f(self.value),
            spread: self.spread,
        }
    }
}

impl<V: Copy> SsaObjectSpreadPart<V> {
    pub fn map<W>(&self, f: impl Fn(V) -> W) -> SsaObjectSpreadPart<W> {
        SsaObjectSpreadPart {
            key: self.key.clone(),
            value: f(self.value),
        }
    }
}
