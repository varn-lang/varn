use tower_lsp_f::lsp_types::{
    CodeAction, CodeActionKind, CodeActionResponse, Position, Range, TextEdit, Uri,
};

use crate::document::DocumentState;

pub const KIND: &str = "source.fixAll.vn";

fn requested(only: Option<&[CodeActionKind]>) -> bool {
    let Some(only) = only else {
        return false;
    };
    only.iter().any(|wanted| {
        let wanted = wanted.as_str();
        KIND == wanted || KIND.starts_with(&format!("{wanted}."))
    })
}

fn position_at(text: &str, offset: usize) -> Position {
    let before = &text[..offset];
    let line = before.matches('\n').count();
    let start = before.rfind('\n').map_or(0, |at| at + 1);
    Position {
        line: line as u32,
        character: text[start..offset].encode_utf16().count() as u32,
    }
}

fn common_prefix(a: &str, b: &str) -> usize {
    a.chars()
        .zip(b.chars())
        .take_while(|(left, right)| left == right)
        .map(|(left, _)| left.len_utf8())
        .sum()
}

fn common_suffix(a: &str, b: &str) -> usize {
    a.chars()
        .rev()
        .zip(b.chars().rev())
        .take_while(|(left, right)| left == right)
        .map(|(left, _)| left.len_utf8())
        .sum()
}

fn minimal_edit(before: &str, after: &str) -> TextEdit {
    let prefix = common_prefix(before, after);
    let room = before.len().min(after.len()) - prefix;
    let suffix = common_suffix(&before[prefix..], &after[prefix..]).min(room);
    let old_end = before.len() - suffix;
    let new_end = after.len() - suffix;
    TextEdit {
        range: Range {
            start: position_at(before, prefix),
            end: position_at(before, old_end),
        },
        new_text: after[prefix..new_end].to_string(),
    }
}

pub fn generate_fix_all_action(
    state: &DocumentState,
    session: &vn_pipeline::Session,
    uri: &Uri,
    only: Option<&[CodeActionKind]>,
) -> Option<CodeActionResponse> {
    if !requested(only) {
        return None;
    }
    let fixable = state
        .diagnostics
        .iter()
        .any(|d| d.code.is_some_and(|c| c.is_lint()) && !d.suggestions.is_empty());
    if !fixable {
        return None;
    }
    let policy = session.lint_policy().ok()?;
    let path = crate::document::uri_to_path(&state.uri);
    let (fixed, applied) = session.fix(&path, &state.source, &policy).ok()?;
    if applied == 0 {
        return None;
    }
    let edit = minimal_edit(&state.source, &fixed);
    Some(CodeActionResponse::CodeAction(CodeAction {
        title: "Fix all auto-fixable lint problems".to_string(),
        kind: Some(CodeActionKind::new(KIND)),
        diagnostics: None,
        edit: Some(crate::features::workspace_edit::doc_edits(vec![(
            uri.clone(),
            vec![edit],
        )])),
        command: None,
        is_preferred: Some(false),
        disabled: None,
        tags: None,
        data: None,
    }))
}
