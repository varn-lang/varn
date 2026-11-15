use vn_macros::natives;
use vn_types::{NativeError};


const INT_MAX: i64 = vn_core::INT_MAX;
const INT_MIN: i64 = vn_core::INT_MIN;

natives! {
    class int {
        MAX_VALUE => INT_MAX,
        MIN_VALUE => INT_MIN,
        parse(s) => {
            s.trim()
                .parse::<i64>()
                .map_err(|e| NativeError::std(vn_core::StdErrorClass::Parse, format!("int.parse({s:?}): {e}")))
        },
        tryParse(s) => s.trim().parse::<i64>().ok(),
        isSafeInteger(val) => (INT_MIN..=INT_MAX).contains(&val),
        toString(this) => this.to_string(),
        valueOf(this) => this,
        toLocaleString(this) => this.to_string(),
        toFixed(this, digits) => {
            let d = vn_core::fixed_digits(digits).ok_or_else(|| {
                NativeError::range_error(format!("toFixed: digits must be within 0..={}", vn_core::FIXED_MAX_DIGITS))
            })?;
            Ok(if d == 0 { this.to_string() } else { format!("{this}.{}", "0".repeat(d)) })
        },
        abs(this) => this.checked_abs().ok_or_else(|| overflow_error("abs", this)),
        sign(this) => this.signum(),
        negate(this) => vn_core::neg_int(this).ok_or_else(|| overflow_error("negate", this)),
        bitwiseNot(this) => !this,
        min(this, other) => this.min(other),
        max(this, other) => this.max(other),
        clamp(this, lo, hi) => {
            if lo > hi {
                return Err(NativeError::range_error(format!("clamp: the lower bound {lo} is above the upper bound {hi}")));
            }
            Ok(this.max(lo).min(hi))
        },
        toHex(this) => if this < 0 { format!("-{:x}", (this as i128).unsigned_abs()) } else { format!("{:x}", this) },
        toBinary(this) => if this < 0 { format!("-{:b}", (this as i128).unsigned_abs()) } else { format!("{:b}", this) },
        toOctal(this) => if this < 0 { format!("-{:o}", (this as i128).unsigned_abs()) } else { format!("{:o}", this) },
        toFloat(this) => this as f64,
        pow(this, exponent) => {
            let e = u64::try_from(exponent)
                .map_err(|_| NativeError::range_error(format!("pow: the exponent must not be negative, got {exponent}")))?;
            vn_core::pow_int(this, e).ok_or_else(|| overflow_error("pow", this))
        },
        isEven(this) => this % 2 == 0,
        isOdd(this) => this % 2 != 0,
        wrappingAdd(this, other) => this.wrapping_add(other),
        wrappingSub(this, other) => this.wrapping_sub(other),
        wrappingMul(this, other) => this.wrapping_mul(other),
        saturatingAdd(this, other) => this.saturating_add(other),
        saturatingSub(this, other) => this.saturating_sub(other),
        saturatingMul(this, other) => this.saturating_mul(other),
        checkedAdd(this, other) => vn_core::add_int(this, other),
        checkedSub(this, other) => vn_core::sub_int(this, other),
        checkedMul(this, other) => vn_core::mul_int(this, other),
        div(this, other) => vn_core::div_int(this, other).map_err(|f| div_fault(f, "div", this, other)),
        floorDiv(this, other) => vn_core::floor_div_int(this, other).map_err(|f| div_fault(f, "floorDiv", this, other)),
        ceilDiv(this, other) => vn_core::ceil_div_int(this, other).map_err(|f| div_fault(f, "ceilDiv", this, other)),
        rem(this, other) => vn_core::rem_int(this, other).map_err(|f| div_fault(f, "rem", this, other)),
        r#mod(this, other) => vn_core::mod_int(this, other).map_err(|f| div_fault(f, "mod", this, other)),
    }
}

fn overflow_error(op: &str, v: i64) -> NativeError {
    NativeError::integer_overflow(format!(
        "integer overflow: {op}({v}) is outside int ({INT_MIN}..={INT_MAX})"
    ))
}

fn div_fault(fault: vn_core::IntDivFault, op: &str, a: i64, b: i64) -> NativeError {
    match fault {
        vn_core::IntDivFault::DivisionByZero => {
            NativeError::division_by_zero(format!("{op}: division by zero"))
        }
        vn_core::IntDivFault::Overflow => {
            NativeError::integer_overflow(format!("integer overflow: {a}.{op}({b}) is outside int"))
        }
    }
}
