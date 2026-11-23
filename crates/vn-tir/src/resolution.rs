use crate::ty::{DynReason, EnumId, FnId, LocalId, ModuleId};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Resolution {
    None,

    Local(LocalId),
    Param(u32),
    Upvalue(u32),

    GlobalSlot(u32),

    NativeGlobal(u32),
    ModuleSlot { module: ModuleId, slot: u32 },

    FieldSlot(u16),
    StaticField(u16),

    VtableSlot(u16),

    DirectFn(FnId),

    Intrinsic(u16),

    NativeOp(u64),

    NativeDecode { plain: u64, typed: u64 },

    EnumVariant { enum_id: EnumId, tag: i64 },

    ByName { name: Arc<str>, why: DynReason },
}

impl Resolution {
    pub fn native_op(&self) -> Option<u64> {
        match self {
            Resolution::NativeOp(id) => Some(*id),
            Resolution::NativeDecode { plain, .. } => Some(*plain),
            Resolution::None
            | Resolution::Local(_)
            | Resolution::Param(_)
            | Resolution::Upvalue(_)
            | Resolution::GlobalSlot(_)
            | Resolution::NativeGlobal(_)
            | Resolution::ModuleSlot { .. }
            | Resolution::FieldSlot(_)
            | Resolution::StaticField(_)
            | Resolution::VtableSlot(_)
            | Resolution::DirectFn(_)
            | Resolution::Intrinsic(_)
            | Resolution::EnumVariant { .. }
            | Resolution::ByName { .. } => None,
        }
    }
}
