mod combinators;
mod composite;
mod entry;
mod generics;
mod literals;
mod primary;
mod suffixes;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeContext {
    Annotation,
    Operand,
}

pub use entry::{parse_operand_type, parse_type};
pub use generics::{parse_type_args, parse_type_params};
