//! Explicit, one-time upgrades from the original text-heavy sets to visual study sets.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use sqlx::{SqliteConnection, SqlitePool};
use srs_core::content::SetFile;
use srs_core::model::{Note, NoteType};

use crate::error::ApiResult;
use crate::store::{self, StoredNote};

const REFRESH: &str = "visual-study-sets-2026-11-03";
const CURATE: &str = "curated-study-content-2026-11-03";
const REORGANIZE: &str = "curated-study-library-2026-11-03";
const SACCHARIDE_STRUCTURES: &str = "monosaccharide-structures-2026-11-03";

// These immutable originals distinguish seed values from edits made in the app.
fn baselines() -> &'static BTreeMap<String, (String, Note)> {
    static NOTES: OnceLock<BTreeMap<String, (String, Note)>> = OnceLock::new();
    NOTES.get_or_init(|| {
        [
            include_str!("seed_baselines/ptms.yaml"),
            include_str!("seed_baselines/ms-vocabulary.yaml"),
            include_str!("seed_baselines/glycans.yaml"),
        ]
        .into_iter()
        .flat_map(|source| {
            let file = SetFile::from_yaml(source).expect("embedded seed baseline is valid");
            file.notes
                .into_iter()
                .map(move |note| (note.id.clone(), (file.note_type.id.clone(), note)))
        })
        .collect()
    })
}

fn curated_baselines() -> &'static BTreeMap<String, (String, Note)> {
    static NOTES: OnceLock<BTreeMap<String, (String, Note)>> = OnceLock::new();
    NOTES.get_or_init(|| {
        [
            include_str!("seed_baselines/glycosylation-reference-v1.yaml"),
            include_str!("seed_baselines/glycan-motifs-v1.yaml"),
            include_str!("seed_baselines/fragmentation-v1.yaml"),
        ]
        .into_iter()
        .flat_map(|source| {
            let file = SetFile::from_yaml(source).expect("embedded seed baseline is valid");
            file.notes
                .into_iter()
                .map(move |note| (note.id.clone(), (file.note_type.id.clone(), note)))
        })
        .collect()
    })
}

fn monosaccharide_baselines() -> &'static BTreeMap<String, (String, Note)> {
    static NOTES: OnceLock<BTreeMap<String, (String, Note)>> = OnceLock::new();
    NOTES.get_or_init(|| {
        let file = SetFile::from_yaml(include_str!("seed_baselines/monosaccharides-v1.yaml"))
            .expect("embedded seed baseline is valid");
        file.notes
            .into_iter()
            .map(|note| (note.id.clone(), (file.note_type.id.clone(), note)))
            .collect()
    })
}

async fn recorded(db: &mut SqliteConnection, refresh: &str, note: &str) -> ApiResult<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM seed_content_refreshes WHERE refresh_id = ? AND note_id = ?",
    )
    .bind(refresh)
    .bind(note)
    .fetch_one(&mut *db)
    .await?;
    Ok(count != 0)
}

async fn remember(db: &mut SqliteConnection, refresh: &str, note: &str) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO seed_content_refreshes (refresh_id, note_id) VALUES (?, ?) ON CONFLICT DO NOTHING",
    )
    .bind(refresh)
    .bind(note)
    .execute(&mut *db)
    .await?;
    Ok(())
}

pub async fn remember_new_note(db: &mut SqliteConnection, note: &str) -> ApiResult<()> {
    if baselines().contains_key(note) {
        remember(db, REFRESH, note).await?;
    }
    if curated_baselines().contains_key(note) {
        remember(db, CURATE, note).await?;
    }
    if monosaccharide_baselines().contains_key(note) {
        remember(db, SACCHARIDE_STRUCTURES, note).await?;
    }
    Ok(())
}

