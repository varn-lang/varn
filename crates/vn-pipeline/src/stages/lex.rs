use std::sync::Arc;
use vn_core::{DiagnosticBag, Token, Trivia};

pub struct Tokens {
    pub tokens: Vec<Token>,
    pub lexeme_buf: Arc<[u8]>,
    pub trivia: Vec<Trivia>,
}

pub(crate) fn scan_tolerant(source: &str, path: &str) -> (Tokens, DiagnosticBag) {
    let (tokens, lexeme_buf, errors, trivia) = vn_lexer::scan(source, path);
    let mut diagnostics = DiagnosticBag::new();
    for e in errors {
        diagnostics.push(e);
    }
    (
        Tokens {
            tokens,
            lexeme_buf,
            trivia,
        },
        diagnostics,
    )
}
