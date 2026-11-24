use super::descriptors::fs_error;
use std::fs;
use std::io::Write;
use std::path::Path;
use vn_types::NativeError;

pub fn read_text(path: &Path) -> Result<String, NativeError> {
    fs::read_to_string(path).map_err(|e| fs_error(e, &path.to_string_lossy()))
}

pub fn read_bytes(path: &Path) -> Result<Vec<u8>, NativeError> {
    fs::read(path).map_err(|e| fs_error(e, &path.to_string_lossy()))
}

pub fn read_lines(path: &Path) -> Result<Vec<String>, NativeError> {
    Ok(read_text(path)?.lines().map(str::to_owned).collect())
}

pub fn read_dir(path: &Path) -> Result<Vec<String>, NativeError> {
    Ok(fs::read_dir(path)
        .map_err(|e| fs_error(e, &path.to_string_lossy()))?
        .flatten()
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .collect())
}

pub fn write(path: &Path, data: &[u8]) -> Result<(), NativeError> {
    fs::write(path, data).map_err(|e| fs_error(e, &path.to_string_lossy()))
}

pub fn append(path: &Path, data: &[u8]) -> Result<(), NativeError> {
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| fs_error(e, &path.to_string_lossy()))?
        .write_all(data)
        .map_err(|e| fs_error(e, &path.to_string_lossy()))
}

pub fn copy(from: &Path, to: &Path) -> Result<(), NativeError> {
    fs::copy(from, to)
        .map(drop)
        .map_err(|e| fs_error(e, &from.to_string_lossy()))
}

pub fn remove_all(path: &Path) -> Result<(), NativeError> {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| fs_error(e, &path.to_string_lossy()))
    } else {
        fs::remove_file(path).map_err(|e| fs_error(e, &path.to_string_lossy()))
    }
}
