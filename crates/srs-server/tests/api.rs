//! End-to-end API tests against an in-memory database seeded with the real amino acid set.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{DateTime, Duration, TimeZone, Utc};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use srs_server::{AppState, app, open_db, seed};
use tower::ServiceExt;

struct TestApp {
    router: Router,
    pool: sqlx::SqlitePool,
    now: Arc<Mutex<DateTime<Utc>>>,
}

impl TestApp {
    async fn new() -> Self {
        let pool = open_db("sqlite::memory:").await.unwrap();
        let seed_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../seed");
        assert_eq!(seed::import_dir(&pool, seed_dir.as_ref()).await.unwrap(), 1);
        let now = Arc::new(Mutex::new(Utc.with_ymd_and_hms(2026, 10, 23, 12, 0, 0).unwrap()));
        let clock_now = now.clone();
        let mut state = AppState::new(pool.clone());
        state.clock = Arc::new(move || *clock_now.lock().unwrap());
        Self {
            router: app(state, None),
            pool,
            now,
        }
    }

    fn advance(&self, by: Duration) {
        *self.now.lock().unwrap() += by;
    }

    async fn call(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json");
        let req = req
            .body(body.map_or(Body::empty(), |b| Body::from(b.to_string())))
            .unwrap();
        let res = self.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, value)
    }

    async fn get(&self, uri: &str) -> Value {
        let (status, v) = self.call("GET", uri, None).await;
        assert_eq!(status, StatusCode::OK, "GET {uri}: {v}");
        v
    }

    async fn post(&self, uri: &str, body: Value) -> (StatusCode, Value) {
        self.call("POST", uri, Some(body)).await
    }

    async fn rate(&self, card: &Value, rating: &str) -> Value {
        let (status, v) = self
            .post(
                "/api/review/answer",
                json!({ "note_id": card["note_id"], "template_id": card["template_id"], "rating": rating }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{v}");
        v
    }
}

#[tokio::test]
async fn default_set_is_imported_and_subscribed() {
    let t = TestApp::new().await;
    let sets = t.get("/api/sets").await;
    assert_eq!(sets[0]["id"], "amino-acids-20");
    assert_eq!(sets[0]["subscribed"], true);
    assert_eq!(sets[0]["note_count"], 20);
    assert_eq!(sets[0]["card_count"], 100);
}

#[tokio::test]
async fn seed_import_is_idempotent_and_keeps_app_edits() {
    let t = TestApp::new().await;
    let mut gly = t.get("/api/notes/aa-gly").await;
    gly["note"] = json!("Edited in the app");
    let fields: serde_json::Map<String, Value> = gly
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, _)| !["id", "note_type_id", "status"].contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let (status, v) = t
        .call("PUT", "/api/notes/aa-gly", Some(json!({ "fields": fields })))
        .await;
    assert_eq!(status, StatusCode::OK, "{v}");

    // Re-import into the same database, as happens on every server start.
    let seed_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../seed");
    seed::import_dir(&t.pool, seed_dir.as_ref()).await.unwrap();
    let count = |sql: &'static str| sqlx::query_scalar::<_, i64>(sql).fetch_one(&t.pool);
    assert_eq!(count("SELECT COUNT(*) FROM notes").await.unwrap(), 20);
    assert_eq!(count("SELECT COUNT(*) FROM cards").await.unwrap(), 120);
    assert_eq!(count("SELECT COUNT(*) FROM set_notes").await.unwrap(), 20);

    assert_eq!(t.get("/api/notes/aa-gly").await["note"], "Edited in the app");
}

#[tokio::test]
async fn daily_session_introduces_ten_new_cards_one_per_amino_acid() {
    let t = TestApp::new().await;
    let next = t.get("/api/review/next").await;
    assert_eq!(next["counts"], json!({ "learning": 0, "review": 0, "new": 10 }));

    let mut card = next["card"].clone();
    let mut seen = Vec::new();
    while !card.is_null() {
        assert_eq!(card["phase"], "new");
        seen.push(card["note_id"].as_str().unwrap().to_string());
        card = t.rate(&card, "good").await["next"]["card"].clone();
    }
    assert_eq!(seen.len(), 10);
    seen.sort();
    seen.dedup();
    assert_eq!(
        seen.len(),
        10,
        "siblings should be buried: one card per amino acid per day"
    );

    // "Good" on a new card schedules it about two days out. Then those ten are due for review,
    // plus ten more new ones.
    t.advance(Duration::days(1));
    assert_eq!(t.get("/api/review/next").await["counts"]["review"], 0);
    t.advance(Duration::days(2));
    let next = t.get("/api/review/next").await;
    assert_eq!(next["counts"]["review"], 10);
    assert_eq!(next["counts"]["new"], 10);
}

