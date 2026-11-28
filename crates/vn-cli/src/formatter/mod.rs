use crate::cli::FmtArgs;
use crate::error::CliError;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use vn_core::term::chalk;

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

fn format_stdin(args: &FmtArgs) -> Result<(), CliError> {
    let mut source = String::new();
    std::io::stdin()
        .read_to_string(&mut source)
        .map_err(|e| CliError::fatal(format!("cannot read the source from stdin: {e}")))?;
    let name = args.stdin_filename.as_deref().unwrap_or("<stdin>");
    let formatted =
        vn_fmt::format_source(&source, name).map_err(|e| CliError::fatal(format!("{name}:{e}")))?;
    std::io::stdout()
        .write_all(formatted.as_bytes())
        .map_err(|e| CliError::fatal(format!("cannot write the formatted source: {e}")))
}

pub fn run_fmt(project: &vn_modules::project::Project, args: FmtArgs) -> Result<(), CliError> {
    if args.stdin {
        return format_stdin(&args);
    }
    let start_time = Instant::now();
    let targets: Vec<&str> = if args.paths.is_empty() {
        vec!["."]
    } else {
        args.paths.iter().map(String::as_str).collect()
    };
    let mut files: Vec<PathBuf> = Vec::new();
    for target in &targets {
        files.extend(crate::discover::vn_files(
            Path::new(target),
            crate::discover::Depth::Recursive,
            &project.manifest.fmt,
            &project.root,
        )?);
    }
    files.sort();
    files.dedup();

    if files.is_empty() {
        if args.verbose {
            println!("No .vn files found to format in '{}'", targets.join("', '"));
        }
        return Ok(());
    }

    let mut changed = 0;
    let mut unformatted = Vec::new();
    let mut failures: Vec<(PathBuf, String)> = Vec::new();

    for file in &files {
        let content = fs::read_to_string(file)
            .map_err(|e| CliError::fatal(format!("Failed to read {}: {e}", file.display())))?;

        let formatted = match vn_fmt::format_source(&content, &file.to_string_lossy()) {
            Ok(formatted) => formatted,
            Err(error) => {
                failures.push((file.clone(), error.to_string()));
                continue;
            }
        };

        if content != formatted {
            changed += 1;
            if args.check {
                unformatted.push(file.clone());
            } else {
                fs::write(file, &formatted).map_err(|e| {
                    CliError::fatal(format!("Failed to write {}: {e}", file.display()))
                })?;
                if args.verbose {
                    println!("  {} {}", chalk("formatted").green(), file.display());
                }
            }
        }
    }

    let elapsed = start_time.elapsed();

    for (file, message) in &failures {
        eprintln!("{}: {}:{message}", chalk("error").red(), file.display());
    }

    if args.check && !unformatted.is_empty() {
        eprintln!(
            "\n{}: {} file{} not properly formatted:\n",
            chalk("error").red(),
            unformatted.len(),
            plural(unformatted.len())
        );
        for f in &unformatted {
            eprintln!("  - {}", f.display());
        }
        eprintln!("\nRun {} to format these files.\n", chalk("vn fmt").bold());
    }
    if !failures.is_empty() || !unformatted.is_empty() {
        return Err(CliError::usage(format!(
            "{} file{} not formatted",
            failures.len() + unformatted.len(),
            plural(failures.len() + unformatted.len())
        )));
    }

    if args.check {
        println!(
            "\n  {} All {} .vn file{} correctly formatted ({:.2?})\n",
            chalk("✓").green().bold(),
            files.len(),
            plural(files.len()),
            elapsed
        );
    } else {
        println!(
            "\n  {} · {} file{} checked, {} formatted ({:.2?})\n",
            chalk("vn fmt").cyan().bold(),
            files.len(),
            plural(files.len()),
            changed,
            elapsed
        );
    }

    Ok(())
}
