use std::collections::{BTreeMap, BTreeSet};

use cranelift_codegen::ir::{types, InstBuilder};
use cranelift_frontend::{FunctionBuilder, Variable};
use vn_types::ssa::{SsaOp, SsaProto, ValueId};

#[derive(Clone, Copy)]
pub(crate) struct View {
    pub data: Variable,
    pub len: Variable,
    pub disc: Variable,
}

pub(crate) struct Views {
    by_value: BTreeMap<ValueId, View>,
    instance_by_canon: BTreeMap<u32, Variable>,
    global_src: BTreeMap<ValueId, u32>,
    this_values: BTreeSet<ValueId>,
}

pub(crate) const GLOBAL_CANON_BIT: u32 = 0x8000_0000;
pub(crate) const THIS_CANON: u32 = 0x4000_0000;

impl Views {
    pub(super) fn declare(b: &mut FunctionBuilder, ssa: &SsaProto) -> Self {
        let mut global_src = BTreeMap::new();
        let mut this_values = BTreeSet::new();
        for inst in ssa.blocks.iter().flat_map(|blk| &blk.insts) {
            if let (Some(d), SsaOp::LoadGlobalIdx(slot)) = (inst.dest, &inst.op) {
                global_src.insert(d, *slot);
            }
            if let (Some(d), SsaOp::This) = (inst.dest, &inst.op) {
                this_values.insert(d);
            }
        }
        let canon = |o: ValueId| -> u32 {
            if this_values.contains(&o) {
                return THIS_CANON;
            }
            global_src
                .get(&o)
                .map(|s| GLOBAL_CANON_BIT | s)
                .unwrap_or(o.0)
        };
        let mut by_value = BTreeMap::new();
        let mut instance_by_canon = BTreeMap::new();
        for inst in ssa.blocks.iter().flat_map(|blk| &blk.insts) {
            match &inst.op {
                SsaOp::GetFixedField { object, .. } | SsaOp::SetFixedField { object, .. } => {
                    let c = canon(*object);
                    instance_by_canon
                        .entry(c)
                        .or_insert_with(|| b.declare_var(types::I64));
                }
                SsaOp::ConstInt(_)
                | SsaOp::ConstFloat(_)
                | SsaOp::ConstBool(_)
                | SsaOp::ConstNull
                | SsaOp::ConstStr(_)
                | SsaOp::Binary { .. }
                | SsaOp::Unary { .. }
                | SsaOp::SelfCall { .. }
                | SsaOp::Call { .. }
                | SsaOp::CallDirect { .. }
                | SsaOp::AllocInstance { .. }
                | SsaOp::LoadGlobalIdx(_)
                | SsaOp::ArrayGetIndex { .. }
                | SsaOp::ArraySetIndex { .. }
                | SsaOp::BytesSetIndex { .. }
                | SsaOp::BytesGetIndex { .. }
                | SsaOp::ConstChar(_)
                | SsaOp::ConstBigInt(_)
                | SsaOp::ConstDecimal(_)
                | SsaOp::MakeEnumVariant { .. }
                | SsaOp::IntrinsicCall { .. }
                | SsaOp::LoadNativeGlobalIdx(_)
                | SsaOp::Try { .. }
                | SsaOp::PopTry
                | SsaOp::CatchParam { .. }
                | SsaOp::StoreGlobalIdx { .. }
                | SsaOp::MakeClosure { .. }
                | SsaOp::LoadCaptured { .. }
                | SsaOp::StoreCaptured { .. }
                | SsaOp::LoadUpvalue(_)
                | SsaOp::StoreUpvalue { .. }
                | SsaOp::CloseUpvalues { .. }
                | SsaOp::Cast { .. }
                | SsaOp::Convert { .. }
                | SsaOp::IsNull { .. }
                | SsaOp::KindOf { .. }
                | SsaOp::TypeTest { .. }
                | SsaOp::ToString { .. }
                | SsaOp::IsArray { .. }
                | SsaOp::GetEnumTag { .. }
                | SsaOp::ObjectKeys { .. }
                | SsaOp::BuildStr { .. }
                | SsaOp::BuildArray { .. }
                | SsaOp::BuildMap { .. }
                | SsaOp::BuildObject { .. }
                | SsaOp::GetProperty { .. }
                | SsaOp::GetSlotMethod { .. }
                | SsaOp::SetProperty { .. }
                | SsaOp::GetIndex { .. }
                | SsaOp::SetIndex { .. }
                | SsaOp::ArrayLength { .. }
                | SsaOp::StrLength { .. }
                | SsaOp::BytesLength { .. }
                | SsaOp::MethodCall { .. }
                | SsaOp::InvokeSlot { .. }
                | SsaOp::CallNativeOp { .. }
                | SsaOp::ArrayPush { .. }
                | SsaOp::This
                | SsaOp::MakeClass { .. }
                | SsaOp::DeclareLayout { .. }
                | SsaOp::FreezeClass { .. }
                | SsaOp::DefineMember { .. }
                | SsaOp::GetSuper { .. }
                | SsaOp::GetSuperSlot { .. }
                | SsaOp::LoadGlobal(_)
                | SsaOp::StoreGlobal { .. }
                | SsaOp::BuildTuple { .. }
                | SsaOp::BuildArraySpread { .. }
                | SsaOp::BuildObjectSpread { .. }
                | SsaOp::ObjectMerge { .. }
                | SsaOp::ObjectRest { .. }
                | SsaOp::GetPropertyMaybe { .. }
                | SsaOp::AssertNotNull { .. }
                | SsaOp::Range { .. }
                | SsaOp::SuperCall { .. }
                | SsaOp::SuperMethodCall { .. }
                | SsaOp::SuperInvokeSlot { .. }
                | SsaOp::ExtensionCall { .. }
                | SsaOp::CallSpread { .. }
                | SsaOp::LoadModule { .. }
                | SsaOp::ModuleSlot { .. }
                | SsaOp::StoreModuleSlot { .. }
                | SsaOp::Await { .. }
                | SsaOp::Spawn { .. }
                | SsaOp::Yield { .. } => {}
            }
        }
        for inst in ssa.blocks.iter().flat_map(|blk| &blk.insts) {
            let object = match &inst.op {
                SsaOp::ArrayGetIndex { object, .. }
                | SsaOp::ArraySetIndex { object, .. }
                | SsaOp::BytesGetIndex { object, .. }
                | SsaOp::BytesSetIndex { object, .. } => *object,
                SsaOp::ConstInt(_)
                | SsaOp::ConstFloat(_)
                | SsaOp::ConstBool(_)
                | SsaOp::ConstNull
                | SsaOp::ConstStr(_)
                | SsaOp::Binary { .. }
                | SsaOp::Unary { .. }
                | SsaOp::SelfCall { .. }
                | SsaOp::Call { .. }
                | SsaOp::CallDirect { .. }
                | SsaOp::AllocInstance { .. }
                | SsaOp::LoadGlobalIdx(_)
                | SsaOp::ConstChar(_)
                | SsaOp::ConstBigInt(_)
                | SsaOp::ConstDecimal(_)
                | SsaOp::MakeEnumVariant { .. }
                | SsaOp::IntrinsicCall { .. }
                | SsaOp::LoadNativeGlobalIdx(_)
                | SsaOp::Try { .. }
                | SsaOp::PopTry
                | SsaOp::CatchParam { .. }
                | SsaOp::StoreGlobalIdx { .. }
                | SsaOp::MakeClosure { .. }
                | SsaOp::LoadCaptured { .. }
                | SsaOp::StoreCaptured { .. }
                | SsaOp::LoadUpvalue(_)
                | SsaOp::StoreUpvalue { .. }
                | SsaOp::CloseUpvalues { .. }
                | SsaOp::Cast { .. }
                | SsaOp::Convert { .. }
                | SsaOp::IsNull { .. }
                | SsaOp::KindOf { .. }
                | SsaOp::TypeTest { .. }
                | SsaOp::ToString { .. }
                | SsaOp::IsArray { .. }
                | SsaOp::GetEnumTag { .. }
                | SsaOp::ObjectKeys { .. }
                | SsaOp::BuildStr { .. }
                | SsaOp::BuildArray { .. }
                | SsaOp::BuildMap { .. }
                | SsaOp::BuildObject { .. }
                | SsaOp::GetProperty { .. }
                | SsaOp::GetSlotMethod { .. }
                | SsaOp::SetProperty { .. }
                | SsaOp::GetIndex { .. }
                | SsaOp::SetIndex { .. }
                | SsaOp::ArrayLength { .. }
                | SsaOp::StrLength { .. }
                | SsaOp::BytesLength { .. }
                | SsaOp::MethodCall { .. }
                | SsaOp::InvokeSlot { .. }
                | SsaOp::CallNativeOp { .. }
                | SsaOp::ArrayPush { .. }
                | SsaOp::This
                | SsaOp::GetFixedField { .. }
                | SsaOp::SetFixedField { .. }
                | SsaOp::MakeClass { .. }
                | SsaOp::DeclareLayout { .. }
                | SsaOp::FreezeClass { .. }
                | SsaOp::DefineMember { .. }
                | SsaOp::GetSuper { .. }
                | SsaOp::GetSuperSlot { .. }
                | SsaOp::LoadGlobal(_)
                | SsaOp::StoreGlobal { .. }
                | SsaOp::BuildTuple { .. }
                | SsaOp::BuildArraySpread { .. }
                | SsaOp::BuildObjectSpread { .. }
                | SsaOp::ObjectMerge { .. }
                | SsaOp::ObjectRest { .. }
                | SsaOp::GetPropertyMaybe { .. }
                | SsaOp::AssertNotNull { .. }
                | SsaOp::Range { .. }
                | SsaOp::SuperCall { .. }
                | SsaOp::SuperMethodCall { .. }
                | SsaOp::SuperInvokeSlot { .. }
                | SsaOp::ExtensionCall { .. }
                | SsaOp::CallSpread { .. }
                | SsaOp::LoadModule { .. }
                | SsaOp::ModuleSlot { .. }
                | SsaOp::StoreModuleSlot { .. }
                | SsaOp::Await { .. }
                | SsaOp::Spawn { .. }
                | SsaOp::Yield { .. } => continue,
            };
            by_value.entry(object).or_insert_with(|| View {
                data: b.declare_var(types::I64),
                len: b.declare_var(types::I64),
                disc: b.declare_var(types::I64),
            });
        }
        let views = Views {
            by_value,
            instance_by_canon,
            global_src,
            this_values,
        };
        views.clear(b);
        views
    }

