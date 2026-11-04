use std::collections::BTreeSet;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use srs_core::content::SetFile;
use srs_server::{AppState, app, open_db, seed};
use tower::ServiceExt;

const TARGET_NOTES: [&str; 9] = [
    "old-all",
    "old-study",
    "old-card",
    "old-history",
    "old-new",
    "old-shared",
    "old-draft",
    "old-unavailable",
    "old-disabled",
];
const ELIGIBLE_NOTES: [&str; 6] = [
    "old-all",
    "old-study",
    "old-card",
    "old-history",
    "old-new",
    "old-shared",
];
const REVIEWED_AT: &str = "2026-10-23T12:00:00Z";
const DUE: &str = "2027-01-01T12:00:00Z";
const TEMPLATES: [&str; 2] = ["image-to-name", "name-to-image"];

async fn call(pool: &SqlitePool, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut state = AppState::new(
        pool.clone(),
        std::env::temp_dir().join("srs-set-resume-test-media"),
    );
    state.clock = Arc::new(|| Utc.with_ymd_and_hms(2026, 11, 3, 12, 0, 0).unwrap());
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(body.map_or(Body::empty(), |value| Body::from(value.to_string())))
        .unwrap();
    let response = app(state, None).oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!(String::from_utf8_lossy(&bytes)))
    };
    (status, value)
}

async fn get(pool: &SqlitePool, path: &str) -> Value {
    let (status, value) = call(pool, "GET", path, None).await;
    assert_eq!(status, StatusCode::OK, "GET {path}: {value}");
    value
}

async fn subscription(pool: &SqlitePool, set_id: &str, body: Value) -> (StatusCode, Value) {
    call(
        pool,
        "PUT",
        &format!("/api/sets/{set_id}/subscription"),
        Some(body),
    )
    .await
}

async fn set_summary(pool: &SqlitePool, set_id: &str) -> Value {
    get(pool, "/api/sets")
        .await
        .as_array()
        .unwrap()
        .iter()
        .find(|set| set["id"] == set_id)
        .unwrap()
        .clone()
}

async fn add_set(pool: &SqlitePool, id: &str, subscribed: bool, note_ids: &[&str]) {
    let notes: Vec<_> = note_ids
        .iter()
        .map(|id| {
            let mut note = json!({ "id": id, "name": id });
            if *id != "old-unavailable" {
                note["image"] = json!(format!("/study/fixture/{id}.svg"));
            }
            note
        })
        .collect();
    let file: SetFile = serde_json::from_value(json!({
        "set": { "id": id, "name": id, "default": subscribed },
        "note_type": {
            "id": "SetResumeFixture",
            "fields": [
                { "name": "name", "type": "text" },
                { "name": "image", "type": "image", "optional": true }
            ],
            "templates": [
                { "id": "image-to-name", "prompt": ["image"], "answer": ["name"], "mode": "flip" },
                { "id": "name-to-image", "prompt": ["name"], "answer": ["image"], "mode": "flip" }
            ]
        },
        "notes": notes
    }))
    .unwrap();
    file.validate().unwrap();
    seed::import_set(pool, &file).await.unwrap();
}

async fn mark_studied(pool: &SqlitePool, user: i64, note: &str) {
    sqlx::query("INSERT INTO studied_notes (user_id, note_id, studied_at) VALUES (?, ?, ?)")
        .bind(user)
        .bind(note)
        .bind(REVIEWED_AT)
        .execute(pool)
        .await
        .unwrap();
}

async fn card_state(pool: &SqlitePool, user: i64, note: &str, template: &str) {
    sqlx::query(
        "INSERT INTO card_state
             (user_id, note_id, template_id, phase, stability, difficulty, due, last_review, reps, lapses)
         VALUES (?, ?, ?, 'review', 30.0, 5.0, ?, ?, 4, 1)",
    )
    .bind(user)
    .bind(note)
    .bind(template)
    .bind(DUE)
    .bind(REVIEWED_AT)
    .execute(pool)
    .await
    .unwrap();
}

