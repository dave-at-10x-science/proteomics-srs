//! Study mode: read through notes not yet studied, with every field shown. Finishing a batch puts
//! the notes' cards into review rotation.

use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult};
use crate::store::{self, StoredNote};
use crate::{AppState, LOCAL_USER};

const DEFAULT_BATCH: usize = 5;
const MAX_BATCH: usize = 100;

#[derive(Debug, Deserialize)]
pub struct BatchQuery {
    pub size: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct Batch {
    /// The next notes to study, in set order.
    pub notes: Vec<StoredNote>,
    /// How many notes are left to study in all, including this batch.
    pub remaining: usize,
}

pub async fn batch(State(state): State<AppState>, Query(q): Query<BatchQuery>) -> ApiResult<Json<Batch>> {
    let size = q.size.unwrap_or(DEFAULT_BATCH);
    if size == 0 || size > MAX_BATCH {
        return Err(ApiError::Invalid(vec![format!(
            "batch size must be 1 to {MAX_BATCH}"
        )]));
    }
    let mut notes = store::unstudied_notes(&state.pool, LOCAL_USER).await?;
    let remaining = notes.len();
    notes.truncate(size);
    Ok(Json(Batch { notes, remaining }))
}

#[derive(Debug, Deserialize)]
pub struct StudiedRequest {
    pub note_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StudiedResponse {
    /// Notes newly marked as studied (ones already studied are not counted again).
    pub added: usize,
}

/// Mark notes as studied, so their cards join review rounds.
pub async fn studied(
    State(state): State<AppState>,
    Json(req): Json<StudiedRequest>,
) -> ApiResult<Json<StudiedResponse>> {
    let added = store::mark_studied(&state.pool, LOCAL_USER, &req.note_ids, state.now()).await?;
    Ok(Json(StudiedResponse { added }))
}