/// Replace only unchanged original values. New pictures fill missing fields; authored values,
/// status, timestamps and all per-user progress survive. Returns whether the type matches.
pub async fn refresh_note(
    db: &mut SqliteConnection,
    existing: &StoredNote,
    note_type: &NoteType,
    updated: &Note,
) -> ApiResult<bool> {
    let mut matches = existing.note_type_id == note_type.id;
    if let Some((old_type, original)) = baselines().get(&updated.id) {
        matches = apply_refresh(db, existing, note_type, updated, old_type, original, REFRESH).await?;
    }
    if let Some((old_type, original)) = curated_baselines().get(&updated.id) {
        let existing = store::note(&mut *db, &updated.id).await?;
        matches = apply_refresh(db, &existing, note_type, updated, old_type, original, CURATE).await?;
    }
    if let Some((old_type, original)) = monosaccharide_baselines().get(&updated.id) {
        let existing = store::note(&mut *db, &updated.id).await?;
        matches = apply_refresh(
            db,
            &existing,
            note_type,
            updated,
            old_type,
            original,
            SACCHARIDE_STRUCTURES,
        )
        .await?;
    }
    Ok(matches)
}

async fn apply_refresh(
    db: &mut SqliteConnection,
    existing: &StoredNote,
    note_type: &NoteType,
    updated: &Note,
    old_type: &str,
    original: &Note,
    receipt: &str,
) -> ApiResult<bool> {
    if existing.note_type_id != old_type && existing.note_type_id != note_type.id {
        return Ok(false);
    }
    if recorded(db, receipt, &updated.id).await? {
        return Ok(existing.note_type_id == note_type.id);
    }

    // The old PTM picture was derived from SMILES. A different chemical source needs the
    // author's own diagram, rather than a stock picture or new example-specific wording.
    let authored_chemistry = old_type == "PTM" && existing.note.get("smiles") != original.get("smiles");
    let authored_sugar = old_type == "Monosaccharide"
        && existing.note.get("smiles").is_some()
        && existing.note.get("smiles") != updated.get("smiles");
    let keep_example = |key: &str| {
        (authored_chemistry && matches!(key, "structure" | "example" | "note"))
            || (authored_sugar && key == "structure_form")
    };
    let mut fields = existing.note.fields.clone();
    for (key, old) in &original.fields {
        if keep_example(key) {
            continue;
        }
        if fields.get(key) == Some(old) {
            match updated.fields.get(key) {
                Some(value) => {
                    fields.insert(key.clone(), value.clone());
                }
                None => {
                    fields.remove(key);
                }
            }
        }
    }
    for (key, value) in &updated.fields {
        if !keep_example(key) && !original.fields.contains_key(key) && !fields.contains_key(key) {
            fields.insert(key.clone(), value.clone());
        }
    }
    if fields != existing.note.fields || existing.note_type_id != note_type.id {
        sqlx::query("UPDATE notes SET fields = ?, note_type_id = ? WHERE id = ?")
            .bind(serde_json::to_string(&fields)?)
            .bind(&note_type.id)
            .bind(&updated.id)
            .execute(&mut *db)
            .await?;
    }
    remember(db, receipt, &updated.id).await?;
    Ok(true)
}

