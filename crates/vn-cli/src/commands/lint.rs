use crate::cli::LintArgs;
use crate::error::CliError;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use vn_core::term::chalk;
use vn_core::{Diagnostic, DiagnosticKind};
use vn_pipeline::Source;

pub fn execute(session: &vn_pipeline::Session, args: LintArgs) -> Result<(), CliError> {
    let start = Instant::now();
    let project = session.project();
    let config = &project.manifest.lint;
    let policy = session.lint_policy().map_err(CliError::usage)?;

    let targets: Vec<&str> = if args.paths.is_empty() {
        vec!["."]
    } else {
        args.paths.iter().map(String::as_str).collect()
    };
    let filter = config.files();
    let mut files: Vec<PathBuf> = Vec::new();
    for target in &targets {
        files.extend(crate::discover::vn_files(
            Path::new(target),
            crate::discover::Depth::Recursive,
            &filter,
            &project.root,
        )?);
    }
    files.sort();
    files.dedup();

    let mut warnings = 0usize;
    let mut errors = 0usize;
    let mut fixed = 0usize;
    for file in &files {
        let path = file.to_string_lossy().to_string();
        let mut text = fs::read_to_string(file)
            .map_err(|e| CliError::fatal(format!("Failed to read {}: {e}", file.display())))?;
        if args.fix {
            match session.fix(&path, &text, &policy) {
                Ok((rewritten, applied)) if applied > 0 => {
                    fs::write(file, &rewritten).map_err(|e| {
                        CliError::fatal(format!("Failed to write {}: {e}", file.display()))
                    })?;
                    fixed += applied;
                    text = rewritten;
                }
                Ok(_) => {}
                Err(message) => {
                    eprintln!("{}: {path}:{message}", chalk("error").red());
                    errors += 1;
                    continue;
                }
            }
        }
        let found = match session.lint(&Source::virtual_source(&path, &text), &policy) {
            Ok(found) => found,
            Err(message) => {
                eprintln!("{}: {path}:{message}", chalk("error").red());
                errors += 1;
                continue;
            }
        };
        for diagnostic in found {
            report(&diagnostic, &text);
            if diagnostic.kind == DiagnosticKind::Error {
                errors += 1;
            } else {
                warnings += 1;
            }
        }
    }

    println!(
        "\n  {} · {} file{} checked, {} warning{}, {} error{}{} ({:.2?})\n",
        chalk("vn lint").cyan().bold(),
        files.len(),
        if files.len() == 1 { "" } else { "s" },
        warnings,
        if warnings == 1 { "" } else { "s" },
        errors,
        if errors == 1 { "" } else { "s" },
        if fixed > 0 {
            format!(", {fixed} fixed")
        } else {
            String::new()
        },
        start.elapsed()
    );
    if errors > 0 || (args.deny_warnings && warnings > 0) {
        return Err(CliError::usage(format!(
            "{} problem{} found",
            errors + warnings,
            if errors + warnings == 1 { "" } else { "s" }
        )));
    }
    Ok(())
}

fn report(diagnostic: &Diagnostic, source: &str) {
    eprintln!(
        "{}\n",
        vn_core::diagnostics::format_diagnostic(diagnostic, source)
    );
}
