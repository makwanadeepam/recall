//! API-key storage backed by the operating system's secure credential store:
//! macOS Keychain, Windows Credential Manager, or the Linux Secret Service.
//!
//! Keys are NEVER written to SQLite, NEVER logged, and NEVER returned to the
//! frontend. The frontend can only ask *whether* a key exists; the plaintext
//! key stays in Rust and is used solely to authenticate the configured provider.

use keyring::{Entry, Error as KeyringError};

use crate::error::{AppError, AppResult};

/// Keychain service namespace for this app's API keys.
const SERVICE: &str = "com.recall.app.ai-key";

/// Restrict provider identifiers used as the keychain "account" to a safe set.
fn sanitize_provider(provider: &str) -> AppResult<String> {
    let p = provider.trim().to_lowercase();
    if p.is_empty() || p.len() > 64 {
        return Err(AppError::Invalid("invalid provider identifier".into()));
    }
    if !p.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) {
        return Err(AppError::Invalid("invalid provider identifier".into()));
    }
    Ok(p)
}

fn entry(provider: &str) -> AppResult<Entry> {
    let account = sanitize_provider(provider)?;
    Entry::new(SERVICE, &account).map_err(|e| AppError::Keychain(e.to_string()))
}

/// Store (or replace) the API key for a provider.
pub fn set_key(provider: &str, key: &str) -> AppResult<()> {
    if key.trim().is_empty() {
        return Err(AppError::Invalid("API key cannot be empty".into()));
    }
    entry(provider)?
        .set_password(key)
        .map_err(|e| AppError::Keychain(e.to_string()))
}

/// Fetch the API key for a provider, or `None` if none is stored.
pub fn get_key(provider: &str) -> AppResult<Option<String>> {
    match entry(provider)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Keychain(e.to_string())),
    }
}

/// Whether a key exists for a provider (no secret is exposed).
pub fn has_key(provider: &str) -> bool {
    matches!(get_key(provider), Ok(Some(_)))
}

/// Delete the stored key for a provider. Succeeds even if nothing was stored.
pub fn delete_key(provider: &str) -> AppResult<()> {
    match entry(provider)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Keychain(e.to_string())),
    }
}
