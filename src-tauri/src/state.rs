//! Shared application state managed by Tauri and injected into commands.

use std::sync::{Mutex, OnceLock};

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub struct AppState {
    /// The single SQLite connection, guarded by a mutex. A desktop app has one
    /// user, so a connection pool would be overkill.
    pub db: Mutex<Connection>,
    /// The HTTP client is built on first AI use, not at startup — see the
    /// performance goal "do not initialize AI clients until needed".
    http: OnceLock<reqwest::Client>,
}

impl AppState {
    pub fn new(conn: Connection) -> Self {
        Self {
            db: Mutex::new(conn),
            http: OnceLock::new(),
        }
    }

    /// Lock the database connection.
    pub fn db(&self) -> AppResult<std::sync::MutexGuard<'_, Connection>> {
        self.db
            .lock()
            .map_err(|_| AppError::Other("database lock poisoned".into()))
    }

    /// Get (or lazily create) the shared HTTP client used for AI requests.
    pub fn http_client(&self) -> &reqwest::Client {
        self.http.get_or_init(|| {
            reqwest::Client::builder()
                .user_agent("Recall/0.1")
                .build()
                .unwrap_or_default()
        })
    }
}
