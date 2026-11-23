use crate::ssa::ir::{BlockId, InstKind, SsaFunc, Terminator};
use vn_tir::BackendTy;

pub fn run(func: &mut SsaFunc) -> bool {
    let n = func.blocks.len();
    if n < 2 {
        return false;
    }

    let Ok(dom) = crate::ssa::dom::DomTree::compute(func) else {
        return false;
    };
    let mut changed = false;

    let mut loops: rustc_hash::FxHashMap<BlockId, Vec<BlockId>> = rustc_hash::FxHashMap::default();
    for b in 0..n {
        for succ in successors(&func.blocks[b].term) {
            if dom.dominates(succ.0 as usize, b) {
                loops.entry(succ).or_default().push(BlockId(b as u32));
            }
        }
    }

    let mut headers: Vec<BlockId> = loops.keys().copied().collect();
    headers.sort_by_key(|h| h.0);
    for header in headers {
        let latches = &loops[&header];
        changed |= hoist_loop(func, latches, header);
    }

    changed
}

fn hoist_loop(func: &mut SsaFunc, latches: &[BlockId], header: BlockId) -> bool {
    let n = func.blocks.len();

    let mut in_loop = vec![false; n];
    in_loop[header.0 as usize] = true;
    let mut stack = latches.to_vec();
    while let Some(b) = stack.pop() {
        if in_loop[b.0 as usize] {
            continue;
        }
        in_loop[b.0 as usize] = true;
        for &p in &func.blocks[b.0 as usize].preds {
            stack.push(p);
        }
    }

    let entries: Vec<BlockId> = func.blocks[header.0 as usize]
        .preds
        .iter()
        .copied()
        .filter(|p| !in_loop[p.0 as usize])
        .collect();
    let [pre] = entries.as_slice() else {
        return false;
    };
    let pre = *pre;
    if !matches!(
        func.blocks[pre.0 as usize].term,
        Terminator::Jump { target, .. } if target == header
    ) {
        return false;
    }

    let mut def_in_loop = vec![false; func.values.len()];
    let mut facts = LoopFacts {
        globals_stable: true,
        memory_stable: true,
    };
    for (b, block) in func.blocks.iter().enumerate() {
        if !in_loop[b] {
            continue;
        }
        for &p in &block.params {
            def_in_loop[p.0 as usize] = true;
        }
        for inst in &block.insts {
            if let Some(d) = inst.dest {
                def_in_loop[d.0 as usize] = true;
            }
            if !is_transparent(&inst.kind) {
                facts.globals_stable = false;
            }
            if !super::dce::is_pure(&inst.kind) {
                facts.memory_stable = false;
            }
        }
    }

    let mut changed = false;
    loop {
        let mut moved_any = false;
        for (b, &is_in) in in_loop.iter().enumerate().take(n) {
            if !is_in || b == pre.0 as usize {
                continue;
            }
            let mut i = 0;
            while i < func.blocks[b].insts.len() {
                let inst = &func.blocks[b].insts[i];
                let movable = hoistable(&inst.kind, facts)
                    && operands(&inst.kind)
                        .iter()
                        .all(|v| !def_in_loop[v.0 as usize]);
                if movable {
                    let inst = func.blocks[b].insts.remove(i);
                    if let Some(d) = inst.dest {
                        def_in_loop[d.0 as usize] = false;
                    }
                    func.blocks[pre.0 as usize].insts.push(inst);
                    moved_any = true;
                    changed = true;
                } else {
                    i += 1;
                }
            }
        }
        if !moved_any {
            break;
        }
    }
    changed
}

#[derive(Debug, Clone, Copy)]
struct LoopFacts {
    globals_stable: bool,

    memory_stable: bool,
}

