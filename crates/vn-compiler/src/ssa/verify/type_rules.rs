use super::structural::VerifyResult;
use crate::ssa::ir::{Inst, SsaFunc};
use vn_tir::{BackendTy, DynReason};

pub(super) fn check_inst_types(func: &SsaFunc, inst: &Inst) -> VerifyResult {
    use vn_types::ssa::SsaOp;
    match &inst.kind {
        SsaOp::Binary { op, lhs: _, rhs: _ } => {
            use vn_types::ssa::SsaBinOp as B;
            let expected: Option<fn(BackendTy) -> bool> = match op {
                B::IntAdd | B::IntSub | B::IntMul | B::IntDiv | B::IntMod | B::IntPow => {
                    Some(is_int)
                }
                B::IntEq | B::IntNe | B::IntLt | B::IntLe | B::IntGt | B::IntGe => Some(is_bool),
                B::IntAnd | B::IntOr | B::IntXor | B::IntShl | B::IntShr | B::IntUshr => {
                    Some(is_int)
                }
                B::FloatAdd
                | B::FloatSub
                | B::FloatMul
                | B::FloatDiv
                | B::FloatMod
                | B::FloatPow => Some(is_float),
                B::FloatEq | B::FloatNe | B::FloatLt | B::FloatLe | B::FloatGt | B::FloatGe => {
                    Some(is_bool)
                }
                B::StrConcat => Some(is_str),
                B::StrEq | B::StrNe | B::BitsEq | B::BitsNe => Some(is_bool),
                B::Dyn(_) => None,
            };
            if let (Some(dest), Some(want)) = (inst.dest, expected)
                && !func.value_ty(dest).is_boxed()
                && !want(func.value_ty(dest))
            {
                return Err(format!(
                    "typed binary {op:?} defines v{}: {:?}, expected {}",
                    dest.0,
                    func.value_ty(dest),
                    expect_label(*op)
                ));
            }
        }
        SsaOp::Unary { op, operand: _ } => {
            use vn_types::ssa::SsaUnOp as U;
            let expected: Option<fn(BackendTy) -> bool> = match op {
                U::NegInt | U::BitNotInt => Some(is_int),
                U::NegFloat => Some(is_float),
                U::Not => Some(is_bool),
                U::Dyn(_) => None,
            };
            if let (Some(dest), Some(want)) = (inst.dest, expected)
                && !func.value_ty(dest).is_boxed()
                && !want(func.value_ty(dest))
            {
                return Err(format!(
                    "typed unary {op:?} defines v{}: {:?}, expected {}",
                    dest.0,
                    func.value_ty(dest),
                    expect_label_un(*op)
                ));
            }
        }
        SsaOp::KindOf { .. } => {
            if let Some(dest) = inst.dest {
                let dty = func.value_ty(dest);
                if dty != BackendTy::Str && !BackendTy::Str.is_boxed() {
                    return Err(format!(
                        "typed kindof defines v{}: {dty:?}, expected Str",
                        dest.0
                    ));
                }
            }
        }
        SsaOp::ConstInt(_) => {
            if let Some(dest) = inst.dest {
                let dty = func.value_ty(dest);
                if !matches!(dty, BackendTy::Num(vn_core::NumKind::Int(_))) {
                    return Err(format!(
                        "ConstInt defines v{}: {dty:?}, expected an integer type",
                        dest.0
                    ));
                }
            }
        }
        SsaOp::ConstFloat(_) => {
            if let Some(dest) = inst.dest {
                let dty = func.value_ty(dest);
                if !matches!(dty, BackendTy::Num(vn_core::NumKind::Float(_))) {
                    return Err(format!(
                        "ConstFloat defines v{}: {dty:?}, expected a float type",
                        dest.0
                    ));
                }
            }
        }
        SsaOp::ConstBool(_) => {
            if let Some(dest) = inst.dest {
                let dty = func.value_ty(dest);
                if dty != BackendTy::Bool {
                    return Err(format!(
                        "ConstBool defines v{}: {dty:?}, expected Bool",
                        dest.0
                    ));
                }
            }
        }
        SsaOp::ConstStr(_) => {
            if let Some(dest) = inst.dest {
                let dty = func.value_ty(dest);
                if dty != BackendTy::Str {
                    return Err(format!(
                        "ConstStr defines v{}: {dty:?}, expected Str",
                        dest.0
                    ));
                }
            }
        }
        SsaOp::ConstChar(_) => {
            if let Some(dest) = inst.dest {
                let dty = func.value_ty(dest);
                if dty != BackendTy::Char {
                    return Err(format!(
                        "ConstChar defines v{}: {dty:?}, expected Char",
                        dest.0
                    ));
                }
            }
        }
        SsaOp::IsNull { .. } => {
            if let Some(dest) = inst.dest {
                let dty = func.value_ty(dest);
                if dty != BackendTy::Bool {
                    return Err(format!(
                        "IsNull defines v{}: {dty:?}, expected Bool",
                        dest.0
                    ));
                }
            }
        }
        SsaOp::TypeTest { .. } => {
            if let Some(dest) = inst.dest {
                let dty = func.value_ty(dest);
                if dty != BackendTy::Bool {
                    return Err(format!(
                        "TypeTest defines v{}: {dty:?}, expected Bool",
                        dest.0
                    ));
                }
            }
        }
        SsaOp::Cast { .. } => {}
        SsaOp::Convert { operand, conv } => {
            let expected = convert_result_ty(*conv);
            if let Some(dest) = inst.dest {
                let dty = func.value_ty(dest);
                if dty != expected {
                    return Err(format!(
                        "Convert {conv:?} defines v{}: {dty:?}, expected {expected:?}",
                        dest.0
                    ));
                }
            }
            let oty = func.value_ty(*operand);
            if let Some(want) = convert_operand_domain(*conv)
                && !domain_holds(want, oty)
            {
                return Err(format!(
                    "Convert {conv:?} reads v{}: {oty:?}, expected a {want:?} number",
                    operand.0
                ));
            }
        }
        SsaOp::ArrayGetIndex { index, .. }
        | SsaOp::ArraySetIndex { index, .. }
        | SsaOp::BytesGetIndex { index, .. }
        | SsaOp::BytesSetIndex { index, .. } => {
            let ity = func.value_ty(*index);
            if ity != BackendTy::INT && !ity.is_boxed() {
                return Err(format!(
                    "Array index expects Int or Dynamic, got v{}: {ity:?}",
                    index.0
                ));
            }
        }
        SsaOp::ConstDecimal(_)
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
        | SsaOp::SetProperty { .. }
        | SsaOp::SetFixedField { .. }
        | SsaOp::SetIndex { .. }
        | SsaOp::ArrayPush { .. }
        | SsaOp::ObjectMerge { .. }
        | SsaOp::MethodCall { .. }
        | SsaOp::InvokeSlot { .. }
        | SsaOp::BuildArray { .. }
        | SsaOp::BuildTuple { .. }
        | SsaOp::BuildObject { .. }
        | SsaOp::BuildMap { .. }
        | SsaOp::ObjectRest { .. }
        | SsaOp::ToString { .. }
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
        | SsaOp::PopTry
        | SsaOp::CatchParam { .. }
        | SsaOp::CloseUpvalues { .. }
        | SsaOp::LoadModule { .. }
        | SsaOp::StoreModuleSlot { .. }
        | SsaOp::Await { .. }
        | SsaOp::Spawn { .. }
        | SsaOp::Yield { .. }
        | SsaOp::IntrinsicCall { .. }
        | SsaOp::CallNativeOp { .. }
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
        | SsaOp::SuperCall { .. }
        | SsaOp::SuperMethodCall { .. }
        | SsaOp::SuperInvokeSlot { .. }
        | SsaOp::ExtensionCall { .. }
        | SsaOp::CallSpread { .. }
        | SsaOp::BuildArraySpread { .. }
        | SsaOp::BuildObjectSpread { .. } => {}
    }
    Ok(())
}

