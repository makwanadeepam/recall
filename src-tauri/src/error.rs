//! Application error type shared across all Tauri commands.
//!
//! Errors serialize to a small JSON object `{ kind, message, hint? }` so the
//! frontend can show understandable, actionable messages instead of raw codes.

use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;

use crate::ai::AiError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Db(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    Invalid(String),

    #[error("Secure storage error: {0}")]
    Keychain(String),

    #[error("File error: {0}")]
    Io(String),

    #[error("Data format error: {0}")]
    Serde(String),

    #[error(transparent)]
    Ai(#[from] AiError),

    #[error("{0}")]
    Other(String),
}

impl AppError {
    /// A short machine-readable category the UI can branch on.
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Db(_) => "db",
            AppError::NotFound(_) => "not_found",
            AppError::Invalid(_) => "invalid",
            AppError::Keychain(_) => "keychain",
            AppError::Io(_) => "io",
            AppError::Serde(_) => "serde",
            AppError::Ai(e) => e.kind(),
            AppError::Other(_) => "other",
        }
    }

    /// An optional actionable hint for the user.
    pub fn hint(&self) -> Option<String> {
        match self {
            AppError::Ai(e) => e.hint(),
            AppError::Keychain(_) => Some(
                "Your operating system's secure credential store may be locked or unavailable."
                    .into(),
            ),
            _ => None,
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("AppError", 3)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.serialize_field("hint", &self.hint())?;
        s.end()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("record not found".into()),
            other => AppError::Db(other.to_string()),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Serde(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
