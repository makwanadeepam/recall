//! Optional AI subsystem.
//!
//! Design notes:
//! - The app is fully usable without any of this. AI only *enhances* recall.
//! - API keys live in the OS keychain and are used only here, in Rust. They are
//!   never sent anywhere except the configured provider's endpoint.
//! - Providers are split into three request families. Request building and
//!   response parsing are pure functions (see `openai`/`anthropic`/`gemini`),
//!   so they can be unit-tested without a network.

pub mod anthropic;
pub mod gemini;
pub mod openai;
pub mod prompts;

use std::collections::HashSet;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("AI is not configured. Choose a provider in Settings.")]
    NotConfigured,

    #[error("AI features are turned off. Enable AI in Settings.")]
    Disabled,

    #[error("No API key is set for the '{0}' provider.")]
    MissingKey(String),

    #[error("The AI provider rejected the request: authentication failed.")]
    Auth,

    #[error("The AI provider is rate limiting requests.")]
    RateLimit,

    #[error("The AI request timed out.")]
    Timeout,

    #[error("Could not reach the AI provider: {0}")]
    Network(String),

    #[error("The AI provider returned an error (HTTP {status}).")]
    Provider { status: u16, message: String },

    #[error("The AI response could not be understood.")]
    BadResponse(String),

    #[error("Invalid AI configuration: {0}")]
    Config(String),
}

impl AiError {
    pub fn kind(&self) -> &'static str {
        match self {
            AiError::NotConfigured => "ai_not_configured",
            AiError::Disabled => "ai_disabled",
            AiError::MissingKey(_) => "ai_missing_key",
            AiError::Auth => "ai_auth",
            AiError::RateLimit => "ai_rate_limit",
            AiError::Timeout => "ai_timeout",
            AiError::Network(_) => "ai_network",
            AiError::Provider { .. } => "ai_provider",
            AiError::BadResponse(_) => "ai_bad_response",
            AiError::Config(_) => "ai_config",
        }
    }

    pub fn hint(&self) -> Option<String> {
        let h = match self {
            AiError::NotConfigured => "Open Settings and configure an AI provider to enable this.",
            AiError::Disabled => "Turn on AI in Settings.",
            AiError::MissingKey(_) => "Add an API key for this provider in Settings.",
            AiError::Auth => "Check the API key, and that it can access the selected model.",
            AiError::RateLimit => "Wait a moment and try again, or check your provider quota.",
            AiError::Timeout => "Check your connection, or try a shorter passage.",
            AiError::Network(_) => {
                "Check your connection, and the Base URL for local/custom providers (e.g. Ollama)."
            }
            AiError::Provider { .. } => "Check the model name and provider URL.",
            AiError::BadResponse(_) => {
                "The model may not follow structured output. Try another model."
            }
            AiError::Config(_) => "Check the provider, Base URL, and model in Settings.",
        };
        Some(h.to_string())
    }
}

// ---------------------------------------------------------------------------
// Config & data types
// ---------------------------------------------------------------------------

pub const PROVIDERS: [&str; 6] = ["openai", "anthropic", "gemini", "openrouter", "ollama", "custom"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    /// May be empty — a per-provider default is used when so.
    #[serde(default)]
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    OpenAiCompatible,
    Anthropic,
    Gemini,
}

pub fn family_for(provider: &str) -> Result<Family, AiError> {
    match provider.trim().to_lowercase().as_str() {
        "openai" | "openrouter" | "ollama" | "custom" => Ok(Family::OpenAiCompatible),
        "anthropic" => Ok(Family::Anthropic),
        "gemini" | "google" => Ok(Family::Gemini),
        other => Err(AiError::Config(format!("unknown provider '{other}'"))),
    }
}

pub fn default_base_url(provider: &str) -> &'static str {
    match provider.trim().to_lowercase().as_str() {
        "openai" => "https://api.openai.com/v1",
        "openrouter" => "https://openrouter.ai/api/v1",
        "ollama" => "http://localhost:11434/v1",
        "anthropic" => "https://api.anthropic.com",
        "gemini" | "google" => "https://generativelanguage.googleapis.com",
        _ => "",
    }
}

/// Resolve the effective base URL: the configured one, or the provider default.
pub fn effective_base_url(config: &AiConfig) -> Result<String, AiError> {
    let trimmed = config.base_url.trim().trim_end_matches('/');
    if !trimmed.is_empty() {
        return Ok(trimmed.to_string());
    }
    let def = default_base_url(&config.provider);
    if def.is_empty() {
        return Err(AiError::Config(
            "a Base URL is required for a custom provider".into(),
        ));
    }
    Ok(def.to_string())
}

/// A fully-built HTTP request (always POST, JSON body).
#[derive(Debug, Clone)]
pub struct HttpRequestSpec {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// A question produced by AI generation, returned to the UI for review/editing
/// before anything is saved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedQuestion {
    pub question: String,
    #[serde(default)]
    pub answer: String,
    #[serde(rename = "type", default = "gen_default_type")]
    pub card_type: String,
    #[serde(default = "gen_default_difficulty")]
    pub difficulty: String,
    #[serde(default)]
    pub distractors: Vec<String>,
}

