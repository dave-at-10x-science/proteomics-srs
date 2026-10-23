//! End-to-end API tests against an in-memory database seeded with the real amino acid set.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{DateTime, Duration, TimeZone, Utc};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use srs_server::claude::Claude;
use srs_server::{AppState, app, open_db, seed};
use tower::ServiceExt;

struct TestApp {
    router: Router,
    pool: sqlx::SqlitePool,
    now: Arc<Mutex<DateTime<Utc>>>,
}

impl TestApp {
    async fn new() -> Self {
        Self::build(None).await
    }

    async fn with_claude(base_url: String) -> Self {
        Self::build(Some(Claude::new("test-key".into(), Some(base_url), None))).await
    }

    async fn build(claude: Option<Claude>) -> Self {
        let pool = open_db("sqlite::memory:").await.unwrap();
        let seed_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../seed");
        assert_eq!(seed::import_dir(&pool, seed_dir.as_ref()).await.unwrap(), 2);
        let now = Arc::new(Mutex::new(Utc.with_ymd_and_hms(2026, 10, 23, 12, 0, 0).unwrap()));
        let clock_now = now.clone();
        let media_dir = std::env::temp_dir().join(format!("srs-media-{}", std::process::id()));
        let mut state = AppState::new(pool.clone(), media_dir);
        state.clock = Arc::new(move || *clock_now.lock().unwrap());
        state.claude = claude.map(Arc::new);
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
    // 20 amino acids x 6 templates, 24 terms x 2 templates.
    assert_eq!(count("SELECT COUNT(*) FROM notes").await.unwrap(), 44);
    assert_eq!(count("SELECT COUNT(*) FROM cards").await.unwrap(), 168);
    assert_eq!(count("SELECT COUNT(*) FROM set_notes").await.unwrap(), 44);

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

#[tokio::test]
async fn images_upload_and_are_served_with_a_strict_csp() {
    let t = TestApp::new().await;
    let png = b"\x89PNG\r\n\x1a\nnot really a png but fine for storage";
    let req = Request::builder()
        .method("POST")
        .uri("/api/media")
        .header("content-type", "image/png")
        .body(Body::from(&png[..]))
        .unwrap();
    let res = t.router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body: Value = serde_json::from_slice(&res.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let url = body["url"].as_str().unwrap();
    assert!(url.starts_with("/media/") && url.ends_with(".png"), "{url}");

    let res = t
        .router
        .clone()
        .oneshot(Request::get(url).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        res.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("default-src 'none'")
    );
    assert_eq!(&res.into_body().collect().await.unwrap().to_bytes()[..], &png[..]);

    let req = Request::builder()
        .method("POST")
        .uri("/api/media")
        .header("content-type", "application/pdf")
        .body(Body::from("x"))
        .unwrap();
    assert_eq!(
        t.router.clone().oneshot(req).await.unwrap().status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn vocabulary_set_is_imported_but_not_subscribed() {
    let t = TestApp::new().await;
    let sets = t.get("/api/sets").await;
    let vocab = sets
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "ms-vocabulary")
        .unwrap();
    assert_eq!(vocab["subscribed"], false);
    assert_eq!(vocab["note_count"], 24);
    assert_eq!(vocab["card_count"], 48);
}

#[tokio::test]
async fn memory_game_uses_only_introduced_notes_and_never_touches_the_schedule() {
    let t = TestApp::new().await;
    // Nothing studied yet: no game options, and boards are refused.
    assert_eq!(t.get("/api/game/options").await, json!([]));
    let (status, _) = t
        .call(
            "GET",
            "/api/game/board?set=amino-acids-20&a=structure&b=name",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Study today's ten new cards.
    let mut card = t.get("/api/review/next").await["card"].clone();
    let mut studied = std::collections::HashSet::new();
    while !card.is_null() {
        studied.insert(card["note_id"].as_str().unwrap().to_string());
        card = t.rate(&card, "good").await["next"]["card"].clone();
    }
    let schedule_before: Vec<(String, String)> =
        sqlx::query_as("SELECT note_id || '/' || template_id, due FROM card_state ORDER BY 1")
            .fetch_all(&t.pool)
            .await
            .unwrap();

    let options = t.get("/api/game/options").await;
    assert_eq!(options[0]["set_id"], "amino-acids-20");
    assert_eq!(options[0]["introduced"], 10);

    let board = t
        .get("/api/game/board?set=amino-acids-20&a=structure&b=name&pairs=12")
        .await;
    let tiles = board["tiles"].as_array().unwrap();
    assert_eq!(tiles.len(), 20, "capped at the 10 introduced notes");
    for tile in tiles {
        assert!(studied.contains(tile["note_id"].as_str().unwrap()));
    }
    let note = &board["notes"][tiles[0]["note_id"].as_str().unwrap()];
    assert!(note["smiles"].is_string() && note["name"].is_string());
    assert!(note.get("pI").is_none(), "only the fields tiles need are sent");

    // Side-chain class repeats across amino acids, and values must be unique per side or matches
    // would be ambiguous. The first ten amino acids span only two classes, so that board is refused.
    let (status, err) = t
        .call(
            "GET",
            "/api/game/board?set=amino-acids-20&a=structure&b=side_chain_class",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(err["error"].as_str().unwrap().contains("found 2"), "{err}");

    let (status, best) = t
        .post(
            "/api/game/results",
            json!({ "set_id": "amino-acids-20", "a": "structure", "b": "name", "pairs": 8, "flips": 22, "duration_ms": 61000, "completed": true }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(best, json!({ "games": 1, "best_ms": 61000, "fewest_flips": 22 }));

    let schedule_after: Vec<(String, String)> =
        sqlx::query_as("SELECT note_id || '/' || template_id, due FROM card_state ORDER BY 1")
            .fetch_all(&t.pool)
            .await
            .unwrap();
    assert_eq!(schedule_before, schedule_after);
    let reviews: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM review_log")
        .fetch_one(&t.pool)
        .await
        .unwrap();
    assert_eq!(reviews, 10);
}

/// A stand-in for the Claude API that records each request body and answers with `reply`.
async fn mock_claude(status: u16, reply: Value) -> (String, Arc<Mutex<Vec<(Value, String)>>>) {
    use axum::http::HeaderMap;
    let seen: Arc<Mutex<Vec<(Value, String)>>> = Arc::default();
    let log = seen.clone();
    let router = Router::new().route(
        "/v1/messages",
        axum::routing::post(move |headers: HeaderMap, axum::Json(body): axum::Json<Value>| {
            let log = log.clone();
            let reply = reply.clone();
            async move {
                let beta = headers["anthropic-beta"].to_str().unwrap().to_string();
                log.lock().unwrap().push((body, beta));
                (StatusCode::from_u16(status).unwrap(), axum::Json(reply))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (format!("http://{addr}"), seen)
}

fn claude_reply(notes: Value) -> Value {
    json!({
        "type": "message",
        "role": "assistant",
        "stop_reason": "end_turn",
        "content": [{ "type": "text", "text": json!({ "notes": notes }).to_string() }],
    })
}

#[tokio::test]
async fn drafting_is_off_without_an_api_key() {
    let t = TestApp::new().await;
    assert_eq!(t.get("/api/draft/status").await["available"], false);
    let (status, v) = t
        .post(
            "/api/sets/ms-vocabulary/draft",
            json!({ "source": "ion mobility" }),
        )
        .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{v}");
    assert!(v["error"].as_str().unwrap().contains("ANTHROPIC_API_KEY"));
}

#[tokio::test]
async fn drafted_notes_wait_as_drafts_until_published() {
    let (url, seen) = mock_claude(
        200,
        claude_reply(json!([
            { "term": "Ion mobility", "definition": "Separating ions by shape as they drift through gas.",
              "technical": null, "example": null },
            // Already in the vocabulary set.
            { "term": "Precursor ion", "definition": "dup", "technical": null, "example": null },
            // Breaks the note type: missing the required definition.
            { "term": "Collision cross section", "definition": null, "technical": null, "example": null },
        ])),
    )
    .await;
    let t = TestApp::with_claude(url).await;
    assert_eq!(t.get("/api/draft/status").await["available"], true);

    let (status, v) = t
        .post(
            "/api/sets/ms-vocabulary/draft",
            json!({ "source": "ion mobility terms", "count": 3 }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{v}");
    assert_eq!(v["created"].as_array().unwrap().len(), 1, "{v}");
    assert_eq!(v["created"][0]["id"], "term-ion-mobility");
    assert_eq!(v["created"][0]["status"], "draft");
    let skipped = v["skipped"].as_array().unwrap();
    assert_eq!(skipped.len(), 2, "{v}");
    assert_eq!(skipped[0]["reason"], "already in your library");
    assert!(skipped[1]["reason"].as_str().unwrap().contains("definition"));

    // The request asked for structured output against the note type's fields.
    let (body, beta) = seen.lock().unwrap()[0].clone();
    assert_eq!(beta, "server-side-fallback-2026-07-01");
    assert_eq!(body["model"], "claude-opus-5-5");
    assert_eq!(body["fallbacks"], "default");
    assert!(body.get("temperature").is_none());
    assert_eq!(body["output_config"]["format"]["type"], "json_schema");
    let item = &body["output_config"]["format"]["schema"]["properties"]["notes"]["items"];
    assert_eq!(
        item["required"],
        json!(["term", "definition", "technical", "example"])
    );
    assert!(body["system"].as_str().unwrap().contains("Precursor ion"));
    assert!(
        body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("ion mobility terms")
    );

    // Drafts are never studied. Subscribe and check the queue holds no draft cards.
    let (status, _) = t
        .call(
            "PUT",
            "/api/sets/ms-vocabulary/subscription",
            Some(json!({ "subscribed": true })),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let draft_cards: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM card_state WHERE note_id = 'term-ion-mobility'")
            .fetch_one(&t.pool)
            .await
            .unwrap();
    assert_eq!(draft_cards, 0);
    let mut seen_ids = Vec::new();
    for _ in 0..40 {
        let next = t.get("/api/review/next").await;
        if next["card"].is_null() {
            break;
        }
        seen_ids.push(next["card"]["note_id"].as_str().unwrap().to_string());
        t.rate(&next["card"], "easy").await;
    }
    assert!(!seen_ids.iter().any(|id| id == "term-ion-mobility"));

    let (status, v) = t.post("/api/sets/ms-vocabulary/publish-drafts", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{v}");
    assert_eq!(v["published"], 1);
    assert_eq!(t.get("/api/notes/term-ion-mobility").await["status"], "published");
}

#[tokio::test]
async fn refusals_and_api_errors_are_reported_plainly() {
    let (url, _) = mock_claude(
        200,
        json!({ "type": "message", "stop_reason": "refusal", "content": [] }),
    )
    .await;
    let t = TestApp::with_claude(url).await;
    let (status, v) = t
        .post("/api/sets/ms-vocabulary/draft", json!({ "source": "anything" }))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{v}");
    assert!(v["error"].as_str().unwrap().contains("declined"));

    let (url, _) = mock_claude(
        529,
        json!({ "type": "error", "error": { "type": "overloaded_error", "message": "Overloaded" } }),
    )
    .await;
    let t = TestApp::with_claude(url).await;
    let (status, v) = t
        .post("/api/sets/ms-vocabulary/draft", json!({ "source": "anything" }))
        .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{v}");
    assert!(v["error"].as_str().unwrap().contains("Overloaded"));
}

#[tokio::test]
async fn client_routes_are_served_the_app_shell() {
    let pool = open_db("sqlite::memory:").await.unwrap();
    let web = std::env::temp_dir().join(format!("srs-web-{}", std::process::id()));
    std::fs::create_dir_all(&web).unwrap();
    std::fs::write(web.join("index.html"), "<html>shell</html>").unwrap();
    let router = app(AppState::new(pool, web.join("media")), Some(web));
    let res = router
        .oneshot(Request::get("/sets/ms-vocabulary").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"<html>shell</html>");
}
