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

async fn call(pool: &SqlitePool, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
    // Recreate the router so settings must survive beyond a single AppState instance.
    let mut state = AppState::new(
        pool.clone(),
        std::env::temp_dir().join("srs-study-order-test-media"),
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
        // Axum's JSON deserialization errors have plain-text bodies.
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!(String::from_utf8_lossy(&bytes)))
    };
    (status, value)
}

async fn get(pool: &SqlitePool, path: &str) -> Value {
    let (status, value) = call(pool, "GET", path, None).await;
    assert_eq!(status, StatusCode::OK, "GET {path}: {value}");
    value
}

async fn choose_order(pool: &SqlitePool, order: &str) {
    let mut settings = get(pool, "/api/settings").await;
    settings["study_order"] = json!(order);
    let (status, value) = call(pool, "PUT", "/api/settings", Some(settings)).await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["study_order"], order);
}

fn note(id: &str, available: bool) -> Value {
    let mut note = json!({ "id": id, "name": id });
    if available {
        note["image"] = json!(format!("/study/fixture/{id}.svg"));
    }
    note
}

async fn add_set(pool: &SqlitePool, id: &str, subscribed: bool, notes: Vec<Value>) {
    let file: SetFile = serde_json::from_value(json!({
        "set": { "id": id, "name": id, "default": subscribed },
        "note_type": {
            "id": "StudyOrderFixture",
            "fields": [
                { "name": "name", "type": "text" },
                { "name": "image", "type": "image", "optional": true }
            ],
            "templates": [{
                "id": "image-to-name", "prompt": ["image"], "answer": ["name"], "mode": "flip"
            }]
        },
        "notes": notes
    }))
    .unwrap();
    file.validate().unwrap();
    seed::import_set(pool, &file).await.unwrap();
}

