use cranelift_codegen::ir::{types, InstBuilder, StackSlotData, StackSlotKind, Value};
use cranelift_frontend::FunctionBuilder;
use vn_types::ssa::SsaProto;

use super::super::Ctx;

pub(crate) struct ScratchWin {
    addr: Value,
    max: usize,
}

impl ScratchWin {
    pub(crate) fn create(b: &mut FunctionBuilder, max: usize) -> Option<ScratchWin> {
        if max == 0 {
            return None;
        }
        let slot = b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            (max * 16) as u32,
            4,
        ));
        Some(ScratchWin {
            addr: b.ins().stack_addr(types::I64, slot, 0),
            max,
        })
    }
}

fn scratch_need(op: &vn_types::ssa::SsaOp) -> usize {
    if let vn_types::ssa::SsaOp::Call { args, .. }
    | vn_types::ssa::SsaOp::CallDirect { args, .. }
    | vn_types::ssa::SsaOp::SelfCall { args }
    | vn_types::ssa::SsaOp::SuperCall { args }
    | vn_types::ssa::SsaOp::MethodCall { args, .. }
    | vn_types::ssa::SsaOp::InvokeSlot { args, .. } = op
    {
        return args.len() + 1;
    }
    if let vn_types::ssa::SsaOp::CallNativeOp { args, .. }
    | vn_types::ssa::SsaOp::SuperMethodCall { args, .. }
    | vn_types::ssa::SsaOp::SuperInvokeSlot { args, .. }
    | vn_types::ssa::SsaOp::ExtensionCall { args, .. } = op
    {
        return args.len() + 1;
    }
    if let vn_types::ssa::SsaOp::CallSpread { args, .. } = op {
        return args.len();
    }
    if let vn_types::ssa::SsaOp::IntrinsicCall { args, .. } = op {
        return args.len() + 1;
    }
    if let vn_types::ssa::SsaOp::BuildStr { parts }
    | vn_types::ssa::SsaOp::BuildTuple { elements: parts } = op
    {
        return parts.len();
    }
    if let vn_types::ssa::SsaOp::BuildArray { elements, .. } = op {
        return elements.len();
    }
    if let vn_types::ssa::SsaOp::BuildMap { pairs } = op {
        return pairs.len() * 2;
    }
    if let vn_types::ssa::SsaOp::ObjectRest { skip_keys, .. } = op {
        return skip_keys.len();
    }
    0
}

pub(crate) fn scratch_max(ssa: &SsaProto) -> usize {
    let mut max = 0usize;
    for blk in &ssa.blocks {
        for inst in &blk.insts {
            let need = scratch_need(&inst.op);
            max = max.max(need);
        }
    }
    max
}

pub(crate) fn scratch_addr(b: &mut FunctionBuilder, ctx: &Ctx<'_>, count: usize) -> Value {
    if let Some(s) = &ctx.scratch
        && count <= s.max
    {
        return s.addr;
    }
    let slot = b.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        (count.max(1) * 16) as u32,
        4,
    ));
    b.ins().stack_addr(types::I64, slot, 0)
}
