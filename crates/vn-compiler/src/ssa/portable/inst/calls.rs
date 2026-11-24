use super::super::{Captured, Site};
use vn_types::ssa::{SsaOp, SsaUpvalue};

pub(super) fn try_project(kind: &SsaOp, site: &Site, captured: &mut Captured) -> Option<SsaOp> {
    match kind {
        SsaOp::Call {
            callee,
            args,
            shape,
        } => Some(SsaOp::Call {
            callee: *callee,
            args: args.iter().copied().collect(),
            shape: *shape,
        }),
        SsaOp::CallDirect {
            target,
            slot,
            callee,
            args,
            shape,
        } => match site.direct_slot {
            Some(_) => Some(SsaOp::CallDirect {
                target: target.clone(),
                slot: *slot,
                callee: *callee,
                args: args.iter().copied().collect(),
                shape: *shape,
            }),
            None => Some(SsaOp::Call {
                callee: *callee,
                args: args.iter().copied().collect(),
                shape: *shape,
            }),
        },
        SsaOp::SelfCall { args } => Some(SsaOp::SelfCall {
            args: args.iter().copied().collect(),
        }),
        SsaOp::MethodCall { recv, name, args } => Some(SsaOp::MethodCall {
            recv: *recv,
            name: name.clone(),
            args: args.iter().copied().collect(),
        }),
        SsaOp::InvokeSlot { recv, slot, args } => Some(SsaOp::InvokeSlot {
            recv: *recv,
            slot: *slot,
            args: args.iter().copied().collect(),
        }),
        SsaOp::CallNativeOp {
            object,
            args,
            op_id,
        } => Some(SsaOp::CallNativeOp {
            object: *object,
            args: args.iter().copied().collect(),
            op_id: *op_id,
        }),
        SsaOp::IntrinsicCall { object, args, wire } => Some(SsaOp::IntrinsicCall {
            object: *object,
            args: args.iter().copied().collect(),
            wire: *wire,
        }),
        SsaOp::SuperCall { args } => Some(SsaOp::SuperCall {
            args: args.iter().copied().collect(),
        }),
        SsaOp::SuperMethodCall { name, args } => Some(SsaOp::SuperMethodCall {
            name: name.clone(),
            args: args.iter().copied().collect(),
        }),
        SsaOp::SuperInvokeSlot { slot, args } => Some(SsaOp::SuperInvokeSlot {
            slot: *slot,
            args: args.iter().copied().collect(),
        }),
        SsaOp::ExtensionCall {
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
        SsaOp::CallSpread { callee, args } => Some(SsaOp::CallSpread {
            callee: *callee,
            args: args.clone(),
        }),
        SsaOp::MakeClosure { func, upvalues } => Some(SsaOp::MakeClosure {
            func: *func,
            upvalues: upvalues
                .iter()
                .map(|src| match src {
                    SsaUpvalue::Captured(var) => SsaUpvalue::Captured(captured.var(*var)),
                    SsaUpvalue::Inherited(idx) => SsaUpvalue::Inherited(*idx),
                })
                .collect(),
        }),
        SsaOp::LoadCaptured { var } => Some(SsaOp::LoadCaptured {
            var: captured.var(*var),
        }),
        SsaOp::StoreCaptured { var, value } => Some(SsaOp::StoreCaptured {
            var: captured.var(*var),
            value: *value,
        }),
        SsaOp::LoadUpvalue(index) => Some(SsaOp::LoadUpvalue(*index)),
        SsaOp::StoreUpvalue { index, value } => Some(SsaOp::StoreUpvalue {
            index: *index,
            value: *value,
        }),
        SsaOp::CloseUpvalues { vars } => Some(SsaOp::CloseUpvalues {
            vars: vars.iter().map(|t| captured.var(*t)).collect(),
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
        | SsaOp::StoreGlobal { .. }
        | SsaOp::StoreGlobalIdx { .. }
        | SsaOp::AllocInstance { .. }
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
        | SsaOp::MakeClass { .. }
        | SsaOp::DeclareLayout { .. }
        | SsaOp::FreezeClass { .. }
        | SsaOp::DefineMember { .. }
        | SsaOp::MakeEnumVariant { .. }
        | SsaOp::Try { .. }
        | SsaOp::PopTry
        | SsaOp::CatchParam { .. }
        | SsaOp::LoadModule { .. }
        | SsaOp::StoreModuleSlot { .. }
        | SsaOp::Await { .. }
        | SsaOp::Spawn { .. }
        | SsaOp::Yield { .. }
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
        | SsaOp::BuildArraySpread { .. }
        | SsaOp::BuildObjectSpread { .. } => None,
    }
}
