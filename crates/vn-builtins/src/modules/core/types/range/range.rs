use vn_macros::natives;
use vn_types::value::{RangeData, RangeElem};
use vn_types::{NativeCtx, NativeError, VmValue};


fn get_range(ctx: &dyn NativeCtx, this: VmValue) -> Option<RangeData> {
    ctx.as_range(this)
}

const MAX_MATERIALIZED: u128 = 1 << 28;

fn length_of(r: &RangeData) -> Result<i64, NativeError> {
    r.len().ok_or_else(|| {
        NativeError::integer_overflow(format!("integer overflow: the length of {r} does not fit int"))
    })
}

fn elements(ctx: &mut dyn NativeCtx, r: &RangeData) -> Result<Vec<VmValue>, NativeError> {
    if r.count() > MAX_MATERIALIZED {
        return Err(NativeError::range_error(format!(
            "{r} has {} elements, more than the {MAX_MATERIALIZED} that can be materialized",
            r.count()
        )));
    }
    let len = length_of(r)?;
    Ok((0..len)
        .filter_map(|i| r.nth(i))
        .map(|raw| ctx.range_element(r, raw))
        .collect())
}

fn raw_of(ctx: &dyn NativeCtx, r: &RangeData, val: VmValue) -> Option<i64> {
    match r.elem {
        RangeElem::Int => ctx.is_int(val).then(|| ctx.as_int(val)),
        RangeElem::Char => val.as_char().map(|c| c as i64),
    }
}

natives! {
    class Range {
        start(ctx, this) => {
            match get_range(ctx, this) {
                Some(r) => ctx.range_element(&r, r.start),
                None => VmValue::null(),
            }
        },
        end(ctx, this) => {
            match get_range(ctx, this) {
                Some(r) => ctx.range_element(&r, r.end),
                None => VmValue::null(),
            }
        },
        inclusive(ctx, this) => get_range(ctx, this).map(|r| r.inclusive).unwrap_or(false),
        length(ctx, this) => get_range(ctx, this).map_or(Ok(0), |r| length_of(&r)),
        stride(ctx, this) => get_range(ctx, this).map(|r| r.step).unwrap_or(1),
        toString(ctx, this) => get_range(ctx, this).map(|r| r.to_string()).unwrap_or_default(),
        contains(ctx, this, val) => {
            match get_range(ctx, this) {
                Some(r) => raw_of(ctx, &r, val).is_some_and(|raw| r.contains(raw)),
                None => false,
            }
        },
        toArray(ctx, this) => {
            match get_range(ctx, this) {
                Some(r) => elements(ctx, &r),
                None => Ok(Vec::new()),
            }
        },
        step(ctx, this, n) => {
            if n <= 0 {
                return Err(NativeError::range_error(format!("range step must be positive, got {n}")));
            }
            let r = get_range(ctx, this).ok_or_else(|| NativeError::type_error("range.step: not a range"))?;
            let step = r.step.checked_mul(n).ok_or_else(|| {
                NativeError::integer_overflow(format!("integer overflow: the step {} * {n} does not fit int", r.step))
            })?;
            Ok(ctx.alloc_range_data(r.with_step(step)))
        },
        forEach(ctx, this, callback) => {
            if let Some(r) = get_range(ctx, this) {
                for item in elements(ctx, &r)? {
                    ctx.call_vm(callback, &[item])?;
                }
            }
            Ok(())
        },
        map(ctx, this, callback) => {
            let Some(r) = get_range(ctx, this) else {
                return Ok(Vec::new());
            };
            elements(ctx, &r)?
                .into_iter()
                .map(|item| ctx.call_vm(callback, &[item]))
                .collect::<Result<Vec<_>, _>>()
        },
        from(ctx, start, end) => ctx.alloc_range(start, end, false),
        fromInclusive(ctx, start, end) => ctx.alloc_range(start, end, true),
    }
}
