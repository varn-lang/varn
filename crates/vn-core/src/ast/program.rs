use super::arena::StmtId;
use crate::source::SourceRange;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Program {
    pub filename: Arc<str>,
    pub body: Vec<StmtId>,
    pub range: SourceRange,
    pub split_tokens: Vec<u32>,
    pub name_tokens: Vec<u32>,
    pub docs: Vec<DocBlock>,
}

#[derive(Clone, Debug)]
pub struct DocBlock {
    pub range: SourceRange,
    pub text: Arc<str>,
}

impl Program {
    pub fn doc_at(&self, start: u32) -> Option<&str> {
        let at = self
            .docs
            .binary_search_by_key(&start, |doc| doc.range.start.offset)
            .ok()?;
        Some(&self.docs[at].text)
    }
}
