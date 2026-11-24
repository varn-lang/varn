use super::super::ir::{SsaFunc, Value};
use vn_core::OpCode;
use vn_types::ssa::SsaBinOp;
use vn_types::ssa::SsaOp;

pub(super) struct Immediates {
    pub(super) imm: Vec<Option<i8>>,

    elided: Vec<bool>,
}

impl Immediates {
    pub(super) fn is_elided(&self, v: Value) -> bool {
        self.elided.get(v.0 as usize).copied().unwrap_or(false)
    }
}

pub(super) struct ImmediateForm {
    pub(super) carrier: Value,
    pub(super) other: Value,
    pub(super) value: i8,
    pub(super) opcode: OpCode,
}

pub(super) fn immediate_operand(kind: &SsaOp, imm: &[Option<i8>]) -> Option<ImmediateForm> {
    let SsaOp::Binary { op, lhs, rhs } = kind else {
        return None;
    };
    let get = |v: &Value| -> Option<i8> { *imm.get(v.0 as usize)? };
    let form = |opcode: OpCode, swapped: bool| -> Option<ImmediateForm> {
        let (carrier, other) = if swapped { (lhs, rhs) } else { (rhs, lhs) };
        Some(ImmediateForm {
            carrier: *carrier,
            other: *other,
            value: get(carrier)?,
            opcode,
        })
    };
    match op {
        SsaBinOp::IntAdd => form(OpCode::AddImm, false).or_else(|| form(OpCode::AddImm, true)),
        SsaBinOp::IntSub => form(OpCode::SubImm, false),
        SsaBinOp::IntLt => form(OpCode::LtImm, false).or_else(|| form(OpCode::GtImm, true)),
        SsaBinOp::IntLe => form(OpCode::LteImm, false).or_else(|| form(OpCode::GteImm, true)),
        SsaBinOp::IntGt => form(OpCode::GtImm, false).or_else(|| form(OpCode::LtImm, true)),
        SsaBinOp::IntGe => form(OpCode::GteImm, false).or_else(|| form(OpCode::LteImm, true)),
        SsaBinOp::IntEq => form(OpCode::EqImm, false).or_else(|| form(OpCode::EqImm, true)),
        SsaBinOp::IntNe => form(OpCode::NeqImm, false).or_else(|| form(OpCode::NeqImm, true)),
        SsaBinOp::IntMul
        | SsaBinOp::IntDiv
        | SsaBinOp::IntMod
        | SsaBinOp::IntPow
        | SsaBinOp::IntAnd
        | SsaBinOp::IntOr
        | SsaBinOp::IntXor
        | SsaBinOp::IntShl
        | SsaBinOp::IntShr
        | SsaBinOp::IntUshr
        | SsaBinOp::FloatAdd
        | SsaBinOp::FloatSub
        | SsaBinOp::FloatMul
        | SsaBinOp::FloatDiv
        | SsaBinOp::FloatMod
        | SsaBinOp::FloatPow
        | SsaBinOp::FloatEq
        | SsaBinOp::FloatNe
        | SsaBinOp::FloatLt
        | SsaBinOp::FloatLe
        | SsaBinOp::FloatGt
        | SsaBinOp::FloatGe
        | SsaBinOp::StrConcat
        | SsaBinOp::StrEq
        | SsaBinOp::StrNe
        | SsaBinOp::BitsEq
        | SsaBinOp::BitsNe
        | SsaBinOp::Dyn(_) => None,
    }
}

pub(super) fn plan_immediates(ssa: &SsaFunc) -> Immediates {
    let n = ssa.values.len();
    let mut imm: Vec<Option<i8>> = vec![None; n];
    for block in &ssa.blocks {
        for inst in &block.insts {
            if let (Some(d), SsaOp::ConstInt(i)) = (inst.dest, &inst.kind)
                && let Ok(small) = i8::try_from(*i)
            {
                imm[d.0 as usize] = Some(small);
            }
        }
    }

    let mut total = vec![0u32; n];
    let mut folded = vec![0u32; n];
    for block in &ssa.blocks {
        for inst in &block.insts {
            inst.kind.for_each_use(|v| total[v.0 as usize] += 1);
            if let Some(form) = immediate_operand(&inst.kind, &imm) {
                folded[form.carrier.0 as usize] += 1;
            }
        }
        block.term.for_each_use(|v| total[v.0 as usize] += 1);
    }

    let elided = (0..n)
        .map(|i| imm[i].is_some() && total[i] > 0 && total[i] == folded[i])
        .collect();
    Immediates { imm, elided }
}
