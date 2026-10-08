//! App settings: theme + AI provider configuration.
//!
//! Non-sensitive settings live in the SQLite `settings` table. API keys live in
//! the OS keychain and are never returned here — the frontend only learns
//! whether a key *exists* via `has_key`.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ai::{self, AiConfig};
use crate::db::repository as repo;
use crate::error::AppResult;
use crate::security::keychain;
use crate::state::AppState;

pub const KEY_THEME: &str = "theme";
pub const KEY_AI_PROVIDER: &str = "ai_provider";
pub const KEY_AI_BASE_URL: &str = "ai_base_url";
pub const KEY_AI_MODEL: &str = "ai_model";
pub const KEY_AI_ENABLED: &str = "ai_enabled";
pub const KEY_AI_CONFIRM: &str = "ai_confirm_before_send";

#[derive(Debug, Serialize)]
pub struct AiConfigView {
    pub provider: String,
    pub base_url: String,
    pub effective_base_url: String,
    pub default_base_url: String,
    pub model: String,
    pub enabled: bool,
    pub confirm_before_send: bool,
    pub has_key: bool,
    pub is_local: bool,
    pub providers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AppSettings {
    pub theme: String,
    pub ai: AiConfigView,
}

#[derive(Debug, Deserialize)]
pub struct AiConfigInput {
    pub provider: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub confirm_before_send: bool,
}

fn default_true() -> bool {
    true
}

/// Load the persisted AI config (not the key). Used by AI commands too.
pub fn load_ai_config(conn: &rusqlite::Connection) -> AppResult<AiConfig> {
    Ok(AiConfig {
        provider: repo::get_setting(conn, KEY_AI_PROVIDER)?.unwrap_or_default(),
        base_url: repo::get_setting(conn, KEY_AI_BASE_URL)?.unwrap_or_default(),
        model: repo::get_setting(conn, KEY_AI_MODEL)?.unwrap_or_default(),
        enabled: repo::get_setting(conn, KEY_AI_ENABLED)?.as_deref() == Some("true"),
    })
}

fn confirm_before_send(conn: &rusqlite::Connection) -> AppResult<bool> {
    // Defaults to true (privacy-first) when unset.
    Ok(repo::get_setting(conn, KEY_AI_CONFIRM)?.as_deref() != Some("false"))
}

fn is_local_endpoint(config: &AiConfig) -> bool {
    if config.provider.eq_ignore_ascii_case("ollama") {
        return true;
    }
    let base = ai::effective_base_url(config).unwrap_or_default().to_lowercase();
    ["localhost", "127.0.0.1", "0.0.0.0", "[::1]", "::1"]
        .iter()
        .any(|h| base.contains(h))
}

fn build_view(conn: &rusqlite::Connection) -> AppResult<AiConfigView> {
    let config = load_ai_config(conn)?;
    let has_key = if config.provider.is_empty() {
        false
    } else {
        keychain::has_key(&config.provider)
    };
    let effective_base_url = ai::effective_base_url(&config).unwrap_or_default();
    Ok(AiConfigView {
        default_base_url: ai::default_base_url(&config.provider).to_string(),
        is_local: is_local_endpoint(&config),
        effective_base_url,
        provider: config.provider,
        base_url: config.base_url,
        model: config.model,
        enabled: config.enabled,
        confirm_before_send: confirm_before_send(conn)?,
        has_key,
        providers: ai::PROVIDERS.iter().map(|s| s.to_string()).collect(),
    })
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> AppResult<AppSettings> {
    let conn = state.db()?;
    let theme = repo::get_setting(&conn, KEY_THEME)?.unwrap_or_else(|| "system".into());
    Ok(AppSettings {
        theme,
        ai: build_view(&conn)?,
    })
}

#[tauri::command]
pub fn set_theme(state: State<AppState>, theme: String) -> AppResult<()> {
    let t = match theme.as_str() {
        "light" | "dark" | "system" => theme,
        _ => "system".into(),
    };
    let conn = state.db()?;
    repo::set_setting(&conn, KEY_THEME, &t)
}

#[tauri::command]
pub fn get_ai_config(state: State<AppState>) -> AppResult<AiConfigView> {
    let conn = state.db()?;
    build_view(&conn)
}

#[tauri::command]
pub fn set_ai_config(state: State<AppState>, config: AiConfigInput) -> AppResult<AiConfigView> {
    // Validate the provider is one we understand (also validates family).
    ai::family_for(&config.provider)?;
    let conn = state.db()?;
    repo::set_setting(&conn, KEY_AI_PROVIDER, config.provider.trim())?;
    repo::set_setting(&conn, KEY_AI_BASE_URL, config.base_url.trim())?;
    repo::set_setting(&conn, KEY_AI_MODEL, config.model.trim())?;
    repo::set_setting(&conn, KEY_AI_ENABLED, if config.enabled { "true" } else { "false" })?;
    repo::set_setting(
        &conn,
        KEY_AI_CONFIRM,
        if config.confirm_before_send { "true" } else { "false" },
    )?;
    build_view(&conn)
}

#[tauri::command]
pub fn ai_set_key(provider: String, key: String) -> AppResult<()> {
    keychain::set_key(&provider, &key)
}

#[tauri::command]
pub fn ai_delete_key(provider: String) -> AppResult<()> {
    keychain::delete_key(&provider)
}

#[tauri::command]
pub fn ai_has_key(provider: String) -> AppResult<bool> {
    Ok(keychain::has_key(&provider))
}
