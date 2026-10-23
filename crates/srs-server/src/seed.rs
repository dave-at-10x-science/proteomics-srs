//! Importing set files from `seed/`.
//!
//! Import runs on every start and is idempotent:
//! - note types and sets are updated from the file (the file owns the schema), so new templates
//!   appear as new cards;
//! - notes are only inserted if missing, so edits made in the app are never overwritten;
//! - new users are subscribed to default sets.

use std::path::Path;

use chrono::Utc;
use sqlx::SqlitePool;
use srs_core::content::SetFile;

use crate::LOCAL_USER;
use crate::store;

/// Import every `*.yaml` set file one level below `dir` (e.g. `seed/amino-acids/amino-acids.yaml`).
/// Returns the number of files imported.
pub async fn import_dir(pool: &SqlitePool, dir: &Path) -> anyhow::Result<usize> {
    if !dir.is_dir() {
        tracing::warn!(dir = %dir.display(), "seed directory not found; skipping import");
        return Ok(0);
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            for f in std::fs::read_dir(&path)? {
                let f = f?.path();
                if f.extension().is_some_and(|e| e == "yaml" || e == "yml") {
                    files.push(f);
                }
            }
        }
    }
    files.sort();
    for f in &files {
        let src = std::fs::read_to_string(f)?;
        let set = SetFile::from_yaml(&src).map_err(|e| anyhow::anyhow!("{}: {e}", f.display()))?;
        import_set(pool, &set).await?;
    }
    Ok(files.len())
}

pub async fn import_set(pool: &SqlitePool, file: &SetFile) -> anyhow::Result<()> {
    let now = Utc::now();
    let mut tx = pool.begin().await?;

    sqlx::query("INSERT INTO note_types (id, definition) VALUES (?, ?) ON CONFLICT (id) DO UPDATE SET definition = excluded.definition")
        .bind(&file.note_type.id)
        .bind(serde_json::to_string(&file.note_type)?)
        .execute(&mut *tx)
        .await?;

    let new_set = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sets WHERE id = ?")
        .bind(&file.set.id)
        .fetch_one(&mut *tx)
        .await?
        == 0;
    sqlx::query(
        "INSERT INTO sets (id, name, is_default, tags) VALUES (?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET name = excluded.name, is_default = excluded.is_default,
             tags = excluded.tags",
    )
    .bind(&file.set.id)
    .bind(&file.set.name)
    .bind(file.set.default)
    .bind(serde_json::to_string(&file.set.tags)?)
    .execute(&mut *tx)
    .await?;

    for note in &file.notes {
        let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM notes WHERE id = ?")
            .bind(&note.id)
            .fetch_one(&mut *tx)
            .await?
            > 0;
        if exists {
            store::ensure_cards(&mut tx, &file.note_type, &note.id).await?;
        } else {
            store::upsert_note(&mut tx, &file.note_type, note, "published", now).await?;
        }
        store::add_to_set(&mut tx, &file.set.id, &note.id).await?;
    }

    // Subscribe to a default set the first time it appears, so later unsubscribes stick.
    if new_set && file.set.default {
        sqlx::query("INSERT INTO subscriptions (user_id, set_id) VALUES (?, ?) ON CONFLICT DO NOTHING")
            .bind(LOCAL_USER)
            .bind(&file.set.id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}
