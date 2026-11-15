use vn_macros::natives;
use vn_types::{NativeCtx, NativeFnResult, VmValue};


natives! {
    class bool {
        toString(this) => if this { "true".to_string() } else { "false".to_string() },
        valueOf(this) => this,
    }
}

pub fn boolean_to_string(ctx: &mut dyn NativeCtx, args: &[VmValue]) -> NativeFnResult {
    let v = args.first().copied().unwrap_or(VmValue::null());
    Ok(ctx.alloc_str(if v.as_bool() { "true" } else { "false" }))
}
