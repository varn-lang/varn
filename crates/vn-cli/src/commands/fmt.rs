use crate::{cli::FmtArgs, error::CliError, formatter};

pub fn execute(session: &vn_pipeline::Session, args: FmtArgs) -> Result<(), CliError> {
    formatter::run_fmt(session.project(), args)
}
