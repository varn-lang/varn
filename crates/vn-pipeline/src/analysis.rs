use vn_core::{DiagnosticBag, Trivia};

use crate::stages::lex::Tokens;
use crate::stages::parse::Parsed;
use crate::{Session, Source};

pub struct Analysis {
    pub tokens: Tokens,
    pub trivia: Vec<Trivia>,
    pub parsed: Parsed,
    pub checked: vn_sem::output::CheckResult,
    pub syntax: DiagnosticBag,
}

impl Session {
    pub fn analyze(&self, source: &Source) -> Analysis {
        let (tokens, trivia, mut syntax) =
            crate::stages::lex::scan_tolerant(source.text(), source.path());
        let (parsed, syntactic) = crate::stages::parse::parse_tolerant(
            tokens.tokens.clone(),
            tokens.lexeme_buf.clone(),
            source.path(),
        );
        syntax.extend(syntactic);
        let checked = crate::stages::check::check_tolerant(
            &parsed.program,
            &parsed.arena,
            parsed.interner.clone(),
            self.resolver(),
        );
        Analysis {
            tokens,
            trivia,
            parsed,
            checked,
            syntax,
        }
    }
}
