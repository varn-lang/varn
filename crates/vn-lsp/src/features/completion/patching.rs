use super::*;

use crate::document::DocumentState;

pub fn patched_source_for_member(
    state: &DocumentState,
    line: u32,
    col: u32,
    trigger_char: Option<&str>,
) -> Option<probe::ProbedSource> {
    let cursor = state.offset_at_line_col(line, col) as usize;
    let before = state.source.get(..cursor)?;
    let after = state.source.get(cursor..)?;
    let continues_word = after
        .chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric() || c == '_');
    let ends_with_access = before.ends_with('.') || before.ends_with("?.");
    if !ends_with_access || continues_word {
        return None;
    }
    let verdict = receivers::dot_receiver(state, line, col, trigger_char);
    if let Some(info) = &verdict.info
        && (!info.is_instance || !matches!(verdict.stage, "spatial" | "scope"))
    {
        return None;
    }
    if let Some(probed) = probe::probed_source(state, cursor as u32) {
        return Some(probed);
    }
    if verdict.info.is_some() {
        return None;
    }
    Some(probe::ProbedSource {
        text: format!("{before}{}{after}", probe::MEMBER_PLACEHOLDER),
        line,
        character: col,
    })
}

pub fn patched_source_for_word(
    state: &DocumentState,
    line: u32,
    col: u32,
) -> Option<probe::ProbedSource> {
    let cursor = state.offset_at_line_col(line, col);
    if !matches!(
        site::completion_site(state, cursor),
        site::CompletionSite::Expression | site::CompletionSite::Statement
    ) {
        return None;
    }
    let cursor = cursor as usize;
    let before = state.source.get(..cursor)?;
    let after = state.source.get(cursor..)?;
    if before.ends_with('.') || before.ends_with("?.") {
        return None;
    }
    if after
        .chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric() || c == '_')
    {
        return None;
    }
    let word_is_keyword = state
        .tokens
        .iter()
        .rfind(|t| t.offset < cursor as u32 && t.end >= cursor as u32)
        .is_some_and(|t| t.kind.is_keyword());
    let word_is_empty = !before
        .chars()
        .next_back()
        .is_some_and(|c| c.is_alphanumeric() || c == '_');
    let broken_here = state
        .diagnostics
        .iter()
        .any(|d| d.kind == vn_core::DiagnosticKind::Error && d.line <= line && line <= d.end_line);
    if !(word_is_keyword || (word_is_empty && broken_here)) {
        return None;
    }
    Some(probe::ProbedSource {
        text: format!("{before}{}{after}", probe::WORD_PLACEHOLDER),
        line,
        character: col,
    })
}
