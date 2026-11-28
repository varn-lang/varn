use crate::chains::ChainKind;
use crate::doc::Doc;
use crate::facts::Facts;
use crate::spacing::{ends_operand, Context, Gap};
use crate::tokens::{is_open, Comment, TokenStream};
use std::collections::BTreeSet;
use vn_core::TokenKind as K;

pub struct Layout<'a> {
    pub stream: &'a TokenStream,
    pub facts: &'a Facts,
    template_depth: usize,
}

fn comment_doc(comment: &Comment) -> Doc {
    if !comment.text.contains('\n') {
        return Doc::text(comment.text.clone());
    }
    let mut parts = Vec::new();
    for (index, line) in comment.text.split('\n').enumerate() {
        let trimmed = line.trim();
        if index > 0 {
            parts.push(Doc::Hard);
            if trimmed.starts_with('*') {
                parts.push(Doc::Space);
            }
        }
        parts.push(Doc::text(trimmed.to_string()));
    }
    Doc::Concat(parts)
}

impl<'a> Layout<'a> {
    pub fn new(stream: &'a TokenStream, facts: &'a Facts) -> Self {
        Self {
            stream,
            facts,
            template_depth: 0,
        }
    }

    fn flat(&self) -> bool {
        self.template_depth > 0
    }

    fn leading(&self, index: usize) -> Doc {
        let mut parts = Vec::new();
        for (position, comment) in self.stream.toks[index].leading.iter().enumerate() {
            if position > 0 && comment.blank_before {
                parts.push(Doc::Hard);
            }
            parts.push(comment_doc(comment));
            parts.push(if comment.line || comment.newline_after {
                Doc::Hard
            } else {
                Doc::Space
            });
        }
        Doc::Concat(parts)
    }

    fn trailing(&self, index: usize) -> Doc {
        let mut parts = Vec::new();
        for comment in &self.stream.toks[index].trailing {
            parts.push(Doc::Suffix(format!(" {}", comment.text)));
            if comment.line {
                parts.push(Doc::BreakParent);
            }
        }
        Doc::Concat(parts)
    }

    fn core(&self, index: usize) -> Doc {
        let tok = &self.stream.toks[index];
        if tok.kind == K::DocComment {
            return comment_doc(&Comment {
                text: tok.text.clone(),
                line: false,
                start: tok.start,
                blank_before: false,
                newline_after: false,
            });
        }
        if tok.text.contains('\n') {
            Doc::Raw(tok.text.clone())
        } else {
            Doc::text(tok.text.clone())
        }
    }

    fn token(&self, index: usize) -> Doc {
        Doc::Concat(vec![
            self.leading(index),
            self.core(index),
            self.trailing(index),
        ])
    }

    fn separation(&self, next: usize) -> Doc {
        let toks = &self.stream.toks;
        let previous = next - 1;
        if toks[previous].kind == K::DocComment || self.facts.decorator_ends.contains(&previous) {
            return if toks[next].newline_before {
                Doc::Hard
            } else {
                Doc::Space
            };
        }
        let had_space = toks[next].start > toks[previous].end;
        let context = Context {
            toks,
            ternary: &self.facts.ternary,
            comparisons: &self.facts.comparisons,
        };
        match context.gap(next, had_space) {
            Gap::None => Doc::Nil,
            Gap::Space => Doc::Space,
        }
    }

    pub fn sequence(&mut self, lo: usize, hi: usize) -> Doc {
        let toks = &self.stream.toks;
        let mut parts = Vec::new();
        let mut index = lo;
        let mut first = true;
        while index < hi {
            let kind = toks[index].kind;
            let close = self.stream.close_of[index].filter(|close| *close < hi);
            let chain = self
                .facts
                .chains
                .get(&index)
                .and_then(|list| list.iter().find(|chain| chain.end < hi))
                .cloned();
            let assign = self
                .facts
                .assigns
                .get(&index)
                .copied()
                .filter(|end| *end < hi);
            let (doc, last) = match (is_open(kind), close) {
                _ if chain.is_some() && !self.flat() => {
                    let chain = chain.expect("checked");
                    (self.chain(index, &chain), chain.end)
                }
                _ if assign.is_some() && !self.flat() => {
                    let end = assign.expect("checked");
                    let rhs = self.sequence(index + 1, end + 1);
                    (
                        Doc::Concat(vec![
                            self.token(index),
                            Doc::group(Doc::indent(Doc::Concat(vec![Doc::Line, rhs]))),
                        ]),
                        end,
                    )
                }
                (true, Some(close)) => {
                    let doc = if kind == K::LBrace && self.facts.containers.contains_key(&index) {
                        self.vertical(index, close)
                    } else {
                        self.group(index, close)
                    };
                    (doc, close)
                }
                _ => {
                    if kind == K::TemplateHead {
                        self.template_depth += 1;
                    } else if kind == K::TemplateTail {
                        self.template_depth = self.template_depth.saturating_sub(1);
                    }
                    (self.token(index), index)
                }
            };
            if !first {
                parts.push(self.separation(index));
            }
            first = false;
            parts.push(doc);
            index = last + 1;
        }
        Doc::Concat(parts)
    }

