use super::*;
use rustc_hash::FxHashSet;
use std::time::Instant;
use tower_lsp_f::lsp_types::{
    CompletionItemKind, CompletionList, CompletionResponse, Position, Range, TextEdit,
};

use crate::document::{
    import_path_at, named_import_module_at, named_imported_names_at, DocumentState,
};
use crate::index::ProjectIndex;

pub fn build_completion_response(
    state: &DocumentState,
    resolver: &vn_pipeline::ModuleResolver,
    line: u32,
    col: u32,
    trigger_char: Option<&str>,
    trigger_kind: String,
    index: Option<&ProjectIndex>,
) -> (Option<CompletionResponse>, Option<String>) {
    let start = Instant::now();
    let ms = || start.elapsed().as_millis();
    let col_utf16 = col;
    let col = columns::byte_column(&state.source, line, col_utf16);
    let prefix = get_word_prefix(&state.source, line, col).unwrap_or_default();
    let trig = compact_trigger(trigger_char, &trigger_kind);
    let lc = format!("{}:{}", line + 1, col + 1);
    if let Some(ctx) = import_path_at(&state.source, &state.tokens, line, col) {
        let mut items = build_import_completions(&ctx.prefix, &state.uri);
        let is_relative = ctx.prefix.starts_with('.') || ctx.specifier.starts_with('.');
        for item in &mut items {
            let full_label = item.label.clone();
            if is_relative {
                let insert_text = imports::import_insert_text(&full_label);
                item.detail = Some(full_label.clone());
                item.label = insert_text.clone();
                item.kind = Some(CompletionItemKind::Module);
                item.filter_text = Some(full_label);
                item.text_edit = None;
                item.insert_text = Some(insert_text);
            } else {
                item.filter_text = Some(full_label.clone());
                item.text_edit = Some(tower_lsp_f::lsp_types::CompletionItemTextEdit::TextEdit(
                    TextEdit {
                        range: Range {
                            start: Position {
                                line,
                                character: columns::utf16_column(
                                    &state.source,
                                    line,
                                    ctx.content_start_col,
                                ),
                            },
                            end: Position {
                                line,
                                character: col_utf16,
                            },
                        },
                        new_text: full_label,
                    },
                ));
                item.insert_text = None;
            }
        }
        let log = format!(
            "cmp {lc} trg={trig} pre={prefix:?} ipath spec={:?} items={} {}ms",
            ctx.specifier,
            items.len(),
            ms(),
        );
        let resp = CompletionResponse::CompletionList(CompletionList {
            is_incomplete: true,
            item_defaults: None,
            apply_kind: None,
            items,
        });
        return (Some(resp), Some(log));
    }

    if let Some(module_path) = named_import_module_at(&state.source, &state.tokens, line, col) {
        let already_imported = named_imported_names_at(&state.source, &state.tokens, line, col);
        let doc_uri = state.uri.clone();
        let all = build_module_export_completions(resolver, &module_path, &doc_uri);
        let total = all.len();
        let items: Vec<_> = all
            .into_iter()
            .filter(|item| !already_imported.contains(&item.label))
            .collect();
        let log = format!(
            "cmp {lc} trg={trig} pre={prefix:?} nimport mod={module_path:?} exp={total} kept={} show=[{}] {}ms",
            items.len(),
            preview_items(&items, 5),
            ms()
        );
        return (
            Some(CompletionResponse::CompletionItemList(items)),
            Some(log),
        );
    }

    let cursor_offset = state.offset_at_line_col(line, col_utf16) as usize;
    let context = lexical::lexical_context(&state.source, cursor_offset);
    if context != lexical::LexicalContext::Code {
        return (
            None,
            Some(format!("cmp {lc} trg={trig} suppressed={context:?}")),
        );
    }

    if let Some((receiver_name, receiver_offset)) =
        reflection::colon_colon_receiver(state, line, col, trigger_char)
    {
        let items = reflection::build_reflection_completions(
            state,
            &receiver_name,
            receiver_offset,
            &prefix,
        );
        let log = format!(
            "cmp {lc} trg={trig} pre={prefix:?} ccref={receiver_name:?} items={} {}ms",
            items.len(),
            ms()
        );
        return (
            Some(CompletionResponse::CompletionItemList(items)),
            Some(log),
        );
    }

    let verdict = receivers::dot_receiver(state, line, col_utf16, trigger_char);
    let dot_miss = verdict.info.is_none().then(|| {
        format!(
            "dotmiss={}({})",
            verdict.stage,
            verdict.receiver.chars().take(24).collect::<String>()
        )
    });
    if let Some(info) = verdict.info {
        let ty_text = state.db.ty_text(&info.ty);
        let inst = info.is_instance;
        let mut items = build_member_completions(state, info, true, &prefix, cursor_offset as u32);
        let mem = items.len();
        let postfix_items = build_postfix_completions(state, line, col);
        let post = postfix_items.len();
        items.extend(postfix_items);
        let log = format!(
            "cmp {lc} trg={trig} pre={prefix:?} dot via={} recv={:?} ty={ty_text} inst={inst} mem={mem} post={post} tot={} show=[{}] {}ms",
            verdict.stage,
            verdict.receiver,
            items.len(),
            preview_items(&items, 8),
            ms()
        );
        return (
            Some(CompletionResponse::CompletionItemList(items)),
            Some(log),
        );
    }

    let postfix_items = build_postfix_completions(state, line, col);
    if !postfix_items.is_empty() {
        let log = format!(
            "cmp {lc} trg={trig} pre={prefix:?} postfix-only items={} {}ms",
            postfix_items.len(),
            ms()
        );
        return (
            Some(CompletionResponse::CompletionItemList(postfix_items)),
            Some(log),
        );
    }

    if let Some(info) = receivers::pattern_receiver(state, line, col) {
        let ty_text = state.db.ty_text(&info.ty);
        let items = build_member_completions(state, info, false, &prefix, cursor_offset as u32);
        let log = format!(
            "cmp {lc} trg={trig} pre={prefix:?} pattern ty={ty_text} items={} {}ms",
            items.len(),
            ms()
        );
        return (
            Some(CompletionResponse::CompletionItemList(items)),
            Some(log),
        );
    }

    let named_arguments = calls::build_call_argument_completions(state, line, col);

    let site = site::completion_site(state, cursor_offset as u32);
    if let site::CompletionSite::ObjectKeys { type_name, present } = &site {
        let items = object_key_completions(state, type_name, present);
        let log = format!(
            "cmp {lc} trg={trig} pre={prefix:?} object-keys type={type_name} items={} {}ms",
            items.len(),
            ms()
        );
        return (
            Some(CompletionResponse::CompletionItemList(items)),
            Some(log),
        );
    }

    let (mut items, scope_n) = build_completions(state, line, col, cursor_offset as u32);
    let (items_kept, scope_n) = restrict_to_site(items, scope_n, &site);
    items = items_kept;
    if site == site::CompletionSite::Type {
        append_primitive_types(&mut items);
    }
    let named_n = named_arguments.as_ref().map_or(0, Vec::len);
    if let Some(named) = named_arguments {
        items.splice(0..0, named);
    }
    let kw_n = items.len() - scope_n - named_n;

    let mut auto_n = 0;
    if let Some(idx) = index
        && !prefix.is_empty()
    {
        let already_known: FxHashSet<String> = state.symbol_map.keys().cloned().collect();
        let auto = autoimport::build_autoimport_completions(
            &state.source,
            &state.uri,
            idx,
            &already_known,
            Some(&prefix),
        );
        auto_n = auto.len();
        items.extend(auto);
    }

    let line_txt: String = state
        .source
        .lines()
        .nth(line as usize)
        .unwrap_or("")
        .trim()
        .chars()
        .take(64)
        .collect();
    let log = format!(
        "cmp {lc} trg={trig} pre={prefix:?} line={line_txt:?} general scope={scope_n} kw={kw_n} auto={auto_n} tot={} show=[{}] {} {}ms",
        items.len(),
        preview_items(&items, 8),
        dot_miss.unwrap_or_default(),
        ms()
    );
    (
        Some(CompletionResponse::CompletionItemList(items)),
        Some(log),
    )
}