fn gen_default_type() -> String {
    "basic".into()
}
fn gen_default_difficulty() -> String {
    "medium".into()
}

/// Result of AI answer evaluation. Advisory only — the UI always shows the
/// expected answer and lets the user make the final call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerEvaluation {
    /// "correct" | "partial" | "incorrect"
    pub correctness: String,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub missing: String,
    #[serde(default)]
    pub feedback: String,
}

// ---------------------------------------------------------------------------
// HTTP send + status mapping
// ---------------------------------------------------------------------------

fn map_reqwest_err(e: reqwest::Error) -> AiError {
    if e.is_timeout() {
        AiError::Timeout
    } else if e.is_connect() {
        AiError::Network(format!("connection failed: {e}"))
    } else {
        AiError::Network(e.to_string())
    }
}

/// Convert a non-success HTTP status into a specific error. Returns `None` for
/// 2xx. Pure so it can be unit-tested for auth/rate-limit/provider handling.
pub fn map_status_error(status: u16, body: &str) -> Option<AiError> {
    if (200..300).contains(&status) {
        return None;
    }
    let snippet = summarize_error_body(body);
    match status {
        401 | 403 => Some(AiError::Auth),
        429 => Some(AiError::RateLimit),
        _ => Some(AiError::Provider {
            status,
            message: snippet,
        }),
    }
}

/// Pull a short human message out of a provider error body if possible.
fn summarize_error_body(body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(body) {
        if let Some(msg) = v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(Value::as_str)
        {
            return msg.chars().take(300).collect();
        }
        if let Some(msg) = v.get("message").and_then(Value::as_str) {
            return msg.chars().take(300).collect();
        }
    }
    body.chars().take(200).collect()
}

async fn send(
    client: &reqwest::Client,
    spec: HttpRequestSpec,
    timeout: Duration,
) -> Result<(u16, String), AiError> {
    let mut req = client
        .post(&spec.url)
        .timeout(timeout)
        .header("content-type", "application/json");
    for (k, v) in &spec.headers {
        req = req.header(k, v);
    }
    let resp = req.body(spec.body).send().await.map_err(map_reqwest_err)?;
    let status = resp.status().as_u16();
    let text = resp.text().await.map_err(map_reqwest_err)?;
    Ok((status, text))
}

fn build_request(
    config: &AiConfig,
    key: &str,
    system: &str,
    user: &str,
) -> Result<HttpRequestSpec, AiError> {
    match family_for(&config.provider)? {
        Family::OpenAiCompatible => openai::build_request(config, key, system, user),
        Family::Anthropic => anthropic::build_request(config, key, system, user),
        Family::Gemini => gemini::build_request(config, key, system, user),
    }
}

fn extract_content(family: Family, body: &str) -> Result<String, AiError> {
    match family {
        Family::OpenAiCompatible => openai::extract_content(body),
        Family::Anthropic => anthropic::extract_content(body),
        Family::Gemini => gemini::extract_content(body),
    }
}

/// One chat round-trip: build → send → map status → extract text.
pub async fn chat(
    client: &reqwest::Client,
    config: &AiConfig,
    key: &str,
    system: &str,
    user: &str,
    timeout: Duration,
) -> Result<String, AiError> {
    let family = family_for(&config.provider)?;
    let spec = build_request(config, key, system, user)?;
    let (status, body) = send(client, spec, timeout).await?;
    if let Some(err) = map_status_error(status, &body) {
        return Err(err);
    }
    extract_content(family, &body)
}

// ---------------------------------------------------------------------------
// High-level operations
// ---------------------------------------------------------------------------

/// A short connection test. Returns the raw model reply on success.
pub async fn test_connection(
    client: &reqwest::Client,
    config: &AiConfig,
    key: &str,
) -> Result<String, AiError> {
    chat(
        client,
        config,
        key,
        "You are a connectivity check.",
        "Reply with the single word: ok",
        Duration::from_secs(30),
    )
    .await
}

pub async fn generate_questions(
    client: &reqwest::Client,
    config: &AiConfig,
    key: &str,
    content: &str,
    count: u32,
) -> Result<Vec<GeneratedQuestion>, AiError> {
    let (system, user) = prompts::question_generation(content, count);
    let text = chat(client, config, key, &system, &user, Duration::from_secs(90)).await?;
    parse_questions(&text)
}

pub async fn evaluate_answer(
    client: &reqwest::Client,
    config: &AiConfig,
    key: &str,
    question: &str,
    expected: &str,
    user_answer: &str,
) -> Result<AnswerEvaluation, AiError> {
    let (system, user) = prompts::answer_evaluation(question, expected, user_answer);
    let text = chat(client, config, key, &system, &user, Duration::from_secs(60)).await?;
    parse_evaluation(&text)
}

