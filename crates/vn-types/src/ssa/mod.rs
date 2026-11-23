use serde::{Deserialize, Serialize};

use crate::register_meta::{SlotClass, SlotKind};

mod op;
mod operands;
mod operators;
mod uses;

pub use op::SsaOp;
pub use operands::{SsaObjectSpreadPart, SsaSpread, SsaUpvalue, SsaVar};
pub use operators::{opcode_of_bin, select_bin, select_un, DynBinOp, DynUnOp, SsaBinOp, SsaUnOp};

pub type SsaTy = SlotKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ValueId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BlockId(pub u32);

impl std::fmt::Display for ValueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SsaValue {
    pub ty: SsaTy,
}

impl SsaValue {
    #[inline]
    pub fn class(&self) -> SlotClass {
        SlotClass::of_kind(self.ty)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SsaBlock {
    pub params: Vec<ValueId>,
    pub insts: Vec<SsaInst>,
    pub term: SsaTerm,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SsaInst {
    pub dest: Option<ValueId>,
    pub op: SsaOp,
    pub line: u32,
    pub ip: u32,
    pub cache: Option<u16>,
    pub resume: Option<SsaResume>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SsaResume {
    pub ip: u32,
    pub live: Vec<ValueId>,
}

impl SsaInst {
    pub fn cache_slot(&self) -> Result<u16, String> {
        self.cache
            .ok_or_else(|| format!("ssa: {:?} has no cache slot", self.op))
    }

    pub fn resume_point(&self) -> Result<&SsaResume, String> {
        self.resume
            .as_ref()
            .ok_or_else(|| format!("ssa: {:?} has no resume point", self.op))
    }

    pub fn try_landing(&self) -> Option<&SsaResume> {
        if matches!(self.op, SsaOp::Try { .. }) {
            self.resume.as_ref()
        } else {
            None
        }
    }

    pub fn for_each_use(&self, mut f: impl FnMut(ValueId)) {
        self.op.for_each_use(&mut f);
        if let Some(resume) = &self.resume {
            resume.live.iter().for_each(|v| f(*v));
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SsaTerm<V = ValueId, B = BlockId> {
    Return(Option<V>),
    Throw(V),
    Jump {
        target: B,
        args: Vec<V>,
    },
    Branch {
        cond: V,
        then_blk: B,
        then_args: Vec<V>,
        else_blk: B,
        else_args: Vec<V>,
    },
    Unreachable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SsaProto {
    pub name: std::sync::Arc<str>,
    pub nparams: u32,
    pub entry: BlockId,
    pub blocks: Vec<SsaBlock>,
    pub values: Vec<SsaValue>,
    pub regs: Vec<u32>,
    pub register_count: u16,
    pub has_this: bool,

    pub captured: Vec<(SsaVar, u32)>,

    pub secondary_entries: Vec<SsaSecondaryEntry>,

    pub call_shapes: Vec<CallShape>,

    pub call_sites: Vec<(u32, u32)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallShape {
    pub params: Vec<crate::register_meta::SlotKind>,
    pub ret: crate::register_meta::SlotKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SsaEntryKind {
    Loop,
    Resume,
    Catch,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SsaSecondaryEntry {
    pub block: BlockId,
    pub ip: u32,

    pub live: Vec<ValueId>,

    pub kind: SsaEntryKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OsrMode {
    Resume,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OsrRequest {
    pub ip: usize,
    pub mode: OsrMode,
}

impl SsaProto {
    #[inline]
    pub fn call_shape_at(&self, ip: u32) -> Option<&CallShape> {
        let at = self
            .call_sites
            .binary_search_by_key(&ip, |(site, _)| *site)
            .ok()?;
        self.call_shapes.get(self.call_sites[at].1 as usize)
    }

    pub fn block(&self, id: BlockId) -> &SsaBlock {
        &self.blocks[id.0 as usize]
    }

    #[inline]
    pub fn value_ty(&self, v: ValueId) -> SsaTy {
        self.values[v.0 as usize].ty
    }

    #[inline]
    pub fn reg(&self, v: ValueId) -> u32 {
        self.regs.get(v.0 as usize).copied().unwrap_or(0)
    }

    pub fn secondary_at(&self, ip: usize) -> Option<&SsaSecondaryEntry> {
        self.secondary_entries
            .iter()
            .find(|e| e.ip as usize == ip && e.kind == SsaEntryKind::Resume)
            .or_else(|| {
                self.secondary_entries
                    .iter()
                    .find(|e| e.ip as usize == ip && e.kind == SsaEntryKind::Loop)
            })
    }

    pub fn catch_at(&self, ip: usize) -> Option<&SsaSecondaryEntry> {
        self.secondary_entries
            .iter()
            .find(|e| e.ip as usize == ip && e.kind == SsaEntryKind::Catch)
    }

    pub fn is_resume_entry(&self, ip: usize) -> bool {
        self.secondary_entries
            .iter()
            .any(|e| e.ip as usize == ip && e.kind == SsaEntryKind::Resume)
    }

    #[inline]
    pub fn captured_reg(&self, var: SsaVar) -> Option<u32> {
        self.captured
            .iter()
            .find_map(|(v, reg)| (*v == var).then_some(*reg))
    }

    pub fn map_registers(&mut self, f: impl Fn(u32) -> u32) {
        for r in self
            .regs
            .iter_mut()
            .chain(self.captured.iter_mut().map(|(_, reg)| reg))
        {
            *r = f(*r);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PortableSsa {
    Available(std::sync::Arc<SsaProto>),
    Unavailable(std::sync::Arc<str>),
}

impl Default for PortableSsa {
    fn default() -> Self {
        Self::Unavailable(std::sync::Arc::from("not built from typed SSA"))
    }
}

impl PortableSsa {
    pub fn get(&self) -> Option<&SsaProto> {
        match self {
            Self::Available(ssa) => Some(ssa),
            Self::Unavailable(_) => None,
        }
    }

    pub fn get_mut(&mut self) -> Option<&mut SsaProto> {
        match self {
            Self::Available(ssa) => Some(std::sync::Arc::make_mut(ssa)),
            Self::Unavailable(_) => None,
        }
    }

    pub fn unavailable(&self) -> Option<&str> {
        match self {
            Self::Available(_) => None,
            Self::Unavailable(why) => Some(why),
        }
    }
}
