use crate::tokens::Tok;
use std::collections::HashSet;
use vn_core::TokenKind as K;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gap {
    None,
    Space,
}

pub struct Context<'a> {
    pub toks: &'a [Tok],
    pub ternary: &'a HashSet<usize>,
    pub comparisons: &'a HashSet<usize>,
}

pub fn ends_operand(prev: &Tok, before: Option<&Tok>) -> bool {
    if matches!(
        prev.kind,
        K::Identifier
            | K::IntegerLiteral
            | K::FloatLiteral
            | K::BinaryLiteral
            | K::OctalLiteral
            | K::HexLiteral
            | K::BigIntLiteral
            | K::DecimalLiteral
            | K::Str
            | K::RawStr
            | K::Char
            | K::Template
            | K::TemplateTail
            | K::RegularExpression
            | K::RParen
            | K::RBracket
            | K::This
            | K::Super
            | K::True
            | K::False
            | K::Null
            | K::Dynamic
    ) {
        return true;
    }
    matches!(prev.kind, K::Bang | K::PlusPlus | K::MinusMinus)
        && before.is_some_and(|b| ends_operand(b, None))
}

fn is_spaced_operator(kind: K) -> bool {
    matches!(
        kind,
        K::Eq
            | K::EqEq
            | K::EqEqEq
            | K::BangEq
            | K::BangEqEq
            | K::LtEq
            | K::GtEq
            | K::PlusEq
            | K::MinusEq
            | K::StarEq
            | K::StarStarEq
            | K::SlashEq
            | K::PercentEq
            | K::AmpEq
            | K::AmpAmpEq
            | K::PipeEq
            | K::PipePipeEq
            | K::CaretEq
            | K::LtLtEq
            | K::GtGtEq
            | K::GtGtGtEq
            | K::QuestionQuestionEq
            | K::AmpAmp
            | K::PipePipe
            | K::QuestionQuestion
            | K::PipeGt
            | K::Arrow
            | K::FatArrow
            | K::Amp
            | K::Pipe
            | K::Caret
            | K::LtLt
            | K::GtGt
            | K::GtGtGt
            | K::Slash
            | K::Percent
            | K::StarStar
    )
}

fn is_control_keyword(kind: K) -> bool {
    matches!(
        kind,
        K::If
            | K::While
            | K::For
            | K::Switch
            | K::Catch
            | K::Match
            | K::With
            | K::Return
            | K::Throw
            | K::Await
            | K::Yield
            | K::In
            | K::Is
            | K::As
            | K::Do
            | K::Else
            | K::Case
            | K::Using
            | K::Delete
            | K::Void
            | K::Async
            | K::Function
            | K::Extends
            | K::Implements
            | K::Try
            | K::Finally
    )
}

fn is_type_head(kind: K) -> bool {
    matches!(
        kind,
        K::Identifier
            | K::Function
            | K::Class
            | K::Struct
            | K::Interface
            | K::Enum
            | K::Type
            | K::Extension
            | K::Get
            | K::Set
            | K::Of
            | K::From
            | K::On
            | K::Out
            | K::Mut
            | K::Native
            | K::RAngle
    )
}

fn operand_before(index: usize, toks: &[Tok]) -> bool {
    index
        .checked_sub(1)
        .is_some_and(|p| ends_operand(&toks[p], p.checked_sub(1).map(|b| &toks[b])))
}

fn prefix_operator(index: usize, toks: &[Tok]) -> bool {
    let kind = toks[index].kind;
    kind == K::Tilde
        || (matches!(kind, K::Plus | K::Minus | K::Bang) && !operand_before(index, toks))
}

