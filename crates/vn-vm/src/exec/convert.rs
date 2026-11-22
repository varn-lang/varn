use crate::error::{RuntimeError, VmResult};
use crate::heap::{Heap, HeapObj};
use crate::value::VmValue;
use bigdecimal::{BigDecimal, RoundingMode};
use num_bigint::BigInt;
use num_traits::{FromPrimitive, ToPrimitive};
use vn_core::{LangPrimitive, ValueConv};

#[cold]
#[inline(never)]
fn out_of_int_range(what: impl std::fmt::Display) -> RuntimeError {
    RuntimeError::integer_overflow(format!("integer overflow: {what} does not fit int"))
}

fn float_to_int(f: f64) -> VmResult<VmValue> {
    vn_core::float_to_int(f)
        .map(VmValue::from_int)
        .ok_or_else(|| out_of_int_range(f))
}

enum Numeric {
    Int(i64),
    Float(f64),
    Big(BigInt),
    Dec(BigDecimal),
}

fn numeric_of(v: VmValue, heap: &Heap) -> VmResult<Numeric> {
    if heap.is_int(v) {
        return Ok(Numeric::Int(heap.as_int(v)));
    }
    if v.is_f64() {
        return Ok(Numeric::Float(v.as_f64()));
    }
    let obj = if v.is_heap() {
        heap.get(v.as_heap())
    } else {
        None
    };
    match obj {
        Some(HeapObj::BigInt(b)) => Ok(Numeric::Big((**b).clone())),
        Some(HeapObj::Decimal(d)) => Ok(Numeric::Dec((**d).clone())),
        Some(
            HeapObj::Str(_)
            | HeapObj::Array(_)
            | HeapObj::Tuple(_)
            | HeapObj::Object(_)
            | HeapObj::Buffer(_)
            | HeapObj::Module(_)
            | HeapObj::FrozenModule(_)
            | HeapObj::VmClosure(_)
            | HeapObj::Class(_)
            | HeapObj::NativeFn(..)
            | HeapObj::BoundMethod(_)
            | HeapObj::Map(_)
            | HeapObj::Set(_)
            | HeapObj::Task(_)
            | HeapObj::TaskHandle(_)
            | HeapObj::Range(_)
            | HeapObj::EnumVariant(_)
            | HeapObj::Generator(_)
            | HeapObj::Spread(_),
        )
        | None => Err(RuntimeError::type_error(format!(
            "type mismatch: a number conversion needs a number, got '{}'",
            crate::exec::advanced::kind_of_val(v, heap)
        ))),
    }
}

fn to_int(n: Numeric) -> VmResult<VmValue> {
    match n {
        Numeric::Int(i) => Ok(VmValue::from_int(i)),
        Numeric::Float(f) => float_to_int(f),
        Numeric::Big(b) => b
            .to_i64()
            .map(VmValue::from_int)
            .ok_or_else(|| out_of_int_range(b)),
        Numeric::Dec(d) => d
            .with_scale_round(0, RoundingMode::Down)
            .to_i64()
            .map(VmValue::from_int)
            .ok_or_else(|| out_of_int_range(d)),
    }
}

fn to_float(n: Numeric) -> VmValue {
    let f = match n {
        Numeric::Int(i) => i as f64,
        Numeric::Float(f) => f,
        Numeric::Big(b) => b.to_f64().unwrap_or(f64::NAN),
        Numeric::Dec(d) => d.to_f64().unwrap_or(f64::NAN),
    };
    VmValue::from_f64(f)
}

fn to_bigint(n: Numeric, heap: &mut Heap) -> VmResult<VmValue> {
    let b = match n {
        Numeric::Int(i) => BigInt::from(i),
        Numeric::Big(b) => b,
        Numeric::Float(f) => BigInt::from_f64(f.trunc()).ok_or_else(|| {
            RuntimeError::integer_overflow(format!("integer overflow: {f} has no bigint value"))
        })?,
        Numeric::Dec(d) => {
            d.with_scale_round(0, RoundingMode::Down)
                .as_bigint_and_exponent()
                .0
        }
    };
    Ok(heap.alloc_bigint(b))
}

fn to_decimal(n: Numeric, heap: &mut Heap) -> VmResult<VmValue> {
    let d = match n {
        Numeric::Int(i) => BigDecimal::from(i),
        Numeric::Big(b) => BigDecimal::from(b),
        Numeric::Dec(d) => d,
        Numeric::Float(f) => BigDecimal::from_f64(f)
            .ok_or_else(|| RuntimeError::new(format!("cannot convert {f} to decimal")))?,
    };
    Ok(heap.alloc_decimal(d))
}

