use crate::ssa::ir::{SsaFunc, Terminator};
use vn_types::ssa::SsaOp;
use vn_types::ssa::{BlockId, SsaEntryKind, SsaSecondaryEntry, ValueId};

use super::Emitted;

pub(super) fn secondary_entries(ssa: &SsaFunc, emitted: &Emitted<'_>) -> Vec<SsaSecondaryEntry> {
    let mut out = loop_entries(ssa, emitted);
    out.extend(resume_entries(ssa, emitted));
    out.extend(catch_entries(ssa, emitted));
    out.sort_by_key(|e| (e.ip, e.block));
    out
}

fn block_ip(emitted: &Emitted<'_>, b: usize) -> Option<u32> {
    let off = emitted.block_offset.get(b).copied().unwrap_or(usize::MAX);
    if off == usize::MAX {
        return None;
    }
    u32::try_from(off).ok()
}

fn loop_entries(ssa: &SsaFunc, emitted: &Emitted<'_>) -> Vec<SsaSecondaryEntry> {
    let off = |b: usize| emitted.block_offset.get(b).copied().unwrap_or(usize::MAX);
    let mut is_header = vec![false; ssa.blocks.len()];
    for (b, block) in ssa.blocks.iter().enumerate() {
        let targets = match &block.term {
            Terminator::Jump { target, .. } => vec![target.0 as usize],
            Terminator::Branch {
                then_blk, else_blk, ..
            } => vec![then_blk.0 as usize, else_blk.0 as usize],
            Terminator::Return(_) | Terminator::Throw(_) | Terminator::Unreachable => Vec::new(),
        };
        for t in targets {
            if off(b) != usize::MAX && off(t) <= off(b) {
                is_header[t] = true;
            }
        }
    }
    is_header
        .iter()
        .enumerate()
        .filter(|(_, h)| **h)
        .filter_map(|(b, _)| {
            let ip = u32::try_from(off(b)).ok()?;
            let mut live: Vec<ValueId> = emitted.liveness.live_in[b]
                .iter()
                .copied()
                .map(ValueId)
                .collect();
            live.sort_unstable();
            Some(SsaSecondaryEntry {
                block: BlockId(b as u32),
                ip,
                live,
                kind: SsaEntryKind::Loop,
            })
        })
        .collect()
}

fn resume_entries(ssa: &SsaFunc, emitted: &Emitted<'_>) -> Vec<SsaSecondaryEntry> {
    let mut out = Vec::new();
    for (b, block) in ssa.blocks.iter().enumerate() {
        let cont = match &block.term {
            Terminator::Jump { target, .. } => target.0,
            Terminator::Branch { .. } => continue,
            Terminator::Return(_) => continue,
            Terminator::Throw(_) => continue,
            Terminator::Unreachable => continue,
        };
        for (i, inst) in block.insts.iter().enumerate() {
            if !is_suspend_site(&inst.kind) {
                continue;
            }
            let Some(next) = emitted.inst_next.get(b).and_then(|row| row.get(i)).copied() else {
                continue;
            };
            if next == usize::MAX {
                continue;
            }
            let Ok(ip) = u32::try_from(next) else {
                continue;
            };
            if ip == u32::MAX {
                continue;
            }
            let mut live: Vec<ValueId> = emitted.liveness.live_after(ssa, b, i);
            live.sort_unstable();
            out.push(SsaSecondaryEntry {
                block: BlockId(cont),
                ip,
                live,
                kind: SsaEntryKind::Resume,
            });
        }
    }
    out
}

fn catch_entries(ssa: &SsaFunc, emitted: &Emitted<'_>) -> Vec<SsaSecondaryEntry> {
    let mut out = Vec::new();
    for block in &ssa.blocks {
        for inst in &block.insts {
            let SsaOp::Try { handler } = &inst.kind else {
                continue;
            };
            let h = handler.0 as usize;
            if h >= ssa.blocks.len() {
                continue;
            }
            let Some(ip) = block_ip(emitted, h) else {
                continue;
            };
            let dest = inst.dest.map(|v| v.0);
            let mut live: Vec<ValueId> = emitted.liveness.live_in[h]
                .iter()
                .copied()
                .filter(|v| Some(*v) != dest)
                .map(ValueId)
                .collect();
            live.sort_unstable();
            out.push(SsaSecondaryEntry {
                block: BlockId(h as u32),
                ip,
                live,
                kind: SsaEntryKind::Catch,
            });
        }
    }
    out
}

fn is_suspend_site(kind: &SsaOp) -> bool {
    match kind {
        SsaOp::Await { .. } => true,
        SsaOp::Yield { .. } => true,
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
        | SsaOp::LoadModule { .. }
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
        | SsaOp::BuildObjectSpread { .. } => false,
    }
}
