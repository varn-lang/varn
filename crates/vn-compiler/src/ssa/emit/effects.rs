use super::super::ir::Inst;
use super::regs::var_reg;
use crate::OptError;
use vn_core::OpCode;
use vn_types::chunk::Chunk;
use vn_types::ssa::SsaOp;

type Result<T> = std::result::Result<T, OptError>;

pub(super) fn emit_effect(
    chunk: &mut Chunk,
    inst: &Inst,
    reg: &[u8],
    ic_slot: Option<u8>,
    nparams: usize,
) -> Result<bool> {
    let line = inst.line;
    match &inst.kind {
        SsaOp::SetProperty {
            object,
            name,
            value,
        } => {
            let idx = chunk.add_str(name);
            let cs = ic_slot.ok_or(OptError::Unsupported("ssa-emit: cache site without a slot"))?;
            chunk.emit_rrc_ic(
                OpCode::SetProperty,
                reg[object.0 as usize],
                reg[value.0 as usize],
                idx,
                cs,
                line,
            );
            return Ok(true);
        }
        SsaOp::SetFixedField {
            object,
            value,
            slot,
            offset,
            kind,
        } => {
            chunk.write(
                Chunk::pack_op(OpCode::SetFixedField, reg[object.0 as usize]),
                line,
            );
            chunk.write(
                Chunk::pack(
                    reg[value.0 as usize],
                    vn_core::FieldAccess::Compact(*kind).encode(),
                ),
                line,
            );
            chunk.write(*slot, line);
            chunk.write(*offset as u16, line);
            return Ok(true);
        }
        SsaOp::SetIndex {
            object,
            index,
            value,
        } => {
            chunk.emit_rrr(
                OpCode::SetIndex,
                reg[object.0 as usize],
                reg[index.0 as usize],
                reg[value.0 as usize],
                line,
            );
            return Ok(true);
        }
        SsaOp::ArraySetIndex {
            object,
            index,
            value,
        } => {
            chunk.emit_rrr(
                OpCode::ArraySetIndex,
                reg[object.0 as usize],
                reg[index.0 as usize],
                reg[value.0 as usize],
                line,
            );
            return Ok(true);
        }
        SsaOp::BytesSetIndex {
            object,
            index,
            value,
        } => {
            chunk.emit_rrr(
                OpCode::BytesSetIndex,
                reg[object.0 as usize],
                reg[index.0 as usize],
                reg[value.0 as usize],
                line,
            );
            return Ok(true);
        }
        SsaOp::ArrayPush { array, value } => {
            chunk.emit_rr(
                OpCode::ArrayPush,
                reg[array.0 as usize],
                reg[value.0 as usize],
                line,
            );
            return Ok(true);
        }
        SsaOp::ObjectMerge { target, source } => {
            chunk.emit_rr(
                OpCode::ObjectMerge,
                reg[target.0 as usize],
                reg[source.0 as usize],
                line,
            );
            return Ok(true);
        }

        SsaOp::AssertNotNull { operand } => {
            chunk.emit1(
                OpCode::AssertNotNull,
                Chunk::pack(reg[operand.0 as usize], 0),
                line,
            );
            return Ok(true);
        }

        SsaOp::StoreGlobal { name, value } => {
            let idx = chunk.add_str(name);
            chunk.emit_rrc(OpCode::DefineGlobal, 0, reg[value.0 as usize], idx, line);
            return Ok(true);
        }
        SsaOp::StoreGlobalIdx { slot, value } => {
            let slot = u16::try_from(*slot)
                .map_err(|_| OptError::Unsupported("ssa-emit: global slot exceeds u16"))?;
            chunk.emit_rrc(
                OpCode::DefineGlobalIdx,
                0,
                reg[value.0 as usize],
                slot,
                line,
            );
            return Ok(true);
        }

        SsaOp::StoreUpvalue { index, value } => {
            chunk.emit1(
                OpCode::StoreUpvalue,
                Chunk::pack(*index as u8, reg[value.0 as usize]),
                line,
            );
            return Ok(true);
        }
        SsaOp::StoreCaptured { var, value } => {
            let dest_reg = var_reg(*var, nparams);
            chunk.emit_rr(OpCode::Move, dest_reg, reg[value.0 as usize], line);
            return Ok(true);
        }
        SsaOp::StoreModuleSlot { value, slot } => {
            chunk.emit_rc(OpCode::StoreModuleSlot, reg[value.0 as usize], *slot, line);
            return Ok(true);
        }
        SsaOp::CloseUpvalues { vars } => {
            let lowest = vars.iter().map(|t| var_reg(*t, nparams)).min().unwrap_or(0);
            chunk.emit1(OpCode::CloseUpvalue, lowest as u16, line);
            return Ok(true);
        }
        SsaOp::PopTry => {
            chunk.emit(OpCode::PopTry, line);
            return Ok(true);
        }
        SsaOp::DeclareLayout { class, layout } => {
            let class_reg = reg[class.0 as usize];
            let layout_idx = chunk.add_layout(std::rc::Rc::new(layout.as_ref().clone()));
            chunk.emit(OpCode::DeclareLayout, line);
            chunk.write(Chunk::pack(class_reg, 0), line);
            chunk.write(layout_idx, line);
            return Ok(true);
        }
        SsaOp::FreezeClass { class } => {
            let class_reg = reg[class.0 as usize];
            chunk.emit(OpCode::FreezeClass, line);
            chunk.write(Chunk::pack(class_reg, 0), line);
            return Ok(true);
        }
        SsaOp::DefineMember {
            class,
            name,
            member,
            kind,
        } => {
            let class_reg = reg[class.0 as usize];
            let value_reg = reg[member.0 as usize];
            let key_idx = chunk.add_str(name);
            chunk.emit(kind.opcode(), line);
            chunk.write(Chunk::pack(class_reg, value_reg), line);
            chunk.write(key_idx, line);
            return Ok(true);
        }
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
        | SsaOp::MethodCall { .. }
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
        | SsaOp::MakeClass { .. }
        | SsaOp::MakeEnumVariant { .. }
        | SsaOp::Try { .. }
        | SsaOp::CatchParam { .. }
        | SsaOp::LoadModule { .. }
        | SsaOp::Await { .. }
        | SsaOp::Spawn { .. }
        | SsaOp::Yield { .. }
        | SsaOp::IntrinsicCall { .. }
        | SsaOp::CallNativeOp { .. }
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
        | SsaOp::InvokeSlot { .. }
        | SsaOp::SuperInvokeSlot { .. }
        | SsaOp::ExtensionCall { .. }
        | SsaOp::CallSpread { .. }
        | SsaOp::BuildArraySpread { .. }
        | SsaOp::BuildObjectSpread { .. } => {}
    }

    Ok(false)
}
