//! Database module: connection setup, migrations, models, and the repository.

pub mod migrations;
pub mod models;
pub mod repository;

#[cfg(test)]
mod tests;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::error::AppResult;

/// Current wall-clock time in milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Open (creating if needed) the SQLite database at `path`, apply pragmas, and
/// run migrations. Returns a ready-to-use connection.
pub fn open(path: &Path) -> AppResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)
        .map_err(|e| crate::error::AppError::Db(format!("cannot open database: {e}")))?;
    repository::configure(&conn)?;
    migrations::run(&mut conn)?;
    Ok(conn)
}

/// Open an in-memory database (used by tests).
#[cfg(test)]
pub fn open_in_memory() -> AppResult<Connection> {
    let mut conn = Connection::open_in_memory()
        .map_err(|e| crate::error::AppError::Db(e.to_string()))?;
    repository::configure(&conn)?;
    migrations::run(&mut conn)?;
    Ok(conn)
}
