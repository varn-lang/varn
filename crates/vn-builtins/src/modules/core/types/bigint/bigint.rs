use num_traits::ToPrimitive;
use vn_macros::natives;
use vn_types::{NativeCtx, NativeError, VmValue};


fn get_bigint(ctx: &dyn NativeCtx, this: VmValue) -> Option<num_bigint::BigInt> {
    ctx.as_bigint(this)
}

natives! {
    class bigint {
        toString(ctx, this) => get_bigint(ctx, this).map(|n| n.to_string()).unwrap_or_else(|| "0".to_string()),
        toStr(ctx, this) => get_bigint(ctx, this).map(|n| n.to_string()).unwrap_or_else(|| "0".to_string()),
        toInt(ctx, this) => {
            let n = get_bigint(ctx, this).ok_or_else(|| NativeError::type_error("bigint.toInt: not a bigint"))?;
            n.to_i64().ok_or_else(|| {
                NativeError::integer_overflow(format!("integer overflow: {n} does not fit int"))
            })
        },
        toFloat(ctx, this) => get_bigint(ctx, this).and_then(|n| n.to_f64()).unwrap_or(0.0),
    }
}
