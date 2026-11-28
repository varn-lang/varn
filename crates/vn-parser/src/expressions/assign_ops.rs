use vn_core::ast::operators::AssignOp;
use vn_core::ast::operators::LogicalOp;
use vn_core::BinOp;
use vn_core::TokenKind;

pub(crate) fn token_to_assign_op(kind: TokenKind) -> Option<AssignOp> {
    match kind {
        TokenKind::Eq => Some(AssignOp::Assign),
        TokenKind::PlusEq => Some(AssignOp::Compound(BinOp::Add)),
        TokenKind::MinusEq => Some(AssignOp::Compound(BinOp::Sub)),
        TokenKind::StarEq => Some(AssignOp::Compound(BinOp::Mul)),
        TokenKind::SlashEq => Some(AssignOp::Compound(BinOp::Div)),
        TokenKind::PercentEq => Some(AssignOp::Compound(BinOp::Mod)),
        TokenKind::StarStarEq => Some(AssignOp::Compound(BinOp::Pow)),
        TokenKind::AmpEq => Some(AssignOp::Compound(BinOp::BitAnd)),
        TokenKind::PipeEq => Some(AssignOp::Compound(BinOp::BitOr)),
        TokenKind::CaretEq => Some(AssignOp::Compound(BinOp::BitXor)),
        TokenKind::LtLtEq => Some(AssignOp::Compound(BinOp::Shl)),
        TokenKind::GtGtEq => Some(AssignOp::Compound(BinOp::Shr)),
        TokenKind::GtGtGtEq => Some(AssignOp::Compound(BinOp::Ushr)),
        TokenKind::AmpAmpEq => Some(AssignOp::Logical(LogicalOp::And)),
        TokenKind::PipePipeEq => Some(AssignOp::Logical(LogicalOp::Or)),
        TokenKind::QuestionQuestionEq => Some(AssignOp::Logical(LogicalOp::Nullish)),
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
        | TokenKind::Plus
        | TokenKind::PlusPlus
        | TokenKind::Minus
        | TokenKind::MinusMinus
        | TokenKind::Star
        | TokenKind::StarStar
        | TokenKind::Slash
        | TokenKind::Percent
        | TokenKind::Amp
        | TokenKind::AmpAmp
        | TokenKind::Pipe
        | TokenKind::PipePipe
        | TokenKind::PipeGt
        | TokenKind::Caret
        | TokenKind::Tilde
        | TokenKind::LtLt
        | TokenKind::GtGt
        | TokenKind::GtGtGt
        | TokenKind::EqEq
        | TokenKind::EqEqEq
        | TokenKind::Bang
        | TokenKind::BangEq
        | TokenKind::BangEqEq
        | TokenKind::LtEq
        | TokenKind::GtEq
        | TokenKind::Arrow
        | TokenKind::FatArrow
        | TokenKind::Let
        | TokenKind::Const
        | TokenKind::Var
        | TokenKind::Function
        | TokenKind::Class
        | TokenKind::Struct
        | TokenKind::Interface
        | TokenKind::Type
        | TokenKind::Enum
        | TokenKind::Namespace
        | TokenKind::Extension
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
        | TokenKind::Import
        | TokenKind::Export
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
        | TokenKind::Abstract
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
        | TokenKind::RawStr => None,
    }
}
