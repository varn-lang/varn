use crate::ssa::ir::InstKind;
use vn_types::ssa::SsaOp;

pub(super) fn project_class(kind: &InstKind) -> Option<SsaOp> {
    match kind {
        InstKind::GetFixedField {
            object,
            slot,
            offset,
            tag,
        } => Some(SsaOp::GetFixedField {
            object: object.0,
            slot: *slot,
            offset: *offset,
            access: *tag,
        }),
        InstKind::SetFixedField {
            object,
            value,
            slot,
            offset,
            tag,
        } => Some(SsaOp::SetFixedField {
            object: object.0,
            value: value.0,
            slot: *slot,
            offset: *offset,
            kind: *tag,
        }),
        InstKind::MakeClass { name, super_class } => Some(SsaOp::MakeClass {
            name: name.clone(),
            super_class: super_class.map(|v| v.0),
        }),
        InstKind::FreezeClass { class } => Some(SsaOp::FreezeClass { class: class.0 }),
        InstKind::DeclareLayout { class, layout } => Some(SsaOp::DeclareLayout {
            class: class.0,
            layout: layout.clone(),
        }),
        InstKind::DefineMember {
            class,
            name,
            value,
            kind,
        } => Some(SsaOp::DefineMember {
            class: class.0,
            name: name.clone(),
            member: value.0,
            kind: *kind,
        }),
        InstKind::GetSuper { name } => Some(SsaOp::GetSuper { name: name.clone() }),
        InstKind::GetSuperSlot { slot } => Some(SsaOp::GetSuperSlot { slot: *slot }),
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
        | InstKind::GetIndex { .. }
        | InstKind::ArrayGetIndex { .. }
        | InstKind::BytesGetIndex { .. }
        | InstKind::MapGetIndex { .. }
        | InstKind::SetProperty { .. }
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
        | InstKind::MakeEnumVariant { .. }
        | InstKind::Try { .. }
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
        | InstKind::SuperCall { .. }
        | InstKind::SuperMethodCall { .. }
        | InstKind::SuperInvokeSlot { .. }
        | InstKind::ExtensionCall { .. }
        | InstKind::CallSpread { .. }
        | InstKind::BuildArraySpread { .. }
        | InstKind::BuildObjectSpread { .. } => None,
    }
}
