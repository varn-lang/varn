use cranelift_codegen::ir::{condcodes::IntCC, types, InstBuilder, MemFlagsData, Value};
use cranelift_frontend::FunctionBuilder;
use vn_types::register_meta::SlotKind;

use super::super::super::emit::{call_helper_void, HEAP_KIND, KIND_MASK};
use super::super::super::native_abi::{NativeClass, NativeShape};
use super::super::store::{clif_ty, drop_home_addrs};
use super::super::{heap, load_value, stack_exit, Ctx, Out};
use crate::NATIVE_FRAMELESS;
use vn_types::ssa::ValueId;

mod direct;
mod frames;
mod marshal;
mod native;

pub(crate) use direct::{emit_direct, DirectCall};
use frames::*;
use marshal::*;
pub(crate) use native::{emit, NativeCall};
