use rustc_hash::FxHashMap;

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
            && let Some(ty) = kind.const_ty()
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
    let InstKind::Binary { op, lhs, rhs } = &inst.kind else {
        return None;
    };
    let (l, r) = (*lhs, *rhs);

    use vn_types::ssa::SsaBinOp::*;
    match op {
        IntAdd => {
            let li = int_const.get(&l).copied();
            let ri = int_const.get(&r).copied();
            match (li, ri) {
                (_, Some(0)) => Some(Simplified::Use(l)),
                (Some(0), _) => Some(Simplified::Use(r)),
                _ => None,
            }
        }
        IntSub => {
            let ri = int_const.get(&r).copied();
            if ri == Some(0) {
                Some(Simplified::Use(l))
            } else if l == r {
                Some(Simplified::Const(InstKind::ConstInt(0)))
            } else {
                None
            }
        }
        IntMul => {
            let li = int_const.get(&l).copied();
            let ri = int_const.get(&r).copied();
            match (li, ri) {
                (_, Some(1)) => Some(Simplified::Use(l)),
                (Some(1), _) => Some(Simplified::Use(r)),
                (_, Some(0)) | (Some(0), _) => Some(Simplified::Const(InstKind::ConstInt(0))),
                _ => None,
            }
        }
        FloatMul => {
            let is_one = |v: Value| float_const.get(&v).copied() == Some(1.0);
            if is_one(r) {
                Some(Simplified::Use(l))
            } else if is_one(l) {
                Some(Simplified::Use(r))
            } else {
                None
            }
        }
        FloatDiv => {
            let is_one = |v: Value| float_const.get(&v).copied() == Some(1.0);
            is_one(r).then_some(Simplified::Use(l))
        }
        IntDiv | IntMod | IntPow | IntEq | IntNe | IntLt | IntLe | IntGt | IntGe | IntAnd
        | IntOr | IntXor | IntShl | IntShr | IntUshr | FloatAdd | FloatSub | FloatMod
        | FloatPow | FloatEq | FloatNe | FloatLt | FloatLe | FloatGt | FloatGe | StrConcat
        | StrEq | StrNe | BitsEq | BitsNe | Dyn(_) => None,
    }
}
