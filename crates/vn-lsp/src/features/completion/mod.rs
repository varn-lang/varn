mod autoimport;
mod calls;
mod columns;
mod helpers;
mod imports;
mod keywords;
mod lexical;
pub mod members;
mod patching;
pub mod postfix;
mod preview;
pub mod probe;
pub mod receiver_literals;
pub mod receivers;
pub mod reflection;
mod response;
pub(crate) mod scope;
mod site;

use tower_lsp_f::lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
};

use crate::document::DocumentState;

pub use imports::{
    build_import_completions, build_module_export_completions, resolve_relative_module_debug,
};
pub use members::build_member_completions;
pub use postfix::build_postfix_completions;
pub use receivers::{dot_receiver, pattern_receiver, DotVerdict};
pub use reflection::build_reflection_completions;

use helpers::{append_primitive_types, get_word_prefix, object_key_completions, restrict_to_site};
pub use patching::{patched_source_for_member, patched_source_for_word};
use preview::compact_trigger;
pub use preview::preview_items;
pub use response::build_completion_response;

pub fn build_completions(
    state: &DocumentState,
    line: u32,
    col: u32,
    cursor_offset: u32,
) -> (Vec<CompletionItem>, usize) {
    let mut items: Vec<CompletionItem> = Vec::with_capacity(160);

    let prefix = get_word_prefix(&state.source, line, col).unwrap_or_default();
    let scope_items = scope::build_scope_completions(state, cursor_offset, &prefix);
    let scope_n = scope_items.len();
    items.extend(scope_items);

    for (idx, kw) in keywords::KEYWORDS.iter().enumerate() {
        items.push(CompletionItem {
            label: kw.label.into(),
            kind: Some(CompletionItemKind::Keyword),
            detail: kw.detail.map(str::to_owned),
            documentation: kw.doc.map(|d| {
                Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: d.into(),
                })
            }),
            insert_text: kw.snippet.map(str::to_owned),
            insert_text_format: if kw.snippet.is_some() {
                Some(InsertTextFormat::Snippet)
            } else {
                None
            },
            sort_text: Some(format!("3_{:02}_{}", idx, kw.label)),
            ..Default::default()
        });
    }

    (items, scope_n)
}
