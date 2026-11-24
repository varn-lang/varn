use crate::ssa::ir::SsaFunc;
use rustc_hash::FxHashSet;
use vn_tir::BackendTy;
use vn_types::ssa::SsaOp;

pub fn run(func: &mut SsaFunc) -> bool {
    let mut changed = false;
    let mut array_values: FxHashSet<u32> = FxHashSet::default();

    for (i, vdef) in func.values.iter().enumerate() {
        if matches!(vdef.ty, BackendTy::Array(_)) {
            array_values.insert(i as u32);
        }
    }

    for block in &func.blocks {
        for inst in &block.insts {
            if let Some(d) = inst.dest
                && matches!(inst.kind, SsaOp::BuildArray { .. })
            {
                array_values.insert(d.0);
            }
        }
    }

    if array_values.is_empty() {
        return false;
    }

    for block in &mut func.blocks {
        for inst in &mut block.insts {
            match &inst.kind {
                SsaOp::GetIndex { object, index } if array_values.contains(&object.0) => {
                    inst.kind = SsaOp::ArrayGetIndex {
                        object: *object,
                        index: *index,
                    };
                    changed = true;
                }
                SsaOp::SetIndex {
                    object,
                    index,
                    value,
                } if array_values.contains(&object.0) => {
                    inst.kind = SsaOp::ArraySetIndex {
                        object: *object,
                        index: *index,
                        value: *value,
                    };
                    changed = true;
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
                | SsaOp::BuildObjectSpread { .. } => {}
            }
        }
    }

    changed
}
