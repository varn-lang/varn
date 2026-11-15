use vn_contract::{ContractError, ContractTy};
use vn_core::ast::{Decl, ExportDecl, TypeNode};
use vn_core::AtomInterner;

use crate::contract_members::{Kind, Member, ParamInfo};
use crate::vn_contract::Mapped;

pub(crate) enum LookupFailure {
    Unparsable(String),
    BadDecorator(String),
    ClassNotFound,
    NoFunctions,
}

pub(crate) struct ContractSection {
    pub(crate) members: Vec<Member>,
    pub(crate) extends: Option<String>,
}

pub(crate) fn lookup(
    source: &str,
    name: &str,
    class: Option<&str>,
) -> Result<ContractSection, LookupFailure> {
    let mut contract = vn_contract::parse(source, name).map_err(|e| match e {
        ContractError::Unparsable(reason) => LookupFailure::Unparsable(reason),
        ContractError::BadDecorator(reason) => LookupFailure::BadDecorator(reason),
    })?;
    match class {
        Some(class) => contract
            .classes
            .remove(class)
            .map(|c| section(c.members, c.extends))
            .ok_or(LookupFailure::ClassNotFound),
        None if contract.functions.is_empty() => Err(LookupFailure::NoFunctions),
        None => Ok(section(contract.functions, None)),
    }
}

fn section(members: Vec<vn_contract::Member>, extends: Option<String>) -> ContractSection {
    ContractSection {
        members: members.into_iter().map(member).collect(),
        extends,
    }
}

fn member(m: vn_contract::Member) -> Member {
    let kind = match m.kind {
        vn_contract::Kind::Function => Kind::Function,
        vn_contract::Kind::Method => Kind::Method,
        vn_contract::Kind::Getter => Kind::Getter,
        vn_contract::Kind::StaticMethod => Kind::StaticMethod,
        vn_contract::Kind::StaticGetter => Kind::StaticGetter,
        vn_contract::Kind::Constructor => Kind::Constructor,
        vn_contract::Kind::Property => Kind::Property,
    };
    let throws = if kind == Kind::Function {
        Vec::new()
    } else {
        m.throws
    };
    Member {
        symbol: m.symbol,
        kind,
        params: m
            .params
            .iter()
            .map(|p| ParamInfo {
                mapped: if p.optional && !p.rest {
                    Mapped::Opt(Box::new(Mapped::of(&p.ty)))
                } else {
                    Mapped::of(&p.ty)
                },
                is_rest: p.rest,
            })
            .collect(),
        ret: Mapped::of(&m.ret),
        fallible: !throws.is_empty(),
        throws,
        capability: m.capability,
    }
}

pub(crate) fn classify(t: &TypeNode, interner: &AtomInterner) -> Mapped {
    Mapped::of(&ContractTy::from_node(t, interner))
}

pub(crate) fn unwrap_export(decl: &Decl) -> &Decl {
    match decl {
        Decl::Export(ExportDecl::Decl { declaration, .. }) => unwrap_export(declaration),
        Decl::Variable(_)
        | Decl::Function(_)
        | Decl::Class(_)
        | Decl::Interface(_)
        | Decl::TypeAlias(_)
        | Decl::Enum(_)
        | Decl::Namespace(_)
        | Decl::Import(_)
        | Decl::Export(_)
        | Decl::Extension(_)
        | Decl::SumType(_) => decl,
    }
}
