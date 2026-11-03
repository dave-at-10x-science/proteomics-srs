//! Explicit, one-time upgrades from the original text-heavy sets to visual study sets.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use sqlx::{SqliteConnection, SqlitePool};
use srs_core::content::SetFile;
use srs_core::model::{Note, NoteType};

use crate::error::ApiResult;
use crate::store::{self, StoredNote};

const REFRESH: &str = "visual-study-sets-2026-11-03";
const REORGANIZE: &str = "visual-study-topics-2026-11-03";

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
    let Some((old_type, original)) = baselines().get(&updated.id) else {
        return Ok(existing.note_type_id == note_type.id);
    };
    if existing.note_type_id != *old_type && existing.note_type_id != note_type.id {
        return Ok(false);
    }
    if recorded(db, REFRESH, &updated.id).await? {
        return Ok(existing.note_type_id == note_type.id);
    }

    // The old PTM picture was derived from SMILES. A different chemical source needs the
    // author's own diagram, rather than a stock picture or new example-specific wording.
    let authored_chemistry = old_type == "PTM" && existing.note.get("smiles") != original.get("smiles");
    let keep_example = |key: &str| authored_chemistry && matches!(key, "structure" | "example" | "note");
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
    remember(db, REFRESH, &updated.id).await?;
    Ok(true)
}

pub async fn finish_import(pool: &SqlitePool) -> ApiResult<()> {
    let mut tx = pool.begin().await?;
    let destinations = [
        ("ms-vocabulary", "ms-ionization", "Term"),
        ("ms-vocabulary", "ms-analyzers", "Term"),
        ("ms-vocabulary", "ms-acquisition", "Term"),
        ("ms-vocabulary", "ms-fragmentation", "Term"),
        ("ms-vocabulary", "ms-workflow", "Term"),
        ("glycans", "glycan-motifs", "GlycoConcept"),
    ];
    let mut all_present = true;
    for (_, destination, _) in destinations {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sets WHERE id = ?")
            .bind(destination)
            .fetch_one(&mut *tx)
            .await?;
        all_present &= count != 0;
    }
    if all_present && !recorded(&mut tx, REORGANIZE, "sets").await? {
        for (source, destination, old_type) in destinations {
            // Follow the topics the learner already selected, once. Later opt-outs stick.
            sqlx::query(
                "INSERT INTO subscriptions (user_id, set_id)
                 SELECT user_id, ? FROM subscriptions WHERE set_id = ? ON CONFLICT DO NOTHING",
            )
            .bind(destination)
            .bind(source)
            .execute(&mut *tx)
            .await?;
            let ids: Vec<String> = sqlx::query_scalar("SELECT note_id FROM set_notes WHERE set_id = ?")
                .bind(destination)
                .fetch_all(&mut *tx)
                .await?;
            for id in ids {
                if baselines().get(&id).is_some_and(|(kind, _)| kind == old_type) {
                    sqlx::query("DELETE FROM set_notes WHERE set_id = ? AND note_id = ?")
                        .bind(source)
                        .bind(&id)
                        .execute(&mut *tx)
                        .await?;
                }
            }
        }
        remember(&mut tx, REORGANIZE, "sets").await?;
    }

    // A shared type also applies to notes authored in the app, not just its seed notes.
    for type_id in ["PTM", "Term", "GlycoConcept", "GlycanMotif"] {
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
