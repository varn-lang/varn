use vn_macros::natives;
use vn_types::NativeError;


natives! {
    sameValue(ctx, left, right) => ctx.values_equal(left, right),
    print(ctx, args) => {
        if !ctx.print_silent() {
            use std::io::Write;
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            for (i, &v) in args.iter().enumerate() {
                if i > 0 {
                    let _ = write!(out, " ");
                }
                let _ = write!(out, "{}", ctx.str_repr_borrowed(v));
            }
            let _ = writeln!(out);
            let _ = out.flush();
        }
        Ok(())
    },
    debug(ctx, args) => {
        let s = args
            .iter()
            .map(|&v| ctx.str_repr_borrowed(v))
            .collect::<Vec<_>>()
            .join(" ");
        vn_core::term::terminal::tagged("debug", s);
        Ok(())
    },
    assertSummary(ctx) => {
        crate::modules::testing::summary(ctx, &[]).map_err(NativeError::failure)?;
        Ok(())
    },
    input(ctx, prompt) => {
        use std::io::Write;
        if let Some(p) = prompt
            && !ctx.print_silent() {
                print!("{p}");
                let _ = std::io::stdout().flush();
            }
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| format!("input: {e}")).map_err(NativeError::failure)?;
        Ok(line.trim_end_matches(['\r', '\n']).to_string())
    },
}
