//! Anthropic Messages API.

use serde_json::{json, Value};

use super::{effective_base_url, AiConfig, AiError, HttpRequestSpec};

const ANTHROPIC_VERSION: &str = "2023-06-01";

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
    let url = format!("{base}/v1/messages");

    let body = json!({
        "model": config.model.trim(),
        "max_tokens": 2048,
        "system": system,
        "messages": [
            { "role": "user", "content": user }
        ]
    });

    let headers = vec![
        ("x-api-key".to_string(), key.to_string()),
        ("anthropic-version".to_string(), ANTHROPIC_VERSION.to_string()),
    ];

    Ok(HttpRequestSpec {
        url,
        headers,
        body: body.to_string(),
    })
}

pub fn extract_content(body: &str) -> Result<String, AiError> {
    let v: Value = serde_json::from_str(body)
        .map_err(|e| AiError::BadResponse(format!("invalid JSON from provider: {e}")))?;
    // content is an array of blocks; concatenate the text blocks.
    let blocks = v
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| AiError::BadResponse("no content array in response".into()))?;
    let mut text = String::new();
    for block in blocks {
        if block.get("type").and_then(Value::as_str) == Some("text") {
            if let Some(t) = block.get("text").and_then(Value::as_str) {
                text.push_str(t);
            }
        }
    }
    if text.is_empty() {
        return Err(AiError::BadResponse("empty text content in response".into()));
    }
    Ok(text)
}
