use super::context::Builder;
use crate::ssa::ir::Value;
use std::sync::Arc;
use vn_tir::BackendTy;
use vn_types::ssa::SsaOp;

impl<'m> Builder<'m> {
    pub(super) fn gname(&self, name: &str) -> Arc<str> {
        Arc::from(format!(
            "{}::{}",
            self.tir.source_file.replace('\\', "/"),
            name
        ))
    }

    pub(super) fn gslot(&self, bare: &str) -> Option<u32> {
        self.tir
            .global_names
            .iter()
            .position(|n| n.as_ref() == bare)
            .map(|i| i as u32)
    }

    pub(super) fn global_load(&self, bare: &str) -> SsaOp {
        match self.gslot(bare) {
            Some(slot) => SsaOp::LoadGlobalIdx(slot),
            None => match vn_core::abi::native_global_index(bare) {
                Some(native) => SsaOp::LoadNativeGlobalIdx(native),
                None => SsaOp::LoadGlobal(self.gname(bare)),
            },
        }
    }

    pub(super) fn global_store(&self, bare: &str, value: Value) -> SsaOp {
        match self.gslot(bare) {
            Some(slot) => SsaOp::StoreGlobalIdx { slot, value },
            None => SsaOp::StoreGlobal {
                name: self.gname(bare),
                value,
            },
        }
    }

    pub(super) fn coerce(&mut self, v: Value, target: BackendTy) -> Value {
        let target = target.representation();
        let from = self.value_ty(v);
        if from == target || (from.is_boxed() && target.is_boxed()) {
            v
        } else {
            self.emit(SsaOp::Cast { operand: v }, target)
        }
    }
}
