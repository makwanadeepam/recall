//! AI operation commands. All are async so they run off the main thread; the DB
//! lock is always released before any network await.

use std::time::Duration;

use serde::Deserialize;
use tauri::State;

use crate::ai::{self, prompts, AiConfig, AiError, AnswerEvaluation, GeneratedQuestion};
use crate::commands::settings::load_ai_config;
use crate::db::models::{KbChatResponse, KbSource};
use crate::db::repository as repo;
use crate::error::{AppError, AppResult};
use crate::security::keychain;
use crate::state::AppState;

/// Load config + key, releasing the DB lock before returning. When
/// `require_enabled` is false (used by the connection test) the enabled flag is
/// not checked, so a user can verify a provider before switching AI on.
fn resolve(state: &AppState, require_enabled: bool) -> AppResult<(AiConfig, String)> {
    let config = {
        let conn = state.db()?;
        load_ai_config(&conn)?
    };
    if config.provider.trim().is_empty() {
        return Err(AiError::NotConfigured.into());
    }
    if require_enabled && !config.enabled {
        return Err(AiError::Disabled.into());
    }
    if config.model.trim().is_empty() {
        return Err(AiError::Config("a model name is required".into()).into());
    }
    let key = resolve_key(&config.provider)?;
    Ok((config, key))
}

fn resolve_key(provider: &str) -> AppResult<String> {
    match keychain::get_key(provider)? {
        Some(k) => Ok(k),
        // Local/custom endpoints often need no real key.
        None if provider.eq_ignore_ascii_case("ollama") => Ok("ollama".into()),
        None if provider.eq_ignore_ascii_case("custom") => Ok(String::new()),
        None => Err(AiError::MissingKey(provider.to_string()).into()),
    }
}

#[tauri::command]
pub async fn ai_test_connection(state: State<'_, AppState>) -> AppResult<String> {
    let (config, key) = resolve(state.inner(), false)?;
    let client = state.http_client();
    Ok(ai::test_connection(client, &config, &key).await?)
}

#[tauri::command]
pub async fn ai_generate_questions(
    state: State<'_, AppState>,
    content: String,
    count: Option<u32>,
) -> AppResult<Vec<GeneratedQuestion>> {
    if content.trim().is_empty() {
        return Err(AppError::Invalid("there is no content to generate questions from".into()));
    }
    let (config, key) = resolve(state.inner(), true)?;
    let n = count.unwrap_or(8).clamp(1, 30);
    let client = state.http_client();
    Ok(ai::generate_questions(client, &config, &key, content.trim(), n).await?)
}

#[derive(Debug, Deserialize)]
pub struct EvalInput {
    pub question: String,
    pub expected_answer: String,
    pub user_answer: String,
}

#[tauri::command]
pub async fn ai_evaluate_answer(
    state: State<'_, AppState>,
    input: EvalInput,
) -> AppResult<AnswerEvaluation> {
    let (config, key) = resolve(state.inner(), true)?;
    let client = state.http_client();
    Ok(ai::evaluate_answer(
        client,
        &config,
        &key,
        &input.question,
        &input.expected_answer,
        &input.user_answer,
    )
    .await?)
}

#[tauri::command]
pub async fn ai_generate_explanation(
    state: State<'_, AppState>,
    question: String,
    answer: String,
) -> AppResult<String> {
    let (config, key) = resolve(state.inner(), true)?;
    let client = state.http_client();
    Ok(ai::generate_explanation(client, &config, &key, &question, &answer).await?)
}

#[derive(Debug, Deserialize)]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
}

/// Chat grounded in the user's knowledge base: retrieve relevant notes locally,
/// then ask the configured provider to answer using only those notes.
#[tauri::command]
pub async fn ai_kb_chat(
    state: State<'_, AppState>,
    question: String,
    history: Vec<ChatTurn>,
) -> AppResult<KbChatResponse> {
    let question = question.trim().to_string();
    if question.is_empty() {
        return Err(AppError::Invalid("ask a question first".into()));
    }
    let (config, key) = resolve(state.inner(), true)?;

    // Retrieve relevant notes (short DB lock, released before the network call).
    let keywords = ai::extract_keywords(&question);
    let context_items = {
        let conn = state.db()?;
        repo::search_context(&conn, &keywords, 5)?
    };

    let mut context = String::new();
    for item in &context_items {
        context.push_str(&format!("## {}\n{}\n\n", item.title, truncate_chars(&item.content, 1500)));
    }

    // Keep only the last few turns to bound the prompt size.
    let hist: Vec<(String, String)> = history
        .iter()
        .rev()
        .take(6)
        .rev()
        .map(|t| (t.role.clone(), t.content.clone()))
        .collect();

    let (system, user) = prompts::kb_chat(&context, &hist, &question);
    let client = state.http_client();
    let answer = ai::chat(client, &config, &key, &system, &user, Duration::from_secs(90)).await?;

    let sources = context_items
        .into_iter()
        .map(|it| KbSource {
            id: it.id,
            title: it.title,
        })
        .collect();

    Ok(KbChatResponse {
        answer: answer.trim().to_string(),
        sources,
    })
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}
