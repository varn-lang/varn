use std::sync::Arc;
use vn_tir::BackendTy;
use vn_types::ssa::SsaOp;

use crate::ssa::ir::{SsaFunc, SsaVar, Terminator};
use crate::ssa::liveness::Liveness;
use rustc_hash::FxHashSet;
use vn_types::ssa::{SsaBlock, SsaProto, SsaTerm, SsaValue, ValueId};

mod entries;
mod inst;
mod why_not;

use inst::project_inst;

pub(crate) struct Emitted<'a> {
    pub reg: &'a [u8],
    pub register_count: u16,
    pub nparams: usize,

    pub ic: &'a crate::ssa::ic::IcSlots,

    pub direct: &'a crate::ssa::ic::DirectSlots,

    pub closure_consts: &'a [Vec<Option<u16>>],

    pub block_offset: &'a [usize],

    pub inst_off: &'a [Vec<usize>],

    pub inst_next: &'a [Vec<usize>],

    pub liveness: &'a Liveness,
}

struct Captured {
    vars: Vec<SsaVar>,
    nparams: usize,
}

impl Captured {
    fn var(&mut self, var: SsaVar) -> vn_types::ssa::SsaVar {
        if !self.vars.contains(&var) {
            self.vars.push(var);
        }
        var
    }

    fn table(&self) -> Vec<(vn_types::ssa::SsaVar, u32)> {
        self.vars
            .iter()
            .map(|v| (*v, u32::from(crate::ssa::emit::var_reg(*v, self.nparams))))
            .collect()
    }
}

