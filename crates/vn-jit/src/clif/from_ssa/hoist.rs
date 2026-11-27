use std::collections::BTreeMap;

use rustc_hash::{FxHashMap, FxHashSet};
use vn_types::ssa::SsaProto;
use vn_types::ssa::ValueId;

pub(super) fn global_sources(ssa: &SsaProto) -> BTreeMap<ValueId, u32> {
    let mut global_src: BTreeMap<ValueId, u32> = Default::default();
    for blk in &ssa.blocks {
        for inst in &blk.insts {
            if let (Some(d), vn_types::ssa::SsaOp::LoadGlobalIdx(slot)) = (inst.def(), &inst.op) {
                global_src.insert(d, *slot);
            }
        }
    }
    global_src
}

pub(super) fn plan_hoists(
    ssa: &SsaProto,
    preds: &[Vec<usize>],
    reached: &[bool],
    loop_headers: &[usize],
    loop_bodies: &FxHashMap<usize, FxHashSet<usize>>,
    has_frame: bool,
) -> BTreeMap<usize, Vec<(usize, u32)>> {
    let mut hoist_at: BTreeMap<usize, Vec<(usize, u32)>> = Default::default();
    for &h in loop_headers {
        let body = &loop_bodies[&h];
        let outside: Vec<usize> = preds[h]
            .iter()
            .copied()
            .filter(|p| reached[*p] && !body.contains(p))
            .collect();
        if outside.len() != 1 || !has_frame {
            continue;
        }
        let mut stored: FxHashSet<u32> = FxHashSet::default();
        let mut loaded: Vec<u32> = Vec::new();
        let mut pure = true;
        for &b in body {
            if !reached[b] {
                continue;
            }
            for inst in &ssa.blocks[b].insts {
                if let vn_types::ssa::SsaOp::LoadGlobalIdx(slot) = &inst.op {
                    if !loaded.contains(slot) {
                        loaded.push(*slot);
                    }
                } else if let vn_types::ssa::SsaOp::StoreGlobalIdx { slot, .. } = &inst.op {
                    stored.insert(*slot);
                } else if !super::range_versioning::version_op_ok(ssa, &inst.op) {
                    pure = false;
                }
            }
        }
        if !pure {
            continue;
        }
        let slots: Vec<u32> = loaded.into_iter().filter(|s| !stored.contains(s)).collect();
        if !slots.is_empty() {
            hoist_at
                .entry(outside[0])
                .or_default()
                .extend(slots.into_iter().map(|s| (h, s)));
        }
    }
    hoist_at
}
