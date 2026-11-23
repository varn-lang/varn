use rustc_hash::FxHashMap;
use vn_tir::{BackendTy, DynReason};

use crate::ssa::ir::{Inst, InstKind, SsaFunc, Value};
use crate::ssa::uses::replace_uses_with_map;

pub fn run(func: &mut SsaFunc) -> bool {
    let mut int_const: FxHashMap<Value, i64> = FxHashMap::default();
    let mut float_const: FxHashMap<Value, f64> = FxHashMap::default();
    for block in &func.blocks {
        for inst in &block.insts {
            match (inst.dest, &inst.kind) {
                (Some(d), InstKind::ConstInt(i)) => {
                    int_const.insert(d, *i);
                }
                (Some(d), InstKind::ConstFloat(f)) => {
                    float_const.insert(d, *f);
                }
                _ => {}
            }
        }
    }

    let mut rewrites: FxHashMap<Value, Value> = FxHashMap::default();
    let mut to_const: Vec<(usize, usize, InstKind)> = Vec::new();

    for (b, block) in func.blocks.iter().enumerate() {
        for (i, inst) in block.insts.iter().enumerate() {
            match simplify(inst, &int_const, &float_const) {
                Some(Simplified::Use(v)) => {
                    if let Some(dest) = inst.dest
                        && func.value_ty(v) == func.value_ty(dest)
                    {
                        rewrites.insert(dest, v);
                    }
                }
                Some(Simplified::Const(kind)) => to_const.push((b, i, kind)),
                None => {}
            }
        }
    }

    let mut changed = !to_const.is_empty();
    for (b, i, kind) in to_const {
        if let Some(dest) = func.blocks[b].insts[i].dest
            && let Some(ty) = const_inst_ty(&kind)
        {
            func.values[dest.0 as usize].ty = ty;
        }
        func.blocks[b].insts[i].kind = kind;
    }
    changed |= replace_uses_with_map(func, &rewrites);
    changed
}

enum Simplified {
    Use(Value),

    Const(InstKind),
}

fn simplify(
    inst: &Inst,
    int_const: &FxHashMap<Value, i64>,
    float_const: &FxHashMap<Value, f64>,
) -> Option<Simplified> {
    let InstKind::Binary { op, lhs, rhs, ty } = &inst.kind else {
        return None;
    };
    let (l, r) = (*lhs, *rhs);

    let representation = ty.representation();
    match &representation {
        BackendTy::Literal(_) => None,
        BackendTy::Num(vn_core::NumKind::Int(_)) => {
            let li = int_const.get(&l).copied();
            let ri = int_const.get(&r).copied();
            match op {
                vn_core::BinOp::Add => match (li, ri) {
                    (_, Some(0)) => Some(Simplified::Use(l)),
                    (Some(0), _) => Some(Simplified::Use(r)),
                    _ => None,
                },
                vn_core::BinOp::Sub => {
                    if ri == Some(0) {
                        Some(Simplified::Use(l))
                    } else if l == r {
                        Some(Simplified::Const(InstKind::ConstInt(0)))
                    } else {
                        None
                    }
                }
                vn_core::BinOp::Mul => match (li, ri) {
                    (_, Some(1)) => Some(Simplified::Use(l)),
                    (Some(1), _) => Some(Simplified::Use(r)),

                    (_, Some(0)) | (Some(0), _) => Some(Simplified::Const(InstKind::ConstInt(0))),
                    _ => None,
                },

                vn_core::BinOp::Div
                | vn_core::BinOp::Mod
                | vn_core::BinOp::Pow
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
                | vn_core::BinOp::In => None,
            }
        }
        BackendTy::Num(vn_core::NumKind::Float(_)) => {
            let is_one = |v: Value| float_const.get(&v).copied() == Some(1.0);
            match op {
                vn_core::BinOp::Mul => {
                    if is_one(r) {
                        Some(Simplified::Use(l))
                    } else if is_one(l) {
                        Some(Simplified::Use(r))
                    } else {
                        None
                    }
                }
                vn_core::BinOp::Div => is_one(r).then_some(Simplified::Use(l)),
                vn_core::BinOp::Add
                | vn_core::BinOp::Sub
                | vn_core::BinOp::Mod
                | vn_core::BinOp::Pow
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
                | vn_core::BinOp::In => None,
            }
        }
        BackendTy::Bool
        | BackendTy::Str
        | BackendTy::Char
        | BackendTy::Bytes
        | BackendTy::Enum(_)
        | BackendTy::Fn(_)
        | BackendTy::Dynamic(_)
        | BackendTy::Decimal
        | BackendTy::BigInt
        | BackendTy::Void
        | BackendTy::Never
        | BackendTy::Array(_)
        | BackendTy::Map(..)
        | BackendTy::Set(_)
        | BackendTy::Tuple(_)
        | BackendTy::Class(_)
        | BackendTy::Nullable(_) => None,
    }
}

fn const_inst_ty(kind: &InstKind) -> Option<BackendTy> {
    match kind {
        InstKind::ConstInt(_) => Some(BackendTy::INT),
        InstKind::ConstFloat(_) => Some(BackendTy::FLOAT),
        InstKind::ConstBool(_) => Some(BackendTy::Bool),
        InstKind::ConstStr(_) => Some(BackendTy::Str),
        InstKind::ConstChar(_) => Some(BackendTy::Char),
        InstKind::ConstDecimal(_) | InstKind::ConstBigInt(_) => {
            Some(BackendTy::Dynamic(DynReason::Boxed))
        }
        InstKind::ConstNull => Some(BackendTy::Dynamic(DynReason::Boxed)),
        InstKind::Binary { .. }
        | InstKind::Unary { .. }
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
        | InstKind::ArrayGetIndex { .. }
        | InstKind::BytesGetIndex { .. }
        | InstKind::MapGetIndex { .. }
        | InstKind::SetProperty { .. }
        | InstKind::SetFixedField { .. }
        | InstKind::SetIndex { .. }
        | InstKind::ArraySetIndex { .. }
        | InstKind::BytesSetIndex { .. }
        | InstKind::MapSetIndex { .. }
        | InstKind::ArrayPush { .. }
        | InstKind::ObjectMerge { .. }
        | InstKind::MethodCall { .. }
        | InstKind::InvokeSlot { .. }
        | InstKind::IsNull { .. }
        | InstKind::TypeTest { .. }
        | InstKind::Cast { .. }
        | InstKind::Convert { .. }
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
        | InstKind::DefineStatic { .. }
        | InstKind::DefineMethod { .. }
        | InstKind::DefineAccessor { .. }
        | InstKind::MakeEnumVariant { .. }
        | InstKind::Try { .. }
        | InstKind::PopTry
        | InstKind::CatchParam { .. }
        | InstKind::CloseUpvalues { .. }
        | InstKind::Dispose { .. }
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
        | InstKind::BuildObjectSpread { .. } => None,
    }
}
