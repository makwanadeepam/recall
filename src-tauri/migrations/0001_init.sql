-- Recall initial schema (migration version 1).
-- All timestamps are stored as INTEGER milliseconds since the Unix epoch.
-- `cards.interval` is stored in days as a REAL so sub-day learning steps work.

CREATE TABLE knowledge_items (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT    NOT NULL,
    content     TEXT    NOT NULL DEFAULT '',
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

-- The library lists items most-recently-updated first.
CREATE INDEX idx_knowledge_items_updated_at ON knowledge_items(updated_at DESC);

CREATE TABLE cards (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    knowledge_item_id INTEGER NOT NULL REFERENCES knowledge_items(id) ON DELETE CASCADE,
    question          TEXT    NOT NULL,
    answer            TEXT    NOT NULL DEFAULT '',
    type              TEXT    NOT NULL DEFAULT 'basic',   -- basic | free | cloze | mcq | reverse
    difficulty        TEXT    NOT NULL DEFAULT 'medium',  -- easy | medium | hard (author hint)
    metadata          TEXT    NOT NULL DEFAULT '{}',      -- JSON: distractors, cloze data, ...
    created_at        INTEGER NOT NULL,
    updated_at        INTEGER NOT NULL,
    due_at            INTEGER NOT NULL,                   -- next review time (ms epoch)
    interval          REAL    NOT NULL DEFAULT 0,         -- days until next review
    ease_factor       REAL    NOT NULL DEFAULT 2.5,
    review_count      INTEGER NOT NULL DEFAULT 0
);

-- The scheduler queries "cards due before now" constantly; keep it indexed.
CREATE INDEX idx_cards_due_at ON cards(due_at);
CREATE INDEX idx_cards_knowledge_item_id ON cards(knowledge_item_id);

CREATE TABLE reviews (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id       INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    rating        TEXT    NOT NULL,   -- again | hard | good | easy
    user_answer   TEXT,
    ai_evaluation TEXT,               -- JSON snapshot of an AnswerEvaluation, if any
    reviewed_at   INTEGER NOT NULL
);

CREATE INDEX idx_reviews_card_id ON reviews(card_id);
CREATE INDEX idx_reviews_reviewed_at ON reviews(reviewed_at);

CREATE TABLE tags (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE TABLE knowledge_item_tags (
    knowledge_item_id INTEGER NOT NULL REFERENCES knowledge_items(id) ON DELETE CASCADE,
    tag_id            INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (knowledge_item_id, tag_id)
);

CREATE INDEX idx_kit_tag_id ON knowledge_item_tags(tag_id);

-- Non-sensitive application settings only. API keys are NEVER stored here;
-- they live in the OS keychain (see src/security/keychain.rs).
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