async fn review_log(pool: &SqlitePool, user: i64, note: &str, template: &str) {
    sqlx::query(
        "INSERT INTO review_log
             (user_id, note_id, template_id, rating, phase_before, reviewed_at, duration_ms, answer)
         VALUES (?, ?, ?, 3, 'review', ?, 2300, 'saved answer')",
    )
    .bind(user)
    .bind(note)
    .bind(template)
    .bind(REVIEWED_AT)
    .execute(pool)
    .await
    .unwrap();
}

async fn all_progress(pool: &SqlitePool, user: i64, note: &str) {
    mark_studied(pool, user, note).await;
    for template in TEMPLATES {
        card_state(pool, user, note, template).await;
        review_log(pool, user, note, template).await;
    }
}

async fn fixture() -> SqlitePool {
    let pool = open_db("sqlite::memory:").await.unwrap();
    add_set(&pool, "old-set", false, &TARGET_NOTES).await;
    add_set(&pool, "other-set", true, &["old-shared", "other-only"]).await;
    add_set(&pool, "retired-set", false, &["retired-only"]).await;
    sqlx::raw_sql(
        "INSERT INTO users (id, name) VALUES (2, 'Another learner');
         INSERT INTO subscriptions (user_id, set_id) VALUES (2, 'old-set'), (2, 'other-set');
         UPDATE users SET new_per_day = 15, reviews_per_day = 37,
             desired_retention = 0.85, study_order = 'random' WHERE id = 1;
         UPDATE notes SET status = 'draft' WHERE id = 'old-draft';
         UPDATE cards SET enabled = 0 WHERE note_id = 'old-disabled';
         UPDATE sets SET retired = 1 WHERE id = 'retired-set';",
    )
    .execute(&pool)
    .await
    .unwrap();

    for note in [
        "old-all",
        "old-shared",
        "old-draft",
        "old-unavailable",
        "old-disabled",
    ] {
        all_progress(&pool, 1, note).await;
    }
    mark_studied(&pool, 1, "old-study").await;
    for template in TEMPLATES {
        card_state(&pool, 1, "old-card", template).await;
        review_log(&pool, 1, "old-history", template).await;
    }
    all_progress(&pool, 1, "other-only").await;
    all_progress(&pool, 1, "retired-only").await;
    for note in TARGET_NOTES.into_iter().chain(["other-only", "retired-only"]) {
        all_progress(&pool, 2, note).await;
    }
    for user in [1, 2] {
        for set in ["old-set", "other-set"] {
            sqlx::query(
                "INSERT INTO game_sessions
                     (user_id, set_id, pairing, pairs, flips, duration_ms, completed, played_at)
                 VALUES (?, ?, 'image:name', 3, 8, 12345, 1, ?)",
            )
            .bind(user)
            .bind(set)
            .bind(REVIEWED_AT)
            .execute(&pool)
            .await
            .unwrap();
        }
    }
    pool
}

async fn snapshot(pool: &SqlitePool, queries: &[(&str, &str)], user: Option<i64>) -> Value {
    let mut snapshot = serde_json::Map::new();
    for (name, sql) in queries {
        let mut query = sqlx::query_scalar::<_, String>(sql);
        if let Some(user) = user {
            query = query.bind(user);
        }
        let rows: Vec<Value> = query
            .fetch_all(pool)
            .await
            .unwrap()
            .iter()
            .map(|row| serde_json::from_str(row).unwrap())
            .collect();
        snapshot.insert((*name).into(), json!(rows));
    }
    Value::Object(snapshot)
}

async fn progress(pool: &SqlitePool, user: i64) -> Value {
    // Put the note ID first so expected survivors can be filtered by global note identity.
    snapshot(
        pool,
        &[
            (
                "studied",
                "SELECT json_array(note_id, studied_at) FROM studied_notes
                WHERE user_id = ? ORDER BY note_id",
            ),
            (
                "states",
                "SELECT json_array(note_id, template_id, phase, stability, difficulty,
                due, last_review, reps, lapses) FROM card_state
                WHERE user_id = ? ORDER BY note_id, template_id",
            ),
            (
                "reviews",
                "SELECT json_array(note_id, id, template_id, rating, phase_before,
                reviewed_at, duration_ms, answer) FROM review_log WHERE user_id = ? ORDER BY id",
            ),
        ],
        Some(user),
    )
    .await
}

