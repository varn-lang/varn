use super::const_fold_values::{fold_binary, fold_convert, fold_unary};
use crate::ssa::ir::{const_ty, BlockId, SsaFunc, Terminator, Value};
use rustc_hash::FxHashMap;
use vn_types::ssa::SsaOp;

pub fn run(func: &mut SsaFunc) -> bool {
    let mut changed = false;
    let mut const_map = FxHashMap::default();

    for block_idx in 0..func.blocks.len() {
        let b_id = BlockId(block_idx as u32);

        for inst_idx in 0..func.blocks[b_id.0 as usize].insts.len() {
            let inst = &func.blocks[b_id.0 as usize].insts[inst_idx];
            if let Some(dest) = inst.dest {
                if const_ty(&inst.kind).is_some() {
                    const_map.insert(dest, inst.kind.clone());
                    continue;
                }

                if let Some(folded_kind) = fold_inst(&inst.kind, &const_map) {
                    if let Some(ty) = const_ty(&folded_kind) {
                        func.values[dest.0 as usize].ty = ty;
                    }
                    func.blocks[b_id.0 as usize].insts[inst_idx].kind = folded_kind.clone();
                    const_map.insert(dest, folded_kind);
                    changed = true;
                }
            }
        }

        let term = func.blocks[b_id.0 as usize].term.clone();
        if let Terminator::Branch {
            cond,
            then_blk,
            then_args,
            else_blk,
            else_args,
        } = term
            && let Some(SsaOp::ConstBool(b)) = const_map.get(&cond)
        {
            if *b {
                func.blocks[b_id.0 as usize].term = Terminator::Jump {
                    target: then_blk,
                    args: then_args,
                };
                let preds = &mut func.blocks[else_blk.0 as usize].preds;
                if let Some(pos) = preds.iter().position(|p| *p == b_id) {
                    preds.remove(pos);
                }
            } else {
                func.blocks[b_id.0 as usize].term = Terminator::Jump {
                    target: else_blk,
                    args: else_args,
                };
                let preds = &mut func.blocks[then_blk.0 as usize].preds;
                if let Some(pos) = preds.iter().position(|p| *p == b_id) {
                    preds.remove(pos);
                }
            }
            changed = true;
        }
    }

    changed
}

fn fold_inst(kind: &SsaOp, const_map: &FxHashMap<Value, SsaOp>) -> Option<SsaOp> {
    match kind {
        SsaOp::Unary { op, operand } => {
            let operand_const = const_map.get(operand)?;
            fold_unary(*op, operand_const)
        }
        SsaOp::KindOf { .. } => None,
        SsaOp::Binary { op, lhs, rhs } => {
            let lhs_const = const_map.get(lhs)?;
            let rhs_const = const_map.get(rhs)?;
            fold_binary(*op, lhs_const, rhs_const)
        }
        SsaOp::Convert { operand, conv } => fold_convert(*conv, const_map.get(operand)?),
        SsaOp::TypeTest { .. } => None,
        SsaOp::IsNull { operand } => {
            let operand_const = const_map.get(operand)?;
            match operand_const {
                SsaOp::ConstNull => Some(SsaOp::ConstBool(true)),
                SsaOp::ConstInt(_)
                | SsaOp::ConstFloat(_)
                | SsaOp::ConstBool(_)
                | SsaOp::ConstStr(_)
                | SsaOp::ConstChar(_)
                | SsaOp::ConstDecimal(_)
                | SsaOp::ConstBigInt(_)
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
                | SsaOp::BuildObjectSpread { .. } => Some(SsaOp::ConstBool(false)),
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
        | SsaOp::BuildObjectSpread { .. } => None,
    }
}
