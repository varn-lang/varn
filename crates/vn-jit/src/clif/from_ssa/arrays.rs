use cranelift_codegen::ir::{condcodes::IntCC, types, Block, InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;
use vn_core::ArrayStorage;
use vn_types::register_meta::SlotKind;

use super::super::emit::{
    array_disc, box_bool, box_char, box_f64, box_int, emit_array_payload, unbox_bool, unbox_char,
    unbox_f64_coerce, unbox_int,
};
use super::{heap, load_value, Ctx, Out};
use vn_types::vm_value::KIND_HEAP;

trait ElemCodegen {
    fn scale(self) -> i64;
    fn load(self, b: &mut FunctionBuilder, addr: Value) -> Value;
    fn store(self, b: &mut FunctionBuilder, v: Value, addr: Value);
    fn clif_ty(self) -> types::Type;
    fn unbox_elem(self, b: &mut FunctionBuilder, v: Value) -> Value;
    fn box_elem(self, b: &mut FunctionBuilder, v: Value) -> Value;
}

impl ElemCodegen for ArrayStorage {
    fn scale(self) -> i64 {
        match self {
            ArrayStorage::Bool => 0,
            ArrayStorage::Char => 2,
            ArrayStorage::Int | ArrayStorage::Float | ArrayStorage::Ref => 3,
            ArrayStorage::Boxed => 4,
        }
    }

    fn load(self, b: &mut FunctionBuilder, addr: Value) -> Value {
        let m = cranelift_codegen::ir::MemFlagsData::trusted();
        match self {
            ArrayStorage::Bool => b.ins().uload8(types::I64, m, addr, 0),
            ArrayStorage::Char => b.ins().uload32(m, addr, 0),
            ArrayStorage::Ref => {
                let raw = b.ins().load(types::I64, m, addr, 0);
                let tag = b.ins().iconst(types::I64, KIND_HEAP as i64);
                b.ins().iconcat(tag, raw)
            }
            ArrayStorage::Int | ArrayStorage::Float | ArrayStorage::Boxed => {
                b.ins().load(self.clif_ty(), m, addr, 0)
            }
        }
    }

    fn store(self, b: &mut FunctionBuilder, v: Value, addr: Value) {
        let m = cranelift_codegen::ir::MemFlagsData::trusted();
        match self {
            ArrayStorage::Bool => {
                b.ins().istore8(m, v, addr, 0);
            }
            ArrayStorage::Char => {
                b.ins().istore32(m, v, addr, 0);
            }
            ArrayStorage::Ref => {
                let raw = if b.func.dfg.value_type(v) == types::I128 {
                    b.ins().isplit(v).1
                } else {
                    v
                };
                b.ins().store(m, raw, addr, 0);
            }
            ArrayStorage::Int | ArrayStorage::Float | ArrayStorage::Boxed => {
                b.ins().store(m, v, addr, 0);
            }
        }
    }

    fn clif_ty(self) -> types::Type {
        match self {
            ArrayStorage::Int | ArrayStorage::Char | ArrayStorage::Bool => types::I64,
            ArrayStorage::Float => types::F64,
            ArrayStorage::Ref | ArrayStorage::Boxed => types::I128,
        }
    }

    fn unbox_elem(self, b: &mut FunctionBuilder, v: Value) -> Value {
        match self {
            ArrayStorage::Int => unbox_int(b, v),
            ArrayStorage::Float => unbox_f64_coerce(b, v),
            ArrayStorage::Char => unbox_char(b, v),
            ArrayStorage::Bool => unbox_bool(b, v),
            ArrayStorage::Ref | ArrayStorage::Boxed => v,
        }
    }

    fn box_elem(self, b: &mut FunctionBuilder, v: Value) -> Value {
        match self {
            ArrayStorage::Int => box_int(b, v),
            ArrayStorage::Float => box_f64(b, v),
            ArrayStorage::Char => box_char(b, v),
            ArrayStorage::Bool => box_bool(b, v),
            ArrayStorage::Ref => {
                if b.func.dfg.value_type(v) == types::I128 {
                    v
                } else {
                    let tag = b.ins().iconst(types::I64, KIND_HEAP as i64);
                    b.ins().iconcat(tag, v)
                }
            }
            ArrayStorage::Boxed => v,
        }
    }
}

pub(super) fn view(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    values: &[Option<Value>],
    object: u32,
    slow: Block,
) -> Result<(Value, Value, Value), String> {
    let Some(v) = ctx.views.of(object) else {
        let obj = heap::boxed_value(b, ctx, values, object)?;
        return Ok(resolve(b, ctx, obj, slow));
    };
    let data = b.use_var(v.data);
    let len = b.use_var(v.len);
    let disc = b.use_var(v.disc);
    let pinned_here = ctx
        .pinned_bodies
        .borrow()
        .get(&object)
        .is_some_and(|body| body.contains(&ctx.emitting_block.get()));
    if pinned_here {
        let valid = b.create_block();
        b.ins().brif(data, valid, &[], slow, &[]);
        b.switch_to_block(valid);
        return Ok((data, len, disc));
    }
    let ready = b.create_block();
    for _ in 0..3 {
        b.append_block_param(ready, types::I64);
    }
    let miss = b.create_block();
    b.ins().brif(
        data,
        ready,
        &[data.into(), len.into(), disc.into()],
        miss,
        &[],
    );

    b.switch_to_block(miss);
    let obj = heap::boxed_value(b, ctx, values, object)?;
    let (d, l, di) = resolve(b, ctx, obj, slow);
    b.def_var(v.data, d);
    b.def_var(v.len, l);
    b.def_var(v.disc, di);

    super::store::drop_home_addrs(ctx);
    b.ins().jump(ready, &[d.into(), l.into(), di.into()]);

    b.switch_to_block(ready);
    let p = b.block_params(ready);
    Ok((p[0], p[1], p[2]))
}

fn resolve(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    obj: Value,
    slow: Block,
) -> (Value, Value, Value) {
    let lay = &ctx.helpers.array_layout;
    let payload = emit_array_payload(b, obj, lay, slow);
    let m = cranelift_codegen::ir::MemFlagsData::trusted();
    let data = b
        .ins()
        .load(types::I64, m, payload, lay.elems_ptr_off as i32);
    let len = b
        .ins()
        .load(types::I64, m, payload, lay.elems_len_off as i32);
    let disc = array_disc(b, payload, lay);
    (data, len, disc)
}

fn bounds(b: &mut FunctionBuilder, key: Value, len: Value, slow: Block) {
    let in_bounds = b.ins().icmp(IntCC::UnsignedLessThan, key, len);
    let hit = b.create_block();
    b.ins().brif(in_bounds, hit, &[], slow, &[]);
    b.switch_to_block(hit);
}

pub(super) fn emit_get(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    values: &[Option<Value>],
    object: u32,
    index: u32,
    dest: Option<u32>,
) -> Result<Out, String> {
    let exec_ctx = heap::exec_ctx(ctx);
    let want = SlotKind::array_storage(dest.map_or(SlotKind::Dynamic, |d| ctx.ssa.value_ty(d)));
    let key = load_value(b, ctx, values, index)?;

    let slow = b.create_block();
    b.set_cold_block(slow);
    let merge = b.create_block();
    b.append_block_param(merge, want.clif_ty());

    let (data, len, disc) = view(b, ctx, values, object, slow)?;
    if !(ctx.elide.get() && ctx.fast_bounds.contains(&index)) {
        bounds(b, key, len, slow);
    }

    let matched = b.create_block();
    let other = b.create_block();
    let is_match = b
        .ins()
        .icmp_imm_u(IntCC::Equal, disc, i64::from(want.bits()));
    b.ins().brif(is_match, matched, &[], other, &[]);

    b.switch_to_block(matched);
    let off = b.ins().ishl_imm_u(key, want.scale());
    let addr = b.ins().iadd(data, off);
    let v = want.load(b, addr);
    b.ins().jump(merge, &[v.into()]);

    b.switch_to_block(other);
    if want == ArrayStorage::Boxed {
        let is_ref = b
            .ins()
            .icmp_imm_u(IntCC::Equal, disc, i64::from(ArrayStorage::Ref.bits()));
        let ref_arm = b.create_block();
        b.ins().brif(is_ref, ref_arm, &[], slow, &[]);
        b.switch_to_block(ref_arm);
        let off = b.ins().ishl_imm_u(key, ArrayStorage::Ref.scale());
        let addr = b.ins().iadd(data, off);
        let v = ArrayStorage::Ref.load(b, addr);
        b.ins().jump(merge, &[v.into()]);
    } else {
        let is_boxed = b.ins().icmp_imm_u(IntCC::Equal, disc, 0);
        let boxed_arm = b.create_block();
        b.ins().brif(is_boxed, boxed_arm, &[], slow, &[]);
        b.switch_to_block(boxed_arm);
        let off = b.ins().ishl_imm_u(key, 4);
        let addr = b.ins().iadd(data, off);
        let raw = b.ins().load(
            types::I128,
            cranelift_codegen::ir::MemFlagsData::trusted(),
            addr,
            0,
        );
        let v = want.unbox_elem(b, raw);
        b.ins().jump(merge, &[v.into()]);
    }

    b.switch_to_block(slow);
    let obj = heap::boxed_value(b, ctx, values, object)?;
    let (ot, op) = b.ins().isplit(obj);
    let boxed_key = box_int(b, key);
    let (kt, kp) = b.ins().isplit(boxed_key);
    crate::clif::from_ssa::exceptions::call_checked(
        b,
        ctx,
        ctx.helpers.jit_array_get_fast,
        &[exec_ctx, ot, op, kt, kp],
    )?;
    let r = crate::clif::emit::load_native_result(b, ctx.helpers, exec_ctx);
    let r = want.unbox_elem(b, r);

    super::store::drop_home_addrs(ctx);
    b.ins().jump(merge, &[r.into()]);

    b.switch_to_block(merge);
    let res = b.block_params(merge)[0];
    Ok(match want {
        ArrayStorage::Int | ArrayStorage::Float | ArrayStorage::Char | ArrayStorage::Bool => {
            Out::Native(res)
        }
        ArrayStorage::Ref => Out::Boxed(res),
        ArrayStorage::Boxed => Out::Boxed(res),
    })
}

pub(super) fn emit_set(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    values: &[Option<Value>],
    object: u32,
    index: u32,
    value: u32,
) -> Result<(), String> {
    let exec_ctx = heap::exec_ctx(ctx);
    let src = SlotKind::array_storage(ctx.ssa.value_ty(value));
    let key = load_value(b, ctx, values, index)?;

    let slow = b.create_block();
    b.set_cold_block(slow);
    let merge = b.create_block();

    if src.stores_without_barrier() {
        let raw = load_value(b, ctx, values, value)?;
        let (data, len, disc) = view(b, ctx, values, object, slow)?;
        if !(ctx.elide.get() && ctx.fast_bounds.contains(&index)) {
            bounds(b, key, len, slow);
        }

        let matched = b.create_block();
        let other = b.create_block();
        let is_match = b
            .ins()
            .icmp_imm_u(IntCC::Equal, disc, i64::from(src.bits()));
        b.ins().brif(is_match, matched, &[], other, &[]);

        b.switch_to_block(matched);
        let off = b.ins().ishl_imm_u(key, src.scale());
        let addr = b.ins().iadd(data, off);
        src.store(b, raw, addr);
        b.ins().jump(merge, &[]);

        b.switch_to_block(other);
        let is_boxed = b.ins().icmp_imm_u(IntCC::Equal, disc, 0);
        let boxed_arm = b.create_block();
        b.ins().brif(is_boxed, boxed_arm, &[], slow, &[]);
        b.switch_to_block(boxed_arm);
        let off = b.ins().ishl_imm_u(key, 4);
        let addr = b.ins().iadd(data, off);
        let boxed = src.box_elem(b, raw);
        b.ins().store(
            cranelift_codegen::ir::MemFlagsData::trusted(),
            boxed,
            addr,
            0,
        );
        b.ins().jump(merge, &[]);
    } else {
        b.ins().jump(slow, &[]);
    }

    b.switch_to_block(slow);
    let obj = heap::boxed_value(b, ctx, values, object)?;
    let (ot, op) = b.ins().isplit(obj);
    let boxed_key = box_int(b, key);
    let (kt, kp) = b.ins().isplit(boxed_key);
    let (vt, vp) = heap::boxed_parts(b, ctx, values, value)?;
    crate::clif::from_ssa::exceptions::call_checked(
        b,
        ctx,
        ctx.helpers.jit_array_set_fast,
        &[exec_ctx, ot, op, kt, kp, vt, vp],
    )?;

    ctx.views.clear(b);

    super::store::drop_home_addrs(ctx);
    b.ins().jump(merge, &[]);

    b.switch_to_block(merge);
    Ok(())
}

pub(super) fn prefill(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    values: &[Option<Value>],
    objects: &[u32],
) -> Result<Block, String> {
    let done = b.create_block();
    let mut slows = Vec::with_capacity(objects.len());
    for object in objects {
        let slow = b.create_block();
        b.set_cold_block(slow);
        slows.push(slow);
        let _ = view(b, ctx, values, *object, slow)?;
    }
    b.ins().jump(done, &[]);
    for slow in slows {
        b.switch_to_block(slow);
        b.ins().jump(done, &[]);
    }
    b.switch_to_block(done);
    Ok(done)
}
