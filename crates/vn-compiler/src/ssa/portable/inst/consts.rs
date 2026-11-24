use vn_types::ssa::SsaOp;

pub(super) fn try_project(kind: &SsaOp) -> Option<SsaOp> {
    match kind {
        SsaOp::ConstInt(n) => Some(SsaOp::ConstInt(*n)),
        SsaOp::ConstFloat(f) => Some(SsaOp::ConstFloat(*f)),
        SsaOp::ConstBool(b) => Some(SsaOp::ConstBool(*b)),
        SsaOp::ConstNull => Some(SsaOp::ConstNull),
        SsaOp::ConstStr(s) => Some(SsaOp::ConstStr(s.clone())),
        SsaOp::ConstChar(c) => Some(SsaOp::ConstChar(*c)),
        SsaOp::ConstBigInt(digits) => Some(SsaOp::ConstBigInt(digits.clone())),
        SsaOp::ConstDecimal(d) => Some(SsaOp::ConstDecimal(d.clone())),
        SsaOp::MakeEnumVariant { def } => Some(SsaOp::MakeEnumVariant { def: def.clone() }),
        SsaOp::LoadGlobalIdx(slot) => Some(SsaOp::LoadGlobalIdx(*slot)),
        SsaOp::LoadNativeGlobalIdx(slot) => Some(SsaOp::LoadNativeGlobalIdx(*slot)),
        SsaOp::StoreGlobalIdx { slot, value } => Some(SsaOp::StoreGlobalIdx {
            slot: *slot,
            value: *value,
        }),
        SsaOp::LoadGlobal(name) => Some(SsaOp::LoadGlobal(name.clone())),
        SsaOp::StoreGlobal { name, value } => Some(SsaOp::StoreGlobal {
            name: name.clone(),
            value: *value,
        }),
        SsaOp::LoadModule { source } => Some(SsaOp::LoadModule {
            source: source.clone(),
        }),
        SsaOp::ModuleSlot { object, slot } => Some(SsaOp::ModuleSlot {
            object: *object,
            slot: *slot,
        }),
        SsaOp::StoreModuleSlot { value, slot } => Some(SsaOp::StoreModuleSlot {
            slot: *slot,
            value: *value,
        }),
        SsaOp::Binary { .. }
        | SsaOp::Unary { .. }
        | SsaOp::KindOf { .. }
        | SsaOp::LoadUpvalue(_)
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
        | SsaOp::Try { .. }
        | SsaOp::PopTry
        | SsaOp::CatchParam { .. }
        | SsaOp::CloseUpvalues { .. }
        | SsaOp::Await { .. }
        | SsaOp::Spawn { .. }
        | SsaOp::Yield { .. }
        | SsaOp::IntrinsicCall { .. }
        | SsaOp::CallNativeOp { .. }
        | SsaOp::AssertNotNull { .. }
        | SsaOp::GetPropertyMaybe { .. }
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
