use crate::error::{RuntimeError, VmResult};
use crate::exec::props::bind_method_to_receiver;
use crate::heap::{Heap, HeapObj};
use crate::value::VmValue;
use std::rc::Rc;
use vn_types::ClassObj;

pub(crate) fn op_class(name: &str, heap: &mut Heap) -> VmValue {
    let cls = ClassObj::new_rc(name);
    VmValue::from_heap(heap.alloc(HeapObj::Class(cls)))
}

pub(crate) fn op_define_member(
    class_nv: VmValue,
    name: &str,
    value_nv: VmValue,
    def: vn_core::MemberDef,
    heap: &mut Heap,
) -> VmResult<()> {
    let cls = get_class_arc(class_nv, heap)?;
    if def == (vn_core::MemberDef::Method { is_static: true })
        && value_nv.is_heap()
        && let Some(HeapObj::EnumVariant(ev)) = heap.get_mut(value_nv.as_heap())
    {
        ev.enum_class_id = Some(cls.id);
    }
    if !cls.define_member(def, name, value_nv) {
        return Err(RuntimeError::new(format!(
            "cannot define {} '{name}': class '{}' is frozen",
            def.label(),
            cls.name
        )));
    }
    if class_nv.is_heap() {
        heap.write_barrier(class_nv.as_heap(), value_nv);
    }
    Ok(())
}

pub(crate) fn op_inherit(
    subclass_nv: VmValue,
    superclass_nv: VmValue,
    heap: &mut Heap,
) -> VmResult<()> {
    let superclass = get_class_arc(superclass_nv, heap)?;
    if subclass_nv.is_heap()
        && let Some(HeapObj::Class(sub)) = heap.get_mut(subclass_nv.as_heap())
    {
        if sub.is_frozen() {
            return Err(RuntimeError::new(format!(
                "cannot inherit into frozen class '{}'",
                sub.name
            )));
        }
        *sub.superclass.borrow_mut() = Some(superclass.clone());

        *sub.vtable.borrow_mut() = superclass.vtable.borrow().clone();
        *sub.vtable_owners.borrow_mut() = superclass.vtable_owners.borrow().clone();
        *sub.method_map.borrow_mut() = superclass.method_map.borrow().clone();

        if !sub.set_layout(superclass.layout()) {
            return Err(RuntimeError::new(format!(
                "cannot inherit into frozen class '{}'",
                sub.name
            )));
        }
    } else {
        return Err(RuntimeError::new("OpInherit: not a class"));
    }
    heap.remember(subclass_nv.as_heap());
    Ok(())
}

pub(crate) fn op_declare_layout(
    class_nv: VmValue,
    layout: Rc<vn_core::layout::ClassLayout>,
    heap: &mut Heap,
) -> VmResult<()> {
    if class_nv.is_heap()
        && let Some(HeapObj::Class(cls)) = heap.get_mut(class_nv.as_heap())
    {
        if !cls.set_layout(layout) {
            return Err(RuntimeError::new(format!(
                "cannot declare layout: class '{}' is frozen",
                cls.name
            )));
        }
        return Ok(());
    }
    Err(RuntimeError::new(format!(
        "OpDeclareLayout: expected class, got {}",
        crate::exec::props::meta::type_name(class_nv, heap)
    )))
}

pub(crate) fn op_alloc_instance(class_nv: VmValue, heap: &mut Heap) -> VmResult<VmValue> {
    if class_nv.is_heap()
        && let Some(HeapObj::Class(cls)) = heap.get(class_nv.as_heap())
    {
        let cls = cls.clone();
        return Ok(VmValue::from_heap(heap.alloc_instance(&cls).0));
    }
    Err(RuntimeError::new(format!(
        "new: expected a class, got {}",
        crate::exec::props::meta::type_name(class_nv, heap)
    )))
}

