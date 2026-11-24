use super::composite_class::project_class;
use super::composite_object::project_object;
use crate::ssa::ir::Value;
use vn_tir::BackendTy;
use vn_types::ssa::SsaOp;

pub(super) fn try_project(kind: &SsaOp, value_tys: &[BackendTy]) -> Option<SsaOp> {
    match kind {
        SsaOp::AllocInstance {
            class,
            payload_size,
            ..
        } => Some(SsaOp::AllocInstance {
            class: *class,
            payload_size: *payload_size,
        }),
        SsaOp::BuildStr { parts } => Some(SsaOp::BuildStr {
            parts: parts.iter().copied().collect(),
        }),
        SsaOp::BuildArray { elements, storage } => Some(SsaOp::BuildArray {
            elements: elements.iter().copied().collect(),
            storage: *storage,
        }),
        SsaOp::BuildMap { pairs } => Some(SsaOp::BuildMap {
            pairs: pairs.iter().copied().collect(),
        }),
        SsaOp::BuildObject { pairs } => Some(SsaOp::BuildObject {
            pairs: pairs.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        }),
        SsaOp::BuildTuple { elements } => Some(SsaOp::BuildTuple {
            elements: elements.iter().copied().collect(),
        }),
        SsaOp::BuildArraySpread { elements } => Some(SsaOp::BuildArraySpread {
            elements: elements.clone(),
        }),
        SsaOp::BuildObjectSpread { parts } => Some(SsaOp::BuildObjectSpread {
            parts: parts.clone(),
        }),
        SsaOp::GetProperty { object, name } => Some(SsaOp::GetProperty {
            object: *object,
            name: name.clone(),
        }),
        SsaOp::GetSlotMethod { object, slot } => Some(SsaOp::GetSlotMethod {
            object: *object,
            slot: *slot,
        }),
        SsaOp::SetProperty {
            object,
            name,
            value,
        } => Some(SsaOp::SetProperty {
            object: *object,
            value: *value,
            name: name.clone(),
        }),
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
        | SsaOp::MethodCall { .. }
        | SsaOp::InvokeSlot { .. }
        | SsaOp::IsNull { .. }
        | SsaOp::TypeTest { .. }
        | SsaOp::Cast { .. }
        | SsaOp::Convert { .. }
        | SsaOp::ObjectRest { .. }
        | SsaOp::ToString { .. }
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
        | SsaOp::CallSpread { .. } => project_indexed(kind, value_tys)
            .or_else(|| project_object(kind))
            .or_else(|| project_class(kind)),
    }
}

fn project_indexed(kind: &SsaOp, value_tys: &[BackendTy]) -> Option<SsaOp> {
    match kind {
        SsaOp::ArrayGetIndex { object, index } if is_int(value_tys, *index) => {
            Some(SsaOp::ArrayGetIndex {
                object: *object,
                index: *index,
            })
        }
        SsaOp::BytesGetIndex { object, index } if is_int(value_tys, *index) => {
            Some(SsaOp::BytesGetIndex {
                object: *object,
                index: *index,
            })
        }
        SsaOp::ArraySetIndex {
            object,
            index,
            value,
        } if is_int(value_tys, *index) => Some(SsaOp::ArraySetIndex {
            object: *object,
            index: *index,
            value: *value,
        }),
        SsaOp::BytesSetIndex {
            object,
            index,
            value,
        } if is_int(value_tys, *index) => Some(SsaOp::BytesSetIndex {
            object: *object,
            index: *index,
            value: *value,
        }),
        SsaOp::GetIndex { object, index }
        | SsaOp::ArrayGetIndex { object, index }
        | SsaOp::BytesGetIndex { object, index } => Some(SsaOp::GetIndex {
            object: *object,
            index: *index,
        }),
        SsaOp::SetIndex {
            object,
            index,
            value,
        }
        | SsaOp::ArraySetIndex {
            object,
            index,
            value,
        }
        | SsaOp::BytesSetIndex {
            object,
            index,
            value,
        } => Some(SsaOp::SetIndex {
            object: *object,
            index: *index,
            value: *value,
        }),
        SsaOp::ArrayPush { array, value } => Some(SsaOp::ArrayPush {
            array: *array,
            value: *value,
        }),
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
        | SsaOp::SetProperty { .. }
        | SsaOp::SetFixedField { .. }
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
        | SsaOp::BuildObjectSpread { .. } => None,
    }
}

fn is_int(value_tys: &[BackendTy], v: Value) -> bool {
    value_tys.get(v.0 as usize).is_some_and(|t| {
        crate::ssa::emit::slot_kind_of(*t) == vn_types::register_meta::SlotKind::Int
    })
}