#[cold]
#[inline(never)]
fn not_a(expected: LangPrimitive, nullable: bool, v: VmValue, heap: &Heap) -> RuntimeError {
    let optional = if nullable { "?" } else { "" };
    RuntimeError::type_error(format!(
        "type mismatch: expected '{}{optional}', got '{}'",
        expected.name(),
        crate::exec::advanced::kind_of_val(v, heap)
    ))
}

fn u64_value(n: u64, heap: &mut Heap) -> VmValue {
    match i64::try_from(n) {
        Ok(small) => VmValue::from_int(small),
        Err(_) => heap.alloc_bigint(BigInt::from(n)),
    }
}

fn u64_overflow(what: impl std::fmt::Display) -> RuntimeError {
    RuntimeError::integer_overflow(format!("integer overflow: {what} does not fit u64"))
}

fn u64_of(n: Numeric) -> VmResult<u64> {
    match n {
        Numeric::Int(i) => u64::try_from(i).map_err(|_| u64_overflow(i)),
        Numeric::Big(b) => b.to_u64().ok_or_else(|| u64_overflow(b)),
        Numeric::Float(_) | Numeric::Dec(_) => Err(RuntimeError::new("convert: not an integer")),
    }
}

fn narrow_u64(v: VmValue, heap: &mut Heap) -> VmResult<VmValue> {
    let n = numeric_of(v, heap)?;
    Ok(u64_value(u64_of(n)?, heap))
}

fn u64_to_bigint(v: VmValue, heap: &mut Heap) -> VmResult<VmValue> {
    match numeric_of(v, heap)? {
        Numeric::Big(_) => Ok(v),
        n @ (Numeric::Int(_) | Numeric::Float(_) | Numeric::Dec(_)) => {
            Ok(heap.alloc_bigint(BigInt::from(u64_of(n)?)))
        }
    }
}

fn u64_to_bits(v: VmValue, heap: &Heap) -> VmResult<VmValue> {
    Ok(VmValue::from_int(u64_of(numeric_of(v, heap)?)? as i64))
}

fn u64_from_bits(v: VmValue, heap: &mut Heap) -> VmResult<VmValue> {
    match numeric_of(v, heap)? {
        Numeric::Int(bits) => Ok(u64_value(bits as u64, heap)),
        Numeric::Float(_) | Numeric::Big(_) | Numeric::Dec(_) => {
            Err(RuntimeError::new("convert: not a bit pattern"))
        }
    }
}

fn narrow(conv: ValueConv, v: VmValue, heap: &Heap) -> VmResult<VmValue> {
    match conv.narrowed() {
        Some(vn_core::NumKind::Int(kind)) => {
            if !heap.is_int(v) {
                return Err(not_a(LangPrimitive::INT, false, v, heap));
            }
            let value = heap.as_int(v);
            if kind.contains(i128::from(value)) {
                Ok(v)
            } else {
                Err(RuntimeError::integer_overflow(format!(
                    "integer overflow: {value} does not fit {}",
                    kind.name()
                )))
            }
        }
        Some(vn_core::NumKind::Float(_)) => {
            let value = if heap.is_int(v) {
                heap.as_int(v) as f64
            } else if v.is_f64() {
                v.as_f64()
            } else {
                return Err(not_a(LangPrimitive::FLOAT, false, v, heap));
            };
            Ok(VmValue::from_f64(f64::from(value as f32)))
        }
        None => Err(RuntimeError::new("convert: not a narrowing conversion")),
    }
}