pub(crate) fn project(
    ssa: &SsaFunc,
    emitted: &Emitted<'_>,
    has_this: bool,
    name: &Arc<str>,
) -> Result<SsaProto, String> {
    let value_tys: Vec<BackendTy> = ssa.values.iter().map(|v| v.ty).collect();

    let lv = emitted.liveness;
    let handlers = crate::ssa::suspend::try_handlers(ssa);

    let mut captured = Captured {
        vars: Vec::new(),
        nparams: emitted.nparams,
    };
    let mut blocks = Vec::with_capacity(ssa.blocks.len());
    let mut call_sites: Vec<(u32, u32)> = Vec::new();
    let mut closure_sites: Vec<(u32, u32)> = Vec::new();
    for (b, block) in ssa.blocks.iter().enumerate() {
        let mut insts = Vec::with_capacity(block.insts.len());
        for (i, inst) in block.insts.iter().enumerate() {
            let own = emitted
                .inst_off
                .get(b)
                .and_then(|v| v.get(i))
                .copied()
                .unwrap_or(usize::MAX);
            let next = emitted
                .inst_next
                .get(b)
                .and_then(|v| v.get(i))
                .copied()
                .unwrap_or(usize::MAX);
            let site = Site {
                ic_slot: emitted.ic.of(b, i),
                direct_slot: emitted.direct.of(b, i),
                closure_const: emitted
                    .closure_consts
                    .get(b)
                    .and_then(|c| c.get(i))
                    .copied()
                    .flatten(),
                landing: match &inst.kind {
                    SsaOp::Try { handler } => {
                        let h = handler.0 as usize;
                        let off = emitted.block_offset.get(h).copied();
                        off.and_then(|o| u32::try_from(o).ok())
                            .map(|o| (o, &lv.live_in[h]))
                    }
                    SsaOp::ConstInt(_)
                    | SsaOp::ConstFloat(_)
                    | SsaOp::ConstBool(_)
                    | SsaOp::ConstStr(_)
                    | SsaOp::ConstChar(_)
                    | SsaOp::ConstDecimal(_)
                    | SsaOp::ConstBigInt(_)
                    | SsaOp::ConstNull
                    | SsaOp::Binary { .. }
                    | SsaOp::Unary { .. }
                    | SsaOp::KindOf { .. }
                    | SsaOp::LoadGlobal(_)
                    | SsaOp::LoadGlobalIdx(_)
                    | SsaOp::LoadNativeGlobalIdx(_)
                    | SsaOp::LoadUpvalue(_)
                    | SsaOp::StoreGlobal { .. }
                    | SsaOp::StoreGlobalIdx { .. }
                    | SsaOp::StoreUpvalue { .. }
                    | SsaOp::Call { .. }
                    | SsaOp::CallDirect { .. }
                    | SsaOp::AllocInstance { .. }
                    | SsaOp::SelfCall { .. }
                    | SsaOp::GetProperty { .. }
                    | SsaOp::GetSlotMethod { .. }
                    | SsaOp::GetFixedField { .. }
                    | SsaOp::GetIndex { .. }
                    | SsaOp::ArrayGetIndex { .. }
                    | SsaOp::BytesGetIndex { .. }
                    | SsaOp::SetProperty { .. }
                    | SsaOp::SetFixedField { .. }
                    | SsaOp::SetIndex { .. }
                    | SsaOp::ArraySetIndex { .. }
                    | SsaOp::BytesSetIndex { .. }
                    | SsaOp::ArrayPush { .. }
                    | SsaOp::ObjectMerge { .. }
                    | SsaOp::MethodCall { .. }
                    | SsaOp::InvokeSlot { .. }
                    | SsaOp::IsNull { .. }
                    | SsaOp::TypeTest { .. }
                    | SsaOp::Cast { .. }
                    | SsaOp::Convert { .. }
                    | SsaOp::BuildArray { .. }
                    | SsaOp::BuildTuple { .. }
                    | SsaOp::BuildObject { .. }
                    | SsaOp::BuildMap { .. }
                    | SsaOp::ObjectRest { .. }
                    | SsaOp::ToString { .. }
                    | SsaOp::BuildStr { .. }
                    | SsaOp::MakeClosure { .. }
                    | SsaOp::LoadCaptured { .. }
                    | SsaOp::StoreCaptured { .. }
                    | SsaOp::MakeClass { .. }
                    | SsaOp::DeclareLayout { .. }
                    | SsaOp::FreezeClass { .. }
                    | SsaOp::DefineMember { .. }
                    | SsaOp::MakeEnumVariant { .. }
                    | SsaOp::PopTry
                    | SsaOp::CatchParam { .. }
                    | SsaOp::CloseUpvalues { .. }
                    | SsaOp::LoadModule { .. }
                    | SsaOp::StoreModuleSlot { .. }
                    | SsaOp::Await { .. }
                    | SsaOp::Spawn { .. }
                    | SsaOp::Yield { .. }
                    | SsaOp::IntrinsicCall { .. }
                    | SsaOp::CallNativeOp { .. }
                    | SsaOp::AssertNotNull { .. }
                    | SsaOp::GetPropertyMaybe { .. }
                    | SsaOp::ModuleSlot { .. }
                    | SsaOp::GetEnumTag { .. }
                    | SsaOp::IsArray { .. }
                    | SsaOp::StrLength { .. }
                    | SsaOp::ArrayLength { .. }
                    | SsaOp::BytesLength { .. }
                    | SsaOp::This
                    | SsaOp::Range { .. }
                    | SsaOp::ObjectKeys { .. }
                    | SsaOp::GetSuper { .. }
                    | SsaOp::GetSuperSlot { .. }
                    | SsaOp::SuperCall { .. }
                    | SsaOp::SuperMethodCall { .. }
                    | SsaOp::SuperInvokeSlot { .. }
                    | SsaOp::ExtensionCall { .. }
                    | SsaOp::CallSpread { .. }
                    | SsaOp::BuildArraySpread { .. }
                    | SsaOp::BuildObjectSpread { .. } => None,
                },
                own_ip: u32::try_from(own).unwrap_or(u32::MAX),
                next_ip: u32::try_from(next).unwrap_or(u32::MAX),
                resume_live: match &inst.kind {
                    SsaOp::LoadModule { .. } | SsaOp::Await { .. } | SsaOp::Yield { .. } => {
                        crate::ssa::suspend::resume_live(ssa, lv, &handlers, b, i)
                    }
                    SsaOp::ConstInt(_)
                    | SsaOp::ConstFloat(_)
                    | SsaOp::ConstBool(_)
                    | SsaOp::ConstStr(_)
                    | SsaOp::ConstChar(_)
                    | SsaOp::ConstDecimal(_)
                    | SsaOp::ConstBigInt(_)
                    | SsaOp::ConstNull
                    | SsaOp::Binary { .. }
                    | SsaOp::Unary { .. }
                    | SsaOp::KindOf { .. }
                    | SsaOp::LoadGlobal(_)
                    | SsaOp::LoadGlobalIdx(_)
                    | SsaOp::LoadNativeGlobalIdx(_)
                    | SsaOp::LoadUpvalue(_)
                    | SsaOp::StoreGlobal { .. }
                    | SsaOp::StoreGlobalIdx { .. }
                    | SsaOp::StoreUpvalue { .. }
                    | SsaOp::Call { .. }
                    | SsaOp::CallDirect { .. }
                    | SsaOp::AllocInstance { .. }
                    | SsaOp::SelfCall { .. }
                    | SsaOp::GetProperty { .. }
                    | SsaOp::GetSlotMethod { .. }
                    | SsaOp::GetFixedField { .. }
                    | SsaOp::GetIndex { .. }
                    | SsaOp::ArrayGetIndex { .. }
                    | SsaOp::BytesGetIndex { .. }
                    | SsaOp::SetProperty { .. }
                    | SsaOp::SetFixedField { .. }
                    | SsaOp::SetIndex { .. }
                    | SsaOp::ArraySetIndex { .. }
                    | SsaOp::BytesSetIndex { .. }
                    | SsaOp::ArrayPush { .. }
                    | SsaOp::ObjectMerge { .. }
                    | SsaOp::MethodCall { .. }
                    | SsaOp::InvokeSlot { .. }
                    | SsaOp::IsNull { .. }
                    | SsaOp::TypeTest { .. }
                    | SsaOp::Cast { .. }
                    | SsaOp::Convert { .. }
                    | SsaOp::BuildArray { .. }
                    | SsaOp::BuildTuple { .. }
                    | SsaOp::BuildObject { .. }
                    | SsaOp::BuildMap { .. }
                    | SsaOp::ObjectRest { .. }
                    | SsaOp::ToString { .. }
                    | SsaOp::BuildStr { .. }
                    | SsaOp::MakeClosure { .. }
                    | SsaOp::LoadCaptured { .. }
                    | SsaOp::StoreCaptured { .. }
                    | SsaOp::MakeClass { .. }
                    | SsaOp::DeclareLayout { .. }
                    | SsaOp::FreezeClass { .. }
                    | SsaOp::DefineMember { .. }
                    | SsaOp::MakeEnumVariant { .. }
                    | SsaOp::Try { .. }
                    | SsaOp::PopTry
                    | SsaOp::CatchParam { .. }
                    | SsaOp::CloseUpvalues { .. }
                    | SsaOp::StoreModuleSlot { .. }
                    | SsaOp::Spawn { .. }
                    | SsaOp::IntrinsicCall { .. }
                    | SsaOp::CallNativeOp { .. }
                    | SsaOp::AssertNotNull { .. }
                    | SsaOp::GetPropertyMaybe { .. }
                    | SsaOp::ModuleSlot { .. }
                    | SsaOp::GetEnumTag { .. }
                    | SsaOp::IsArray { .. }
                    | SsaOp::StrLength { .. }
                    | SsaOp::ArrayLength { .. }
                    | SsaOp::BytesLength { .. }
                    | SsaOp::This
                    | SsaOp::Range { .. }
                    | SsaOp::ObjectKeys { .. }
                    | SsaOp::GetSuper { .. }
                    | SsaOp::GetSuperSlot { .. }
                    | SsaOp::SuperCall { .. }
                    | SsaOp::SuperMethodCall { .. }
                    | SsaOp::SuperInvokeSlot { .. }
                    | SsaOp::ExtensionCall { .. }
                    | SsaOp::CallSpread { .. }
                    | SsaOp::BuildArraySpread { .. }
                    | SsaOp::BuildObjectSpread { .. } => Vec::new(),
                },
            };
            let site_ip = site.next_ip;
            let closure_const = site.closure_const;
            let projected = project_inst(inst, &value_tys, site, &mut captured)
                .ok_or_else(|| why_not::why_not(&inst.kind, &value_tys))?;
            if let SsaOp::MakeClosure { .. } = &inst.kind {
                let proto =
                    closure_const.ok_or_else(|| why_not::why_not(&inst.kind, &value_tys))?;
                closure_sites.push((site_ip, u32::from(proto)));
            }
            if let SsaOp::Call {
                shape: Some(shape), ..
            }
            | SsaOp::CallDirect {
                shape: Some(shape), ..
            } = &inst.kind
            {
                call_sites.push((site_ip, *shape));
            }
            insts.push(projected);
        }
        blocks.push(SsaBlock {
            params: block.params.clone(),
            insts,
            term: project_term(&block.term),
        });
    }

    let values = ssa
        .values
        .iter()
        .map(|v| SsaValue {
            ty: super::emit::slot_kind_of(v.ty),
        })
        .collect();

    call_sites.sort_unstable();
    closure_sites.sort_unstable();
    Ok(SsaProto {
        name: name.clone(),
        nparams: ssa
            .blocks
            .get(ssa.entry.0 as usize)
            .map(|b| b.params.len() as u32)
            .unwrap_or(0),
        entry: ssa.entry,
        blocks,
        values,
        regs: emitted.reg.iter().map(|&r| r as u32).collect(),
        register_count: emitted.register_count,
        has_this,
        captured: captured.table(),
        secondary_entries: entries::secondary_entries(ssa, &emitted),
        call_shapes: ssa.call_shapes.clone(),
        call_sites,
        closure_sites,
    })
}

struct Site<'a> {
    ic_slot: Option<u8>,

    direct_slot: Option<u8>,

    closure_const: Option<u16>,

    landing: Option<(u32, &'a FxHashSet<u32>)>,

    own_ip: u32,

    next_ip: u32,

    resume_live: Vec<ValueId>,
}

fn project_term(term: &Terminator) -> SsaTerm {
    term.clone()
}
