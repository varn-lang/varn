use crate::scope::{Kind, Scopes};
use crate::Source;
use vn_core::Suggestion;

pub(crate) fn unused_fix(
    source: &Source<'_>,
    scopes: &Scopes<'_>,
    index: usize,
    name: &str,
    range: vn_core::SourceRange,
) -> Option<Suggestion> {
    let binding = &scopes.bindings[index];
    if let Some((declaration, position)) = binding.import {
        let info = &scopes.imports[declaration];
        let unused_all = info.specifiers.len() == 1
            || scopes
                .bindings
                .iter()
                .filter(|other| other.import.is_some_and(|(d, _)| d == declaration))
                .all(|other| other.reads == 0);
        let text = source.text;
        if unused_all {
            let mut end = info.end as usize;
            if text[end..].starts_with(';') {
                end += 1;
            }
            if text[end..].starts_with("\r\n") {
                end += 2;
            } else if text[end..].starts_with('\n') {
                end += 1;
            }
            return Some(
                Suggestion::new("Remove the unused import")
                    .with_range(source.range(info.start, end as u32))
                    .replace(""),
            );
        }
        let (start, end) = info.specifiers[position];
        let (mut from, mut to) = (start as usize, end as usize);
        let after = text[to..].trim_start();
        if after.starts_with(',') {
            to = text.len() - after.len() + 1;
            to += text[to..].len() - text[to..].trim_start().len();
        } else {
            let before = text[..from].trim_end();
            if before.ends_with(',') {
                from = before.len() - 1;
            }
        }
        return Some(
            Suggestion::new(format!("Remove `{name}` from the import"))
                .with_range(source.range(from as u32, to as u32))
                .replace(""),
        );
    }
    if let Some((start, body)) = scopes.catch_heads.get(&index).copied()
        && binding.writes == 0
    {
        let head = source.text.get(start as usize..body as usize)?;
        if head.starts_with('(') {
            return Some(
                Suggestion::new(format!("Remove the unused `{name}` binding"))
                    .with_range(source.range(start, body))
                    .replace(""),
            );
        }
    }
    if binding.renamable
        && binding.writes == 0
        && matches!(
            binding.kind,
            Kind::Let | Kind::Const | Kind::Param | Kind::CatchParam | Kind::MatchBinding
        )
    {
        let replacement = if binding.shorthand {
            format!("{name}: _{name}")
        } else {
            format!("_{name}")
        };
        return Some(
            Suggestion::new(format!("Rename to `_{name}`"))
                .with_range(range)
                .replace(replacement),
        );
    }
    None
}
