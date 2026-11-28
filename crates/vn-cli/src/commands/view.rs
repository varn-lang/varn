use std::io::{Read, Write};

use crate::cli::{ViewArgs, ViewKind};
use crate::error::CliError;
use vn_debug::machine::{collect, render_clif, render_machine_code, TierPolicy};
use vn_pipeline::{CachePolicy, Request, Session, Source};

pub fn execute(session: &Session, args: ViewArgs) -> Result<(), CliError> {
    let source = if args.stdin {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|e| CliError::fatal(format!("cannot read the source from stdin: {e}")))?;
        Source::virtual_source(&args.file, &text)
    } else {
        Source::file(&args.file)?
    };
    let executable = session
        .compile(source, Request::executable(CachePolicy::Bypass))
        .into_executable()?;

    let settings = vn_vm::ExecSettings::of(vn_core::env::process(), false);
    let helpers = vn_vm::jit::table_build::build_jit_helpers(&settings);
    let policy = TierPolicy {
        calls_with_loops: settings.tier_loop,
        calls_without_loops: settings.tier_straight,
    };
    let functions = collect(&executable.entry_proto, &helpers, policy);
    if functions.is_empty() {
        return Err(CliError::fatal("the host has no Cranelift target"));
    }
    let text = match args.kind {
        ViewKind::Machine => render_machine_code(&functions, args.wrapper),
        ViewKind::Clif => render_clif(&functions),
    };
    std::io::stdout()
        .write_all(text.as_bytes())
        .map_err(|e| CliError::fatal(format!("cannot write the view: {e}")))
}
