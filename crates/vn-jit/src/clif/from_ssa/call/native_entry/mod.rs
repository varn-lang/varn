mod direct;
mod frames;
mod marshal;
mod native;

pub(crate) use direct::{emit_direct, DirectCall};
use frames::*;
use marshal::*;
pub(crate) use native::{emit, NativeCall};
