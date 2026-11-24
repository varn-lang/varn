use super::super::ir::{SsaFunc, SsaVar};
use super::slot_kind_of;
use crate::OptError;
use vn_types::ssa::SsaOp;

type Result<T> = std::result::Result<T, OptError>;

pub(super) struct Assignment {
    pub reg: Vec<u8>,
    pub scratch: u8,
    pub null_reg: u8,
    pub call_base: u8,
    pub register_count: u16,

    pub liveness: crate::ssa::liveness::Liveness,
}

pub(super) fn assign_registers(ssa: &SsaFunc, nparams: usize) -> Result<Assignment> {
    let mut reg = vec![0u8; ssa.values.len()];
    let mut assigned = vec![false; ssa.values.len()];

    let entry_params = &ssa.blocks[ssa.entry.0 as usize].params;
    if entry_params.len() != nparams {
        return Err(OptError::Unsupported("ssa-emit: entry params != arity"));
    }
    for (i, p) in entry_params.iter().enumerate() {
        reg[p.0 as usize] = (1 + i) as u8;
        assigned[p.0 as usize] = true;
    }

    let nvals = ssa.values.len();
    let order = super::emission_order(ssa);
    let lv = crate::ssa::liveness::Liveness::analyze_ordered(ssa, &order);
    let def = &lv.def;
    let end = &lv.end;

    let mut base = 1 + nparams as u32;
    for block in &ssa.blocks {
        for inst in &block.insts {
            if let SsaOp::LoadCaptured { var } | SsaOp::StoreCaptured { var, .. } = &inst.kind {
                base = base.max(var_reg(*var, nparams) as u32 + 1);
            }
        }
    }
    let mut order: Vec<usize> = (0..nvals)
        .filter(|&v| !assigned[v] && def[v] != u32::MAX)
        .collect();
    order.sort_by_key(|&v| def[v]);
    let mut max_call = 0u32;
    for block in &ssa.blocks {
        for inst in &block.insts {
            let t = window_size(&inst.kind);
            max_call = max_call.max(t);
        }
    }

    let kind_of_v = |v: usize| slot_kind_of(ssa.values[v].ty);

    let ceiling = 256u32.saturating_sub(2 + max_call);
    let mut next: u32 = base;

    let mut free: [Vec<u32>; POOLS] = Default::default();

    let mut active: Vec<(u32, u32, usize)> = Vec::new();
    for &v in &order {
        let d = def[v];
        let vk = pool_index(kind_of_v(v));
        let mut i = 0;
        while i < active.len() {
            if active[i].0 < d {
                let (_, r, rk) = active[i];
                free[rk].push(r);
                active.swap_remove(i);
            } else {
                i += 1;
            }
        }
        let r = if let Some(m) = free[vk].iter().copied().min() {
            free[vk].retain(|&x| x != m);
            m
        } else if next < ceiling {
            let r = next;
            next += 1;
            r
        } else {
            let borrowed = free
                .iter()
                .enumerate()
                .filter_map(|(k, p)| p.iter().copied().min().map(|m| (m, k)))
                .min();
            match borrowed {
                Some((m, k)) => {
                    free[k].retain(|&x| x != m);
                    m
                }
                None => {
                    let r = next;
                    next += 1;
                    r
                }
            }
        };
        if r > 255 {
            return Err(OptError::OutOfRegisters {
                function: String::new(),
                needed: r as usize + 1,
            });
        }
        reg[v] = r as u8;
        assigned[v] = true;
        active.push((end[v], r, vk));
    }

    let total = next + 2 + max_call;
    if total as usize > crate::MAX_REGISTERS {
        return Err(OptError::OutOfRegisters {
            function: String::new(),
            needed: total as usize,
        });
    }
    let scratch = next as u8;
    let null_reg = (next + 1) as u8;
    let call_base = (next + 2) as u8;
    let register_count = total.max(1) as u16;
    coalesce_window_args(ssa, &mut reg, call_base);
    Ok(Assignment {
        reg,
        scratch,
        null_reg,
        call_base,
        register_count,
        liveness: lv,
    })
}

