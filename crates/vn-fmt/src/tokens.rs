use vn_core::{Token, TokenKind, Trivia, TriviaKind};

#[derive(Clone, Debug)]
pub struct Comment {
    pub text: String,
    pub line: bool,
    pub start: u32,
    pub blank_before: bool,
    pub newline_after: bool,
}

#[derive(Clone, Debug)]
pub struct Tok {
    pub kind: TokenKind,
    pub start: u32,
    pub end: u32,
    pub text: String,
    pub leading: Vec<Comment>,
    pub trailing: Vec<Comment>,
    pub blank_before: bool,
    pub newline_before: bool,
}

pub struct TokenStream {
    pub toks: Vec<Tok>,
    pub open_of: Vec<Option<usize>>,
    pub close_of: Vec<Option<usize>>,
}

fn newlines(source: &str, from: u32, to: u32) -> usize {
    if from >= to {
        return 0;
    }
    source[from as usize..to as usize]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
}

pub fn is_open(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace | TokenKind::QuestionLBracket
    )
}

pub fn is_close(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace
    )
}

impl TokenStream {
    pub fn build(source: &str, tokens: &[Token], trivia: &[Trivia]) -> Option<TokenStream> {
        let mut toks: Vec<Tok> = tokens
            .iter()
            .map(|t| {
                let start = t.range.start.offset;
                let end = t.range.end.offset.max(start);
                Tok {
                    kind: t.kind,
                    start,
                    end,
                    text: source[start as usize..end as usize].to_string(),
                    leading: Vec::new(),
                    trailing: Vec::new(),
                    blank_before: false,
                    newline_before: false,
                }
            })
            .collect();
        if toks.is_empty() {
            return None;
        }

        let mut ordered: Vec<&Trivia> = trivia.iter().collect();
        ordered.sort_by_key(|t| t.range.start.offset);
        for item in ordered {
            let start = item.range.start.offset;
            let end = item.range.end.offset;
            let next = toks.partition_point(|t| t.start < end);
            if next >= toks.len() {
                return None;
            }
            let previous = next.checked_sub(1);
            let line = item.kind == TriviaKind::Line;
            let same_line_as_previous =
                previous.is_some_and(|p| newlines(source, toks[p].end, start) == 0);
            let newline_after = newlines(source, end, toks[next].start) > 0;
            let comment = Comment {
                text: source[start as usize..end as usize].trim_end().to_string(),
                line,
                start,
                blank_before: false,
                newline_after,
            };
            match previous {
                Some(p) if same_line_as_previous && (line || newline_after) => {
                    toks[p].trailing.push(comment)
                }
                _ => toks[next].leading.push(comment),
            }
        }

        for index in 0..toks.len() {
            let previous_end = if index == 0 {
                0
            } else {
                let p = &toks[index - 1];
                p.trailing
                    .last()
                    .map(|c| c.start + c.text.len() as u32)
                    .unwrap_or(p.end)
            };
            let mut cursor = previous_end;
            for comment in &mut toks[index].leading {
                comment.blank_before = newlines(source, cursor, comment.start) >= 2;
                cursor = comment.start + comment.text.len() as u32;
            }
            let first = toks[index]
                .leading
                .first()
                .map(|c| c.start)
                .unwrap_or(toks[index].start);
            let gap_start = if index == 0 { 0 } else { toks[index - 1].end };
            toks[index].blank_before = newlines(source, previous_end, first) >= 2;
            toks[index].newline_before = newlines(source, gap_start, toks[index].start) > 0;
        }

        let mut open_of = vec![None; toks.len()];
        let mut close_of = vec![None; toks.len()];
        let mut stack: Vec<usize> = Vec::new();
        for (index, tok) in toks.iter().enumerate() {
            if is_open(tok.kind) {
                stack.push(index);
            } else if is_close(tok.kind) {
                let open = stack.pop()?;
                open_of[index] = Some(open);
                close_of[open] = Some(index);
            }
        }
        if !stack.is_empty() {
            return None;
        }
        Some(TokenStream {
            toks,
            open_of,
            close_of,
        })
    }

    pub fn index_at_or_after(&self, offset: u32) -> usize {
        self.toks.partition_point(|t| t.start < offset)
    }
}
