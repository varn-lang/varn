use super::numeric;
use super::Ctx;
use vn_types::ssa::{SsaOp, SsaProto};

pub(super) fn may_push_frame(ctx: &Ctx<'_>, op: &SsaOp) -> bool {
    if let SsaOp::SelfCall { .. } = op {
        return ctx.frame.is_some();
    }
    matches!(
        op,
        SsaOp::Call { .. }
            | SsaOp::CallDirect { .. }
            | SsaOp::CallNativeOp { .. }
            | SsaOp::MethodCall { .. }
            | SsaOp::InvokeSlot { .. }
            | SsaOp::SuperCall { .. }
            | SsaOp::SuperMethodCall { .. }
            | SsaOp::SuperInvokeSlot { .. }
            | SsaOp::ExtensionCall { .. }
            | SsaOp::CallSpread { .. }
            | SsaOp::GetProperty { .. }
            | SsaOp::SetProperty { .. }
            | SsaOp::GetIndex { .. }
            | SsaOp::SetIndex { .. }
            | SsaOp::GetPropertyMaybe { .. }
            | SsaOp::BuildObjectSpread { .. }
            | SsaOp::Spawn { .. }
            | SsaOp::LoadModule { .. }
            | SsaOp::Await { .. }
            | SsaOp::Yield { .. }
    )
}

pub(super) fn needs_frame(ssa: &SsaProto, op: &SsaOp) -> bool {
    matches!(
        op,
        SsaOp::Call { .. }
            | SsaOp::CallDirect { .. }
            | SsaOp::AllocInstance { .. }
            | SsaOp::CallNativeOp { .. }
            | SsaOp::MethodCall { .. }
            | SsaOp::InvokeSlot { .. }
            | SsaOp::LoadGlobalIdx(_)
            | SsaOp::LoadNativeGlobalIdx(_)
            | SsaOp::StoreGlobalIdx { .. }
            | SsaOp::MakeClosure { .. }
            | SsaOp::LoadCaptured { .. }
            | SsaOp::StoreCaptured { .. }
            | SsaOp::LoadUpvalue(_)
            | SsaOp::StoreUpvalue { .. }
            | SsaOp::CloseUpvalues { .. }
            | SsaOp::Try { .. }
            | SsaOp::PopTry
            | SsaOp::CatchParam { .. }
            | SsaOp::MakeEnumVariant { .. }
            | SsaOp::IntrinsicCall { .. }
            | SsaOp::ArrayGetIndex { .. }
            | SsaOp::BytesGetIndex { .. }
            | SsaOp::ArraySetIndex { .. }
            | SsaOp::BytesSetIndex { .. }
            | SsaOp::LoadGlobal(_)
            | SsaOp::StoreGlobal { .. }
            | SsaOp::BuildTuple { .. }
            | SsaOp::BuildArraySpread { .. }
            | SsaOp::BuildObjectSpread { .. }
            | SsaOp::ObjectMerge { .. }
            | SsaOp::ObjectRest { .. }
            | SsaOp::GetPropertyMaybe { .. }
            | SsaOp::AssertNotNull { .. }
            | SsaOp::Range { .. }
            | SsaOp::SuperCall { .. }
            | SsaOp::SuperMethodCall { .. }
            | SsaOp::SuperInvokeSlot { .. }
            | SsaOp::ExtensionCall { .. }
            | SsaOp::CallSpread { .. }
            | SsaOp::LoadModule { .. }
            | SsaOp::ModuleSlot { .. }
            | SsaOp::StoreModuleSlot { .. }
            | SsaOp::Await { .. }
            | SsaOp::Spawn { .. }
            | SsaOp::Yield { .. }
    ) || matches!(op, SsaOp::Convert { operand, conv }
        if !numeric::is_inline_convert(ssa, *operand, *conv))
}
