use crate::ssa::ir::{Block, BlockId, SsaFunc, Terminator};
use vn_types::ssa::SsaOp;

pub fn simplify_and_compact(func: &mut SsaFunc) -> Result<bool, crate::OptError> {
    let mut changed = false;

    changed |= fold_redundant_branches(func);

    changed |= jump_forwarding(func);

    changed |= merge_blocks(func);

    let discarded = compact_cfg(func)?;
    changed |= discarded;

    Ok(changed)
}

fn get_reachable(func: &SsaFunc) -> Vec<BlockId> {
    let mut visited = vec![false; func.blocks.len()];
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(func.entry);
    visited[func.entry.0 as usize] = true;

    let mut reachable = Vec::new();
    while let Some(curr) = queue.pop_front() {
        reachable.push(curr);
        let block = func.block(curr);
        for inst in &block.insts {
            if let SsaOp::Try { handler } = &inst.kind
                && !visited[handler.0 as usize]
            {
                visited[handler.0 as usize] = true;
                queue.push_back(*handler);
            }
        }
        for succ in succs(&block.term) {
            if !visited[succ.0 as usize] {
                visited[succ.0 as usize] = true;
                queue.push_back(succ);
            }
        }
    }
    reachable
}

fn succs(t: &Terminator) -> Vec<BlockId> {
    match t {
        Terminator::Return(_) | Terminator::Throw(_) | Terminator::Unreachable => Vec::new(),
        Terminator::Jump { target, .. } => vec![*target],
        Terminator::Branch {
            then_blk, else_blk, ..
        } => vec![*then_blk, *else_blk],
    }
}

fn fold_redundant_branches(func: &mut SsaFunc) -> bool {
    let mut changed = false;
    for b_idx in 0..func.blocks.len() {
        let b_id = BlockId(b_idx as u32);
        let term = func.block(b_id).term.clone();
        if let Terminator::Branch {
            cond: _,
            then_blk,
            then_args,
            else_blk,
            else_args,
        } = term
            && then_blk == else_blk
            && then_args == else_args
        {
            func.blocks[b_id.0 as usize].term = Terminator::Jump {
                target: then_blk,
                args: then_args,
            };

            changed = true;
        }
    }
    changed
}

fn jump_forwarding(func: &mut SsaFunc) -> bool {
    let mut changed = false;
    let n = func.blocks.len();
    for b_idx in 0..n {
        let b_id = BlockId(b_idx as u32);
        if func.block(b_id).params.is_empty()
            && func.block(b_id).insts.is_empty()
            && let Terminator::Jump { target: c_id, args } = func.block(b_id).term.clone()
            && c_id != b_id
        {
            let b_preds = func.block(b_id).preds.clone();
            if b_preds.is_empty() {
                continue;
            }

            let mut updates = Vec::new();
            for a_id in b_preds {
                let a_term = &func.blocks[a_id.0 as usize].term;
                match a_term {
                    Terminator::Jump { target, .. } if *target == b_id => {
                        updates.push((
                            a_id,
                            Terminator::Jump {
                                target: c_id,
                                args: args.clone(),
                            },
                        ));
                    }
                    Terminator::Branch {
                        cond,
                        then_blk,
                        then_args,
                        else_blk,
                        else_args,
                    } => {
                        if *then_blk == b_id || *else_blk == b_id {
                            let new_then = if *then_blk == b_id { c_id } else { *then_blk };
                            let new_then_args = if *then_blk == b_id {
                                args.clone()
                            } else {
                                then_args.clone()
                            };
                            let new_else = if *else_blk == b_id { c_id } else { *else_blk };
                            let new_else_args = if *else_blk == b_id {
                                args.clone()
                            } else {
                                else_args.clone()
                            };
                            updates.push((
                                a_id,
                                Terminator::Branch {
                                    cond: *cond,
                                    then_blk: new_then,
                                    then_args: new_then_args,
                                    else_blk: new_else,
                                    else_args: new_else_args,
                                },
                            ));
                        }
                    }
                    Terminator::Return(_)
                    | Terminator::Throw(_)
                    | Terminator::Jump { .. }
                    | Terminator::Unreachable => {}
                }
            }

            for (a_id, new_term) in &updates {
                func.blocks[a_id.0 as usize].term = new_term.clone();
                func.blocks[c_id.0 as usize].preds.push(*a_id);
                changed = true;
            }

            func.blocks[b_id.0 as usize]
                .preds
                .retain(|p| !updates.iter().any(|(a_id, _)| a_id == p));

            if func.blocks[b_id.0 as usize].preds.is_empty() {
                let c_preds = &mut func.blocks[c_id.0 as usize].preds;
                if let Some(pos) = c_preds.iter().position(|p| *p == b_id) {
                    c_preds.remove(pos);
                }
            }
        }
    }
    changed
}

