use crate::source::SourceRange;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TriviaKind {
    Line,
    Block,
    Doc(Arc<str>),
}

#[derive(Clone, Debug)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub range: SourceRange,
}
