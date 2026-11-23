use super::{SsaOp, SsaTerm};

impl SsaOp {
    pub fn for_each_use(&self, mut f: impl FnMut(u32)) {
        match self {
            SsaOp::ConstInt(_)
            | SsaOp::ConstFloat(_)
            | SsaOp::ConstBool(_)
            | SsaOp::ConstNull
            | SsaOp::ConstStr(_)
            | SsaOp::ConstChar(_)
            | SsaOp::ConstBigInt(_)
            | SsaOp::ConstDecimal(_)
            | SsaOp::MakeEnumVariant { .. }
            | SsaOp::LoadGlobalIdx(_)
            | SsaOp::LoadNativeGlobalIdx(_)
            | SsaOp::PopTry
            | SsaOp::LoadCaptured { .. }
            | SsaOp::LoadUpvalue(_)
            | SsaOp::This
            | SsaOp::GetSuper { .. }
            | SsaOp::GetSuperSlot { .. }
            | SsaOp::LoadGlobal(_)
            | SsaOp::AllocInstance { .. } => {}
            SsaOp::Binary { lhs, rhs, .. } => {
                f(*lhs);
                f(*rhs);
            }
            SsaOp::Unary { operand, .. }
            | SsaOp::Cast { operand }
            | SsaOp::Convert { operand, .. }
            | SsaOp::IsNull { operand }
            | SsaOp::KindOf { operand }
            | SsaOp::ToString { operand }
            | SsaOp::IsArray { operand }
            | SsaOp::GetEnumTag { operand }
            | SsaOp::ObjectKeys { operand }
            | SsaOp::ArrayLength { operand }
            | SsaOp::StrLength { operand }
            | SsaOp::BytesLength { operand }
            | SsaOp::AssertNotNull { operand }
            | SsaOp::WrapSpread { operand }
            | SsaOp::Spawn { operand } => f(*operand),
            SsaOp::SelfCall { args } | SsaOp::SuperCall { args } => args.iter().for_each(|a| f(*a)),
            SsaOp::SuperMethodCall { args, .. } | SsaOp::SuperInvokeSlot { args, .. } => {
                args.iter().for_each(|a| f(*a))
            }
            SsaOp::Call { callee, args } | SsaOp::CallDirect { callee, args, .. } => {
                f(*callee);
                args.iter().for_each(|a| f(*a));
            }
            SsaOp::ArrayGetIndex { object, index }
            | SsaOp::BytesGetIndex { object, index }
            | SsaOp::GetIndex { object, index } => {
                f(*object);
                f(*index);
            }
            SsaOp::ArraySetIndex {
                object,
                index,
                value,
            }
            | SsaOp::BytesSetIndex {
                object,
                index,
                value,
            }
            | SsaOp::SetIndex {
                object,
                index,
                value,
            } => {
                f(*object);
                f(*index);
                f(*value);
            }
            SsaOp::IntrinsicCall { object, args, .. }
            | SsaOp::CallNativeOp { object, args, .. } => {
                f(*object);
                args.iter().for_each(|a| f(*a));
            }
            SsaOp::Try {
                catch_value, live, ..
            } => {
                f(*catch_value);
                live.iter().for_each(|a| f(*a));
            }
            SsaOp::CatchParam { try_val } => f(*try_val),
            SsaOp::StoreGlobalIdx { value, .. }
            | SsaOp::StoreCaptured { value, .. }
            | SsaOp::StoreUpvalue { value, .. }
            | SsaOp::StoreGlobal { value, .. }
            | SsaOp::StoreModuleSlot { value, .. } => f(*value),
            SsaOp::MakeClosure { .. } => {}
            SsaOp::CloseUpvalues { vars } => vars.iter().for_each(|a| f(*a)),
            SsaOp::TypeTest { value, class } => {
                f(*value);
                f(*class);
            }
            SsaOp::BuildStr { parts } => parts.iter().for_each(|a| f(*a)),
            SsaOp::BuildArray { elements, .. } | SsaOp::BuildTuple { elements } => {
                elements.iter().for_each(|a| f(*a))
            }
            SsaOp::BuildMap { pairs } => pairs.iter().for_each(|(k, v)| {
                f(*k);
                f(*v);
            }),
            SsaOp::BuildObject { values, .. } => values.iter().for_each(|a| f(*a)),
            SsaOp::GetProperty { object, .. }
            | SsaOp::GetSlotMethod { object, .. }
            | SsaOp::GetFixedField { object, .. }
            | SsaOp::ObjectRest { object, .. }
            | SsaOp::GetPropertyMaybe { object, .. }
            | SsaOp::BindMethod { object, .. }
            | SsaOp::ModuleSlot { object, .. } => f(*object),
            SsaOp::SetProperty { object, value, .. }
            | SsaOp::SetFixedField { object, value, .. } => {
                f(*object);
                f(*value);
            }
            SsaOp::MethodCall { recv, args, .. }
            | SsaOp::InvokeSlot { recv, args, .. }
            | SsaOp::ExtensionCall { recv, args, .. } => {
                f(*recv);
                args.iter().for_each(|a| f(*a));
            }
            SsaOp::ArrayPush { array, value } => {
                f(*array);
                f(*value);
            }
            SsaOp::MakeClass { super_class, .. } => {
                if let Some(s) = super_class {
                    f(*s);
                }
            }
            SsaOp::FreezeClass { class }
            | SsaOp::DeclareLayout { class, .. }
            | SsaOp::DefineMember { class, .. } => f(*class),
            SsaOp::BuildArraySpread { elements } => elements.iter().for_each(|e| f(e.value)),
            SsaOp::BuildObjectSpread { parts, .. } => parts.iter().for_each(|p| f(p.value)),
            SsaOp::ObjectMerge { target, source } => {
                f(*target);
                f(*source);
            }
            SsaOp::ArrayExtend { array, source } => {
                f(*array);
                f(*source);
            }
            SsaOp::Range { start, end, .. } => {
                f(*start);
                f(*end);
            }
            SsaOp::CallSpread { callee, args } => {
                f(*callee);
                args.iter().for_each(|e| f(e.value));
            }
            SsaOp::LoadModule { live, .. } => live.iter().for_each(|a| f(*a)),
            SsaOp::Await { operand, live, .. } | SsaOp::Yield { operand, live, .. } => {
                f(*operand);
                live.iter().for_each(|a| f(*a));
            }
            SsaOp::Dispose { var, .. } => f(*var),
        }
    }
}

impl SsaTerm {
    pub fn for_each_use(&self, mut f: impl FnMut(u32)) {
        match self {
            SsaTerm::Return(Some(v)) | SsaTerm::Throw(v) => f(*v),
            SsaTerm::Return(None) | SsaTerm::Unreachable => {}
            SsaTerm::Jump { args, .. } => args.iter().for_each(|a| f(*a)),
            SsaTerm::Branch {
                cond,
                then_args,
                else_args,
                ..
            } => {
                f(*cond);
                then_args.iter().for_each(|a| f(*a));
                else_args.iter().for_each(|a| f(*a));
            }
        }
    }
}
