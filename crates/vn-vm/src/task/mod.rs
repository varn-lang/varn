use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::value::VmValue;

mod cell;
mod errors;
mod lazy;

pub use cell::TaskCell;
pub(crate) use cell::{
    alloc_handle, gather_one, new_lazy, outcome_of_cell, settle, track_cell, Status,
};
pub(crate) use errors::{derives_from_error, error_value};
pub use lazy::LazyTask;
pub(crate) use lazy::TaskArgs;

pub(crate) type Outcome = Result<VmValue, VmValue>;
pub(crate) type Inbox = Rc<RefCell<VecDeque<u64>>>;