    fn chain(&mut self, start: usize, chain: &crate::chains::Chain) -> Doc {
        let toks = &self.stream.toks;
        let end = chain.end;
        match &chain.kind {
            ChainKind::Operators(operators) | ChainKind::Pipeline(operators) => {
                let leading_operator = matches!(chain.kind, ChainKind::Pipeline(_));
                let mut bounds = vec![start];
                for op in operators {
                    bounds.push(*op);
                }
                let mut tail = Vec::new();
                let first = self.sequence_skipping(start, operators[0], start, end);
                for (position, op) in operators.iter().enumerate() {
                    let stop = operators.get(position + 1).copied().unwrap_or(end + 1);
                    let operand = self.sequence_skipping(op + 1, stop, start, end);
                    let operator = self.token(*op);
                    if leading_operator {
                        tail.extend([Doc::Line, operator, Doc::Space, operand]);
                    } else {
                        tail.extend([Doc::Space, operator, Doc::Line, operand]);
                    }
                }
                let _ = toks;
                Doc::group(Doc::Concat(vec![first, Doc::indent(Doc::Concat(tail))]))
            }
            ChainKind::Members(dots) => {
                let head_end = dots[0];
                let short_head = head_end == start + 1
                    && matches!(toks[start].kind, K::This | K::Identifier)
                    && toks[start].text.chars().count() <= crate::INDENT_WIDTH;
                let first_link = usize::from(short_head);
                let head = self.sequence(start, dots.get(first_link).copied().unwrap_or(end + 1));
                let mut links = Vec::new();
                for (position, dot) in dots.iter().enumerate().skip(first_link) {
                    let stop = dots.get(position + 1).copied().unwrap_or(end + 1);
                    links.push(self.sequence(*dot, stop));
                }
                let spill = head.forces_break()
                    || links[..links.len().saturating_sub(1)]
                        .iter()
                        .any(Doc::forces_break);
                let mut tail = Vec::new();
                for link in links {
                    tail.extend([Doc::Soft, link]);
                }
                Doc::Group(
                    Box::new(Doc::Concat(vec![head, Doc::indent(Doc::Concat(tail))])),
                    spill,
                )
            }
            ChainKind::Ternary {
                question,
                colon,
                trailing_question,
            } => {
                let test = self.sequence(start, *question);
                let consequent = self.sequence(question + 1, *colon);
                let alternate = self.sequence(colon + 1, end + 1);
                let question_token = self.token(*question);
                let colon_token = self.token(*colon);
                let tail = if *trailing_question {
                    vec![
                        Doc::Space,
                        question_token,
                        Doc::Space,
                        consequent,
                        Doc::Space,
                        colon_token,
                        Doc::Line,
                        alternate,
                    ]
                } else {
                    vec![
                        Doc::Line,
                        question_token,
                        Doc::Space,
                        consequent,
                        Doc::Line,
                        colon_token,
                        Doc::Space,
                        alternate,
                    ]
                };
                Doc::group(Doc::Concat(vec![test, Doc::indent(Doc::Concat(tail))]))
            }
        }
    }

    fn sequence_skipping(
        &mut self,
        lo: usize,
        hi: usize,
        _chain_start: usize,
        _chain_end: usize,
    ) -> Doc {
        self.sequence(lo, hi)
    }

    fn items(&self, container: usize, close: usize) -> Vec<usize> {
        let mut starts: BTreeSet<usize> = BTreeSet::new();
        for offset in &self.facts.containers[&container] {
            let index = self.stream.index_at_or_after(*offset);
            if index > container && index < close {
                starts.insert(index);
            }
        }
        if close > container + 1 {
            starts.insert(container + 1);
        }
        starts.into_iter().collect()
    }

