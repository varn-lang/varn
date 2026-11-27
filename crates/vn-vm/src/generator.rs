use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::closure::VmClosure;
use crate::error::{RuntimeError, VmResult};
use crate::exec::scheduler::{freeze_into, thaw_from, FrozenFrame};
use crate::exec::{ExecCtx, VmSuspend};
use crate::frame::CallFrame;
use crate::value::VmValue;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Fresh,
    Suspended,
    Running,
    Done,
}

struct Start {
    closure: Rc<VmClosure>,
    args: Vec<VmValue>,
    current_class: Option<Rc<vn_types::ClassObj>>,
}

pub struct GenState {
    status: Cell<Status>,
    start: RefCell<Option<Start>>,
    saved: RefCell<FrozenFrame>,
    resume_reg: Cell<u8>,
    pub(crate) is_async: bool,
    closing: Cell<bool>,
    owner: Cell<u64>,
}

impl std::fmt::Debug for GenState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Generator(async={})", self.is_async)
    }
}

impl GenState {
    pub(crate) fn new(
        closure: Rc<VmClosure>,
        args: Vec<VmValue>,
        current_class: Option<Rc<vn_types::ClassObj>>,
    ) -> Rc<Self> {
        let is_async = closure.proto.is_async;
        Rc::new(GenState {
            status: Cell::new(Status::Fresh),
            start: RefCell::new(Some(Start {
                closure,
                args,
                current_class,
            })),
            saved: RefCell::new(FrozenFrame::empty()),
            resume_reg: Cell::new(0),
            is_async,
            closing: Cell::new(false),
            owner: Cell::new(0),
        })
    }

    pub(crate) fn set_owner(&self, owner: vn_types::HeapRef) {
        self.owner.set(owner.addr());
    }

    pub(crate) fn owner(&self) -> u64 {
        self.owner.get()
    }

    pub(crate) fn is_suspended(&self) -> bool {
        self.status.get() == Status::Suspended
    }

    pub(crate) fn begin_close(&self) {
        self.closing.set(true);
    }

    pub(crate) fn resume_input(&self) -> VmValue {
        VmValue::from_bool(self.closing.get())
    }

    pub(crate) fn close(&self) {
        self.finish();
    }

    fn finish(&self) {
        self.status.set(Status::Done);
        if let Ok(mut start) = self.start.try_borrow_mut() {
            *start = None;
        }
        if let Ok(mut saved) = self.saved.try_borrow_mut() {
            *saved = FrozenFrame::empty();
        }
    }

    pub(crate) fn trace(&self, value: &mut dyn FnMut(VmValue)) {
        match self.status.get() {
            Status::Fresh => {
                let Ok(start) = self.start.try_borrow() else {
                    return;
                };
                if let Some(start) = &*start {
                    start.args.iter().for_each(|&v| value(v));
                    for uv in &start.closure.upvalues {
                        value(uv.inner.borrow().value);
                    }
                }
            }
            Status::Suspended => {
                if let Ok(saved) = self.saved.try_borrow() {
                    saved.trace(value);
                }
            }
            Status::Running | Status::Done => {}
        }
    }
}

pub(crate) enum Entry {
    Done(VmValue),
    Mounted(usize),
}

impl ExecCtx {
    pub(crate) fn resume_generator(
        &mut self,
        generator: &Rc<GenState>,
        input: VmValue,
    ) -> VmResult<VmValue> {
        match self.enter_generator(generator, input)? {
            Entry::Done(v) => Ok(v),
            Entry::Mounted(depth) => self.run_generator(generator, depth),
        }
    }

    pub(crate) fn enter_generator(
        &mut self,
        generator: &Rc<GenState>,
        input: VmValue,
    ) -> VmResult<Entry> {
        let status = generator.status.replace(Status::Running);
        let depth = self.frames.len();
        match status {
            Status::Done => {
                generator.status.set(Status::Done);
                Ok(Entry::Done(VmValue::null()))
            }
            Status::Running => Err(RuntimeError::new("generator is already running")),
            Status::Fresh => {
                let Some(Start {
                    closure,
                    args,
                    current_class,
                }) = generator.start.borrow_mut().take()
                else {
                    generator.status.set(Status::Done);
                    return Ok(Entry::Done(VmValue::null()));
                };
                if self.call_depth() >= crate::frame::MAX_CALL_DEPTH {
                    generator.finish();
                    return Err(crate::frame::call_depth_exceeded());
                }
                let alloc = self.stack.push_frame(&closure.proto);
                let nregs = closure.proto.register_count as usize;
                self.stack.adopt_values(alloc, 0, &args, nregs);
                let mut frame = CallFrame::new_owned(closure, alloc);
                frame.current_class = current_class;
                self.frames.push(frame);
                Ok(Entry::Mounted(depth))
            }
            Status::Suspended => {
                thaw_from(self, &mut generator.saved.borrow_mut());
                let base = self.frames[depth].base;
                let _ = self
                    .stack
                    .unbox_into_reg(base, generator.resume_reg.get() as usize, input);
                Ok(Entry::Mounted(depth))
            }
        }
    }

