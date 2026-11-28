use tower_lsp_f::lsp_types::{Position, Range, TextEdit};

fn canonical(source: &str) -> Option<String> {
    let formatted = vn_fmt::format_source(source, "<buffer>").ok()?;
    (formatted != source).then_some(formatted)
}

pub fn build_formatting(source: &str) -> Option<Vec<TextEdit>> {
    let formatted = canonical(source)?;
    let line_count = source.lines().count() as u32;
    let last_len = source.lines().last().map(|l| l.len() as u32).unwrap_or(0);
    Some(vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: line_count.max(1) - 1,
                character: last_len,
            },
        },
        new_text: formatted,
    }])
}

pub fn build_range_edits(source: &str, start_line: u32, end_line: u32) -> Option<Vec<TextEdit>> {
    let formatted = canonical(source)?;
    let src_lines: Vec<&str> = source.lines().collect();
    let fmt_lines: Vec<&str> = formatted.lines().collect();
    if src_lines.len() != fmt_lines.len() {
        return build_formatting(source);
    }
    let mut edits = Vec::new();
    for idx in start_line..=end_line.min((src_lines.len() as u32).saturating_sub(1)) {
        let old = src_lines[idx as usize];
        let new = fmt_lines[idx as usize];
        if old != new {
            edits.push(TextEdit {
                range: Range {
                    start: Position {
                        line: idx,
                        character: 0,
                    },
                    end: Position {
                        line: idx,
                        character: old.len() as u32,
                    },
                },
                new_text: new.to_string(),
            });
        }
    }
    if edits.is_empty() {
        None
    } else {
        Some(edits)
    }
}
