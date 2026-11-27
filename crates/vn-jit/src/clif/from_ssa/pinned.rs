use std::collections::{BTreeMap, BTreeSet};

use vn_types::ssa::{SsaOp, SsaProto, ValueId};

pub(super) struct Pins {
    pub prefill: BTreeMap<usize, Vec<ValueId>>,
    pub bodies: BTreeMap<ValueId, BTreeSet<usize>>,
}

pub(super) fn compute(
    ssa: &SsaProto,
    preds: &[Vec<usize>],
    rpo_pos: &[usize],
    reached: &[bool],
) -> Pins {
    let mut def_block: BTreeMap<ValueId, usize> = BTreeMap::new();
    for (b, blk) in ssa.blocks.iter().enumerate() {
        if !reached[b] {
            continue;
        }
        for inst in &blk.insts {
            if let Some(d) = inst.def() {
                def_block.insert(d, b);
            }
        }
    }
    let mut readers: BTreeMap<ValueId, Vec<usize>> = BTreeMap::new();
    for (b, blk) in ssa.blocks.iter().enumerate() {
        if !reached[b] {
            continue;
        }
        for inst in &blk.insts {
            if let SsaOp::ArrayGetIndex { object, .. } = &inst.op {
                readers.entry(*object).or_default().push(b);
            }
        }
    }
    let mut headers: Vec<usize> = Vec::new();
    for h in 0..ssa.blocks.len() {
        if !reached[h] {
            continue;
        }
        let body = super::cfg::loop_body(preds, rpo_pos, reached, h);
        if !body.is_empty() {
            headers.push(h);
        }
    }
    let mut bodies: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for h in &headers {
        let body = super::cfg::loop_body(preds, rpo_pos, reached, *h);
        bodies.insert(*h, body.into_iter().collect());
    }
    let mut best: BTreeMap<ValueId, (usize, usize, usize)> = BTreeMap::new();
    for (object, uses) in &readers {
        for h in &headers {
            let body = &bodies[h];
            if !uses.iter().all(|u| body.contains(u)) {
                continue;
            }
            let mut ok = true;
            for b in body.iter() {
                let blk = &ssa.blocks[*b];
                if blk.params.contains(object) {
                    ok = false;
                    break;
                }
                for inst in &blk.insts {
                    if !super::view_invalidation::keeps_views(ssa, &inst.op) {
                        ok = false;
                        break;
                    }
                    if let SsaOp::ArraySetIndex { object: o, .. }
                    | SsaOp::BytesSetIndex { object: o, .. }
                    | SsaOp::ArrayPush { array: o, .. } = &inst.op
                    {
                        if o == object {
                            ok = false;
                            break;
                        }
                    }
                }
                if !ok {
                    break;
                }
            }
            if !ok {
                continue;
            }
            match def_block.get(object) {
                Some(db) if body.contains(db) => continue,
                None => continue,
                Some(_) => {}
            }
            let outside: Vec<usize> = preds[*h]
                .iter()
                .copied()
                .filter(|p| reached[*p] && !body.contains(p))
                .collect();
            if outside.len() != 1 {
                continue;
            }
            let pre = outside[0];
            let size = body.len();
            match best.get(object) {
                Some((_, s, _)) if *s >= size => {}
                Some(_) | None => {
                    best.insert(*object, (pre, size, *h));
                }
            }
        }
    }
    let mut prefill: BTreeMap<usize, Vec<ValueId>> = BTreeMap::new();
    let mut pinned_bodies: BTreeMap<ValueId, BTreeSet<usize>> = BTreeMap::new();
    for (object, (pre, _, header)) in best {
        prefill.entry(pre).or_default().push(object);
        pinned_bodies.insert(object, bodies[&header].clone());
    }
    for v in prefill.values_mut() {
        v.sort_unstable();
        v.dedup();
    }
    Pins {
        prefill,
        bodies: pinned_bodies,
    }
}
