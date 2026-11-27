use vn_core::TokenKind;

use crate::document::TokenRecord;

use super::*;
pub(super) fn is_declaration_slot(tokens: &[&TokenRecord], name: usize) -> bool {
    if !in_type_body(tokens, name) {
        return false;
    }
    let mut i = name;
    while i > 0 && is_modifier(tokens[i - 1].kind) {
        i -= 1;
    }
    i == 0
        || matches!(
            tokens[i - 1].kind,
            TokenKind::LBrace | TokenKind::RBrace | TokenKind::Semicolon | TokenKind::DocComment
        )
        || tokens[i - 1].line < tokens[name].line
}

pub(super) fn is_modifier(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Public
            | TokenKind::Private
            | TokenKind::Protected
            | TokenKind::Static
            | TokenKind::Readonly
            | TokenKind::Abstract
            | TokenKind::Override
            | TokenKind::Declare
            | TokenKind::Mut
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BodyKind {
    Class,
    Interface,
    Enum,
}

pub(super) fn type_body_kind(tokens: &[&TokenRecord], index: usize) -> Option<BodyKind> {
    let mut stack: Vec<Option<BodyKind>> = Vec::new();
    let mut pending = None;
    for token in &tokens[..index.min(tokens.len())] {
        let kind = token.kind;
        if matches!(kind, TokenKind::Class | TokenKind::Struct) {
            pending = Some(BodyKind::Class);
        } else if kind == TokenKind::Interface {
            pending = Some(BodyKind::Interface);
        } else if kind == TokenKind::Enum {
            pending = Some(BodyKind::Enum);
        } else if kind == TokenKind::LBrace {
            stack.push(pending.take());
        } else if kind == TokenKind::RBrace {
            stack.pop();
        } else if kind == TokenKind::Semicolon {
            pending = None;
        }
    }
    stack.last().copied().flatten()
}

pub(super) fn in_type_body(tokens: &[&TokenRecord], index: usize) -> bool {
    matches!(
        type_body_kind(tokens, index),
        Some(BodyKind::Class | BodyKind::Interface)
    )
}

pub(super) fn default_site(tokens: &[&TokenRecord], last: usize) -> CompletionSite {
    let kind = tokens[last].kind;
    if kind == TokenKind::Export {
        return CompletionSite::AfterExport;
    }
    let slot_start = matches!(
        kind,
        TokenKind::LBrace | TokenKind::RBrace | TokenKind::Semicolon | TokenKind::DocComment
    ) || is_modifier(kind)
        || matches!(kind, TokenKind::Async | TokenKind::Get | TokenKind::Set);
    match type_body_kind(tokens, last + 1) {
        Some(BodyKind::Class) if slot_start => return CompletionSite::ClassBody,
        Some(BodyKind::Interface) if slot_start => return CompletionSite::InterfaceBody,
        Some(BodyKind::Enum) if slot_start => return CompletionSite::EnumBody,
        Some(_) | None => {}
    }
    if matches!(
        kind,
        TokenKind::LBrace
            | TokenKind::RBrace
            | TokenKind::Semicolon
            | TokenKind::Else
            | TokenKind::Do
            | TokenKind::DocComment
    ) {
        return CompletionSite::Statement;
    }
    if kind == TokenKind::RParen && closes_control_header(tokens, last) {
        return CompletionSite::Statement;
    }
    CompletionSite::Expression
}

pub(super) fn closes_control_header(tokens: &[&TokenRecord], close: usize) -> bool {
    let mut depth = 0usize;
    for i in (0..=close).rev() {
        let kind = tokens[i].kind;
        if kind == TokenKind::RParen {
            depth += 1;
        } else if kind == TokenKind::LParen {
            depth -= 1;
            if depth == 0 {
                return i.checked_sub(1).is_some_and(|k| {
                    matches!(
                        tokens[k].kind,
                        TokenKind::If | TokenKind::While | TokenKind::For | TokenKind::Catch
                    )
                });
            }
        }
    }
    false
}

pub(super) fn innermost_open_paren(tokens: &[&TokenRecord], from: usize) -> Option<usize> {
    let mut depth = 0usize;
    for i in (0..from).rev() {
        let kind = tokens[i].kind;
        if kind == TokenKind::RParen {
            depth += 1;
        } else if kind == TokenKind::LParen {
            if depth == 0 {
                return Some(i);
            }
            depth -= 1;
        } else if matches!(kind, TokenKind::LBrace | TokenKind::Semicolon) && depth == 0 {
            return None;
        }
    }
    None
}

pub(super) fn is_parameter_list_open(tokens: &[&TokenRecord], open: usize) -> bool {
    let Some(callee) = open.checked_sub(1) else {
        return false;
    };
    let kind = tokens[callee].kind;
    if kind == TokenKind::Constructor {
        return true;
    }
    if kind == TokenKind::Identifier || kind.is_keyword() {
        let before = callee.checked_sub(1).map(|k| tokens[k].kind);
        if before == Some(TokenKind::Function) {
            return true;
        }
        return in_type_body(tokens, callee) && is_declaration_slot_name(tokens, callee);
    }
    if kind == TokenKind::RAngle || kind == TokenKind::Gt {
        return generic_declaration_open(tokens, callee);
    }
    matches!(
        kind,
        TokenKind::Eq
            | TokenKind::LParen
            | TokenKind::Comma
            | TokenKind::Colon
            | TokenKind::Return
            | TokenKind::Async
            | TokenKind::FatArrow
    )
}

pub(super) fn is_declaration_slot_name(tokens: &[&TokenRecord], name: usize) -> bool {
    let mut i = name;
    while i > 0 && (is_modifier(tokens[i - 1].kind) || tokens[i - 1].kind == TokenKind::Async) {
        i -= 1;
    }
    i == 0
        || matches!(
            tokens[i - 1].kind,
            TokenKind::LBrace | TokenKind::RBrace | TokenKind::Semicolon | TokenKind::DocComment
        )
        || tokens[i - 1].line < tokens[name].line
}

pub(super) fn generic_declaration_open(tokens: &[&TokenRecord], close: usize) -> bool {
    let mut depth = 0usize;
    for i in (0..=close).rev() {
        let kind = tokens[i].kind;
        if matches!(kind, TokenKind::RAngle | TokenKind::Gt) {
            depth += 1;
        } else if matches!(kind, TokenKind::LAngle | TokenKind::Lt) {
            depth -= 1;
            if depth == 0 {
                return i
                    .checked_sub(2)
                    .is_some_and(|k| tokens[k].kind == TokenKind::Function);
            }
        }
    }
    false
}

pub(super) fn is_parameter_list_close(tokens: &[&TokenRecord], close: usize) -> bool {
    let mut depth = 0usize;
    for i in (0..=close).rev() {
        let kind = tokens[i].kind;
        if kind == TokenKind::RParen {
            depth += 1;
        } else if kind == TokenKind::LParen {
            depth -= 1;
            if depth == 0 {
                return is_parameter_list_open(tokens, i);
            }
        }
    }
    false
}
