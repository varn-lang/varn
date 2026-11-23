use super::super::{Captured, Site};
use crate::ssa::ir::{InstKind, SsaVar};
use vn_types::ssa::{SsaOp, SsaUpvalue};

pub(super) fn try_project(kind: &InstKind, site: &Site, captured: &mut Captured) -> Option<SsaOp> {
    match kind {
        InstKind::Call { callee, args, .. } => Some(SsaOp::Call {
            callee: *callee,
            args: args.iter().copied().collect(),
        }),
        InstKind::CallDirect {
            target,
            slot,
            callee,
            args,
            ..
        } => match site.direct_slot {
            Some(_) => Some(SsaOp::CallDirect {
                target: target.clone(),
                slot: *slot,
                callee: *callee,
                args: args.iter().copied().collect(),
            }),
            None => Some(SsaOp::Call {
                callee: *callee,
                args: args.iter().copied().collect(),
            }),
        },
        InstKind::SelfCall { args } => Some(SsaOp::SelfCall {
            args: args.iter().copied().collect(),
        }),
        InstKind::MethodCall { recv, name, args } => Some(SsaOp::MethodCall {
            recv: *recv,
            name: name.clone(),
            args: args.iter().copied().collect(),
        }),
        InstKind::InvokeSlot { recv, slot, args } => Some(SsaOp::InvokeSlot {
            recv: *recv,
            slot: *slot,
            args: args.iter().copied().collect(),
        }),
        InstKind::CallNativeOp {
            object,
            args,
            op_id,
        } => Some(SsaOp::CallNativeOp {
            object: *object,
            args: args.iter().copied().collect(),
            op_id: *op_id,
        }),
        InstKind::IntrinsicCall {
            object,
            args,
            wire_byte,
        } => Some(SsaOp::IntrinsicCall {
            object: *object,
            args: args.iter().copied().collect(),
            wire: *wire_byte,
        }),
        InstKind::SuperCall { args } => Some(SsaOp::SuperCall {
            args: args.iter().copied().collect(),
        }),
        InstKind::SuperMethodCall { name, args } => Some(SsaOp::SuperMethodCall {
            name: name.clone(),
            args: args.iter().copied().collect(),
        }),
        InstKind::SuperInvokeSlot { slot, args } => Some(SsaOp::SuperInvokeSlot {
            slot: *slot,
            args: args.iter().copied().collect(),
        }),
        InstKind::ExtensionCall {
            func,
            slot,
            recv,
            args,
        } => Some(SsaOp::ExtensionCall {
            func: func.clone(),
            slot: *slot,
            recv: *recv,
            args: args.iter().copied().collect(),
        }),
        InstKind::CallSpread { callee, args } => Some(SsaOp::CallSpread {
            callee: *callee,
            args: args.clone(),
        }),
        InstKind::MakeClosure { upvalues_src, .. } => Some(SsaOp::MakeClosure {
            proto: u32::from(site.closure_const?),
            upvalues: upvalues_src
                .iter()
                .map(|src| match src {
                    vn_tir::TirUpvalue::ParentLocal(id) => {
                        SsaUpvalue::Captured(captured.var(SsaVar::Local(*id)))
                    }
                    vn_tir::TirUpvalue::ParentParam(i) => {
                        SsaUpvalue::Captured(captured.var(SsaVar::Param(*i)))
                    }
                    vn_tir::TirUpvalue::ParentUpvalue(idx) => SsaUpvalue::Inherited(*idx),
                })
                .collect(),
        }),
        InstKind::LoadCaptured { var } => Some(SsaOp::LoadCaptured {
            var: captured.var(*var),
        }),
        InstKind::StoreCaptured { var, value } => Some(SsaOp::StoreCaptured {
            var: captured.var(*var),
            value: *value,
        }),
        InstKind::LoadUpvalue(index) => Some(SsaOp::LoadUpvalue(*index)),
        InstKind::StoreUpvalue { index, value } => Some(SsaOp::StoreUpvalue {
            index: *index,
            value: *value,
        }),
        InstKind::CloseUpvalues { vars } => Some(SsaOp::CloseUpvalues {
            vars: vars.iter().map(|t| captured.var(*t)).collect(),
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
        | InstKind::StoreGlobal { .. }
        | InstKind::StoreGlobalIdx { .. }
        | InstKind::AllocInstance { .. }
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
        | InstKind::MakeClass { .. }
        | InstKind::DeclareLayout { .. }
        | InstKind::FreezeClass { .. }
        | InstKind::DefineMember { .. }
        | InstKind::MakeEnumVariant { .. }
        | InstKind::Try { .. }
        | InstKind::PopTry
        | InstKind::CatchParam { .. }
        | InstKind::LoadModule { .. }
        | InstKind::StoreModuleSlot { .. }
        | InstKind::Await { .. }
        | InstKind::Spawn { .. }
        | InstKind::Yield { .. }
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
        | InstKind::BuildArraySpread { .. }
        | InstKind::BuildObjectSpread { .. } => None,
    }
}
