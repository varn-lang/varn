use super::ir::SsaFunc;
use crate::OptError;
use vn_types::chunk::Chunk;
use vn_types::ssa::SsaOp;

pub(crate) struct IcSlots {
    slots: Vec<Vec<Option<u8>>>,
    uncached: Vec<Vec<bool>>,
    count: u16,
}

impl IcSlots {
    pub(crate) fn number(ssa: &SsaFunc, order: &[usize]) -> Result<Self, OptError> {
        let mut slots: Vec<Vec<Option<u8>>> = ssa
            .blocks
            .iter()
            .map(|b| vec![None; b.insts.len()])
            .collect();
        let mut uncached: Vec<Vec<bool>> = ssa
            .blocks
            .iter()
            .map(|b| vec![false; b.insts.len()])
            .collect();
        let mut count: u16 = 0;
        for &b in order {
            for (i, inst) in ssa.blocks[b].insts.iter().enumerate() {
                let n = sites(&inst.kind, inst.dest.is_some());
                if n == 0 {
                    continue;
                }
                if count + n > Chunk::MAX_IC_SLOTS {
                    uncached[b][i] = true;
                    continue;
                }
                slots[b][i] = Some(count as u8);
                count += n;
            }
        }
        Ok(Self {
            slots,
            uncached,
            count,
        })
    }

    pub(crate) fn emit_slot(&self, block: usize, inst: usize) -> Option<u8> {
        match self.of(block, inst) {
            Some(slot) => Some(slot),
            None => self
                .uncached
                .get(block)?
                .get(inst)
                .copied()?
                .then_some(Chunk::NO_IC_SLOT),
        }
    }

    pub(crate) fn of(&self, block: usize, inst: usize) -> Option<u8> {
        self.slots.get(block)?.get(inst).copied().flatten()
    }

    pub(crate) fn count(&self) -> u16 {
        self.count
    }
}

pub(crate) struct DirectSlots {
    slots: Vec<Vec<Option<u8>>>,
    count: u16,
}

impl DirectSlots {
    pub(crate) fn number(ssa: &SsaFunc, order: &[usize]) -> Result<Self, OptError> {
        let mut slots: Vec<Vec<Option<u8>>> = ssa
            .blocks
            .iter()
            .map(|b| vec![None; b.insts.len()])
            .collect();
        let mut count: u16 = 0;
        for &b in order {
            for (i, inst) in ssa.blocks[b].insts.iter().enumerate() {
                if !matches!(&inst.kind, SsaOp::CallDirect { .. }) {
                    continue;
                }
                if count + 1 > Chunk::MAX_IC_SLOTS {
                    continue;
                }
                slots[b][i] = Some(count as u8);
                count += 1;
            }
        }
        Ok(Self { slots, count })
    }

    pub(crate) fn of(&self, block: usize, inst: usize) -> Option<u8> {
        self.slots.get(block)?.get(inst).copied().flatten()
    }

    pub(crate) fn count(&self) -> u16 {
        self.count
    }
}

fn sites(kind: &SsaOp, has_dest: bool) -> u16 {
    match kind {
        SsaOp::SetProperty { .. } | SsaOp::InvokeSlot { .. } => 1,
        SsaOp::GetProperty { .. } | SsaOp::MethodCall { .. } => {
            u16::from(has_dest || crate::passes::dce::dest_droppable(kind))
        }
        SsaOp::BuildObjectSpread { parts } if has_dest => {
            parts.iter().filter(|p| p.key.is_some()).count() as u16
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
        | SsaOp::GetFixedField { .. }
        | SsaOp::GetIndex { .. }
        | SsaOp::ArrayGetIndex { .. }
        | SsaOp::BytesGetIndex { .. }
        | SsaOp::SetFixedField { .. }
        | SsaOp::SetIndex { .. }
        | SsaOp::ArraySetIndex { .. }
        | SsaOp::BytesSetIndex { .. }
        | SsaOp::ArrayPush { .. }
        | SsaOp::ObjectMerge { .. }
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
        | SsaOp::GetSlotMethod { .. }
        | SsaOp::GetSuper { .. }
        | SsaOp::GetSuperSlot { .. }
        | SsaOp::SuperCall { .. }
        | SsaOp::SuperMethodCall { .. }
        | SsaOp::SuperInvokeSlot { .. }
        | SsaOp::ExtensionCall { .. }
        | SsaOp::CallSpread { .. }
        | SsaOp::BuildArraySpread { .. }
        | SsaOp::BuildObjectSpread { .. } => 0,
    }
}