    fn separated(&mut self, starts: &[usize], end: usize) -> Doc {
        let mut parts = Vec::new();
        for (position, start) in starts.iter().enumerate() {
            let stop = starts.get(position + 1).copied().unwrap_or(end);
            if position > 0 {
                parts.push(Doc::Hard);
                if self.stream.toks[*start].blank_before {
                    parts.push(Doc::Hard);
                }
            }
            parts.push(self.sequence(*start, stop));
        }
        Doc::Concat(parts)
    }

    fn dangling(&self, close: usize) -> Vec<Doc> {
        let mut parts = Vec::new();
        for comment in &self.stream.toks[close].leading {
            parts.push(Doc::Hard);
            if comment.blank_before {
                parts.push(Doc::Hard);
            }
            parts.push(comment_doc(comment));
        }
        parts
    }

    fn vertical(&mut self, open: usize, close: usize) -> Doc {
        let starts = self.items(open, close);
        let open_doc = self.token(open);
        let close_core = Doc::Concat(vec![self.core(close), self.trailing(close)]);
        let dangling = self.dangling(close);
        if starts.is_empty() && dangling.is_empty() {
            return Doc::Concat(vec![open_doc, close_core]);
        }
        let mut inner = vec![Doc::Hard, self.separated(&starts, close)];
        if starts.is_empty() {
            inner.clear();
            inner.extend(dangling);
        } else {
            inner.extend(dangling);
        }
        Doc::Concat(vec![
            open_doc,
            Doc::indent(Doc::Concat(inner)),
            Doc::Hard,
            close_core,
        ])
    }

    pub fn root(&mut self) -> Doc {
        let toks = &self.stream.toks;
        let end = toks.len() - 1;
        let mut starts: BTreeSet<usize> = BTreeSet::new();
        for offset in &self.facts.root_items {
            let index = self.stream.index_at_or_after(*offset);
            if index < end {
                starts.insert(index);
            }
        }
        if end > 0 {
            starts.insert(0);
        }
        let starts: Vec<usize> = starts.into_iter().collect();
        let mut parts = vec![self.separated(&starts, end)];
        for comment in &self.stream.toks[end].leading {
            if !starts.is_empty() {
                parts.push(Doc::Hard);
            }
            if comment.blank_before && !starts.is_empty() {
                parts.push(Doc::Hard);
            }
            parts.push(comment_doc(comment));
        }
        if !starts.is_empty() || !self.stream.toks[end].leading.is_empty() {
            parts.push(Doc::Hard);
        }
        Doc::Concat(parts)
    }

