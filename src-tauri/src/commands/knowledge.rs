//! Commands for the knowledge library (CRUD + search-driven listing).

use tauri::State;

use crate::db::models::{KnowledgeItem, KnowledgeItemInput, KnowledgeItemSummary};
use crate::db::{now_ms, repository as repo};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub fn create_item(state: State<AppState>, input: KnowledgeItemInput) -> AppResult<KnowledgeItem> {
    let conn = state.db()?;
    repo::create_item(&conn, &input, now_ms())
}

#[tauri::command]
pub fn update_item(
    state: State<AppState>,
    id: i64,
    input: KnowledgeItemInput,
) -> AppResult<KnowledgeItem> {
    let conn = state.db()?;
    repo::update_item(&conn, id, &input, now_ms())
}

#[tauri::command]
pub fn get_item(state: State<AppState>, id: i64) -> AppResult<KnowledgeItem> {
    let conn = state.db()?;
    repo::get_item(&conn, id)
}

#[tauri::command]
pub fn delete_item(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db()?;
    repo::delete_item(&conn, id)
}

#[tauri::command]
pub fn list_items(
    state: State<AppState>,
    search: Option<String>,
    tag: Option<String>,
) -> AppResult<Vec<KnowledgeItemSummary>> {
    let conn = state.db()?;
    repo::list_items(&conn, search.as_deref(), tag.as_deref(), now_ms())
}
