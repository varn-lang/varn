mod facts;
mod fixes;
mod naming;
mod rules;
mod scope;

pub use facts::TypeFacts;
use std::collections::HashSet;
use vn_core::ast::{AstArena, Program};
use vn_core::{
    AtomInterner, Diagnostic, ErrorCode, SourceLocation, SourceRange, Suggestion, Trivia,
};

pub struct Source<'a> {
    pub text: &'a str,
    lines: Vec<usize>,
}

impl<'a> Source<'a> {
    fn new(text: &'a str) -> Self {
        let mut lines = vec![0];
        lines.extend(text.match_indices('\n').map(|(at, _)| at + 1));
        Self { text, lines }
    }

    pub fn location(&self, offset: u32) -> SourceLocation {
        let offset = (offset as usize).min(self.text.len());
        let line = self.lines.partition_point(|start| *start <= offset);
        let start = self.lines[line - 1];
        let column = self.text[start..offset].chars().count();
        SourceLocation::new(line as u32, column as u32, offset as u32)
    }

    pub fn range(&self, start: u32, end: u32) -> SourceRange {
        SourceRange::new(self.location(start), self.location(end))
    }
}

pub fn rule_codes() -> &'static [ErrorCode] {
    &[
        ErrorCode::LintUnusedBinding,
        ErrorCode::LintPreferConst,
        ErrorCode::LintEmptyCatch,
        ErrorCode::LintConstantCondition,
        ErrorCode::LintNamingConvention,
        ErrorCode::LintSelfAssign,
    ]
}

pub fn rule_of(name: &str) -> Option<ErrorCode> {
    rule_codes()
        .iter()
        .copied()
        .find(|code| code.name() == name || code.to_string().eq_ignore_ascii_case(name))
}

pub struct Unit<'a> {
    pub text: &'a str,
    pub filename: &'a str,
    pub program: &'a Program,
    pub arena: &'a AstArena,
    pub interner: &'a AtomInterner,
    pub trivia: &'a [Trivia],
}

pub fn lint(unit: &Unit<'_>, facts: &TypeFacts) -> Vec<Diagnostic> {
    let source = Source::new(unit.text);
    let mut found = rules::run(
        &source,
        unit.program,
        unit.arena,
        unit.interner,
        unit.trivia,
        facts,
    );
    let ignored = suppressions(&source, unit.trivia);
    found.retain(|diagnostic| {
        !ignored.iter().any(|(line, code)| {
            *line == diagnostic.range.start.line && code.is_none_or(|c| c == diagnostic.code)
        })
    });
    found.sort_by_key(|d| (d.range.start.offset, d.code.code_number()));
    found
        .into_iter()
        .map(|d| d.with_file(unit.filename.to_string()))
        .collect()
}

fn suppressions(source: &Source<'_>, trivia: &[Trivia]) -> HashSet<(u32, Option<ErrorCode>)> {
    const MARKER: &str = "vn-lint-ignore";
    let mut ignored = HashSet::new();
    for item in trivia {
        let start = item.range.start.offset as usize;
        let end = item.range.end.offset as usize;
        let text = &source.text[start..end];
        let Some(at) = text.find(MARKER) else {
            continue;
        };
        let names = text[at + MARKER.len()..]
            .trim_end_matches("*/")
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();
        let line = source.location(item.range.start.offset).line;
        let own_line = source.text[..start]
            .rsplit('\n')
            .next()
            .is_some_and(|before| before.trim().is_empty());
        let targets = if own_line { vec![line + 1] } else { vec![line] };
        for target in targets {
            if names.is_empty() {
                ignored.insert((target, None));
            }
            for name in &names {
                if let Some(code) = rule_of(name) {
                    ignored.insert((target, Some(code)));
                }
            }
        }
    }
    ignored
}

pub fn apply_fixes(text: &str, diagnostics: &[Diagnostic]) -> (String, usize) {
    let mut edits: Vec<(usize, usize, &str)> = diagnostics
        .iter()
        .flat_map(|d| d.suggestions.iter())
        .filter_map(|s: &Suggestion| match (&s.range, &s.replacement) {
            (Some(range), Some(text)) => Some((
                range.start.offset as usize,
                range.end.offset as usize,
                text.as_str(),
            )),
            _ => None,
        })
        .collect();
    edits.sort_by_key(|(start, _, _)| *start);
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut applied = 0;
    for (start, end, replacement) in edits {
        if start < cursor {
            continue;
        }
        out.push_str(&text[cursor..start]);
        out.push_str(replacement);
        cursor = end;
        applied += 1;
    }
    out.push_str(&text[cursor..]);
    (out, applied)
}
