use vn_core::BinOp;
use vn_core::TokenKind;

pub(crate) fn token_to_binary_op(kind: TokenKind) -> Option<BinOp> {
    match kind {
        TokenKind::Plus => Some(BinOp::Add),
        TokenKind::Minus => Some(BinOp::Sub),
        TokenKind::Star => Some(BinOp::Mul),
        TokenKind::Slash => Some(BinOp::Div),
        TokenKind::Percent => Some(BinOp::Mod),
        TokenKind::StarStar => Some(BinOp::Pow),
        TokenKind::EqEq | TokenKind::EqEqEq => Some(BinOp::Eq),
        TokenKind::BangEq | TokenKind::BangEqEq => Some(BinOp::Ne),
        TokenKind::LAngle => Some(BinOp::Lt),
        TokenKind::RAngle => Some(BinOp::Gt),
        TokenKind::LtEq => Some(BinOp::Le),
        TokenKind::GtEq => Some(BinOp::Ge),
        TokenKind::Amp => Some(BinOp::BitAnd),
        TokenKind::Pipe => Some(BinOp::BitOr),
        TokenKind::Caret => Some(BinOp::BitXor),
        TokenKind::LtLt => Some(BinOp::Shl),
        TokenKind::GtGt => Some(BinOp::Shr),
        TokenKind::GtGtGt => Some(BinOp::Ushr),
        TokenKind::In => Some(BinOp::In),
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
        | TokenKind::PlusPlus
        | TokenKind::PlusEq
        | TokenKind::MinusMinus
        | TokenKind::MinusEq
        | TokenKind::StarEq
        | TokenKind::StarStarEq
        | TokenKind::SlashEq
        | TokenKind::PercentEq
        | TokenKind::AmpAmp
        | TokenKind::AmpEq
        | TokenKind::AmpAmpEq
        | TokenKind::PipePipe
        | TokenKind::PipeEq
        | TokenKind::PipePipeEq
        | TokenKind::PipeGt
        | TokenKind::CaretEq
        | TokenKind::Tilde
        | TokenKind::LtLtEq
        | TokenKind::GtGtEq
        | TokenKind::GtGtGtEq
        | TokenKind::Eq
        | TokenKind::Bang
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
