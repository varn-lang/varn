use cranelift_codegen::ir::{FuncRef, Value};
use cranelift_codegen::isa::CallConv;
use rustc_hash::{FxHashMap, FxHashSet};
use vn_types::register_meta::FrameLayout;
use vn_types::ssa::{SsaProto, ValueId};
use vn_types::{FunctionProto, VmValue};

use super::super::abi::Activation;
use super::super::piece::CompiledPiece;
use super::call;
use super::views;
use crate::JitHelpers;

pub(super) struct FrameIo {
    pub exec_ctx: Value,
    pub closure: Value,
    pub base: Option<Value>,

    pub layout: FrameLayout,
}

pub(super) struct SelfAbi {
    pub depth: Value,
}

pub(super) struct Ctx<'a> {
    pub cc: CallConv,
    pub helpers: &'a JitHelpers,
    pub ssa: &'a SsaProto,
    pub proto: &'a FunctionProto,

    pub constants: &'a [VmValue],

    pub fusable_cmp: FxHashSet<ValueId>,

    pub fuse_ok: std::cell::Cell<bool>,

    pub pending_cmp: std::cell::RefCell<Option<super::super::generic::PendingStrCmp>>,

    pub sso_consts: FxHashMap<ValueId, super::super::generic::SsoConst>,
    pub self_ref: FuncRef,

    pub self_abi: Option<SelfAbi>,

    pub redundant_self_guards: FxHashSet<u32>,

    pub arg_word_base: usize,

    pub exec_ctx: Value,

    pub frame: Option<FrameIo>,
    pub activation: Activation,

    pub this: Option<Value>,

    pub has_round: bool,

    pub views: views::Views,

    pub in_range_steps: FxHashSet<ValueId>,

    pub no_overflow: FxHashSet<ValueId>,

    pub fast_bounds: FxHashSet<ValueId>,

    pub elide: std::cell::Cell<bool>,

    pub pinned_bodies:
        std::cell::RefCell<std::collections::BTreeMap<ValueId, std::collections::BTreeSet<usize>>>,

    pub emitting_block: std::cell::Cell<usize>,

    pub site_ip: std::cell::Cell<u32>,

    pub carried: FxHashMap<ValueId, cranelift_frontend::Variable>,

    pub try_live: FxHashSet<ValueId>,

    pub fused_awaits: FxHashMap<ValueId, (ValueId, Vec<ValueId>)>,

    pub pure_try: std::cell::RefCell<PureTryState>,

    pub home_addrs: std::cell::RefCell<FxHashMap<u32, Value>>,

    pub natives: std::cell::RefCell<Vec<u64>>,

    pub scratch: Option<call::ScratchWin>,
}

pub(in crate::clif) const NEEDS_ACTIVATION: &str = "from_ssa: body needs a FrameStore activation";

#[derive(Default)]
pub(super) struct PureTryState {
    pub err_ret: Option<cranelift_codegen::ir::Block>,
    pub entry_stacks: Vec<Vec<usize>>,
    pub landing_error: std::collections::BTreeMap<usize, ValueId>,
    pub clif_map: std::collections::BTreeMap<usize, cranelift_codegen::ir::Block>,
    pub stack: Vec<usize>,
}

pub(in crate::clif) struct Lowered {
    pub piece: CompiledPiece,

    pub self_body: Option<CompiledPiece>,

    pub natives: Vec<u64>,

    pub frameless: bool,
}

impl Ctx<'_> {
    pub(super) fn frame_for(&self, op: &str) -> Result<&FrameIo, String> {
        self.frame
            .as_ref()
            .ok_or_else(|| format!("from_ssa: {op} without a frame"))
    }

    pub(super) fn exits(
        &self,
        b: &mut cranelift_frontend::FunctionBuilder,
    ) -> (
        Option<cranelift_codegen::ir::Block>,
        Option<cranelift_codegen::ir::Block>,
    ) {
        let mut st = self.pure_try.borrow_mut();
        let landing = st.stack.last().and_then(|h| st.clif_map.get(h).copied());
        if landing.is_none() && st.err_ret.is_none() {
            st.err_ret = Some(b.create_block());
        }
        (landing, st.err_ret)
    }
}
