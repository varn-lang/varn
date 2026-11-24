use vn_tir::BackendTy;
use vn_types::ssa::SsaOp;

pub(super) fn why_not(kind: &SsaOp, value_tys: &[BackendTy]) -> String {
    let ty = |v: &crate::ssa::ir::Value| value_tys.get(v.0 as usize).copied();
    match kind {
        SsaOp::Binary { op, lhs, rhs, .. } => {
            format!(
                "no portable form for {op:?} on {:?} and {:?}",
                ty(lhs),
                ty(rhs)
            )
        }
        SsaOp::Unary { op, operand, .. } => {
            format!("no portable form for {op:?} on {:?}", ty(operand))
        }
        other @ SsaOp::KindOf { .. }
        | other @ SsaOp::ConstInt(_)
        | other @ SsaOp::ConstFloat(_)
        | other @ SsaOp::ConstBool(_)
        | other @ SsaOp::ConstStr(_)
        | other @ SsaOp::ConstChar(_)
        | other @ SsaOp::ConstDecimal(_)
        | other @ SsaOp::ConstBigInt(_)
        | other @ SsaOp::ConstNull
        | other @ SsaOp::LoadGlobal(_)
        | other @ SsaOp::LoadGlobalIdx(_)
        | other @ SsaOp::LoadNativeGlobalIdx(_)
        | other @ SsaOp::LoadUpvalue(_)
        | other @ SsaOp::StoreGlobal { .. }
        | other @ SsaOp::StoreGlobalIdx { .. }
        | other @ SsaOp::StoreUpvalue { .. }
        | other @ SsaOp::Call { .. }
        | other @ SsaOp::CallDirect { .. }
        | other @ SsaOp::AllocInstance { .. }
        | other @ SsaOp::SelfCall { .. }
        | other @ SsaOp::GetProperty { .. }
        | other @ SsaOp::GetSlotMethod { .. }
        | other @ SsaOp::GetFixedField { .. }
        | other @ SsaOp::GetIndex { .. }
        | other @ SsaOp::ArrayGetIndex { .. }
        | other @ SsaOp::BytesGetIndex { .. }
        | other @ SsaOp::SetProperty { .. }
        | other @ SsaOp::SetFixedField { .. }
        | other @ SsaOp::SetIndex { .. }
        | other @ SsaOp::ArraySetIndex { .. }
        | other @ SsaOp::BytesSetIndex { .. }
        | other @ SsaOp::ArrayPush { .. }
        | other @ SsaOp::ObjectMerge { .. }
        | other @ SsaOp::MethodCall { .. }
        | other @ SsaOp::InvokeSlot { .. }
        | other @ SsaOp::IsNull { .. }
        | other @ SsaOp::TypeTest { .. }
        | other @ SsaOp::Cast { .. }
        | other @ SsaOp::Convert { .. }
        | other @ SsaOp::BuildArray { .. }
        | other @ SsaOp::BuildTuple { .. }
        | other @ SsaOp::BuildObject { .. }
        | other @ SsaOp::BuildMap { .. }
        | other @ SsaOp::ObjectRest { .. }
        | other @ SsaOp::ToString { .. }
        | other @ SsaOp::BuildStr { .. }
        | other @ SsaOp::MakeClosure { .. }
        | other @ SsaOp::LoadCaptured { .. }
        | other @ SsaOp::StoreCaptured { .. }
        | other @ SsaOp::MakeClass { .. }
        | other @ SsaOp::DeclareLayout { .. }
        | other @ SsaOp::FreezeClass { .. }
        | other @ SsaOp::DefineMember { .. }
        | other @ SsaOp::MakeEnumVariant { .. }
        | other @ SsaOp::Try { .. }
        | other @ SsaOp::PopTry
        | other @ SsaOp::CatchParam { .. }
        | other @ SsaOp::CloseUpvalues { .. }
        | other @ SsaOp::LoadModule { .. }
        | other @ SsaOp::StoreModuleSlot { .. }
        | other @ SsaOp::Await { .. }
        | other @ SsaOp::Spawn { .. }
        | other @ SsaOp::Yield { .. }
        | other @ SsaOp::IntrinsicCall { .. }
        | other @ SsaOp::CallNativeOp { .. }
        | other @ SsaOp::AssertNotNull { .. }
        | other @ SsaOp::GetPropertyMaybe { .. }
        | other @ SsaOp::ModuleSlot { .. }
        | other @ SsaOp::GetEnumTag { .. }
        | other @ SsaOp::IsArray { .. }
        | other @ SsaOp::StrLength { .. }
        | other @ SsaOp::ArrayLength { .. }
        | other @ SsaOp::BytesLength { .. }
        | other @ SsaOp::This
        | other @ SsaOp::Range { .. }
        | other @ SsaOp::ObjectKeys { .. }
        | other @ SsaOp::GetSuper { .. }
        | other @ SsaOp::GetSuperSlot { .. }
        | other @ SsaOp::SuperCall { .. }
        | other @ SsaOp::SuperMethodCall { .. }
        | other @ SsaOp::SuperInvokeSlot { .. }
        | other @ SsaOp::ExtensionCall { .. }
        | other @ SsaOp::CallSpread { .. }
        | other @ SsaOp::BuildArraySpread { .. }
        | other @ SsaOp::BuildObjectSpread { .. } => {
            let full = format!("{other:?}");
            let end = full.find([' ', '(', '{']).unwrap_or(full.len());
            format!("no portable form for {}", &full[..end])
        }
    }
}