async fn shared_data(pool: &SqlitePool) -> Value {
    snapshot(
        pool,
        &[
            (
                "notes",
                "SELECT json_array(id, note_type_id, fields, status, created_at, updated_at)
                FROM notes ORDER BY id",
            ),
            (
                "types",
                "SELECT json_array(id, definition) FROM note_types ORDER BY id",
            ),
            (
                "cards",
                "SELECT json_array(note_id, template_id, position, enabled, content_available)
                FROM cards ORDER BY note_id, template_id",
            ),
            (
                "sets",
                "SELECT json_array(id, name, is_default, tags, retired) FROM sets ORDER BY id",
            ),
            (
                "memberships",
                "SELECT json_array(set_id, note_id, position) FROM set_notes
                ORDER BY set_id, note_id",
            ),
            (
                "settings",
                "SELECT json_array(id, name, new_per_day, reviews_per_day, desired_retention,
                study_order, created_at) FROM users ORDER BY id",
            ),
            (
                "games",
                "SELECT json_array(id, user_id, set_id, pairing, pairs, flips, duration_ms,
                completed, played_at) FROM game_sessions ORDER BY id",
            ),
        ],
        None,
    )
    .await
}

async fn subscriptions(pool: &SqlitePool) -> BTreeSet<(i64, String)> {
    sqlx::query_as("SELECT user_id, set_id FROM subscriptions")
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .collect()
}

async fn everything(pool: &SqlitePool) -> Value {
    json!({
        "me": progress(pool, 1).await,
        "other_user": progress(pool, 2).await,
        "shared": shared_data(pool).await,
        "subscriptions": subscriptions(pool).await
    })
}

fn without_target_notes(mut progress: Value) -> Value {
    for rows in progress.as_object_mut().unwrap().values_mut() {
        rows.as_array_mut()
            .unwrap()
            .retain(|row| !TARGET_NOTES.contains(&row[0].as_str().unwrap()));
    }
    progress
}