fn merge_blocks(func: &mut SsaFunc) -> bool {
    let mut changed = false;
    let n = func.blocks.len();
    for a_idx in 0..n {
        let a_id = BlockId(a_idx as u32);
        let term = func.block(a_id).term.clone();
        if let Terminator::Jump { target: b_id, args } = term
            && b_id != a_id
        {
            let b_preds = &func.block(b_id).preds;
            if b_preds == &[a_id] {
                let b_params = func.block(b_id).params.clone();
                for (&param, &arg) in b_params.iter().zip(args.iter()) {
                    func.replace_all_uses(param, arg);
                }

                let b_insts = std::mem::take(&mut func.blocks[b_id.0 as usize].insts);
                let b_term = std::mem::replace(
                    &mut func.blocks[b_id.0 as usize].term,
                    Terminator::Unreachable,
                );
                let (b_term_line, b_term_scope) = {
                    let b_block = &func.blocks[b_id.0 as usize];
                    (b_block.term_line, b_block.term_scope)
                };

                let handlers: Vec<BlockId> = b_insts
                    .iter()
                    .filter_map(|i| match &i.kind {
                        SsaOp::Try { handler } => Some(*handler),
                        SsaOp::ConstInt(_)
                        | SsaOp::ConstFloat(_)
                        | SsaOp::ConstBool(_)
                        | SsaOp::ConstStr(_)
                        | SsaOp::ConstChar(_)
                        | SsaOp::ConstDecimal(_)
                        | SsaOp::ConstBigInt(_)
                        | SsaOp::ConstNull
                        | SsaOp::Binary { .. }
                        | SsaOp::Unary { .. }
                        | SsaOp::KindOf { .. }
                        | SsaOp::LoadGlobal(_)
                        | SsaOp::LoadGlobalIdx(_)
                        | SsaOp::LoadNativeGlobalIdx(_)
                        | SsaOp::LoadUpvalue(_)
                        | SsaOp::StoreGlobal { .. }
                        | SsaOp::StoreGlobalIdx { .. }
                        | SsaOp::StoreUpvalue { .. }
                        | SsaOp::Call { .. }
                        | SsaOp::CallDirect { .. }
                        | SsaOp::AllocInstance { .. }
                        | SsaOp::SelfCall { .. }
                        | SsaOp::GetProperty { .. }
                        | SsaOp::GetSlotMethod { .. }
                        | SsaOp::GetFixedField { .. }
                        | SsaOp::GetIndex { .. }
                        | SsaOp::ArrayGetIndex { .. }
                        | SsaOp::BytesGetIndex { .. }
                        | SsaOp::SetProperty { .. }
                        | SsaOp::SetFixedField { .. }
                        | SsaOp::SetIndex { .. }
                        | SsaOp::ArraySetIndex { .. }
                        | SsaOp::BytesSetIndex { .. }
                        | SsaOp::ArrayPush { .. }
                        | SsaOp::ObjectMerge { .. }
                        | SsaOp::MethodCall { .. }
                        | SsaOp::InvokeSlot { .. }
                        | SsaOp::IsNull { .. }
                        | SsaOp::TypeTest { .. }
                        | SsaOp::Cast { .. }
                        | SsaOp::Convert { .. }
                        | SsaOp::BuildArray { .. }
                        | SsaOp::BuildTuple { .. }
                        | SsaOp::BuildObject { .. }
                        | SsaOp::BuildMap { .. }
                        | SsaOp::ObjectRest { .. }
                        | SsaOp::ToString { .. }
                        | SsaOp::BuildStr { .. }
                        | SsaOp::MakeClosure { .. }
                        | SsaOp::LoadCaptured { .. }
                        | SsaOp::StoreCaptured { .. }
                        | SsaOp::MakeClass { .. }
                        | SsaOp::DeclareLayout { .. }
                        | SsaOp::FreezeClass { .. }
                        | SsaOp::DefineMember { .. }
                        | SsaOp::MakeEnumVariant { .. }
                        | SsaOp::PopTry
                        | SsaOp::CatchParam { .. }
                        | SsaOp::CloseUpvalues { .. }
                        | SsaOp::LoadModule { .. }
                        | SsaOp::StoreModuleSlot { .. }
                        | SsaOp::Await { .. }
                        | SsaOp::Spawn { .. }
                        | SsaOp::Yield { .. }
                        | SsaOp::IntrinsicCall { .. }
                        | SsaOp::CallNativeOp { .. }
                        | SsaOp::AssertNotNull { .. }
                        | SsaOp::GetPropertyMaybe { .. }
                        | SsaOp::ModuleSlot { .. }
                        | SsaOp::GetEnumTag { .. }
                        | SsaOp::IsArray { .. }
                        | SsaOp::StrLength { .. }
                        | SsaOp::ArrayLength { .. }
                        | SsaOp::BytesLength { .. }
                        | SsaOp::This
                        | SsaOp::Range { .. }
                        | SsaOp::ObjectKeys { .. }
                        | SsaOp::GetSuper { .. }
                        | SsaOp::GetSuperSlot { .. }
                        | SsaOp::SuperCall { .. }
                        | SsaOp::SuperMethodCall { .. }
                        | SsaOp::SuperInvokeSlot { .. }
                        | SsaOp::ExtensionCall { .. }
                        | SsaOp::CallSpread { .. }
                        | SsaOp::BuildArraySpread { .. }
                        | SsaOp::BuildObjectSpread { .. } => None,
                    })
                    .collect();
                func.blocks[a_id.0 as usize].insts.extend(b_insts);
                for h in handlers {
                    for p in &mut func.blocks[h.0 as usize].preds {
                        if *p == b_id {
                            *p = a_id;
                        }
                    }
                }

                let succs = succs(&b_term);
                for s in succs {
                    let preds = &mut func.blocks[s.0 as usize].preds;
                    for p in preds {
                        if *p == b_id {
                            *p = a_id;
                        }
                    }
                }

                func.blocks[b_id.0 as usize].preds.clear();

                func.blocks[a_id.0 as usize].term = b_term;
                func.blocks[a_id.0 as usize].term_line = b_term_line;
                func.blocks[a_id.0 as usize].term_scope = b_term_scope;
                changed = true;
            }
        }
    }
    changed
}

