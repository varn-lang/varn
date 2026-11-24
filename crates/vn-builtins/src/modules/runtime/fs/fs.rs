use std::fs;
use vn_macros::natives;
use vn_types::value::SendValue;
use vn_core::sandbox::Capability;
use vn_core::{ErrorField, StdErrorClass};
use vn_types::{NativeCtx, NativeError, VmValue};

use crate::runtime::blocking;

#[path = "descriptors.rs"]
mod descriptors;
#[path = "paths.rs"]
mod paths;
use descriptors::{encoding_error, fs_error, io_error, type_error};

fn capability_denied(capability: &str, path: &str) -> NativeError {
    NativeError::std(
        StdErrorClass::Capability,
        format!("permission denied ({capability}) for path '{path}'"),
    )
    .with_str(ErrorField::Capability, capability)
    .with_str(ErrorField::Target, path)
}

fn allow_read(ctx: &mut dyn NativeCtx, path: &std::path::Path) -> Result<(), NativeError> {
    if ctx.check_fs_read_path(path) {
        Ok(())
    } else {
        Err(capability_denied(
            Capability::FsRead.name(),
            &path.to_string_lossy(),
        ))
    }
}

fn allow_write(ctx: &mut dyn NativeCtx, path: &std::path::Path) -> Result<(), NativeError> {
    if ctx.check_fs_write_path(path) {
        Ok(())
    } else {
        Err(capability_denied(
            Capability::FsWrite.name(),
            &path.to_string_lossy(),
        ))
    }
}

fn allow_copy(
    ctx: &mut dyn NativeCtx,
    from: &std::path::Path,
    to: &std::path::Path,
) -> Result<(), NativeError> {
    if !ctx.check_fs_read_path(from) {
        return Err(capability_denied(
            Capability::FsRead.name(),
            &from.to_string_lossy(),
        ));
    }
    allow_write(ctx, to)
}

fn strings(ctx: &mut dyn NativeCtx, items: Vec<String>) -> Vec<VmValue> {
    items.iter().map(|item| ctx.alloc_str(item)).collect()
}

fn bytes_arg(ctx: &mut dyn NativeCtx, data: VmValue) -> Result<Vec<u8>, NativeError> {
    ctx.buffer_to_bytes(data)
        .ok_or_else(|| type_error("expected Bytes"))
}

fn text_array(items: Vec<String>) -> SendValue {
    SendValue::Array(items.into_iter().map(SendValue::Str).collect())
}


vn_macros::vn_enum! {
    module: MODULE_ID,
    contract: "src/modules/runtime/fs/fs_runtime.vn",
    pub enum FileMode;
}

vn_macros::vn_enum! {
    module: MODULE_ID,
    contract: "src/modules/runtime/fs/fs_runtime.vn",
    pub enum SeekOrigin;
}

vn_macros::vn_record! {
    contract: "src/modules/runtime/fs/fs_runtime.vn",
    class: "RawStats",
    pub struct RawStatsRec;
}

