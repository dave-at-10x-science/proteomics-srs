//! Database access. Queries are plain SQL with bound parameters, kept portable enough to move
//! to Postgres later.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqliteExecutor, SqlitePool};
use srs_core::model::{CardId, Note, NoteType};
use srs_core::queue::{Candidate, Today};
use srs_core::scheduler::{CardState, Memory, Phase, Rating};

use crate::error::{ApiError, ApiResult};

fn phase_str(p: Phase) -> &'static str {
    match p {
        Phase::New => "new",
        Phase::Learning => "learning",
        Phase::Review => "review",
        Phase::Relearning => "relearning",
    }
}

fn parse_phase(s: &str) -> Phase {
    match s {
        "learning" => Phase::Learning,
        "review" => Phase::Review,
        "relearning" => Phase::Relearning,
        _ => Phase::New,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub new_per_day: u32,
    pub reviews_per_day: Option<u32>,
    pub desired_retention: f32,
}

pub async fn settings(pool: &SqlitePool, user: i64) -> ApiResult<Settings> {
    let row = sqlx::query("SELECT new_per_day, reviews_per_day, desired_retention FROM users WHERE id = ?")
        .bind(user)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("user {user}")))?;
    Ok(Settings {
        new_per_day: row.get::<i64, _>(0) as u32,
        reviews_per_day: row.get::<Option<i64>, _>(1).map(|v| v as u32),
        desired_retention: row.get::<f64, _>(2) as f32,
    })
}

pub async fn save_settings(pool: &SqlitePool, user: i64, s: &Settings) -> ApiResult<()> {
    sqlx::query("UPDATE users SET new_per_day = ?, reviews_per_day = ?, desired_retention = ? WHERE id = ?")
        .bind(s.new_per_day as i64)
        .bind(s.reviews_per_day.map(|v| v as i64))
        .bind(s.desired_retention as f64)
        .bind(user)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn note_type(db: impl SqliteExecutor<'_>, id: &str) -> ApiResult<NoteType> {
    let def: String = sqlx::query_scalar("SELECT definition FROM note_types WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("note type `{id}`")))?;
    Ok(serde_json::from_str(&def)?)
}

pub async fn note_types(pool: &SqlitePool) -> ApiResult<Vec<NoteType>> {
    let defs: Vec<String> = sqlx::query_scalar("SELECT definition FROM note_types ORDER BY id")
        .fetch_all(pool)
        .await?;
    defs.iter().map(|d| Ok(serde_json::from_str(d)?)).collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct StoredNote {
    #[serde(flatten)]
    pub note: Note,
    pub note_type_id: String,
    pub status: String,
}

fn note_from_row(row: &sqlx::sqlite::SqliteRow) -> ApiResult<StoredNote> {
    let fields: BTreeMap<String, serde_json::Value> = serde_json::from_str(row.get("fields"))?;
    Ok(StoredNote {
        note: Note {
            id: row.get("id"),
            fields,
        },
        note_type_id: row.get("note_type_id"),
        status: row.get("status"),
    })
}

pub async fn note(db: impl SqliteExecutor<'_>, id: &str) -> ApiResult<StoredNote> {
    let row = sqlx::query("SELECT id, note_type_id, fields, status FROM notes WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("note `{id}`")))?;
    note_from_row(&row)
}

pub async fn set_notes(pool: &SqlitePool, set_id: &str) -> ApiResult<Vec<StoredNote>> {
    let rows = sqlx::query(
        "SELECT n.id, n.note_type_id, n.fields, n.status
         FROM set_notes sn JOIN notes n ON n.id = sn.note_id
         WHERE sn.set_id = ? ORDER BY sn.position",
    )
    .bind(set_id)
    .fetch_all(pool)
    .await?;
    rows.iter().map(note_from_row).collect()
}

/// Published notes in the user's subscribed sets that haven't been studied yet, in set order.
pub async fn unstudied_notes(pool: &SqlitePool, user: i64) -> ApiResult<Vec<StoredNote>> {
    let rows = sqlx::query(
        "SELECT n.id, n.note_type_id, n.fields, n.status
         FROM notes n
         JOIN set_notes sn ON sn.note_id = n.id
         JOIN subscriptions s ON s.set_id = sn.set_id AND s.user_id = ?1
         WHERE n.status = 'published'
           AND NOT EXISTS (SELECT 1 FROM studied_notes st WHERE st.user_id = ?1 AND st.note_id = n.id)
         GROUP BY n.id
         ORDER BY MIN(sn.set_id), MIN(sn.position)",
    )
    .bind(user)
    .fetch_all(pool)
    .await?;
    rows.iter().map(note_from_row).collect()
}

pub async fn mark_studied(
    pool: &SqlitePool,
    user: i64,
    note_ids: &[String],
    now: DateTime<Utc>,
) -> ApiResult<usize> {
    let mut tx = pool.begin().await?;
    let mut added = 0;
    for id in note_ids {
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes WHERE id = ?")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        if exists == 0 {
            return Err(ApiError::NotFound(format!("note `{id}`")));
        }
        added += sqlx::query(
            "INSERT INTO studied_notes (user_id, note_id, studied_at) VALUES (?, ?, ?) ON CONFLICT DO NOTHING",
        )
        .bind(user)
        .bind(id)
        .bind(now)
        .execute(&mut *tx)
        .await?
        .rows_affected() as usize;
    }
    tx.commit().await?;
    Ok(added)
}

/// The first set (by id) containing the note.
pub async fn set_of_note(pool: &SqlitePool, note_id: &str) -> ApiResult<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT MIN(set_id) FROM set_notes WHERE note_id = ?")
            .bind(note_id)
            .fetch_one(pool)
            .await?,
    )
}

/// Insert or replace a note's fields, and make sure it has a card for every template.
pub async fn upsert_note(
    db: &mut sqlx::SqliteConnection,
    note_type: &NoteType,
    note: &Note,
    status: &str,
    now: DateTime<Utc>,
) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO notes (id, note_type_id, fields, status, updated_at) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET fields = excluded.fields, status = excluded.status,
             updated_at = excluded.updated_at",
    )
    .bind(&note.id)
    .bind(&note_type.id)
    .bind(serde_json::to_string(&note.fields)?)
    .bind(status)
    .bind(now)
    .execute(&mut *db)
    .await?;
    ensure_cards(db, note_type, &note.id).await
}