fn window_size(kind: &SsaOp) -> u32 {
    match kind {
        SsaOp::Call { args, .. } => args.len() as u32 + 1,
        SsaOp::CallDirect { args, .. } => args.len() as u32 + 1,
        SsaOp::SelfCall { args } => args.len() as u32 + 1,
        SsaOp::MethodCall { args, .. } => args.len() as u32,
        SsaOp::InvokeSlot { args, .. } => args.len() as u32,

        SsaOp::BuildArray { elements, .. } => elements.len() as u32,
        SsaOp::BuildTuple { elements } => elements.len() as u32,
        SsaOp::BuildMap { pairs } => (pairs.len() * 2) as u32,

        SsaOp::BuildObject { pairs } => pairs.len() as u32,

        SsaOp::IntrinsicCall { args, .. } => args.len() as u32 + 1,
        SsaOp::CallNativeOp { args, .. } => args.len() as u32 + 1,

        SsaOp::SuperCall { args } => args.len() as u32 + 2,
        SsaOp::SuperMethodCall { args, .. } => args.len() as u32 + 1,
        SsaOp::SuperInvokeSlot { args, .. } => args.len() as u32 + 1,

        SsaOp::ExtensionCall { args, .. } => args.len() as u32 + 2,
        SsaOp::CallSpread { args, .. } => args.len() as u32 + 1,

        SsaOp::BuildArraySpread { .. }
        | SsaOp::ConstInt(_)
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
        | SsaOp::IsNull { .. }
        | SsaOp::TypeTest { .. }
        | SsaOp::Cast { .. }
        | SsaOp::Convert { .. }
        | SsaOp::ObjectRest { .. }
        | SsaOp::ToString { .. }
        | SsaOp::BuildStr { .. }
        | SsaOp::MakeClosure { .. }
        | SsaOp::LoadCaptured { .. }
        | SsaOp::StoreCaptured { .. }
        | SsaOp::MakeClass { .. }
        | SsaOp::DeclareLayout { .. }
        | SsaOp::FreezeClass { .. }
        | SsaOp::AllocInstance { .. }
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
        | SsaOp::BuildObjectSpread { .. } => 0,
    }
}

fn coalesce_window_args(ssa: &SsaFunc, reg: &mut [u8], call_base: u8) {
    let mut uses = vec![0u32; ssa.values.len()];
    for block in &ssa.blocks {
        for inst in &block.insts {
            crate::ssa::uses::visit_uses(&inst.kind, &mut |v| uses[v.0 as usize] += 1);
        }
        block.term.for_each_use(|v| uses[v.0 as usize] += 1);
    }
    for block in &ssa.blocks {
        for (at, inst) in block.insts.iter().enumerate() {
            let (SsaOp::Call { args, .. }
            | SsaOp::CallDirect { args, .. }
            | SsaOp::SelfCall { args }) = &inst.kind
            else {
                continue;
            };
            for (i, arg) in args.iter().enumerate() {
                if uses[arg.0 as usize] != 1 {
                    continue;
                }
                let Some(def_at) = block.insts[..at]
                    .iter()
                    .rposition(|prior| prior.dest == Some(*arg))
                else {
                    continue;
                };
                if block.insts[def_at + 1..at]
                    .iter()
                    .any(|between| window_size(&between.kind) > 0)
                {
                    continue;
                }
                reg[arg.0 as usize] = call_base + 1 + i as u8;
            }
        }
    }
}

pub(crate) fn var_reg(var: SsaVar, nparams: usize) -> u8 {
    match var {
        SsaVar::Param(i) => (1 + i) as u8,
        SsaVar::Local(id) => (1 + nparams + id as usize) as u8,
    }
}

const POOLS: usize = 7;

fn pool_index(k: vn_types::register_meta::SlotKind) -> usize {
    use vn_types::register_meta::SlotKind;
    match k {
        SlotKind::Int => 0,
        SlotKind::Float => 1,
        SlotKind::Bool => 2,
        SlotKind::Str => 3,
        SlotKind::Ref => 4,
        SlotKind::Dynamic => 5,
        SlotKind::Char => 6,
    }
}
