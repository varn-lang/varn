use rusqlite::Connection;
use std::time::Duration;
use vn_macros::natives;
use vn_types::value::SendValue;
use vn_core::sandbox::Capability;
use vn_types::{NativeCtx, NativeError};

use crate::runtime::blocking;

#[path = "handles.rs"]
mod handles;
#[path = "rows.rs"]
mod rows;
use handles::{failure, lookup, register, release};

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const MEMORY: &str = ":memory:";


fn open_connection(path: &std::path::Path, display: &str) -> Result<Connection, NativeError> {
    let conn = if display == MEMORY {
        Connection::open_in_memory()
    } else {
        Connection::open(path)
    }
    .map_err(|e| failure(e, display))?;
    conn.busy_timeout(BUSY_TIMEOUT).map_err(|e| failure(e, display))?;
    conn.pragma_update(None, "foreign_keys", true)
        .map_err(|e| failure(e, display))?;
    Ok(conn)
}

fn allow_path(
    ctx: &mut dyn NativeCtx,
    path: &std::path::Path,
    display: &str,
) -> Result<(), NativeError> {
    if display == MEMORY {
        return Ok(());
    }
    if !ctx.check_fs_read_path(path) {
        return Err(NativeError::capability(Capability::FsRead.name(), display));
    }
    if !ctx.check_fs_write_path(path) {
        return Err(NativeError::capability(Capability::FsWrite.name(), display));
    }
    Ok(())
}

natives! {
    open(ctx, path) => {
        let full = ctx.resolve_fs_path(path);
        allow_path(ctx, &full, path)?;
        Ok(register(open_connection(&full, path)?))
    },
    close(db_id) => {
        release(db_id);
        Ok(())
    },
    script(db_id, sql) => lookup(db_id)?.lock().execute_batch(sql).map_err(|e| failure(e, sql)),
    execute(ctx, db_id, sql, params) => {
        let bound = rows::params(ctx, params)?;
        rows::execute(&lookup(db_id)?.lock(), sql, bound)
    },
    executeAsync(ctx, db_id, sql, params) => {
        let bound = rows::params(ctx, params)?;
        let db = lookup(db_id)?;
        let sql = sql.to_owned();
        Ok(blocking::task(ctx, move || rows::execute(&db.lock(), &sql, bound).map(SendValue::Int)))
    },
    query(ctx, db_id, sql, params, limit) => {
        let bound = rows::params(ctx, params)?;
        let found = rows::select(&lookup(db_id)?.lock(), sql, bound, limit)?;
        Ok(found.iter().map(|row| row.to_value_ctx(ctx)).collect())
    },
    queryAsync(ctx, db_id, sql, params, limit) => {
        let bound = rows::params(ctx, params)?;
        let db = lookup(db_id)?;
        let sql = sql.to_owned();
        Ok(blocking::task(ctx, move || {
            rows::select(&db.lock(), &sql, bound, limit).map(SendValue::Array)
        }))
    },
}
