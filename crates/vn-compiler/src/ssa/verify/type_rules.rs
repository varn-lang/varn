use super::structural::VerifyResult;
use crate::ssa::ir::{Inst, SsaFunc};
use vn_tir::{BackendTy, DynReason};

pub(super) fn check_inst_types(func: &SsaFunc, inst: &Inst) -> VerifyResult {
    use crate::ssa::ir::InstKind;
    match &inst.kind {
        InstKind::Binary { op, lhs, rhs, ty } => {
            if !ty.is_boxed() {
                let lty = func.value_ty(*lhs);
                let rty = func.value_ty(*rhs);
                if lty != *ty {
                    return Err(format!(
                        "typed binary {op:?}.{ty:?} expects lhs type {ty:?}, got v{}: {lty:?}",
                        lhs.0
                    ));
                }
                if rty != *ty {
                    return Err(format!(
                        "typed binary {op:?}.{ty:?} expects rhs type {ty:?}, got v{}: {rty:?}",
                        rhs.0
                    ));
                }
                if let Some(dest) = inst.dest {
                    let dty = func.value_ty(dest);
                    let expected_res = binary_result_ty(*op, *ty);
                    let is_str_concat = matches!(op, vn_core::BinOp::Add)
                        && (lty == BackendTy::Str || rty == BackendTy::Str)
                        && dty == BackendTy::Str;
                    if dty != expected_res && !is_str_concat {
                        return Err(format!(
                            "typed binary {op:?}.{ty:?} defines v{}: {dty:?}, expected {expected_res:?}",
                            dest.0
                        ));
                    }
                }
            }
        }
        InstKind::Unary { op, operand, ty } => {
            if matches!(op, vn_core::UnOp::Neg | vn_core::UnOp::BitNot) && !ty.is_boxed() {
                let oty = func.value_ty(*operand);
                if oty != *ty {
                    return Err(format!(
                        "typed unary {op:?}.{ty:?} expects operand type {ty:?}, got v{}: {oty:?}",
                        operand.0
                    ));
                }
            }
            if let Some(dest) = inst.dest {
                let dty = func.value_ty(dest);
                let expected = match op {
                    vn_core::UnOp::Not => BackendTy::Bool,
                    vn_core::UnOp::KindOf => BackendTy::Str,
                    vn_core::UnOp::Neg | vn_core::UnOp::BitNot => *ty,
                };
                if dty != expected && !expected.is_boxed() {
                    return Err(format!(
                        "typed unary {op:?}.{ty:?} defines v{}: {dty:?}, expected {expected:?}",
                        dest.0
                    ));
                }
            }
        }
        InstKind::KindOf { .. } => {
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
        InstKind::ConstInt(_) => {
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
        InstKind::ConstFloat(_) => {
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
        InstKind::ConstBool(_) => {
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
        InstKind::ConstStr(_) => {
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
        InstKind::ConstChar(_) => {
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
        InstKind::IsNull { .. } => {
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
        InstKind::TypeTest { .. } => {
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
        InstKind::Cast { ty, .. } => {
            if let Some(dest) = inst.dest {
                let dty = func.value_ty(dest);
                if dty != *ty {
                    return Err(format!(
                        "Cast defines v{}: {dty:?}, expected {ty:?}",
                        dest.0
                    ));
                }
            }
        }
        InstKind::Convert { operand, conv } => {
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
        InstKind::ArrayGetIndex { index, .. }
        | InstKind::ArraySetIndex { index, .. }
        | InstKind::BytesGetIndex { index, .. }
        | InstKind::BytesSetIndex { index, .. } => {
            let ity = func.value_ty(*index);
            if ity != BackendTy::INT && !ity.is_boxed() {
                return Err(format!(
                    "Array index expects Int or Dynamic, got v{}: {ity:?}",
                    index.0
                ));
            }
        }
        InstKind::ConstDecimal(_)
        | InstKind::ConstBigInt(_)
        | InstKind::ConstNull
        | InstKind::LoadGlobal(_)
        | InstKind::LoadGlobalIdx(_)
        | InstKind::LoadNativeGlobalIdx(_)
        | InstKind::LoadUpvalue(_)
        | InstKind::StoreGlobal { .. }
        | InstKind::StoreGlobalIdx { .. }
        | InstKind::StoreUpvalue { .. }
        | InstKind::Call { .. }
        | InstKind::CallDirect { .. }
        | InstKind::AllocInstance { .. }
        | InstKind::SelfCall { .. }
        | InstKind::GetProperty { .. }
        | InstKind::GetSlotMethod { .. }
        | InstKind::GetFixedField { .. }
        | InstKind::GetIndex { .. }
        | InstKind::SetProperty { .. }
        | InstKind::SetFixedField { .. }
        | InstKind::SetIndex { .. }
        | InstKind::ArrayPush { .. }
        | InstKind::ObjectMerge { .. }
        | InstKind::MethodCall { .. }
        | InstKind::InvokeSlot { .. }
        | InstKind::BuildArray { .. }
        | InstKind::BuildTuple { .. }
        | InstKind::BuildObject { .. }
        | InstKind::BuildMap { .. }
        | InstKind::ObjectRest { .. }
        | InstKind::ToString { .. }
        | InstKind::BuildStr { .. }
        | InstKind::MakeClosure { .. }
        | InstKind::LoadCaptured { .. }
        | InstKind::StoreCaptured { .. }
        | InstKind::MakeClass { .. }
        | InstKind::DeclareLayout { .. }
        | InstKind::FreezeClass { .. }
        | InstKind::DefineMember { .. }
        | InstKind::MakeEnumVariant { .. }
        | InstKind::Try { .. }
        | InstKind::PopTry
        | InstKind::CatchParam { .. }
        | InstKind::CloseUpvalues { .. }
        | InstKind::LoadModule { .. }
        | InstKind::StoreModuleSlot { .. }
        | InstKind::Await { .. }
        | InstKind::Spawn { .. }
        | InstKind::Yield { .. }
        | InstKind::IntrinsicCall { .. }
        | InstKind::CallNativeOp { .. }
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
        | InstKind::SuperCall { .. }
        | InstKind::SuperMethodCall { .. }
        | InstKind::SuperInvokeSlot { .. }
        | InstKind::ExtensionCall { .. }
        | InstKind::CallSpread { .. }
        | InstKind::BuildArraySpread { .. }
        | InstKind::BuildObjectSpread { .. } => {}
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

fn binary_result_ty(op: vn_core::BinOp, operand_ty: BackendTy) -> BackendTy {
    if op.produces_bool() {
        BackendTy::Bool
    } else {
        operand_ty
    }
}