pub async fn generate_explanation(
    client: &reqwest::Client,
    config: &AiConfig,
    key: &str,
    question: &str,
    answer: &str,
) -> Result<String, AiError> {
    let (system, user) = prompts::explanation(question, answer);
    let text = chat(client, config, key, &system, &user, Duration::from_secs(60)).await?;
    Ok(text.trim().to_string())
}

// ---------------------------------------------------------------------------
// Keyword extraction (for knowledge-base retrieval)
// ---------------------------------------------------------------------------

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "your", "with", "what", "how", "why", "when",
    "where", "which", "who", "whom", "that", "this", "these", "those", "from", "into", "about",
    "does", "did", "can", "could", "would", "should", "will", "have", "has", "had", "was", "were",
    "its", "our", "out", "get", "got", "all", "any", "may", "might", "than", "then", "them", "they",
    "there", "here", "over", "under", "between", "explain", "tell", "give", "please",
];

/// Extract up to 12 distinct lowercase keywords (length >= 3, no stopwords) from
/// free text. Used to retrieve relevant notes for a chat question.
pub fn extract_keywords(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for raw in text.split(|c: char| !c.is_alphanumeric()) {
        if raw.len() < 3 {
            continue;
        }
        let w = raw.to_lowercase();
        if w.len() < 3 || STOPWORDS.contains(&w.as_str()) {
            continue;
        }
        if seen.insert(w.clone()) {
            out.push(w);
            if out.len() >= 12 {
                break;
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tolerant parsing of model output
// ---------------------------------------------------------------------------

/// Strip Markdown code fences and return the substring most likely to be JSON.
pub fn extract_json(text: &str) -> &str {
    let mut t = text.trim();
    // Remove a leading ```json / ``` fence and trailing ```.
    if let Some(rest) = t.strip_prefix("```") {
        // drop an optional language tag on the first line
        let rest = rest.splitn(2, '\n').nth(1).unwrap_or(rest);
        t = rest.trim_end_matches("```").trim();
    }
    // Narrow to the outermost array/object if there's surrounding prose.
    let start = t.find(['[', '{']);
    let end = t.rfind([']', '}']);
    if let (Some(s), Some(e)) = (start, end) {
        if e >= s {
            return &t[s..=e];
        }
    }
    t
}

pub fn parse_questions(text: &str) -> Result<Vec<GeneratedQuestion>, AiError> {
    let json = extract_json(text);

    // Accept either a bare array or an object wrapping a "questions" array.
    let parsed: Vec<GeneratedQuestion> = match serde_json::from_str::<Vec<GeneratedQuestion>>(json) {
        Ok(v) => v,
        Err(_) => {
            let obj: Value = serde_json::from_str(json)
                .map_err(|e| AiError::BadResponse(format!("not valid JSON: {e}")))?;
            let arr = obj
                .get("questions")
                .cloned()
                .ok_or_else(|| AiError::BadResponse("no questions array in response".into()))?;
            serde_json::from_value(arr)
                .map_err(|e| AiError::BadResponse(format!("malformed question objects: {e}")))?
        }
    };

    let cleaned: Vec<GeneratedQuestion> = parsed
        .into_iter()
        .filter_map(sanitize_generated)
        .collect();

    if cleaned.is_empty() {
        return Err(AiError::BadResponse(
            "the model returned no usable questions".into(),
        ));
    }
    Ok(cleaned)
}

fn sanitize_generated(mut q: GeneratedQuestion) -> Option<GeneratedQuestion> {
    q.question = q.question.trim().to_string();
    q.answer = q.answer.trim().to_string();
    if q.question.is_empty() {
        return None;
    }
    // Clamp to the card types generation is allowed to produce.
    let t = q.card_type.trim().to_lowercase();
    q.card_type = match t.as_str() {
        "basic" | "cloze" | "mcq" => t,
        "multiple_choice" | "multiple-choice" | "multiplechoice" => "mcq".into(),
        _ => "basic".into(),
    };
    let d = q.difficulty.trim().to_lowercase();
    q.difficulty = match d.as_str() {
        "easy" | "medium" | "hard" => d,
        _ => "medium".into(),
    };
    q.distractors = q
        .distractors
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    // A well-formed MCQ needs distractors; if none, fall back to basic.
    if q.card_type == "mcq" && q.distractors.is_empty() {
        q.card_type = "basic".into();
    }
    Some(q)
}

pub fn parse_evaluation(text: &str) -> Result<AnswerEvaluation, AiError> {
    let json = extract_json(text);
    let mut eval: AnswerEvaluation = serde_json::from_str(json)
        .map_err(|e| AiError::BadResponse(format!("could not parse evaluation: {e}")))?;
    let c = eval.correctness.trim().to_lowercase();
    eval.correctness = match c.as_str() {
        "correct" | "right" | "good" => "correct".into(),
        "partial" | "partially_correct" | "partially correct" => "partial".into(),
        "incorrect" | "wrong" | "bad" => "incorrect".into(),
        _ => c,
    };
    eval.score = eval.score.clamp(0.0, 1.0);
    Ok(eval)
}

#[cfg(test)]
mod tests;
