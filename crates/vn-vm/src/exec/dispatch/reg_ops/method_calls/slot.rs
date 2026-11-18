use crate::closure::VmClosure;
use crate::error::{RuntimeError, VmResult};
use crate::exec::ctx::ExecCtx;
use crate::exec::method_args::{MethodArgs, MethodOutcome};
use crate::value::VmValue;
use std::rc::Rc;
use vn_types::chunk::{CacheEntry, ICKind};
use vn_types::value::ClassObj;

impl ExecCtx {
    pub(crate) fn exec_invoke_slot_reg(
        &mut self,
        this_val: VmValue,
        slot: usize,
        base: usize,
        cs: usize,
        arg_start: usize,
        arg_count: usize,
        dest: usize,
        frame_idx: usize,
        closure: &VmClosure,
    ) -> VmResult<bool> {
        let args = MethodArgs::Regs {
            base,
            start: arg_start,
            count: arg_count,
        };
        match self.invoke_slot_slow(this_val, slot, cs, args, frame_idx, closure)? {
            MethodOutcome::Value(v) => {
                self.stack.unbox_into_reg(base, dest, v)?;
                Ok(false)
            }
            MethodOutcome::FramePushed => {
                self.frames.last_mut().expect("a pushed frame").return_reg = dest as u16;
                Ok(true)
            }
        }
    }

    pub(crate) fn exec_super_invoke_slot_reg(
        &mut self,
        this_val: VmValue,
        slot: usize,
        base: usize,
        arg_start: usize,
        arg_count: usize,
        dest: usize,
        frame_idx: usize,
    ) -> VmResult<bool> {
        let args = MethodArgs::Regs {
            base,
            start: arg_start,
            count: arg_count,
        };
        match self.invoke_super_slot_slow(this_val, slot, args, frame_idx)? {
            MethodOutcome::Value(v) => {
                self.stack.unbox_into_reg(base, dest, v)?;
                Ok(false)
            }
            MethodOutcome::FramePushed => {
                self.frames.last_mut().expect("a pushed frame").return_reg = dest as u16;
                Ok(true)
            }
        }
    }

    pub(crate) fn invoke_slot_slow(
        &mut self,
        this_val: VmValue,
        slot: usize,
        cs: usize,
        args: MethodArgs<'_>,
        frame_idx: usize,
        closure: &VmClosure,
    ) -> VmResult<MethodOutcome> {
        let receiver_class = crate::exec::props::get_class(this_val, &self.heap);
        let Some(cls) = receiver_class else {
            return Err(RuntimeError::new("InvokeSlot: value has no class"));
        };
        if let Some((method_val, owner)) = probe_slot(closure, cs, &cls, slot) {
            return self.dispatch_slot_method(method_val, owner, this_val, args, frame_idx);
        }
        self.record_ic_miss_callmethod();
        let Some((method_val, owner)) = slot_method(&cls, slot) else {
            return Err(slot_out_of_range(&cls, slot));
        };
        record_slot(closure, cs, &cls, slot);
        self.dispatch_slot_method(method_val, owner, this_val, args, frame_idx)
    }

    pub(crate) fn invoke_super_slot_slow(
        &mut self,
        this_val: VmValue,
        slot: usize,
        args: MethodArgs<'_>,
        frame_idx: usize,
    ) -> VmResult<MethodOutcome> {
        let receiver_class = crate::exec::props::get_class(this_val, &self.heap);
        let running = self.frames[frame_idx].closure();
        let home = self.frames[frame_idx]
            .current_class
            .clone()
            .or_else(|| crate::exec::class::home_class(receiver_class.clone(), running, &self.heap))
            .or(receiver_class)
            .ok_or_else(|| RuntimeError::new("super: 'this' has no class"))?;
        let parent = home.superclass.borrow().clone();
        let Some(super_cls) = parent else {
            return Err(RuntimeError::new("no superclass"));
        };
        let vtable = super_cls.vtable.borrow();
        if slot >= vtable.len() {
            return Err(slot_out_of_range(&super_cls, slot));
        }
        let method_val = vtable[slot];
        let owner = super_cls
            .vtable_owners
            .borrow()
            .get(slot)
            .cloned()
            .flatten();
        drop(vtable);
        self.dispatch_super_method(
            method_val,
            owner.or(Some(super_cls)),
            this_val,
            args,
            frame_idx,
        )
    }

