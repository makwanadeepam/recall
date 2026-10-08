//! All SQL lives here. Every query is parameterized (no string interpolation of
//! user input) to prevent SQL injection. Functions take a `&Connection` or
//! `&mut Connection` and never touch Tauri types, which keeps them unit-testable.

use std::collections::{HashMap, HashSet};

use rusqlite::{params, params_from_iter, types::Value as SqlValue, Connection};
use serde_json::Value;

use super::models::*;
use crate::error::{AppError, AppResult};

const CARD_COLS: &str = "id, knowledge_item_id, question, answer, type, difficulty, metadata, \
                         created_at, updated_at, due_at, interval, ease_factor, review_count";

// ---------------------------------------------------------------------------
// Connection setup
// ---------------------------------------------------------------------------

/// Apply the pragmas we rely on. Called once per connection.
pub fn configure(conn: &Connection) -> AppResult<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_card_type(t: &str) -> AppResult<()> {
    if CARD_TYPES.contains(&t) {
        Ok(())
    } else {
        Err(AppError::Invalid(format!("unknown card type '{t}'")))
    }
}

fn validate_difficulty(d: &str) -> AppResult<()> {
    if DIFFICULTIES.contains(&d) {
        Ok(())
    } else {
        Err(AppError::Invalid(format!("unknown difficulty '{d}'")))
    }
}

fn validate_rating(r: &str) -> AppResult<()> {
    if RATINGS.contains(&r) {
        Ok(())
    } else {
        Err(AppError::Invalid(format!("unknown rating '{r}'")))
    }
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for t in tags {
        let trimmed = t.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_lowercase();
        if seen.insert(key) {
            out.push(trimmed.to_string());
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Row mapping
// ---------------------------------------------------------------------------

fn row_to_card(row: &rusqlite::Row) -> rusqlite::Result<Card> {
    let metadata_str: String = row.get(6)?;
    let metadata: Value =
        serde_json::from_str(&metadata_str).unwrap_or_else(|_| Value::Object(Default::default()));
    Ok(Card {
        id: row.get(0)?,
        knowledge_item_id: row.get(1)?,
        question: row.get(2)?,
        answer: row.get(3)?,
        card_type: row.get(4)?,
        difficulty: row.get(5)?,
        metadata,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        due_at: row.get(9)?,
        interval: row.get(10)?,
        ease_factor: row.get(11)?,
        review_count: row.get(12)?,
    })
}

// ---------------------------------------------------------------------------
// Tags
// ---------------------------------------------------------------------------

fn upsert_tag(conn: &Connection, name: &str) -> AppResult<i64> {
    conn.execute("INSERT OR IGNORE INTO tags(name) VALUES (?1)", params![name])?;
    let id: i64 = conn.query_row(
        "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
        params![name],
        |r| r.get(0),
    )?;
    Ok(id)
}

fn set_item_tags(conn: &Connection, item_id: i64, tags: &[String]) -> AppResult<()> {
    conn.execute(
        "DELETE FROM knowledge_item_tags WHERE knowledge_item_id = ?1",
        params![item_id],
    )?;
    for tag in normalize_tags(tags) {
        let tag_id = upsert_tag(conn, &tag)?;
        conn.execute(
            "INSERT OR IGNORE INTO knowledge_item_tags(knowledge_item_id, tag_id) VALUES (?1, ?2)",
            params![item_id, tag_id],
        )?;
    }
    prune_orphan_tags(conn)?;
    Ok(())
}

fn prune_orphan_tags(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM knowledge_item_tags)",
        [],
    )?;
    Ok(())
}

fn tags_for_item(conn: &Connection, item_id: i64) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT t.name FROM tags t \
         JOIN knowledge_item_tags kit ON kit.tag_id = t.id \
         WHERE kit.knowledge_item_id = ?1 ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map(params![item_id], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn tags_for_items(conn: &Connection, ids: &[i64]) -> AppResult<HashMap<i64, Vec<String>>> {
    let mut map: HashMap<i64, Vec<String>> = HashMap::new();
    if ids.is_empty() {
        return Ok(map);
    }
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!(
        "SELECT kit.knowledge_item_id, t.name FROM knowledge_item_tags kit \
         JOIN tags t ON t.id = kit.tag_id \
         WHERE kit.knowledge_item_id IN ({placeholders}) ORDER BY t.name COLLATE NOCASE"
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<SqlValue> = ids.iter().map(|i| SqlValue::Integer(*i)).collect();
    let rows = stmt.query_map(params_from_iter(params), |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (id, name) = row?;
        map.entry(id).or_default().push(name);
    }
    Ok(map)
}

pub fn list_all_tags(conn: &Connection) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM tags ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

// ---------------------------------------------------------------------------
// Knowledge items
// ---------------------------------------------------------------------------

pub fn create_item(conn: &Connection, input: &KnowledgeItemInput, now: i64) -> AppResult<KnowledgeItem> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(AppError::Invalid("title cannot be empty".into()));
    }
    conn.execute(
        "INSERT INTO knowledge_items(title, content, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![title, input.content, now],
    )?;
    let id = conn.last_insert_rowid();
    set_item_tags(conn, id, &input.tags)?;
    get_item(conn, id)
}

pub fn update_item(
    conn: &Connection,
    id: i64,
    input: &KnowledgeItemInput,
    now: i64,
) -> AppResult<KnowledgeItem> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(AppError::Invalid("title cannot be empty".into()));
    }
    let changed = conn.execute(
        "UPDATE knowledge_items SET title = ?1, content = ?2, updated_at = ?3 WHERE id = ?4",
        params![title, input.content, now, id],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("knowledge item {id}")));
    }
    set_item_tags(conn, id, &input.tags)?;
    get_item(conn, id)
}

