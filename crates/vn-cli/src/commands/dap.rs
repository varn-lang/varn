use crate::error::CliError;

pub fn execute() -> Result<(), CliError> {
    crate::dap::adapter::run_stdio();
    Ok(())
}
