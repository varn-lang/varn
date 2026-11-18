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

fn allow_read(ctx: &mut dyn NativeCtx, path: &str) -> Result<(), NativeError> {
    if ctx.check_fs_read(path) {
        Ok(())
    } else {
        Err(capability_denied(Capability::FsRead.as_str(), path))
    }
}

fn allow_write(ctx: &mut dyn NativeCtx, path: &str) -> Result<(), NativeError> {
    if ctx.check_fs_write(path) {
        Ok(())
    } else {
        Err(capability_denied(Capability::FsWrite.as_str(), path))
    }
}

fn allow_copy(ctx: &mut dyn NativeCtx, from: &str, to: &str) -> Result<(), NativeError> {
    if !ctx.check_fs_read(from) {
        return Err(capability_denied(Capability::FsRead.as_str(), from));
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
        if !ctx.check_fs_read(path) {
            return Err(capability_denied(Capability::FsRead.as_str(), path));
        }
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(|e| fs_error(e, path))?;
        Ok(descriptors::register(file))
    },
    openWrite(ctx, path, append) => {
        if !ctx.check_fs_write(path) {
            return Err(capability_denied(Capability::FsWrite.as_str(), path));
        }
        let file = if append {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
        } else {
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
        }
        .map_err(|e| fs_error(e, path))?;
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
        allow_read(ctx, path)?;
        Ok(ctx.alloc_buffer_from_bytes(&paths::read_bytes(path)?))
    },
    readFileBytesAsync(ctx, path) => {
        allow_read(ctx, path)?;
        let path = path.to_owned();
        Ok(blocking::task(ctx, move || paths::read_bytes(&path).map(SendValue::Bytes)))
    },
    writeFileBytes(ctx, path, data) => {
        allow_write(ctx, path)?;
        paths::write(path, &bytes_arg(ctx, data)?)
    },
    writeFileBytesAsync(ctx, path, data) => {
        allow_write(ctx, path)?;
        let (path, bytes) = (path.to_owned(), bytes_arg(ctx, data)?);
        Ok(blocking::task(ctx, move || paths::write(&path, &bytes).map(|()| SendValue::Null)))
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
        allow_read(ctx, path)?;
        std::path::Path::new(path).try_exists().map_err(|e| fs_error(e, path))
    },
    stat(ctx, path) => {
        if !ctx.check_fs_read(path) {
            return Err(capability_denied(Capability::FsRead.as_str(), path));
        }
        let meta = fs::metadata(path).map_err(|e| fs_error(e, path))?;
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
        if !ctx.check_fs_write(path) {
            return Err(capability_denied(Capability::FsWrite.as_str(), path));
        }
        fs::create_dir(path).map_err(|e| fs_error(e, path))
    },
    mkdirAll(ctx, path) => {
        if !ctx.check_fs_write(path) {
            return Err(capability_denied(Capability::FsWrite.as_str(), path));
        }
        fs::create_dir_all(path).map_err(|e| fs_error(e, path))
    },
    readDir(ctx, path) => {
        allow_read(ctx, path)?;
        Ok(strings(ctx, paths::read_dir(path)?))
    },
    readDirAsync(ctx, path) => {
        allow_read(ctx, path)?;
        let path = path.to_owned();
        Ok(blocking::task(ctx, move || paths::read_dir(&path).map(text_array)))
    },
    remove(ctx, path) => {
        if !ctx.check_fs_write(path) {
            return Err(capability_denied(Capability::FsWrite.as_str(), path));
        }
        let p = std::path::Path::new(path);
        if p.is_dir() {
            fs::remove_dir(p).map_err(|e| fs_error(e, path))
        } else {
            fs::remove_file(p).map_err(|e| fs_error(e, path))
        }
    },
    removeAll(ctx, path) => {
        allow_write(ctx, path)?;
        paths::remove_all(path)
    },
    removeAllAsync(ctx, path) => {
        allow_write(ctx, path)?;
        let path = path.to_owned();
        Ok(blocking::task(ctx, move || paths::remove_all(&path).map(|()| SendValue::Null)))
    },
    rename(ctx, from, to) => {
        if !ctx.check_fs_read(from) || !ctx.check_fs_write(to) {
            return Err(capability_denied(Capability::FsWrite.as_str(), to));
        }
        fs::rename(from, to).map_err(|e| fs_error(e, from))
    },
    copyFile(ctx, from, to) => {
        allow_copy(ctx, from, to)?;
        paths::copy(from, to)
    },
    copyFileAsync(ctx, from, to) => {
        allow_copy(ctx, from, to)?;
        let (from, to) = (from.to_owned(), to.to_owned());
        Ok(blocking::task(ctx, move || paths::copy(&from, &to).map(|()| SendValue::Null)))
    },
    readFileText(ctx, path) => {
        allow_read(ctx, path)?;
        paths::read_text(path)
    },
    readFileTextAsync(ctx, path) => {
        allow_read(ctx, path)?;
        let path = path.to_owned();
        Ok(blocking::task(ctx, move || paths::read_text(&path).map(SendValue::Str)))
    },
    writeFileText(ctx, path, data) => {
        allow_write(ctx, path)?;
        paths::write(path, data.as_bytes())
    },
    writeFileTextAsync(ctx, path, data) => {
        allow_write(ctx, path)?;
        let (path, data) = (path.to_owned(), data.to_owned());
        Ok(blocking::task(ctx, move || paths::write(&path, data.as_bytes()).map(|()| SendValue::Null)))
    },
    appendFileText(ctx, path, data) => {
        allow_write(ctx, path)?;
        paths::append(path, data.as_bytes())
    },
    appendFileTextAsync(ctx, path, data) => {
        allow_write(ctx, path)?;
        let (path, data) = (path.to_owned(), data.to_owned());
        Ok(blocking::task(ctx, move || paths::append(&path, data.as_bytes()).map(|()| SendValue::Null)))
    },
    readLines(ctx, path) => {
        allow_read(ctx, path)?;
        Ok(strings(ctx, paths::read_lines(path)?))
    },
    readLinesAsync(ctx, path) => {
        allow_read(ctx, path)?;
        let path = path.to_owned();
        Ok(blocking::task(ctx, move || paths::read_lines(&path).map(text_array)))
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