pub async fn ensure_cards(
    db: &mut sqlx::SqliteConnection,
    note_type: &NoteType,
    note_id: &str,
) -> ApiResult<()> {
    for (i, t) in note_type.templates.iter().enumerate() {
        sqlx::query(
            "INSERT INTO cards (note_id, template_id, position, enabled) VALUES (?, ?, ?, ?)
             ON CONFLICT (note_id, template_id) DO UPDATE SET position = excluded.position",
        )
        .bind(note_id)
        .bind(&t.id)
        .bind(i as i64)
        .bind(t.enabled_by_default)
        .execute(&mut *db)
        .await?;
    }
    Ok(())
}

pub async fn add_to_set(db: &mut sqlx::SqliteConnection, set_id: &str, note_id: &str) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO set_notes (set_id, note_id, position)
         VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM set_notes WHERE set_id = ?1))
         ON CONFLICT DO NOTHING",
    )
    .bind(set_id)
    .bind(note_id)
    .execute(&mut *db)
    .await?;
    Ok(())
}

/// Every enabled, published card in the user's subscribed sets, with its state.
pub async fn candidates(pool: &SqlitePool, user: i64, now: DateTime<Utc>) -> ApiResult<Vec<Candidate>> {
    let rows = sqlx::query(
        "SELECT c.note_id, c.template_id, MIN(sn.set_id) AS set_id,
                cs.phase, cs.stability, cs.difficulty, cs.due, cs.last_review, cs.reps, cs.lapses,
                EXISTS (SELECT 1 FROM studied_notes st WHERE st.user_id = ?1 AND st.note_id = c.note_id)
                    AS studied
         FROM cards c
         JOIN notes n ON n.id = c.note_id AND n.status = 'published'
         JOIN set_notes sn ON sn.note_id = c.note_id
         JOIN subscriptions s ON s.set_id = sn.set_id AND s.user_id = ?1
         LEFT JOIN card_state cs
             ON cs.user_id = ?1 AND cs.note_id = c.note_id AND cs.template_id = c.template_id
         WHERE c.enabled = 1
         GROUP BY c.note_id, c.template_id
         ORDER BY MIN(sn.set_id), MIN(sn.position), c.position",
    )
    .bind(user)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|r| Candidate {
            id: CardId::new(r.get::<String, _>("note_id"), r.get::<String, _>("template_id")),
            set_id: r.get("set_id"),
            state: state_from_row(r).unwrap_or_else(|| CardState::new(now)),
            studied: r.get("studied"),
        })
        .collect())
}

