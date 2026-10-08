//! Google Gemini (Generative Language API).
//!
//! The API key is sent in the `x-goog-api-key` header rather than the URL query
//! string, so the key never appears in a URL.

use serde_json::{json, Value};

use super::{effective_base_url, AiConfig, AiError, HttpRequestSpec};

pub fn build_request(
    config: &AiConfig,
    key: &str,
    system: &str,
    user: &str,
) -> Result<HttpRequestSpec, AiError> {
    let model = config.model.trim();
    if model.is_empty() {
        return Err(AiError::Config("a model name is required".into()));
    }
    let base = effective_base_url(config)?;
    let url = format!("{base}/v1beta/models/{model}:generateContent");

    let body = json!({
        "systemInstruction": {
            "parts": [ { "text": system } ]
        },
        "contents": [
            { "role": "user", "parts": [ { "text": user } ] }
        ],
        "generationConfig": {
            "temperature": 0.4
        }
    });

    let headers = vec![("x-goog-api-key".to_string(), key.to_string())];

    Ok(HttpRequestSpec {
        url,
        headers,
        body: body.to_string(),
    })
}

pub fn extract_content(body: &str) -> Result<String, AiError> {
    let v: Value = serde_json::from_str(body)
        .map_err(|e| AiError::BadResponse(format!("invalid JSON from provider: {e}")))?;
    let parts = v
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(Value::as_array)
        .ok_or_else(|| AiError::BadResponse("no candidate content in response".into()))?;
    let mut text = String::new();
    for part in parts {
        if let Some(t) = part.get("text").and_then(Value::as_str) {
            text.push_str(t);
        }
    }
    if text.is_empty() {
        return Err(AiError::BadResponse("empty text content in response".into()));
    }
    Ok(text)
}
