//! File IO for import/export, plus pure parsers for Markdown and plain text.
//!
//! The parsing functions take no IO and are unit-tested. Actual reads/writes go
//! through the small helpers here, which cap file size and surface friendly
//! errors.

use std::path::Path;

use crate::db::models::ExportBundle;
use crate::error::{AppError, AppResult};

/// Refuse to read anything larger than this (guards against accidental huge files).
const MAX_IMPORT_BYTES: u64 = 64 * 1024 * 1024;

pub fn read_text_file(path: &Path) -> AppResult<String> {
    let meta = std::fs::metadata(path)
        .map_err(|e| AppError::Io(format!("cannot open {}: {e}", path.display())))?;
    if meta.len() > MAX_IMPORT_BYTES {
        return Err(AppError::Invalid("file is too large to import".into()));
    }
    let bytes = std::fs::read(path)
        .map_err(|e| AppError::Io(format!("cannot read {}: {e}", path.display())))?;
    String::from_utf8(bytes).map_err(|_| AppError::Invalid("file is not valid UTF-8 text".into()))
}

pub fn write_text_file(path: &Path, contents: &str) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).ok();
        }
    }
    std::fs::write(path, contents)
        .map_err(|e| AppError::Io(format!("cannot write {}: {e}", path.display())))
}

// ---------------------------------------------------------------------------
// Parsed representations (for Markdown / plain-text import)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCard {
    pub question: String,
    pub answer: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedItem {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub cards: Vec<ParsedCard>,
}

/// Parse a Markdown document into one or more knowledge items.
///
/// - Each top-level `# Heading` starts a new item (title = heading text).
/// - A `Tags: a, b` line inside a section sets that item's tags.
/// - `Q:` / `A:` line pairs inside a section become basic cards.
/// - Content before the first heading (or a document with no headings) becomes
///   a single item titled from its first line.
pub fn parse_markdown(md: &str) -> Vec<ParsedItem> {
    let mut items: Vec<ParsedItem> = Vec::new();
    let mut current: Option<ParsedItem> = None;
    let mut preamble: Vec<String> = Vec::new();

    for line in md.lines() {
        if let Some(heading) = line.strip_prefix("# ") {
            if let Some(item) = current.take() {
                items.push(finish_item(item));
            }
            current = Some(ParsedItem {
                title: heading.trim().to_string(),
                content: String::new(),
                tags: Vec::new(),
                cards: Vec::new(),
            });
        } else if let Some(item) = current.as_mut() {
            item.content.push_str(line);
            item.content.push('\n');
        } else {
            preamble.push(line.to_string());
        }
    }
    if let Some(item) = current.take() {
        items.push(finish_item(item));
    }

    // Handle content that appeared before any heading.
    let pre = preamble.join("\n");
    if items.is_empty() && !pre.trim().is_empty() {
        items.push(finish_item(ParsedItem {
            title: first_line_title(&pre),
            content: pre,
            tags: Vec::new(),
            cards: Vec::new(),
        }));
    }

    items
}

/// Parse plain text into a single knowledge item.
pub fn parse_plaintext(text: &str) -> Vec<ParsedItem> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    vec![ParsedItem {
        title: first_line_title(text),
        content: text.trim_end().to_string(),
        tags: Vec::new(),
        cards: Vec::new(),
    }]
}

fn finish_item(mut item: ParsedItem) -> ParsedItem {
    // Extract Tags: line and Q:/A: pairs from the content.
    let mut kept: Vec<String> = Vec::new();
    let mut pending_q: Option<String> = None;

    for line in item.content.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();

        if let Some(rest) = lower.strip_prefix("tags:") {
            let raw = &trimmed[trimmed.len() - rest.len()..];
            item.tags = raw
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            continue;
        }

        if let Some(idx) = split_prefix(trimmed, &["q:", "question:"]) {
            // A new question; if one was pending without an answer, drop it.
            pending_q = Some(trimmed[idx..].trim().to_string());
            continue;
        }
        if let Some(idx) = split_prefix(trimmed, &["a:", "answer:"]) {
            if let Some(q) = pending_q.take() {
                let a = trimmed[idx..].trim().to_string();
                if !q.is_empty() {
                    item.cards.push(ParsedCard {
                        question: q,
                        answer: a,
                    });
                }
            }
            continue;
        }

        kept.push(line.to_string());
    }

    item.content = kept.join("\n").trim().to_string();
    if item.title.is_empty() {
        item.title = first_line_title(&item.content);
    }
    item
}

/// If `s` starts (case-insensitively) with one of `prefixes`, return the byte
/// index just after the matched prefix.
fn split_prefix(s: &str, prefixes: &[&str]) -> Option<usize> {
    let lower = s.to_lowercase();
    for p in prefixes {
        if lower.starts_with(p) {
            return Some(p.len());
        }
    }
    None
}

fn first_line_title(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("Untitled");
    let line = line.trim_start_matches('#').trim();
    if line.chars().count() > 80 {
        let truncated: String = line.chars().take(80).collect();
        format!("{truncated}…")
    } else if line.is_empty() {
        "Untitled".to_string()
    } else {
        line.to_string()
    }
}

// ---------------------------------------------------------------------------
// Markdown export (human-readable, not lossless for scheduling data)
// ---------------------------------------------------------------------------

pub fn bundle_to_markdown(bundle: &ExportBundle) -> String {
    let mut out = String::new();
    out.push_str("<!-- Recall export (Markdown). For a lossless backup, export as JSON. -->\n\n");

    for item in &bundle.knowledge_items {
        out.push_str(&format!("# {}\n\n", item.title));
        if !item.tags.is_empty() {
            out.push_str(&format!("Tags: {}\n\n", item.tags.join(", ")));
        }
        if !item.content.trim().is_empty() {
            out.push_str(item.content.trim());
            out.push_str("\n\n");
        }
        let cards: Vec<_> = bundle
            .cards
            .iter()
            .filter(|c| c.knowledge_item_id == item.id)
            .collect();
        if !cards.is_empty() {
            out.push_str("## Cards\n\n");
            for c in cards {
                out.push_str(&format!("Q: {}\n", c.question.replace('\n', " ")));
                out.push_str(&format!("A: {}\n\n", c.answer.replace('\n', " ")));
            }
        }
        out.push_str("---\n\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_splits_on_headings_and_extracts_cards() {
        let md = "# TCP\nTags: networking, protocols\nTCP is connection oriented.\nQ: How many steps in the handshake?\nA: Three\n\n# UDP\nUDP is connectionless.";
        let items = parse_markdown(md);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "TCP");
        assert_eq!(items[0].tags, vec!["networking", "protocols"]);
        assert_eq!(items[0].cards.len(), 1);
        assert_eq!(items[0].cards[0].answer, "Three");
        assert!(items[0].content.contains("connection oriented"));
        assert!(!items[0].content.to_lowercase().contains("tags:"));
        assert_eq!(items[1].title, "UDP");
    }

    #[test]
    fn markdown_without_heading_becomes_one_item() {
        let md = "Just some free notes\nwith two lines.";
        let items = parse_markdown(md);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Just some free notes");
    }

    #[test]
    fn plaintext_makes_single_item() {
        let items = parse_plaintext("Photosynthesis\nconverts light to energy.");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Photosynthesis");
        assert!(items[0].content.contains("light to energy"));
    }
}