#[tokio::test]
async fn set_summaries_count_distinct_concepts_with_any_current_user_progress() {
    let pool = fixture().await;
    let old = set_summary(&pool, "old-set").await;
    assert_eq!(old["subscribed"], false);
    assert_eq!(old["note_count"], 9);
    // Studied-only, state-only and log-only notes count; multiple cards and logs don't multiply them.
    assert_eq!(old["studied_count"], 8);
    assert_eq!(set_summary(&pool, "other-set").await["studied_count"], 2);

    let (status, created) = call(
        &pool,
        "POST",
        "/api/sets",
        Some(json!({ "id": "fresh-set", "name": "Fresh set" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["studied_count"], 0);
}

#[tokio::test]
async fn enabling_with_resume_or_a_legacy_request_and_disabling_preserve_all_progress() {
    for body in [
        json!({ "subscribed": true }),
        json!({ "subscribed": true, "progress": "resume" }),
    ] {
        let pool = fixture().await;
        let me = progress(&pool, 1).await;
        let other_user = progress(&pool, 2).await;
        let shared = shared_data(&pool).await;
        for _ in 0..2 {
            let (status, value) = subscription(&pool, "old-set", body.clone()).await;
            assert_eq!(status, StatusCode::NO_CONTENT, "{value}");
            assert_eq!(set_summary(&pool, "old-set").await["subscribed"], true);
            assert_eq!(progress(&pool, 1).await, me);
            assert_eq!(progress(&pool, 2).await, other_user);
            assert_eq!(shared_data(&pool).await, shared);
        }
        let (status, value) = subscription(&pool, "old-set", json!({ "subscribed": false })).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "{value}");
        assert_eq!(set_summary(&pool, "old-set").await["subscribed"], false);
        assert_eq!(progress(&pool, 1).await, me);
        assert_eq!(progress(&pool, 2).await, other_user);
        assert_eq!(shared_data(&pool).await, shared);
    }
}

#[tokio::test]
async fn restarting_clears_only_this_users_set_notes_and_requires_study_again() {
    let pool = fixture().await;
    let expected_survivors = without_target_notes(progress(&pool, 1).await);
    let other_user = progress(&pool, 2).await;
    let shared = shared_data(&pool).await;
    let mut expected_subscriptions = subscriptions(&pool).await;
    expected_subscriptions.insert((1, "old-set".into()));

    let (status, value) = subscription(
        &pool,
        "old-set",
        json!({ "subscribed": true, "progress": "restart" }),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{value}");
    assert_eq!(progress(&pool, 1).await, expected_survivors);
    assert_eq!(progress(&pool, 2).await, other_user);
    assert_eq!(shared_data(&pool).await, shared);
    assert_eq!(subscriptions(&pool).await, expected_subscriptions);
    assert_eq!(set_summary(&pool, "old-set").await["studied_count"], 0);
    // Shared concepts have one global history, so the other set loses only that shared concept's progress.
    assert_eq!(set_summary(&pool, "other-set").await["studied_count"], 1);

    let batch = get(&pool, "/api/study/batch?size=100").await;
    let ids: BTreeSet<_> = batch["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|note| note["id"].as_str().unwrap())
        .collect();
    assert_eq!(batch["remaining"], 6);
    assert_eq!(ids, ELIGIBLE_NOTES.into_iter().collect());
    let round = get(&pool, "/api/review/round?size=100").await;
    assert_eq!(round["counts"], json!({ "learning": 0, "review": 0, "new": 0 }));
    assert_eq!(round["cards"], json!([]));

    let (status, value) = call(
        &pool,
        "POST",
        "/api/study/studied",
        Some(json!({ "note_ids": ["old-all"] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["added"], 1);
    assert_eq!(get(&pool, "/api/review/round").await["counts"]["new"], 2);
}

#[tokio::test]
async fn restarting_an_enabled_set_is_rejected_without_resetting_or_disabling_it() {
    let pool = fixture().await;
    let (status, value) = subscription(&pool, "old-set", json!({ "subscribed": true })).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{value}");
    let before = everything(&pool).await;
    let (status, value) = subscription(
        &pool,
        "old-set",
        json!({ "subscribed": true, "progress": "restart" }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{value}");
    assert_eq!(everything(&pool).await, before);
}

#[tokio::test]
async fn restart_cannot_be_combined_with_disabling_or_an_unknown_progress_choice() {
    let pool = fixture().await;
    let before = everything(&pool).await;
    for body in [
        json!({ "subscribed": false, "progress": "restart" }),
        json!({ "subscribed": true, "progress": "forget" }),
    ] {
        let (status, value) = subscription(&pool, "old-set", body).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{value}");
        assert_eq!(everything(&pool).await, before);
    }
}

#[tokio::test]
async fn missing_and_retired_sets_cannot_be_enabled_or_have_their_progress_reset() {
    let pool = fixture().await;
    let before = everything(&pool).await;
    for set_id in ["missing-set", "retired-set"] {
        let (status, value) = subscription(
            &pool,
            set_id,
            json!({ "subscribed": true, "progress": "restart" }),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{value}");
        assert_eq!(everything(&pool).await, before);
    }
}

#[tokio::test]
async fn a_failed_restart_rolls_back_progress_deletion_and_subscription_changes() {
    let pool = fixture().await;
    // Fail after the subscription insert and card-state deletion so both must be rolled back.
    sqlx::raw_sql(
        "CREATE TRIGGER fail_target_review_delete BEFORE DELETE ON review_log
         WHEN OLD.user_id = 1 AND OLD.note_id = 'old-all'
         BEGIN SELECT RAISE(ABORT, 'injected review deletion failure'); END;",
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = everything(&pool).await;
    let (status, value) = subscription(
        &pool,
        "old-set",
        json!({ "subscribed": true, "progress": "restart" }),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{value}");
    assert_eq!(everything(&pool).await, before);
}