    pub(crate) fn canon(&self, object: ValueId) -> u32 {
        if self.this_values.contains(&object) {
            return THIS_CANON;
        }
        self.global_src
            .get(&object)
            .map(|s| GLOBAL_CANON_BIT | s)
            .unwrap_or(object.0)
    }

    pub(crate) fn instance_var(&self, canon: u32) -> Option<Variable> {
        self.instance_by_canon.get(&canon).copied()
    }

    pub(super) fn invalidate_global(&self, b: &mut FunctionBuilder, slot: u32) {
        let c = GLOBAL_CANON_BIT | slot;
        if let Some(v) = self.instance_by_canon.get(&c) {
            let zero = b.ins().iconst(types::I64, 0);
            b.def_var(*v, zero);
        }
    }

    pub(super) fn of(&self, object: ValueId) -> Option<View> {
        self.by_value.get(&object).copied()
    }

    pub(super) fn forget(&self, b: &mut FunctionBuilder, value: ValueId) {
        if let Some(view) = self.by_value.get(&value) {
            let zero = b.ins().iconst(types::I64, 0);
            b.def_var(view.data, zero);
            b.def_var(view.len, zero);
            b.def_var(view.disc, zero);
        }
    }

    pub(super) fn clear(&self, b: &mut FunctionBuilder) {
        if self.by_value.is_empty() {
            return;
        }
        let zero = b.ins().iconst(types::I64, 0);
        for view in self.by_value.values() {
            b.def_var(view.data, zero);
            b.def_var(view.len, zero);
            b.def_var(view.disc, zero);
        }
    }

    pub(super) fn clear_instances(&self, b: &mut FunctionBuilder) {
        let zero = b.ins().iconst(types::I64, 0);
        for (canon, var) in &self.instance_by_canon {
            if canon & GLOBAL_CANON_BIT != 0 {
                b.def_var(*var, zero);
            }
        }
    }

    pub(super) fn forget_instance(&self, b: &mut FunctionBuilder, value: ValueId) {
        if let Some(var) = self.instance_by_canon.get(&value.0) {
            let zero = b.ins().iconst(types::I64, 0);
            b.def_var(*var, zero);
        }
    }

    pub(super) fn clear_all(&self, b: &mut FunctionBuilder) {
        self.clear(b);
        self.clear_instances(b);
    }
}
