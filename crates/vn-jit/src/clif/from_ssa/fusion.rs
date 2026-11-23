use rustc_hash::{FxHashMap, FxHashSet};
use vn_types::ssa::{SsaBinOp, SsaOp, SsaProto, SsaTerm};

pub(super) fn fusable_compares(ssa: &SsaProto) -> FxHashSet<u32> {
    let mut uses: FxHashMap<u32, u32> = FxHashMap::default();
    for blk in &ssa.blocks {
        for inst in &blk.insts {
            inst.for_each_use(|v| *uses.entry(v).or_default() += 1);
        }
        blk.term.for_each_use(|v| *uses.entry(v).or_default() += 1);
    }
    ssa.blocks
        .iter()
        .filter_map(|blk| {
            let last = blk.insts.last()?;
            let d = last.dest?;
            let is_str_compare = matches!(
                last.op,
                SsaOp::Binary {
                    op: SsaBinOp::StrEq | SsaBinOp::StrNe,
                    ..
                }
            );
            let branches_on_it = matches!(&blk.term, SsaTerm::Branch { cond, .. } if *cond == d);
            (is_str_compare && branches_on_it && uses.get(&d).copied() == Some(1)).then_some(d)
        })
        .collect()
}