pub(crate) fn convert_result_ty(conv: vn_core::ValueConv) -> BackendTy {
    use vn_core::ValueConv::*;
    match conv {
        IntToFloat | DynToFloat | BigIntToFloat | DecimalToFloat => BackendTy::FLOAT,
        FloatToInt | BigIntToInt | DecimalToInt | DynToInt => BackendTy::INT,
        IntToBigInt | IntToDecimal | FloatToBigInt | FloatToDecimal | BigIntToDecimal
        | DecimalToBigInt => BackendTy::Dynamic(DynReason::Boxed),
        ExpectStr => BackendTy::Str,
        ExpectBool => BackendTy::Bool,
        ExpectChar => BackendTy::Char,
        ExpectStrOrNull | ExpectBoolOrNull | ExpectIntOrNull | ExpectFloatOrNull | ExpectRef => {
            BackendTy::Dynamic(DynReason::Boxed)
        }
        ExpectInt => BackendTy::INT,
        ExpectFloat => BackendTy::FLOAT,
        NarrowI8 | NarrowI16 | NarrowI32 | NarrowU8 | NarrowU16 | NarrowU32 | NarrowU64
        | NarrowF32 => conv.narrowed().map_or(BackendTy::INT, BackendTy::Num),
        U64ToBigInt => BackendTy::Dynamic(DynReason::Boxed),
        U64ToBits => BackendTy::INT,
        U64FromBits => BackendTy::Num(vn_core::NumKind::Int(vn_core::IntKind::U64)),
    }
}

