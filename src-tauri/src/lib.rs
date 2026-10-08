//! Recall — library entry point.
//!
//! Wires up the SQLite-backed state, the dialog plugin (used only from Rust for
//! import/export), and every IPC command, then runs the Tauri app.

mod ai;
mod commands;
mod db;
mod error;
mod filesystem;
mod security;
mod state;

use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Store the database in the OS-appropriate per-app data directory.
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("could not resolve app data dir: {e}"))?;
            let db_path = data_dir.join("recall.db");
            let conn = db::open(&db_path).map_err(|e| format!("database init failed: {e}"))?;
            app.manage(AppState::new(conn));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // knowledge
            commands::knowledge::create_item,
            commands::knowledge::update_item,
            commands::knowledge::get_item,
            commands::knowledge::delete_item,
            commands::knowledge::list_items,
            // cards
            commands::cards::create_card,
            commands::cards::update_card,
            commands::cards::get_card,
            commands::cards::delete_card,
            commands::cards::list_cards,
            // reviews / scheduling
            commands::reviews::due_cards,
            commands::reviews::record_review,
            // stats
            commands::stats::dashboard_stats,
            commands::stats::list_tags,
            // settings
            commands::settings::get_settings,
            commands::settings::set_theme,
            commands::settings::get_ai_config,
            commands::settings::set_ai_config,
            commands::settings::ai_set_key,
            commands::settings::ai_delete_key,
            commands::settings::ai_has_key,
            // ai
            commands::ai::ai_test_connection,
            commands::ai::ai_generate_questions,
            commands::ai::ai_evaluate_answer,
            commands::ai::ai_generate_explanation,
            commands::ai::ai_kb_chat,
            // import / export
            commands::io::export_data,
            commands::io::import_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Recall");
}
