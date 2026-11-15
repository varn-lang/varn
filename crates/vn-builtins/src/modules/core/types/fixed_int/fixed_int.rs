use std::fmt::Display;
use vn_macros::natives;
use vn_types::NativeError;

#[derive(Clone, Copy)]
enum Division {
    Truncate,
    Floor,
    Ceil,
    Remainder,
    Euclid,
}

fn overflow(op: &str, value: impl Display, kind: vn_core::NumKind) -> NativeError {
    NativeError::integer_overflow(format!(
        "integer overflow: {op}({value}) is outside {} ({})",
        kind.name(),
        kind.range_text()
    ))
}

fn narrow<T: TryFrom<i128>>(op: &str, wide: i128, kind: vn_core::NumKind) -> Result<T, NativeError> {
    T::try_from(wide).map_err(|_| overflow(op, wide, kind))
}

fn divide(op: &str, how: Division, a: i128, b: i128) -> Result<i128, NativeError> {
    if b == 0 {
        return Err(NativeError::division_by_zero(format!("{op}: division by zero")));
    }
    let quotient = a / b;
    let inexact = a % b != 0;
    let opposite = (a < 0) != (b < 0);
    Ok(match how {
        Division::Truncate => quotient,
        Division::Floor if inexact && opposite => quotient - 1,
        Division::Floor => quotient,
        Division::Ceil if inexact && !opposite => quotient + 1,
        Division::Ceil => quotient,
        Division::Remainder => a % b,
        Division::Euclid => a.rem_euclid(b),
    })
}

fn digits(value: i128, radix: u32) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let magnitude = value.unsigned_abs();
    match radix {
        16 => format!("{sign}{magnitude:x}"),
        8 => format!("{sign}{magnitude:o}"),
        _ => format!("{sign}{magnitude:b}"),
    }
}

fn count(op: &str, amount: i64) -> Result<u32, NativeError> {
    u32::try_from(amount).map_err(|_| {
        NativeError::range_error(format!("{op}: the amount must not be negative, got {amount}"))
    })
}

natives! {
    class $T {
        MIN_VALUE => T::MIN,
        MAX_VALUE => T::MAX,
        BITS => i64::from(T::BITS),
        parse(s) => {
            s.trim().parse::<T>().map_err(|e| {
                NativeError::std(vn_core::StdErrorClass::Parse, format!("{}.parse({s:?}): {e}", K.name()))
            })
        },
        tryParse(s) => s.trim().parse::<T>().ok(),

        toString(this) => this.to_string(),
        valueOf(this) => this,
        toHex(this) => digits(i128::from(this), 16),
        toBinary(this) => digits(i128::from(this), 2),
        toOctal(this) => digits(i128::from(this), 8),
        toInt(this) => i64::try_from(this).map_err(|_| overflow("toInt", this, K)),
        toFloat(this) => this as f64,
        abs(this) => this.checked_abs().ok_or_else(|| overflow("abs", this, K)),
        negate(this) => this.checked_neg().ok_or_else(|| overflow("negate", this, K)),
        sign(this) => i64::from(this.signum()),
        min(this, other) => this.min(other),
        max(this, other) => this.max(other),
        clamp(this, lo, hi) => {
            if lo > hi {
                return Err(NativeError::range_error(format!("clamp: the lower bound {lo} is above the upper bound {hi}")));
            }
            Ok(this.max(lo).min(hi))
        },
        pow(this, exponent) => {
            let e = u32::try_from(exponent).map_err(|_| {
                NativeError::range_error(format!("pow: the exponent must not be negative, got {exponent}"))
            })?;
            this.checked_pow(e).ok_or_else(|| overflow("pow", this, K))
        },
        isEven(this) => this % 2 == 0,
        isOdd(this) => this % 2 != 0,

        countOnes(this) => i64::from(this.count_ones()),
        leadingZeros(this) => i64::from(this.leading_zeros()),
        trailingZeros(this) => i64::from(this.trailing_zeros()),
        rotateLeft(this, amount) => Ok(this.rotate_left(count("rotateLeft", amount)?)),
        rotateRight(this, amount) => Ok(this.rotate_right(count("rotateRight", amount)?)),
        swapBytes(this) => this.swap_bytes(),

        wrappingAdd(this, other) => this.wrapping_add(other),
        wrappingSub(this, other) => this.wrapping_sub(other),
        wrappingMul(this, other) => this.wrapping_mul(other),
        saturatingAdd(this, other) => this.saturating_add(other),
        saturatingSub(this, other) => this.saturating_sub(other),
        saturatingMul(this, other) => this.saturating_mul(other),
        checkedAdd(this, other) => this.checked_add(other),
        checkedSub(this, other) => this.checked_sub(other),
        checkedMul(this, other) => this.checked_mul(other),
        div(this, other) => narrow::<T>("div", divide("div", Division::Truncate, i128::from(this), i128::from(other))?, K),
        floorDiv(this, other) => narrow::<T>("floorDiv", divide("floorDiv", Division::Floor, i128::from(this), i128::from(other))?, K),
        ceilDiv(this, other) => narrow::<T>("ceilDiv", divide("ceilDiv", Division::Ceil, i128::from(this), i128::from(other))?, K),
        rem(this, other) => narrow::<T>("rem", divide("rem", Division::Remainder, i128::from(this), i128::from(other))?, K),
        r#mod(this, other) => narrow::<T>("mod", divide("mod", Division::Euclid, i128::from(this), i128::from(other))?, K),
    }
}
