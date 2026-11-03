use serde_json::{Value, json};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use srs_core::content::SetFile;
use srs_server::seed;

const REFRESH: &str = include_str!("../migrations/0005_refresh_monosaccharide_visuals.sql");
const UPDATED_SEED: &str = include_str!("../../../seed/monosaccharides/monosaccharides.yaml");
const LEGACY_TIME: &str = "2026-10-23T12:00:00Z";

// Original imported values, before visual symbols and shorter contexts were introduced.
const LEGACY_NOTES: &str = r#"[
  {"id":"mono-glc","name":"Glucose","abbreviation":"Glc","shorthand":"Hex","symbol":"Blue circle","residue_mass":162.05282,"formula":"C6H10O5","note":"Glucose, mannose and galactose are isomers: all three are \"Hex\" to the mass spectrometer. Glucose is part of the N-glycan precursor in the ER and is trimmed off before the glycoprotein leaves."},
  {"id":"mono-man","name":"Mannose","abbreviation":"Man","shorthand":"Hex","symbol":"Green circle","residue_mass":162.05282,"formula":"C6H10O5","note":"Three mannoses form the shared N-glycan core; high-mannose glycans carry 5 to 9."},
  {"id":"mono-gal","name":"Galactose","abbreviation":"Gal","shorthand":"Hex","symbol":"Yellow circle","residue_mass":162.05282,"formula":"C6H10O5","note":"Extends the antennae of complex N-glycans and forms O-glycan core 1 (Galβ1-3GalNAc)."},
  {"id":"mono-glcnac","name":"N-Acetylglucosamine","abbreviation":"GlcNAc","shorthand":"HexNAc","symbol":"Blue square","residue_mass":203.07937,"formula":"C8H13NO5","note":"Two GlcNAcs anchor every N-glycan to asparagine. A single GlcNAc on Ser/Thr is O-GlcNAc."},
  {"id":"mono-galnac","name":"N-Acetylgalactosamine","abbreviation":"GalNAc","shorthand":"HexNAc","symbol":"Yellow square","residue_mass":203.07937,"formula":"C8H13NO5","note":"The first sugar of mucin-type O-glycans, attached to Ser or Thr. Same mass as GlcNAc."},
  {"id":"mono-fuc","name":"Fucose","abbreviation":"Fuc","shorthand":"dHex","symbol":"Red triangle","residue_mass":146.05791,"formula":"C6H10O4","note":"A deoxyhexose. Core fucose (α1-6 on the Asn-linked GlcNAc) is common on complex N-glycans; antenna fucose builds Lewis antigens."},
  {"id":"mono-neuac","name":"N-Acetylneuraminic acid","abbreviation":"NeuAc","shorthand":"NeuAc","symbol":"Purple diamond","residue_mass":291.09542,"formula":"C11H17NO8","note":"The main human sialic acid, capping glycan antennae with a negative charge. Labile in the source and in CID; oxonium ions at m/z 292.103 and 274.092."},
  {"id":"mono-neugc","name":"N-Glycolylneuraminic acid","abbreviation":"NeuGc","shorthand":"NeuGc","symbol":"Light blue diamond","residue_mass":307.09033,"formula":"C11H17NO9","note":"One oxygen heavier than NeuAc (+15.995 Da). Humans can't make it, so it flags non-human (e.g. mouse or hamster cell) glycosylation in biotherapeutics."},
  {"id":"mono-xyl","name":"Xylose","abbreviation":"Xyl","shorthand":"Pent","symbol":"Orange star","residue_mass":132.04226,"formula":"C5H8O4","note":"A pentose. Starts the linker of glycosaminoglycan chains on proteoglycans, and appears as β1-2 xylose on plant N-glycans."}
]"#;

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
    for migration in [
        include_str!("../migrations/0001_initial.sql"),
        include_str!("../migrations/0002_game_sessions.sql"),
        include_str!("../migrations/0003_studied_notes.sql"),
        include_str!("../migrations/0004_uppercase_three_letter_codes.sql"),
        include_str!("../migrations/0006_visual_study_sets.sql"),
    ] {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(
        "INSERT INTO note_types (id, definition) VALUES ('Monosaccharide', '{}'), ('Other', '{}');
         INSERT INTO sets (id, name) VALUES ('monosaccharides', 'Monosaccharides');",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

fn legacy_notes() -> Vec<(String, Value)> {
    serde_json::from_str::<Vec<Value>>(LEGACY_NOTES)
        .unwrap()
        .into_iter()
        .map(|mut note| {
            let id = note.as_object_mut().unwrap().remove("id").unwrap();
            (id.as_str().unwrap().to_string(), note)
        })
        .collect()
}

async fn insert_note(pool: &SqlitePool, id: &str, note_type: &str, fields: &Value, status: &str) {
    sqlx::query(
        "INSERT INTO notes (id, note_type_id, fields, status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(note_type)
    .bind(fields.to_string())
    .bind(status)
    .bind(LEGACY_TIME)
    .bind(LEGACY_TIME)
    .execute(pool)
    .await
    .unwrap();
}

async fn fields(pool: &SqlitePool, id: &str) -> Value {
    let fields: String = sqlx::query_scalar("SELECT fields FROM notes WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    serde_json::from_str(&fields).unwrap()
}

async fn assert_seed_fields(pool: &SqlitePool) {
    let seed = SetFile::from_yaml(UPDATED_SEED).unwrap();
    for note in seed.notes {
        assert_eq!(
            fields(pool, &note.id).await,
            serde_json::to_value(note.fields).unwrap(),
            "{} should match the refreshed seed",
            note.id
        );
    }
}

#[tokio::test]
async fn refresh_allows_a_fresh_database_to_import_the_updated_seed() {
    let pool = legacy_database().await;
    sqlx::raw_sql(REFRESH).execute(&pool).await.unwrap();
    seed::import_set(&pool, &SetFile::from_yaml(UPDATED_SEED).unwrap())
        .await
        .unwrap();

    assert_seed_fields(&pool).await;
    let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cards WHERE enabled = 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let retired: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cards WHERE enabled = 0")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(active, 27);
    assert_eq!(retired, 18);
}

#[tokio::test]
async fn refresh_updates_original_fields_once_without_changing_note_metadata() {
    let pool = legacy_database().await;
    for (id, original) in legacy_notes() {
        insert_note(&pool, &id, "Monosaccharide", &original, "published").await;
    }

    sqlx::raw_sql(REFRESH).execute(&pool).await.unwrap();
    assert_seed_fields(&pool).await;
    let unchanged_metadata: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notes WHERE status = 'published' AND created_at = ? AND updated_at = ?",
    )
    .bind(LEGACY_TIME)
    .bind(LEGACY_TIME)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(unchanged_metadata, 9);

    let before: i64 = sqlx::query_scalar("SELECT total_changes()")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(REFRESH).execute(&pool).await.unwrap();
    let after: i64 = sqlx::query_scalar("SELECT total_changes()")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after, before, "reapplying the refresh must not update rows");
}

#[tokio::test]
async fn refresh_preserves_authored_values_and_only_changes_matching_seed_fields() {
    let pool = legacy_database().await;
    let originals = legacy_notes();
    let mut custom = originals
        .iter()
        .find(|(id, _)| id == "mono-neuac")
        .unwrap()
        .1
        .clone();
    custom["name"] = json!("My sialic acid");
    custom["abbreviation"] = json!("My abbreviation");
    custom["abbreviation_alias"] = json!("My alias");
    custom["shorthand"] = json!("My class");
    custom["symbol"] = json!("/media/custom.svg");
    custom["note"] = json!("A context I wrote");
    custom["residue_mass"] = json!(999.0);
    custom["formula"] = json!("My formula");
    custom["mnemonic"] = json!("My extra field");
    insert_note(&pool, "mono-neuac", "Monosaccharide", &custom, "draft").await;

    let mut partial = originals
        .iter()
        .find(|(id, _)| id == "mono-glc")
        .unwrap()
        .1
        .clone();
    partial["note"] = json!("Keep this edited context");
    partial["mnemonic"] = json!("Keep this extra field");
    insert_note(&pool, "mono-glc", "Monosaccharide", &partial, "published").await;
    partial["symbol"] = json!("/study/monosaccharides/glc.svg");
    partial.as_object_mut().unwrap().remove("residue_mass");
    partial.as_object_mut().unwrap().remove("formula");

    let other_type = &originals.iter().find(|(id, _)| id == "mono-neugc").unwrap().1;
    insert_note(&pool, "mono-neugc", "Other", other_type, "published").await;
    insert_note(
        &pool,
        "authored-glc",
        "Monosaccharide",
        &originals[0].1,
        "published",
    )
    .await;

    sqlx::raw_sql(REFRESH).execute(&pool).await.unwrap();
    assert_eq!(fields(&pool, "mono-neuac").await, custom);
    assert_eq!(fields(&pool, "mono-glc").await, partial);
    assert_eq!(fields(&pool, "mono-neugc").await, *other_type);
    assert_eq!(fields(&pool, "authored-glc").await, originals[0].1);
    let status: String = sqlx::query_scalar("SELECT status FROM notes WHERE id = 'mono-neuac'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "draft");
}

#[tokio::test]
async fn refresh_retires_old_directions_without_removing_progress_or_other_cards() {
    let pool = legacy_database().await;
    let original = &legacy_notes()[0].1;
    insert_note(&pool, "mono-glc", "Monosaccharide", original, "published").await;
    insert_note(&pool, "other-glc", "Other", original, "published").await;
    for template in [
        "name-to-residue-mass",
        "name-to-shorthand",
        "name-to-abbreviation",
    ] {
        for id in ["mono-glc", "other-glc"] {
            sqlx::query("INSERT INTO cards (note_id, template_id, position) VALUES (?, ?, 0)")
                .bind(id)
                .bind(template)
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query(
            "INSERT INTO card_state
             (user_id, note_id, template_id, phase, stability, difficulty, due, last_review, reps, lapses)
             VALUES (1, 'mono-glc', ?, 'review', 4.5, 3.2, ?, ?, 7, 2)",
        )
        .bind(template)
        .bind(LEGACY_TIME)
        .bind(LEGACY_TIME)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO review_log
             (user_id, note_id, template_id, rating, phase_before, reviewed_at, duration_ms, answer)
             VALUES (1, 'mono-glc', ?, 3, 'review', ?, 321, 'Legacy answer')",
        )
        .bind(template)
        .bind(LEGACY_TIME)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::raw_sql(
        "INSERT INTO studied_notes (user_id, note_id, studied_at)
         VALUES (1, 'mono-glc', '2026-10-23T12:00:00Z');
         INSERT INTO subscriptions (user_id, set_id) VALUES (1, 'monosaccharides');
         INSERT INTO game_sessions
         (user_id, set_id, pairing, pairs, flips, duration_ms, completed, played_at)
         VALUES (1, 'monosaccharides', 'name:symbol', 3, 8, 12000, 1, '2026-10-23T12:00:00Z');",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(REFRESH).execute(&pool).await.unwrap();
    seed::import_set(&pool, &SetFile::from_yaml(UPDATED_SEED).unwrap())
        .await
        .unwrap();

    let retired: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cards WHERE note_id = 'mono-glc' AND enabled = 0
         AND template_id IN ('name-to-residue-mass', 'name-to-shorthand')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(retired, 2);
    let retained: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM card_state WHERE user_id = 1 AND note_id = 'mono-glc'
         AND phase = 'review' AND stability = 4.5 AND difficulty = 3.2
         AND due = ? AND last_review = ? AND reps = 7 AND lapses = 2",
    )
    .bind(LEGACY_TIME)
    .bind(LEGACY_TIME)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(retained, 3);
    let reviews: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM review_log WHERE user_id = 1 AND note_id = 'mono-glc'
         AND rating = 3 AND phase_before = 'review' AND reviewed_at = ?
         AND duration_ms = 321 AND answer = 'Legacy answer'",
    )
    .bind(LEGACY_TIME)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(reviews, 3);
    let other_cards: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cards WHERE note_id = 'other-glc' AND enabled = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(other_cards, 3);
    for table in ["studied_notes", "subscriptions", "game_sessions"] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE user_id = 1"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1, "{table} must survive the refresh");
    }
}
