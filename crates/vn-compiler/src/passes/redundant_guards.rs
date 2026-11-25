use crate::ssa::ir::{BlockId, SsaFunc, Terminator, Value};
use rustc_hash::{FxHashMap, FxHashSet};
use vn_types::ssa::SsaOp;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NullFact {
    Null,
    NonNull,
}

pub fn run(func: &mut SsaFunc) -> bool {
    let mut changed = false;

    let mut inherently_non_null: FxHashSet<Value> = FxHashSet::default();
    let mut null_literals: FxHashSet<Value> = FxHashSet::default();

    for block in &func.blocks {
        for inst in &block.insts {
            if let Some(dest) = inst.dest {
                if let SsaOp::ConstNull = &inst.kind {
                    null_literals.insert(dest);
                } else if matches!(
                    &inst.kind,
                    SsaOp::ConstInt(_)
                        | SsaOp::ConstFloat(_)
                        | SsaOp::ConstBool(_)
                        | SsaOp::ConstStr(_)
                        | SsaOp::ConstChar(_)
                        | SsaOp::ConstDecimal(_)
                        | SsaOp::ConstBigInt(_)
                        | SsaOp::BuildArray { .. }
                        | SsaOp::BuildTuple { .. }
                        | SsaOp::BuildObject { .. }
                        | SsaOp::BuildMap { .. }
                        | SsaOp::BuildStr { .. }
                        | SsaOp::MakeClosure { .. }
                        | SsaOp::MakeClass { .. }
                        | SsaOp::MakeEnumVariant { .. }
                        | SsaOp::This
                ) {
                    inherently_non_null.insert(dest);
                }
            }
        }
    }

    for block in &mut func.blocks {
        for inst in &mut block.insts {
            if let SsaOp::IsNull { operand } = &inst.kind {
                if inherently_non_null.contains(operand) {
                    inst.kind = SsaOp::ConstBool(false);
                    changed = true;
                } else if null_literals.contains(operand) {
                    inst.kind = SsaOp::ConstBool(true);
                    changed = true;
                }
            }
        }
    }

    let mut defs: FxHashMap<Value, SsaOp> = FxHashMap::default();
    for block in &func.blocks {
        for inst in &block.insts {
            if let Some(d) = inst.dest {
                defs.insert(d, inst.kind.clone());
            }
        }
    }

    let mut block_facts: FxHashMap<BlockId, FxHashMap<Value, NullFact>> = FxHashMap::default();

    for (b_idx, block) in func.blocks.iter().enumerate() {
        let b_id = BlockId(b_idx as u32);
        if let Terminator::Branch {
            cond,
            then_blk,
            else_blk,
            ..
        } = &block.term
        {
            let mut then_facts = FxHashMap::default();
            let mut else_facts = FxHashMap::default();

            if let Some(def) = defs.get(cond) {
                if let SsaOp::IsNull { operand } = def {
                    then_facts.insert(*operand, NullFact::Null);
                    else_facts.insert(*operand, NullFact::NonNull);
                } else if let SsaOp::Binary {
                    op: vn_types::ssa::SsaBinOp::Dyn(vn_core::BinOp::Eq),
                    lhs,
                    rhs,
                    ..
                } = def
                {
                    if null_literals.contains(rhs)
                        || matches!(defs.get(rhs), Some(SsaOp::ConstNull))
                    {
                        then_facts.insert(*lhs, NullFact::Null);
                        else_facts.insert(*lhs, NullFact::NonNull);
                    } else if null_literals.contains(lhs)
                        || matches!(defs.get(lhs), Some(SsaOp::ConstNull))
                    {
                        then_facts.insert(*rhs, NullFact::Null);
                        else_facts.insert(*rhs, NullFact::NonNull);
                    }
                } else if let SsaOp::Binary {
                    op: vn_types::ssa::SsaBinOp::Dyn(vn_core::BinOp::Ne),
                    lhs,
                    rhs,
                    ..
                } = def
                {
                    if null_literals.contains(rhs)
                        || matches!(defs.get(rhs), Some(SsaOp::ConstNull))
                    {
                        then_facts.insert(*lhs, NullFact::NonNull);
                        else_facts.insert(*lhs, NullFact::Null);
                    } else if null_literals.contains(lhs)
                        || matches!(defs.get(lhs), Some(SsaOp::ConstNull))
                    {
                        then_facts.insert(*rhs, NullFact::NonNull);
                        else_facts.insert(*rhs, NullFact::Null);
                    }
                }
            }

            if func.blocks[then_blk.0 as usize].preds == [b_id] {
                block_facts.entry(*then_blk).or_default().extend(then_facts);
            }
            if func.blocks[else_blk.0 as usize].preds == [b_id] {
                block_facts.entry(*else_blk).or_default().extend(else_facts);
            }
        }
    }

    for (b_id, facts) in &block_facts {
        let block = &mut func.blocks[b_id.0 as usize];
        for inst in &mut block.insts {
            if let SsaOp::IsNull { operand } = &inst.kind
                && let Some(fact) = facts.get(operand)
            {
                match fact {
                    NullFact::Null => {
                        inst.kind = SsaOp::ConstBool(true);
                        changed = true;
                    }
                    NullFact::NonNull => {
                        inst.kind = SsaOp::ConstBool(false);
                        changed = true;
                    }
                }
            }
        }
    }

    changed
}
