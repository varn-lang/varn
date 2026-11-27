use vn_core::TokenKind;

use crate::document::{DocumentState, TokenRecord};

mod annotations;
mod classify;
mod declarations;
mod object_keys;

use annotations::*;
use classify::*;
use declarations::*;
use object_keys::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionSite {
    Statement,
    Expression,
    ClassBody,
    InterfaceBody,
    EnumBody,
    AfterExport,
    Type,
    NewTarget,
    ExtendsClass,
    ExtendsInterface,
    Implements,
    ObjectKeys {
        type_name: String,
        present: Vec<String>,
    },
}

pub fn completion_site(state: &DocumentState, cursor: u32) -> CompletionSite {
    let mut before: Vec<&TokenRecord> = state
        .tokens
        .iter()
        .filter(|t| t.offset < cursor && t.kind != TokenKind::EOF)
        .collect();
    if before
        .last()
        .is_some_and(|t| t.end >= cursor && is_word(t.kind))
    {
        before.pop();
    }
    if before.is_empty() {
        return CompletionSite::Statement;
    }
    classify(state, &before, before.len() - 1)
}

pub(super) fn is_word(kind: TokenKind) -> bool {
    kind == TokenKind::Identifier || kind.is_keyword()
}
