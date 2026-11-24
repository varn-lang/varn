use vn_types::ssa::SsaOp;

pub(super) fn try_project(kind: &SsaOp, _value_tys: &[vn_tir::BackendTy]) -> Option<SsaOp> {
    match kind {
        SsaOp::Binary { op, lhs, rhs } => Some(SsaOp::Binary {
            op: *op,
            lhs: *lhs,
            rhs: *rhs,
        }),
        SsaOp::Unary { op, operand } => Some(SsaOp::Unary {
            op: *op,
            operand: *operand,
        }),
        SsaOp::KindOf { operand } => Some(SsaOp::KindOf { operand: *operand }),
        SsaOp::IsArray { operand } => Some(SsaOp::IsArray { operand: *operand }),
        SsaOp::ToString { operand } => Some(SsaOp::ToString { operand: *operand }),
        SsaOp::ObjectKeys { operand } => Some(SsaOp::ObjectKeys { operand: *operand }),
        SsaOp::GetEnumTag { operand } => Some(SsaOp::GetEnumTag { operand: *operand }),
        SsaOp::Cast { operand, .. } => Some(SsaOp::Cast { operand: *operand }),
        SsaOp::Convert { operand, conv } => Some(SsaOp::Convert {
            operand: *operand,
            conv: *conv,
        }),
        SsaOp::IsNull { operand } => Some(SsaOp::IsNull { operand: *operand }),
        SsaOp::TypeTest { value, class } => Some(SsaOp::TypeTest {
            value: *value,
            class: *class,
        }),
        SsaOp::AssertNotNull { operand } => Some(SsaOp::AssertNotNull { operand: *operand }),
        SsaOp::Range {
            start,
            end,
            inclusive,
        } => Some(SsaOp::Range {
            start: *start,
            end: *end,
            inclusive: *inclusive,
        }),
        SsaOp::PopTry => Some(SsaOp::PopTry),
        SsaOp::CatchParam { try_val } => Some(SsaOp::CatchParam { try_val: *try_val }),
        SsaOp::Await { operand } => Some(SsaOp::Await { operand: *operand }),
        SsaOp::Spawn { operand } => Some(SsaOp::Spawn { operand: *operand }),
        SsaOp::Yield { operand } => Some(SsaOp::Yield { operand: *operand }),
        SsaOp::ConstInt(_)
        | SsaOp::ConstFloat(_)
        | SsaOp::ConstBool(_)
        | SsaOp::ConstStr(_)
        | SsaOp::ConstChar(_)
        | SsaOp::ConstDecimal(_)
        | SsaOp::ConstBigInt(_)
        | SsaOp::ConstNull
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
        | SsaOp::BuildArray { .. }
        | SsaOp::BuildTuple { .. }
        | SsaOp::BuildObject { .. }
        | SsaOp::BuildMap { .. }
        | SsaOp::ObjectRest { .. }
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
        | SsaOp::CloseUpvalues { .. }
        | SsaOp::LoadModule { .. }
        | SsaOp::StoreModuleSlot { .. }
        | SsaOp::IntrinsicCall { .. }
        | SsaOp::CallNativeOp { .. }
        | SsaOp::GetPropertyMaybe { .. }
        | SsaOp::ModuleSlot { .. }
        | SsaOp::StrLength { .. }
        | SsaOp::ArrayLength { .. }
        | SsaOp::BytesLength { .. }
        | SsaOp::This
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
