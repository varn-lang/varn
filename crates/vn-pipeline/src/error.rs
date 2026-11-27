use std::fmt;
use std::sync::Arc;
use vn_core::DiagnosticBag;

pub struct DiagnosticReport {
    pub file: Arc<str>,
    pub source: Arc<str>,
    pub diagnostics: DiagnosticBag,
}

impl DiagnosticReport {
    pub fn new(file: &str, source: &str, mut diagnostics: DiagnosticBag) -> Self {
        diagnostics.normalize();
        let items = diagnostics.take();
        for mut diagnostic in items {
            diagnostic.file = Arc::from(relative_path(&diagnostic.file));
            for related in &mut diagnostic.related {
                related.file = Arc::from(relative_path(&related.file));
            }
            diagnostics.push(diagnostic);
        }
        Self {
            file: Arc::from(relative_path(file)),
            source: Arc::from(source),
            diagnostics,
        }
    }
}

pub struct DiagnosticFailure {
    pub subject: Arc<str>,
    pub reports: Vec<DiagnosticReport>,
}

impl DiagnosticFailure {
    pub fn error_count(&self) -> usize {
        self.reports
            .iter()
            .map(|report| report.diagnostics.iter().filter(|d| d.is_error()).count())
            .sum()
    }

    pub fn warning_count(&self) -> usize {
        self.reports
            .iter()
            .map(|report| {
                report
                    .diagnostics
                    .iter()
                    .filter(|d| d.kind == vn_core::DiagnosticKind::Warning)
                    .count()
            })
            .sum()
    }
}

pub enum PipelineError {
    Diagnostics(DiagnosticFailure),
    Io(String),
    Internal(String),
    Project(vn_modules::project::ManifestError),
}

impl PipelineError {
    pub fn io(message: impl Into<String>) -> Self {
        PipelineError::Io(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        PipelineError::Internal(message.into())
    }

    pub fn diagnostics(file: &str, source: &str, diagnostics: DiagnosticBag) -> Self {
        PipelineError::diagnostics_across(
            file,
            vec![DiagnosticReport::new(file, source, diagnostics)],
        )
    }

    pub fn diagnostics_across(subject: &str, reports: Vec<DiagnosticReport>) -> Self {
        PipelineError::Diagnostics(DiagnosticFailure {
            subject: Arc::from(relative_path(subject)),
            reports,
        })
    }
}

pub fn relative_path(file: &str) -> String {
    let cleaned = vn_core::paths::portable_path(file);
    std::env::current_dir()
        .ok()
        .and_then(|cwd| {
            let root = vn_core::paths::portable_path(&cwd.to_string_lossy());
            cleaned.strip_prefix(&format!("{root}/")).map(str::to_owned)
        })
        .unwrap_or(cleaned)
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineError::Diagnostics(failure) => {
                for report in &failure.reports {
                    writeln!(
                        f,
                        "{}",
                        vn_core::diagnostics::render_diagnostics(
                            &report.source,
                            &report.diagnostics
                        )
                    )?;
                }
                write!(
                    f,
                    "
{}",
                    vn_core::diagnostics::render_summary(
                        &failure.subject,
                        failure.error_count(),
                        failure.warning_count()
                    )
                )
            }
            PipelineError::Io(message) | PipelineError::Internal(message) => write!(f, "{message}"),
            PipelineError::Project(error) => write!(f, "{error}"),
        }
    }
}

impl From<String> for PipelineError {
    fn from(message: String) -> Self {
        PipelineError::internal(message)
    }
}

impl From<std::io::Error> for PipelineError {
    fn from(error: std::io::Error) -> Self {
        PipelineError::io(error.to_string())
    }
}