pub(crate) fn pool_layout(
    proto: &vn_types::FunctionProto,
    idx: usize,
) -> VmResult<Rc<vn_core::layout::ClassLayout>> {
    match proto.chunk.constants.get(idx) {
        Some(vn_types::PoolEntry::Layout(layout)) => Ok(Rc::clone(layout)),
        Some(
            vn_types::PoolEntry::Literal(_)
            | vn_types::PoolEntry::Function(_)
            | vn_types::PoolEntry::Shape(_)
            | vn_types::PoolEntry::Variant(_),
        )
        | None => Err(RuntimeError::new("DeclareLayout: not a layout constant")),
    }
}

pub(crate) fn home_class(
    receiver: Option<Rc<ClassObj>>,
    running: &crate::closure::VmClosure,
    heap: &Heap,
) -> Option<Rc<ClassObj>> {
    let runs = |held: &VmValue| {
        heap.closure_of(*held)
            .is_some_and(|k| Rc::ptr_eq(&k.proto, &running.proto))
    };
    let mut class = receiver;
    while let Some(c) = class {
        if let Some(at) = c.vtable.borrow().iter().position(runs) {
            let owner = c.vtable_owners.borrow().get(at).cloned().flatten();
            return owner.or(Some(c.clone()));
        }
        class = c.superclass.borrow().clone();
    }
    None
}

pub(crate) fn op_get_super(
    cls: Rc<ClassObj>,
    name: &str,
    receiver_nv: VmValue,
    heap: &mut Heap,
) -> VmResult<VmValue> {
    let super_cls = cls
        .superclass
        .borrow()
        .as_ref()
        .ok_or_else(|| RuntimeError::new("no superclass"))?
        .clone();
    let method = match super_cls.find_method(name) {
        Some(method) => method,
        None if name == vn_core::TokenKind::Constructor.as_str() => {
            return Ok(heap.alloc_native_fn(
                default_constructor,
                vn_core::TokenKind::Constructor.as_str(),
            ));
        }
        None => {
            return Err(RuntimeError::new(format!(
                "super: method '{}' not found",
                name
            )));
        }
    };
    Ok(bind_method_to_receiver(
        heap,
        receiver_nv,
        method,
        Some(super_cls.clone()),
    ))
}

pub(crate) fn op_get_super_slot(
    cls: Rc<ClassObj>,
    slot: usize,
    receiver_nv: VmValue,
    heap: &mut Heap,
) -> VmResult<VmValue> {
    let super_cls = cls
        .superclass
        .borrow()
        .clone()
        .ok_or_else(|| RuntimeError::new("no superclass"))?;
    let vtable = super_cls.vtable.borrow();
    let method = vtable.get(slot).copied().ok_or_else(|| {
        RuntimeError::new(format!(
            "GetSuperSlot: slot {slot} out of range for class '{}' (vtable len {})",
            super_cls.name,
            vtable.len()
        ))
    })?;
    let owner = super_cls
        .vtable_owners
        .borrow()
        .get(slot)
        .cloned()
        .flatten();
    drop(vtable);
    Ok(bind_method_to_receiver(
        heap,
        receiver_nv,
        method,
        owner.or(Some(super_cls)),
    ))
}

fn default_constructor(
    _ctx: &mut dyn vn_types::NativeCtx,
    _args: &[VmValue],
) -> vn_types::NativeFnResult {
    Ok(VmValue::null())
}

pub(crate) fn op_freeze_class(class_nv: VmValue, heap: &Heap) -> VmResult<()> {
    let cls = get_class_arc(class_nv, heap)?;
    cls.freeze();
    Ok(())
}

fn get_class_arc(nv: VmValue, heap: &Heap) -> VmResult<Rc<ClassObj>> {
    if nv.is_heap()
        && let Some(HeapObj::Class(c)) = heap.get(nv.as_heap())
    {
        return Ok(c.clone());
    }
    Err(RuntimeError::new("expected class"))
}
