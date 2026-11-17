use std::collections::BTreeMap;

use crate::ContractTy;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Function,
    Method,
    Getter,
    StaticMethod,
    StaticGetter,
    Constructor,
    Property,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: ContractTy,
    pub optional: bool,
    pub rest: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub symbol: String,
    pub kind: Kind,
    pub params: Vec<Param>,
    pub ret: ContractTy,
    pub throws: Vec<String>,
    pub capability: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassContract {
    pub extends: Option<String>,
    pub members: Vec<Member>,
    pub is_struct: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModuleContract {
    pub classes: BTreeMap<String, ClassContract>,
    pub functions: Vec<Member>,
    pub exports: Vec<String>,
    pub has_code: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContractError {
    Unparsable(String),
    BadDecorator(String),
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unparsable(reason) | Self::BadDecorator(reason) => f.write_str(reason),
        }
    }
}
