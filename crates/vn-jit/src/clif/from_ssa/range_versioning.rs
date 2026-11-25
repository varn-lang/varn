use super::range_math::{add_range, apply_cap, header_check, mul_range, sub_range, Check, I64_HI};
use rustc_hash::{FxHashMap, FxHashSet};
use vn_types::register_meta::SlotKind;
use vn_types::ssa::ValueId;
use vn_types::ssa::{SsaBinOp, SsaOp, SsaProto, SsaTerm};

pub(super) struct VersionPlan {
    pub header: usize,
    pub preheader: usize,
    pub guards: Vec<(ValueId, i128)>,
    pub elide: FxHashSet<ValueId>,
}

pub(super) fn version_op_ok(ssa: &SsaProto, op: &SsaOp) -> bool {
    if super::view_invalidation::keeps_views(ssa, op) {
        return true;
    }
    matches!(
        op,
        SsaOp::SetFixedField { .. } | SsaOp::LoadGlobalIdx(_) | SsaOp::StoreGlobalIdx { .. }
    )
}

pub(super) fn version_plans(
    ssa: &SsaProto,
    preds: &[Vec<usize>],
    reached: &[bool],
    def: &[Option<&SsaOp>],
    dblk: &[Option<usize>],
    bodies: &FxHashMap<usize, rustc_hash::FxHashSet<usize>>,
    headers: &[usize],
    ranges: &FxHashMap<ValueId, (i128, i128)>,
) -> Vec<VersionPlan> {
    let _ = def;
    let mut narrowed: FxHashMap<ValueId, (i128, i128)> = ranges.clone();
    let mut checks: FxHashMap<(usize, ValueId), Check> = FxHashMap::default();
    let mut nconverged = false;
    for _ in 0..64 {
        let mut changed = false;
        checks.clear();
        for &h in headers {
            for &p in &ssa.blocks[h].params {
                if ssa.value_ty(p) != SlotKind::Int {
                    continue;
                }
                if let Some(c) = header_check(ssa, def, &narrowed, h, p) {
                    checks.insert((h, p), c);
                }
            }
        }
        for &h in headers {
            let body = &bodies[&h];
            for &b in body {
                if !reached[b] || b == h {
                    continue;
                }
                let blk = &ssa.blocks[b];
                for inst in &blk.insts {
                    let (Some(d), SsaOp::Binary { op, lhs, rhs }) = (inst.dest, &inst.op) else {
                        continue;
                    };
                    if ssa.value_ty(d) != SlotKind::Int {
                        continue;
                    }
                    if !matches!(op, SsaBinOp::IntAdd | SsaBinOp::IntSub | SsaBinOp::IntMul) {
                        continue;
                    }
                    let read = |v: ValueId| -> Option<(i128, i128)> {
                        let mut r = *narrowed.get(&v)?;
                        for ((hh, kk), c) in checks.iter() {
                            if *kk == v && bodies[hh].contains(&b) && b != *hh {
                                r = apply_cap(r, c)?;
                            }
                        }
                        Some(r)
                    };
                    let (Some(a), Some(c)) = (read(*lhs), read(*rhs)) else {
                        continue;
                    };
                    let r = match op {
                        SsaBinOp::IntAdd => add_range(a, c),
                        SsaBinOp::IntSub => sub_range(a, c),
                        SsaBinOp::IntMul
                        | SsaBinOp::IntDiv
                        | SsaBinOp::IntMod
                        | SsaBinOp::IntPow
                        | SsaBinOp::IntEq
                        | SsaBinOp::IntNe
                        | SsaBinOp::IntLt
                        | SsaBinOp::IntLe
                        | SsaBinOp::IntGt
                        | SsaBinOp::IntGe
                        | SsaBinOp::IntAnd
                        | SsaBinOp::IntOr
                        | SsaBinOp::IntXor
                        | SsaBinOp::IntShl
                        | SsaBinOp::IntShr
                        | SsaBinOp::IntUshr
                        | SsaBinOp::FloatAdd
                        | SsaBinOp::FloatSub
                        | SsaBinOp::FloatMul
                        | SsaBinOp::FloatDiv
                        | SsaBinOp::FloatMod
                        | SsaBinOp::FloatPow
                        | SsaBinOp::FloatEq
                        | SsaBinOp::FloatNe
                        | SsaBinOp::FloatLt
                        | SsaBinOp::FloatLe
                        | SsaBinOp::FloatGt
                        | SsaBinOp::FloatGe
                        | SsaBinOp::StrConcat
                        | SsaBinOp::StrEq
                        | SsaBinOp::StrNe
                        | SsaBinOp::BitsEq
                        | SsaBinOp::BitsNe
                        | SsaBinOp::Dyn(_) => mul_range(a, c),
                    };
                    if let Some(r) = r
                        && narrowed.get(&d) != Some(&r)
                    {
                        if r.0 > r.1 {
                            return Vec::new();
                        }
                        narrowed.insert(d, r);
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            nconverged = true;
            break;
        }
    }
    if !nconverged {
        return Vec::new();
    }

    let use_range = |v: ValueId, b: usize| -> Option<(i128, i128)> {
        let mut r = *narrowed.get(&v)?;
        for ((hh, kk), c) in checks.iter() {
            if *kk == v && bodies[hh].contains(&b) && b != *hh {
                r = apply_cap(r, c)?;
            }
        }
        Some(r)
    };

    let mut plans = Vec::new();
    for &h in headers {
        if headers
            .iter()
            .any(|&h2| h2 != h && bodies[&h].contains(&h2))
        {
            continue;
        }
        let body = &bodies[&h];
        let mut ok = true;
        let mut elide = FxHashSet::default();
        let mut arrays: FxHashMap<ValueId, i128> = FxHashMap::default();
        for &b in body {
            if !reached[b] {
                continue;
            }
            let blk = &ssa.blocks[b];
            match &blk.term {
                SsaTerm::Jump { .. } | SsaTerm::Branch { .. } => {}
                SsaTerm::Return(_) | SsaTerm::Throw(_) | SsaTerm::Unreachable => {
                    ok = false;
                    break;
                }
            }
            for inst in &blk.insts {
                if !version_op_ok(ssa, &inst.op) {
                    ok = false;
                    break;
                }
                if matches!(inst.op, SsaOp::StoreGlobalIdx { .. }) {
                    ok = false;
                    break;
                }
                if let SsaOp::ArrayGetIndex { object, index }
                | SsaOp::BytesGetIndex { object, index }
                | SsaOp::ArraySetIndex { object, index, .. }
                | SsaOp::BytesSetIndex { object, index, .. } = &inst.op
                {
                    if ssa.value_ty(*index) != SlotKind::Int {
                        ok = false;
                        break;
                    }
                    if b == h {
                        ok = false;
                        break;
                    }
                    let Some((lo, hi)) = use_range(*index, b) else {
                        ok = false;
                        break;
                    };
                    if lo < 0 || hi + 1 > I64_HI {
                        ok = false;
                        break;
                    }
                    if dblk[object.0 as usize].is_none_or(|db| body.contains(&db)) {
                        ok = false;
                        break;
                    }
                    elide.insert(*index);
                    arrays
                        .entry(*object)
                        .and_modify(|m| *m = (*m).max(hi + 1))
                        .or_insert(hi + 1);
                }
            }
            if !ok {
                break;
            }
        }
        if !ok || arrays.is_empty() {
            continue;
        }
        let outside: Vec<usize> = preds[h]
            .iter()
            .copied()
            .filter(|p| reached[*p] && !body.contains(p))
            .collect();
        if outside.len() != 1 {
            continue;
        }
        let pre = outside[0];
        if !matches!(
            ssa.blocks[pre].term,
            SsaTerm::Jump { target, .. } if target.0 as usize == h
        ) {
            continue;
        }
        plans.push(VersionPlan {
            header: h,
            preheader: pre,
            guards: arrays.into_iter().collect(),
            elide,
        });
    }
    plans
}
