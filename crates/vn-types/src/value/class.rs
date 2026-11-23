use super::RuntimeString;
use crate::vm_value::VmValue;
use rustc_hash::FxHashMap as HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use vn_core::AccessorKind;

static NEXT_CLASS_ID: AtomicU32 = AtomicU32::new(1);

thread_local! {
    static CLASS_REGISTRY: RefCell<HashMap<u32, std::rc::Weak<ClassObj>>> =
        RefCell::new(HashMap::default());
}

#[derive(Debug)]
pub struct ClassObj {
    pub id: u32,
    pub name: String,
    pub is_native: bool,
    pub superclass: RefCell<Option<Rc<ClassObj>>>,
    pub vtable: RefCell<Vec<VmValue>>,
    pub vtable_owners: RefCell<Vec<Option<Rc<ClassObj>>>>,
    pub method_map: RefCell<HashMap<RuntimeString, usize>>,
    pub statics: RefCell<HashMap<RuntimeString, VmValue>>,
    pub static_fields: RefCell<Vec<RuntimeString>>,
    pub vtable_version: AtomicU32,
    pub frozen: AtomicBool,
    pub root_shape: RefCell<Rc<super::shape::Shape>>,
    pub static_accessors: [RefCell<HashMap<RuntimeString, VmValue>>; 2],

    pub ctor_cache: RefCell<Option<(u32, Option<VmValue>)>>,

    pub ctor_rt_cache: RefCell<Option<CtorRtCacheEntry>>,

    pub instance_shape_cache: RefCell<Option<(Rc<super::shape::Shape>, usize)>>,
    pub layout: RefCell<Rc<vn_core::layout::ClassLayout>>,
}

pub type CtorRtCacheEntry = (u32, Option<Rc<dyn std::any::Any>>);

impl ClassObj {
    pub fn new(name: impl Into<String>) -> Self {
        ClassObj {
            id: NEXT_CLASS_ID.fetch_add(1, Ordering::Relaxed),
            name: name.into(),
            is_native: false,
            superclass: RefCell::new(None),
            vtable_version: AtomicU32::new(1),
            frozen: AtomicBool::new(false),
            vtable: RefCell::new(Vec::new()),
            vtable_owners: RefCell::new(Vec::new()),
            method_map: RefCell::new(HashMap::default()),
            statics: RefCell::new(HashMap::default()),
            static_fields: RefCell::new(Vec::new()),
            root_shape: RefCell::new(super::shape::root_shape()),
            static_accessors: [
                RefCell::new(HashMap::default()),
                RefCell::new(HashMap::default()),
            ],
            ctor_cache: RefCell::new(None),
            ctor_rt_cache: RefCell::new(None),
            instance_shape_cache: RefCell::new(None),
            layout: RefCell::new(Rc::new(vn_core::layout::ClassLayout::from_fields(&[]))),
        }
    }
    pub fn new_native(name: impl Into<String>) -> Self {
        let mut cls = Self::new(name);
        cls.is_native = true;
        cls
    }

    pub fn new_rc(name: impl Into<String>) -> Rc<Self> {
        let cls = Rc::new(Self::new(name));
        cls.init_root_shape();
        CLASS_REGISTRY.with(|reg| {
            reg.borrow_mut().insert(cls.id, Rc::downgrade(&cls));
        });
        cls
    }

    pub fn new_native_rc(name: impl Into<String>) -> Rc<Self> {
        let cls = Rc::new(Self::new_native(name));
        cls.init_root_shape();
        CLASS_REGISTRY.with(|reg| {
            reg.borrow_mut().insert(cls.id, Rc::downgrade(&cls));
        });
        cls
    }

    pub fn find_by_id(id: u32) -> Option<Rc<Self>> {
        CLASS_REGISTRY.with(|reg| reg.borrow().get(&id).and_then(|w| w.upgrade()))
    }

    pub fn init_root_shape(self: &Rc<Self>) {
        let mut root = self.root_shape.borrow_mut();
        if root.class.is_none() {
            *root = super::shape::Shape::create(Some(self.clone()), HashMap::default());
        }
    }

