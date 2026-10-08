//! Dashboard statistics and tag listing.

use tauri::State;

use crate::db::models::DashboardStats;
use crate::db::{now_ms, repository as repo};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub fn dashboard_stats(state: State<AppState>) -> AppResult<DashboardStats> {
    let conn = state.db()?;
    repo::dashboard_stats(&conn, now_ms())
}

#[tauri::command]
pub fn list_tags(state: State<AppState>) -> AppResult<Vec<String>> {
    let conn = state.db()?;
    repo::list_all_tags(&conn)
}