natives! {
    class RawStats {}
    openRead(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        allow_read(ctx, &full)?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(&full)
            .map_err(|e| fs_error(e, &full.to_string_lossy()))?;
        Ok(descriptors::register(file))
    },
    openWrite(ctx, path, append) => {
        let full = ctx.resolve_fs_path(path);
        allow_write(ctx, &full)?;
        let file = if append {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&full)
        } else {
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&full)
        }
        .map_err(|e| fs_error(e, &full.to_string_lossy()))?;
        Ok(descriptors::register(file))
    },
    readFd(fd, len) => {
        let buf = descriptors::read_some(&descriptors::handle(fd)?, len)?;
        String::from_utf8(buf).map_err(encoding_error)
    },
    readFdBytes(ctx, fd, len) => {
        let buf = descriptors::read_some(&descriptors::handle(fd)?, len)?;
        Ok(ctx.alloc_buffer_from_bytes(&buf))
    },
    writeFd(fd, data) => {
        descriptors::write_all(&descriptors::handle(fd)?, data.as_bytes())?;
        Ok(data.len() as i64)
    },
    writeFdBytes(ctx, fd, data) => {
        let bytes = bytes_arg(ctx, data)?;
        descriptors::write_all(&descriptors::handle(fd)?, &bytes)?;
        Ok(bytes.len() as i64)
    },
    readFileBytes(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        allow_read(ctx, &full)?;
        Ok(ctx.alloc_buffer_from_bytes(&paths::read_bytes(&full)?))
    },
    readFileBytesAsync(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        allow_read(ctx, &full)?;
        Ok(blocking::task(ctx, move || paths::read_bytes(&full).map(SendValue::Bytes)))
    },
    writeFileBytes(ctx, path, data) => {
        let full = ctx.resolve_fs_path(path);
        allow_write(ctx, &full)?;
        paths::write(&full, &bytes_arg(ctx, data)?)
    },
    writeFileBytesAsync(ctx, path, data) => {
        let full = ctx.resolve_fs_path(path);
        allow_write(ctx, &full)?;
        let bytes = bytes_arg(ctx, data)?;
        Ok(blocking::task(ctx, move || paths::write(&full, &bytes).map(|()| SendValue::Null)))
    },
    seek(ctx, fd, offset, origin) => {
        let origin = SeekOrigin::from_value(ctx, origin)
            .ok_or_else(|| type_error("expected a SeekOrigin"))?;
        let file_arc = descriptors::handle(fd)?;
        let mut file = file_arc.lock().unwrap_or_else(|e| e.into_inner());

        use std::io::{Seek, SeekFrom};
        let seek_from = match origin {
            SeekOrigin::Start => SeekFrom::Start(offset as u64),
            SeekOrigin::Current => SeekFrom::Current(offset),
            SeekOrigin::End => SeekFrom::End(offset),
        };
        let pos = file.seek(seek_from).map_err(io_error)?;
        Ok(pos as i64)
    },
    close(fd) => descriptors::unregister(fd),
    exists(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        allow_read(ctx, &full)?;
        std::path::Path::new(&full).try_exists().map_err(|e| fs_error(e, &full.to_string_lossy()))
    },
    stat(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        if !ctx.check_fs_read_path(&full) {
            return Err(capability_denied(Capability::FsRead.name(), &full.to_string_lossy()));
        }
        let meta = fs::metadata(&full).map_err(|e| fs_error(e, &full.to_string_lossy()))?;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64);
        RawStatsRec {
            size: meta.len() as i64,
            is_dir: meta.is_dir(),
            is_file: meta.is_file(),
            mtime: mtime.map_or_else(VmValue::null, VmValue::from_int),
        }
        .alloc(ctx)
        .map_err(NativeError::failure)
    },
    mkdir(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        if !ctx.check_fs_write_path(&full) {
            return Err(capability_denied(Capability::FsWrite.name(), &full.to_string_lossy()));
        }
        fs::create_dir(&full).map_err(|e| fs_error(e, &full.to_string_lossy()))
    },
    mkdirAll(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        if !ctx.check_fs_write_path(&full) {
            return Err(capability_denied(Capability::FsWrite.name(), &full.to_string_lossy()));
        }
        fs::create_dir_all(&full).map_err(|e| fs_error(e, &full.to_string_lossy()))
    },
    readDir(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        allow_read(ctx, &full)?;
        Ok(strings(ctx, paths::read_dir(&full)?))
    },
    readDirAsync(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        allow_read(ctx, &full)?;
        Ok(blocking::task(ctx, move || paths::read_dir(&full).map(text_array)))
    },
    remove(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        if !ctx.check_fs_write_path(&full) {
            return Err(capability_denied(Capability::FsWrite.name(), &full.to_string_lossy()));
        }
        if full.is_dir() {
            fs::remove_dir(&full).map_err(|e| fs_error(e, &full.to_string_lossy()))
        } else {
            fs::remove_file(&full).map_err(|e| fs_error(e, &full.to_string_lossy()))
        }
    },
    removeAll(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        allow_write(ctx, &full)?;
        paths::remove_all(&full)
    },
    removeAllAsync(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        allow_write(ctx, &full)?;
        Ok(blocking::task(ctx, move || paths::remove_all(&full).map(|()| SendValue::Null)))
    },
    rename(ctx, from, to) => {
        let full_from = ctx.resolve_fs_path(from);
        let full_to = ctx.resolve_fs_path(to);
        if !ctx.check_fs_read_path(&full_from) || !ctx.check_fs_write_path(&full_to) {
            return Err(capability_denied(Capability::FsWrite.name(), &full_to.to_string_lossy()));
        }
        fs::rename(&full_from, &full_to).map_err(|e| fs_error(e, &full_from.to_string_lossy()))
    },
    copyFile(ctx, from, to) => {
        let full_from = ctx.resolve_fs_path(from);
        let full_to = ctx.resolve_fs_path(to);
        allow_copy(ctx, &full_from, &full_to)?;
        paths::copy(&full_from, &full_to)
    },
    copyFileAsync(ctx, from, to) => {
        let full_from = ctx.resolve_fs_path(from);
        let full_to = ctx.resolve_fs_path(to);
        allow_copy(ctx, &full_from, &full_to)?;
        Ok(blocking::task(ctx, move || paths::copy(&full_from, &full_to).map(|()| SendValue::Null)))
    },
    readFileText(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        allow_read(ctx, &full)?;
        paths::read_text(&full)
    },
    readFileTextAsync(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        allow_read(ctx, &full)?;
        Ok(blocking::task(ctx, move || paths::read_text(&full).map(SendValue::Str)))
    },
    writeFileText(ctx, path, data) => {
        let full = ctx.resolve_fs_path(path);
        allow_write(ctx, &full)?;
        paths::write(&full, data.as_bytes())
    },
    writeFileTextAsync(ctx, path, data) => {
        let full = ctx.resolve_fs_path(path);
        allow_write(ctx, &full)?;
        let data = data.to_owned();
        Ok(blocking::task(ctx, move || paths::write(&full, data.as_bytes()).map(|()| SendValue::Null)))
    },
    appendFileText(ctx, path, data) => {
        let full = ctx.resolve_fs_path(path);
        allow_write(ctx, &full)?;
        paths::append(&full, data.as_bytes())
    },
    appendFileTextAsync(ctx, path, data) => {
        let full = ctx.resolve_fs_path(path);
        allow_write(ctx, &full)?;
        let data = data.to_owned();
        Ok(blocking::task(ctx, move || paths::append(&full, data.as_bytes()).map(|()| SendValue::Null)))
    },
    readLines(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        allow_read(ctx, &full)?;
        Ok(strings(ctx, paths::read_lines(&full)?))
    },
    readLinesAsync(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        allow_read(ctx, &full)?;
        Ok(blocking::task(ctx, move || paths::read_lines(&full).map(text_array)))
    },
    readFdAll(fd) => {
        let buf = descriptors::read_all(&descriptors::handle(fd)?)?;
        String::from_utf8(buf).map_err(encoding_error)
    },
    readFdAllBytes(ctx, fd) => {
        let buf = descriptors::read_all(&descriptors::handle(fd)?)?;
        Ok(ctx.alloc_buffer_from_bytes(&buf))
    },
    readFdBytesAsync(ctx, fd, len) => {
        let file = descriptors::handle(fd)?;
        Ok(blocking::task(ctx, move || descriptors::read_some(&file, len).map(SendValue::Bytes)))
    },
    readFdAllBytesAsync(ctx, fd) => {
        let file = descriptors::handle(fd)?;
        Ok(blocking::task(ctx, move || descriptors::read_all(&file).map(SendValue::Bytes)))
    },
    writeFdBytesAsync(ctx, fd, data) => {
        let file = descriptors::handle(fd)?;
        let bytes = bytes_arg(ctx, data)?;
        Ok(blocking::task(ctx, move || {
            descriptors::write_all(&file, &bytes).map(|()| SendValue::Int(bytes.len() as i64))
        }))
    },
    tempDir() => std::env::temp_dir().to_string_lossy().into_owned(),
}
