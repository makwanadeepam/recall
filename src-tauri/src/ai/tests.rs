//! Tests for the AI subsystem.
//!
//! Pure functions (request building, status mapping, response parsing) are
//! tested directly. The HTTP send path is tested end-to-end against a tiny
//! local TCP server so we cover success, auth failure, rate limiting,
//! malformed responses, timeouts, and network failure without external deps.

use super::*;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::thread;
use std::time::Duration;

fn openai_cfg(base: &str) -> AiConfig {
    AiConfig {
        provider: "custom".into(),
        base_url: base.into(),
        model: "test-model".into(),
        enabled: true,
    }
}

/// Serve a single HTTP response (optionally after a delay) then close.
fn spawn_server(response: String, delay: Option<Duration>) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .ok();
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf); // best-effort drain of request
            if let Some(d) = delay {
                thread::sleep(d);
            }
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
            // stream drops here → EOF for the client (Connection: close)
        }
    });
    addr
}

fn http_response(status_line: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}"
    )
}

// ---- Pure: request building -------------------------------------------------

#[test]
fn openai_request_is_well_formed() {
    let cfg = AiConfig {
        provider: "openai".into(),
        base_url: "".into(),
        model: "gpt-4o-mini".into(),
        enabled: true,
    };
    let spec = openai::build_request(&cfg, "secret-key", "sys", "hello").unwrap();
    assert_eq!(spec.url, "https://api.openai.com/v1/chat/completions");
    assert!(spec
        .headers
        .iter()
        .any(|(k, v)| k == "authorization" && v == "Bearer secret-key"));
    let body: serde_json::Value = serde_json::from_str(&spec.body).unwrap();
    assert_eq!(body["model"], "gpt-4o-mini");
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["messages"][1]["content"], "hello");
}

#[test]
fn ollama_uses_local_default_base_url() {
    let cfg = AiConfig {
        provider: "ollama".into(),
        base_url: "".into(),
        model: "llama3.2".into(),
        enabled: true,
    };
    let spec = openai::build_request(&cfg, "ollama", "s", "u").unwrap();
    assert_eq!(spec.url, "http://localhost:11434/v1/chat/completions");
}

#[test]
fn anthropic_request_uses_x_api_key_header() {
    let cfg = AiConfig {
        provider: "anthropic".into(),
        base_url: "".into(),
        model: "claude-x".into(),
        enabled: true,
    };
    let spec = anthropic::build_request(&cfg, "abc", "sys", "user").unwrap();
    assert_eq!(spec.url, "https://api.anthropic.com/v1/messages");
    assert!(spec.headers.iter().any(|(k, v)| k == "x-api-key" && v == "abc"));
    assert!(spec
        .headers
        .iter()
        .any(|(k, _)| k == "anthropic-version"));
}

#[test]
fn gemini_keeps_key_in_header_not_url() {
    let cfg = AiConfig {
        provider: "gemini".into(),
        base_url: "".into(),
        model: "gemini-2.0-flash".into(),
        enabled: true,
    };
    let spec = gemini::build_request(&cfg, "topsecret", "sys", "user").unwrap();
    assert!(spec.url.contains("gemini-2.0-flash:generateContent"));
    assert!(!spec.url.contains("topsecret"), "key must not be in the URL");
    assert!(spec
        .headers
        .iter()
        .any(|(k, v)| k == "x-goog-api-key" && v == "topsecret"));
}

#[test]
fn custom_provider_requires_base_url() {
    let cfg = openai_cfg("");
    let err = openai::build_request(&cfg, "k", "s", "u").unwrap_err();
    assert_eq!(err.kind(), "ai_config");
}

// ---- Pure: status mapping ---------------------------------------------------

#[test]
fn status_mapping_covers_auth_ratelimit_provider() {
    assert!(map_status_error(200, "{}").is_none());
    assert!(matches!(map_status_error(401, "{}"), Some(AiError::Auth)));
    assert!(matches!(map_status_error(403, "{}"), Some(AiError::Auth)));
    assert!(matches!(map_status_error(429, "{}"), Some(AiError::RateLimit)));
    match map_status_error(500, r#"{"error":{"message":"boom"}}"#) {
        Some(AiError::Provider { status, message }) => {
            assert_eq!(status, 500);
            assert!(message.contains("boom"));
        }
        other => panic!("expected Provider error, got {other:?}"),
    }
}

// ---- Pure: content extraction ----------------------------------------------

#[test]
fn extract_openai_content() {
    let body = r#"{"choices":[{"message":{"content":"the answer"}}]}"#;
    assert_eq!(openai::extract_content(body).unwrap(), "the answer");
}