fn compact_cfg(func: &mut SsaFunc) -> Result<bool, crate::OptError> {
    let reachable = get_reachable(func);
    if reachable.len() == func.blocks.len() {
        return Ok(false);
    }

    let mut old_to_new = vec![None; func.blocks.len()];
    for (new_idx, &old_id) in reachable.iter().enumerate() {
        old_to_new[old_id.0 as usize] = Some(BlockId(new_idx as u32));
    }
    let remap = |id: BlockId| {
        old_to_new[id.0 as usize]
            .ok_or_else(|| crate::OptError::Internal(format!("cfg: unreachable target b{}", id.0)))
    };

    let mut new_blocks = Vec::with_capacity(reachable.len());
    for &old_id in &reachable {
        let mut block = std::mem::replace(
            &mut func.blocks[old_id.0 as usize],
            Block {
                params: Vec::new(),
                insts: Vec::new(),
                term: Terminator::Unreachable,
                term_line: 0,
                term_scope: 0,
                preds: Vec::new(),
            },
        );

        block.preds = block
            .preds
            .into_iter()
            .filter_map(|p| old_to_new[p.0 as usize])
            .collect();

        for inst in &mut block.insts {
            if let SsaOp::Try { handler } = &mut inst.kind {
                *handler = remap(*handler)?;
            }
        }
        match &mut block.term {
            Terminator::Jump { target, .. } => {
                *target = remap(*target)?;
            }
            Terminator::Branch {
                then_blk, else_blk, ..
            } => {
                *then_blk = remap(*then_blk)?;
                *else_blk = remap(*else_blk)?;
            }
            Terminator::Return(_) | Terminator::Throw(_) | Terminator::Unreachable => {}
        }

        new_blocks.push(block);
    }

    func.entry = BlockId(0);
    func.blocks = new_blocks;
    Ok(true)
}
