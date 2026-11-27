use std::cell::Cell;
use std::rc::Rc;

use vn_types::{ClassObj, FunctionProto};

use crate::heap::HeapInner;
use crate::value::VmValue;

const INLINE_ARGS: usize = 3;

pub(crate) enum TaskArgs {
    Inline(u8, [Cell<VmValue>; INLINE_ARGS]),
    Heap(Vec<Cell<VmValue>>),
}

impl TaskArgs {
    pub(crate) fn collect(mut values: impl ExactSizeIterator<Item = VmValue>) -> Self {
        let len = values.len();
        if len > INLINE_ARGS {
            return TaskArgs::Heap(values.map(Cell::new).collect());
        }
        let inline = [
            Cell::new(VmValue::null()),
            Cell::new(VmValue::null()),
            Cell::new(VmValue::null()),
        ];
        for slot in inline.iter().take(len) {
            if let Some(v) = values.next() {
                slot.set(v);
            }
        }
        TaskArgs::Inline(len as u8, inline)
    }

    pub(crate) fn cells(&self) -> &[Cell<VmValue>] {
        match self {
            TaskArgs::Inline(len, inline) => &inline[..*len as usize],
            TaskArgs::Heap(values) => values,
        }
    }
}

pub struct LazyTask {
    pub(crate) proto: Rc<FunctionProto>,
    pub(crate) upvalues: Vec<crate::closure::VmUpvalue>,
    pub(crate) module_base: u32,
    pub(crate) args: TaskArgs,
    pub(crate) current_class: Option<Rc<ClassObj>>,
}

impl std::fmt::Debug for LazyTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LazyTask({})",
            self.proto.name.as_deref().unwrap_or("<anon>")
        )
    }
}

impl LazyTask {
    pub(crate) fn trace_values(&self, f: &mut dyn FnMut(VmValue)) {
        for cell in self.args.cells() {
            f(cell.get());
        }
        for uv in &self.upvalues {
            if let Ok(inner) = uv.inner.try_borrow() {
                f(inner.value);
            }
        }
    }

    pub(super) fn holds_young_ref(&self, heap: &HeapInner) -> bool {
        let mut found = false;
        self.trace_values(&mut |v| found |= heap.is_young(v));
        found
    }
}
