//! Database tests: migrations, CRUD, scheduling persistence, search, and a
//! full export → import → export round-trip equivalence check.

use serde_json::json;

use super::models::*;
use super::{migrations, now_ms, open_in_memory, repository as repo};

const NOW: i64 = 1_700_000_000_000;

fn item_input(title: &str, tags: &[&str]) -> KnowledgeItemInput {
    KnowledgeItemInput {
        title: title.into(),
        content: format!("Content for {title}"),
        tags: tags.iter().map(|s| s.to_string()).collect(),
    }
}

fn card_input(item_id: i64, q: &str, a: &str) -> CardInput {
    CardInput {
        knowledge_item_id: item_id,
        question: q.into(),
        answer: a.into(),
        card_type: "basic".into(),
        difficulty: "medium".into(),
        metadata: json!({}),
    }
}

#[test]
fn migrations_set_user_version() {
    let conn = open_in_memory().unwrap();
    let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(v, migrations::latest_version());
}

#[test]
fn migrations_are_idempotent() {
    let mut conn = open_in_memory().unwrap();
    // Running again should be a no-op, not an error.
    migrations::run(&mut conn).unwrap();
    let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    assert_eq!(v, migrations::latest_version());
}

#[test]
fn knowledge_item_crud_with_tags() {
    let conn = open_in_memory().unwrap();

    let item = repo::create_item(&conn, &item_input("TCP", &["net", "core"]), NOW).unwrap();
    assert_eq!(item.title, "TCP");
    assert_eq!(item.tags.len(), 2);

    let fetched = repo::get_item(&conn, item.id).unwrap();
    assert_eq!(fetched.content, "Content for TCP");

    let updated = repo::update_item(
        &conn,
        item.id,
        &KnowledgeItemInput {
            title: "TCP/IP".into(),
            content: "Updated".into(),
            tags: vec!["net".into()],
        },
        NOW + 1,
    )
    .unwrap();
    assert_eq!(updated.title, "TCP/IP");
    assert_eq!(updated.tags, vec!["net"]);
    assert_eq!(updated.updated_at, NOW + 1);

    // The now-unused "core" tag should have been pruned.
    let tags = repo::list_all_tags(&conn).unwrap();
    assert_eq!(tags, vec!["net"]);

    repo::delete_item(&conn, item.id).unwrap();
    assert!(repo::get_item(&conn, item.id).is_err());
    assert!(repo::list_all_tags(&conn).unwrap().is_empty());
}

#[test]
fn empty_title_is_rejected() {
    let conn = open_in_memory().unwrap();
    let err = repo::create_item(&conn, &item_input("   ", &[]), NOW).unwrap_err();
    assert_eq!(err.kind(), "invalid");
}

#[test]
fn card_crud_and_cascade_delete() {
    let conn = open_in_memory().unwrap();
    let item = repo::create_item(&conn, &item_input("Bio", &[]), NOW).unwrap();

    let card = repo::create_card(&conn, &card_input(item.id, "Q?", "A."), NOW).unwrap();
    assert_eq!(card.ease_factor, 2.5);
    assert_eq!(card.review_count, 0);
    assert_eq!(card.due_at, NOW); // new cards are immediately available

    let updated = repo::update_card(
        &conn,
        card.id,
        &CardInput {
            knowledge_item_id: item.id,
            question: "Q2?".into(),
            answer: "A2".into(),
            card_type: "mcq".into(),
            difficulty: "hard".into(),
            metadata: json!({ "distractors": ["x", "y"] }),
        },
        NOW + 5,
    )
    .unwrap();
    assert_eq!(updated.card_type, "mcq");
    assert_eq!(updated.metadata["distractors"][0], "x");

    assert_eq!(repo::list_cards_for_item(&conn, item.id).unwrap().len(), 1);

    // Deleting the parent cascades to its cards.
    repo::delete_item(&conn, item.id).unwrap();
    assert!(repo::list_cards_for_item(&conn, item.id).unwrap().is_empty());
}

#[test]
fn invalid_card_type_rejected() {
    let conn = open_in_memory().unwrap();
    let item = repo::create_item(&conn, &item_input("X", &[]), NOW).unwrap();
    let mut input = card_input(item.id, "Q", "A");
    input.card_type = "bogus".into();
    assert_eq!(repo::create_card(&conn, &input, NOW).unwrap_err().kind(), "invalid");
}

