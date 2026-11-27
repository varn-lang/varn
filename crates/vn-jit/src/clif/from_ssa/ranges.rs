use super::branch_facts::{per_block_facts, refine};
use super::range_math::{
    add_range, header_check, is_one, mul_range, sub_range, Check, I64_HI, I64_LO,
};
use super::range_versioning::{version_plans, VersionPlan};
use rustc_hash::{FxHashMap, FxHashSet};
use vn_types::register_meta::SlotKind;
use vn_types::ssa::{BlockId, ValueId};
use vn_types::ssa::{SsaBinOp, SsaOp, SsaProto, SsaTerm};

#[derive(Clone, Copy, PartialEq)]
enum Edge {
    Jump,
    Then,
    Else,
}

fn def_map(ssa: &SsaProto) -> Vec<Option<&SsaOp>> {
    let mut def = vec![None; ssa.values.len()];
    for blk in &ssa.blocks {
        for inst in &blk.insts {
            if let Some(d) = inst.def() {
                def[d.0 as usize] = Some(&inst.op);
            }
        }
    }
    def
}

fn def_block(ssa: &SsaProto) -> Vec<Option<usize>> {
    let mut def = vec![None; ssa.values.len()];
    for (b, blk) in ssa.blocks.iter().enumerate() {
        for &p in &blk.params {
            def[p.0 as usize] = Some(b);
        }
        for inst in &blk.insts {
            if let Some(d) = inst.def() {
                def[d.0 as usize] = Some(b);
            }
        }
    }
    def
}

pub(super) fn loop_sets(
    ssa: &SsaProto,
    preds: &[Vec<usize>],
    rpo_pos: &[usize],
    reached: &[bool],
) -> (Vec<usize>, FxHashMap<usize, rustc_hash::FxHashSet<usize>>) {
    let def = def_map(ssa);
    let mut headers: Vec<usize> = Vec::new();
    for header in 0..ssa.blocks.len() {
        if !reached[header] {
            continue;
        }
        if super::cfg::loop_body(preds, rpo_pos, reached, header).is_empty() {
            continue;
        }
        let blk = &ssa.blocks[header];
        let SsaTerm::Branch {
            cond,
            then_blk,
            else_blk,
            ..
        } = &blk.term
        else {
            continue;
        };
        let body = super::cfg::loop_body(preds, rpo_pos, reached, header);
        if !body.contains(&(then_blk.0 as usize)) || body.contains(&(else_blk.0 as usize)) {
            continue;
        }
        if !blk.insts.iter().any(|i| i.def() == Some(*cond)) {
            continue;
        }
        if !matches!(
            def[cond.0 as usize],
            Some(SsaOp::Binary {
                op: SsaBinOp::IntLt | SsaBinOp::IntLe | SsaBinOp::IntGt | SsaBinOp::IntGe,
                ..
            })
        ) {
            continue;
        }
        headers.push(header);
    }
    let bodies: FxHashMap<usize, rustc_hash::FxHashSet<usize>> = headers
        .iter()
        .map(|&h| (h, super::cfg::loop_body(preds, rpo_pos, reached, h)))
        .collect();
    (headers, bodies)
}

