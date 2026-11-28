use crate::cli::LintArgs;
use crate::error::CliError;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use vn_core::term::chalk;
use vn_core::{Diagnostic, DiagnosticKind, ErrorCode};

const MAX_FIX_PASSES: usize = 5;

fn codes(names: &[String], what: &str) -> Result<HashSet<ErrorCode>, CliError> {
    names
        .iter()
        .map(|name| {
            vn_lint::rule_of(name).ok_or_else(|| {
                CliError::usage(format!("`{name}` in [lint] {what} is not a lint rule"))
            })
        })
        .collect()
}

pub fn execute(session: &vn_pipeline::Session, args: LintArgs) -> Result<(), CliError> {
    let start = Instant::now();
    let project = session.project();
    let config = &project.manifest.lint;
    let allowed = codes(&config.allow, "allow")?;
    let denied = codes(&config.deny, "deny")?;

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
        let mut found = match vn_lint::lint(&text, &path) {
            Ok(found) => found,
            Err(message) => {
                eprintln!("{}: {path}:{message}", chalk("error").red());
                errors += 1;
                continue;
            }
        };
        found.retain(|d| !allowed.contains(&d.code));
        let mut passes = 0;
        while args.fix && passes < MAX_FIX_PASSES && found.iter().any(|d| !d.suggestions.is_empty())
        {
            passes += 1;
            let (rewritten, applied) = vn_lint::apply_fixes(&text, &found);
            if applied == 0 {
                break;
            }
            fs::write(file, &rewritten)
                .map_err(|e| CliError::fatal(format!("Failed to write {}: {e}", file.display())))?;
            fixed += applied;
            text = rewritten;
            found = vn_lint::lint(&text, &path).unwrap_or_default();
            found.retain(|d| !allowed.contains(&d.code));
        }
        for mut diagnostic in found {
            if denied.contains(&diagnostic.code) {
                diagnostic.kind = DiagnosticKind::Error;
            }
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
