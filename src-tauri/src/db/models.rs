//! Serde-serializable data models. These are the shapes crossing the IPC
//! boundary, so the field names here match the TypeScript types in `src/db/models.ts`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A note/passage the user wants to remember. May own many cards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Lightweight row for the library list — omits full content, adds counts.
#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeItemSummary {
    pub id: i64,
    pub title: String,
    pub excerpt: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub tags: Vec<String>,
    pub card_count: i64,
    pub due_count: i64,
}

/// A single recall card belonging to a knowledge item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: i64,
    pub knowledge_item_id: i64,
    pub question: String,
    pub answer: String,
    #[serde(rename = "type")]
    pub card_type: String,
    pub difficulty: String,
    #[serde(default = "default_metadata")]
    pub metadata: Value,
    pub created_at: i64,
    pub updated_at: i64,
    pub due_at: i64,
    pub interval: f64,
    pub ease_factor: f64,
    pub review_count: i64,
}

fn default_metadata() -> Value {
    Value::Object(serde_json::Map::new())
}

/// A card plus the title of its source item — used in review sessions and
/// activity feeds so the UI can show context without an extra lookup.
#[derive(Debug, Clone, Serialize)]
pub struct DueCard {
    #[serde(flatten)]
    pub card: Card,
    pub knowledge_title: String,
}

/// A recorded review outcome. Part of the schema-complete model set; export
/// uses `ExportReview`, so this is retained for future read APIs.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: i64,
    pub card_id: i64,
    pub rating: String,
    pub user_answer: Option<String>,
    pub ai_evaluation: Option<String>,
    pub reviewed_at: i64,
}

/// The scheduling result computed by the frontend scheduler and persisted here.
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleUpdate {
    pub interval: f64,
    pub ease_factor: f64,
    pub due_at: i64,
    pub review_count: i64,
}

/// Payload for creating/updating a knowledge item.
#[derive(Debug, Clone, Deserialize)]
pub struct KnowledgeItemInput {
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Payload for creating/updating a card.
#[derive(Debug, Clone, Deserialize)]
pub struct CardInput {
    pub knowledge_item_id: i64,
    pub question: String,
    #[serde(default)]
    pub answer: String,
    #[serde(rename = "type", default = "default_card_type")]
    pub card_type: String,
    #[serde(default = "default_difficulty")]
    pub difficulty: String,
    #[serde(default = "default_metadata")]
    pub metadata: Value,
}

fn default_card_type() -> String {
    "basic".to_string()
}
fn default_difficulty() -> String {
    "medium".to_string()
}

pub const CARD_TYPES: [&str; 5] = ["basic", "free", "cloze", "mcq", "reverse"];
pub const DIFFICULTIES: [&str; 3] = ["easy", "medium", "hard"];
pub const RATINGS: [&str; 4] = ["again", "hard", "good", "easy"];

/// Aggregate numbers for the dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct DashboardStats {
    pub due_now: i64,
    pub due_today: i64,
    pub new_cards: i64,
    pub reviewed_today: i64,
    pub total_cards: i64,
    pub total_items: i64,
    pub mastered: i64,
    pub streak: i64,
    pub recent_activity: Vec<ActivityEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivityEntry {
    pub reviewed_at: i64,
    pub rating: String,
    pub card_id: i64,
    pub question: String,
    pub knowledge_title: String,
}

// ---- Import / Export ----

pub const EXPORT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportBundle {
    pub version: u32,
    #[serde(default)]
    pub exported_at: i64,
    #[serde(default)]
    pub knowledge_items: Vec<ExportKnowledgeItem>,
    #[serde(default)]
    pub cards: Vec<ExportCard>,
    #[serde(default)]
    pub reviews: Vec<ExportReview>,
}

/// Export/import uses the item's own id as a stable key so cards can reference
/// their parent across an export→import round-trip even when the database
/// re-numbers rows on import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportKnowledgeItem {
    pub id: i64,
    pub title: String,
    #[serde(default)]
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCard {
    pub id: i64,
    pub knowledge_item_id: i64,
    pub question: String,
    #[serde(default)]
    pub answer: String,
    #[serde(rename = "type", default = "default_card_type")]
    pub card_type: String,
    #[serde(default = "default_difficulty")]
    pub difficulty: String,
    #[serde(default = "default_metadata")]
    pub metadata: Value,
    pub created_at: i64,
    pub updated_at: i64,
    pub due_at: i64,
    #[serde(default)]
    pub interval: f64,
    #[serde(default = "default_ease")]
    pub ease_factor: f64,
    #[serde(default)]
    pub review_count: i64,
}

fn default_ease() -> f64 {
    2.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReview {
    pub card_id: i64,
    pub rating: String,
    #[serde(default)]
    pub user_answer: Option<String>,
    #[serde(default)]
    pub ai_evaluation: Option<String>,
    pub reviewed_at: i64,
}

// ---- Knowledge-base chat ----

/// A knowledge item that was used as context for a chat answer (a "source").
#[derive(Debug, Clone, Serialize)]
pub struct KbSource {
    pub id: i64,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct KbChatResponse {
    pub answer: String,
    pub sources: Vec<KbSource>,
}