fn hoistable(kind: &InstKind, facts: LoopFacts) -> bool {
    match kind {
        InstKind::GetFixedField { .. }
        | InstKind::ArrayGetIndex { .. }
        | InstKind::BytesGetIndex { .. } => facts.memory_stable,

        InstKind::Binary { op, ty, .. } => {
            let typed = matches!(ty, BackendTy::Num(_));
            let safe_op = matches!(
                op,
                vn_core::BinOp::Add
                    | vn_core::BinOp::Sub
                    | vn_core::BinOp::Mul
                    | vn_core::BinOp::Eq
                    | vn_core::BinOp::Ne
                    | vn_core::BinOp::Lt
                    | vn_core::BinOp::Le
                    | vn_core::BinOp::Gt
                    | vn_core::BinOp::Ge
            );
            typed && safe_op
        }
        InstKind::Unary { op, ty, .. } => {
            matches!(ty, BackendTy::Num(_) | BackendTy::Bool)
                && matches!(op, vn_core::UnOp::Neg | vn_core::UnOp::Not)
        }
        InstKind::KindOf { .. } => true,
        InstKind::LoadGlobal(_) | InstKind::LoadGlobalIdx(_) | InstKind::LoadNativeGlobalIdx(_) => {
            facts.globals_stable
        }

        InstKind::IsNull { .. }
        | InstKind::TypeTest { .. }
        | InstKind::IsArray { .. }
        | InstKind::GetEnumTag { .. }
        | InstKind::StrLength { .. } => true,
        InstKind::MakeClosure { upvalues_src, .. } => upvalues_src.is_empty(),
        InstKind::ConstInt(_)
        | InstKind::ConstFloat(_)
        | InstKind::ConstBool(_)
        | InstKind::ConstStr(_)
        | InstKind::ConstChar(_)
        | InstKind::ConstDecimal(_)
        | InstKind::ConstBigInt(_)
        | InstKind::ConstNull
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
        | InstKind::GetIndex { .. }
        | InstKind::SetProperty { .. }
        | InstKind::SetFixedField { .. }
        | InstKind::SetIndex { .. }
        | InstKind::ArraySetIndex { .. }
        | InstKind::BytesSetIndex { .. }
        | InstKind::ArrayPush { .. }
        | InstKind::ObjectMerge { .. }
        | InstKind::MethodCall { .. }
        | InstKind::InvokeSlot { .. }
        | InstKind::Cast { .. }
        | InstKind::Convert { .. }
        | InstKind::BuildArray { .. }
        | InstKind::BuildTuple { .. }
        | InstKind::BuildObject { .. }
        | InstKind::BuildMap { .. }
        | InstKind::ObjectRest { .. }
        | InstKind::ToString { .. }
        | InstKind::BuildStr { .. }
        | InstKind::LoadCaptured { .. }
        | InstKind::StoreCaptured { .. }
        | InstKind::MakeClass { .. }
        | InstKind::DeclareLayout { .. }
        | InstKind::FreezeClass { .. }
        | InstKind::DefineMember { .. }
        | InstKind::MakeEnumVariant { .. }
        | InstKind::Try { .. }
        | InstKind::PopTry
        | InstKind::CatchParam { .. }
        | InstKind::CloseUpvalues { .. }
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
        | InstKind::BuildObjectSpread { .. } => false,
    }
}

fn is_transparent(kind: &InstKind) -> bool {
    matches!(
        kind,
        InstKind::ConstInt(_)
            | InstKind::ConstFloat(_)
            | InstKind::ConstBool(_)
            | InstKind::ConstStr(_)
            | InstKind::ConstChar(_)
            | InstKind::ConstDecimal(_)
            | InstKind::ConstBigInt(_)
            | InstKind::ConstNull
            | InstKind::Binary { .. }
            | InstKind::Unary { .. }
            | InstKind::KindOf { .. }
            | InstKind::LoadGlobal(_)
            | InstKind::LoadGlobalIdx(_)
            | InstKind::LoadNativeGlobalIdx(_)
            | InstKind::LoadUpvalue(_)
            | InstKind::StoreUpvalue { .. }
            | InstKind::GetFixedField { .. }
            | InstKind::SetFixedField { .. }
            | InstKind::ArrayGetIndex { .. }
            | InstKind::ArraySetIndex { .. }
            | InstKind::BytesSetIndex { .. }
            | InstKind::IsNull { .. }
            | InstKind::TypeTest { .. }
            | InstKind::IsArray { .. }
            | InstKind::StrLength { .. }
            | InstKind::ArrayLength { .. }
            | InstKind::BytesLength { .. }
            | InstKind::GetEnumTag { .. }
            | InstKind::MakeClosure { .. }
    )
}

fn operands(kind: &InstKind) -> Vec<crate::ssa::ir::Value> {
    crate::ssa::verify::inst_uses(kind)
}

fn successors(t: &Terminator) -> Vec<BlockId> {
    match t {
        Terminator::Jump { target, .. } => vec![*target],
        Terminator::Branch {
            then_blk, else_blk, ..
        } => vec![*then_blk, *else_blk],
        Terminator::Return(_) | Terminator::Throw(_) | Terminator::Unreachable => Vec::new(),
    }
}
