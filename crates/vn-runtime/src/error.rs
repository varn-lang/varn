use std::fmt;
use vn_core::term::chalk::chalk;
use vn_pipeline::relative_path;
use vn_vm::RuntimeError;

const FRAMES_SHOWN_FIRST: usize = 12;
const FRAMES_SHOWN_LAST: usize = 6;

pub struct ExecError {
    pub error: RuntimeError,
}

impl From<RuntimeError> for ExecError {
    fn from(error: RuntimeError) -> Self {
        Self { error }
    }
}

fn write_chain(error: &RuntimeError, f: &mut fmt::Formatter<'_>, label: &str) -> fmt::Result {
    let class = if error.class == vn_core::RuntimeErrorKind::Error.name() {
        String::new()
    } else {
        format!("{}: ", error.class)
    };
    write!(
        f,
        "{}{}",
        chalk(label).red().bold(),
        chalk(format!(": {class}{}", error.message)).bold()
    )?;
    let omitted = error
        .frames
        .len()
        .saturating_sub(FRAMES_SHOWN_FIRST + FRAMES_SHOWN_LAST);
    for (at, frame) in error.frames.iter().enumerate() {
        if omitted > 0 && at >= FRAMES_SHOWN_FIRST && at < FRAMES_SHOWN_FIRST + omitted {
            if at == FRAMES_SHOWN_FIRST {
                write!(
                    f,
                    "\n    {}",
                    chalk(format!("... {omitted} more frames")).dim()
                )?;
            }
            continue;
        }
        write!(
            f,
            "\n    {} {} {}",
            chalk("at").dim(),
            frame.fn_name,
            chalk(format!("({}:{})", relative_path(&frame.file), frame.line)).dim()
        )?;
    }
    if let Some(cause) = &error.cause {
        writeln!(f)?;
        write_chain(cause, f, "caused by")?;
    }
    Ok(())
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_chain(&self.error, f, "error")
    }
}
