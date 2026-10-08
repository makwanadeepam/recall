//! Commands for building a review queue and recording review outcomes.

use serde::Deserialize;
use tauri::State;

use crate::db::models::{Card, DueCard, ScheduleUpdate};
use crate::db::{now_ms, repository as repo};
use crate::error::AppResult;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct DueQuery {
    pub limit: Option<i64>,
    pub include_new: Option<bool>,
    pub new_limit: Option<i64>,
    pub item_id: Option<i64>,
    pub tag: Option<String>,
}

#[tauri::command]
pub fn due_cards(state: State<AppState>, query: DueQuery) -> AppResult<Vec<DueCard>> {
    let conn = state.db()?;
    repo::due_cards(
        &conn,
        now_ms(),
        query.limit.unwrap_or(200).clamp(1, 1000),
        query.include_new.unwrap_or(true),
        query.new_limit.unwrap_or(20).clamp(0, 1000),
        query.item_id,
        query.tag.as_deref(),
    )
}

#[derive(Debug, Deserialize)]
pub struct ReviewInput {
    pub card_id: i64,
    pub rating: String,
    #[serde(default)]
    pub user_answer: Option<String>,
    #[serde(default)]
    pub ai_evaluation: Option<String>,
    pub schedule: ScheduleUpdate,
}

#[tauri::command]
pub fn record_review(state: State<AppState>, input: ReviewInput) -> AppResult<Card> {
    let mut conn = state.db()?;
    repo::record_review(
        &mut conn,
        input.card_id,
        &input.rating,
        input.user_answer.as_deref(),
        input.ai_evaluation.as_deref(),
        &input.schedule,
        now_ms(),
    )
}
