use super::operand::{At, Layout, Operand};
use super::{layout, ConstKind};
use crate::chunk::PoolEntry;
use vn_core::OpCode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifyError {
    Empty,
    BadOpcode { offset: usize },
    Truncated { offset: usize, end: usize },
    NoTerminator,
    BadJump { offset: usize, target: usize },
    BadConst { offset: usize, index: usize },
    BadReg { offset: usize, reg: usize },
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "chunk has no code"),
            Self::BadOpcode { offset } => write!(f, "invalid opcode at word {offset}"),
            Self::Truncated { offset, end } => {
                write!(f, "instruction at word {offset} runs past the end ({end})")
            }
            Self::NoTerminator => write!(f, "chunk does not end with a terminator"),
            Self::BadJump { offset, target } => {
                write!(
                    f,
                    "jump at word {offset} lands outside an instruction ({target})"
                )
            }
            Self::BadConst { offset, index } => {
                write!(f, "constant {index} at word {offset} is out of range")
            }
            Self::BadReg { offset, reg } => {
                write!(f, "register r{reg} at word {offset} is out of range")
            }
        }
    }
}

fn is_terminator(op: OpCode) -> bool {
    matches!(
        op,
        OpCode::Return
            | OpCode::Throw
            | OpCode::Yield
            | OpCode::Await
            | OpCode::Jump
            | OpCode::Loop
    )
}

fn const_matches(kind: ConstKind, entry: &PoolEntry) -> bool {
    match kind {
        ConstKind::Function => matches!(entry, PoolEntry::Function(_)),
        ConstKind::Shape => matches!(entry, PoolEntry::Shape(_)),
        ConstKind::Layout => matches!(entry, PoolEntry::Layout(_)),
        ConstKind::Variant => matches!(entry, PoolEntry::Variant(_)),
        ConstKind::Value | ConstKind::Name | ConstKind::Module | ConstKind::NativeOp => true,
    }
}

pub fn verify_chunk(
    code: &[u16],
    constants: &[PoolEntry],
    reg_count: u32,
) -> Result<(), VerifyError> {
    if code.is_empty() {
        return Err(VerifyError::Empty);
    }
    let mut boundaries = vec![false; code.len()];
    let mut layouts: Vec<(usize, Layout)> = Vec::new();
    let mut offset = 0;
    while offset < code.len() {
        let layout = layout(code, offset, constants).ok_or(VerifyError::BadOpcode { offset })?;
        if offset + layout.len > code.len() {
            return Err(VerifyError::Truncated {
                offset,
                end: offset + layout.len,
            });
        }
        for operand in &layout.operands {
            match *operand {
                Operand::Reg { at, .. } => {
                    let reg = at.read(code, offset) as usize;
                    if reg >= reg_count as usize {
                        return Err(VerifyError::BadReg { offset, reg });
                    }
                }
                Operand::Run { start, count, .. } => {
                    let first = start.read(code, offset) as usize;
                    if count > 0 && first + count > reg_count as usize {
                        return Err(VerifyError::BadReg { offset, reg: first });
                    }
                }
                Operand::Fixed { reg, .. } => {
                    if reg as usize >= reg_count as usize {
                        return Err(VerifyError::BadReg {
                            offset,
                            reg: reg as usize,
                        });
                    }
                }
                Operand::Const { word, kind } => {
                    let index = At::Word(word).read(code, offset) as usize;
                    match constants.get(index) {
                        Some(entry) if const_matches(kind, entry) => {}
                        _ => return Err(VerifyError::BadConst { offset, index }),
                    }
                }
                Operand::Imm { .. } | Operand::Jump { .. } => {}
            }
        }
        boundaries[offset] = true;
        let len = layout.len;
        layouts.push((offset, layout));
        offset += len;
    }
    let terminator = layouts.last().map(|(_, layout)| layout.op);
    if !terminator.is_some_and(is_terminator) {
        return Err(VerifyError::NoTerminator);
    }
    for (offset, layout) in &layouts {
        if let Some(target) = layout.jump_target(code, *offset)
            && (target >= code.len() || !boundaries[target])
        {
            return Err(VerifyError::BadJump {
                offset: *offset,
                target,
            });
        }
    }
    Ok(())
}
