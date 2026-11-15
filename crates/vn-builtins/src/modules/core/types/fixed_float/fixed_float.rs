use vn_macros::natives;
use vn_types::NativeError;

natives! {
    class $T {
        MAX_VALUE => T::MAX,
        LOWEST => T::MIN,
        MIN_POSITIVE => T::MIN_POSITIVE,
        NAN => T::NAN,
        INFINITY => T::INFINITY,
        NEG_INFINITY => T::NEG_INFINITY,
        EPSILON => T::EPSILON,
        parse(s) => {
            s.trim().parse::<T>().map_err(|e| {
                NativeError::std(vn_core::StdErrorClass::Parse, format!("{}.parse({s:?}): {e}", K.name()))
            })
        },
        tryParse(s) => s.trim().parse::<T>().ok(),
        isNaN(this) => this.is_nan(),
        isFinite(this) => this.is_finite(),
        isInfinite(this) => this.is_infinite(),

        toString(this) => vn_core::float_to_string(f64::from(this)),
        valueOf(this) => this,
        toFixed(this, digits) => {
            let d = vn_core::fixed_digits(digits).ok_or_else(|| {
                NativeError::range_error(format!("toFixed: digits must be within 0..={}", vn_core::FIXED_MAX_DIGITS))
            })?;
            Ok(vn_core::float_to_fixed(f64::from(this), d))
        },
        abs(this) => this.abs(),
        sign(this) => vn_core::fsign(f64::from(this)) as T,
        negate(this) => -this,
        min(this, other) => vn_core::fmin(f64::from(this), f64::from(other)) as T,
        max(this, other) => vn_core::fmax(f64::from(this), f64::from(other)) as T,
        clamp(this, lo, hi) => {
            if lo > hi {
                return Err(NativeError::range_error(format!("clamp: the lower bound {lo} is above the upper bound {hi}")));
            }
            Ok(vn_core::fmin(vn_core::fmax(f64::from(this), f64::from(lo)), f64::from(hi)) as T)
        },
        pow(this, exponent) => this.powf(exponent),
        isInteger(this) => this.is_finite() && this.fract() == 0.0,
        floor(this) => this.floor(),
        ceil(this) => this.ceil(),
        round(this) => this.round(),
        trunc(this) => this.trunc(),
        toFloat(this) => f64::from(this),
        toInt(this) => {
            vn_core::float_to_int(f64::from(this))
                .ok_or_else(|| NativeError::integer_overflow(format!("integer overflow: {this} does not fit int")))
        },
    }
}
