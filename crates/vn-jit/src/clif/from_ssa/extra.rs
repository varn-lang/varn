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
    match op {
        SsaOp::LoadGlobal(name) => modules::load_global(b, ctx, name),
        SsaOp::StoreGlobal { name, value } => modules::store_global(b, ctx, values, name, *value),
        SsaOp::BuildTuple { elements } => build::tuple(b, ctx, values, elements),
        SsaOp::BuildArraySpread { elements } => build::array_spread(b, ctx, values, elements),
        SsaOp::BuildObjectSpread { parts } => {
            build::object_spread(b, ctx, values, parts, inst.cache_slot()?)
        }
        SsaOp::ObjectMerge { target, source } => {
            build::object_merge(b, ctx, values, *target, *source)
        }
        SsaOp::ObjectRest { object, skip_keys } => {
            build::object_rest(b, ctx, values, *object, skip_keys)
        }
        SsaOp::GetPropertyMaybe { object, name } => {
            props::property_maybe(b, ctx, values, *object, name)
        }
        SsaOp::AssertNotNull { operand } => props::assert_not_null(b, ctx, values, *operand),
        SsaOp::BindMethod { object, name } => props::bind_method(b, ctx, values, *object, name),
        SsaOp::GetSlotMethod { object, slot } => props::slot_method(b, ctx, values, *object, *slot),
        SsaOp::GetSuperSlot { slot } => props::super_slot(b, ctx, *slot),
        SsaOp::ArrayExtend { array, source } => {
            props::array_extend(b, ctx, values, *array, *source)
        }
        SsaOp::WrapSpread { operand } => props::wrap_spread(b, ctx, values, *operand),
        SsaOp::Range {
            start,
            end,
            inclusive,
        } => props::range(b, ctx, values, *start, *end, *inclusive),
        SsaOp::SuperCall { args } => invoke::super_call(b, ctx, values, args),
        SsaOp::SuperMethodCall { name, args } => {
            invoke::super_method_call(b, ctx, values, name, args)
        }
        SsaOp::SuperInvokeSlot { slot, args } => {
            invoke::super_invoke_slot(b, ctx, values, *slot, args)
        }
        SsaOp::ExtensionCall {
            func,
            slot,
            recv,
            args,
        } => invoke::extension_call(b, ctx, values, func, *slot, *recv, args),
        SsaOp::CallSpread { callee, args } => invoke::call_spread(b, ctx, values, *callee, args),
        SsaOp::LoadModule { source } => {
            let resume = inst.resume_point()?;
            modules::load_module(b, ctx, values, source, resume.ip, &resume.live)
        }
        SsaOp::ModuleSlot { object, slot } => modules::module_slot(b, ctx, values, *object, *slot),
        SsaOp::StoreModuleSlot { slot, value } => {
            modules::store_module_slot(b, ctx, values, *slot, *value)
        }
        SsaOp::Await { operand } => {
            let resume = inst.resume_point()?;
            suspend::await_(b, ctx, values, *operand, resume.ip, &resume.live, dest)
        }
        SsaOp::Spawn { operand } => suspend::spawn(b, ctx, values, *operand),
        SsaOp::Yield { operand } => {
            let resume = inst.resume_point()?;
            suspend::yield_(b, ctx, values, *operand, resume.ip, &resume.live, dest)
        }
        SsaOp::Dispose { var, is_await } => {
            suspend::dispose(b, ctx, values, *var, *is_await, inst.cache_slot()?)
        }
        SsaOp::ConstInt(_)
        | SsaOp::ConstFloat(_)
        | SsaOp::ConstBool(_)
        | SsaOp::ConstNull
        | SsaOp::ConstStr(_)
        | SsaOp::Binary { .. }
        | SsaOp::Unary { .. }
        | SsaOp::SelfCall { .. }
        | SsaOp::Call { .. }
        | SsaOp::CallDirect { .. }
        | SsaOp::AllocInstance { .. }
        | SsaOp::LoadGlobalIdx(_)
        | SsaOp::ArrayGetIndex { .. }
        | SsaOp::BytesGetIndex { .. }
        | SsaOp::ArraySetIndex { .. }
        | SsaOp::BytesSetIndex { .. }
        | SsaOp::ConstChar(_)
        | SsaOp::ConstBigInt(_)
        | SsaOp::ConstDecimal(_)
        | SsaOp::MakeEnumVariant { .. }
        | SsaOp::IntrinsicCall { .. }
        | SsaOp::LoadNativeGlobalIdx(_)
        | SsaOp::Try { .. }
        | SsaOp::PopTry
        | SsaOp::CatchParam { .. }
        | SsaOp::StoreGlobalIdx { .. }
        | SsaOp::MakeClosure { .. }
        | SsaOp::LoadCaptured { .. }
        | SsaOp::StoreCaptured { .. }
        | SsaOp::LoadUpvalue(_)
        | SsaOp::StoreUpvalue { .. }
        | SsaOp::CloseUpvalues { .. }
        | SsaOp::Cast { .. }
        | SsaOp::Convert { .. }
        | SsaOp::IsNull { .. }
        | SsaOp::KindOf { .. }
        | SsaOp::TypeTest { .. }
        | SsaOp::ToString { .. }
        | SsaOp::IsArray { .. }
        | SsaOp::GetEnumTag { .. }
        | SsaOp::ObjectKeys { .. }
        | SsaOp::BuildStr { .. }
        | SsaOp::BuildArray { .. }
        | SsaOp::BuildMap { .. }
        | SsaOp::BuildObject { .. }
        | SsaOp::GetProperty { .. }
        | SsaOp::SetProperty { .. }
        | SsaOp::GetIndex { .. }
        | SsaOp::SetIndex { .. }
        | SsaOp::ArrayLength { .. }
        | SsaOp::StrLength { .. }
        | SsaOp::BytesLength { .. }
        | SsaOp::MethodCall { .. }
        | SsaOp::InvokeSlot { .. }
        | SsaOp::CallNativeOp { .. }
        | SsaOp::ArrayPush { .. }
        | SsaOp::This
        | SsaOp::GetFixedField { .. }
        | SsaOp::SetFixedField { .. }
        | SsaOp::MakeClass { .. }
        | SsaOp::DeclareLayout { .. }
        | SsaOp::FreezeClass { .. }
        | SsaOp::DefineMember { .. }
        | SsaOp::GetSuper { .. } => Ok(None),
    }
}