pub async fn finish_import(pool: &SqlitePool) -> ApiResult<()> {
    let mut tx = pool.begin().await?;
    let destinations = ["mass-spec-concepts", "ms-fragmentation", "glycan-motifs"];
    let mut all_present = true;
    for destination in destinations {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sets WHERE id = ?")
            .bind(destination)
            .fetch_one(&mut *tx)
            .await?;
        all_present &= count != 0;
    }
    if all_present && !recorded(&mut tx, REORGANIZE, "sets").await? {
        for source in [
            "ms-vocabulary",
            "ms-ionization",
            "ms-analyzers",
            "ms-acquisition",
            "ms-workflow",
        ] {
            // Follow selected topics once. Old all-in-one libraries also included fragmentation.
            sqlx::query(
                "INSERT INTO subscriptions (user_id, set_id)
                 SELECT user_id, ? FROM subscriptions WHERE set_id = ? ON CONFLICT DO NOTHING",
            )
            .bind("mass-spec-concepts")
            .bind(source)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO subscriptions (user_id, set_id)
                 SELECT DISTINCT s.user_id, 'ms-fragmentation' FROM subscriptions s
                 JOIN set_notes old ON old.set_id = s.set_id
                 JOIN set_notes current ON current.note_id = old.note_id AND current.set_id = 'ms-fragmentation'
                 WHERE s.set_id = ? ON CONFLICT DO NOTHING",
            )
            .bind(source)
            .execute(&mut *tx)
            .await?;
            let ids: Vec<String> = sqlx::query_scalar(
                "SELECT old.note_id FROM set_notes old WHERE old.set_id = ?
                 AND NOT EXISTS (SELECT 1 FROM set_notes current
                     WHERE current.note_id = old.note_id AND current.set_id = 'ms-fragmentation')
                 ORDER BY old.position",
            )
            .bind(source)
            .fetch_all(&mut *tx)
            .await?;
            for id in ids {
                store::add_to_set(&mut tx, "mass-spec-concepts", &id).await?;
            }
            sqlx::query("UPDATE sets SET retired = 1 WHERE id = ?")
                .bind(source)
                .execute(&mut *tx)
                .await?;
        }
        // The first visual upgrade already inherited this subscription; respect later opt-outs.
        if !recorded(&mut tx, "visual-study-topics-2026-11-03", "sets").await? {
            sqlx::query(
                "INSERT INTO subscriptions (user_id, set_id)
                 SELECT user_id, 'glycan-motifs' FROM subscriptions WHERE set_id = 'glycans'
                 ON CONFLICT DO NOTHING",
            )
            .execute(&mut *tx)
            .await?;
        }

        // Seed reference paragraphs leave the library. Keep separately authored notes usable.
        let references: Vec<String> =
            sqlx::query_scalar("SELECT note_id FROM set_notes WHERE set_id = 'glycans' ORDER BY position")
                .fetch_all(&mut *tx)
                .await?;
        let authored: Vec<_> = references
            .into_iter()
            .filter(|id| {
                !baselines()
                    .get(id)
                    .is_some_and(|(kind, _)| kind == "GlycoConcept")
            })
            .collect();
        if !authored.is_empty() {
            sqlx::query(
                "INSERT INTO sets (id, name) VALUES ('glycosylation-authored', 'Your glycosylation notes')
                 ON CONFLICT DO NOTHING",
            )
            .execute(&mut *tx)
            .await?;
            for id in authored {
                store::add_to_set(&mut tx, "glycosylation-authored", &id).await?;
            }
            sqlx::query(
                "INSERT INTO subscriptions (user_id, set_id)
                 SELECT user_id, 'glycosylation-authored' FROM subscriptions WHERE set_id = 'glycans'
                 ON CONFLICT DO NOTHING",
            )
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("UPDATE sets SET retired = 1 WHERE id = 'glycans'")
            .execute(&mut *tx)
            .await?;
        remember(&mut tx, REORGANIZE, "sets").await?;
    }

    // A shared type also applies to notes authored in the app, not just its seed notes.
    for type_id in ["PTM", "Term", "GlycoConcept", "GlycanMotif", "Monosaccharide"] {
        let definition: Option<String> = sqlx::query_scalar("SELECT definition FROM note_types WHERE id = ?")
            .bind(type_id)
            .fetch_optional(&mut *tx)
            .await?;
        let Some(definition) = definition else {
            continue;
        };
        let note_type: NoteType = serde_json::from_str(&definition)?;
        let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM notes WHERE note_type_id = ?")
            .bind(type_id)
            .fetch_all(&mut *tx)
            .await?;
        for id in ids {
            store::ensure_cards(&mut tx, &note_type, &id).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
