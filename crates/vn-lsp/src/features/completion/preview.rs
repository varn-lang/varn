pub fn preview_items(items: &[tower_lsp_f::lsp_types::CompletionItem], n: usize) -> String {
    let mut sorted: Vec<_> = items.iter().collect();
    sorted.sort_by(|a, b| {
        a.sort_text
            .as_deref()
            .unwrap_or("~")
            .cmp(b.sort_text.as_deref().unwrap_or("~"))
            .then_with(|| a.label.cmp(&b.label))
    });
    sorted
        .into_iter()
        .take(n)
        .map(|i| {
            let mut s = format!(
                "{}({})",
                i.label.as_str(),
                i.sort_text.as_deref().unwrap_or("~")
            );
            if let Some(te) = &i.text_edit {
                match te {
                    tower_lsp_f::lsp_types::CompletionItemTextEdit::TextEdit(e) => {
                        s.push_str(&format!(
                            "~>{}:{}-{}:{}={}",
                            e.range.start.line,
                            e.range.start.character,
                            e.range.end.line,
                            e.range.end.character,
                            e.new_text
                        ));
                    }
                    tower_lsp_f::lsp_types::CompletionItemTextEdit::InsertReplaceEdit(e) => {
                        s.push_str(&format!(
                            "~>{}:{}-{}:{}={}",
                            e.insert.start.line,
                            e.insert.start.character,
                            e.replace.end.line,
                            e.replace.end.character,
                            e.new_text
                        ));
                    }
                }
            }
            if let Some(f) = &i.filter_text
                && f != &i.label
            {
                s.push_str(&format!("?{f}"));
            }
            s
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn compact_trigger(trigger_char: Option<&str>, trigger_kind: &str) -> String {
    if let Some(c) = trigger_char {
        return format!("'{c}'");
    }
    if trigger_kind.contains("Invoked") {
        "inv".to_owned()
    } else {
        "re".to_owned()
    }
}