#[test]
fn extract_openai_content_malformed() {
    assert!(openai::extract_content("not json").is_err());
    assert!(openai::extract_content(r#"{"choices":[]}"#).is_err());
}

#[test]
fn extract_anthropic_and_gemini_content() {
    let a = r#"{"content":[{"type":"text","text":"hi "},{"type":"text","text":"there"}]}"#;
    assert_eq!(anthropic::extract_content(a).unwrap(), "hi there");
    let g = r#"{"candidates":[{"content":{"parts":[{"text":"g-text"}]}}]}"#;
    assert_eq!(gemini::extract_content(g).unwrap(), "g-text");
}

// ---- Pure: parsing model output --------------------------------------------

#[test]
fn parse_questions_from_bare_array() {
    let text = r#"[
      {"question":"Q1","answer":"A1","type":"basic","difficulty":"easy"},
      {"question":"Q2","answer":"A2","type":"mcq","difficulty":"medium","distractors":["x","y","z"]}
    ]"#;
    let qs = parse_questions(text).unwrap();
    assert_eq!(qs.len(), 2);
    assert_eq!(qs[1].card_type, "mcq");
    assert_eq!(qs[1].distractors.len(), 3);
}

#[test]
fn parse_questions_tolerates_markdown_fence_and_wrapper_object() {
    let text = "```json\n{\"questions\":[{\"question\":\"Q\",\"answer\":\"A\"}]}\n```";
    let qs = parse_questions(text).unwrap();
    assert_eq!(qs.len(), 1);
    assert_eq!(qs[0].card_type, "basic"); // default
}

#[test]
fn parse_questions_downgrades_mcq_without_distractors() {
    let text = r#"[{"question":"Q","answer":"A","type":"mcq"}]"#;
    let qs = parse_questions(text).unwrap();
    assert_eq!(qs[0].card_type, "basic");
}

#[test]
fn parse_questions_rejects_malformed() {
    assert!(parse_questions("totally not json").is_err());
    assert!(parse_questions("[]").is_err()); // no usable questions
}

#[test]
fn parse_evaluation_normalizes_fields() {
    let text = r#"{"correctness":"Partially Correct","score":1.7,"missing":"x","feedback":"ok"}"#;
    let eval = parse_evaluation(text).unwrap();
    assert_eq!(eval.correctness, "partial");
    assert_eq!(eval.score, 1.0); // clamped
}

#[test]
fn parse_evaluation_rejects_malformed() {
    assert!(parse_evaluation("nope").is_err());
}

#[test]
fn extract_keywords_drops_stopwords_and_short_words() {
    let kws = extract_keywords("What is the TCP handshake and why do we use it?");
    assert!(kws.contains(&"tcp".to_string()));
    assert!(kws.contains(&"handshake".to_string()));
    // stopwords and short words are excluded
    assert!(!kws.contains(&"the".to_string()));
    assert!(!kws.contains(&"why".to_string()));
    assert!(!kws.contains(&"is".to_string()));
}

#[test]
fn extract_keywords_dedupes_and_caps() {
    let kws = extract_keywords("alpha alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu");
    // distinct only
    assert_eq!(kws.iter().filter(|w| *w == "alpha").count(), 1);
    // capped at 12
    assert!(kws.len() <= 12);
}

// ---- End-to-end HTTP behavior ----------------------------------------------

#[tokio::test]
async fn chat_success() {
    let body = r#"{"choices":[{"message":{"content":"hello world"}}]}"#;
    let addr = spawn_server(http_response("200 OK", body), None);
    let client = reqwest::Client::new();
    let cfg = openai_cfg(&format!("http://{addr}"));
    let out = chat(&client, &cfg, "k", "s", "u", Duration::from_secs(5))
        .await
        .unwrap();
    assert_eq!(out, "hello world");
}

#[tokio::test]
async fn chat_authentication_failure() {
    let addr = spawn_server(http_response("401 Unauthorized", r#"{"error":{"message":"bad key"}}"#), None);
    let client = reqwest::Client::new();
    let cfg = openai_cfg(&format!("http://{addr}"));
    let err = chat(&client, &cfg, "k", "s", "u", Duration::from_secs(5))
        .await
        .unwrap_err();
    assert!(matches!(err, AiError::Auth));
}

#[tokio::test]
async fn chat_rate_limit() {
    let addr = spawn_server(http_response("429 Too Many Requests", "{}"), None);
    let client = reqwest::Client::new();
    let cfg = openai_cfg(&format!("http://{addr}"));
    let err = chat(&client, &cfg, "k", "s", "u", Duration::from_secs(5))
        .await
        .unwrap_err();
    assert!(matches!(err, AiError::RateLimit));
}

#[tokio::test]
async fn chat_malformed_response() {
    let addr = spawn_server(http_response("200 OK", "this is not json"), None);
    let client = reqwest::Client::new();
    let cfg = openai_cfg(&format!("http://{addr}"));
    let err = chat(&client, &cfg, "k", "s", "u", Duration::from_secs(5))
        .await
        .unwrap_err();
    assert!(matches!(err, AiError::BadResponse(_)));
}

#[tokio::test]
async fn chat_timeout() {
    // Server delays well past the client timeout.
    let addr = spawn_server(
        http_response("200 OK", r#"{"choices":[{"message":{"content":"late"}}]}"#),
        Some(Duration::from_secs(2)),
    );
    let client = reqwest::Client::new();
    let cfg = openai_cfg(&format!("http://{addr}"));
    let err = chat(&client, &cfg, "k", "s", "u", Duration::from_millis(250))
        .await
        .unwrap_err();
    assert!(matches!(err, AiError::Timeout), "got {err:?}");
}

#[tokio::test]
async fn chat_network_failure() {
    // Bind then immediately release the port so connecting is refused.
    let addr = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap()
    };
    let client = reqwest::Client::new();
    let cfg = openai_cfg(&format!("http://{addr}"));
    let err = chat(&client, &cfg, "k", "s", "u", Duration::from_secs(3))
        .await
        .unwrap_err();
    assert!(matches!(err, AiError::Network(_)), "got {err:?}");
}
