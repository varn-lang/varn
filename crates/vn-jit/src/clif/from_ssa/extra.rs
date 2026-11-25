mod build;
mod extra_shared;
mod invoke;
mod modules;
mod props;
mod suspend;

pub(super) use suspend::fused_await_calls;

use super::store::Out;
use super::Ctx;
use cranelift_codegen::ir::Value;
use cranelift_frontend::FunctionBuilder;
use vn_types::ssa::SsaOp;

pub(super) fn try_emit(
    b: &mut FunctionBuilder,
    ctx: &Ctx<'_>,
    values: &mut [Option<Value>],
    inst: &vn_types::ssa::SsaInst,
) -> Result<Option<Option<Out>>, String> {
    let (op, dest) = (&inst.op, inst.dest);
    if let SsaOp::LoadGlobal(name) = op {
        modules::load_global(b, ctx, name)
    } else if let SsaOp::StoreGlobal { name, value } = op {
        modules::store_global(b, ctx, values, name, *value)
    } else if let SsaOp::BuildTuple { elements } = op {
        build::tuple(b, ctx, values, elements)
    } else if let SsaOp::BuildArraySpread { elements } = op {
        build::array_spread(b, ctx, values, elements)
    } else if let SsaOp::BuildObjectSpread { parts } = op {
        build::object_spread(b, ctx, values, parts, inst.cache_slot()?)
    } else if let SsaOp::ObjectMerge { target, source } = op {
        build::object_merge(b, ctx, values, *target, *source)
    } else if let SsaOp::ObjectRest { object, skip_keys } = op {
        build::object_rest(b, ctx, values, *object, skip_keys)
    } else if let SsaOp::GetPropertyMaybe { object, name } = op {
        props::property_maybe(b, ctx, values, *object, name)
    } else if let SsaOp::AssertNotNull { operand } = op {
        props::assert_not_null(b, ctx, values, *operand)
    } else if let SsaOp::GetSlotMethod { object, slot } = op {
        props::slot_method(b, ctx, values, *object, *slot)
    } else if let SsaOp::GetSuperSlot { slot } = op {
        props::super_slot(b, ctx, *slot)
    } else if let SsaOp::Range {
        start,
        end,
        inclusive,
    } = op
    {
        props::range(b, ctx, values, *start, *end, *inclusive)
    } else if let SsaOp::SuperCall { args } = op {
        invoke::super_call(b, ctx, values, args)
    } else if let SsaOp::SuperMethodCall { name, args } = op {
        invoke::super_method_call(b, ctx, values, name, args)
    } else if let SsaOp::SuperInvokeSlot { slot, args } = op {
        invoke::super_invoke_slot(b, ctx, values, *slot, args)
    } else if let SsaOp::ExtensionCall {
        func,
        slot,
        recv,
        args,
    } = op
    {
        invoke::extension_call(b, ctx, values, func, *slot, *recv, args)
    } else if let SsaOp::CallSpread { callee, args } = op {
        invoke::call_spread(b, ctx, values, *callee, args)
    } else if let SsaOp::LoadModule { source } = op {
        let resume = inst.resume_point()?;
        modules::load_module(b, ctx, values, source, resume.ip, &resume.live)
    } else if let SsaOp::ModuleSlot { object, slot } = op {
        modules::module_slot(b, ctx, values, *object, *slot)
    } else if let SsaOp::StoreModuleSlot { slot, value } = op {
        modules::store_module_slot(b, ctx, values, *slot, *value)
    } else if let SsaOp::Await { operand } = op {
        let resume = inst.resume_point()?;
        suspend::await_(b, ctx, values, *operand, resume.ip, &resume.live, dest)
    } else if let SsaOp::Spawn { operand } = op {
        suspend::spawn(b, ctx, values, *operand)
    } else if let SsaOp::Yield { operand } = op {
        let resume = inst.resume_point()?;
        suspend::yield_(b, ctx, values, *operand, resume.ip, &resume.live, dest)
    } else {
        Ok(None)
    }
}
