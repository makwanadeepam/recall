//! A tiny forward-only migration runner.
//!
//! Migrations are plain `.sql` files embedded at compile time. The database's
//! `PRAGMA user_version` tracks how many have been applied; on startup we apply
//! any with a higher version inside a transaction. Adding a feature later means
//! adding a new file and a new `(version, sql)` entry here — nothing else.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Ordered list of (version, sql). Versions must be strictly increasing.
const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../../migrations/0001_init.sql"))];

pub fn run(conn: &mut Connection) -> AppResult<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    for (version, sql) in MIGRATIONS {
        if *version > current {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)
                .map_err(|e| AppError::Db(format!("migration {version} failed: {e}")))?;
            // PRAGMA statements can't be parameterized.
            tx.execute_batch(&format!("PRAGMA user_version = {version};"))?;
            tx.commit()?;
        }
    }

    Ok(())
}

/// The highest known schema version (useful for diagnostics/tests).
#[allow(dead_code)]
pub fn latest_version() -> i64 {
    MIGRATIONS.last().map(|(v, _)| *v).unwrap_or(0)
}
