use vn_core::StdErrorClass;
use std::io::Write as IoWrite;
use vn_macros::natives;
use vn_types::NativeError;
use vn_types::{NativeCtx, VmValue};


natives! {
    write(ctx, args) => {
        if !ctx.print_silent() {
            for &v in args {
                print!("{}", ctx.str_repr_borrowed(v));
            }
        }
        Ok(())
    },
    writeln(ctx, args) => {
        if !ctx.print_silent() {
            for &v in args {
                print!("{}", ctx.str_repr_borrowed(v));
            }
            println!();
        }
        Ok(())
    },
    flush(ctx) => {
        if !ctx.print_silent() {
            std::io::stdout().flush().map_err(|e| format!("E_IO_FLUSH|io.flush: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        }
        Ok(())
    },
    readLine(ctx, prompt) => {
        if let Some(p) = prompt
            && !ctx.print_silent() {
                print!("{p}");
                let _ = std::io::stdout().flush();
            }
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| format!("E_IO_READ|io.read_line: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        Ok(line.trim_end_matches(['\r', '\n']).to_string())
    },
}

pub fn read_line(ctx: &mut dyn NativeCtx, args: &[VmValue]) -> Result<VmValue, String> {
    if let Some(&p) = args.first()
        && !p.is_null()
        && !ctx.print_silent()
    {
        print!("{}", ctx.str_repr_borrowed(p));
        let _ = std::io::stdout().flush();
    }
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| format!("E_IO_READ|io.read_line: {e}"))?;
    Ok(ctx.alloc_str_owned(line.trim_end_matches(['\r', '\n']).to_string()))
}
