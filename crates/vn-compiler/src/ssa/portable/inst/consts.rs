use crate::ssa::ir::InstKind;
use vn_types::ssa::SsaOp;

pub(super) fn try_project(kind: &InstKind) -> Option<SsaOp> {
    match kind {
        InstKind::ConstInt(n) => Some(SsaOp::ConstInt(*n)),
        InstKind::ConstFloat(f) => Some(SsaOp::ConstFloat(*f)),
        InstKind::ConstBool(b) => Some(SsaOp::ConstBool(*b)),
        InstKind::ConstNull => Some(SsaOp::ConstNull),
        InstKind::ConstStr(s) => Some(SsaOp::ConstStr(s.clone())),
        InstKind::ConstChar(c) => Some(SsaOp::ConstChar(*c)),
        InstKind::ConstBigInt(digits) => Some(SsaOp::ConstBigInt(digits.clone())),
        InstKind::ConstDecimal(d) => Some(SsaOp::ConstDecimal(d.clone())),
        InstKind::MakeEnumVariant { def } => Some(SsaOp::MakeEnumVariant { def: def.clone() }),
        InstKind::LoadGlobalIdx(slot) => Some(SsaOp::LoadGlobalIdx(*slot)),
        InstKind::LoadNativeGlobalIdx(slot) => Some(SsaOp::LoadNativeGlobalIdx(*slot)),
        InstKind::StoreGlobalIdx { slot, value } => Some(SsaOp::StoreGlobalIdx {
            slot: *slot,
            value: *value,
        }),
        InstKind::LoadGlobal(name) => Some(SsaOp::LoadGlobal(name.clone())),
        InstKind::StoreGlobal { name, value } => Some(SsaOp::StoreGlobal {
            name: name.clone(),
            value: *value,
        }),
        InstKind::LoadModule { source } => Some(SsaOp::LoadModule {
            source: source.clone(),
        }),
        InstKind::ModuleSlot { object, slot } => Some(SsaOp::ModuleSlot {
            object: *object,
            slot: *slot,
        }),
        InstKind::StoreModuleSlot { value, slot } => Some(SsaOp::StoreModuleSlot {
            slot: *slot,
            value: *value,
        }),
        InstKind::Binary { .. }
        | InstKind::Unary { .. }
        | InstKind::LoadUpvalue(_)
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
        | InstKind::Try { .. }
        | InstKind::PopTry
        | InstKind::CatchParam { .. }
        | InstKind::CloseUpvalues { .. }
        | InstKind::Await { .. }
        | InstKind::Spawn { .. }
        | InstKind::Yield { .. }
        | InstKind::IntrinsicCall { .. }
        | InstKind::CallNativeOp { .. }
        | InstKind::AssertNotNull { .. }
        | InstKind::GetPropertyMaybe { .. }
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
    }
}
