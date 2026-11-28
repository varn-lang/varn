use crate::error::{PResult, ParseError};
mod angle;
mod regex_literals;

use crate::ParseProfile;
use std::sync::Arc;
use vn_core::ast::{AstArena, AstId, AstTypeKind, ExprId, ExprKind, StmtId, StmtKind, TypeNode};
use vn_core::{ErrorCode, ParsedNumber, SourceLocation, SourceRange, Token, TokenKind};

static EOF_SENTINEL: Token = Token {
    kind: TokenKind::EOF,
    range: SourceRange {
        start: vn_core::SourceLocation {
            offset: 0,
            line: 0,
            column: 0,
        },
        end: vn_core::SourceLocation {
            offset: 0,
            line: 0,
            column: 0,
        },
    },
    parsed_num: None,
    lex_start: 0,
    lex_len: 0,
};

fn piece_range(token: SourceRange, piece: u8) -> SourceRange {
    let at = |extra: u32| SourceLocation {
        offset: token.start.offset + extra,
        line: token.start.line,
        column: token.start.column + extra,
    };
    SourceRange {
        start: at(u32::from(piece)),
        end: at(u32::from(piece) + 1),
    }
}

pub struct TokenStream {
    tokens: Vec<Token>,
    lexeme_buf: Arc<[u8]>,
    pos: usize,
    pub filename: Arc<str>,

    pending_doc: Option<Arc<str>>,

    pub errors: vn_core::DiagnosticBag,
    pub profile: ParseProfile,
    pub async_body: bool,

    piece: u8,
    pub(crate) split_tokens: Vec<u32>,
    pub(crate) name_tokens: Vec<u32>,
    next_ast_id: AstId,
    regex_literals: Vec<regex_literals::RegexLiteral>,

    pub interner: vn_core::AtomInterner,

    pub arena: AstArena,
}

impl TokenStream {
    pub fn new(
        tokens: Vec<Token>,
        lexeme_buf: Arc<[u8]>,
        filename: Arc<str>,
        interner: vn_core::AtomInterner,
    ) -> Self {
        TokenStream {
            tokens,
            lexeme_buf,
            pos: 0,
            filename,
            pending_doc: None,
            errors: vn_core::DiagnosticBag::new(),
            profile: ParseProfile::default(),
            async_body: false,
            piece: 0,
            split_tokens: Vec::new(),
            name_tokens: Vec::new(),
            next_ast_id: 1,
            regex_literals: Vec::new(),
            interner,
            arena: AstArena::new(),
        }
    }

    pub fn in_function_body<R>(&mut self, is_async: bool, parse: impl FnOnce(&mut Self) -> R) -> R {
        let outer = std::mem::replace(&mut self.async_body, is_async);
        let parsed = parse(self);
        self.async_body = outer;
        parsed
    }

    #[inline(always)]
    pub fn next_ast_id(&mut self) -> AstId {
        let id = self.next_ast_id;
        self.next_ast_id += 1;
        id
    }

    #[inline(always)]
    pub fn expr(&mut self, range: SourceRange, kind: ExprKind) -> ExprId {
        self.arena.alloc_expr(kind, range)
    }

    #[inline(always)]
    pub fn stmt(&mut self, range: SourceRange, kind: StmtKind) -> StmtId {
        self.arena.alloc_stmt(kind, range)
    }

    #[inline(always)]
    pub fn expr_range(&self, id: ExprId) -> SourceRange {
        self.arena.expr(id).range
    }

    #[inline(always)]
    pub fn type_node(&mut self, range: SourceRange, kind: AstTypeKind) -> TypeNode {
        TypeNode {
            id: self.next_ast_id(),
            range,
            kind,
        }
    }

    pub fn push_error(&mut self, message: String, range: SourceRange) {
        self.push_diagnostic(ErrorCode::UnexpectedToken, message, range);
    }

    pub fn push_diagnostic(&mut self, code: ErrorCode, message: String, range: SourceRange) {
        self.errors.push(
            vn_core::Diagnostic::error(code, message)
                .with_range(range)
                .with_file(self.filename.clone()),
        );
    }

    #[inline]
    pub fn kind(&self) -> TokenKind {
        if self.piece > 0 {
            return self.token().kind.angle_pieces()[usize::from(self.piece)];
        }
        self.token().kind
    }

