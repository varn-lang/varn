use crate::ssa::ir::{BlockId, SsaFunc, Terminator};
use vn_types::ssa::SsaOp;

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

fn hoistable(kind: &SsaOp, facts: LoopFacts) -> bool {
    match kind {
        SsaOp::GetFixedField { .. } | SsaOp::ArrayGetIndex { .. } | SsaOp::BytesGetIndex { .. } => {
            facts.memory_stable
        }

        SsaOp::Binary { op, .. } => {
            matches!(
                op,
                vn_types::ssa::SsaBinOp::IntAdd
                    | vn_types::ssa::SsaBinOp::IntSub
                    | vn_types::ssa::SsaBinOp::IntMul
                    | vn_types::ssa::SsaBinOp::IntEq
                    | vn_types::ssa::SsaBinOp::IntNe
                    | vn_types::ssa::SsaBinOp::IntLt
                    | vn_types::ssa::SsaBinOp::IntLe
                    | vn_types::ssa::SsaBinOp::IntGt
                    | vn_types::ssa::SsaBinOp::IntGe
                    | vn_types::ssa::SsaBinOp::FloatAdd
                    | vn_types::ssa::SsaBinOp::FloatSub
                    | vn_types::ssa::SsaBinOp::FloatMul
                    | vn_types::ssa::SsaBinOp::FloatEq
                    | vn_types::ssa::SsaBinOp::FloatNe
                    | vn_types::ssa::SsaBinOp::FloatLt
                    | vn_types::ssa::SsaBinOp::FloatLe
                    | vn_types::ssa::SsaBinOp::FloatGt
                    | vn_types::ssa::SsaBinOp::FloatGe
            )
        }
        SsaOp::Unary { op, .. } => {
            matches!(
                op,
                vn_types::ssa::SsaUnOp::NegInt
                    | vn_types::ssa::SsaUnOp::NegFloat
                    | vn_types::ssa::SsaUnOp::Not
            )
        }
        SsaOp::KindOf { .. } => true,
        SsaOp::LoadGlobal(_) | SsaOp::LoadGlobalIdx(_) | SsaOp::LoadNativeGlobalIdx(_) => {
            facts.globals_stable
        }

        SsaOp::IsNull { .. }
        | SsaOp::TypeTest { .. }
        | SsaOp::IsArray { .. }
        | SsaOp::GetEnumTag { .. }
        | SsaOp::StrLength { .. } => true,
        SsaOp::MakeClosure { upvalues, .. } => upvalues.is_empty(),
        SsaOp::ConstInt(_)
        | SsaOp::ConstFloat(_)
        | SsaOp::ConstBool(_)
        | SsaOp::ConstStr(_)
        | SsaOp::ConstChar(_)
        | SsaOp::ConstDecimal(_)
        | SsaOp::ConstBigInt(_)
        | SsaOp::ConstNull
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
        | SsaOp::GetIndex { .. }
        | SsaOp::SetProperty { .. }
        | SsaOp::SetFixedField { .. }
        | SsaOp::SetIndex { .. }
        | SsaOp::ArraySetIndex { .. }
        | SsaOp::BytesSetIndex { .. }
        | SsaOp::ArrayPush { .. }
        | SsaOp::ObjectMerge { .. }
        | SsaOp::MethodCall { .. }
        | SsaOp::InvokeSlot { .. }
        | SsaOp::Cast { .. }
        | SsaOp::Convert { .. }
        | SsaOp::BuildArray { .. }
        | SsaOp::BuildTuple { .. }
        | SsaOp::BuildObject { .. }
        | SsaOp::BuildMap { .. }
        | SsaOp::ObjectRest { .. }
        | SsaOp::ToString { .. }
        | SsaOp::BuildStr { .. }
        | SsaOp::LoadCaptured { .. }
        | SsaOp::StoreCaptured { .. }
        | SsaOp::MakeClass { .. }
        | SsaOp::DeclareLayout { .. }
        | SsaOp::FreezeClass { .. }
        | SsaOp::DefineMember { .. }
        | SsaOp::MakeEnumVariant { .. }
        | SsaOp::Try { .. }
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
        | SsaOp::BuildObjectSpread { .. } => false,
    }
}

fn is_transparent(kind: &SsaOp) -> bool {
    matches!(
        kind,
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
            | SsaOp::StoreUpvalue { .. }
            | SsaOp::GetFixedField { .. }
            | SsaOp::SetFixedField { .. }
            | SsaOp::ArrayGetIndex { .. }
            | SsaOp::ArraySetIndex { .. }
            | SsaOp::BytesSetIndex { .. }
            | SsaOp::IsNull { .. }
            | SsaOp::TypeTest { .. }
            | SsaOp::IsArray { .. }
            | SsaOp::StrLength { .. }
            | SsaOp::ArrayLength { .. }
            | SsaOp::BytesLength { .. }
            | SsaOp::GetEnumTag { .. }
            | SsaOp::MakeClosure { .. }
    )
}

fn operands(kind: &SsaOp) -> Vec<crate::ssa::ir::Value> {
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