pub fn get_item(conn: &Connection, id: i64) -> AppResult<KnowledgeItem> {
    let mut item = conn.query_row(
        "SELECT id, title, content, created_at, updated_at FROM knowledge_items WHERE id = ?1",
        params![id],
        |r| {
            Ok(KnowledgeItem {
                id: r.get(0)?,
                title: r.get(1)?,
                content: r.get(2)?,
                created_at: r.get(3)?,
                updated_at: r.get(4)?,
                tags: Vec::new(),
            })
        },
    )?;
    item.tags = tags_for_item(conn, id)?;
    Ok(item)
}

pub fn delete_item(conn: &Connection, id: i64) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM knowledge_items WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("knowledge item {id}")));
    }
    prune_orphan_tags(conn)?;
    Ok(())
}

pub fn list_items(
    conn: &Connection,
    search: Option<&str>,
    tag: Option<&str>,
    now: i64,
) -> AppResult<Vec<KnowledgeItemSummary>> {
    let mut sql = String::from(
        "SELECT ki.id, ki.title, substr(ki.content, 1, 240), ki.created_at, ki.updated_at, \
         (SELECT COUNT(*) FROM cards c WHERE c.knowledge_item_id = ki.id), \
         (SELECT COUNT(*) FROM cards c WHERE c.knowledge_item_id = ki.id AND c.due_at <= ?1) \
         FROM knowledge_items ki",
    );
    let mut clauses: Vec<String> = Vec::new();
    let mut bind: Vec<SqlValue> = vec![SqlValue::Integer(now)];

    if let Some(term) = search.map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{}%", escape_like(term));
        // Reuse one bound value across the OR branches.
        bind.push(SqlValue::Text(like));
        let p = bind.len(); // 1-based index of the LIKE param
        clauses.push(format!(
            "(ki.title LIKE ?{p} ESCAPE '\\' OR ki.content LIKE ?{p} ESCAPE '\\' \
             OR EXISTS(SELECT 1 FROM cards c WHERE c.knowledge_item_id = ki.id \
                       AND (c.question LIKE ?{p} ESCAPE '\\' OR c.answer LIKE ?{p} ESCAPE '\\')) \
             OR EXISTS(SELECT 1 FROM knowledge_item_tags kit JOIN tags t ON t.id = kit.tag_id \
                       WHERE kit.knowledge_item_id = ki.id AND t.name LIKE ?{p} ESCAPE '\\'))"
        ));
    }

    if let Some(tag) = tag.map(str::trim).filter(|s| !s.is_empty()) {
        bind.push(SqlValue::Text(tag.to_string()));
        let p = bind.len();
        clauses.push(format!(
            "EXISTS(SELECT 1 FROM knowledge_item_tags kit JOIN tags t ON t.id = kit.tag_id \
             WHERE kit.knowledge_item_id = ki.id AND t.name = ?{p} COLLATE NOCASE)"
        ));
    }

    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY ki.updated_at DESC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(bind), |r| {
        Ok(KnowledgeItemSummary {
            id: r.get(0)?,
            title: r.get(1)?,
            excerpt: r.get(2)?,
            created_at: r.get(3)?,
            updated_at: r.get(4)?,
            tags: Vec::new(),
            card_count: r.get(5)?,
            due_count: r.get(6)?,
        })
    })?;
    let mut summaries: Vec<KnowledgeItemSummary> = rows.collect::<Result<_, _>>()?;

    let ids: Vec<i64> = summaries.iter().map(|s| s.id).collect();
    let mut tagmap = tags_for_items(conn, &ids)?;
    for s in &mut summaries {
        s.tags = tagmap.remove(&s.id).unwrap_or_default();
    }
    Ok(summaries)
}

