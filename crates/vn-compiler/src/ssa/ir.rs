mod control;
mod func;
mod ids;
mod ops;

pub use control::Terminator;
pub use func::{Block, Inst, SsaFunc};
pub use ids::{BlockId, SsaVar, Value, ValueDef};
pub use ops::InstKind;
