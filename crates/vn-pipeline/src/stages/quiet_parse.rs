use crate::PipelineError;
use vn_core::ast::{AstArena, Program};

pub(crate) fn parse_module(
    source: &str,
    path: &str,
) -> Result<(Program, AstArena, vn_core::AtomInterner), PipelineError> {
    let (tokens, _, lex_diagnostics) = crate::stages::lex::scan_tolerant(source, path);
    let (parsed, mut diagnostics) =
        crate::stages::parse::parse_tolerant(tokens.tokens, tokens.lexeme_buf, path);
    diagnostics.extend(lex_diagnostics);
    if !diagnostics.is_empty() {
        return Err(PipelineError::diagnostics(path, source, diagnostics));
    }
    Ok((parsed.program, parsed.arena, parsed.interner))
}
