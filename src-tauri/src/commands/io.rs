//! Import / export commands.
//!
//! The native file dialog and all file IO happen in Rust, so the webview is
//! never granted filesystem or dialog permissions. The frontend just calls
//! `export_data` / `import_data` with a format.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::db::models::{CardInput, ExportBundle, KnowledgeItemInput};
use crate::db::{now_ms, repository as repo};
use crate::error::{AppError, AppResult};
use crate::filesystem::{self, ParsedItem};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct IoResult {
    pub path: String,
    pub items: usize,
    pub cards: usize,
    pub reviews: usize,
}

fn to_pathbuf(fp: tauri_plugin_dialog::FilePath) -> AppResult<PathBuf> {
    fp.into_path().map_err(|e| AppError::Io(e.to_string()))
}

fn detect_by_ext(path: &Path) -> String {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .as_deref()
    {
        Some("json") => "json".into(),
        Some("md") | Some("markdown") => "markdown".into(),
        _ => "text".into(),
    }
}

#[tauri::command]
pub async fn export_data(
    app: AppHandle,
    state: State<'_, AppState>,
    format: String,
) -> AppResult<Option<IoResult>> {
    let fmt = format.to_lowercase();

    // Build the export payload first (short DB lock), then show the dialog.
    let bundle = {
        let conn = state.db()?;
        repo::export_bundle(&conn, now_ms())?
    };

    let content = if fmt == "markdown" || fmt == "md" {
        filesystem::bundle_to_markdown(&bundle)
    } else {
        serde_json::to_string_pretty(&bundle)?
    };

    let dialog = app.dialog().file();
    let picked = if fmt == "markdown" || fmt == "md" {
        dialog
            .add_filter("Markdown", &["md"])
            .set_file_name("recall-export.md")
            .blocking_save_file()
    } else {
        dialog
            .add_filter("JSON", &["json"])
            .set_file_name("recall-export.json")
            .blocking_save_file()
    };

    let Some(fp) = picked else {
        return Ok(None); // user cancelled
    };
    let path = to_pathbuf(fp)?;
    filesystem::write_text_file(&path, &content)?;

    Ok(Some(IoResult {
        path: path.display().to_string(),
        items: bundle.knowledge_items.len(),
        cards: bundle.cards.len(),
        reviews: bundle.reviews.len(),
    }))
}

#[tauri::command]
pub async fn import_data(
    app: AppHandle,
    state: State<'_, AppState>,
    format: String,
) -> AppResult<Option<IoResult>> {
    let fmt = format.to_lowercase();

    let dialog = app.dialog().file();
    let picked = match fmt.as_str() {
        "json" => dialog.add_filter("JSON", &["json"]).blocking_pick_file(),
        "markdown" | "md" => dialog
            .add_filter("Markdown", &["md", "markdown"])
            .blocking_pick_file(),
        "text" | "txt" => dialog.add_filter("Text", &["txt", "text"]).blocking_pick_file(),
        _ => dialog
            .add_filter("Supported", &["json", "md", "markdown", "txt", "text"])
            .blocking_pick_file(),
    };

    let Some(fp) = picked else {
        return Ok(None); // user cancelled
    };
    let path = to_pathbuf(fp)?;
    let text = filesystem::read_text_file(&path)?;

    let detected = if fmt.is_empty() || fmt == "auto" {
        detect_by_ext(&path)
    } else {
        fmt
    };

    let now = now_ms();
    let result = match detected.as_str() {
        "json" => {
            let bundle: ExportBundle = serde_json::from_str(&text)
                .map_err(|e| AppError::Invalid(format!("not a valid Recall JSON export: {e}")))?;
            let mut conn = state.db()?;
            let (items, cards, reviews) = repo::import_bundle(&mut conn, &bundle)?;
            IoResult {
                path: path.display().to_string(),
                items,
                cards,
                reviews,
            }
        }
        "markdown" | "md" => {
            let parsed = filesystem::parse_markdown(&text);
            let (items, cards) = import_parsed(&state, parsed, now)?;
            IoResult {
                path: path.display().to_string(),
                items,
                cards,
                reviews: 0,
            }
        }
        _ => {
            let parsed = filesystem::parse_plaintext(&text);
            let (items, cards) = import_parsed(&state, parsed, now)?;
            IoResult {
                path: path.display().to_string(),
                items,
                cards,
                reviews: 0,
            }
        }
    };

    Ok(Some(result))
}

fn import_parsed(
    state: &State<'_, AppState>,
    items: Vec<ParsedItem>,
    now: i64,
) -> AppResult<(usize, usize)> {
    let conn = state.db()?;
    let mut item_count = 0usize;
    let mut card_count = 0usize;
    for pi in items {
        let item = repo::create_item(
            &conn,
            &KnowledgeItemInput {
                title: pi.title,
                content: pi.content,
                tags: pi.tags,
            },
            now,
        )?;
        item_count += 1;
        for pc in pi.cards {
            repo::create_card(
                &conn,
                &CardInput {
                    knowledge_item_id: item.id,
                    question: pc.question,
                    answer: pc.answer,
                    card_type: "basic".into(),
                    difficulty: "medium".into(),
                    metadata: json!({}),
                },
                now,
            )?;
            card_count += 1;
        }
    }
    Ok((item_count, card_count))
}
