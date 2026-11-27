use std::path::{Path, PathBuf};

use crate::error::CliError;
use vn_modules::project::FileFilter;

const SKIPPED_DIRS: &[&str] = &["target", "node_modules"];

#[derive(Clone, Copy)]
pub enum Depth {
    Direct,
    Recursive,
}

pub fn vn_files(
    path: &Path,
    depth: Depth,
    filter: &FileFilter,
    root: &Path,
) -> Result<Vec<PathBuf>, CliError> {
    if has_glob_meta(&path.to_string_lossy()) {
        return expand_glob(path, filter, root);
    }

    if !path.exists() {
        return Err(CliError::usage(format!(
            "Path '{}' does not exist",
            path.display()
        )));
    }

    let mut files = Vec::new();
    if path.is_file() {
        if is_vn(path) {
            files.push(path.to_path_buf());
        }
        return Ok(files);
    }

    collect(path, depth, &mut files)?;
    files.retain(|file| filter.admits(&relative_to(file, root)));
    files.sort();
    Ok(files)
}

fn has_glob_meta(s: &str) -> bool {
    s.bytes().any(|b| b == b'*' || b == b'?' || b == b'[')
}

fn expand_glob(pattern: &Path, filter: &FileFilter, root: &Path) -> Result<Vec<PathBuf>, CliError> {
    let norm = normalize_seps(&pattern.to_string_lossy());
    let norm = norm.trim_start_matches("./");
    let base = static_base(norm);
    let base_path = if base.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(base)
    };
    if !base_path.exists() {
        return Ok(Vec::new());
    }

    let mut pool = Vec::new();
    if norm.contains("**") {
        collect_all(&base_path, &mut pool)?;
    } else {
        let entries = std::fs::read_dir(&base_path).map_err(|e| {
            CliError::fatal(format!(
                "Failed to read directory '{}': {e}",
                base_path.display()
            ))
        })?;
        for entry in entries.flatten() {
            pool.push(entry.path());
        }
    }

    let mut files = Vec::new();
    for cand in &pool {
        let cand_norm = normalize_seps(&cand.to_string_lossy());
        let cand_norm = cand_norm.trim_start_matches("./");
        if !match_glob(norm, cand_norm) {
            continue;
        }
        if cand.is_file() {
            if is_vn(cand) {
                files.push(cand.clone());
            }
        } else if cand.is_dir() {
            collect(cand, Depth::Direct, &mut files)?;
        }
    }
    files.retain(|file| filter.admits(&relative_to(file, root)));
    files.sort();
    files.dedup();
    Ok(files)
}

fn normalize_seps(s: &str) -> String {
    s.replace('\\', "/")
}

fn static_base(pattern: &str) -> &str {
    let first_meta = pattern
        .bytes()
        .position(|b| b == b'*' || b == b'?' || b == b'[')
        .unwrap_or(pattern.len());
    match pattern[..first_meta].rfind('/') {
        Some(i) => &pattern[..i],
        None => "",
    }
}

fn collect_all(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), CliError> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        CliError::fatal(format!("Failed to read directory '{}': {e}", dir.display()))
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if !skipped(&path) {
                out.push(path.clone());
                collect_all(&path, out)?;
            }
        } else if path.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

fn match_glob(pattern: &str, path: &str) -> bool {
    let pat: Vec<&str> = pattern.split('/').collect();
    let parts: Vec<&str> = path.split('/').collect();
    match_segments(&pat, &parts)
}

fn match_segments(pat: &[&str], path: &[&str]) -> bool {
    if pat.is_empty() {
        return path.is_empty();
    }
    if pat[0] == "**" {
        return (0..=path.len()).any(|i| match_segments(&pat[1..], &path[i..]));
    }
    if path.is_empty() {
        return false;
    }
    segment_match(pat[0].as_bytes(), path[0].as_bytes()) && match_segments(&pat[1..], &path[1..])
}

fn segment_match(pat: &[u8], name: &[u8]) -> bool {
    let (mut p, mut n) = (0usize, 0usize);
    let (mut star, mut mark) = (None, 0usize);
    while n < name.len() {
        if p < pat.len() && (pat[p] == b'?' || pat[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pat.len() && pat[p] == b'[' {
            match char_class(&pat[p..], name[n]) {
                Some(consumed) => {
                    p += consumed;
                    n += 1;
                }
                None => return false,
            }
        } else if p < pat.len() && pat[p] == b'*' {
            star = Some(p);
            mark = n;
            p += 1;
        } else if let Some(s) = star {
            mark += 1;
            n = mark;
            p = s + 1;
        } else {
            return false;
        }
    }
    while p < pat.len() && pat[p] == b'*' {
        p += 1;
    }
    p == pat.len()
}

fn char_class(pat: &[u8], c: u8) -> Option<usize> {
    let mut i = 1usize;
    let mut negate = false;
    if i < pat.len() && (pat[i] == b'!' || pat[i] == b'^') {
        negate = true;
        i += 1;
    }
    let mut matched = false;
    if i < pat.len() && pat[i] == b']' {
        matched = c == b']';
        i += 1;
    }
    while i < pat.len() && pat[i] != b']' {
        if i + 2 < pat.len() && pat[i + 1] == b'-' && pat[i + 2] != b']' && pat[i] <= pat[i + 2] {
            matched = matched || (pat[i] <= c && c <= pat[i + 2]);
            i += 3;
        } else {
            matched = matched || pat[i] == c;
            i += 1;
        }
    }
    if i >= pat.len() {
        return None;
    }
    if matched != negate {
        Some(i + 1)
    } else {
        None
    }
}

fn collect(dir: &Path, depth: Depth, out: &mut Vec<PathBuf>) -> Result<(), CliError> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        CliError::fatal(format!("Failed to read directory '{}': {e}", dir.display()))
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if matches!(depth, Depth::Recursive) && !skipped(&path) {
                collect(&path, depth, out)?;
            }
        } else if path.is_file() && is_vn(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn relative_to(file: &Path, root: &Path) -> String {
    let absolute = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
    absolute
        .strip_prefix(root)
        .unwrap_or(&absolute)
        .to_string_lossy()
        .into_owned()
}

fn skipped(dir: &Path) -> bool {
    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.starts_with('.') || SKIPPED_DIRS.contains(&name)
}

fn is_vn(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == vn_modules::VN_FILE_EXTENSION)
}