pub(crate) fn convert(conv: ValueConv, v: VmValue, heap: &mut Heap) -> VmResult<VmValue> {
    match conv {
        ValueConv::NarrowI8
        | ValueConv::NarrowI16
        | ValueConv::NarrowI32
        | ValueConv::NarrowU8
        | ValueConv::NarrowU16
        | ValueConv::NarrowU32
        | ValueConv::NarrowF32 => return narrow(conv, v, heap),
        ValueConv::NarrowU64 => return narrow_u64(v, heap),
        ValueConv::U64ToBigInt => return u64_to_bigint(v, heap),
        ValueConv::U64ToBits => return u64_to_bits(v, heap),
        ValueConv::U64FromBits => return u64_from_bits(v, heap),
        ValueConv::ExpectRef => {
            return if v.is_heap() || v.is_null() {
                Ok(v)
            } else {
                Err(RuntimeError::type_error(format!(
                    "type mismatch: expected '{}', got '{}'",
                    vn_core::RuntimeKind::Object.name(),
                    crate::exec::advanced::kind_of_val(v, heap)
                )))
            };
        }
        ValueConv::ExpectStr | ValueConv::ExpectStrOrNull => {
            let nullable = conv == ValueConv::ExpectStrOrNull;
            return if heap.is_string(v) || (nullable && v.is_null()) {
                Ok(v)
            } else {
                Err(not_a(LangPrimitive::Str, nullable, v, heap))
            };
        }
        ValueConv::ExpectBool | ValueConv::ExpectBoolOrNull => {
            let nullable = conv == ValueConv::ExpectBoolOrNull;
            return if v.is_bool() || (nullable && v.is_null()) {
                Ok(v)
            } else {
                Err(not_a(LangPrimitive::Bool, nullable, v, heap))
            };
        }
        ValueConv::ExpectChar => {
            return if v.is_char() {
                Ok(v)
            } else {
                Err(not_a(LangPrimitive::Char, false, v, heap))
            };
        }
        ValueConv::ExpectInt | ValueConv::ExpectIntOrNull => {
            let nullable = conv == ValueConv::ExpectIntOrNull;
            return if heap.is_int(v) || (nullable && v.is_null()) {
                Ok(v)
            } else {
                Err(not_a(LangPrimitive::INT, nullable, v, heap))
            };
        }
        ValueConv::ExpectFloat | ValueConv::ExpectFloatOrNull => {
            let nullable = conv == ValueConv::ExpectFloatOrNull;
            return if v.is_f64() || (nullable && v.is_null()) {
                Ok(v)
            } else if heap.is_int(v) {
                Ok(VmValue::from_f64(heap.as_int(v) as f64))
            } else {
                Err(not_a(LangPrimitive::FLOAT, nullable, v, heap))
            };
        }
        ValueConv::IntToFloat
        | ValueConv::FloatToInt
        | ValueConv::IntToBigInt
        | ValueConv::BigIntToInt
        | ValueConv::IntToDecimal
        | ValueConv::DecimalToInt
        | ValueConv::DynToInt
        | ValueConv::DynToFloat
        | ValueConv::BigIntToFloat
        | ValueConv::FloatToBigInt
        | ValueConv::DecimalToFloat
        | ValueConv::FloatToDecimal
        | ValueConv::BigIntToDecimal
        | ValueConv::DecimalToBigInt => {}
    }
    let n = numeric_of(v, heap)?;
    match conv {
        ValueConv::ExpectStr
        | ValueConv::ExpectBool
        | ValueConv::ExpectStrOrNull
        | ValueConv::ExpectBoolOrNull
        | ValueConv::ExpectInt
        | ValueConv::ExpectFloat
        | ValueConv::ExpectIntOrNull
        | ValueConv::ExpectFloatOrNull
        | ValueConv::ExpectRef
        | ValueConv::ExpectChar => Ok(v),
        ValueConv::NarrowI8
        | ValueConv::NarrowI16
        | ValueConv::NarrowI32
        | ValueConv::NarrowU8
        | ValueConv::NarrowU16
        | ValueConv::NarrowU32
        | ValueConv::NarrowF32 => narrow(conv, v, heap),
        ValueConv::NarrowU64 => narrow_u64(v, heap),
        ValueConv::U64ToBigInt => u64_to_bigint(v, heap),
        ValueConv::U64ToBits => u64_to_bits(v, heap),
        ValueConv::U64FromBits => u64_from_bits(v, heap),
        ValueConv::FloatToInt
        | ValueConv::BigIntToInt
        | ValueConv::DecimalToInt
        | ValueConv::DynToInt => to_int(n),
        ValueConv::IntToFloat
        | ValueConv::DynToFloat
        | ValueConv::BigIntToFloat
        | ValueConv::DecimalToFloat => Ok(to_float(n)),
        ValueConv::IntToBigInt | ValueConv::FloatToBigInt | ValueConv::DecimalToBigInt => {
            to_bigint(n, heap)
        }
        ValueConv::IntToDecimal | ValueConv::FloatToDecimal | ValueConv::BigIntToDecimal => {
            to_decimal(n, heap)
        }
    }
}