    #[inline]
    pub fn peek_kind(&self, offset: usize) -> TokenKind {
        let idx = self.pos + offset;
        if idx < self.tokens.len() {
            self.tokens[idx].kind
        } else {
            TokenKind::EOF
        }
    }

    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }

    #[inline]
    pub fn line(&self) -> u32 {
        self.range().start.line
    }

    #[inline]
    pub fn prev_line(&self) -> u32 {
        if self.pos > 0 || self.piece > 0 {
            self.prev_range().end.line
        } else {
            0
        }
    }

    #[inline]
    pub fn prev_end_range(&self) -> SourceRange {
        let end = self.prev_range().end;
        SourceRange { start: end, end }
    }

    #[inline]
    pub fn column(&self) -> u32 {
        self.range().start.column
    }

    #[inline]
    pub fn prev_range(&self) -> SourceRange {
        if self.piece > 0 {
            return piece_range(self.token().range, self.piece - 1);
        }
        if self.pos > 0 {
            self.tokens[self.pos - 1].range
        } else {
            self.range()
        }
    }

    #[inline]
    pub fn peek_line(&self, offset: usize) -> u32 {
        let idx = self.pos.saturating_add(offset);
        if idx < self.tokens.len() {
            self.tokens[idx].range.start.line
        } else {
            if let Some(last) = self.tokens.last() {
                last.range.end.line
            } else {
                0
            }
        }
    }

    #[inline]
    pub fn token(&self) -> &Token {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            self.tokens.last().unwrap_or(&EOF_SENTINEL)
        }
    }

    #[inline]
    pub fn range(&self) -> SourceRange {
        let range = self.token().range;
        if self.piece > 0 {
            return piece_range(range, self.piece);
        }
        range
    }

    #[inline]
    pub fn lexeme(&self) -> &str {
        let tok = self.token();
        tok.get_lexeme(&self.lexeme_buf)
    }

    pub fn consume_str(&mut self) -> String {
        let s = self.lexeme().to_owned();
        self.advance();
        s
    }

    #[inline]
    pub fn parsed_num(&self) -> Option<ParsedNumber> {
        self.token().parsed_num
    }

    #[inline]
    pub fn advance(&mut self) {
        if self.piece > 0 && usize::from(self.piece) + 1 < self.token().kind.angle_pieces().len() {
            self.piece += 1;
            return;
        }
        self.piece = 0;
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    #[inline]
    pub fn check(&self, kind: TokenKind) -> bool {
        self.kind() == kind
    }

    #[inline]
    pub fn eat(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn describe_current(&self) -> String {
        let tok = self.token();
        let kind = tok.kind;
        if kind == TokenKind::EOF {
            "end of file".to_owned()
        } else if kind == TokenKind::Identifier {
            format!("identifier `{}`", tok.get_lexeme(&self.lexeme_buf))
        } else if matches!(
            kind,
            TokenKind::IntegerLiteral
                | TokenKind::FloatLiteral
                | TokenKind::BigIntLiteral
                | TokenKind::DecimalLiteral
                | TokenKind::Str
                | TokenKind::Char
        ) {
            format!("{} literal", kind.as_str())
        } else {
            format!("`{}`", tok.get_lexeme(&self.lexeme_buf))
        }
    }

    fn mismatch(&self, kind: TokenKind) -> ParseError {
        let found = self.describe_current();
        if self.kind() == TokenKind::EOF {
            let unterminated = [
                (TokenKind::RParen, ErrorCode::UnterminatedParen),
                (TokenKind::RBrace, ErrorCode::UnterminatedBrace),
                (TokenKind::RBracket, ErrorCode::UnterminatedBracket),
            ]
            .into_iter()
            .find(|(closing, _)| *closing == kind);
            if let Some((_, code)) = unterminated {
                return ParseError::new(
                    code,
                    format!("expected `{}` before the end of the file", kind.as_str()),
                );
            }
        }
        let wanted = if kind == TokenKind::Identifier {
            "an identifier".to_owned()
        } else {
            format!("`{}`", kind.as_str())
        };
        ParseError::expected(format!("expected {wanted}, found {found}"))
    }

    pub fn expect(&mut self, kind: TokenKind) -> PResult<()> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.mismatch(kind))
        }
    }

    pub fn expect_token(&mut self, kind: TokenKind) -> PResult<Token> {
        if self.check(kind) {
            let tok = self.token().clone();
            self.advance();
            Ok(tok)
        } else {
            Err(self.mismatch(kind))
        }
    }

    pub fn expect_id(&mut self) -> PResult<vn_core::Atom> {
        if self.kind().can_be_identifier() {
            Ok(self.consume_name())
        } else {
            Err(self.mismatch(TokenKind::Identifier))
        }
    }

    pub fn note_name_token(&mut self) {
        if self.kind() != TokenKind::Identifier {
            self.name_tokens.push(self.range().start.offset);
        }
    }

    pub fn consume_name(&mut self) -> vn_core::Atom {
        self.note_name_token();
        self.consume_lexeme()
    }

    pub fn consume_lexeme(&mut self) -> vn_core::Atom {
        let tok = self.token().clone();
        let text = tok.get_lexeme(&self.lexeme_buf);
        let atom = self.interner.intern(text);
        self.advance();
        atom
    }

    #[inline]
    pub fn is_eof(&self) -> bool {
        self.kind() == TokenKind::EOF
    }

    #[inline]
    pub fn span_from(&self, start: SourceRange) -> SourceRange {
        if self.pos == 0 && self.piece == 0 {
            return start;
        }
        SourceRange {
            start: start.start,
            end: self.prev_range().end,
        }
    }

    pub fn eat_semicolon(&mut self) {
        while self.eat(TokenKind::Semicolon) {}
    }

    pub fn store_pending_doc(&mut self, doc: Arc<str>) {
        self.pending_doc = Some(doc);
    }

    pub fn current_doc(&mut self) -> Option<String> {
        self.pending_doc.take().map(|rc| rc.to_string())
    }
}
