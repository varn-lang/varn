use vn_core::TokenKind;
use vn_sem::symbol::SymbolKind;

use crate::document::{DocumentState, TokenRecord};

use super::*;
pub(super) fn after_colon(tokens: &[&TokenRecord], colon: usize) -> CompletionSite {
    let Some(label) = colon.checked_sub(1) else {
        return CompletionSite::Expression;
    };
    let mut name = label;
    if tokens[name].kind == TokenKind::Question && name > 0 {
        name -= 1;
    }
    if tokens[name].kind == TokenKind::RParen {
        return if is_parameter_list_close(tokens, name) {
            CompletionSite::Type
        } else {
            CompletionSite::Expression
        };
    }
    if tokens[name].kind != TokenKind::Identifier && !tokens[name].kind.is_keyword() {
        return CompletionSite::Expression;
    }
    let binding = name
        .checked_sub(1)
        .map(|k| tokens[k].kind)
        .unwrap_or(TokenKind::EOF);
    if matches!(
        binding,
        TokenKind::Let | TokenKind::Const | TokenKind::Var | TokenKind::Using
    ) {
        return CompletionSite::Type;
    }
    if is_declaration_slot(tokens, name) {
        return CompletionSite::Type;
    }
    match innermost_open_paren(tokens, name) {
        Some(open) if is_parameter_list_open(tokens, open) => CompletionSite::Type,
        Some(_) | None => CompletionSite::Expression,
    }
}

pub(super) fn after_eq(tokens: &[&TokenRecord], eq: usize) -> CompletionSite {
    let mut i = eq;
    if i == 0 {
        return CompletionSite::Expression;
    }
    i -= 1;
    if matches!(tokens[i].kind, TokenKind::RAngle) {
        let mut depth = 0usize;
        loop {
            let kind = tokens[i].kind;
            if matches!(kind, TokenKind::RAngle) {
                depth += 1;
            } else if matches!(kind, TokenKind::LAngle) {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            if i == 0 {
                return CompletionSite::Expression;
            }
            i -= 1;
        }
        if i == 0 {
            return CompletionSite::Expression;
        }
        i -= 1;
    }
    if tokens[i].kind == TokenKind::Identifier && i > 0 && tokens[i - 1].kind == TokenKind::Type {
        return CompletionSite::Type;
    }
    CompletionSite::Expression
}

pub(super) fn after_angle(
    state: &DocumentState,
    tokens: &[&TokenRecord],
    last: usize,
) -> CompletionSite {
    let mut depth = 0usize;
    for i in (0..=last).rev() {
        let kind = tokens[i].kind;
        if matches!(kind, TokenKind::RAngle) {
            depth += 1;
        } else if matches!(kind, TokenKind::LAngle) {
            if depth == 0 {
                let Some(owner) = i.checked_sub(1) else {
                    return CompletionSite::Expression;
                };
                let name = state.lexeme(tokens[owner]);
                let names_a_type = matches!(
                    state.symbol_map.get(name),
                    Some(
                        SymbolKind::Class
                            | SymbolKind::Interface
                            | SymbolKind::TypeAlias
                            | SymbolKind::Enum
                    )
                );
                return if tokens[owner].kind == TokenKind::Identifier && names_a_type {
                    CompletionSite::Type
                } else {
                    CompletionSite::Expression
                };
            }
            depth -= 1;
        } else if matches!(
            kind,
            TokenKind::LBrace | TokenKind::RBrace | TokenKind::Semicolon | TokenKind::LParen
        ) {
            return CompletionSite::Expression;
        }
    }
    CompletionSite::Expression
}

pub(super) fn after_union(
    state: &DocumentState,
    tokens: &[&TokenRecord],
    last: usize,
) -> CompletionSite {
    let mut i = last;
    while i > 0 {
        let kind = tokens[i - 1].kind;
        let operand = matches!(
            kind,
            TokenKind::Identifier
                | TokenKind::Pipe
                | TokenKind::Amp
                | TokenKind::Dot
                | TokenKind::LBracket
                | TokenKind::RBracket
                | TokenKind::Question
                | TokenKind::Null
                | TokenKind::Str
        );
        if !operand {
            break;
        }
        i -= 1;
    }
    if i == 0 {
        return CompletionSite::Expression;
    }
    if classify(state, tokens, i - 1) == CompletionSite::Type {
        CompletionSite::Type
    } else {
        CompletionSite::Expression
    }
}
