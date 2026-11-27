mod binding;
mod matching;
mod sequence;
mod variants;

use vn_sem::types::Type;

pub(crate) use sequence::SequenceShape;

pub(super) type VariantSubst = (
    Vec<(std::sync::Arc<str>, Type)>,
    rustc_hash::FxHashMap<std::sync::Arc<str>, Type>,
);