fn state_from_row(r: &sqlx::sqlite::SqliteRow) -> Option<CardState> {
    let phase: Option<String> = r.get("phase");
    let phase = parse_phase(&phase?);
    let memory = match (
        r.get::<Option<f64>, _>("stability"),
        r.get::<Option<f64>, _>("difficulty"),
    ) {
        (Some(s), Some(d)) => Some(Memory {
            stability: s as f32,
            difficulty: d as f32,
        }),
        _ => None,
    };
    Some(CardState {
        phase,
        memory,
        due: r.get("due"),
        last_review: r.get("last_review"),
        reps: r.get::<i64, _>("reps") as u32,
        lapses: r.get::<i64, _>("lapses") as u32,
    })
}

pub async fn card_state(
    pool: &SqlitePool,
    user: i64,
    id: &CardId,
    now: DateTime<Utc>,
) -> ApiResult<CardState> {
    let row = sqlx::query(
        "SELECT phase, stability, difficulty, due, last_review, reps, lapses FROM card_state
         WHERE user_id = ? AND note_id = ? AND template_id = ?",
    )
    .bind(user)
    .bind(&id.note_id)
    .bind(&id.template_id)
    .fetch_optional(pool)
    .await?;
    Ok(row
        .as_ref()
        .and_then(state_from_row)
        .unwrap_or_else(|| CardState::new(now)))
}

pub async fn card_exists(pool: &SqlitePool, id: &CardId) -> ApiResult<bool> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cards WHERE note_id = ? AND template_id = ?")
        .bind(&id.note_id)
        .bind(&id.template_id)
        .fetch_one(pool)
        .await?;
    Ok(n > 0)
}

pub struct ReviewRecord<'a> {
    pub user: i64,
    pub card: &'a CardId,
    pub rating: Rating,
    pub before: &'a CardState,
    pub after: &'a CardState,
    pub at: DateTime<Utc>,
    pub duration_ms: Option<i64>,
    pub answer: Option<&'a str>,
}

/// Save the new card state and append to the review log, atomically.
pub async fn record_review(pool: &SqlitePool, r: ReviewRecord<'_>) -> ApiResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO card_state
             (user_id, note_id, template_id, phase, stability, difficulty, due, last_review, reps, lapses)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (user_id, note_id, template_id) DO UPDATE SET
             phase = excluded.phase, stability = excluded.stability, difficulty = excluded.difficulty,
             due = excluded.due, last_review = excluded.last_review, reps = excluded.reps,
             lapses = excluded.lapses",
    )
    .bind(r.user)
    .bind(&r.card.note_id)
    .bind(&r.card.template_id)
    .bind(phase_str(r.after.phase))
    .bind(r.after.memory.map(|m| m.stability as f64))
    .bind(r.after.memory.map(|m| m.difficulty as f64))
    .bind(r.after.due)
    .bind(r.after.last_review)
    .bind(r.after.reps as i64)
    .bind(r.after.lapses as i64)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO review_log
             (user_id, note_id, template_id, rating, phase_before, reviewed_at, duration_ms, answer)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(r.user)
    .bind(&r.card.note_id)
    .bind(&r.card.template_id)
    .bind(r.rating as i64)
    .bind(phase_str(r.before.phase))
    .bind(r.at)
    .bind(r.duration_ms)
    .bind(r.answer)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// What the user has done since the start of the study day.
pub async fn today(
    pool: &SqlitePool,
    user: i64,
    now: DateTime<Utc>,
    day_start: DateTime<Utc>,
    day_end: DateTime<Utc>,
) -> ApiResult<Today> {
    let reviews_done: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM review_log WHERE user_id = ? AND reviewed_at >= ? AND phase_before = 'review'",
    )
    .bind(user)
    .bind(day_start)
    .fetch_one(pool)
    .await?;
    Ok(Today {
        now,
        day_end,
        reviews_done: reviews_done as usize,
    })
}