    fn dispatch_slot_method(
        &mut self,
        method_val: VmValue,
        owner: Option<Rc<ClassObj>>,
        this_val: VmValue,
        args: MethodArgs<'_>,
        frame_idx: usize,
    ) -> VmResult<MethodOutcome> {
        if let Some(nc) = self.heap.closure_of(method_val).cloned()
            && !nc.proto.is_generator
            && !nc.proto.is_async
            && args.len() <= nc.proto.arity
        {
            self.record_ic_hit_callmethod();
            return self.invoke_vm_method_fast(nc, owner, this_val, args, "#slot");
        }
        if let Some((f, _, mask)) = self.heap.native_of(method_val) {
            self.record_ic_hit_callmethod();
            return self.call_native_method(f, mask, this_val, args, "#slot");
        }
        self.finish_generic_method_call(method_val, this_val, args, frame_idx)
    }

    fn dispatch_super_method(
        &mut self,
        method_val: VmValue,
        owner: Option<Rc<ClassObj>>,
        this_val: VmValue,
        args: MethodArgs<'_>,
        frame_idx: usize,
    ) -> VmResult<MethodOutcome> {
        if let Some(nc) = self.heap.closure_of(method_val).cloned()
            && !nc.proto.is_generator
            && !nc.proto.is_async
            && args.len() <= nc.proto.arity
        {
            return self.invoke_vm_method_fast(nc, owner, this_val, args, "super");
        }
        if let Some((f, _, mask)) = self.heap.native_of(method_val) {
            return self.call_native_method(f, mask, this_val, args, "super");
        }
        self.finish_generic_method_call(method_val, this_val, args, frame_idx)
    }
}

fn slot_method(cls: &Rc<ClassObj>, slot: usize) -> Option<(VmValue, Option<Rc<ClassObj>>)> {
    let vtable = cls.vtable.borrow();
    if slot >= vtable.len() {
        return None;
    }
    let method = vtable[slot];
    let owner = cls.vtable_owners.borrow().get(slot).cloned().flatten();
    Some((method, owner))
}

fn slot_out_of_range(cls: &Rc<ClassObj>, slot: usize) -> RuntimeError {
    RuntimeError::new(format!(
        "InvokeSlot: slot {slot} out of range for class '{}' (vtable len {})",
        cls.name,
        cls.vtable.borrow().len(),
    ))
}

fn probe_slot(
    closure: &VmClosure,
    cs: usize,
    cls: &Rc<ClassObj>,
    slot: usize,
) -> Option<(VmValue, Option<Rc<ClassObj>>)> {
    if cs >= closure.ic_cache_len() {
        return None;
    }
    if closure
        .feedback
        .borrow()
        .sites
        .get(cs)
        .is_some_and(|s| s.megamorphic)
    {
        return None;
    }
    let ic = unsafe { &*closure.ic_cache.as_ptr() };
    for entry in &ic[cs].entries {
        if entry.is_class == ICKind::SLOT_METHOD
            && entry.id == cls.id
            && entry.slot as usize == slot
            && let Some(cached) = entry.class.clone()
        {
            return slot_method(&cached, slot);
        }
    }
    None
}

fn record_slot(closure: &VmClosure, cs: usize, cls: &Rc<ClassObj>, slot: usize) {
    if cs >= closure.ic_cache_len() {
        return;
    }
    if closure
        .feedback
        .borrow()
        .sites
        .get(cs)
        .is_some_and(|s| s.megamorphic)
    {
        return;
    }
    let entry = CacheEntry {
        id: cls.id,
        slot: slot as u16,
        is_class: ICKind::SLOT_METHOD,
        vtable_ver: 0,
        class: Some(cls.clone()),
        ..Default::default()
    };
    closure.ic_cache.borrow_mut()[cs].find_or_insert(entry);
    closure.feedback.borrow_mut().observe(cs, cls.id);
}
