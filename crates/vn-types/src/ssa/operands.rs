use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SsaSpread {
    pub value: u32,
    pub spread: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SsaObjectSpreadPart {
    pub key: Option<std::sync::Arc<str>>,
    pub value: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SsaUpvalue {
    Captured(u32),

    Inherited(u32),
}