    pub fn layout(&self) -> Rc<vn_core::layout::ClassLayout> {
        Rc::clone(&self.layout.borrow())
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen.load(Ordering::Relaxed)
    }

    pub fn freeze(&self) {
        self.frozen.store(true, Ordering::Relaxed);
    }

    pub fn mark_value(self: &Rc<Self>) -> bool {
        if self.is_frozen() {
            return false;
        }
        let mut layout = (*self.layout.borrow()).as_ref().clone();
        layout.is_value = true;
        self.set_layout(Rc::new(layout))
    }

    pub fn set_layout(self: &Rc<Self>, layout: Rc<vn_core::layout::ClassLayout>) -> bool {
        if self.is_frozen() {
            return false;
        }
        super::instance::invalidate_layout_cache();
        let properties: HashMap<RuntimeString, usize> = layout
            .fields
            .iter()
            .enumerate()
            .map(|(slot, f)| (f.name.clone(), slot))
            .collect();
        *self.root_shape.borrow_mut() = super::shape::Shape::create(Some(self.clone()), properties);
        *self.instance_shape_cache.borrow_mut() = None;
        *self.layout.borrow_mut() = layout;
        true
    }

    pub fn extend_layout(
        self: &Rc<Self>,
        own: &[(RuntimeString, Option<vn_core::FieldKind>)],
    ) -> bool {
        if self.is_frozen() {
            return false;
        }
        let mut fields: Vec<(RuntimeString, Option<vn_core::FieldKind>)> = self
            .layout()
            .fields
            .iter()
            .map(|f| (f.name.clone(), f.kind))
            .collect();
        fields.extend(own.iter().cloned());
        self.set_layout(Rc::new(vn_core::layout::ClassLayout::from_fields(&fields)))
    }

    pub fn for_each_value_mut(&self, f: &mut dyn FnMut(&mut VmValue)) {
        for v in self.vtable.borrow_mut().iter_mut() {
            f(v);
        }
        for v in self.statics.borrow_mut().values_mut() {
            f(v);
        }
        for v in self.static_accessors[0].borrow_mut().values_mut() {
            f(v);
        }
        for v in self.static_accessors[1].borrow_mut().values_mut() {
            f(v);
        }
        *self.ctor_cache.borrow_mut() = None;
    }

    pub fn add_method(&self, name: impl Into<Arc<str>>, value: VmValue) -> bool {
        self.add_method_with_owner(name, value, None)
    }

    pub fn add_method_with_owner(
        &self,
        name: impl Into<Arc<str>>,
        value: VmValue,
        owner: Option<Rc<ClassObj>>,
    ) -> bool {
        if self.is_frozen() {
            return false;
        }
        let name: Arc<str> = name.into();
        let mut method_map = self.method_map.borrow_mut();
        let mut vtable = self.vtable.borrow_mut();
        let mut vtable_owners = self.vtable_owners.borrow_mut();
        if let Some(&idx) = method_map.get(&name) {
            vtable[idx] = value;
            vtable_owners[idx] = owner;
        } else {
            let idx = vtable.len();
            vtable.push(value);
            vtable_owners.push(owner);
            method_map.insert(name, idx);
        }
        drop((method_map, vtable, vtable_owners));
        self.vtable_version.fetch_add(1, Ordering::Relaxed);
        true
    }

    pub fn add_accessor(&self, kind: AccessorKind, name: &str, value: VmValue) -> bool {
        self.add_accessor_with_owner(kind, name, value, None)
    }

    pub fn add_accessor_with_owner(
        &self,
        kind: AccessorKind,
        name: &str,
        value: VmValue,
        owner: Option<Rc<ClassObj>>,
    ) -> bool {
        self.add_method_with_owner(kind.key(name), value, owner)
    }

    pub fn add_getter(&self, name: &str, value: VmValue) -> bool {
        self.add_accessor(AccessorKind::Get, name, value)
    }

    pub fn accessor_slot(&self, kind: AccessorKind, name: &str) -> Option<usize> {
        kind.with_key(name, |key| self.method_map.borrow().get(key).copied())
    }

