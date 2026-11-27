use cranelift_codegen::ir::{InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;
use vn_types::register_meta::SlotKind;

use super::super::super::super::native_abi::{NativeClass, NativeShape};
use super::super::super::{heap, load_value, Ctx};
use vn_types::ssa::ValueId;

pub(super) fn stage_argv(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    values: &[Option<Value>],
    ectx: Value,
    closure: Value,
    receiver: Value,
    args: &[ValueId],
    shape: &NativeShape,
) -> Result<Vec<Value>, String> {
    let (rtag, rpayload) = b.ins().isplit(receiver);
    let mut argv = vec![ectx, closure, rtag, rpayload];
    for (v, class) in args.iter().zip(&shape.params[1..]) {
        match crate::clif::native_abi::NativeClass::of(*class) {
            NativeClass::Word | NativeClass::Float => argv.push(load_value(b, ctx, values, *v)?),
            NativeClass::Boxed => {
                let boxed = heap::boxed_value(b, ctx, values, *v)?;
                let (t, p) = b.ins().isplit(boxed);
                argv.push(t);
                argv.push(p);
            }
        }
    }
    Ok(argv)
}

pub(super) fn box_call_result(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    res: &[Value],
    shape_ret: NativeClass,
    dest_kind: SlotKind,
) -> Result<Value, String> {
    Ok(match shape_ret {
        NativeClass::Word | NativeClass::Float => res[0],
        NativeClass::Boxed => {
            let boxed = b.ins().iconcat(res[0], res[1]);
            if NativeClass::of(dest_kind) == NativeClass::Boxed {
                boxed
            } else {
                heap::unbox_checked(b, ctx, dest_kind, boxed)?
            }
        }
    })
}

pub(super) fn declared_shape(
    ctx: &Ctx<'_>,
    arg_kinds: &[SlotKind],
    dest_kind: SlotKind,
) -> Option<NativeShape> {
    let declared = ctx.ssa.call_shape_at(ctx.site_ip.get())?;
    if declared.params.len() != arg_kinds.len() {
        return None;
    }
    let same_class = |a: SlotKind, b: SlotKind| NativeClass::of(a) == NativeClass::of(b);
    let args_fit = declared.params.iter().zip(arg_kinds).all(|(param, arg)| {
        NativeClass::of(*param) == NativeClass::Boxed || same_class(*param, *arg)
    });
    let ret_fits =
        same_class(declared.ret, dest_kind) || NativeClass::of(declared.ret) == NativeClass::Boxed;
    (args_fit && ret_fits).then(|| NativeShape::new(&declared.params, declared.ret))
}
