use vn_core::TokenKind;

use crate::document::{DocumentState, TokenRecord};

use super::*;
pub(super) fn classify(
    state: &DocumentState,
    tokens: &[&TokenRecord],
    last: usize,
) -> CompletionSite {
    let kind = tokens[last].kind;
    if kind == TokenKind::New {
        return CompletionSite::NewTarget;
    }
    if matches!(kind, TokenKind::As | TokenKind::Is) {
        return CompletionSite::Type;
    }
    if matches!(
        kind,
        TokenKind::Extends | TokenKind::Implements | TokenKind::Comma
    ) && let Some(site) = heritage_site(tokens, last)
    {
        return site;
    }
    if kind == TokenKind::Colon {
        return after_colon(tokens, last);
    }
    if kind == TokenKind::Eq {
        return after_eq(tokens, last);
    }
    if kind == TokenKind::Comma
        && let Some(site) = object_keys(state, tokens, last)
    {
        return site;
    }
    if kind == TokenKind::Comma && type_body_kind(tokens, last + 1) == Some(BodyKind::Enum) {
        return CompletionSite::EnumBody;
    }
    if matches!(kind, TokenKind::LAngle | TokenKind::Lt | TokenKind::Comma) {
        return after_angle(state, tokens, last);
    }
    if matches!(kind, TokenKind::Pipe | TokenKind::Amp) {
        return after_union(state, tokens, last);
    }
    if kind == TokenKind::LBrace
        && let Some(site) = object_keys(state, tokens, last)
    {
        return site;
    }
    default_site(tokens, last)
}

pub(super) fn heritage_site(tokens: &[&TokenRecord], last: usize) -> Option<CompletionSite> {
    let mut i = last;
    loop {
        let kind = tokens[i].kind;
        if kind == TokenKind::Implements {
            return Some(CompletionSite::Implements);
        }
        if kind == TokenKind::Extends {
            return extends_site(tokens, i);
        }
        let continues = matches!(
            kind,
            TokenKind::Identifier | TokenKind::Comma | TokenKind::Dot
        );
        if !continues || i == 0 {
            return None;
        }
        i -= 1;
    }
}

pub(super) fn extends_site(tokens: &[&TokenRecord], extends: usize) -> Option<CompletionSite> {
    let mut angle_depth = 0usize;
    for token in tokens[..extends].iter().rev() {
        let kind = token.kind;
        if matches!(kind, TokenKind::RAngle | TokenKind::Gt) {
            angle_depth += 1;
        } else if matches!(kind, TokenKind::LAngle | TokenKind::Lt) {
            if angle_depth == 0 {
                return Some(CompletionSite::Type);
            }
            angle_depth -= 1;
        } else if angle_depth == 0 {
            if kind == TokenKind::Interface {
                return Some(CompletionSite::ExtendsInterface);
            }
            if matches!(kind, TokenKind::Class | TokenKind::Struct) {
                return Some(CompletionSite::ExtendsClass);
            }
            if matches!(
                kind,
                TokenKind::LBrace | TokenKind::RBrace | TokenKind::Semicolon
            ) {
                return None;
            }
        }
    }
    None
}