    fn group(&mut self, open: usize, close: usize) -> Doc {
        let toks = &self.stream.toks;
        let kind = toks[open].kind;
        let flat = self.flat();
        let open_doc = self.token(open);
        let close_core = Doc::Concat(vec![self.core(close), self.trailing(close)]);
        let dangling = self.dangling(close);
        if close == open + 1 && dangling.is_empty() {
            return Doc::Concat(vec![open_doc, close_core]);
        }

        let indexing = matches!(kind, K::LBracket | K::QuestionLBracket)
            && open > 0
            && ends_operand(&toks[open - 1], open.checked_sub(2).map(|b| &toks[b]));
        if indexing {
            let inside = self.sequence(open + 1, close);
            return Doc::Concat(vec![open_doc, inside, Doc::Concat(dangling), close_core]);
        }

        let for_header = open > 0 && toks[open - 1].kind == K::For;
        let semicolons = kind == K::LBrace || for_header;
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        let mut separators: Vec<usize> = Vec::new();
        let mut cursor = open + 1;
        let mut scan = open + 1;
        while scan < close {
            let scanned = toks[scan].kind;
            if is_open(scanned) {
                scan = self.stream.close_of[scan].unwrap_or(scan) + 1;
                continue;
            }
            if scanned == K::Comma || (scanned == K::Semicolon && semicolons) {
                ranges.push((cursor, scan));
                separators.push(scan);
                cursor = scan + 1;
            }
            scan += 1;
        }
        let trailing_separator = if cursor >= close {
            separators.pop()
        } else {
            ranges.push((cursor, close));
            None
        };

        let mut docs: Vec<Doc> = ranges
            .iter()
            .map(|(lo, hi)| self.sequence(*lo, *hi))
            .collect();
        let has_separator = !separators.is_empty() || trailing_separator.is_some();
        let call_like = kind == K::LParen
            && open > 0
            && ends_operand(&toks[open - 1], open.checked_sub(2).map(|b| &toks[b]));
        let trailing_allowed = !flat
            && !for_header
            && (matches!(kind, K::LBracket | K::QuestionLBracket | K::LBrace)
                || (kind == K::LParen
                    && (has_separator || call_like)
                    && !ranges
                        .last()
                        .is_some_and(|(lo, _)| toks[*lo].kind == K::DotDotDot)));
        let separator_text = separators
            .last()
            .copied()
            .or(trailing_separator)
            .map(|index| toks[index].text.clone())
            .unwrap_or_else(|| ",".to_string());

        let sole_literal = ranges.len() == 1 && {
            let (lo, hi) = ranges[0];
            let head = if toks[lo].kind == K::Hash { lo + 1 } else { lo };
            hi > head
                && matches!(toks[head].kind, K::LBracket | K::LBrace)
                && self.stream.close_of[head] == Some(hi - 1)
        };
        let hug = kind == K::LParen
            && !flat
            && !docs.is_empty()
            && (sole_literal
                || (docs.last().is_some_and(Doc::forces_break)
                    && docs[..docs.len() - 1].iter().all(|doc| !doc.forces_break())));
        if hug {
            let mut parts = vec![open_doc];
            let count = docs.len();
            for (position, doc) in docs.drain(..).enumerate() {
                parts.push(doc);
                if position + 1 < count {
                    parts.push(self.token(separators[position]));
                    parts.push(Doc::Space);
                }
            }
            if let (Some(index), false) = (trailing_separator, trailing_allowed) {
                parts.push(self.token(index));
            }
            parts.push(close_core);
            return Doc::group(Doc::Concat(parts));
        }

        let arrow_head = if kind == K::LParen && !flat && !docs.is_empty() {
            self.arrow_tail(ranges[ranges.len() - 1])
        } else {
            None
        };
        let hugged = arrow_head.and_then(|head_end| {
            let (last_lo, _) = ranges[ranges.len() - 1];
            if docs[..docs.len() - 1].iter().any(Doc::forces_break) || !dangling.is_empty() {
                return None;
            }
            let mut head_parts = vec![open_doc.clone()];
            let mut parts = vec![open_doc.clone()];
            for (position, doc) in docs.iter().enumerate() {
                parts.push(doc.clone());
                if position + 1 < docs.len() {
                    let separator = self.token(separators[position]);
                    head_parts.extend([doc.clone(), separator.clone(), Doc::Space]);
                    parts.extend([separator, Doc::Space]);
                }
            }
            head_parts.push(self.sequence(last_lo, head_end + 1));
            if let (Some(index), false) = (trailing_separator, trailing_allowed) {
                parts.push(self.token(index));
            }
            parts.push(close_core.clone());
            Some((Doc::Concat(head_parts), Doc::group(Doc::Concat(parts))))
        });

        let spaced = kind == K::LBrace;
        let soft = if flat {
            if spaced {
                Doc::Space
            } else {
                Doc::Nil
            }
        } else if spaced {
            Doc::Line
        } else {
            Doc::Soft
        };
        let line = if flat { Doc::Space } else { Doc::Line };

        let mut inner = vec![soft.clone()];
        let count = docs.len();
        for (position, doc) in docs.into_iter().enumerate() {
            inner.push(doc);
            if position + 1 < count {
                inner.push(self.token(separators[position]));
                inner.push(line.clone());
            }
        }
        if let Some(index) = trailing_separator {
            inner.push(self.trailing(index));
        }
        if trailing_allowed {
            inner.push(Doc::if_break(Doc::text(separator_text), Doc::Nil));
        } else if let Some(index) = trailing_separator {
            inner.push(self.core(index));
        }
        inner.extend(dangling);

        let mut forced = false;
        if kind == K::LBrace && !flat {
            let first = &toks[open + 1];
            forced = open + 1 != close && first.newline_before;
        }
        let mut parts = vec![open_doc, Doc::indent(Doc::Concat(inner)), soft, close_core];
        if forced {
            parts.insert(0, Doc::BreakParent);
        }
        let expanded = Doc::group(Doc::Concat(parts));
        match hugged {
            Some((head, hugged)) => Doc::hugging(head, hugged, expanded),
            None => expanded,
        }
    }

    fn arrow_tail(&self, range: (usize, usize)) -> Option<usize> {
        let toks = &self.stream.toks;
        let (lo, hi) = range;
        let mut index = lo;
        while index < hi {
            if toks[index].kind == K::FatArrow {
                return self
                    .facts
                    .assigns
                    .get(&index)
                    .filter(|end| **end + 1 == hi)
                    .map(|_| index);
            }
            index = match self.stream.close_of[index] {
                Some(close) if is_open(toks[index].kind) => close + 1,
                _ => index + 1,
            };
        }
        None
    }
}
