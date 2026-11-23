use super::composite_class::project_class;
use super::composite_object::project_object;
use crate::ssa::ir::{InstKind, Value};
use vn_tir::BackendTy;
use vn_types::ssa::SsaOp;

pub(super) fn try_project(kind: &InstKind, value_tys: &[BackendTy]) -> Option<SsaOp> {
    match kind {
        InstKind::AllocInstance {
            class,
            payload_size,
            ..
        } => Some(SsaOp::AllocInstance {
            class: *class,
            payload_size: *payload_size,
        }),
        InstKind::BuildStr { parts } => Some(SsaOp::BuildStr {
            parts: parts.iter().copied().collect(),
        }),
        InstKind::BuildArray { elements, storage } => Some(SsaOp::BuildArray {
            elements: elements.iter().copied().collect(),
            storage: *storage,
        }),
        InstKind::BuildMap { pairs } => Some(SsaOp::BuildMap {
            pairs: pairs.iter().copied().collect(),
        }),
        InstKind::BuildObject { pairs } => Some(SsaOp::BuildObject {
            pairs: pairs.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        }),
        InstKind::BuildTuple { elements } => Some(SsaOp::BuildTuple {
            elements: elements.iter().copied().collect(),
        }),
        InstKind::BuildArraySpread { elements } => Some(SsaOp::BuildArraySpread {
            elements: elements.clone(),
        }),
        InstKind::BuildObjectSpread { parts } => Some(SsaOp::BuildObjectSpread {
            parts: parts.clone(),
        }),
        InstKind::GetProperty { object, name } => Some(SsaOp::GetProperty {
            object: *object,
            name: name.clone(),
        }),
        InstKind::GetSlotMethod { object, slot } => Some(SsaOp::GetSlotMethod {
            object: *object,
            slot: *slot,
        }),
        InstKind::SetProperty {
            object,
            name,
            value,
        } => Some(SsaOp::SetProperty {
            object: *object,
            value: *value,
            name: name.clone(),
        }),
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
        | InstKind::SelfCall { .. }
        | InstKind::GetFixedField { .. }
        | InstKind::GetIndex { .. }
        | InstKind::ArrayGetIndex { .. }
        | InstKind::BytesGetIndex { .. }
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
        | InstKind::ObjectRest { .. }
        | InstKind::ToString { .. }
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
        | InstKind::CallSpread { .. } => project_indexed(kind, value_tys)
            .or_else(|| project_object(kind))
            .or_else(|| project_class(kind)),
    }
}

fn project_indexed(kind: &InstKind, value_tys: &[BackendTy]) -> Option<SsaOp> {
    match kind {
        InstKind::ArrayGetIndex { object, index } if is_int(value_tys, *index) => {
            Some(SsaOp::ArrayGetIndex {
                object: *object,
                index: *index,
            })
        }
        InstKind::BytesGetIndex { object, index } if is_int(value_tys, *index) => {
            Some(SsaOp::BytesGetIndex {
                object: *object,
                index: *index,
            })
        }
        InstKind::ArraySetIndex {
            object,
            index,
            value,
        } if is_int(value_tys, *index) => Some(SsaOp::ArraySetIndex {
            object: *object,
            index: *index,
            value: *value,
        }),
        InstKind::BytesSetIndex {
            object,
            index,
            value,
        } if is_int(value_tys, *index) => Some(SsaOp::BytesSetIndex {
            object: *object,
            index: *index,
            value: *value,
        }),
        InstKind::GetIndex { object, index }
        | InstKind::ArrayGetIndex { object, index }
        | InstKind::BytesGetIndex { object, index } => Some(SsaOp::GetIndex {
            object: *object,
            index: *index,
        }),
        InstKind::SetIndex {
            object,
            index,
            value,
        }
        | InstKind::ArraySetIndex {
            object,
            index,
            value,
        }
        | InstKind::BytesSetIndex {
            object,
            index,
            value,
        } => Some(SsaOp::SetIndex {
            object: *object,
            index: *index,
            value: *value,
        }),
        InstKind::ArrayPush { array, value } => Some(SsaOp::ArrayPush {
            array: *array,
            value: *value,
        }),
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
        | InstKind::GetFixedField { .. }
        | InstKind::SetProperty { .. }
        | InstKind::SetFixedField { .. }
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
        | InstKind::BuildObjectSpread { .. } => None,
    }
}

fn is_int(value_tys: &[BackendTy], v: Value) -> bool {
    value_tys.get(v.0 as usize).is_some_and(|t| {
        crate::ssa::emit::slot_kind_of(*t) == vn_types::register_meta::SlotKind::Int
    })
}
