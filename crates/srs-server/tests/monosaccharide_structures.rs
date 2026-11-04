use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use srs_core::content::SetFile;
use srs_core::model::FieldType;
use srs_server::{AppState, app, open_db, seed, store};
use tower::ServiceExt;

const PREVIOUS: &str = include_str!("../src/seed_baselines/monosaccharides-v1.yaml");
const CURRENT: &str = include_str!("../../../seed/monosaccharides/monosaccharides.yaml");
const LEGACY_TIME: &str = "2026-10-23T12:00:00Z";

async fn import(pool: &SqlitePool) {
    seed::import_set(pool, &SetFile::from_yaml(CURRENT).unwrap())
        .await
        .unwrap();
}

async fn get(pool: &SqlitePool, uri: &str) -> Value {
    let state = AppState::new(pool.clone(), std::env::temp_dir().join("srs-sugar-test-media"));
    let response = app(state, None)
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status, StatusCode::OK, "{uri}: {value}");
    value
}

// Insert the former installed content directly: importing it through today's code would
// incorrectly mark this fixture as already having received the new structures.
async fn previous_library() -> SqlitePool {
    let pool = open_db("sqlite::memory:").await.unwrap();
    let previous = SetFile::from_yaml(PREVIOUS).unwrap();
    sqlx::query("INSERT INTO note_types (id, definition) VALUES (?, ?)")
        .bind(&previous.note_type.id)
        .bind(serde_json::to_string(&previous.note_type).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO sets (id, name) VALUES (?, ?)")
        .bind(&previous.set.id)
        .bind(&previous.set.name)
        .execute(&pool)
        .await
        .unwrap();
    for (position, note) in previous.notes.iter().enumerate() {
        sqlx::query(
            "INSERT INTO notes (id, note_type_id, fields, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&note.id)
        .bind(&previous.note_type.id)
        .bind(serde_json::to_string(&note.fields).unwrap())
        .bind(LEGACY_TIME)
        .bind(LEGACY_TIME)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO set_notes (set_id, note_id, position) VALUES (?, ?, ?)")
            .bind(&previous.set.id)
            .bind(&note.id)
            .bind(position as i64)
            .execute(&pool)
            .await
            .unwrap();
        for (position, template) in previous.note_type.templates.iter().enumerate() {
            sqlx::query("INSERT INTO cards (note_id, template_id, position, enabled) VALUES (?, ?, ?, ?)")
                .bind(&note.id)
                .bind(&template.id)
                .bind(position as i64)
                .bind(template.enabled_by_default)
                .execute(&pool)
                .await
                .unwrap();
        }
    }
    pool
}

#[tokio::test]
async fn monosaccharide_cards_keep_snfg_questions_and_include_explicit_skeletal_forms() {
    let pool = open_db("sqlite::memory:").await.unwrap();
    import(&pool).await;
    let previous = SetFile::from_yaml(PREVIOUS).unwrap();
    let kind = store::note_type(&pool, "Monosaccharide").await.unwrap();
    assert_eq!(kind.templates, previous.note_type.templates);
    let chemical = kind.field("chemical_structure").unwrap();
    assert_eq!(chemical.field_type, FieldType::Image);
    assert_eq!(chemical.from.as_deref(), Some("smiles"));
    assert!(chemical.optional);

    let notes = get(&pool, "/api/sets/monosaccharides/notes").await;
    assert_eq!(notes.as_array().unwrap().len(), 9);
    for original in previous.notes {
        let note = notes
            .as_array()
            .unwrap()
            .iter()
            .find(|note| note["id"] == original.id)
            .unwrap();
        assert_eq!(note["symbol"], original.fields["symbol"]);
        let smiles = note["smiles"].as_str().unwrap();
        assert!(
            smiles.contains('@'),
            "{} needs explicit stereochemistry",
            original.id
        );
        let form = note["structure_form"].as_str().unwrap();
        assert!(
            form.contains('α') || form.contains('β'),
            "{}: {form}",
            original.id
        );
        assert!(
            form.contains("-D-") || form.contains("-L-"),
            "{}: {form}",
            original.id
        );
        assert!(form.contains("pyran"), "{}: {form}", original.id);
        assert!(
            note.get("chemical_structure").is_none(),
            "the image is derived from SMILES"
        );
    }
    let counts: (i64, i64) = sqlx::query_as("SELECT COUNT(*), SUM(enabled) FROM cards")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(counts, (45, 27));

    let question = get(
        &pool,
        "/api/review/card?note_id=mono-glc&template_id=name-to-symbol",
    )
    .await;
    let choices = question["choices"].as_array().unwrap();
    assert_eq!(choices.len(), 4);
    assert!(choices.contains(&json!("/study/monosaccharides/glc.svg")));
    assert!(
        choices
            .iter()
            .all(|choice| choice.as_str().unwrap().starts_with("/study/monosaccharides/"))
    );
}

#[tokio::test]
async fn adding_skeletal_forms_preserves_authored_content_and_learning_history() {
    let pool = previous_library().await;
    let current = SetFile::from_yaml(CURRENT).unwrap();
    let authored_smiles = current
        .notes
        .iter()
        .find(|note| note.id == "mono-gal")
        .unwrap()
        .text("smiles")
        .unwrap();
    sqlx::query(
        "UPDATE notes SET fields = json_set(fields, '$.name', 'My glucose', '$.note', 'My context'),
         status = 'draft' WHERE id = 'mono-glc'",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE notes SET fields = json_set(fields, '$.smiles', ?) WHERE id = 'mono-man'")
        .bind(&authored_smiles)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO card_state (user_id, note_id, template_id, phase, stability, difficulty,
         due, last_review, reps, lapses)
         VALUES (1, 'mono-glc', 'name-to-symbol', 'review', 4.5, 3.2, ?, ?, 7, 2)",
    )
    .bind(LEGACY_TIME)
    .bind(LEGACY_TIME)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO review_log (user_id, note_id, template_id, rating, phase_before, reviewed_at, answer)
         VALUES (1, 'mono-glc', 'name-to-symbol', 3, 'review', ?, '/study/monosaccharides/glc.svg')",
    )
    .bind(LEGACY_TIME)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO studied_notes (user_id, note_id, studied_at) VALUES (1, 'mono-glc', ?)")
        .bind(LEGACY_TIME)
        .execute(&pool)
        .await
        .unwrap();

    import(&pool).await;
    let glucose = store::note(&pool, "mono-glc").await.unwrap();
    assert_eq!(glucose.note.text("name").as_deref(), Some("My glucose"));
    assert_eq!(glucose.note.text("note").as_deref(), Some("My context"));
    assert_eq!(glucose.status, "draft");
    assert!(glucose.note.text("smiles").is_some());
    assert!(glucose.note.text("structure_form").is_some());
    let mannose = store::note(&pool, "mono-man").await.unwrap();
    assert_eq!(
        mannose.note.text("smiles").as_deref(),
        Some(authored_smiles.as_str())
    );
    assert!(
        mannose.note.get("structure_form").is_none(),
        "a stock caption would mislabel authored chemistry"
    );

    sqlx::query(
        "UPDATE notes SET fields = json_remove(fields, '$.smiles', '$.structure_form') WHERE id = 'mono-glc'",
    )
    .execute(&pool)
    .await
    .unwrap();
    import(&pool).await;
    let glucose = store::note(&pool, "mono-glc").await.unwrap();
    assert!(glucose.note.get("smiles").is_none());
    assert!(glucose.note.get("structure_form").is_none());
    assert_eq!(glucose.note.text("name").as_deref(), Some("My glucose"));
    let metadata: (String, String, String) =
        sqlx::query_as("SELECT status, created_at, updated_at FROM notes WHERE id = 'mono-glc'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(metadata, ("draft".into(), LEGACY_TIME.into(), LEGACY_TIME.into()));
    let retained_state: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM card_state WHERE user_id = 1 AND note_id = 'mono-glc'
         AND template_id = 'name-to-symbol' AND phase = 'review' AND stability = 4.5
         AND difficulty = 3.2 AND due = ? AND last_review = ? AND reps = 7 AND lapses = 2",
    )
    .bind(LEGACY_TIME)
    .bind(LEGACY_TIME)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(retained_state, 1);
    let retained_log: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM review_log WHERE user_id = 1 AND note_id = 'mono-glc'
         AND template_id = 'name-to-symbol' AND rating = 3 AND reviewed_at = ?
         AND answer = '/study/monosaccharides/glc.svg'",
    )
    .bind(LEGACY_TIME)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(retained_log, 1);
    let studied_at: String =
        sqlx::query_scalar("SELECT studied_at FROM studied_notes WHERE user_id = 1 AND note_id = 'mono-glc'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(studied_at, LEGACY_TIME);
    let cards: (i64, i64) = sqlx::query_as("SELECT COUNT(*), SUM(enabled) FROM cards")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(cards, (45, 27));
}

#[tokio::test]
async fn freshly_imported_structure_fields_can_be_removed_without_returning_on_restart() {
    let pool = open_db("sqlite::memory:").await.unwrap();
    import(&pool).await;
    sqlx::query(
        "UPDATE notes SET fields = json_remove(fields, '$.smiles', '$.structure_form') WHERE id = 'mono-glc'",
    )
    .execute(&pool)
    .await
    .unwrap();
    import(&pool).await;
    let glucose = store::note(&pool, "mono-glc").await.unwrap();
    assert!(glucose.note.get("smiles").is_none());
    assert!(glucose.note.get("structure_form").is_none());
    assert!(
        store::note(&pool, "mono-man")
            .await
            .unwrap()
            .note
            .text("smiles")
            .is_some()
    );
}
