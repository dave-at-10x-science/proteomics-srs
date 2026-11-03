use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use srs_core::content::SetFile;
use srs_server::{AppState, app, open_db, seed, store};
use tower::ServiceExt;

const SEED_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../seed");
const REFRESH: &str = include_str!("../migrations/0006_visual_study_sets.sql");

#[test]
fn shared_note_types_have_the_same_definition_in_every_topic_file() {
    let mut kinds = std::collections::BTreeMap::new();
    for directory in std::fs::read_dir(SEED_DIR).unwrap() {
        let directory = directory.unwrap().path();
        if !directory.is_dir() {
            continue;
        }
        for file in std::fs::read_dir(directory).unwrap() {
            let path = file.unwrap().path();
            if !path.extension().is_some_and(|extension| extension == "yaml") {
                continue;
            }
            let content = SetFile::from_yaml(&std::fs::read_to_string(&path).unwrap()).unwrap();
            if let Some(previous) = kinds.insert(content.note_type.id.clone(), content.note_type.clone()) {
                assert_eq!(
                    previous,
                    content.note_type,
                    "conflicting shared schema in {}",
                    path.display()
                );
            }
        }
    }
}

async fn import(pool: &SqlitePool) {
    assert_eq!(seed::import_dir(pool, SEED_DIR.as_ref()).await.unwrap(), 6);
}

async fn call(pool: &SqlitePool, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut state = AppState::new(pool.clone(), std::env::temp_dir().join("srs-visual-test-media"));
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
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, value)
}

async fn legacy_database() -> SqlitePool {
    let options = "sqlite::memory:"
        .parse::<SqliteConnectOptions>()
        .unwrap()
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    for sql in [
        include_str!("../migrations/0001_initial.sql"),
        include_str!("../migrations/0002_game_sessions.sql"),
        include_str!("../migrations/0003_studied_notes.sql"),
        include_str!("../migrations/0004_uppercase_three_letter_codes.sql"),
        include_str!("../migrations/0005_refresh_monosaccharide_visuals.sql"),
        include_str!("../migrations/0007_curate_study_library.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&pool).await.unwrap();
    }
    for original in [
        include_str!("../src/seed_baselines/ptms.yaml"),
        include_str!("../src/seed_baselines/ms-vocabulary.yaml"),
        include_str!("../src/seed_baselines/glycans.yaml"),
    ] {
        let file = SetFile::from_yaml(original).unwrap();
        sqlx::query("INSERT INTO note_types (id, definition) VALUES (?, ?)")
            .bind(&file.note_type.id)
            .bind(serde_json::to_string(&file.note_type).unwrap())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO sets (id, name) VALUES (?, ?)")
            .bind(&file.set.id)
            .bind(&file.set.name)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO subscriptions (user_id, set_id) VALUES (1, ?)")
            .bind(&file.set.id)
            .execute(&pool)
            .await
            .unwrap();
        for (position, note) in file.notes.iter().enumerate() {
            sqlx::query("INSERT INTO notes (id, note_type_id, fields, created_at, updated_at) VALUES (?, ?, ?, 'old-created', 'old-updated')")
                .bind(&note.id).bind(&file.note_type.id).bind(serde_json::to_string(&note.fields).unwrap())
                .execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO set_notes (set_id, note_id, position) VALUES (?, ?, ?)")
                .bind(&file.set.id)
                .bind(&note.id)
                .bind(position as i64)
                .execute(&pool)
                .await
                .unwrap();
            for (position, template) in file.note_type.templates.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO cards (note_id, template_id, position, enabled) VALUES (?, ?, ?, 1)",
                )
                .bind(&note.id)
                .bind(&template.id)
                .bind(position as i64)
                .execute(&pool)
                .await
                .unwrap();
            }
        }
    }
    pool
}

async fn add_history(pool: &SqlitePool, note: &str, template: &str) {
    sqlx::query("INSERT INTO card_state (user_id, note_id, template_id, phase, stability, difficulty, due, reps, lapses) VALUES (1, ?, ?, 'review', 8, 3, '2026-11-01T12:00:00Z', 5, 1)")
        .bind(note).bind(template).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO review_log (user_id, note_id, template_id, rating, phase_before, reviewed_at) VALUES (1, ?, ?, 3, 'review', '2026-10-01T12:00:00Z')")
        .bind(note).bind(template).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO studied_notes (user_id, note_id, studied_at) VALUES (1, ?, '2026-10-01T12:00:00Z') ON CONFLICT DO NOTHING")
        .bind(note).execute(pool).await.unwrap();
}