#[test]
fn due_queue_orders_review_then_new() {
    let mut conn = open_in_memory().unwrap();
    let item = repo::create_item(&conn, &item_input("Q", &[]), NOW).unwrap();
    let c1 = repo::create_card(&conn, &card_input(item.id, "review-due", "a"), NOW).unwrap();
    let _c2 = repo::create_card(&conn, &card_input(item.id, "new-card", "b"), NOW).unwrap();

    // Make c1 a review-due card (reviewed once, due in the past).
    repo::record_review(
        &mut conn,
        c1.id,
        "good",
        None,
        None,
        &ScheduleUpdate {
            interval: 1.0,
            ease_factor: 2.5,
            due_at: NOW - 1000,
            review_count: 1,
        },
        NOW,
    )
    .unwrap();

    let queue = repo::due_cards(&conn, NOW, 100, true, 20, None, None).unwrap();
    assert_eq!(queue.len(), 2);
    assert_eq!(queue[0].card.question, "review-due");
    assert_eq!(queue[1].card.question, "new-card");

    // Excluding new cards leaves only the review-due one.
    let only_due = repo::due_cards(&conn, NOW, 100, false, 0, None, None).unwrap();
    assert_eq!(only_due.len(), 1);
}

#[test]
fn record_review_persists_schedule_and_inserts_review() {
    let mut conn = open_in_memory().unwrap();
    let item = repo::create_item(&conn, &item_input("Q", &[]), NOW).unwrap();
    let card = repo::create_card(&conn, &card_input(item.id, "q", "a"), NOW).unwrap();

    let updated = repo::record_review(
        &mut conn,
        card.id,
        "good",
        Some("my answer"),
        None,
        &ScheduleUpdate {
            interval: 4.0,
            ease_factor: 2.6,
            due_at: NOW + 4 * 86_400_000,
            review_count: 1,
        },
        NOW,
    )
    .unwrap();
    assert_eq!(updated.review_count, 1);
    assert_eq!(updated.interval, 4.0);

    let review_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM reviews WHERE card_id = ?1", [card.id], |r| r.get(0))
        .unwrap();
    assert_eq!(review_count, 1);
}

#[test]
fn invalid_schedule_is_rejected() {
    let mut conn = open_in_memory().unwrap();
    let item = repo::create_item(&conn, &item_input("Q", &[]), NOW).unwrap();
    let card = repo::create_card(&conn, &card_input(item.id, "q", "a"), NOW).unwrap();
    let bad = ScheduleUpdate {
        interval: -5.0,
        ease_factor: 2.5,
        due_at: NOW,
        review_count: 1,
    };
    assert!(repo::record_review(&mut conn, card.id, "good", None, None, &bad, NOW).is_err());
}

#[test]
fn search_matches_title_content_question_and_tag() {
    let conn = open_in_memory().unwrap();
    let networking = repo::create_item(&conn, &item_input("Networking", &["cs"]), NOW).unwrap();
    repo::create_card(&conn, &card_input(networking.id, "What is a handshake?", "SYN"), NOW).unwrap();
    repo::create_item(&conn, &item_input("Cooking", &["food"]), NOW).unwrap();

    assert_eq!(repo::list_items(&conn, Some("network"), None, NOW).unwrap().len(), 1);
    assert_eq!(repo::list_items(&conn, Some("handshake"), None, NOW).unwrap().len(), 1);
    assert_eq!(repo::list_items(&conn, Some("cs"), None, NOW).unwrap().len(), 1);
    assert_eq!(repo::list_items(&conn, None, Some("food"), NOW).unwrap().len(), 1);
    assert_eq!(repo::list_items(&conn, None, None, NOW).unwrap().len(), 2);
    // A wildcard character must be treated literally, not as a pattern.
    assert_eq!(repo::list_items(&conn, Some("%"), None, NOW).unwrap().len(), 0);
}

#[test]
fn dashboard_stats_reflect_activity() {
    let mut conn = open_in_memory().unwrap();
    let item = repo::create_item(&conn, &item_input("Q", &[]), NOW).unwrap();
    let mastered_card = repo::create_card(&conn, &card_input(item.id, "mastered", "a"), NOW).unwrap();
    let _new_card = repo::create_card(&conn, &card_input(item.id, "new", "b"), NOW).unwrap();

    repo::record_review(
        &mut conn,
        mastered_card.id,
        "easy",
        None,
        None,
        &ScheduleUpdate {
            interval: 30.0,
            ease_factor: 2.8,
            due_at: now_ms() + 30 * 86_400_000,
            review_count: 1,
        },
        now_ms(),
    )
    .unwrap();

    let stats = repo::dashboard_stats(&conn, now_ms()).unwrap();
    assert_eq!(stats.total_cards, 2);
    assert_eq!(stats.total_items, 1);
    assert_eq!(stats.new_cards, 1);
    assert_eq!(stats.mastered, 1);
    assert_eq!(stats.reviewed_today, 1);
    assert_eq!(stats.streak, 1);
    assert_eq!(stats.recent_activity.len(), 1);
    assert_eq!(stats.recent_activity[0].question, "mastered");
}

// ---- Round-trip export/import ----------------------------------------------

