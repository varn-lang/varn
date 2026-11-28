use crate::error::{PResult, ParseError};
use crate::stream::TokenStream;
use vn_core::ast::{Decl, StmtId, StmtKind};
use vn_core::{SourceRange, TokenKind};

fn cover(s: &mut TokenStream, stmt: StmtId, start: SourceRange) -> StmtId {
    let full = s.span_from(start);
    let node = s.arena.stmt_mut(stmt);
    node.range = full;
    if let StmtKind::Decl(decl) = &mut node.kind {
        decl.set_range(full);
    }
    stmt
}

pub(super) fn try_parse_decl_stmt(
    s: &mut TokenStream,
    kind: TokenKind,
    next_kind: TokenKind,
    decorators: Vec<vn_core::ast::Decorator>,
    start: SourceRange,
) -> Option<PResult<StmtId>> {
    if kind == TokenKind::Declare {
        s.advance();
        return try_parse_decl_stmt_mode(s, s.kind(), s.peek_kind(1), decorators, true, start);
    }

    try_parse_decl_stmt_mode(s, kind, next_kind, decorators, false, start)
}

pub(super) fn try_parse_decl_stmt_mode(
    s: &mut TokenStream,
    kind: TokenKind,
    next_kind: TokenKind,
    decorators: Vec<vn_core::ast::Decorator>,
    is_declare: bool,
    start: SourceRange,
) -> Option<PResult<StmtId>> {
    let result = match kind {
        TokenKind::Function => {
            let decl = match super::decls::parse_function_decl(s, decorators, false, is_declare) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            let range = decl.range;
            Ok(s.stmt(range, StmtKind::Decl(Box::new(Decl::Function(decl)))))
        }
        TokenKind::Async if next_kind == TokenKind::Function => {
            s.advance();
            let decl = match super::decls::parse_function_decl(s, decorators, true, is_declare) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            let range = decl.range;
            Ok(s.stmt(range, StmtKind::Decl(Box::new(Decl::Function(decl)))))
        }
        TokenKind::Class | TokenKind::Abstract => {
            let decl = match super::decls::parse_class_decl(s, decorators, is_declare) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            let range = decl.range;
            Ok(s.stmt(range, StmtKind::Decl(Box::new(Decl::Class(decl)))))
        }
        TokenKind::Interface => {
            if !decorators.is_empty() {
                return Some(Err(ParseError::invalid(
                    "decorators are not supported on interfaces".to_owned(),
                )));
            }
            let decl = match super::decls::parse_interface_decl(s) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            let range = decl.range;
            Ok(s.stmt(range, StmtKind::Decl(Box::new(Decl::Interface(decl)))))
        }
        TokenKind::Type => {
            if !decorators.is_empty() {
                return Some(Err(ParseError::invalid(
                    "decorators are not supported on type aliases".to_owned(),
                )));
            }
            let decl = match super::decls::parse_type_alias_decl(s) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            let range = *decl.range();
            Ok(s.stmt(range, StmtKind::Decl(Box::new(decl))))
        }
        TokenKind::Enum => {
            if !decorators.is_empty() {
                return Some(Err(ParseError::invalid(
                    "decorators are not supported on enums".to_owned(),
                )));
            }
            let mut decl = match super::decls::parse_enum_decl(s) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            decl.is_declare = is_declare;
            let range = decl.range;
            Ok(s.stmt(range, StmtKind::Decl(Box::new(Decl::Enum(decl)))))
        }
        TokenKind::Namespace => {
            if !decorators.is_empty() {
                return Some(Err(ParseError::invalid(
                    "decorators are not supported on namespaces".to_owned(),
                )));
            }
            let decl = match super::decls::parse_namespace_decl(s, is_declare) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            let range = decl.range;
            Ok(s.stmt(range, StmtKind::Decl(Box::new(Decl::Namespace(decl)))))
        }
        TokenKind::Struct => {
            let decl = match super::decls::parse_struct_decl(s, decorators) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            let range = decl.range;
            Ok(s.stmt(range, StmtKind::Decl(Box::new(Decl::Class(decl)))))
        }
        TokenKind::Extension => {
            if !decorators.is_empty() {
                return Some(Err(ParseError::invalid(
                    "decorators are not supported on extensions".to_owned(),
                )));
            }
            let decl = match super::decls::parse_extension_decl(s) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            let range = decl.range;
            Ok(s.stmt(range, StmtKind::Decl(Box::new(Decl::Extension(decl)))))
        }
        TokenKind::Let | TokenKind::Const | TokenKind::Var => {
            if !decorators.is_empty() {
                return Some(Err(ParseError::invalid(
                    "decorators are not supported on variable declarations".to_owned(),
                )));
            }
            let decl = match super::decls::parse_var_decl_with_declare(s, is_declare) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            s.eat_semicolon();
            let range = decl.range;
            Ok(s.stmt(range, StmtKind::Decl(Box::new(Decl::Variable(decl)))))
        }
        TokenKind::Import if next_kind == TokenKind::Dot => return None,
        TokenKind::Import => {
            if !decorators.is_empty() {
                return Some(Err(ParseError::invalid(
                    "decorators are not supported on imports".to_owned(),
                )));
            }
            let decl = match super::decls::parse_import_decl(s) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            s.eat_semicolon();
            let range = decl.range;
            Ok(s.stmt(range, StmtKind::Decl(Box::new(Decl::Import(decl)))))
        }
        TokenKind::Export => {
            let decl = match super::decls::parse_export_decl(s, decorators, start) {
                Ok(decl) => decl,
                Err(err) => return Some(Err(err)),
            };
            s.eat_semicolon();
            let range = *decl.range();
            Ok(s.stmt(range, StmtKind::Decl(Box::new(Decl::Export(decl)))))
        }
        TokenKind::EOF
        | TokenKind::Dynamic
        | TokenKind::Identifier
        | TokenKind::IntegerLiteral
        | TokenKind::FloatLiteral
        | TokenKind::BinaryLiteral
        | TokenKind::OctalLiteral
        | TokenKind::HexLiteral
        | TokenKind::BigIntLiteral
        | TokenKind::Str
        | TokenKind::Char
        | TokenKind::Template
        | TokenKind::TemplateHead
        | TokenKind::TemplateMiddle
        | TokenKind::TemplateTail
        | TokenKind::RegularExpression
        | TokenKind::LParen
        | TokenKind::RParen
        | TokenKind::LBrace
        | TokenKind::RBrace
        | TokenKind::LBracket
        | TokenKind::RBracket
        | TokenKind::LAngle
        | TokenKind::RAngle
        | TokenKind::Semicolon
        | TokenKind::Comma
        | TokenKind::Dot
        | TokenKind::DotDot
        | TokenKind::DotDotDot
        | TokenKind::DotDotEq
        | TokenKind::Colon
        | TokenKind::ColonColon
        | TokenKind::Question
        | TokenKind::QuestionDot
        | TokenKind::QuestionLBracket
        | TokenKind::QuestionQuestion
        | TokenKind::QuestionQuestionEq
        | TokenKind::Plus
        | TokenKind::PlusPlus
        | TokenKind::PlusEq
        | TokenKind::Minus
        | TokenKind::MinusMinus
        | TokenKind::MinusEq
        | TokenKind::Star
        | TokenKind::StarStar
        | TokenKind::StarEq
        | TokenKind::StarStarEq
        | TokenKind::Slash
        | TokenKind::SlashEq
        | TokenKind::Percent
        | TokenKind::PercentEq
        | TokenKind::Amp
        | TokenKind::AmpAmp
        | TokenKind::AmpEq
        | TokenKind::AmpAmpEq
        | TokenKind::Pipe
        | TokenKind::PipePipe
        | TokenKind::PipeEq
        | TokenKind::PipePipeEq
        | TokenKind::PipeGt
        | TokenKind::Caret
        | TokenKind::CaretEq
        | TokenKind::Tilde
        | TokenKind::LtLt
        | TokenKind::LtLtEq
        | TokenKind::GtGt
        | TokenKind::GtGtEq
        | TokenKind::GtGtGt
        | TokenKind::GtGtGtEq
        | TokenKind::Eq
        | TokenKind::EqEq
        | TokenKind::EqEqEq
        | TokenKind::Bang
        | TokenKind::BangEq
        | TokenKind::BangEqEq
        | TokenKind::LtEq
        | TokenKind::GtEq
        | TokenKind::Arrow
        | TokenKind::FatArrow
        | TokenKind::On
        | TokenKind::If
        | TokenKind::Else
        | TokenKind::Switch
        | TokenKind::Case
        | TokenKind::Default
        | TokenKind::While
        | TokenKind::For
        | TokenKind::Do
        | TokenKind::Break
        | TokenKind::Continue
        | TokenKind::Return
        | TokenKind::Throw
        | TokenKind::Try
        | TokenKind::Catch
        | TokenKind::Finally
        | TokenKind::Using
        | TokenKind::With
        | TokenKind::From
        | TokenKind::As
        | TokenKind::Async
        | TokenKind::Await
        | TokenKind::Yield
        | TokenKind::New
        | TokenKind::This
        | TokenKind::Super
        | TokenKind::Delete
        | TokenKind::In
        | TokenKind::Of
        | TokenKind::Void
        | TokenKind::Is
        | TokenKind::True
        | TokenKind::False
        | TokenKind::Null
        | TokenKind::Public
        | TokenKind::Private
        | TokenKind::Protected
        | TokenKind::Static
        | TokenKind::Override
        | TokenKind::Readonly
        | TokenKind::Mut
        | TokenKind::Out
        | TokenKind::Declare
        | TokenKind::Native
        | TokenKind::Extends
        | TokenKind::Implements
        | TokenKind::Get
        | TokenKind::Set
        | TokenKind::Constructor
        | TokenKind::Match
        | TokenKind::At
        | TokenKind::Hash
        | TokenKind::Backslash
        | TokenKind::Dollar
        | TokenKind::Backtick
        | TokenKind::Newline
        | TokenKind::Whitespace
        | TokenKind::Placeholder
        | TokenKind::DecimalLiteral
        | TokenKind::RawStr => return None,
    };

    Some(result.map(|stmt| cover(s, stmt, start)))
}
