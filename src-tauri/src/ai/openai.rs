//! OpenAI-compatible chat completions. Covers OpenAI, OpenRouter, Ollama, and
//! any generic `/v1/chat/completions` endpoint. This is the most important
//! provider family because it enables fully-local AI via Ollama.

use serde_json::{json, Value};

use super::{effective_base_url, AiConfig, AiError, HttpRequestSpec};

pub fn build_request(
    config: &AiConfig,
    key: &str,
    system: &str,
    user: &str,
) -> Result<HttpRequestSpec, AiError> {
    if config.model.trim().is_empty() {
        return Err(AiError::Config("a model name is required".into()));
    }
    let base = effective_base_url(config)?;
    let url = format!("{base}/chat/completions");

    let body = json!({
        "model": config.model.trim(),
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ],
        "temperature": 0.4,
        "stream": false
    });

    let mut headers = vec![("authorization".to_string(), format!("Bearer {key}"))];
    // OpenRouter appreciates these attribution headers; harmless elsewhere.
    if config.provider.eq_ignore_ascii_case("openrouter") {
        headers.push(("HTTP-Referer".into(), "https://recall.local".into()));
        headers.push(("X-Title".into(), "Recall".into()));
    }

    Ok(HttpRequestSpec {
        url,
        headers,
        body: body.to_string(),
    })
}

pub fn extract_content(body: &str) -> Result<String, AiError> {
    let v: Value = serde_json::from_str(body)
        .map_err(|e| AiError::BadResponse(format!("invalid JSON from provider: {e}")))?;
    let content = v
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(Value::as_str)
        .ok_or_else(|| AiError::BadResponse("no message content in response".into()))?;
    Ok(content.to_string())
}
