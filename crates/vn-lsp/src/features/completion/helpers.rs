use super::*;
use tower_lsp_f::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};

use crate::document::DocumentState;
use vn_sem::semantic_info::ResolvedMemberKind;

pub(super) fn get_word_prefix(source: &str, line: u32, col: u32) -> Option<String> {
    let line_str = source.lines().nth(line as usize)?;
    let col_idx = (col as usize).min(line_str.len());
    let prefix = &line_str[..col_idx];
    let word = prefix
        .rsplit(|c: char| !c.is_alphanumeric() && c != '_')
        .next()?
        .trim();
    if word.is_empty() {
        None
    } else {
        Some(word.to_string())
    }
}

pub(super) fn append_primitive_types(items: &mut Vec<CompletionItem>) {
    for primitive in vn_core::LangPrimitive::ALL {
        let name = primitive.name();
        if items.iter().any(|item| item.label == name) {
            continue;
        }
        items.push(CompletionItem {
            label: name.to_owned(),
            kind: Some(CompletionItemKind::Keyword),
            detail: Some("primitive type".to_owned()),
            sort_text: Some(format!("2_0_{name}")),
            ..Default::default()
        });
    }
}

pub(super) fn keyword_contexts(label: &str) -> u8 {
    keywords::KEYWORDS
        .iter()
        .find(|entry| entry.label == label)
        .map_or(0, |entry| entry.contexts)
}

pub(super) fn restrict_to_site(
    items: Vec<CompletionItem>,
    scope_n: usize,
    site: &site::CompletionSite,
) -> (Vec<CompletionItem>, usize) {
    use site::CompletionSite as Site;
    let (scope_kinds, keyword_mask): (Option<&[CompletionItemKind]>, u8) = match site {
        Site::Statement => (None, keywords::STATEMENT | keywords::EXPRESSION),
        Site::Expression | Site::ObjectKeys { .. } => (None, keywords::EXPRESSION),
        Site::ClassBody => (Some(&[]), keywords::MEMBER),
        Site::InterfaceBody => (Some(&[]), keywords::INTERFACE_MEMBER),
        Site::EnumBody => (Some(&[]), 0),
        Site::AfterExport => (Some(&[]), keywords::EXPORT),
        Site::Type => (
            Some(&[
                CompletionItemKind::Class,
                CompletionItemKind::Interface,
                CompletionItemKind::Enum,
                CompletionItemKind::TypeParameter,
                CompletionItemKind::Module,
            ]),
            keywords::TYPE,
        ),
        Site::NewTarget | Site::ExtendsClass => (Some(&[CompletionItemKind::Class]), 0),
        Site::ExtendsInterface | Site::Implements => (Some(&[CompletionItemKind::Interface]), 0),
    };
    let mut kept = Vec::with_capacity(items.len());
    let mut kept_scope = 0;
    for (index, item) in items.into_iter().enumerate() {
        let is_scope = index < scope_n;
        let keep = if is_scope {
            scope_kinds.is_none_or(|kinds| item.kind.is_some_and(|kind| kinds.contains(&kind)))
        } else {
            keyword_contexts(&item.label) & keyword_mask != 0
        };
        if keep {
            if is_scope {
                kept_scope += 1;
            }
            kept.push(item);
        }
    }
    (kept, kept_scope)
}

pub(super) fn object_key_completions(
    state: &DocumentState,
    type_name: &str,
    present: &[String],
) -> Vec<CompletionItem> {
    let ty = state.db.named_type(type_name);
    state
        .members_of_type(&ty)
        .into_iter()
        .filter(|m| !m.is_static)
        .filter(|m| m.kind != ResolvedMemberKind::Constructor)
        .filter(|m| !present.iter().any(|key| key.as_str() == m.name.as_ref()))
        .map(|m| CompletionItem {
            label: m.name.to_string(),
            kind: Some(CompletionItemKind::Field),
            detail: Some(state.ty_text(&m.ty)),
            insert_text: Some(format!("{}: $0", m.name)),
            insert_text_format: Some(InsertTextFormat::Snippet),
            sort_text: Some(format!("0_{}", m.name)),
            ..Default::default()
        })
        .collect()
}
