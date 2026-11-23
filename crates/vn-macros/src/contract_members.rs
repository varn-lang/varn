use crate::natives::Mapped;

pub(crate) struct ParamInfo {
    pub(crate) name: String,
    pub(crate) mapped: Mapped,
    pub(crate) is_rest: bool,
}

pub(crate) struct Member {
    pub(crate) symbol: String,
    pub(crate) kind: vn_contract::Kind,
    pub(crate) params: Vec<ParamInfo>,
    pub(crate) ret: Mapped,

    pub(crate) field: Option<vn_core::FieldKind>,

    pub(crate) fallible: bool,

    pub(crate) throws: Vec<String>,

    pub(crate) capability: Vec<String>,
}
