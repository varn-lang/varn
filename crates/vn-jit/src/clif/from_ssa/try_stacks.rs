use super::cfg::normal_succs;
use vn_types::ssa::SsaProto;

pub(super) fn block_exit(ssa: &SsaProto, entry: &[usize], b: usize) -> Option<Vec<usize>> {
    let mut stack = entry.to_vec();
    for inst in &ssa.blocks[b].insts {
        replay_try_inst(ssa, &mut stack, inst)?;
    }
    Some(stack)
}

pub(super) fn stack_before_try(
    ssa: &SsaProto,
    entry: &[usize],
    b: usize,
    try_idx: usize,
) -> Option<Vec<usize>> {
    let mut stack = entry.to_vec();
    for (i, inst) in ssa.blocks[b].insts.iter().enumerate() {
        if i == try_idx {
            return Some(stack);
        }
        replay_try_inst(ssa, &mut stack, inst)?;
    }
    None
}

fn replay_try_inst(
    ssa: &SsaProto,
    stack: &mut Vec<usize>,
    inst: &vn_types::ssa::SsaInst,
) -> Option<()> {
    if let vn_types::ssa::SsaOp::Try { .. } = &inst.op {
        let e = ssa.catch_at(inst.try_landing()?.ip as usize)?;
        stack.push(e.block.0 as usize);
    } else if matches!(&inst.op, vn_types::ssa::SsaOp::PopTry) {
        stack.pop()?;
    }
    Some(())
}

pub(super) fn try_entry_stacks(
    ssa: &SsaProto,
    rpo: &[usize],
    reached: &[bool],
) -> Option<Vec<Vec<usize>>> {
    let n = ssa.blocks.len();
    let mut entry: Vec<Option<Vec<usize>>> = vec![None; n];
    entry[ssa.entry.0 as usize] = Some(Vec::new());
    for _ in 0..n + 2 {
        let mut changed = false;
        for &b in rpo {
            if !reached[b] {
                continue;
            }
            let Some(cur) = entry[b].clone() else {
                continue;
            };
            for (ti, inst) in ssa.blocks[b].insts.iter().enumerate() {
                if let Some(landing) = inst.try_landing() {
                    let e = ssa.catch_at(landing.ip as usize)?;
                    let h = e.block.0 as usize;
                    if h >= n {
                        return None;
                    }
                    let outer = stack_before_try(ssa, &cur, b, ti)?;
                    match &entry[h] {
                        None => {
                            entry[h] = Some(outer);
                            changed = true;
                        }
                        Some(prev) if *prev != outer => return None,
                        _ => {}
                    }
                }
            }
            let exit = block_exit(ssa, &cur, b)?;
            for s in normal_succs(ssa, b) {
                if s >= n || !reached[s] {
                    continue;
                }
                match &entry[s] {
                    None => {
                        entry[s] = Some(exit.clone());
                        changed = true;
                    }
                    Some(prev) if *prev != exit => return None,
                    _ => {}
                }
            }
        }
        if !changed {
            let mut out = Vec::with_capacity(n);
            for (i, e) in entry.into_iter().enumerate() {
                if reached[i] {
                    out.push(e?);
                } else {
                    out.push(Vec::new());
                }
            }
            return Some(out);
        }
    }
    None
}
