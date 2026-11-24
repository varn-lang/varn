use vn_types::ssa::SsaOp;

pub(super) fn project_class(kind: &SsaOp) -> Option<SsaOp> {
    match kind {
        SsaOp::GetFixedField {
            object,
            slot,
            offset,
            access,
        } => Some(SsaOp::GetFixedField {
            object: *object,
            slot: *slot,
            offset: *offset,
            access: *access,
        }),
        SsaOp::SetFixedField {
            object,
            value,
            slot,
            offset,
            kind,
        } => Some(SsaOp::SetFixedField {
            object: *object,
            value: *value,
            slot: *slot,
            offset: *offset,
            kind: *kind,
        }),
        SsaOp::MakeClass { name, super_class } => Some(SsaOp::MakeClass {
            name: name.clone(),
            super_class: *super_class,
        }),
        SsaOp::FreezeClass { class } => Some(SsaOp::FreezeClass { class: *class }),
        SsaOp::DeclareLayout { class, layout } => Some(SsaOp::DeclareLayout {
            class: *class,
            layout: layout.clone(),
        }),
        SsaOp::DefineMember {
            class,
            name,
            member,
            kind,
        } => Some(SsaOp::DefineMember {
            class: *class,
            name: name.clone(),
            member: *member,
            kind: *kind,
        }),
        SsaOp::GetSuper { name } => Some(SsaOp::GetSuper { name: name.clone() }),
        SsaOp::GetSuperSlot { slot } => Some(SsaOp::GetSuperSlot { slot: *slot }),
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
        | SsaOp::GetIndex { .. }
        | SsaOp::ArrayGetIndex { .. }
        | SsaOp::BytesGetIndex { .. }
        | SsaOp::SetProperty { .. }
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
        | SsaOp::SuperCall { .. }
        | SsaOp::SuperMethodCall { .. }
        | SsaOp::SuperInvokeSlot { .. }
        | SsaOp::ExtensionCall { .. }
        | SsaOp::CallSpread { .. }
        | SsaOp::BuildArraySpread { .. }
        | SsaOp::BuildObjectSpread { .. } => None,
    }
}
