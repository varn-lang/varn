#![allow(clippy::disallowed_methods)]
use std::sync::OnceLock;
use std::time::Instant;
use vn_core::StdErrorClass;
use vn_macros::natives;
use vn_types::NativeError;


static START_TIME: OnceLock<Instant> = OnceLock::new();

const FIRST_READ_OFFSET_MS: u64 = 1;

fn clock_origin() -> Instant {
    Instant::now()
        .checked_sub(std::time::Duration::from_millis(FIRST_READ_OFFSET_MS))
        .unwrap_or_else(Instant::now)
}

natives! {
    platform() => std::env::consts::OS.to_string(),
    cwd() => {
        (std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|e| format!("sys.cwd: cannot read the current directory: {e}"))).map_err(|e| NativeError::std(StdErrorClass::Io, e))
    },
    args(ctx) => std::env::args().map(|s| ctx.alloc_str_owned(s)).collect::<Vec<_>>(),
    exit(code) => terminate(code.unwrap_or(0) as i32),
    env(ctx, key) => {
        if !ctx.check_env(key) {
            return Err(NativeError::capability("sys.env", key));
        }
        match std::env::var(key) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => {
                Err(NativeError::std(StdErrorClass::Parse, format!("sys.env: variable '{key}' is not valid Unicode")))
            }
        }
    },
    setEnv(ctx, key, val) => {
        if !ctx.check_env(key) {
            return Err(NativeError::capability("sys.env", key));
        }
        unsafe { std::env::set_var(key, val) };
        Ok(())
    },
    now() => {
        let start = START_TIME.get_or_init(clock_origin);
        Ok(start.elapsed().as_secs_f64() * 1000.0)
    },
}

fn terminate(code: i32) {
    std::process::exit(code)
}
