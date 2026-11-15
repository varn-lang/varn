use vn_macros::natives;
use vn_types::{NativeCtx, NativeError, NativeFnResult, VmValue};


natives! {
    class float {
        MAX_VALUE => f64::MAX,
        LOWEST => f64::MIN,
        MIN_POSITIVE => f64::MIN_POSITIVE,
        NAN => f64::NAN,
        INFINITY => f64::INFINITY,
        NEG_INFINITY => f64::NEG_INFINITY,
        EPSILON => f64::EPSILON,
        parse(s) => {
            s.trim()
                .parse::<f64>()
                .map_err(|e| NativeError::std(vn_core::StdErrorClass::Parse, format!("float.parse({s:?}): {e}")))
        },
        tryParse(s) => s.trim().parse::<f64>().ok(),
        isNaN(this) => this.is_nan(),
        isFinite(this) => this.is_finite(),
        isInfinite(this) => this.is_infinite(),
        toString(this) => vn_core::float_to_string(this),
        valueOf(this) => this,
        toFixed(this, digits) => {
            let d = vn_core::fixed_digits(digits).ok_or_else(|| {
                NativeError::range_error(format!("toFixed: digits must be within 0..={}", vn_core::FIXED_MAX_DIGITS))
            })?;
            Ok(vn_core::float_to_fixed(this, d))
        },
        abs(this) => this.abs(),
        sign(this) => vn_core::fsign(this),
        negate(this) => -this,
        min(this, other) => vn_core::fmin(this, other),
        max(this, other) => vn_core::fmax(this, other),
        clamp(this, lo, hi) => {
            if lo > hi {
                return Err(NativeError::range_error(format!("clamp: the lower bound {lo} is above the upper bound {hi}")));
            }
            Ok(vn_core::fmin(vn_core::fmax(this, lo), hi))
        },
        pow(this, exponent) => this.powf(exponent),
        isInteger(this) => this.is_finite() && this.fract() == 0.0,
        floor(this) => this.floor(),
        ceil(this) => this.ceil(),
        round(this) => this.round(),
        trunc(this) => this.trunc(),
        toInt(this) => {
            vn_core::float_to_int(this)
                .ok_or_else(|| NativeError::integer_overflow(format!("integer overflow: {this} does not fit int")))
        },
    }
}

pub fn float_is_finite(_ctx: &mut dyn NativeCtx, args: &[VmValue]) -> NativeFnResult {
    if let Some(&v) = args.first() {
        if v.is_f64() {
            return Ok(VmValue::from_bool(v.as_f64().is_finite()));
        }
        return Ok(VmValue::from_bool(v.is_int()));
    }
    Ok(VmValue::bool_false())
}

pub fn float_is_nan(_ctx: &mut dyn NativeCtx, args: &[VmValue]) -> NativeFnResult {
    if let Some(&v) = args.first() {
        return Ok(VmValue::from_bool(v.is_f64() && v.as_f64().is_nan()));
    }
    Ok(VmValue::bool_false())
}

pub fn float_parse(ctx: &mut dyn NativeCtx, args: &[VmValue]) -> NativeFnResult {
    let s = args.first().map(|&v| ctx.str_repr(v)).unwrap_or_default();
    let f = s.trim().parse::<f64>().unwrap_or(f64::NAN);
    Ok(VmValue::from_f64(f))
}
