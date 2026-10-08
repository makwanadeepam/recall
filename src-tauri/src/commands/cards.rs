//! Commands for recall cards.

use tauri::State;

use crate::db::models::{Card, CardInput};
use crate::db::{now_ms, repository as repo};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub fn create_card(state: State<AppState>, input: CardInput) -> AppResult<Card> {
    let conn = state.db()?;
    repo::create_card(&conn, &input, now_ms())
}

#[tauri::command]
pub fn update_card(state: State<AppState>, id: i64, input: CardInput) -> AppResult<Card> {
    let conn = state.db()?;
    repo::update_card(&conn, id, &input, now_ms())
}

#[tauri::command]
pub fn get_card(state: State<AppState>, id: i64) -> AppResult<Card> {
    let conn = state.db()?;
    repo::get_card(&conn, id)
}

#[tauri::command]
pub fn delete_card(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db()?;
    repo::delete_card(&conn, id)
}

#[tauri::command]
pub fn list_cards(state: State<AppState>, item: i64) -> AppResult<Vec<Card>> {
    let conn = state.db()?;
    repo::list_cards_for_item(&conn, item)
}
