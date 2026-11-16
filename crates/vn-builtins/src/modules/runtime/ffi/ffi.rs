use vn_core::sandbox::Capability;
use vn_core::StdErrorClass;
use vn_macros::natives;
use vn_types::NativeError;
use vn_types::NativeCtx;

#[path = "call.rs"]
mod call;
#[path = "memory.rs"]
mod memory;


#[derive(Debug)]
struct LoadedLib {
    lib: libloading::Library,
    is_system_lib: bool,
}


fn permit(ctx: &mut dyn NativeCtx) -> Result<(), NativeError> {
    if ctx.has_capability(Capability::SysFfi) {
        Ok(())
    } else {
        Err(NativeError::capability(Capability::SysFfi.as_str(), "runtime:ffi"))
    }
}

natives! {
    dlopen(ctx, path) => {
        permit(ctx)?;
        let is_system_lib = path == "libc.so.6"
            || path == "libc.so"
            || path.starts_with("libc.")
            || path == "libSystem.B.dylib"
            || path.eq_ignore_ascii_case("kernel32.dll");

        unsafe {
            let lib = libloading::Library::new(path)
                .map_err(|e| format!("ffi.dlopen: failed to load '{path}': {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
            let resource_id = ctx.resources().insert(LoadedLib { lib, is_system_lib });
            Ok(resource_id as i64)
        }
    },
    dlclose(ctx, libHandle) => {
        permit(ctx)?;
        let handle_id = libHandle as u32;
        if let Some(loaded) = ctx.resources().remove::<LoadedLib>(handle_id) {
            if loaded.is_system_lib {
                std::mem::forget(loaded.lib);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    },
    dlsym(ctx, libHandle, symbol) => {
        permit(ctx)?;
        let handle_id = libHandle as u32;
        let res = ctx.resources().get::<LoadedLib>(handle_id)
            .ok_or_else(|| "ffi.dlsym: invalid library handle".to_string()).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;

        unsafe {
            let c_str = std::ffi::CString::new(symbol)
                .map_err(|e| format!("ffi.dlsym: invalid symbol name: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
            let sym: libloading::Symbol<*const ()> = res.lib.get(c_str.as_bytes_with_nul())
                .map_err(|e| format!("ffi.dlsym: symbol '{symbol}' not found: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
            let ptr = *sym as usize as i64;
            Ok(ptr)
        }
    },
    call(ctx, fnPtr, retType, paramTypes, args) => {
        permit(ctx)?;
        call::call(ctx, fnPtr, retType, paramTypes, args)
    },
    kindName(raw) => {
        vn_core::FfiKind::of_raw(raw)
            .map(|kind| kind.name().to_owned())
            .ok_or_else(|| NativeError::range_error(format!("ffi.kindName: {raw} is not an FFI type")))
    },
    alloc(ctx, size) => {
        permit(ctx)?;
        memory::allocate("alloc", size).map(|addr| addr as i64)
    },
    free(ctx, ptr) => {
        permit(ctx)?;
        memory::release("free", ptr as usize)
    },
    readInt(ctx, ptr, offset, size) => {
        permit(ctx)?;
        let width = match size {
            1 | 2 | 4 | 8 => size as usize,
            _ => return Err(NativeError::range_error(format!("ffi.readInt: unsupported size {size}, expected 1, 2, 4 or 8"))),
        };
        let addr = memory::span("readInt", ptr, offset, width)?;
        unsafe {
            Ok(match width {
                1 => std::ptr::read_unaligned(addr as *const i8) as i64,
                2 => std::ptr::read_unaligned(addr as *const i16) as i64,
                4 => std::ptr::read_unaligned(addr as *const i32) as i64,
                _ => std::ptr::read_unaligned(addr as *const i64),
            })
        }
    },
    writeInt(ctx, ptr, offset, val, size) => {
        permit(ctx)?;
        let width = match size {
            1 | 2 | 4 | 8 => size as usize,
            _ => return Err(NativeError::range_error(format!("ffi.writeInt: unsupported size {size}, expected 1, 2, 4 or 8"))),
        };
        let addr = memory::span("writeInt", ptr, offset, width)?;
        unsafe {
            match width {
                1 => std::ptr::write_unaligned(addr as *mut u8, val as u8),
                2 => std::ptr::write_unaligned(addr as *mut i16, val as i16),
                4 => std::ptr::write_unaligned(addr as *mut i32, val as i32),
                _ => std::ptr::write_unaligned(addr as *mut i64, val),
            }
        }
        Ok(())
    },
    readFloat(ctx, ptr, offset, isDouble) => {
        permit(ctx)?;
        let width = if isDouble { 8 } else { 4 };
        let addr = memory::span("readFloat", ptr, offset, width)?;
        unsafe {
            if isDouble {
                Ok(std::ptr::read_unaligned(addr as *const f64))
            } else {
                Ok(std::ptr::read_unaligned(addr as *const f32) as f64)
            }
        }
    },
    writeFloat(ctx, ptr, offset, val, isDouble) => {
        permit(ctx)?;
        let width = if isDouble { 8 } else { 4 };
        let addr = memory::span("writeFloat", ptr, offset, width)?;
        unsafe {
            if isDouble {
                std::ptr::write_unaligned(addr as *mut f64, val);
            } else {
                std::ptr::write_unaligned(addr as *mut f32, val as f32);
            }
        }
        Ok(())
    },
    allocCString(ctx, s) => {
        permit(ctx)?;
        memory::allocate_cstring("allocCString", s).map(|addr| addr as i64)
    },
    readCString(ctx, ptr, offset) => {
        permit(ctx)?;
        let (start, limit) = memory::cstring_bounds("readCString", ptr, offset)?;
        unsafe {
            let addr = start as *const u8;
            let mut len = 0usize;
            while len < limit && *addr.add(len) != 0 {
                len += 1;
            }
            if len == limit {
                return Err(NativeError::range_error("ffi.readCString: string is not NUL-terminated"));
            }
            let slice = std::slice::from_raw_parts(addr, len);
            Ok(String::from_utf8_lossy(slice).into_owned())
        }
    },
}
