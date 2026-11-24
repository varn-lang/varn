use crate::ssa::ir::{BlockId, SsaFunc, Terminator, Value};
use rustc_hash::{FxHashMap, FxHashSet};
use vn_types::ssa::SsaOp;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NullFact {
    Null,
    NonNull,
}

pub fn run(func: &mut SsaFunc) -> bool {
    let mut changed = false;

    let mut inherently_non_null: FxHashSet<Value> = FxHashSet::default();
    let mut null_literals: FxHashSet<Value> = FxHashSet::default();

    for block in &func.blocks {
        for inst in &block.insts {
            if let Some(dest) = inst.dest {
                match &inst.kind {
                    SsaOp::ConstNull => {
                        null_literals.insert(dest);
                    }
                    SsaOp::ConstInt(_)
                    | SsaOp::ConstFloat(_)
                    | SsaOp::ConstBool(_)
                    | SsaOp::ConstStr(_)
                    | SsaOp::ConstChar(_)
                    | SsaOp::ConstDecimal(_)
                    | SsaOp::ConstBigInt(_)
                    | SsaOp::BuildArray { .. }
                    | SsaOp::BuildTuple { .. }
                    | SsaOp::BuildObject { .. }
                    | SsaOp::BuildMap { .. }
                    | SsaOp::BuildStr { .. }
                    | SsaOp::MakeClosure { .. }
                    | SsaOp::MakeClass { .. }
                    | SsaOp::MakeEnumVariant { .. }
                    | SsaOp::This => {
                        inherently_non_null.insert(dest);
                    }
                    SsaOp::Binary { .. }
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
                    | SsaOp::ObjectRest { .. }
                    | SsaOp::ToString { .. }
                    | SsaOp::LoadCaptured { .. }
                    | SsaOp::StoreCaptured { .. }
                    | SsaOp::DeclareLayout { .. }
                    | SsaOp::FreezeClass { .. }
                    | SsaOp::DefineMember { .. }
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
                    | SsaOp::BuildObjectSpread { .. } => {}
                }
            }
        }
    }

    for block in &mut func.blocks {
        for inst in &mut block.insts {
            if let SsaOp::IsNull { operand } = &inst.kind {
                if inherently_non_null.contains(operand) {
                    inst.kind = SsaOp::ConstBool(false);
                    changed = true;
                } else if null_literals.contains(operand) {
                    inst.kind = SsaOp::ConstBool(true);
                    changed = true;
                }
            }
        }
    }

    let mut defs: FxHashMap<Value, SsaOp> = FxHashMap::default();
    for block in &func.blocks {
        for inst in &block.insts {
            if let Some(d) = inst.dest {
                defs.insert(d, inst.kind.clone());
            }
        }
    }

    let mut block_facts: FxHashMap<BlockId, FxHashMap<Value, NullFact>> = FxHashMap::default();

    for (b_idx, block) in func.blocks.iter().enumerate() {
        let b_id = BlockId(b_idx as u32);
        if let Terminator::Branch {
            cond,
            then_blk,
            else_blk,
            ..
        } = &block.term
        {
            let mut then_facts = FxHashMap::default();
            let mut else_facts = FxHashMap::default();

            if let Some(def) = defs.get(cond) {
                match def {
                    SsaOp::IsNull { operand } => {
                        then_facts.insert(*operand, NullFact::Null);
                        else_facts.insert(*operand, NullFact::NonNull);
                    }
                    SsaOp::Binary {
                        op: vn_types::ssa::SsaBinOp::Dyn(vn_core::BinOp::Eq),
                        lhs,
                        rhs,
                        ..
                    } => {
                        if null_literals.contains(rhs)
                            || matches!(defs.get(rhs), Some(SsaOp::ConstNull))
                        {
                            then_facts.insert(*lhs, NullFact::Null);
                            else_facts.insert(*lhs, NullFact::NonNull);
                        } else if null_literals.contains(lhs)
                            || matches!(defs.get(lhs), Some(SsaOp::ConstNull))
                        {
                            then_facts.insert(*rhs, NullFact::Null);
                            else_facts.insert(*rhs, NullFact::NonNull);
                        }
                    }
                    SsaOp::Binary {
                        op: vn_types::ssa::SsaBinOp::Dyn(vn_core::BinOp::Ne),
                        lhs,
                        rhs,
                        ..
                    } => {
                        if null_literals.contains(rhs)
                            || matches!(defs.get(rhs), Some(SsaOp::ConstNull))
                        {
                            then_facts.insert(*lhs, NullFact::NonNull);
                            else_facts.insert(*lhs, NullFact::Null);
                        } else if null_literals.contains(lhs)
                            || matches!(defs.get(lhs), Some(SsaOp::ConstNull))
                        {
                            then_facts.insert(*rhs, NullFact::NonNull);
                            else_facts.insert(*rhs, NullFact::Null);
                        }
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
                    | SsaOp::GetSuper { .. }
                    | SsaOp::GetSuperSlot { .. }
                    | SsaOp::SuperCall { .. }
                    | SsaOp::SuperMethodCall { .. }
                    | SsaOp::SuperInvokeSlot { .. }
                    | SsaOp::ExtensionCall { .. }
                    | SsaOp::CallSpread { .. }
                    | SsaOp::BuildArraySpread { .. }
                    | SsaOp::BuildObjectSpread { .. }
                    | SsaOp::TypeTest { .. } => {}
                }
            }

            if func.blocks[then_blk.0 as usize].preds == [b_id] {
                block_facts.entry(*then_blk).or_default().extend(then_facts);
            }
            if func.blocks[else_blk.0 as usize].preds == [b_id] {
                block_facts.entry(*else_blk).or_default().extend(else_facts);
            }
        }
    }

    for (b_id, facts) in &block_facts {
        let block = &mut func.blocks[b_id.0 as usize];
        for inst in &mut block.insts {
            if let SsaOp::IsNull { operand } = &inst.kind
                && let Some(fact) = facts.get(operand)
            {
                match fact {
                    NullFact::Null => {
                        inst.kind = SsaOp::ConstBool(true);
                        changed = true;
                    }
                    NullFact::NonNull => {
                        inst.kind = SsaOp::ConstBool(false);
                        changed = true;
                    }
                }
            }
        }
    }

    changed
}
