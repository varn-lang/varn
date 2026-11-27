use crate::exit::ExitCode;
use std::fmt;
use vn_modules::project::ManifestError;
use vn_pipeline::PipelineError;
use vn_runtime::ExecError;

pub enum CliError {
    Pipeline(PipelineError),
    Exec(ExecError),
    Fatal(String),
    Usage(String),
    Summary { message: String, exit: ExitCode },
}

impl CliError {
    pub fn fatal(message: impl Into<String>) -> Self {
        CliError::Fatal(message.into())
    }

    pub fn usage(message: impl Into<String>) -> Self {
        CliError::Usage(message.into())
    }

    pub fn exit_code(&self) -> ExitCode {
        match self {
            CliError::Pipeline(PipelineError::Diagnostics(_)) => ExitCode::Diagnostics,
            CliError::Exec(_) => ExitCode::Uncaught,
            CliError::Pipeline(
                PipelineError::Io(_) | PipelineError::Internal(_) | PipelineError::Project(_),
            )
            | CliError::Fatal(_) => ExitCode::Fatal,
            CliError::Usage(_) => ExitCode::Usage,
            CliError::Summary { exit, .. } => *exit,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Pipeline(error) => write!(f, "{error}"),
            CliError::Exec(error) => write!(f, "{error}"),
            CliError::Fatal(message)
            | CliError::Usage(message)
            | CliError::Summary { message, .. } => write!(f, "{message}"),
        }
    }
}

impl From<PipelineError> for CliError {
    fn from(error: PipelineError) -> Self {
        CliError::Pipeline(error)
    }
}

impl From<ExecError> for CliError {
    fn from(error: ExecError) -> Self {
        CliError::Exec(error)
    }
}

impl From<ManifestError> for CliError {
    fn from(error: ManifestError) -> Self {
        CliError::Fatal(error.to_string())
    }
}
