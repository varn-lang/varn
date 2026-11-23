use super::super::ir::{Inst, InstKind};
use super::regs::var_reg;
use crate::OptError;
use vn_core::OpCode;
use vn_types::chunk::Chunk;

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
        InstKind::SetProperty {
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
        InstKind::SetFixedField {
            object,
            value,
            slot,
            offset,
            tag,
        } => {
            chunk.write(
                Chunk::pack_op(OpCode::SetFixedField, reg[object.0 as usize]),
                line,
            );
            chunk.write(
                Chunk::pack(
                    reg[value.0 as usize],
                    vn_core::FieldAccess::Compact(*tag).encode(),
                ),
                line,
            );
            chunk.write(*slot, line);
            chunk.write(*offset as u16, line);
            return Ok(true);
        }
        InstKind::SetIndex {
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
        InstKind::ArraySetIndex {
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
        InstKind::BytesSetIndex {
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
        InstKind::ArrayPush { array, value } => {
            chunk.emit_rr(
                OpCode::ArrayPush,
                reg[array.0 as usize],
                reg[value.0 as usize],
                line,
            );
            return Ok(true);
        }
        InstKind::ObjectMerge { target, source } => {
            chunk.emit_rr(
                OpCode::ObjectMerge,
                reg[target.0 as usize],
                reg[source.0 as usize],
                line,
            );
            return Ok(true);
        }

        InstKind::AssertNotNull { operand } => {
            chunk.emit1(
                OpCode::AssertNotNull,
                Chunk::pack(reg[operand.0 as usize], 0),
                line,
            );
            return Ok(true);
        }

        InstKind::StoreGlobal { name, value } => {
            let idx = chunk.add_str(name);
            chunk.emit_rrc(OpCode::DefineGlobal, 0, reg[value.0 as usize], idx, line);
            return Ok(true);
        }
        InstKind::StoreGlobalIdx { slot, value } => {
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

        InstKind::StoreUpvalue { index, value } => {
            chunk.emit1(
                OpCode::StoreUpvalue,
                Chunk::pack(*index as u8, reg[value.0 as usize]),
                line,
            );
            return Ok(true);
        }
        InstKind::StoreCaptured { var, value } => {
            let dest_reg = var_reg(*var, nparams);
            chunk.emit_rr(OpCode::Move, dest_reg, reg[value.0 as usize], line);
            return Ok(true);
        }
        InstKind::StoreModuleSlot { value, slot } => {
            chunk.emit_rc(OpCode::StoreModuleSlot, reg[value.0 as usize], *slot, line);
            return Ok(true);
        }
        InstKind::CloseUpvalues { vars } => {
            let lowest = vars.iter().map(|t| var_reg(*t, nparams)).min().unwrap_or(0);
            chunk.emit1(OpCode::CloseUpvalue, lowest as u16, line);
            return Ok(true);
        }
        InstKind::PopTry => {
            chunk.emit(OpCode::PopTry, line);
            return Ok(true);
        }
        InstKind::DeclareLayout { class, layout } => {
            let class_reg = reg[class.0 as usize];
            let layout_idx = chunk.add_layout(std::rc::Rc::new(layout.as_ref().clone()));
            chunk.emit(OpCode::DeclareLayout, line);
            chunk.write(Chunk::pack(class_reg, 0), line);
            chunk.write(layout_idx, line);
            return Ok(true);
        }
        InstKind::FreezeClass { class } => {
            let class_reg = reg[class.0 as usize];
            chunk.emit(OpCode::FreezeClass, line);
            chunk.write(Chunk::pack(class_reg, 0), line);
            return Ok(true);
        }
        InstKind::DefineMember {
            class,
            name,
            value,
            kind,
        } => {
            let class_reg = reg[class.0 as usize];
            let value_reg = reg[value.0 as usize];
            let key_idx = chunk.add_str(name);
            chunk.emit(kind.opcode(), line);
            chunk.write(Chunk::pack(class_reg, value_reg), line);
            chunk.write(key_idx, line);
            return Ok(true);
        }
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
        | InstKind::LoadGlobal(_)
        | InstKind::LoadGlobalIdx(_)
        | InstKind::LoadNativeGlobalIdx(_)
        | InstKind::LoadUpvalue(_)
        | InstKind::Call { .. }
        | InstKind::CallDirect { .. }
        | InstKind::AllocInstance { .. }
        | InstKind::SelfCall { .. }
        | InstKind::GetProperty { .. }
        | InstKind::GetSlotMethod { .. }
        | InstKind::GetFixedField { .. }
        | InstKind::GetIndex { .. }
        | InstKind::ArrayGetIndex { .. }
        | InstKind::BytesGetIndex { .. }
        | InstKind::MethodCall { .. }
        | InstKind::IsNull { .. }
        | InstKind::TypeTest { .. }
        | InstKind::Cast { .. }
        | InstKind::Convert { .. }
        | InstKind::BuildArray { .. }
        | InstKind::BuildTuple { .. }
        | InstKind::BuildObject { .. }
        | InstKind::BuildMap { .. }
        | InstKind::ObjectRest { .. }
        | InstKind::ToString { .. }
        | InstKind::BuildStr { .. }
        | InstKind::MakeClosure { .. }
        | InstKind::LoadCaptured { .. }
        | InstKind::MakeClass { .. }
        | InstKind::MakeEnumVariant { .. }
        | InstKind::Try { .. }
        | InstKind::CatchParam { .. }
        | InstKind::LoadModule { .. }
        | InstKind::Await { .. }
        | InstKind::Spawn { .. }
        | InstKind::Yield { .. }
        | InstKind::IntrinsicCall { .. }
        | InstKind::CallNativeOp { .. }
        | InstKind::GetPropertyMaybe { .. }
        | InstKind::ModuleSlot { .. }
        | InstKind::GetEnumTag { .. }
        | InstKind::IsArray { .. }
        | InstKind::StrLength { .. }
        | InstKind::ArrayLength { .. }
        | InstKind::BytesLength { .. }
        | InstKind::This
        | InstKind::Range { .. }
        | InstKind::ObjectKeys { .. }
        | InstKind::GetSuper { .. }
        | InstKind::GetSuperSlot { .. }
        | InstKind::SuperCall { .. }
        | InstKind::SuperMethodCall { .. }
        | InstKind::InvokeSlot { .. }
        | InstKind::SuperInvokeSlot { .. }
        | InstKind::ExtensionCall { .. }
        | InstKind::CallSpread { .. }
        | InstKind::BuildArraySpread { .. }
        | InstKind::BuildObjectSpread { .. } => {}
    }

    Ok(false)
}