impl Context<'_> {
    pub fn gap(&self, next_index: usize, source_had_space: bool) -> Gap {
        let toks = self.toks;
        let prev_index = next_index - 1;
        let prev = &toks[prev_index];
        let next = &toks[next_index];

        if self.comparisons.contains(&next_index) || self.comparisons.contains(&prev_index) {
            return Gap::Space;
        }
        if matches!(prev.kind, K::TemplateHead | K::TemplateMiddle)
            || matches!(next.kind, K::TemplateMiddle | K::TemplateTail)
        {
            return Gap::None;
        }

        if let Some(gap) = self.before_next(next_index, source_had_space) {
            return gap;
        }
        if let Some(gap) = self.after_prev(prev_index, next_index, source_had_space) {
            return gap;
        }

        if is_spaced_operator(next.kind) || is_spaced_operator(prev.kind) {
            return Gap::Space;
        }
        if matches!(next.kind, K::Plus | K::Minus | K::Star) {
            return if operand_before(next_index, toks) || source_had_space {
                Gap::Space
            } else {
                Gap::None
            };
        }
        if prev.kind == K::RAngle && matches!(next.kind, K::LParen | K::LBracket) {
            return Gap::None;
        }
        if next.kind == K::LParen {
            return self.before_paren(prev_index);
        }
        if next.kind == K::LBracket {
            let indexed = ends_operand(prev, prev_index.checked_sub(1).map(|b| &toks[b]))
                || prev.kind == K::RAngle;
            return if indexed { Gap::None } else { Gap::Space };
        }
        Gap::Space
    }

    fn before_next(&self, next_index: usize, source_had_space: bool) -> Option<Gap> {
        let toks = self.toks;
        let prev_index = next_index - 1;
        let prev = &toks[prev_index];
        let next = &toks[next_index];
        if matches!(
            next.kind,
            K::Comma
                | K::Semicolon
                | K::RParen
                | K::RBracket
                | K::Dot
                | K::QuestionDot
                | K::ColonColon
                | K::DotDot
                | K::DotDotEq
                | K::QuestionLBracket
        ) {
            return Some(Gap::None);
        }
        if matches!(next.kind, K::PlusPlus | K::MinusMinus)
            && ends_operand(prev, prev_index.checked_sub(1).map(|b| &toks[b]))
        {
            return Some(Gap::None);
        }
        if matches!(next.kind, K::Colon | K::Question) {
            return Some(if self.ternary.contains(&next_index) {
                Gap::Space
            } else {
                Gap::None
            });
        }
        if next.kind == K::Bang && operand_before(next_index, toks) {
            return Some(Gap::None);
        }
        if next.kind == K::RAngle {
            return Some(if prev.kind == K::RAngle && source_had_space {
                Gap::Space
            } else {
                Gap::None
            });
        }
        if next.kind == K::LAngle {
            return Some(if is_type_head(prev.kind) {
                Gap::None
            } else {
                Gap::Space
            });
        }
        None
    }

    fn after_prev(
        &self,
        prev_index: usize,
        next_index: usize,
        source_had_space: bool,
    ) -> Option<Gap> {
        let toks = self.toks;
        let prev = &toks[prev_index];
        let next = &toks[next_index];
        if matches!(
            prev.kind,
            K::LParen
                | K::LBracket
                | K::QuestionLBracket
                | K::Dot
                | K::QuestionDot
                | K::ColonColon
                | K::At
                | K::Hash
                | K::Tilde
                | K::DotDotDot
                | K::Backslash
                | K::Dollar
                | K::LAngle
                | K::DotDot
                | K::DotDotEq
        ) {
            return Some(Gap::None);
        }
        if matches!(prev.kind, K::Plus | K::Minus) && prefix_operator(prev_index, toks) {
            return Some(
                if matches!(next.kind, K::Plus | K::Minus | K::PlusPlus | K::MinusMinus) {
                    Gap::Space
                } else {
                    Gap::None
                },
            );
        }
        if prev.kind == K::Bang && prefix_operator(prev_index, toks) {
            return Some(Gap::None);
        }
        if prev.kind == K::Colon {
            return Some(Gap::Space);
        }
        if prev.kind == K::Question && !self.ternary.contains(&prev_index) {
            return Some(if source_had_space {
                Gap::Space
            } else {
                Gap::None
            });
        }
        if matches!(prev.kind, K::Comma | K::Semicolon) {
            return Some(Gap::Space);
        }
        None
    }

    fn before_paren(&self, prev_index: usize) -> Gap {
        let toks = self.toks;
        let prev = &toks[prev_index];
        let named = prev_index
            .checked_sub(1)
            .is_some_and(|b| matches!(toks[b].kind, K::Dot | K::QuestionDot));
        if named {
            return Gap::None;
        }
        if matches!(prev.kind, K::If | K::For | K::While | K::Switch | K::Catch) {
            return Gap::None;
        }
        if is_control_keyword(prev.kind) || matches!(prev.kind, K::LBrace | K::RBrace) {
            return Gap::Space;
        }
        let call = ends_operand(prev, None)
            || matches!(
                prev.kind,
                K::RAngle | K::Super | K::This | K::New | K::Import | K::Export | K::Constructor
            );
        if call {
            Gap::None
        } else {
            Gap::Space
        }
    }
}