    pub fn own_method_names(&self) -> Vec<RuntimeString> {
        self.method_map
            .borrow()
            .keys()
            .filter(|key| AccessorKind::of_key(key).is_none())
            .cloned()
            .collect()
    }

    fn register_static_field(&self, name: impl Into<Arc<str>>) -> Option<Arc<str>> {
        if self.is_frozen() {
            return None;
        }
        let name: Arc<str> = name.into();
        let mut fields = self.static_fields.borrow_mut();
        if !fields.contains(&name) {
            fields.push(name.clone());
        }
        Some(name)
    }

    fn static_accessor_map(&self, kind: AccessorKind) -> &RefCell<HashMap<RuntimeString, VmValue>> {
        &self.static_accessors[kind as usize]
    }

    pub fn add_static(&self, name: impl Into<Arc<str>>, value: VmValue) -> bool {
        let Some(name) = self.register_static_field(name) else {
            return false;
        };
        self.statics.borrow_mut().insert(name, value);
        true
    }

    pub fn add_static_accessor(
        &self,
        kind: AccessorKind,
        name: impl Into<Arc<str>>,
        value: VmValue,
    ) -> bool {
        let Some(name) = self.register_static_field(name) else {
            return false;
        };
        self.static_accessor_map(kind)
            .borrow_mut()
            .insert(name, value);
        true
    }

    pub fn find_static_accessor(&self, kind: AccessorKind, name: &str) -> Option<VmValue> {
        if let Some(v) = self.static_accessor_map(kind).borrow().get(name) {
            return Some(*v);
        }
        self.superclass
            .borrow()
            .as_ref()?
            .find_static_accessor(kind, name)
    }

    pub fn define_member(
        self: &Rc<Self>,
        def: vn_core::MemberDef,
        name: &str,
        value: VmValue,
    ) -> bool {
        match def {
            vn_core::MemberDef::Method { is_static: false } => {
                self.add_method_with_owner(name, value, Some(Rc::clone(self)))
            }
            vn_core::MemberDef::Method { is_static: true } => self.add_static(name, value),
            vn_core::MemberDef::Accessor {
                kind,
                is_static: false,
            } => self.add_accessor_with_owner(kind, name, value, Some(Rc::clone(self))),
            vn_core::MemberDef::Accessor {
                kind,
                is_static: true,
            } => self.add_static_accessor(kind, name, value),
        }
    }

    pub fn get_static(&self, name: &str) -> Option<VmValue> {
        self.statics.borrow().get(name).copied()
    }

    pub fn find_accessor(&self, kind: AccessorKind, name: &str) -> Option<VmValue> {
        kind.with_key(name, |key| self.find_method(key))
    }

    pub fn find_method(&self, name: &str) -> Option<VmValue> {
        if let Some(&idx) = self.method_map.borrow().get(name) {
            return Some(self.vtable.borrow()[idx]);
        }
        if let Some(super_cls) = &*self.superclass.borrow() {
            return super_cls.find_method(name);
        }
        None
    }

    pub fn constructor(&self) -> Option<VmValue> {
        if self.is_frozen() {
            if let Some((_, ctor)) = self.ctor_cache.borrow().as_ref() {
                return *ctor;
            }
        } else {
            let ver = self.vtable_version.load(Ordering::Relaxed);
            if let Some((cached_ver, ctor)) = self.ctor_cache.borrow().as_ref()
                && *cached_ver == ver
            {
                return *ctor;
            }
        }
        let ctor = self.find_method(vn_core::TokenKind::Constructor.as_str());
        *self.ctor_cache.borrow_mut() = Some((self.vtable_version.load(Ordering::Relaxed), ctor));
        ctor
    }
}

pub fn find_method_with_owner(class: &Rc<ClassObj>, name: &str) -> Option<(VmValue, Rc<ClassObj>)> {
    if let Some(&idx) = class.method_map.borrow().get(name) {
        return Some((class.vtable.borrow()[idx], class.clone()));
    }
    if let Some(super_cls) = &*class.superclass.borrow() {
        return find_method_with_owner(super_cls, name);
    }
    None
}
