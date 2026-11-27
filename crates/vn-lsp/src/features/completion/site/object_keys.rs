use vn_core::TokenKind;

use crate::document::{DocumentState, TokenRecord};

use super::*;
pub(super) fn object_keys(
    state: &DocumentState,
    tokens: &[&TokenRecord],
    last: usize,
) -> Option<CompletionSite> {
    let mut depth = 0usize;
    let mut open = None;
    for i in (0..=last).rev() {
        let kind = tokens[i].kind;
        if matches!(
            kind,
            TokenKind::RBrace | TokenKind::RParen | TokenKind::RBracket
        ) {
            depth += 1;
        } else if matches!(
            kind,
            TokenKind::LBrace | TokenKind::LParen | TokenKind::LBracket
        ) {
            if depth == 0 {
                if kind == TokenKind::LBrace {
                    open = Some(i);
                }
                break;
            }
            depth -= 1;
        }
    }
    let open = open?;
    if open < 3
        || tokens[open - 1].kind != TokenKind::Eq
        || tokens[open - 2].kind != TokenKind::Identifier
        || tokens[open - 3].kind != TokenKind::Colon
    {
        return None;
    }

    let mut present = Vec::new();
    let mut nested = 0usize;
    for i in open + 1..=last {
        let kind = tokens[i].kind;
        if matches!(
            kind,
            TokenKind::LBrace | TokenKind::LParen | TokenKind::LBracket
        ) {
            nested += 1;
        } else if matches!(
            kind,
            TokenKind::RBrace | TokenKind::RParen | TokenKind::RBracket
        ) {
            nested = nested.saturating_sub(1);
        } else if nested == 0
            && kind == TokenKind::Identifier
            && i < last
            && tokens[i + 1].kind == TokenKind::Colon
        {
            present.push(state.lexeme(tokens[i]).to_owned());
        }
    }
    Some(CompletionSite::ObjectKeys {
        type_name: state.lexeme(tokens[open - 2]).to_owned(),
        present,
    })
}