pub(super) fn analyze(
    ssa: &SsaProto,
    preds: &[Vec<usize>],
    rpo_pos: &[usize],
    reached: &[bool],
) -> (FxHashSet<ValueId>, Vec<VersionPlan>) {
    let def = def_map(ssa);
    let dblk = def_block(ssa);
    let pred_edges: Vec<Vec<(BlockId, Vec<ValueId>, Edge)>> = (0..ssa.blocks.len())
        .map(|from| match &ssa.blocks[from].term {
            SsaTerm::Jump { target, args } => vec![(*target, args.clone(), Edge::Jump)],
            SsaTerm::Branch {
                then_blk,
                then_args,
                else_blk,
                else_args,
                ..
            } => vec![
                (*then_blk, then_args.clone(), Edge::Then),
                (*else_blk, else_args.clone(), Edge::Else),
            ],
            SsaTerm::Return(_) | SsaTerm::Throw(_) | SsaTerm::Unreachable => vec![],
        })
        .collect();

    let (headers, bodies) = loop_sets(ssa, preds, rpo_pos, reached);

    let mut ranges: FxHashMap<ValueId, (i128, i128)> = FxHashMap::default();
    for (v, op) in def.iter().enumerate() {
        if let Some(SsaOp::ConstInt(c)) = op
            && ssa.value_ty(ValueId(v as u32)) == SlotKind::Int
        {
            ranges.insert(ValueId(v as u32), (*c as i128, *c as i128));
        }
    }

    let consts = |v: ValueId| -> Option<i128> {
        match def[v.0 as usize] {
            Some(SsaOp::ConstInt(c)) if ssa.value_ty(v) == SlotKind::Int => Some(*c as i128),
            _ => None,
        }
    };
    let facts = per_block_facts(ssa, &def, preds, rpo_pos, reached, &consts);

    let mut converged = false;
    for _ in 0..64 {
        let mut changed = false;
        let mut checks: FxHashMap<(usize, ValueId), Check> = FxHashMap::default();
        for &h in &headers {
            for &p in &ssa.blocks[h].params {
                if ssa.value_ty(p) != SlotKind::Int {
                    continue;
                }
                if let Some(c) = header_check(ssa, &def, &ranges, h, p) {
                    checks.insert((h, p), c);
                }
            }
        }
        for (b, blk) in ssa.blocks.iter().enumerate() {
            if !reached[b] {
                continue;
            }
            for (pi, &p) in blk.params.iter().enumerate() {
                if ssa.value_ty(p) != SlotKind::Int {
                    continue;
                }
                let latched = bodies.contains_key(&b);
                let mut elo: Option<i128> = None;
                let mut ehi: Option<i128> = None;
                let mut ok = true;
                for pred in preds[b].iter().filter(|p| reached[**p]) {
                    let pred_in_body = bodies.get(&b).is_some_and(|bd| bd.contains(pred));
                    if latched && pred_in_body {
                        continue;
                    }
                    for (t, args, _) in pred_edges[*pred].iter() {
                        if t.0 as usize != b {
                            continue;
                        }
                        let Some(&a) = args.get(pi) else {
                            ok = false;
                            continue;
                        };
                        match ranges.get(&a) {
                            Some(&(l, h)) => {
                                elo = Some(elo.map_or(l, |x: i128| x.min(l)));
                                ehi = Some(ehi.map_or(h, |x: i128| x.max(h)));
                            }
                            None => {
                                ok = false;
                            }
                        }
                    }
                }
                if !ok || elo.is_none() {
                    continue;
                }
                let mut l = elo.unwrap();
                let mut h = ehi.unwrap();
                let entry = (l, h);
                if latched {
                    for pred in preds[b].iter().filter(|p| reached[**p]) {
                        let pred_in_body = bodies.get(&b).is_some_and(|bd| bd.contains(pred));
                        if !pred_in_body {
                            continue;
                        }
                        for (t, args, _) in pred_edges[*pred].iter() {
                            if t.0 as usize != b {
                                continue;
                            }
                            let Some(&a) = args.get(pi) else {
                                ok = false;
                                continue;
                            };
                            if a == p {
                                continue;
                            }
                            if let Some((cl, ch)) =
                                step_contrib(ssa, &def, &ranges, &checks, b, p, a, entry)
                            {
                                l = l.min(cl);
                                h = h.max(ch);
                                continue;
                            }
                            match ranges.get(&a) {
                                Some(&(al, ah)) => {
                                    l = l.min(al);
                                    h = h.max(ah);
                                }
                                None => {
                                    ok = false;
                                }
                            }
                        }
                    }
                    if !ok {
                        continue;
                    }
                }
                if l > h {
                    continue;
                }
                if ranges.get(&p) != Some(&(l, h)) {
                    ranges.insert(p, (l, h));
                    changed = true;
                }
            }
            for inst in &blk.insts {
                let (Some(d), SsaOp::Binary { op, lhs, rhs }) = (inst.def(), &inst.op) else {
                    continue;
                };
                if ssa.value_ty(d) != SlotKind::Int {
                    continue;
                }
                let (Some(a), Some(c)) = (
                    refine(&facts[b], *lhs, ranges.get(lhs).copied()),
                    refine(&facts[b], *rhs, ranges.get(rhs).copied()),
                ) else {
                    continue;
                };
                let r = match op {
                    SsaBinOp::IntAdd => add_range(a, c),
                    SsaBinOp::IntSub => sub_range(a, c),
                    SsaBinOp::IntMul => mul_range(a, c),
                    SsaBinOp::IntDiv
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
                    | SsaBinOp::Dyn(_) => None,
                };
                if let Some(r) = r
                    && ranges.get(&d) != Some(&r)
                {
                    ranges.insert(d, r);
                    changed = true;
                }
            }
        }
        if !changed {
            converged = true;
            break;
        }
    }

    let no_overflow: FxHashSet<ValueId> = if converged {
        ranges
            .iter()
            .filter(|&(_, &(l, h))| l >= I64_LO && h <= I64_HI)
            .filter(|(v, _)| {
                matches!(def[v.0 as usize], Some(SsaOp::Binary { op, .. }) if matches!(op, SsaBinOp::IntAdd | SsaBinOp::IntSub | SsaBinOp::IntMul))
            })
            .map(|(v, _)| *v)
            .collect()
    } else {
        FxHashSet::default()
    };

    let plans = if converged {
        version_plans(ssa, preds, reached, &def, &dblk, &bodies, &headers, &ranges)
    } else {
        Vec::new()
    };

    if super::super::trace() {
        eprintln!(
            "clif: ranges headers={} known={} no_overflow={} versions={}",
            headers.len(),
            ranges.len(),
            no_overflow.len(),
            plans.len()
        );
    }
    (no_overflow, plans)
}

fn step_contrib(
    _ssa: &SsaProto,
    def: &[Option<&SsaOp>],
    ranges: &FxHashMap<ValueId, (i128, i128)>,
    checks: &FxHashMap<(usize, ValueId), Check>,
    header: usize,
    param: ValueId,
    arg: ValueId,
    entry: (i128, i128),
) -> Option<(i128, i128)> {
    let c = checks.get(&(header, param))?;
    let (elo, ehi) = entry;
    let Some(SsaOp::Binary {
        op: step,
        lhs: sl,
        rhs: sr,
    }) = def[arg.0 as usize]
    else {
        return None;
    };
    let is_inc = matches!(step, SsaBinOp::IntAdd)
        && ((*sl == param && is_one(def, ranges, *sr))
            || (*sr == param && is_one(def, ranges, *sl)));
    let is_dec = matches!(step, SsaBinOp::IntSub) && *sl == param && is_one(def, ranges, *sr);
    if c.below && is_inc {
        let top = if c.le { c.bound + 1 } else { c.bound };
        Some((elo + 1, top))
    } else if !c.below && is_dec {
        let floor = if c.le { c.bound - 1 } else { c.bound };
        Some((floor, ehi - 1))
    } else {
        None
    }
}
