use crate::{cli::EvalArgs, error::CliError};

pub fn execute(session: &vn_pipeline::Session, args: EvalArgs) -> Result<(), CliError> {
    let debug = vn_pipeline::DebugFlags::default();
    let executable = crate::exec::compile(
        session,
        vn_pipeline::Source::virtual_source(crate::source_name::EVAL, &args.code),
        vn_pipeline::CachePolicy::Bypass,
        &debug,
        args.verbose,
    )?;
    crate::exec::run(
        session,
        executable,
        crate::exec::policy(&args.permissions, &debug, false),
    )
}