    pub(crate) fn run_generator(
        &mut self,
        generator: &Rc<GenState>,
        depth: usize,
    ) -> VmResult<VmValue> {
        crate::exec::scheduler::request_osr(self, vn_types::ssa::OsrMode::Resume);
        loop {
            let outer = std::mem::replace(&mut self.jit_suspend_buf, std::ptr::null_mut());
            let ran = self.run_until(depth);
            self.jit_suspend_buf = outer;
            if let Err(e) = ran {
                self.abandon_generator(generator, depth);
                return Err(e);
            }
            match self.vm_suspend.take() {
                Some(VmSuspend::Yield { value, dest_reg }) => {
                    return self.park_generator(generator, depth, value, dest_reg);
                }
                Some(VmSuspend::Await { .. }) => {
                    self.abandon_generator(generator, depth);
                    return Err(RuntimeError::new(
                        "cannot use `await` inside a sync generator — declare it `async function*`",
                    ));
                }
                Some(VmSuspend::DebugBreak) => {}
                None => {
                    generator.finish();
                    return Ok(VmValue::null());
                }
            }
        }
    }

    pub(crate) fn close_generator_state(&mut self, generator: &Rc<GenState>) -> VmResult<()> {
        if !generator.is_suspended() {
            generator.close();
            return Ok(());
        }
        let ran = self.resume_generator(generator, VmValue::from_bool(true));
        generator.close();
        ran.map(|_| ())
    }

    pub(crate) fn step_generator_task(&mut self, generator: &Rc<GenState>) -> VmValue {
        let output = crate::task::TaskCell::pending();
        crate::exec::scheduler::queue_for(self).push_ready(crate::exec::scheduler::ReadyTask {
            start: crate::exec::scheduler::Start::Step(Rc::clone(generator)),
            output: Rc::clone(&output),
            step: None,
        });
        crate::task::alloc_handle(&mut self.heap, output)
    }

    pub(crate) fn finish_step(
        &mut self,
        generator: &Rc<GenState>,
        suspend: Option<VmSuspend>,
    ) -> VmResult<VmValue> {
        match suspend {
            Some(VmSuspend::Yield { .. }) if generator.closing.get() => {
                generator.close();
                Ok(VmValue::null())
            }
            Some(VmSuspend::Yield { value, dest_reg }) => {
                self.park_generator(generator, 0, value, dest_reg)
            }
            Some(VmSuspend::Await { .. }) | Some(VmSuspend::DebugBreak) | None => {
                generator.close();
                Ok(VmValue::null())
            }
        }
    }

    pub(crate) fn generator_entry(&mut self, depth: usize) -> Option<usize> {
        let frame = &self.frames[depth];
        let closure = frame.closure();
        if self.settings.no_jit {
            return None;
        }
        let entry = if frame.ip == 0 {
            closure.hot_jit_fn(&self.settings)
        } else {
            closure.osr_jit_fn(
                vn_types::ssa::OsrRequest {
                    ip: frame.ip,
                    mode: vn_types::ssa::OsrMode::Resume,
                },
                &self.settings,
            )
        }?;
        let owner = depth + 1;
        self.try_handlers.retain(|h| h.frame_depth != owner);
        Some(entry as usize)
    }

    pub(crate) fn leave_generator(
        &mut self,
        generator: &Rc<GenState>,
        depth: usize,
    ) -> VmResult<VmValue> {
        match self.vm_suspend.take() {
            Some(VmSuspend::Yield { value, dest_reg }) => {
                self.park_generator(generator, depth, value, dest_reg)
            }
            Some(VmSuspend::Await { .. }) | Some(VmSuspend::DebugBreak) | None => {
                self.abandon_generator(generator, depth);
                Ok(VmValue::null())
            }
        }
    }

    fn park_generator(
        &mut self,
        generator: &GenState,
        depth: usize,
        value: VmValue,
        dest_reg: u8,
    ) -> VmResult<VmValue> {
        if value.is_null() {
            self.abandon_generator(generator, depth);
            return Err(RuntimeError::new(
                "a generator cannot yield null: null marks the end of the iteration",
            ));
        }
        freeze_into(self, depth, false, &mut generator.saved.borrow_mut(), true)
            .map_err(RuntimeError::new)?;
        self.frames.pop();
        self.stack.pop_frame();
        generator.resume_reg.set(dest_reg);
        generator.status.set(Status::Suspended);
        if generator.owner() != 0 {
            self.heap
                .remember(unsafe { vn_types::HeapRef::from_addr_unchecked(generator.owner()) });
        }
        Ok(value)
    }

    pub(crate) fn abandon_generator(&mut self, generator: &GenState, depth: usize) {
        while self.frames.len() > depth {
            let frame = self.frames.pop().expect("a frame above the generator");
            self.drop_frame_storage(frame.base);
        }
        self.try_handlers.retain(|h| h.frame_depth <= depth);
        generator.finish();
    }
}
