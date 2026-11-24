use crate::ssa::ir::{InstKind, SsaFunc};
use rustc_hash::FxHashSet;
use vn_tir::BackendTy;

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
                && matches!(inst.kind, InstKind::BuildArray { .. })
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
                InstKind::GetIndex { object, index } if array_values.contains(&object.0) => {
                    inst.kind = InstKind::ArrayGetIndex {
                        object: *object,
                        index: *index,
                    };
                    changed = true;
                }
                InstKind::SetIndex {
                    object,
                    index,
                    value,
                } if array_values.contains(&object.0) => {
                    inst.kind = InstKind::ArraySetIndex {
                        object: *object,
                        index: *index,
                        value: *value,
                    };
                    changed = true;
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
                | InstKind::KindOf { .. }
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
                | InstKind::SetProperty { .. }
                | InstKind::SetFixedField { .. }
                | InstKind::SetIndex { .. }
                | InstKind::ArraySetIndex { .. }
                | InstKind::BytesSetIndex { .. }
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
                | InstKind::BuildObjectSpread { .. } => {}
            }
        }
    }

    changed
}