/// Escape LIKE wildcards in user search input so `%` and `_` are literal.
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

// ---------------------------------------------------------------------------
// Cards
// ---------------------------------------------------------------------------

pub fn create_card(conn: &Connection, input: &CardInput, now: i64) -> AppResult<Card> {
    validate_card_type(&input.card_type)?;
    validate_difficulty(&input.difficulty)?;
    if input.question.trim().is_empty() {
        return Err(AppError::Invalid("question cannot be empty".into()));
    }
    // Ensure the parent exists (defensive; FK would also catch it).
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM knowledge_items WHERE id = ?1",
            params![input.knowledge_item_id],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !exists {
        return Err(AppError::NotFound(format!(
            "knowledge item {}",
            input.knowledge_item_id
        )));
    }
    let metadata = serde_json::to_string(&input.metadata)?;
    conn.execute(
        "INSERT INTO cards(knowledge_item_id, question, answer, type, difficulty, metadata, \
         created_at, updated_at, due_at, interval, ease_factor, review_count) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?7, 0, 2.5, 0)",
        params![
            input.knowledge_item_id,
            input.question.trim(),
            input.answer,
            input.card_type,
            input.difficulty,
            metadata,
            now
        ],
    )?;
    get_card(conn, conn.last_insert_rowid())
}

pub fn update_card(conn: &Connection, id: i64, input: &CardInput, now: i64) -> AppResult<Card> {
    validate_card_type(&input.card_type)?;
    validate_difficulty(&input.difficulty)?;
    if input.question.trim().is_empty() {
        return Err(AppError::Invalid("question cannot be empty".into()));
    }
    let metadata = serde_json::to_string(&input.metadata)?;
    let changed = conn.execute(
        "UPDATE cards SET question = ?1, answer = ?2, type = ?3, difficulty = ?4, metadata = ?5, \
         updated_at = ?6 WHERE id = ?7",
        params![
            input.question.trim(),
            input.answer,
            input.card_type,
            input.difficulty,
            metadata,
            now,
            id
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("card {id}")));
    }
    get_card(conn, id)
}

pub fn get_card(conn: &Connection, id: i64) -> AppResult<Card> {
    let sql = format!("SELECT {CARD_COLS} FROM cards WHERE id = ?1");
    Ok(conn.query_row(&sql, params![id], row_to_card)?)
}

pub fn delete_card(conn: &Connection, id: i64) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM cards WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("card {id}")));
    }
    Ok(())
}

