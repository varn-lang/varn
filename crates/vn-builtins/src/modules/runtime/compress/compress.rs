use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};
use flate2::Compression;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use vn_core::StdErrorClass;
use vn_macros::natives;
use vn_types::NativeError;


fn zip_dir_recursive(
    writer: &mut zip::ZipWriter<File>,
    base_dir: &Path,
    current_dir: &Path,
) -> Result<(), String> {
    let entries = fs::read_dir(current_dir).map_err(|e| format!("Zip read dir error: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Zip dir entry error: {e}"))?;
        let path = entry.path();
        let rel_path = path
            .strip_prefix(base_dir)
            .map_err(|e| format!("Zip strip prefix error: {e}"))?;
        let name = rel_path.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            writer
                .add_directory(&name, options)
                .map_err(|e| format!("Zip add dir error: {e}"))?;
            zip_dir_recursive(writer, base_dir, &path)?;
        } else {
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            writer
                .start_file(&name, options)
                .map_err(|e| format!("Zip start file error: {e}"))?;
            let mut f = File::open(&path).map_err(|e| format!("Zip open file error: {e}"))?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)
                .map_err(|e| format!("Zip read file error: {e}"))?;
            writer
                .write_all(&buf)
                .map_err(|e| format!("Zip write error: {e}"))?;
        }
    }
    Ok(())
}

natives! {
    gzip(ctx, data) => {
        let input_bytes = if let Some(b) = ctx.buffer_to_bytes(data) {
            b
        } else if ctx.is_string(data) {
            ctx.str_repr(data).into_bytes()
        } else {
            return Err(NativeError::type_error("gzip expects Bytes or str".to_string()));
        };
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&input_bytes).map_err(|e| format!("gzip compression error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        let compressed = encoder.finish().map_err(|e| format!("gzip finish error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        Ok(ctx.alloc_buffer_from_bytes(&compressed))
    },
    gunzip(ctx, bytes) => {
        let raw_bytes = ctx.buffer_to_bytes(bytes).ok_or_else(|| "gunzip expects Bytes".to_string()).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        let mut decoder = GzDecoder::new(&raw_bytes[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).map_err(|e| format!("gunzip decompression error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        Ok(ctx.alloc_buffer_from_bytes(&decompressed))
    },
    deflate(ctx, data) => {
        let input_bytes = if let Some(b) = ctx.buffer_to_bytes(data) {
            b
        } else if ctx.is_string(data) {
            ctx.str_repr(data).into_bytes()
        } else {
            return Err(NativeError::type_error("deflate expects Bytes or str".to_string()));
        };
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&input_bytes).map_err(|e| format!("deflate compression error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        let compressed = encoder.finish().map_err(|e| format!("deflate finish error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        Ok(ctx.alloc_buffer_from_bytes(&compressed))
    },
    inflate(ctx, bytes) => {
        let raw_bytes = ctx.buffer_to_bytes(bytes).ok_or_else(|| "inflate expects Bytes".to_string()).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        let mut decoder = ZlibDecoder::new(&raw_bytes[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).map_err(|e| format!("inflate decompression error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        Ok(ctx.alloc_buffer_from_bytes(&decompressed))
    },
    tarCreate(ctx, source_dir, tar_path) => {
        let tar_path = ctx.resolve_fs_path(tar_path);
        let tar_file = File::create(&tar_path).map_err(|e| format!("Tar create error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        let mut tar_builder = tar::Builder::new(tar_file);
        let source_dir = ctx.resolve_fs_path(source_dir);
        tar_builder.append_dir_all(".", &source_dir).map_err(|e| format!("Tar append error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        tar_builder.finish().map_err(|e| format!("Tar finish error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        Ok(())
    },
    tarExtract(ctx, tar_path, dest_dir) => {
        let tar_path = ctx.resolve_fs_path(tar_path);
        let dest_dir = ctx.resolve_fs_path(dest_dir);
        let tar_file = File::open(&tar_path).map_err(|e| format!("Tar open error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        let mut archive = tar::Archive::new(tar_file);
        archive.unpack(&dest_dir).map_err(|e| format!("Tar unpack error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        Ok(())
    },
    zipCreate(ctx, source_dir, zip_path) => {
        let zip_path = ctx.resolve_fs_path(zip_path);
        let zip_file = File::create(&zip_path).map_err(|e| format!("Zip create error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        let mut writer = zip::ZipWriter::new(zip_file);
        let source_dir = ctx.resolve_fs_path(source_dir);
        let base = Path::new(&source_dir);
        zip_dir_recursive(&mut writer, base, base).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        writer.finish().map_err(|e| format!("Zip finish error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        Ok(())
    },
    zipExtract(ctx, zip_path, dest_dir) => {
        let zip_path = ctx.resolve_fs_path(zip_path);
        let dest_dir = ctx.resolve_fs_path(dest_dir);
        let zip_file = File::open(&zip_path).map_err(|e| format!("Zip open error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        let mut archive = zip::ZipArchive::new(zip_file).map_err(|e| format!("Zip archive error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        archive.extract(&dest_dir).map_err(|e| format!("Zip extract error: {e}")).map_err(|e| NativeError::std(StdErrorClass::Io, e))?;
        Ok(())
    },
}
