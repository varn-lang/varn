use crate::native_ctx::NativeCtx;
use crate::native_error::NativeError;
use crate::vm_value::VmValue;

pub trait FromVm: Sized {
    fn from_vm(ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError>;
}

pub trait IntoVm {
    const STORAGE: Option<vn_core::ArrayStorage> = None;

    fn into_vm(self, ctx: &mut dyn NativeCtx) -> VmValue;
}

pub trait IntoNativeRet {
    type Ok;

    fn into_native_ret(self) -> Result<Self::Ok, NativeError>;
}

impl<T: IntoVm> IntoNativeRet for T {
    type Ok = T;

    #[inline]
    fn into_native_ret(self) -> Result<T, NativeError> {
        Ok(self)
    }
}

impl<T> IntoNativeRet for Result<T, NativeError> {
    type Ok = T;

    #[inline]
    fn into_native_ret(self) -> Result<T, NativeError> {
        self
    }
}

#[inline]
pub fn into_native_ret<T: IntoNativeRet>(value: T) -> Result<T::Ok, NativeError> {
    value.into_native_ret()
}

macro_rules! narrow_int {
    ($($kind:ty),* $(,)?) => {$(
        impl FromVm for $kind {
            #[inline]
            fn from_vm(ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError> {
                let wide = i64::from_vm(ctx, v)?;
                <$kind>::try_from(wide).map_err(|_| {
                    NativeError::integer_overflow(format!(
                        "integer overflow: {wide} does not fit {}",
                        stringify!($kind)
                    ))
                })
            }
        }
    )*};
}

narrow_int!(i8, i16, i32, u8, u16, u32);

macro_rules! narrow_int_into {
    ($($kind:ty),* $(,)?) => {$(
        impl IntoVm for $kind {
            const STORAGE: Option<vn_core::ArrayStorage> = Some(vn_core::ArrayStorage::Int);
            #[inline]
            fn into_vm(self, _ctx: &mut dyn NativeCtx) -> VmValue {
                VmValue::from_int(i64::from(self))
            }
        }
    )*};
}

narrow_int_into!(i8, i16, u8, u16, u32);

impl FromVm for u64 {
    fn from_vm(ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError> {
        if v.is_int() {
            let value = v.as_int();
            return u64::try_from(value).map_err(|_| {
                NativeError::integer_overflow(format!("integer overflow: {value} does not fit u64"))
            });
        }
        ctx.as_bigint(v)
            .ok_or_else(|| NativeError::type_error(format!("expected u64, got {v:?}")))
            .and_then(|big| {
                u64::try_from(&big).map_err(|_| {
                    NativeError::integer_overflow(format!(
                        "integer overflow: {big} does not fit u64"
                    ))
                })
            })
    }
}

impl IntoVm for u64 {
    fn into_vm(self, ctx: &mut dyn NativeCtx) -> VmValue {
        match i64::try_from(self) {
            Ok(small) => VmValue::from_int(small),
            Err(_) => ctx.alloc_bigint(num_bigint::BigInt::from(self)),
        }
    }
}

impl FromVm for f32 {
    #[inline]
    fn from_vm(ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError> {
        f64::from_vm(ctx, v).map(|wide| wide as f32)
    }
}

impl IntoVm for f32 {
    const STORAGE: Option<vn_core::ArrayStorage> = Some(vn_core::ArrayStorage::Float);

    #[inline]
    fn into_vm(self, _ctx: &mut dyn NativeCtx) -> VmValue {
        VmValue::from_f64(f64::from(self))
    }
}

impl FromVm for VmValue {
    #[inline]
    fn from_vm(_ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError> {
        Ok(v)
    }
}

impl FromVm for i64 {
    #[inline]
    fn from_vm(_ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError> {
        if v.is_int() {
            Ok(v.as_int())
        } else {
            Err(NativeError::type_error(format!("expected int, got {v:?}")))
        }
    }
}

impl FromVm for f64 {
    #[inline]
    fn from_vm(_ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError> {
        if v.is_f64() || v.is_int() {
            Ok(v.to_f64())
        } else {
            Err(NativeError::type_error(format!(
                "expected float, got {v:?}"
            )))
        }
    }
}

impl FromVm for bool {
    #[inline]
    fn from_vm(_ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError> {
        if v.is_bool() {
            Ok(v.as_bool())
        } else {
            Err(NativeError::type_error(format!("expected bool, got {v:?}")))
        }
    }
}

impl FromVm for String {
    #[inline]
    fn from_vm(ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError> {
        ctx.str_owned(v)
            .ok_or_else(|| NativeError::type_error(format!("expected str, got {v:?}")))
    }
}

pub enum VnStr {
    Sso {
        buf: [u8; crate::vm_value::SSO_MAX_LEN],
        len: u8,
        ascii: bool,
    },
    Shared(std::sync::Arc<str>, bool),
}

impl VnStr {
    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            VnStr::Sso { buf, len, .. } => unsafe {
                std::str::from_utf8_unchecked(&buf[..*len as usize])
            },
            VnStr::Shared(s, _) => s,
        }
    }

    #[inline]
    pub fn is_ascii(&self) -> bool {
        match self {
            VnStr::Sso { ascii, .. } => *ascii,
            VnStr::Shared(_, ascii) => *ascii,
        }
    }

    #[inline]
    pub fn char_code_at(&self, pos: i64) -> i64 {
        if pos < 0 {
            return -1;
        }
        let pos = pos as usize;
        let s = self.as_str();
        if self.is_ascii() {
            return s.as_bytes().get(pos).map(|&b| b as i64).unwrap_or(-1);
        }
        s.chars().nth(pos).map(|c| c as i64).unwrap_or(-1)
    }
}

impl std::ops::Deref for VnStr {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for VnStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromVm for VnStr {
    #[inline]
    fn from_vm(ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError> {
        if v.is_sso() {
            let mut buf = [0u8; crate::vm_value::SSO_MAX_LEN];
            let len = v.sso_as_str(&mut buf).len() as u8;
            let ascii = buf[..len as usize].is_ascii();
            return Ok(VnStr::Sso { buf, len, ascii });
        }
        ctx.str_shared_cached(v)
            .map(|(s, ascii)| VnStr::Shared(s, ascii))
            .ok_or_else(|| NativeError::type_error(format!("expected str, got {v:?}")))
    }
}

impl FromVm for char {
    #[inline]
    fn from_vm(_ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError> {
        v.as_char()
            .ok_or_else(|| NativeError::type_error(format!("expected char, got {v:?}")))
    }
}

impl<T: FromVm> FromVm for Option<T> {
    #[inline]
    fn from_vm(ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError> {
        if v.is_null() {
            Ok(None)
        } else {
            T::from_vm(ctx, v).map(Some)
        }
    }
}

impl IntoVm for VmValue {
    #[inline]
    fn into_vm(self, _ctx: &mut dyn NativeCtx) -> VmValue {
        self
    }
}

impl IntoVm for i64 {
    const STORAGE: Option<vn_core::ArrayStorage> = Some(vn_core::ArrayStorage::Int);

    #[inline]
    fn into_vm(self, _ctx: &mut dyn NativeCtx) -> VmValue {
        VmValue::from_int(self)
    }
}

impl IntoVm for i32 {
    const STORAGE: Option<vn_core::ArrayStorage> = Some(vn_core::ArrayStorage::Int);

    #[inline]
    fn into_vm(self, _ctx: &mut dyn NativeCtx) -> VmValue {
        VmValue::from_int(self as i64)
    }
}

impl IntoVm for usize {
    const STORAGE: Option<vn_core::ArrayStorage> = Some(vn_core::ArrayStorage::Int);

    #[inline]
    fn into_vm(self, _ctx: &mut dyn NativeCtx) -> VmValue {
        VmValue::from_int(self as i64)
    }
}

impl IntoVm for f64 {
    const STORAGE: Option<vn_core::ArrayStorage> = Some(vn_core::ArrayStorage::Float);

    #[inline]
    fn into_vm(self, _ctx: &mut dyn NativeCtx) -> VmValue {
        VmValue::from_f64(self)
    }
}

impl IntoVm for bool {
    const STORAGE: Option<vn_core::ArrayStorage> = Some(vn_core::ArrayStorage::Bool);

    #[inline]
    fn into_vm(self, _ctx: &mut dyn NativeCtx) -> VmValue {
        VmValue::from_bool(self)
    }
}

impl IntoVm for String {
    const STORAGE: Option<vn_core::ArrayStorage> = Some(vn_core::ArrayStorage::Boxed);

    #[inline]
    fn into_vm(self, ctx: &mut dyn NativeCtx) -> VmValue {
        ctx.alloc_str_owned(self)
    }
}

impl IntoVm for &str {
    const STORAGE: Option<vn_core::ArrayStorage> = Some(vn_core::ArrayStorage::Boxed);

    #[inline]
    fn into_vm(self, ctx: &mut dyn NativeCtx) -> VmValue {
        ctx.alloc_str(self)
    }
}

impl IntoVm for char {
    const STORAGE: Option<vn_core::ArrayStorage> = Some(vn_core::ArrayStorage::Char);

    #[inline]
    fn into_vm(self, _ctx: &mut dyn NativeCtx) -> VmValue {
        VmValue::from_char(self)
    }
}

impl IntoVm for () {
    #[inline]
    fn into_vm(self, _ctx: &mut dyn NativeCtx) -> VmValue {
        VmValue::null()
    }
}

impl<T: IntoVm> IntoVm for Option<T> {
    const STORAGE: Option<vn_core::ArrayStorage> = Some(vn_core::ArrayStorage::Boxed);

    #[inline]
    fn into_vm(self, ctx: &mut dyn NativeCtx) -> VmValue {
        match self {
            Some(t) => t.into_vm(ctx),
            None => VmValue::null(),
        }
    }
}

impl<T: IntoVm> IntoVm for Vec<T> {
    const STORAGE: Option<vn_core::ArrayStorage> = Some(vn_core::ArrayStorage::Ref);

    fn into_vm(self, ctx: &mut dyn NativeCtx) -> VmValue {
        let items = self.into_iter().map(|item| item.into_vm(ctx)).collect();
        match T::STORAGE {
            Some(storage) => ctx.alloc_array(storage, items),
            None => ctx.alloc_array_inferred(items),
        }
    }
}

impl<T: FromVm> FromVm for Vec<T> {
    fn from_vm(ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError> {
        if !ctx.is_array(v) {
            return Err(NativeError::type_error(format!(
                "expected array, got {v:?}"
            )));
        }
        (0..ctx.array_len(v))
            .map(|i| T::from_vm(ctx, ctx.array_get(v, i).unwrap_or_else(VmValue::null)))
            .collect()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct VnArray(pub VmValue);

impl VnArray {
    #[inline]
    pub fn raw(self) -> VmValue {
        self.0
    }

    #[inline]
    pub fn len(self, ctx: &dyn NativeCtx) -> usize {
        ctx.array_len(self.0)
    }

    #[inline]
    pub fn is_empty(self, ctx: &dyn NativeCtx) -> bool {
        self.len(ctx) == 0
    }

    #[inline]
    pub fn get(self, ctx: &dyn NativeCtx, idx: usize) -> Option<VmValue> {
        ctx.array_get(self.0, idx)
    }

    #[inline]
    pub fn set(self, ctx: &mut dyn NativeCtx, idx: usize, val: VmValue) {
        ctx.array_set(self.0, idx, val);
    }

    #[inline]
    pub fn push(self, ctx: &mut dyn NativeCtx, val: VmValue) {
        ctx.array_push(self.0, val);
    }

    #[inline]
    pub fn pop(self, ctx: &mut dyn NativeCtx) -> Option<VmValue> {
        ctx.array_pop(self.0)
    }

    pub fn to_vec(self, ctx: &dyn NativeCtx) -> Vec<VmValue> {
        let len = self.len(ctx);
        let mut out = Vec::with_capacity(len);
        for i in 0..len {
            out.push(self.get(ctx, i).unwrap_or_else(VmValue::null));
        }
        out
    }
}

impl FromVm for VnArray {
    #[inline]
    fn from_vm(ctx: &dyn NativeCtx, v: VmValue) -> Result<Self, NativeError> {
        if ctx.is_array(v) {
            Ok(VnArray(v))
        } else {
            Err(NativeError::type_error(format!(
                "expected array, got {v:?}"
            )))
        }
    }
}

impl IntoVm for VnArray {
    #[inline]
    fn into_vm(self, _ctx: &mut dyn NativeCtx) -> VmValue {
        self.0
    }
}