pub fn list_cards_for_item(conn: &Connection, item_id: i64) -> AppResult<Vec<Card>> {
    let sql = format!("SELECT {CARD_COLS} FROM cards WHERE knowledge_item_id = ?1 ORDER BY created_at ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![item_id], row_to_card)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Build a review queue: cards already due (review_count > 0, due_at <= now),
/// followed by up to `new_limit` brand-new cards. Optionally scoped to a
/// single knowledge item or a tag. `limit` caps the total.
pub fn due_cards(
    conn: &Connection,
    now: i64,
    limit: i64,
    include_new: bool,
    new_limit: i64,
    item_id: Option<i64>,
    tag: Option<&str>,
) -> AppResult<Vec<DueCard>> {
    let mut out: Vec<DueCard> = Vec::new();

    // Build a scoped query using positional params and owned values (avoids any
    // borrow-lifetime issues with dynamic parameter lists).
    let build = |core: &str, order: &str| -> (String, Vec<SqlValue>) {
        let mut sql = format!(
            "SELECT {CARD_COLS}, \
             (SELECT title FROM knowledge_items ki WHERE ki.id = c.knowledge_item_id) \
             FROM cards c WHERE {core}"
        );
        let mut bind: Vec<SqlValue> = Vec::new();
        if let Some(iid) = item_id {
            sql.push_str(" AND c.knowledge_item_id = ?");
            bind.push(SqlValue::Integer(iid));
        }
        if let Some(t) = tag {
            sql.push_str(
                " AND EXISTS(SELECT 1 FROM knowledge_item_tags kit JOIN tags t ON t.id = kit.tag_id \
                 WHERE kit.knowledge_item_id = c.knowledge_item_id AND t.name = ? COLLATE NOCASE)",
            );
            bind.push(SqlValue::Text(t.to_string()));
        }
        sql.push_str(order);
        (sql, bind)
    };

    // 1) Review-due cards (reviewed at least once, due on or before now).
    {
        let (sql, mut bind) = build("c.review_count > 0 AND c.due_at <= ?", " ORDER BY c.due_at ASC LIMIT ?");
        // The `due_at <= ?` placeholder is the FIRST param, so prepend `now`.
        bind.insert(0, SqlValue::Integer(now));
        bind.push(SqlValue::Integer(limit));
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(bind), row_to_due_card)?;
        for row in rows {
            out.push(row?);
        }
    }

    // 2) New (never-reviewed) cards, capped separately.
    if include_new && (out.len() as i64) < limit {
        let cap = (limit - out.len() as i64).min(new_limit.max(0));
        if cap > 0 {
            let (sql, mut bind) = build("c.review_count = 0", " ORDER BY c.created_at ASC LIMIT ?");
            bind.push(SqlValue::Integer(cap));
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params_from_iter(bind), row_to_due_card)?;
            for row in rows {
                out.push(row?);
            }
        }
    }

    Ok(out)
}

fn row_to_due_card(r: &rusqlite::Row) -> rusqlite::Result<DueCard> {
    let card = row_to_card(r)?;
    let knowledge_title: String = r.get(13)?;
    Ok(DueCard {
        card,
        knowledge_title,
    })
}

// ---------------------------------------------------------------------------
// Reviews / scheduling
// ---------------------------------------------------------------------------

/// Persist a review: update the card's scheduling fields and insert a review
/// row, atomically. The next-review computation is done by the frontend
/// scheduler module and passed in as `schedule`.
pub fn record_review(
    conn: &mut Connection,
    card_id: i64,
    rating: &str,
    user_answer: Option<&str>,
    ai_evaluation: Option<&str>,
    schedule: &ScheduleUpdate,
    now: i64,
) -> AppResult<Card> {
    validate_rating(rating)?;
    if schedule.interval < 0.0 || !schedule.interval.is_finite() {
        return Err(AppError::Invalid("interval must be a non-negative number".into()));
    }
    if schedule.ease_factor < 1.0 || !schedule.ease_factor.is_finite() {
        return Err(AppError::Invalid("ease factor out of range".into()));
    }

    let tx = conn.transaction()?;
    let changed = tx.execute(
        "UPDATE cards SET interval = ?1, ease_factor = ?2, due_at = ?3, review_count = ?4, \
         updated_at = ?5 WHERE id = ?6",
        params![
            schedule.interval,
            schedule.ease_factor,
            schedule.due_at,
            schedule.review_count,
            now,
            card_id
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("card {card_id}")));
    }
    tx.execute(
        "INSERT INTO reviews(card_id, rating, user_answer, ai_evaluation, reviewed_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![card_id, rating, user_answer, ai_evaluation, now],
    )?;
    tx.commit()?;
    get_card(conn, card_id)
}

