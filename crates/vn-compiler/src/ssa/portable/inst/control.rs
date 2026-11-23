use crate::ssa::ir::InstKind;
use vn_tir::BackendTy;
use vn_types::ssa::SsaOp;

pub(super) fn try_project(kind: &InstKind, value_tys: &[BackendTy]) -> Option<SsaOp> {
    match kind {
        InstKind::Binary { op, lhs, rhs, ty } => {
            let ty_slot = crate::ssa::emit::slot_kind_of(*ty);
            let lhs_slot = value_tys
                .get(lhs.0 as usize)
                .map(|t| crate::ssa::emit::slot_kind_of(*t))
                .unwrap_or(vn_types::register_meta::SlotKind::Dynamic);
            let rhs_slot = value_tys
                .get(rhs.0 as usize)
                .map(|t| crate::ssa::emit::slot_kind_of(*t))
                .unwrap_or(vn_types::register_meta::SlotKind::Dynamic);
            Some(SsaOp::Binary {
                op: vn_types::ssa::select_bin(*op, ty_slot, lhs_slot, rhs_slot),
                lhs: lhs.0,
                rhs: rhs.0,
            })
        }
        InstKind::Unary {
            op: vn_core::UnOp::KindOf,
            operand,
            ..
        } => Some(SsaOp::KindOf { operand: operand.0 }),
        InstKind::Unary { op, operand, .. } => {
            let operand_slot = value_tys
                .get(operand.0 as usize)
                .map(|t| crate::ssa::emit::slot_kind_of(*t))
                .unwrap_or(vn_types::register_meta::SlotKind::Dynamic);
            Some(SsaOp::Unary {
                op: vn_types::ssa::select_un(*op, operand_slot)?,
                operand: operand.0,
            })
        }
        InstKind::IsArray { operand } => Some(SsaOp::IsArray { operand: operand.0 }),
        InstKind::ToString { operand } => Some(SsaOp::ToString { operand: operand.0 }),
        InstKind::ObjectKeys { operand } => Some(SsaOp::ObjectKeys { operand: operand.0 }),
        InstKind::GetEnumTag { operand } => Some(SsaOp::GetEnumTag { operand: operand.0 }),
        InstKind::Cast { operand, .. } => Some(SsaOp::Cast { operand: operand.0 }),
        InstKind::Convert { operand, conv } => Some(SsaOp::Convert {
            operand: operand.0,
            conv: *conv,
        }),
        InstKind::IsNull { operand } => Some(SsaOp::IsNull { operand: operand.0 }),
        InstKind::TypeTest { value, class } => Some(SsaOp::TypeTest {
            value: value.0,
            class: class.0,
        }),
        InstKind::AssertNotNull { operand } => Some(SsaOp::AssertNotNull { operand: operand.0 }),
        InstKind::Range {
            start,
            end,
            inclusive,
        } => Some(SsaOp::Range {
            start: start.0,
            end: end.0,
            inclusive: *inclusive,
        }),
        InstKind::PopTry => Some(SsaOp::PopTry),
        InstKind::CatchParam { try_val } => Some(SsaOp::CatchParam { try_val: try_val.0 }),
        InstKind::Await { operand } => Some(SsaOp::Await { operand: operand.0 }),
        InstKind::Spawn { operand } => Some(SsaOp::Spawn { operand: operand.0 }),
        InstKind::Yield { operand } => Some(SsaOp::Yield { operand: operand.0 }),
        InstKind::ConstInt(_)
        | InstKind::ConstFloat(_)
        | InstKind::ConstBool(_)
        | InstKind::ConstStr(_)
        | InstKind::ConstChar(_)
        | InstKind::ConstDecimal(_)
        | InstKind::ConstBigInt(_)
        | InstKind::ConstNull
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
        | InstKind::MapGetIndex { .. }
        | InstKind::SetProperty { .. }
        | InstKind::SetFixedField { .. }
        | InstKind::SetIndex { .. }
        | InstKind::ArraySetIndex { .. }
        | InstKind::BytesSetIndex { .. }
        | InstKind::MapSetIndex { .. }
        | InstKind::ArrayPush { .. }
        | InstKind::ObjectMerge { .. }
        | InstKind::MethodCall { .. }
        | InstKind::InvokeSlot { .. }
        | InstKind::BuildArray { .. }
        | InstKind::BuildTuple { .. }
        | InstKind::BuildObject { .. }
        | InstKind::BuildMap { .. }
        | InstKind::ObjectRest { .. }
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
        | InstKind::CloseUpvalues { .. }
        | InstKind::LoadModule { .. }
        | InstKind::StoreModuleSlot { .. }
        | InstKind::IntrinsicCall { .. }
        | InstKind::CallNativeOp { .. }
        | InstKind::GetPropertyMaybe { .. }
        | InstKind::ModuleSlot { .. }
        | InstKind::StrLength { .. }
        | InstKind::ArrayLength { .. }
        | InstKind::BytesLength { .. }
        | InstKind::This
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
