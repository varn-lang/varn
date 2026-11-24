use vn_types::ssa::SsaOp;

pub(crate) fn dest_droppable(kind: &SsaOp) -> bool {
    use SsaOp::*;
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

pub(crate) fn is_pure(kind: &SsaOp) -> bool {
    use SsaOp::*;
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
        | KindOf { .. }
        | ModuleSlot { .. }
        | This
        | CatchParam { .. } => true,

        GetFixedField { .. } | ArrayGetIndex { .. } | BytesGetIndex { .. } => true,

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

        Binary { op, .. } => {
            matches!(
                op,
                vn_types::ssa::SsaBinOp::IntEq
                    | vn_types::ssa::SsaBinOp::IntNe
                    | vn_types::ssa::SsaBinOp::IntLt
                    | vn_types::ssa::SsaBinOp::IntLe
                    | vn_types::ssa::SsaBinOp::IntGt
                    | vn_types::ssa::SsaBinOp::IntGe
                    | vn_types::ssa::SsaBinOp::IntAnd
                    | vn_types::ssa::SsaBinOp::IntOr
                    | vn_types::ssa::SsaBinOp::IntXor
                    | vn_types::ssa::SsaBinOp::FloatAdd
                    | vn_types::ssa::SsaBinOp::FloatSub
                    | vn_types::ssa::SsaBinOp::FloatMul
                    | vn_types::ssa::SsaBinOp::FloatDiv
                    | vn_types::ssa::SsaBinOp::FloatMod
                    | vn_types::ssa::SsaBinOp::FloatPow
                    | vn_types::ssa::SsaBinOp::FloatEq
                    | vn_types::ssa::SsaBinOp::FloatNe
                    | vn_types::ssa::SsaBinOp::FloatLt
                    | vn_types::ssa::SsaBinOp::FloatLe
                    | vn_types::ssa::SsaBinOp::FloatGt
                    | vn_types::ssa::SsaBinOp::FloatGe
                    | vn_types::ssa::SsaBinOp::StrConcat
                    | vn_types::ssa::SsaBinOp::StrEq
                    | vn_types::ssa::SsaBinOp::StrNe
                    | vn_types::ssa::SsaBinOp::BitsEq
                    | vn_types::ssa::SsaBinOp::BitsNe
            )
        }
        Unary { op, .. } => match op {
            vn_types::ssa::SsaUnOp::NegFloat
            | vn_types::ssa::SsaUnOp::Not
            | vn_types::ssa::SsaUnOp::BitNotInt => true,
            vn_types::ssa::SsaUnOp::NegInt | vn_types::ssa::SsaUnOp::Dyn(_) => false,
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
        | ObjectMerge { .. } => false,

        MakeClass { .. } | DeclareLayout { .. } | FreezeClass { .. } | DefineMember { .. } => false,

        Try { .. }
        | PopTry
        | CloseUpvalues { .. }
        | LoadModule { .. }
        | AssertNotNull { .. }
        | Await { .. }
        | Spawn { .. }
        | Yield { .. } => false,
    }
}