// ---------------------------------------------------------------------------
// Dashboard stats
// ---------------------------------------------------------------------------

pub fn dashboard_stats(conn: &Connection, now: i64) -> AppResult<DashboardStats> {
    let due_now: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cards WHERE review_count > 0 AND due_at <= ?1",
        params![now],
        |r| r.get(0),
    )?;
    // "Due today" compares local calendar dates directly, which sidesteps the
    // SQLite localtime→epoch conversion pitfall.
    let due_today: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cards WHERE review_count > 0 \
         AND date(due_at/1000, 'unixepoch', 'localtime') <= date('now', 'localtime')",
        [],
        |r| r.get(0),
    )?;
    let new_cards: i64 =
        conn.query_row("SELECT COUNT(*) FROM cards WHERE review_count = 0", [], |r| r.get(0))?;
    let total_cards: i64 = conn.query_row("SELECT COUNT(*) FROM cards", [], |r| r.get(0))?;
    let total_items: i64 =
        conn.query_row("SELECT COUNT(*) FROM knowledge_items", [], |r| r.get(0))?;
    // "Mastered" = interval has grown to 21+ days (a common maturity threshold).
    let mastered: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cards WHERE interval >= 21.0",
        [],
        |r| r.get(0),
    )?;
    let reviewed_today: i64 = conn.query_row(
        "SELECT COUNT(*) FROM reviews \
         WHERE date(reviewed_at/1000, 'unixepoch', 'localtime') = date('now', 'localtime')",
        [],
        |r| r.get(0),
    )?;

    let streak = compute_streak(conn)?;
    let recent_activity = recent_activity(conn, 12)?;

    Ok(DashboardStats {
        due_now,
        due_today,
        new_cards,
        reviewed_today,
        total_cards,
        total_items,
        mastered,
        streak,
        recent_activity,
    })
}