async fn add_study_sets(pool: &SqlitePool, mut first: Vec<Value>, second: Vec<Value>) {
    let shared_id = first[0]["id"].as_str().unwrap().to_string();
    first.extend([
        note("excluded-studied", true),
        note("excluded-draft", true),
        note("excluded-disabled", true),
        note("excluded-unavailable", false),
    ]);
    add_set(pool, "a-set", true, first).await;
    add_set(pool, "z-set", true, second).await;
    add_set(pool, "0-retired", true, vec![note("excluded-retired", true)]).await;
    add_set(
        pool,
        "0-not-selected",
        false,
        vec![note("excluded-not-selected", true)],
    )
    .await;
    sqlx::raw_sql(
        "UPDATE notes SET status = 'draft' WHERE id = 'excluded-draft';
         UPDATE cards SET enabled = 0 WHERE note_id = 'excluded-disabled';
         UPDATE sets SET retired = 1 WHERE id = '0-retired';",
    )
    .execute(pool)
    .await
    .unwrap();
    // A concept in two selected sets must still appear once in the global study pool.
    sqlx::query("INSERT INTO set_notes (set_id, note_id, position) VALUES ('z-set', ?, 99)")
        .bind(shared_id)
        .execute(pool)
        .await
        .unwrap();
    let (status, value) = call(
        pool,
        "POST",
        "/api/study/studied",
        Some(json!({ "note_ids": ["excluded-studied"] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["added"], 1);
}

fn ids(batch: &Value) -> Vec<&str> {
    batch["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|note| note["id"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn study_order_defaults_to_set_order_for_existing_users() {
    let pool = open_db("sqlite::memory:").await.unwrap();
    let settings = get(&pool, "/api/settings").await;
    assert_eq!(settings["study_order"], "set_order");
}

#[tokio::test]
async fn study_order_is_persisted_with_other_settings() {
    let pool = open_db("sqlite::memory:").await.unwrap();
    let (status, value) = call(
        &pool,
        "PUT",
        "/api/settings",
        Some(json!({
            "new_per_day": 23,
            "reviews_per_day": 50,
            "desired_retention": 0.85,
            "study_order": "random"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{value}");
    let saved = get(&pool, "/api/settings").await;
    assert_eq!(saved["study_order"], "random");
    assert_eq!(saved["new_per_day"], 23);
    assert_eq!(saved["reviews_per_day"], 50);
    assert_eq!(saved["desired_retention"], json!(0.85));

    choose_order(&pool, "set_order").await;
    assert_eq!(get(&pool, "/api/settings").await["study_order"], "set_order");
}

#[tokio::test]
async fn legacy_settings_requests_fall_back_to_set_order_when_the_field_is_omitted() {
    let pool = open_db("sqlite::memory:").await.unwrap();
    choose_order(&pool, "random").await;
    let (status, value) = call(
        &pool,
        "PUT",
        "/api/settings",
        Some(json!({ "new_per_day": 12, "reviews_per_day": null, "desired_retention": 0.9 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["study_order"], "set_order");
    assert_eq!(get(&pool, "/api/settings").await["study_order"], "set_order");
}

#[tokio::test]
async fn invalid_study_orders_are_rejected_without_changing_settings() {
    let pool = open_db("sqlite::memory:").await.unwrap();
    choose_order(&pool, "random").await;
    // Axum reports unknown enum variants as data errors, but null as a JSON parse error.
    for (invalid, expected_status) in [
        (json!("shuffle"), StatusCode::UNPROCESSABLE_ENTITY),
        (Value::Null, StatusCode::BAD_REQUEST),
    ] {
        let mut settings = get(&pool, "/api/settings").await;
        settings["study_order"] = invalid;
        let (status, value) = call(&pool, "PUT", "/api/settings", Some(settings)).await;
        assert_eq!(status, expected_status, "{value}");
        assert_eq!(get(&pool, "/api/settings").await["study_order"], "random");
    }
}

#[tokio::test]
async fn set_order_batches_follow_set_and_note_positions_and_exclude_unstudyable_notes() {
    let pool = open_db("sqlite::memory:").await.unwrap();
    add_study_sets(
        &pool,
        vec![note("visible-z-first", true), note("visible-a-second", true)],
        vec![note("visible-y-third", true), note("visible-b-fourth", true)],
    )
    .await;
    let expected = [
        "visible-z-first",
        "visible-a-second",
        "visible-y-third",
        "visible-b-fourth",
    ];
    for _ in 0..2 {
        let batch = get(&pool, "/api/study/batch?size=100").await;
        assert_eq!(batch["remaining"], 4);
        assert_eq!(ids(&batch), expected);
    }
    let batch = get(&pool, "/api/study/batch?size=3").await;
    assert_eq!(batch["remaining"], 4);
    assert_eq!(ids(&batch), expected[..3]);
}

#[tokio::test]
async fn random_batches_keep_the_entire_eligible_pool_unique_and_leave_progress_until_done() {
    let pool = open_db("sqlite::memory:").await.unwrap();
    let first: Vec<_> = (0..50).map(|i| note(&format!("a-note-{i:02}"), true)).collect();
    let second: Vec<_> = (0..50).map(|i| note(&format!("z-note-{i:02}"), true)).collect();
    let expected: BTreeSet<_> = first
        .iter()
        .chain(&second)
        .map(|note| note["id"].as_str().unwrap().to_string())
        .collect();
    add_study_sets(&pool, first, second).await;
    choose_order(&pool, "random").await;

    // Check pool integrity without assuming a particular random permutation.
    let full = get(&pool, "/api/study/batch?size=100").await;
    let actual: BTreeSet<_> = ids(&full).into_iter().map(str::to_string).collect();
    assert_eq!(full["remaining"], 100);
    assert_eq!(ids(&full).len(), 100);
    assert_eq!(actual, expected);

    let small = get(&pool, "/api/study/batch?size=7").await;
    let chosen: BTreeSet<_> = ids(&small).into_iter().map(str::to_string).collect();
    assert_eq!(
        small["remaining"], 100,
        "fetching a batch does not mark notes studied"
    );
    assert_eq!(ids(&small).len(), 7);
    assert_eq!(chosen.len(), 7);
    assert!(chosen.is_subset(&expected));

    let (status, value) = call(
        &pool,
        "POST",
        "/api/study/studied",
        Some(json!({ "note_ids": chosen })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["added"], 7);
    let remaining = get(&pool, "/api/study/batch?size=100").await;
    let remaining_ids: BTreeSet<_> = ids(&remaining).into_iter().map(str::to_string).collect();
    assert_eq!(remaining["remaining"], 93);
    assert_eq!(ids(&remaining).len(), 93);
    assert_eq!(remaining_ids, expected.difference(&chosen).cloned().collect());
}