fn domain_holds(domain: vn_core::NumericDomain, ty: BackendTy) -> bool {
    use vn_core::NumericDomain as D;
    match domain {
        D::Int => matches!(ty, BackendTy::Num(vn_core::NumKind::Int(_))),
        D::Float => matches!(ty, BackendTy::Num(vn_core::NumKind::Float(_))),
        D::BigInt | D::Decimal => false,
    }
}

fn convert_operand_domain(conv: vn_core::ValueConv) -> Option<vn_core::NumericDomain> {
    use vn_core::NumericDomain as D;
    use vn_core::ValueConv::*;
    match conv {
        IntToFloat | U64ToBigInt | U64ToBits | U64FromBits => Some(D::Int),
        FloatToInt | FloatToBigInt | FloatToDecimal => Some(D::Float),
        IntToBigInt | IntToDecimal | BigIntToInt | DecimalToInt | DynToInt | DynToFloat
        | BigIntToFloat | DecimalToFloat | BigIntToDecimal | DecimalToBigInt | ExpectStr
        | ExpectBool | ExpectStrOrNull | ExpectBoolOrNull | ExpectInt | ExpectFloat
        | ExpectIntOrNull | ExpectFloatOrNull | ExpectRef | ExpectChar | NarrowI8 | NarrowI16
        | NarrowI32 | NarrowU8 | NarrowU16 | NarrowU32 | NarrowU64 | NarrowF32 => None,
    }
}

fn is_int(ty: BackendTy) -> bool {
    matches!(ty, BackendTy::Num(vn_core::NumKind::Int(_)))
}

fn is_float(ty: BackendTy) -> bool {
    matches!(ty, BackendTy::Num(vn_core::NumKind::Float(_)))
}

fn is_bool(ty: BackendTy) -> bool {
    ty == BackendTy::Bool
}

fn is_str(ty: BackendTy) -> bool {
    ty == BackendTy::Str
}

fn expect_label(op: vn_types::ssa::SsaBinOp) -> &'static str {
    use vn_types::ssa::SsaBinOp as B;
    match op {
        B::IntAdd | B::IntSub | B::IntMul | B::IntDiv | B::IntMod | B::IntPow => "an integer type",
        B::IntEq | B::IntNe | B::IntLt | B::IntLe | B::IntGt | B::IntGe => "Bool",
        B::IntAnd | B::IntOr | B::IntXor | B::IntShl | B::IntShr | B::IntUshr => "an integer type",
        B::FloatAdd | B::FloatSub | B::FloatMul | B::FloatDiv | B::FloatMod | B::FloatPow => {
            "a float type"
        }
        B::FloatEq | B::FloatNe | B::FloatLt | B::FloatLe | B::FloatGt | B::FloatGe => "Bool",
        B::StrConcat => "Str",
        B::StrEq | B::StrNe | B::BitsEq | B::BitsNe => "Bool",
        B::Dyn(_) => "Dynamic",
    }
}

fn expect_label_un(op: vn_types::ssa::SsaUnOp) -> &'static str {
    use vn_types::ssa::SsaUnOp as U;
    match op {
        U::NegInt | U::BitNotInt => "an integer type",
        U::NegFloat => "a float type",
        U::Not => "Bool",
        U::Dyn(_) => "Dynamic",
    }
}