// Construct the previously shipped library without running today's refresh logic.
async fn insert_previous_set(pool: &SqlitePool, file: &SetFile) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO note_types (id, definition) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET definition = excluded.definition")
        .bind(&file.note_type.id).bind(serde_json::to_string(&file.note_type).unwrap())
        .execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO sets (id, name) VALUES (?, ?)")
        .bind(&file.set.id)
        .bind(&file.set.name)
        .execute(&mut *tx)
        .await
        .unwrap();
    let now = Utc.with_ymd_and_hms(2026, 10, 20, 12, 0, 0).unwrap();
    for note in &file.notes {
        store::upsert_note(&mut tx, &file.note_type, note, "published", now)
            .await
            .unwrap();
        sqlx::query("UPDATE notes SET created_at = '2026-10-20T12:00:00Z' WHERE id = ?")
            .bind(&note.id)
            .execute(&mut *tx)
            .await
            .unwrap();
        store::add_to_set(&mut tx, &file.set.id, &note.id).await.unwrap();
        sqlx::query("INSERT INTO seed_content_refreshes VALUES ('visual-study-sets-2026-11-03', ?)")
            .bind(&note.id)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn curation_upgrades_the_previous_visual_library_without_reviving_opt_outs_or_losing_authored_notes() {
    let pool = open_db("sqlite::memory:").await.unwrap();
    for source in [
        include_str!("../src/seed_baselines/glycosylation-reference-v1.yaml"),
        include_str!("../src/seed_baselines/glycan-motifs-v1.yaml"),
        include_str!("../src/seed_baselines/fragmentation-v1.yaml"),
    ] {
        insert_previous_set(&pool, &SetFile::from_yaml(source).unwrap()).await;
    }
    let combined = SetFile::from_yaml(include_str!(
        "../../../seed/mass-spec-concepts/mass-spec-concepts.yaml"
    ))
    .unwrap();
    for (id, range) in [
        ("ms-vocabulary", 0..4),
        ("ms-ionization", 4..8),
        ("ms-analyzers", 8..11),
        ("ms-acquisition", 11..16),
        ("ms-workflow", 16..22),
    ] {
        let mut file = combined.clone();
        file.set.id = id.into();
        file.notes = combined.notes[range].to_vec();
        insert_previous_set(&pool, &file).await;
    }
    sqlx::raw_sql(
        "INSERT INTO seed_content_refreshes VALUES ('visual-study-topics-2026-11-03', 'sets');
         INSERT INTO subscriptions (user_id, set_id) VALUES (1, 'ms-ionization'), (1, 'glycans');
         UPDATE notes SET fields = json_set(fields, '$.definition', 'My own ion explanation', '$.image', '/media/my-b.svg')
           WHERE id = 'term-by-ions';
         UPDATE notes SET fields = json_set(fields, '$.detail', 'My own glycan context') WHERE id = 'gly-core-fucose';
         INSERT INTO notes (id, note_type_id, fields) VALUES
           ('my-ms-note', 'Term', '{\"term\":\"My concept\",\"definition\":\"My explanation\",\"image\":\"/media/my-ms.svg\"}'),
           ('my-glycan-note', 'GlycoConcept', '{\"term\":\"My glycan\",\"kind\":\"Custom\",\"definition\":\"Keep this\"}');
         INSERT INTO set_notes VALUES ('ms-ionization', 'my-ms-note', 99), ('glycans', 'my-glycan-note', 99);
         INSERT INTO cards (note_id, template_id, position, enabled) VALUES ('my-glycan-note', 'term-to-definition', 0, 1);
         INSERT INTO game_sessions (user_id, set_id, pairing, pairs, flips, duration_ms, completed, played_at)
           VALUES (1, 'ms-ionization', 'image:term', 4, 10, 15000, 1, '2026-10-20T12:00:00Z');",
    ).execute(&pool).await.unwrap();
    for (note, template) in [
        ("term-by-ions", "image-to-term"),
        ("gly-n-core", "term-to-definition"),
        ("my-glycan-note", "term-to-definition"),
    ] {
        add_history(&pool, note, template).await;
    }
    import(&pool).await;
    let (_, sets) = call(&pool, "GET", "/api/sets", None).await;
    let sets = sets.as_array().unwrap();
    assert!(sets.iter().all(|set| {
        ![
            "glycans",
            "ms-vocabulary",
            "ms-ionization",
            "ms-analyzers",
            "ms-acquisition",
            "ms-workflow",
        ]
        .contains(&set["id"].as_str().unwrap())
    }));
    for (id, subscribed) in [
        ("mass-spec-concepts", true),
        ("glycan-motifs", false),
        ("ms-fragmentation", false),
        ("glycosylation-authored", true),
    ] {
        assert_eq!(
            sets.iter().find(|set| set["id"] == id).unwrap()["subscribed"],
            subscribed
        );
    }
    let b = store::note(&pool, "term-by-ions").await.unwrap();
    assert_eq!(b.note.text("definition").unwrap(), "My own ion explanation");
    assert_eq!(b.note.text("image").unwrap(), "/media/my-b.svg");
    assert_eq!(
        store::note(&pool, "gly-n-core").await.unwrap().note_type_id,
        "GlycanMotif"
    );
    assert_eq!(
        store::note(&pool, "gly-core-fucose")
            .await
            .unwrap()
            .note
            .text("detail")
            .unwrap(),
        "My own glycan context"
    );
    assert_eq!(
        store::note(&pool, "gly-tn")
            .await
            .unwrap()
            .note
            .text("family")
            .unwrap(),
        "O-GalNAc"
    );
    assert_eq!(
        store::set_of_note(&pool, "my-ms-note").await.unwrap().unwrap(),
        "mass-spec-concepts"
    );
    assert_eq!(
        store::set_of_note(&pool, "my-glycan-note")
            .await
            .unwrap()
            .unwrap(),
        "glycosylation-authored"
    );
    assert!(store::set_of_note(&pool, "gly-endo-h").await.unwrap().is_none());
    for table in ["card_state", "review_log", "studied_notes"] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table}"))
                .fetch_one(&pool)
                .await
                .unwrap(),
            3
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM game_sessions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT created_at FROM notes WHERE id = 'gly-n-core'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        "2026-10-20T12:00:00Z"
    );
    // Stale authoring screens cannot save new content into a hidden collection.
    let (status, _) = call(&pool, "POST", "/api/notes", Some(json!({"set_id":"ms-ionization","note_type_id":"Term","id":"lost-note","fields":{"term":"Lost","definition":"Must reject"}}))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        call(&pool, "POST", "/api/sets/glycans/publish-drafts", None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            &pool,
            "GET",
            "/api/game/board?set=ms-ionization&a=image&b=term",
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    sqlx::query("DELETE FROM subscriptions WHERE user_id = 1 AND set_id = 'mass-spec-concepts'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE notes SET fields = json_remove(fields, '$.image') WHERE id = 'gly-n-core'")
        .execute(&pool)
        .await
        .unwrap();
    import(&pool).await;
    assert!(
        store::note(&pool, "gly-n-core")
            .await
            .unwrap()
            .note
            .get("image")
            .is_none()
    );
    let (_, sets) = call(&pool, "GET", "/api/sets", None).await;
    assert_eq!(
        sets.as_array()
            .unwrap()
            .iter()
            .find(|set| set["id"] == "mass-spec-concepts")
            .unwrap()["subscribed"],
        false
    );
    let candidates = store::candidates(&pool, 1, Utc.with_ymd_and_hms(2026, 11, 3, 12, 0, 0).unwrap())
        .await
        .unwrap();
    assert!(
        candidates
            .iter()
            .all(|card| ["amino-acids-20", "glycosylation-authored"].contains(&card.set_id.as_str()))
    );
    assert!(candidates.iter().any(|card| card.id.note_id == "my-glycan-note"));
}

#[tokio::test]
async fn all_visual_questions_have_bundled_pictures_and_usable_choices() {
    let pool = open_db("sqlite::memory:").await.unwrap();
    import(&pool).await;
    let sets = ["ptms", "glycan-motifs", "mass-spec-concepts", "ms-fragmentation"];
    let mut checked = 0;
    let mut shared_term = None;
    for set_id in sets {
        let notes = store::set_notes(&pool, set_id).await.unwrap();
        assert!(!notes.is_empty());
        for stored in notes {
            let kind = store::note_type(&pool, &stored.note_type_id).await.unwrap();
            if kind.id == "Term" {
                if let Some(expected) = &shared_term {
                    assert_eq!(expected, &kind);
                } else {
                    shared_term = Some(kind.clone());
                }
            }
            let (field, template) = if kind.id == "PTM" {
                ("structure", "name-to-structure")
            } else {
                ("image", "term-to-image")
            };
            let picture = stored.note.text(field).unwrap();
            assert!(picture.starts_with("/study/"));
            let file = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/public"))
                .join(picture.trim_start_matches('/'));
            assert!(file.is_file(), "missing picture {picture}");
            let uri = format!(
                "/api/review/card?note_id={}&template_id={template}",
                stored.note.id
            );
            let (status, card) = call(&pool, "GET", &uri, None).await;
            assert_eq!(status, StatusCode::OK, "{uri}: {card}");
            let choices = card["choices"].as_array().unwrap();
            assert_eq!(
                choices.len(),
                4.min(store::set_notes(&pool, set_id).await.unwrap().len())
            );
            assert!(choices.contains(&json!(picture)));
            assert_eq!(
                choices.iter().collect::<std::collections::HashSet<_>>().len(),
                choices.len()
            );
            let (_, result) = call(
                &pool,
                "POST",
                "/api/review/check",
                Some(json!({"note_id":stored.note.id,"template_id":template,"input":picture})),
            )
            .await;
            assert_eq!(result["correct"], true);
            checked += 1;
        }
    }
    assert_eq!(checked, 59);
}

#[tokio::test]
async fn refresh_preserves_authored_fields_history_and_topic_choices() {
    let pool = legacy_database().await;
    for (note, template) in [
        ("ptm-acetyl", "name-to-mass-shift"),
        ("ptm-phospho", "structure-to-name"),
        ("term-esi", "term-to-definition"),
        ("gly-core2", "definition-to-term"),
    ] {
        add_history(&pool, note, template).await;
    }
    sqlx::query("UPDATE notes SET fields = json_set(fields, '$.definition', 'My own definition', '$.image', '/media/my-esi.svg') WHERE id = 'term-esi'")
        .execute(&pool).await.unwrap();
    sqlx::query(
        "UPDATE notes SET fields = json_set(fields, '$.detail', 'My own explanation') WHERE id = 'gly-core2'",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(REFRESH).execute(&pool).await.unwrap();
    import(&pool).await;
    let esi = store::note(&pool, "term-esi").await.unwrap();
    assert_eq!(esi.note.text("definition").unwrap(), "My own definition");
    assert_eq!(esi.note.text("image").unwrap(), "/media/my-esi.svg");
    let core2 = store::note(&pool, "gly-core2").await.unwrap();
    assert_eq!(core2.note_type_id, "GlycanMotif");
    assert_eq!(core2.note.text("detail").unwrap(), "My own explanation");
    assert!(core2.note.text("image").unwrap().ends_with("core2.svg"));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM card_state")
            .fetch_one(&pool)
            .await
            .unwrap(),
        4
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM review_log")
            .fetch_one(&pool)
            .await
            .unwrap(),
        4
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT enabled FROM cards WHERE note_id = 'ptm-phospho' AND template_id = 'structure-to-name'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT enabled FROM cards WHERE note_id = 'ptm-acetyl' AND template_id = 'name-to-mass-shift'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    let metadata: (String, String) =
        sqlx::query_as("SELECT created_at, updated_at FROM notes WHERE id = 'gly-core2'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(metadata, ("old-created".into(), "old-updated".into()));
    assert!(
        store::set_notes(&pool, "glycan-motifs")
            .await
            .unwrap()
            .iter()
            .any(|n| n.note.id == "gly-core2")
    );
    assert!(
        store::set_notes(&pool, "mass-spec-concepts")
            .await
            .unwrap()
            .iter()
            .any(|n| n.note.id == "term-esi")
    );
    let subscriptions: Vec<String> = sqlx::query_scalar("SELECT set_id FROM subscriptions WHERE user_id = 1")
        .fetch_all(&pool)
        .await
        .unwrap();
    for set in ["mass-spec-concepts", "ms-fragmentation", "glycan-motifs"] {
        assert!(subscriptions.contains(&set.into()));
    }
    sqlx::query("DELETE FROM subscriptions WHERE user_id = 1 AND set_id = 'mass-spec-concepts'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE notes SET fields = json_remove(fields, '$.image') WHERE id = 'term-esi'")
        .execute(&pool)
        .await
        .unwrap();
    import(&pool).await;
    assert!(
        store::note(&pool, "term-esi")
            .await
            .unwrap()
            .note
            .get("image")
            .is_none(),
        "a later deletion is an authored edit"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM subscriptions WHERE user_id = 1 AND set_id = 'mass-spec-concepts'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn pictures_can_be_added_or_removed_without_losing_a_card_schedule() {
    let pool = open_db("sqlite::memory:").await.unwrap();
    import(&pool).await;
    let mut note = store::note(&pool, "term-mz").await.unwrap();
    let image = note.note.fields.remove("image").unwrap();
    let update = |fields: &_| Some(json!({"fields":fields}));
    assert_eq!(
        call(&pool, "PUT", "/api/notes/term-mz", update(&note.note.fields))
            .await
            .0,
        StatusCode::OK
    );
    let uri = "/api/review/card?note_id=term-mz&template_id=term-to-image";
    assert_eq!(call(&pool, "GET", uri, None).await.0, StatusCode::NOT_FOUND);
    note.note.fields.insert("image".into(), image.clone());
    call(&pool, "PUT", "/api/notes/term-mz", update(&note.note.fields)).await;
    add_history(&pool, "term-mz", "term-to-image").await;
    assert_eq!(call(&pool, "GET", uri, None).await.0, StatusCode::OK);
    note.note.fields.remove("image");
    call(&pool, "PUT", "/api/notes/term-mz", update(&note.note.fields)).await;
    assert_eq!(call(&pool, "GET", uri, None).await.0, StatusCode::NOT_FOUND);
    note.note.fields.insert("image".into(), image);
    call(&pool, "PUT", "/api/notes/term-mz", update(&note.note.fields)).await;
    assert_eq!(call(&pool, "GET", uri, None).await.0, StatusCode::OK);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT reps FROM card_state WHERE note_id = 'term-mz' AND template_id = 'term-to-image'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        5
    );
}

#[tokio::test]
async fn edited_ptm_chemistry_keeps_its_example_without_receiving_a_stock_diagram() {
    let pool = legacy_database().await;
    let original = store::note(&pool, "ptm-phospho").await.unwrap();
    let phosphotyrosine = "C1=CC(=CC=C1C[C@@H](C(=O)O)N)OP(=O)(O)O";
    sqlx::query("UPDATE notes SET fields = json_set(fields, '$.smiles', ?) WHERE id = 'ptm-phospho'")
        .bind(phosphotyrosine)
        .execute(&pool)
        .await
        .unwrap();
    add_history(&pool, "ptm-phospho", "structure-to-name").await;
    sqlx::raw_sql(REFRESH).execute(&pool).await.unwrap();
    import(&pool).await;
    import(&pool).await;

    let refreshed = store::note(&pool, "ptm-phospho").await.unwrap();
    assert_eq!(refreshed.note.text("smiles").unwrap(), phosphotyrosine);
    assert!(refreshed.note.get("structure").is_none());
    for field in ["example", "note"] {
        assert_eq!(refreshed.note.get(field), original.note.get(field));
    }
    for template in ["structure-to-name", "name-to-structure"] {
        let card: (bool, bool) = sqlx::query_as(
            "SELECT enabled, content_available FROM cards WHERE note_id = 'ptm-phospho' AND template_id = ?",
        )
        .bind(template)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(card, (true, false));
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT reps FROM card_state WHERE note_id = 'ptm-phospho' AND template_id = 'structure-to-name'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        5
    );
    assert_eq!(
        call(
            &pool,
            "GET",
            "/api/review/card?note_id=ptm-phospho&template_id=name-to-residues",
            None
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn authored_notes_keep_their_existing_drills_when_seed_schemas_change() {
    let pool = legacy_database().await;
    sqlx::raw_sql(
        "INSERT INTO notes (id, note_type_id, fields) VALUES
         ('my-term', 'Term', '{\"term\":\"My concept\",\"definition\":\"My explanation\"}');
         INSERT INTO set_notes (set_id, note_id, position) VALUES ('ms-vocabulary', 'my-term', 99);
         INSERT INTO cards (note_id, template_id, position, enabled) VALUES
         ('my-term', 'term-to-definition', 0, 1);",
    )
    .execute(&pool)
    .await
    .unwrap();
    add_history(&pool, "my-term", "term-to-definition").await;
    sqlx::raw_sql(REFRESH).execute(&pool).await.unwrap();
    import(&pool).await;
    assert_eq!(
        call(
            &pool,
            "GET",
            "/api/review/card?note_id=my-term&template_id=term-to-definition",
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &pool,
            "GET",
            "/api/review/card?note_id=my-term&template_id=term-to-image",
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        store::note(&pool, "my-term")
            .await
            .unwrap()
            .note
            .text("definition")
            .unwrap(),
        "My explanation"
    );
    assert!(
        store::set_notes(&pool, "mass-spec-concepts")
            .await
            .unwrap()
            .iter()
            .any(|n| n.note.id == "my-term")
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT reps FROM card_state WHERE note_id = 'my-term'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        5
    );
}
