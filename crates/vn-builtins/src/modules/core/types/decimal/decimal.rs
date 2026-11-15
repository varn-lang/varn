use bigdecimal::{BigDecimal, RoundingMode};
use num_traits::{Signed, ToPrimitive, Zero};
use vn_macros::natives;
use vn_types::{NativeCtx, NativeError, VmValue};


fn get_decimal(ctx: &dyn NativeCtx, this: VmValue) -> Option<BigDecimal> {
    if ctx.is_int(this) {
        return Some(BigDecimal::from(ctx.as_int(this)));
    }
    ctx.as_decimal(this)
}

fn alloc_decimal(ctx: &mut dyn NativeCtx, d: BigDecimal) -> VmValue {
    ctx.alloc_decimal(d)
}

fn rounded(ctx: &mut dyn NativeCtx, this: VmValue, mode: RoundingMode) -> VmValue {
    match get_decimal(ctx, this) {
        Some(d) => alloc_decimal(ctx, d.with_scale_round(0, mode)),
        None => VmValue::null(),
    }
}

fn not_decimal() -> NativeError {
    NativeError::type_error("decimal: receiver is not a decimal")
}

fn pow_decimal(d: &BigDecimal, exp: i64) -> Result<BigDecimal, NativeError> {
    use vn_core::numeric_big::DecimalPowFault;
    vn_core::numeric_big::pow_decimal(d, &BigDecimal::from(exp)).map_err(|fault| match fault {
        DecimalPowFault::NonIntegral => NativeError::range_error("the exponent of a decimal power must be an integer"),
        DecimalPowFault::TooLarge => NativeError::range_error("the result of this decimal power would have too many digits"),
        DecimalPowFault::ZeroToNegative => NativeError::division_by_zero("zero cannot be raised to a negative power"),
    })
}

natives! {
    class decimal {
        parse(ctx, s) => {
            let d = s
                .trim()
                .parse::<BigDecimal>()
                .map_err(|e| NativeError::std(vn_core::StdErrorClass::Parse, format!("decimal.parse({s:?}): {e}")))?;
            Ok(alloc_decimal(ctx, d))
        },
        tryParse(ctx, s) => {
            let d = s.trim().parse::<BigDecimal>().ok()?;
            Some(alloc_decimal(ctx, d))
        },
        toString(ctx, this) => get_decimal(ctx, this).map(|d| d.to_plain_string()).unwrap_or_else(|| "0".to_string()),
        toFixed(ctx, this, digits) => {
            let places = vn_core::fixed_digits(digits).ok_or_else(|| {
                NativeError::range_error(format!("toFixed: digits must be within 0..={}", vn_core::FIXED_MAX_DIGITS))
            })?;
            Ok(match get_decimal(ctx, this) {
                Some(d) => d.with_scale_round(places as i64, RoundingMode::HalfEven).to_plain_string(),
                None => "0".to_string(),
            })
        },
        abs(ctx, this) => match get_decimal(ctx, this) { Some(d) => alloc_decimal(ctx, d.abs()), None => VmValue::null() },
        sign(ctx, this) => {
            match get_decimal(ctx, this) {
                Some(d) if d.is_zero() => 0,
                Some(d) if d.is_negative() => -1,
                Some(_) => 1,
                None => 0,
            }
        },
        negate(ctx, this) => match get_decimal(ctx, this) { Some(d) => alloc_decimal(ctx, -d), None => VmValue::null() },
        ceil(ctx, this) => rounded(ctx, this, RoundingMode::Ceiling),
        floor(ctx, this) => rounded(ctx, this, RoundingMode::Floor),
        round(ctx, this) => rounded(ctx, this, RoundingMode::HalfEven),
        trunc(ctx, this) => rounded(ctx, this, RoundingMode::Down),
        fract(ctx, this) => {
            match get_decimal(ctx, this) {
                Some(d) => {
                    let whole = d.with_scale_round(0, RoundingMode::Down);
                    alloc_decimal(ctx, d - whole)
                }
                None => VmValue::null(),
            }
        },
        scale(ctx, this) => get_decimal(ctx, this).map(|d| d.as_bigint_and_exponent().1).unwrap_or(0),
        isZero(ctx, this) => get_decimal(ctx, this).map(|d| d.is_zero()).unwrap_or(false),
        isPositive(ctx, this) => get_decimal(ctx, this).map(|d| d.is_positive()).unwrap_or(false),
        isNegative(ctx, this) => get_decimal(ctx, this).map(|d| d.is_negative()).unwrap_or(false),
        pow(ctx, this, exp) => {
            let d = get_decimal(ctx, this).ok_or_else(not_decimal)?;
            let r = pow_decimal(&d, exp)?;
            Ok(alloc_decimal(ctx, r))
        },
        toInt(ctx, this) => {
            let d = get_decimal(ctx, this).ok_or_else(not_decimal)?;
            d.with_scale_round(0, RoundingMode::Down).to_i64().ok_or_else(|| {
                NativeError::integer_overflow(format!("integer overflow: {d} does not fit int"))
            })
        },
        toFloat(ctx, this) => get_decimal(ctx, this).and_then(|d| d.to_f64()).unwrap_or(0.0),
    }
}