#[tokio::test]
async fn again_brings_the_card_back_in_the_same_session() {
    let t = TestApp::new().await;
    let card = t.get("/api/review/next").await["card"].clone();
    let res = t.rate(&card, "again").await;
    assert_eq!(res["state"]["phase"], "learning");

    // Work through the rest of the new cards.
    let mut next = res["next"]["card"].clone();
    while !next.is_null() && next["note_id"] != card["note_id"] {
        next = t.rate(&next, "good").await["next"]["card"].clone();
    }
    // The failed card is offered again (learn-ahead) before the session ends.
    assert_eq!(next["note_id"], card["note_id"]);
    assert_eq!(next["phase"], "learning");
}

#[tokio::test]
async fn cards_include_what_the_app_needs_to_render_them() {
    let t = TestApp::new().await;
    let card = t.get("/api/review/next").await["card"].clone();
    assert_eq!(card["template_id"], "structure-to-name");
    assert!(card["fields"]["smiles"].is_string());
    assert!(card["note_type"]["depiction_templates"]["amino-acid-backbone"].is_object());
    let i = &card["intervals"];
    assert_eq!(i["again"], 600);
    assert!(i["hard"].as_i64() < i["good"].as_i64() && i["good"].as_i64() < i["easy"].as_i64());
}

#[tokio::test]
async fn multiple_choice_has_four_distinct_options_including_the_answer() {
    let t = TestApp::new().await;
    // Introduce a card, then come back the next day when the sibling templates are available.
    let mut card = t.get("/api/review/next").await["card"].clone();
    for _ in 0..30 {
        if card["template"]["mode"] == "multiple_choice" {
            break;
        }
        let next = t.rate(&card, "easy").await["next"]["card"].clone();
        card = if next.is_null() {
            t.advance(Duration::days(1));
            t.get("/api/review/next").await["card"].clone()
        } else {
            next
        };
    }
    assert_eq!(card["template"]["mode"], "multiple_choice", "{card}");
    let choices: Vec<&str> = card["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert_eq!(choices.len(), 4);
    let mut unique = choices.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 4);
    let field = card["template"]["answer"][0].as_str().unwrap();
    assert!(choices.contains(&card["fields"][field].as_str().unwrap()));
}

#[tokio::test]
async fn typed_answers_are_checked() {
    let t = TestApp::new().await;
    let req =
        |input: &str| json!({ "note_id": "aa-ile", "template_id": "structure-to-name", "input": input });
    let (_, ok) = t.post("/api/review/check", req("ile")).await;
    assert_eq!(ok["correct"], true);
    assert_eq!(ok["expected"]["name"], "Isoleucine");
    let (_, bad) = t.post("/api/review/check", req("leucine")).await;
    assert_eq!(bad["correct"], false);
}

#[tokio::test]
async fn notes_can_be_added_to_a_new_set() {
    let t = TestApp::new().await;
    let (status, set) = t.post("/api/sets", json!({ "name": "Proteases" })).await;
    assert_eq!(status, StatusCode::CREATED, "{set}");
    assert_eq!(set["id"], "proteases");

    // Invalid: missing required fields, wrong type.
    let (status, err) = t
        .post(
            "/api/notes",
            json!({ "set_id": "proteases", "note_type_id": "AminoAcid", "fields": { "name": "X", "pI": "high" } }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(err["errors"].as_array().unwrap().len() > 3, "{err}");

    let mut fields = t.get("/api/notes/aa-gly").await;
    let fields = fields.as_object_mut().unwrap();
    for k in ["id", "note_type_id", "status", "note"] {
        fields.remove(k);
    }
    fields.insert("name".into(), json!("Selenocysteine"));
    let (status, note) = t
        .post(
            "/api/notes",
            json!({ "set_id": "proteases", "note_type_id": "AminoAcid", "fields": fields }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{note}");
    assert_eq!(note["id"], "aminoacid-selenocysteine");

    let sets = t.get("/api/sets").await;
    let proteases = sets
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "proteases")
        .unwrap();
    assert_eq!(proteases["note_count"], 1);
    assert_eq!(proteases["card_count"], 5);

    let (status, _) = t
        .call("DELETE", "/api/notes/aminoacid-selenocysteine", None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn unsubscribing_empties_the_queue() {
    let t = TestApp::new().await;
    let (status, _) = t
        .call(
            "PUT",
            "/api/sets/amino-acids-20/subscription",
            Some(json!({ "subscribed": false })),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let next = t.get("/api/review/next").await;
    assert!(next["card"].is_null());
}

#[tokio::test]
async fn settings_are_validated() {
    let t = TestApp::new().await;
    let (status, _) = t
        .call(
            "PUT",
            "/api/settings",
            Some(json!({ "new_per_day": 5, "reviews_per_day": null, "desired_retention": 0.5 })),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = t
        .call(
            "PUT",
            "/api/settings",
            Some(json!({ "new_per_day": 5, "reviews_per_day": null, "desired_retention": 0.85 })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(t.get("/api/review/next").await["counts"]["new"], 5);
}