/// Canonical, id-independent projection of a bundle for equivalence comparison.
fn project(b: &ExportBundle) -> (Vec<String>, Vec<String>, Vec<String>) {
    let title_of = |id: i64| {
        b.knowledge_items
            .iter()
            .find(|i| i.id == id)
            .map(|i| i.title.clone())
            .unwrap_or_default()
    };
    let question_of = |id: i64| {
        b.cards
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.question.clone())
            .unwrap_or_default()
    };

    let mut items: Vec<String> = b
        .knowledge_items
        .iter()
        .map(|i| {
            let mut tags = i.tags.clone();
            tags.sort();
            format!("{}|{}|{}", i.title, i.content, tags.join(","))
        })
        .collect();
    items.sort();

    let mut cards: Vec<String> = b
        .cards
        .iter()
        .map(|c| {
            format!(
                "{}|{}|{}|{}|{}",
                title_of(c.knowledge_item_id),
                c.question,
                c.answer,
                c.card_type,
                c.interval
            )
        })
        .collect();
    cards.sort();

    let mut reviews: Vec<String> = b
        .reviews
        .iter()
        .map(|r| format!("{}|{}|{}", question_of(r.card_id), r.rating, r.reviewed_at))
        .collect();
    reviews.sort();

    (items, cards, reviews)
}

#[test]
fn export_import_roundtrip_preserves_data() {
    let mut db1 = open_in_memory().unwrap();

    let tcp = repo::create_item(&db1, &item_input("TCP", &["net", "core"]), NOW).unwrap();
    let dns = repo::create_item(&db1, &item_input("DNS", &["net"]), NOW).unwrap();
    let c1 = repo::create_card(&db1, &card_input(tcp.id, "handshake steps?", "SYN/SYN-ACK/ACK"), NOW).unwrap();
    repo::create_card(&db1, &card_input(dns.id, "what is a record?", "address mapping"), NOW).unwrap();
    repo::record_review(
        &mut db1,
        c1.id,
        "good",
        Some("SYN then ACK"),
        None,
        &ScheduleUpdate {
            interval: 3.0,
            ease_factor: 2.5,
            due_at: NOW + 3 * 86_400_000,
            review_count: 1,
        },
        NOW,
    )
    .unwrap();

    let exported = repo::export_bundle(&db1, NOW).unwrap();
    assert_eq!(exported.version, EXPORT_VERSION);
    assert_eq!(exported.knowledge_items.len(), 2);
    assert_eq!(exported.cards.len(), 2);
    assert_eq!(exported.reviews.len(), 1);

    // Serialize and re-parse to prove the on-disk JSON is faithful.
    let jsonstr = serde_json::to_string_pretty(&exported).unwrap();
    let reparsed: ExportBundle = serde_json::from_str(&jsonstr).unwrap();

    let mut db2 = open_in_memory().unwrap();
    let (i, c, r) = repo::import_bundle(&mut db2, &reparsed).unwrap();
    assert_eq!((i, c, r), (2, 2, 1));

    let exported2 = repo::export_bundle(&db2, NOW).unwrap();
    assert_eq!(project(&exported), project(&exported2));
}

#[test]
fn import_rejects_wrong_version() {
    let mut conn = open_in_memory().unwrap();
    let bundle = ExportBundle {
        version: 999,
        exported_at: NOW,
        knowledge_items: vec![],
        cards: vec![],
        reviews: vec![],
    };
    assert!(repo::import_bundle(&mut conn, &bundle).is_err());
}

#[test]
fn search_context_finds_and_ranks_relevant_notes() {
    let conn = open_in_memory().unwrap();
    repo::create_item(&conn, &item_input("TCP handshake", &[]), NOW).unwrap();
    repo::create_item(&conn, &item_input("Photosynthesis", &[]), NOW).unwrap();

    // Relevant keywords retrieve only the matching note.
    let ctx = repo::search_context(&conn, &["tcp".into(), "handshake".into()], 5).unwrap();
    assert_eq!(ctx.len(), 1);
    assert_eq!(ctx[0].title, "TCP handshake");

    // Unrelated keywords and empty input return nothing.
    assert!(repo::search_context(&conn, &["quantum".into()], 5).unwrap().is_empty());
    assert!(repo::search_context(&conn, &[], 5).unwrap().is_empty());
}

#[test]
fn settings_roundtrip() {
    let conn = open_in_memory().unwrap();
    assert_eq!(repo::get_setting(&conn, "theme").unwrap(), None);
    repo::set_setting(&conn, "theme", "dark").unwrap();
    assert_eq!(repo::get_setting(&conn, "theme").unwrap(), Some("dark".into()));
    repo::set_setting(&conn, "theme", "light").unwrap();
    assert_eq!(repo::get_setting(&conn, "theme").unwrap(), Some("light".into()));
}
