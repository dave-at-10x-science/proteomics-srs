//! Sets, note types, notes, and settings.

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use srs_core::Note;
use srs_core::content::validate_note;
use srs_core::model::NoteType;

use super::slug;
use crate::error::{ApiError, ApiResult};
use crate::store::{self, Settings, StoredNote};
use crate::{AppState, LOCAL_USER};

pub async fn get_settings(State(state): State<AppState>) -> ApiResult<Json<Settings>> {
    Ok(Json(store::settings(&state.pool, LOCAL_USER).await?))
}

pub async fn put_settings(
    State(state): State<AppState>,
    Json(s): Json<Settings>,
) -> ApiResult<Json<Settings>> {
    let mut errors = Vec::new();
    if !(0.7..=0.99).contains(&s.desired_retention) {
        errors.push("desired_retention must be between 0.70 and 0.99".to_string());
    }
    if s.new_per_day > 200 {
        errors.push("new_per_day must be at most 200".to_string());
    }
    if !errors.is_empty() {
        return Err(ApiError::Invalid(errors));
    }
    store::save_settings(&state.pool, LOCAL_USER, &s).await?;
    Ok(Json(s))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetSummary {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub tags: Vec<String>,
    pub subscribed: bool,
    pub note_count: i64,
    pub card_count: i64,
}

pub async fn list_sets(State(state): State<AppState>) -> ApiResult<Json<Vec<SetSummary>>> {
    let rows = sqlx::query(
        "SELECT s.id, s.name, s.is_default, s.tags,
                EXISTS (SELECT 1 FROM subscriptions WHERE user_id = ?1 AND set_id = s.id) AS subscribed,
                (SELECT COUNT(*) FROM set_notes sn WHERE sn.set_id = s.id) AS note_count,
                (SELECT COUNT(*) FROM set_notes sn JOIN cards c ON c.note_id = sn.note_id
                 WHERE sn.set_id = s.id AND c.enabled = 1) AS card_count
         FROM sets s ORDER BY s.is_default DESC, s.name",
    )
    .bind(LOCAL_USER)
    .fetch_all(&state.pool)
    .await?;
    let sets = rows
        .iter()
        .map(|r| {
            Ok(SetSummary {
                id: r.get("id"),
                name: r.get("name"),
                is_default: r.get("is_default"),
                tags: serde_json::from_str(r.get("tags"))?,
                subscribed: r.get("subscribed"),
                note_count: r.get("note_count"),
                card_count: r.get("card_count"),
            })
        })
        .collect::<ApiResult<_>>()?;
    Ok(Json(sets))
}

#[derive(Debug, Deserialize)]
pub struct NewSet {
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

pub async fn create_set(
    State(state): State<AppState>,
    Json(req): Json<NewSet>,
) -> ApiResult<(StatusCode, Json<SetSummary>)> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(ApiError::invalid("name must not be empty"));
    }
    let id = req.id.unwrap_or_else(|| slug(name));
    if id.is_empty() {
        return Err(ApiError::invalid(
            "could not make an id from the name; pass one explicitly",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let inserted = sqlx::query("INSERT INTO sets (id, name, tags) VALUES (?, ?, ?) ON CONFLICT DO NOTHING")
        .bind(&id)
        .bind(name)
        .bind(serde_json::to_string(&req.tags)?)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if inserted == 0 {
        return Err(ApiError::invalid(format!("set `{id}` already exists")));
    }
    // You almost always want to study a set you just made.
    sqlx::query("INSERT INTO subscriptions (user_id, set_id) VALUES (?, ?)")
        .bind(LOCAL_USER)
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let summary = SetSummary {
        id,
        name: name.to_string(),
        is_default: false,
        tags: req.tags,
        subscribed: true,
        note_count: 0,
        card_count: 0,
    };
    Ok((StatusCode::CREATED, Json(summary)))
}

#[derive(Debug, Deserialize)]
pub struct Subscription {
    pub subscribed: bool,
}

pub async fn set_subscription(
    State(state): State<AppState>,
    Path(set_id): Path<String>,
    Json(req): Json<Subscription>,
) -> ApiResult<StatusCode> {
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sets WHERE id = ?")
        .bind(&set_id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(ApiError::NotFound(format!("set `{set_id}`")));
    }
    let sql = if req.subscribed {
        "INSERT INTO subscriptions (user_id, set_id) VALUES (?, ?) ON CONFLICT DO NOTHING"
    } else {
        "DELETE FROM subscriptions WHERE user_id = ? AND set_id = ?"
    };
    sqlx::query(sql)
        .bind(LOCAL_USER)
        .bind(&set_id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_set_notes(
    State(state): State<AppState>,
    Path(set_id): Path<String>,
) -> ApiResult<Json<Vec<StoredNote>>> {
    Ok(Json(store::set_notes(&state.pool, &set_id).await?))
}

pub async fn list_note_types(State(state): State<AppState>) -> ApiResult<Json<Vec<NoteType>>> {
    Ok(Json(store::note_types(&state.pool).await?))
}

pub async fn get_note_type(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<NoteType>> {
    Ok(Json(store::note_type(&state.pool, &id).await?))
}

pub async fn get_note(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<StoredNote>> {
    Ok(Json(store::note(&state.pool, &id).await?))
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Draft,
    #[default]
    Published,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Draft => "draft",
            Status::Published => "published",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NewNote {
    pub set_id: String,
    pub note_type_id: String,
    pub id: Option<String>,
    pub fields: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub status: Status,
}

pub async fn create_note(
    State(state): State<AppState>,
    Json(req): Json<NewNote>,
) -> ApiResult<(StatusCode, Json<StoredNote>)> {
    let note_type = store::note_type(&state.pool, &req.note_type_id).await?;
    let set_exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sets WHERE id = ?")
        .bind(&req.set_id)
        .fetch_one(&state.pool)
        .await?;
    if set_exists == 0 {
        return Err(ApiError::NotFound(format!("set `{}`", req.set_id)));
    }
    let id = match req.id {
        Some(id) => id,
        None => {
            // Derive from the first field, e.g. "Trypsin" -> "enzyme-trypsin".
            let first = note_type
                .fields
                .first()
                .and_then(|f| req.fields.get(&f.name))
                .and_then(|v| v.as_str());
            format!("{}-{}", slug(&note_type.id), slug(first.unwrap_or_default()))
        }
    };
    let note = Note {
        id,
        fields: req.fields,
    };
    let errors = validate_note(&note_type, &note);
    if !errors.is_empty() {
        return Err(ApiError::Invalid(errors));
    }

    let mut tx = state.pool.begin().await?;
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes WHERE id = ?")
        .bind(&note.id)
        .fetch_one(&mut *tx)
        .await?;
    if exists > 0 {
        return Err(ApiError::invalid(format!("note `{}` already exists", note.id)));
    }
    store::upsert_note(&mut tx, &note_type, &note, req.status.as_str(), state.now()).await?;
    store::add_to_set(&mut tx, &req.set_id, &note.id).await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(store::note(&state.pool, &note.id).await?),
    ))
}

#[derive(Debug, Deserialize)]
pub struct NoteUpdate {
    pub fields: BTreeMap<String, serde_json::Value>,
    pub status: Option<Status>,
}

pub async fn update_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<NoteUpdate>,
) -> ApiResult<Json<StoredNote>> {
    let existing = store::note(&state.pool, &id).await?;
    let note_type = store::note_type(&state.pool, &existing.note_type_id).await?;
    let note = Note {
        id: id.clone(),
        fields: req.fields,
    };
    let errors = validate_note(&note_type, &note);
    if !errors.is_empty() {
        return Err(ApiError::Invalid(errors));
    }
    let status = req
        .status
        .map_or(existing.status.clone(), |s| s.as_str().to_string());
    let mut tx = state.pool.begin().await?;
    store::upsert_note(&mut tx, &note_type, &note, &status, state.now()).await?;
    tx.commit().await?;
    Ok(Json(store::note(&state.pool, &id).await?))
}

pub async fn delete_note(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<StatusCode> {
    let deleted = sqlx::query("DELETE FROM notes WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("note `{id}`")));
    }
    Ok(StatusCode::NO_CONTENT)
}
