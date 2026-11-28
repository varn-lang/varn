mod chains;
mod doc;
mod facts;
mod layout;
mod printer;
mod spacing;
mod tokens;

use std::fmt;
use vn_core::{AtomInterner, Token, TokenKind};

pub const LINE_WIDTH: usize = 100;
pub const INDENT_WIDTH: usize = 4;

#[derive(Debug)]
pub enum FormatError {
    Syntax(String),
    Unstable,
    Altered(String),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::Syntax(message) => write!(f, "{message}"),
            FormatError::Unstable => {
                write!(f, "formatter bug: the output changes when formatted again")
            }
            FormatError::Altered(message) => write!(f, "formatter bug: {message}"),
        }
    }
}

fn located(error: &vn_core::Diagnostic) -> String {
    format!(
        "{}:{}: {}",
        error.range.start.line, error.range.start.column, error.message
    )
}

pub fn render_unchecked(source: &str, filename: &str) -> Result<String, FormatError> {
    render(source, filename).map(|rendered| rendered.text)
}

struct Rendered {
    text: String,
    split_tokens: Vec<u32>,
}

fn render(source: &str, filename: &str) -> Result<Rendered, FormatError> {
    let (tokens, buffer, errors, trivia) = vn_lexer::scan_with_trivia(source, filename);
    if let Some(error) = errors.first() {
        return Err(FormatError::Syntax(located(error)));
    }
    let (program, diagnostics, arena, _) =
        vn_parser::parse_partial(tokens.clone(), buffer, filename, AtomInterner::new());
    if let Some(error) = diagnostics.iter().next() {
        return Err(FormatError::Syntax(located(error)));
    }
    let Some(stream) = tokens::TokenStream::build(source, &tokens, &trivia, &program.split_tokens)
    else {
        return Err(FormatError::Syntax("unbalanced brackets".to_string()));
    };
    let facts = facts::collect(&stream, &program, &arena);
    let document = layout::Layout::new(&stream, &facts).root();
    Ok(Rendered {
        text: printer::Printer::new(LINE_WIDTH, INDENT_WIDTH).print(&document),
        split_tokens: program.split_tokens,
    })
}

fn significant(tokens: &[Token], source: &str, split_tokens: &[u32]) -> Vec<(TokenKind, String)> {
    let mut out: Vec<(TokenKind, String)> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::EOF {
            continue;
        }
        let closes = tokens.get(index + 1).is_some_and(|next| {
            matches!(
                next.kind,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace
            )
        });
        if matches!(token.kind, TokenKind::Comma | TokenKind::Semicolon) && closes {
            continue;
        }
        let start = token.range.start.offset as usize;
        let end = token.range.end.offset as usize;
        if split_tokens
            .binary_search(&token.range.start.offset)
            .is_ok()
        {
            for (piece, kind) in token.kind.angle_pieces().iter().enumerate() {
                out.push((*kind, source[start + piece..start + piece + 1].to_string()));
            }
        } else {
            out.push((token.kind, source[start..end].to_string()));
        }
    }
    out
}

pub fn format_source(original: &str, filename: &str) -> Result<String, FormatError> {
    const CRLF: &str = "\r\n";
    let crlf = original.contains(CRLF);
    let normalized;
    let source = if crlf {
        normalized = original.replace(CRLF, "\n");
        normalized.as_str()
    } else {
        original
    };
    let formatted = check(source, filename)?;
    Ok(if crlf {
        formatted.replace('\n', CRLF)
    } else {
        formatted
    })
}

fn comments(source: &str, filename: &str) -> Vec<String> {
    let (tokens, _, _, trivia) = vn_lexer::scan_with_trivia(source, filename);
    let mut found: Vec<(u32, String)> = trivia
        .iter()
        .map(|item| (item.range.start.offset, item.range))
        .map(|(at, range)| {
            (
                at,
                source[range.start.offset as usize..range.end.offset as usize].to_string(),
            )
        })
        .collect();
    for token in tokens.iter().filter(|t| t.kind == TokenKind::DocComment) {
        let start = token.range.start.offset as usize;
        found.push((
            token.range.start.offset,
            source[start..token.range.end.offset as usize].to_string(),
        ));
    }
    found.sort_by_key(|(at, _)| *at);
    found
        .into_iter()
        .map(|(_, text)| text.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

fn check(source: &str, filename: &str) -> Result<String, FormatError> {
    let formatted = render(source, filename)?;
    let again = render(&formatted.text, filename)?;
    let (before, _, _) = vn_lexer::scan(source, filename);
    let (after, _, _) = vn_lexer::scan(&formatted.text, filename);
    if significant(&before, source, &formatted.split_tokens)
        != significant(&after, &formatted.text, &again.split_tokens)
    {
        return Err(FormatError::Altered(
            "the formatted code does not have the same tokens as the source".to_string(),
        ));
    }
    if comments(source, filename) != comments(&formatted.text, filename) {
        return Err(FormatError::Altered(
            "the formatted code does not keep the comments of the source".to_string(),
        ));
    }
    if again.text != formatted.text {
        return Err(FormatError::Unstable);
    }
    Ok(formatted.text)
}
