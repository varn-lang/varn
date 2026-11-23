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
            SsaOp::Try { catch_value } => f(*catch_value),
            SsaOp::CatchParam { try_val } => f(*try_val),
            SsaOp::StoreGlobalIdx { value, .. }
            | SsaOp::StoreCaptured { value, .. }
            | SsaOp::StoreUpvalue { value, .. }
            | SsaOp::StoreGlobal { value, .. }
            | SsaOp::StoreModuleSlot { value, .. } => f(*value),
            SsaOp::MakeClosure { .. } => {}
            SsaOp::CloseUpvalues { .. } => {}
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
            SsaOp::BuildObject { pairs } => pairs.iter().for_each(|(_, v)| f(*v)),
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
            SsaOp::LoadModule { .. } => {}
            SsaOp::Await { operand } | SsaOp::Yield { operand } => f(*operand),
            SsaOp::Dispose { .. } => {}
        }
    }
}

impl<V: Copy, B: Copy> SsaTerm<V, B> {
    pub fn for_each_use(&self, mut f: impl FnMut(V)) {
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

    pub fn for_each_use_mut(&mut self, mut f: impl FnMut(&mut V)) {
        match self {
            SsaTerm::Return(Some(v)) | SsaTerm::Throw(v) => f(v),
            SsaTerm::Return(None) | SsaTerm::Unreachable => {}
            SsaTerm::Jump { args, .. } => args.iter_mut().for_each(f),
            SsaTerm::Branch {
                cond,
                then_args,
                else_args,
                ..
            } => {
                f(cond);
                then_args.iter_mut().for_each(&mut f);
                else_args.iter_mut().for_each(f);
            }
        }
    }

    pub fn map_ids<W, C>(&self, value: impl Fn(V) -> W, block: impl Fn(B) -> C) -> SsaTerm<W, C> {
        match self {
            SsaTerm::Return(v) => SsaTerm::Return(v.map(&value)),
            SsaTerm::Throw(v) => SsaTerm::Throw(value(*v)),
            SsaTerm::Jump { target, args } => SsaTerm::Jump {
                target: block(*target),
                args: args.iter().map(|v| value(*v)).collect(),
            },
            SsaTerm::Branch {
                cond,
                then_blk,
                then_args,
                else_blk,
                else_args,
            } => SsaTerm::Branch {
                cond: value(*cond),
                then_blk: block(*then_blk),
                then_args: then_args.iter().map(|v| value(*v)).collect(),
                else_blk: block(*else_blk),
                else_args: else_args.iter().map(|v| value(*v)).collect(),
            },
            SsaTerm::Unreachable => SsaTerm::Unreachable,
        }
    }
}
