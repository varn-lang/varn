use std::sync::Arc;
use vn_tir::BackendTy;

use crate::ssa::ir::{InstKind, SsaFunc, Terminator, VarId};
use crate::ssa::liveness::Liveness;
use rustc_hash::FxHashSet;
use vn_types::ssa::{SsaBlock, SsaProto, SsaTerm, SsaValue};

mod entries;
mod inst;
mod ops;
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
    vars: Vec<VarId>,
    nparams: usize,
}

impl Captured {
    fn var(&mut self, var: VarId) -> vn_types::ssa::SsaVar {
        if !self.vars.contains(&var) {
            self.vars.push(var);
        }
        var.portable()
    }

    fn table(&self) -> Vec<(vn_types::ssa::SsaVar, u32)> {
        self.vars
            .iter()
            .map(|v| {
                (
                    v.portable(),
                    u32::from(crate::ssa::emit::var_reg(*v, self.nparams)),
                )
            })
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
                    InstKind::Try { handler } => {
                        let h = handler.0 as usize;
                        let off = emitted.block_offset.get(h).copied();
                        off.and_then(|o| u32::try_from(o).ok())
                            .map(|o| (o, &lv.live_in[h]))
                    }
                    InstKind::ConstInt(_)
                    | InstKind::ConstFloat(_)
                    | InstKind::ConstBool(_)
                    | InstKind::ConstStr(_)
                    | InstKind::ConstChar(_)
                    | InstKind::ConstDecimal(_)
                    | InstKind::ConstBigInt(_)
                    | InstKind::ConstNull
                    | InstKind::Binary { .. }
                    | InstKind::Unary { .. }
                    | InstKind::LoadGlobal(_)
                    | InstKind::LoadGlobalIdx(_)
                    | InstKind::LoadNativeGlobalIdx(_)
                    | InstKind::LoadUpvalue(_)
                    | InstKind::StoreGlobal { .. }
                    | InstKind::StoreGlobalIdx { .. }
                    | InstKind::StoreUpvalue { .. }
                    | InstKind::Call { .. }
                    | InstKind::CallDirect { .. }
                    | InstKind::AllocInstance { .. }
                    | InstKind::SelfCall { .. }
                    | InstKind::GetProperty { .. }
                    | InstKind::GetSlotMethod { .. }
                    | InstKind::GetFixedField { .. }
                    | InstKind::GetIndex { .. }
                    | InstKind::ArrayGetIndex { .. }
                    | InstKind::BytesGetIndex { .. }
                    | InstKind::MapGetIndex { .. }
                    | InstKind::SetProperty { .. }
                    | InstKind::SetFixedField { .. }
                    | InstKind::SetIndex { .. }
                    | InstKind::ArraySetIndex { .. }
                    | InstKind::BytesSetIndex { .. }
                    | InstKind::MapSetIndex { .. }
                    | InstKind::ArrayPush { .. }
                    | InstKind::ObjectMerge { .. }
                    | InstKind::MethodCall { .. }
                    | InstKind::InvokeSlot { .. }
                    | InstKind::IsNull { .. }
                    | InstKind::TypeTest { .. }
                    | InstKind::Cast { .. }
                    | InstKind::Convert { .. }
                    | InstKind::BuildArray { .. }
                    | InstKind::BuildTuple { .. }
                    | InstKind::BuildObject { .. }
                    | InstKind::BuildMap { .. }
                    | InstKind::ObjectRest { .. }
                    | InstKind::ToString { .. }
                    | InstKind::BuildStr { .. }
                    | InstKind::MakeClosure { .. }
                    | InstKind::LoadCaptured { .. }
                    | InstKind::StoreCaptured { .. }
                    | InstKind::MakeClass { .. }
                    | InstKind::DeclareLayout { .. }
                    | InstKind::FreezeClass { .. }
                    | InstKind::DefineMember { .. }
                    | InstKind::MakeEnumVariant { .. }
                    | InstKind::PopTry
                    | InstKind::CatchParam { .. }
                    | InstKind::CloseUpvalues { .. }
                    | InstKind::Dispose { .. }
                    | InstKind::LoadModule { .. }
                    | InstKind::StoreModuleSlot { .. }
                    | InstKind::Await { .. }
                    | InstKind::Spawn { .. }
                    | InstKind::Yield { .. }
                    | InstKind::IntrinsicCall { .. }
                    | InstKind::CallNativeOp { .. }
                    | InstKind::AssertNotNull { .. }
                    | InstKind::GetPropertyMaybe { .. }
                    | InstKind::ModuleSlot { .. }
                    | InstKind::GetEnumTag { .. }
                    | InstKind::IsArray { .. }
                    | InstKind::StrLength { .. }
                    | InstKind::ArrayLength { .. }
                    | InstKind::BytesLength { .. }
                    | InstKind::This
                    | InstKind::Range { .. }
                    | InstKind::ObjectKeys { .. }
                    | InstKind::GetSuper { .. }
                    | InstKind::GetSuperSlot { .. }
                    | InstKind::SuperCall { .. }
                    | InstKind::SuperMethodCall { .. }
                    | InstKind::SuperInvokeSlot { .. }
                    | InstKind::ExtensionCall { .. }
                    | InstKind::CallSpread { .. }
                    | InstKind::BuildArraySpread { .. }
                    | InstKind::BuildObjectSpread { .. } => None,
                },
                own_ip: u32::try_from(own).unwrap_or(u32::MAX),
                next_ip: u32::try_from(next).unwrap_or(u32::MAX),
                resume_live: match &inst.kind {
                    InstKind::LoadModule { .. }
                    | InstKind::Await { .. }
                    | InstKind::Yield { .. } => {
                        crate::ssa::suspend::resume_live(ssa, lv, &handlers, b, i)
                            .into_iter()
                            .map(|v| v.0)
                            .collect()
                    }
                    InstKind::ConstInt(_)
                    | InstKind::ConstFloat(_)
                    | InstKind::ConstBool(_)
                    | InstKind::ConstStr(_)
                    | InstKind::ConstChar(_)
                    | InstKind::ConstDecimal(_)
                    | InstKind::ConstBigInt(_)
                    | InstKind::ConstNull
                    | InstKind::Binary { .. }
                    | InstKind::Unary { .. }
                    | InstKind::LoadGlobal(_)
                    | InstKind::LoadGlobalIdx(_)
                    | InstKind::LoadNativeGlobalIdx(_)
                    | InstKind::LoadUpvalue(_)
                    | InstKind::StoreGlobal { .. }
                    | InstKind::StoreGlobalIdx { .. }
                    | InstKind::StoreUpvalue { .. }
                    | InstKind::Call { .. }
                    | InstKind::CallDirect { .. }
                    | InstKind::AllocInstance { .. }
                    | InstKind::SelfCall { .. }
                    | InstKind::GetProperty { .. }
                    | InstKind::GetSlotMethod { .. }
                    | InstKind::GetFixedField { .. }
                    | InstKind::GetIndex { .. }
                    | InstKind::ArrayGetIndex { .. }
                    | InstKind::BytesGetIndex { .. }
                    | InstKind::MapGetIndex { .. }
                    | InstKind::SetProperty { .. }
                    | InstKind::SetFixedField { .. }
                    | InstKind::SetIndex { .. }
                    | InstKind::ArraySetIndex { .. }
                    | InstKind::BytesSetIndex { .. }
                    | InstKind::MapSetIndex { .. }
                    | InstKind::ArrayPush { .. }
                    | InstKind::ObjectMerge { .. }
                    | InstKind::MethodCall { .. }
                    | InstKind::InvokeSlot { .. }
                    | InstKind::IsNull { .. }
                    | InstKind::TypeTest { .. }
                    | InstKind::Cast { .. }
                    | InstKind::Convert { .. }
                    | InstKind::BuildArray { .. }
                    | InstKind::BuildTuple { .. }
                    | InstKind::BuildObject { .. }
                    | InstKind::BuildMap { .. }
                    | InstKind::ObjectRest { .. }
                    | InstKind::ToString { .. }
                    | InstKind::BuildStr { .. }
                    | InstKind::MakeClosure { .. }
                    | InstKind::LoadCaptured { .. }
                    | InstKind::StoreCaptured { .. }
                    | InstKind::MakeClass { .. }
                    | InstKind::DeclareLayout { .. }
                    | InstKind::FreezeClass { .. }
                    | InstKind::DefineMember { .. }
                    | InstKind::MakeEnumVariant { .. }
                    | InstKind::Try { .. }
                    | InstKind::PopTry
                    | InstKind::CatchParam { .. }
                    | InstKind::CloseUpvalues { .. }
                    | InstKind::Dispose { .. }
                    | InstKind::StoreModuleSlot { .. }
                    | InstKind::Spawn { .. }
                    | InstKind::IntrinsicCall { .. }
                    | InstKind::CallNativeOp { .. }
                    | InstKind::AssertNotNull { .. }
                    | InstKind::GetPropertyMaybe { .. }
                    | InstKind::ModuleSlot { .. }
                    | InstKind::GetEnumTag { .. }
                    | InstKind::IsArray { .. }
                    | InstKind::StrLength { .. }
                    | InstKind::ArrayLength { .. }
                    | InstKind::BytesLength { .. }
                    | InstKind::This
                    | InstKind::Range { .. }
                    | InstKind::ObjectKeys { .. }
                    | InstKind::GetSuper { .. }
                    | InstKind::GetSuperSlot { .. }
                    | InstKind::SuperCall { .. }
                    | InstKind::SuperMethodCall { .. }
                    | InstKind::SuperInvokeSlot { .. }
                    | InstKind::ExtensionCall { .. }
                    | InstKind::CallSpread { .. }
                    | InstKind::BuildArraySpread { .. }
                    | InstKind::BuildObjectSpread { .. } => Vec::new(),
                },
            };
            let site_ip = site.next_ip;
            let projected = project_inst(inst, &value_tys, site, &mut captured)
                .ok_or_else(|| why_not::why_not(&inst.kind, &value_tys))?;
            if let InstKind::Call {
                shape: Some(shape), ..
            }
            | InstKind::CallDirect {
                shape: Some(shape), ..
            } = &inst.kind
            {
                call_sites.push((site_ip, *shape));
            }
            insts.push(projected);
        }
        blocks.push(SsaBlock {
            params: block.params.iter().map(|v| v.0).collect(),
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
    Ok(SsaProto {
        name: name.clone(),
        nparams: ssa
            .blocks
            .get(ssa.entry.0 as usize)
            .map(|b| b.params.len() as u32)
            .unwrap_or(0),
        entry: ssa.entry.0,
        blocks,
        values,
        regs: emitted.reg.iter().map(|&r| r as u32).collect(),
        register_count: emitted.register_count,
        has_this,
        captured: captured.table(),
        secondary_entries: entries::secondary_entries(ssa, &emitted),
        call_shapes: ssa.call_shapes.clone(),
        call_sites,
    })
}

struct Site<'a> {
    ic_slot: Option<u8>,

    direct_slot: Option<u8>,

    closure_const: Option<u16>,

    landing: Option<(u32, &'a FxHashSet<u32>)>,

    own_ip: u32,

    next_ip: u32,

    resume_live: Vec<u32>,
}

fn project_term(term: &Terminator) -> SsaTerm {
    term.map_ids(|v| v.0, |b| b.0)
}