/// Consecutive days (ending today or yesterday) with at least one review.
fn compute_streak(conn: &Connection) -> AppResult<i64> {
    let today: i64 = conn.query_row(
        "SELECT CAST(julianday(date('now','localtime')) AS INTEGER)",
        [],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(
        "SELECT DISTINCT CAST(julianday(date(reviewed_at/1000,'unixepoch','localtime')) AS INTEGER) \
         FROM reviews",
    )?;
    let days: HashSet<i64> = stmt
        .query_map([], |r| r.get::<_, i64>(0))?
        .collect::<Result<_, _>>()?;

    if days.is_empty() {
        return Ok(0);
    }
    // Allow a streak that is "current" as of today or yesterday.
    let mut cursor = if days.contains(&today) {
        today
    } else if days.contains(&(today - 1)) {
        today - 1
    } else {
        return Ok(0);
    };
    let mut streak = 0;
    while days.contains(&cursor) {
        streak += 1;
        cursor -= 1;
    }
    Ok(streak)
}

fn recent_activity(conn: &Connection, limit: i64) -> AppResult<Vec<ActivityEntry>> {
    let mut stmt = conn.prepare(
        "SELECT r.reviewed_at, r.rating, r.card_id, c.question, ki.title \
         FROM reviews r \
         JOIN cards c ON c.id = r.card_id \
         JOIN knowledge_items ki ON ki.id = c.knowledge_item_id \
         ORDER BY r.reviewed_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |r| {
        Ok(ActivityEntry {
            reviewed_at: r.get(0)?,
            rating: r.get(1)?,
            card_id: r.get(2)?,
            question: r.get(3)?,
            knowledge_title: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

// ---------------------------------------------------------------------------
// Settings (non-sensitive key/value)
// ---------------------------------------------------------------------------

pub fn get_setting(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    let value = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| {
            r.get::<_, String>(0)
        })
        .ok();
    Ok(value)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Knowledge-base retrieval (for chat)
// ---------------------------------------------------------------------------

/// A retrieved note used as context for a chat answer.
pub struct ContextItem {
    pub id: i64,
    pub title: String,
    pub content: String,
}

/// Find the notes most relevant to a set of query keywords. Lightweight: a
/// keyword LIKE filter narrows candidates, then we rank them in Rust by keyword
/// frequency (title weighted higher). No embeddings or external index.
pub fn search_context(
    conn: &Connection,
    keywords: &[String],
    limit: usize,
) -> AppResult<Vec<ContextItem>> {
    if keywords.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let mut clauses: Vec<String> = Vec::new();
    let mut binds: Vec<SqlValue> = Vec::new();
    for kw in keywords {
        let like = format!("%{}%", escape_like(kw));
        binds.push(SqlValue::Text(like.clone()));
        let p1 = binds.len();
        binds.push(SqlValue::Text(like));
        let p2 = binds.len();
        clauses.push(format!(
            "(title LIKE ?{p1} ESCAPE '\\' OR content LIKE ?{p2} ESCAPE '\\')"
        ));
    }
    let sql = format!(
        "SELECT id, title, content FROM knowledge_items WHERE {} ORDER BY updated_at DESC LIMIT 100",
        clauses.join(" OR ")
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(binds), |r| {
        Ok(ContextItem {
            id: r.get(0)?,
            title: r.get(1)?,
            content: r.get(2)?,
        })
    })?;
    let mut items: Vec<ContextItem> = rows.collect::<Result<_, _>>()?;

    items.sort_by_key(|it| std::cmp::Reverse(context_score(it, keywords)));
    items.truncate(limit);
    Ok(items)
}

fn context_score(item: &ContextItem, keywords: &[String]) -> i64 {
    let title = item.title.to_lowercase();
    let content = item.content.to_lowercase();
    let mut score = 0i64;
    for kw in keywords {
        score += (title.matches(kw.as_str()).count() as i64) * 3;
        score += content.matches(kw.as_str()).count() as i64;
    }
    score
}

// ---------------------------------------------------------------------------
// Import / Export
// ---------------------------------------------------------------------------

pub fn export_bundle(conn: &Connection, now: i64) -> AppResult<ExportBundle> {
    let mut items_stmt = conn.prepare(
        "SELECT id, title, content, created_at, updated_at FROM knowledge_items ORDER BY id",
    )?;
    let items: Vec<ExportKnowledgeItem> = items_stmt
        .query_map([], |r| {
            Ok(ExportKnowledgeItem {
                id: r.get(0)?,
                title: r.get(1)?,
                content: r.get(2)?,
                created_at: r.get(3)?,
                updated_at: r.get(4)?,
                tags: Vec::new(),
            })
        })?
        .collect::<Result<_, _>>()?;

    let mut items = items;
    for item in &mut items {
        item.tags = tags_for_item(conn, item.id)?;
    }

    let sql = format!("SELECT {CARD_COLS} FROM cards ORDER BY id");
    let mut cards_stmt = conn.prepare(&sql)?;
    let cards: Vec<ExportCard> = cards_stmt
        .query_map([], |r| {
            let card = row_to_card(r)?;
            Ok(ExportCard {
                id: card.id,
                knowledge_item_id: card.knowledge_item_id,
                question: card.question,
                answer: card.answer,
                card_type: card.card_type,
                difficulty: card.difficulty,
                metadata: card.metadata,
                created_at: card.created_at,
                updated_at: card.updated_at,
                due_at: card.due_at,
                interval: card.interval,
                ease_factor: card.ease_factor,
                review_count: card.review_count,
            })
        })?
        .collect::<Result<_, _>>()?;

    let mut reviews_stmt = conn.prepare(
        "SELECT card_id, rating, user_answer, ai_evaluation, reviewed_at FROM reviews ORDER BY id",
    )?;
    let reviews: Vec<ExportReview> = reviews_stmt
        .query_map([], |r| {
            Ok(ExportReview {
                card_id: r.get(0)?,
                rating: r.get(1)?,
                user_answer: r.get(2)?,
                ai_evaluation: r.get(3)?,
                reviewed_at: r.get(4)?,
            })
        })?
        .collect::<Result<_, _>>()?;

    Ok(ExportBundle {
        version: EXPORT_VERSION,
        exported_at: now,
        knowledge_items: items,
        cards,
        reviews,
    })
}

/// Import a bundle. Rows are inserted with fresh ids; the bundle's item ids are
/// remapped so cards and reviews stay attached to the right parents. Returns a
/// summary count of (items, cards, reviews) inserted.
pub fn import_bundle(
    conn: &mut Connection,
    bundle: &ExportBundle,
) -> AppResult<(usize, usize, usize)> {
    if bundle.version != EXPORT_VERSION {
        return Err(AppError::Invalid(format!(
            "unsupported export version {} (this app understands version {})",
            bundle.version, EXPORT_VERSION
        )));
    }

    let tx = conn.transaction()?;
    let mut item_id_map: HashMap<i64, i64> = HashMap::new();
    let mut card_id_map: HashMap<i64, i64> = HashMap::new();

    for item in &bundle.knowledge_items {
        tx.execute(
            "INSERT INTO knowledge_items(title, content, created_at, updated_at) VALUES (?1,?2,?3,?4)",
            params![item.title, item.content, item.created_at, item.updated_at],
        )?;
        let new_id = tx.last_insert_rowid();
        item_id_map.insert(item.id, new_id);

        for tag in normalize_tags(&item.tags) {
            tx.execute("INSERT OR IGNORE INTO tags(name) VALUES (?1)", params![tag])?;
            let tag_id: i64 = tx.query_row(
                "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
                params![tag],
                |r| r.get(0),
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO knowledge_item_tags(knowledge_item_id, tag_id) VALUES (?1,?2)",
                params![new_id, tag_id],
            )?;
        }
    }

    let mut card_count = 0usize;
    for card in &bundle.cards {
        let Some(&new_item_id) = item_id_map.get(&card.knowledge_item_id) else {
            // Skip cards whose parent wasn't in the bundle rather than failing.
            continue;
        };
        validate_card_type(&card.card_type)?;
        validate_difficulty(&card.difficulty)?;
        let metadata = serde_json::to_string(&card.metadata)?;
        tx.execute(
            "INSERT INTO cards(knowledge_item_id, question, answer, type, difficulty, metadata, \
             created_at, updated_at, due_at, interval, ease_factor, review_count) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                new_item_id,
                card.question,
                card.answer,
                card.card_type,
                card.difficulty,
                metadata,
                card.created_at,
                card.updated_at,
                card.due_at,
                card.interval,
                card.ease_factor,
                card.review_count
            ],
        )?;
        card_id_map.insert(card.id, tx.last_insert_rowid());
        card_count += 1;
    }

    let mut review_count = 0usize;
    for review in &bundle.reviews {
        let Some(&new_card_id) = card_id_map.get(&review.card_id) else {
            continue;
        };
        validate_rating(&review.rating)?;
        tx.execute(
            "INSERT INTO reviews(card_id, rating, user_answer, ai_evaluation, reviewed_at) \
             VALUES (?1,?2,?3,?4,?5)",
            params![
                new_card_id,
                review.rating,
                review.user_answer,
                review.ai_evaluation,
                review.reviewed_at
            ],
        )?;
        review_count += 1;
    }

    tx.commit()?;
    Ok((item_id_map.len(), card_count, review_count))
}
