use crate::ssa::ir::InstKind;
use vn_tir::BackendTy;

pub(crate) fn dest_droppable(kind: &InstKind) -> bool {
    use InstKind::*;
    matches!(
        kind,
        SelfCall { .. }
            | SuperCall { .. }
            | SuperMethodCall { .. }
            | ExtensionCall { .. }
            | IntrinsicCall { .. }
            | CallNativeOp { .. }
    )
}

pub(crate) fn is_pure(kind: &InstKind) -> bool {
    use InstKind::*;
    match kind {
        ConstInt(_)
        | ConstFloat(_)
        | ConstBool(_)
        | ConstStr(_)
        | ConstChar(_)
        | ConstDecimal(_)
        | ConstBigInt(_)
        | ConstNull
        | LoadGlobal(_)
        | LoadGlobalIdx(_)
        | LoadNativeGlobalIdx(_)
        | LoadUpvalue(_)
        | LoadCaptured { .. }
        | ModuleSlot { .. }
        | This
        | CatchParam { .. } => true,

        GetFixedField { .. } | ArrayGetIndex { .. } | BytesGetIndex { .. } | MapGetIndex { .. } => {
            true
        }

        IsNull { .. }
        | TypeTest { .. }
        | Cast { .. }
        | IsArray { .. }
        | GetEnumTag { .. }
        | ObjectKeys { .. } => true,

        StrLength { .. } | ArrayLength { .. } | BytesLength { .. } => true,
        Convert { conv, .. } => !conv.can_fault(),

        AllocInstance { .. }
        | BuildArray { .. }
        | BuildTuple { .. }
        | BuildObject { .. }
        | BuildMap { .. }
        | MakeClosure { .. }
        | MakeEnumVariant { .. }
        | Range { .. } => true,

        Binary { op, ty, .. } => {
            let typed = matches!(ty, BackendTy::Num(_) | BackendTy::Bool);
            let int_can_overflow = *ty == BackendTy::INT
                && matches!(
                    op,
                    vn_core::BinOp::Add
                        | vn_core::BinOp::Sub
                        | vn_core::BinOp::Mul
                        | vn_core::BinOp::Shl
                        | vn_core::BinOp::Shr
                        | vn_core::BinOp::Ushr
                );

            let never_traps = matches!(
                op,
                vn_core::BinOp::Add
                    | vn_core::BinOp::Sub
                    | vn_core::BinOp::Mul
                    | vn_core::BinOp::Eq
                    | vn_core::BinOp::Ne
                    | vn_core::BinOp::Lt
                    | vn_core::BinOp::Le
                    | vn_core::BinOp::Gt
                    | vn_core::BinOp::Ge
                    | vn_core::BinOp::BitAnd
                    | vn_core::BinOp::BitOr
                    | vn_core::BinOp::BitXor
                    | vn_core::BinOp::Shl
                    | vn_core::BinOp::Shr
                    | vn_core::BinOp::Ushr
            );
            typed && never_traps && !int_can_overflow
        }
        Unary { op, ty, .. } => match op {
            vn_core::UnOp::KindOf => true,
            vn_core::UnOp::Neg => matches!(ty, BackendTy::Num(vn_core::NumKind::Float(_))),
            vn_core::UnOp::Not | vn_core::UnOp::BitNot => {
                matches!(ty, BackendTy::Num(_) | BackendTy::Bool)
            }
        },

        GetProperty { .. }
        | GetSlotMethod { .. }
        | GetPropertyMaybe { .. }
        | GetIndex { .. }
        | GetSuper { .. }
        | GetSuperSlot { .. } => false,

        ToString { .. } | BuildStr { .. } => false,

        BuildArraySpread { .. }
        | BuildObjectSpread { .. }
        | CallSpread { .. }
        | ObjectRest { .. } => false,

        Call { .. }
        | CallDirect { .. }
        | SelfCall { .. }
        | MethodCall { .. }
        | InvokeSlot { .. }
        | SuperCall { .. }
        | SuperMethodCall { .. }
        | SuperInvokeSlot { .. }
        | ExtensionCall { .. }
        | IntrinsicCall { .. }
        | CallNativeOp { .. }
        | ArrayPush { .. } => false,

        StoreGlobal { .. }
        | StoreGlobalIdx { .. }
        | StoreUpvalue { .. }
        | StoreCaptured { .. }
        | StoreModuleSlot { .. }
        | SetProperty { .. }
        | SetFixedField { .. }
        | SetIndex { .. }
        | ArraySetIndex { .. }
        | BytesSetIndex { .. }
        | MapSetIndex { .. }
        | ObjectMerge { .. } => false,

        MakeClass { .. } | DeclareLayout { .. } | FreezeClass { .. } | DefineMember { .. } => false,

        Try { .. }
        | PopTry
        | CloseUpvalues { .. }
        | Dispose { .. }
        | LoadModule { .. }
        | AssertNotNull { .. }
        | Await { .. }
        | Spawn { .. }
        | Yield { .. } => false,
    }
}
