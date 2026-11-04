//! Study mode: read through notes not yet studied, with every field shown. Finishing a batch puts
//! the notes' cards into review rotation.

use axum::Json;
use axum::extract::{Query, State};
use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult};
use crate::store::{self, StoredNote, StudyOrder};
use crate::{AppState, LOCAL_USER};

const DEFAULT_BATCH: usize = 5;
const MAX_BATCH: usize = 100;

#[derive(Debug, Deserialize)]
pub struct BatchQuery {
    pub size: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct Batch {
    /// The next notes to study, following the learner's chosen order.
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
    let notes = store::unstudied_notes(&state.pool, LOCAL_USER).await?;
    let settings = store::settings(&state.pool, LOCAL_USER).await?;
    Ok(Json(select_batch(
        notes,
        settings.study_order,
        size,
        &mut rand::rng(),
    )))
}

fn select_batch(mut notes: Vec<StoredNote>, order: StudyOrder, size: usize, rng: &mut impl Rng) -> Batch {
    let remaining = notes.len();
    if order == StudyOrder::Random {
        // Shuffle across all selected sets before limiting the batch, not just within its first set.
        notes.shuffle(rng);
    }
    notes.truncate(size);
    Batch { notes, remaining }
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

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashSet};

    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use srs_core::Note;

    use super::*;

    fn two_sets() -> Vec<StoredNote> {
        ["a", "b"]
            .into_iter()
            .flat_map(|set| {
                (0..10).map(move |i| StoredNote {
                    note: Note {
                        id: format!("{set}-{i}"),
                        fields: BTreeMap::new(),
                    },
                    note_type_id: "Concept".into(),
                    status: "published".into(),
                })
            })
            .collect()
    }

    #[test]
    fn random_batches_mix_the_full_pool_before_taking_a_batch() {
        let batch = select_batch(two_sets(), StudyOrder::Random, 8, &mut StdRng::seed_from_u64(42));
        assert_eq!(batch.remaining, 20);
        assert_eq!(batch.notes.len(), 8);
        let ids: HashSet<_> = batch.notes.iter().map(|note| note.note.id.as_str()).collect();
        assert_eq!(ids.len(), 8);
        assert!(ids.iter().any(|id| id.starts_with("a-")));
        assert!(ids.iter().any(|id| id.starts_with("b-")));
    }

    #[test]
    fn set_order_keeps_the_concepts_in_their_existing_order() {
        let batch = select_batch(
            two_sets(),
            StudyOrder::SetOrder,
            3,
            &mut StdRng::seed_from_u64(42),
        );
        assert_eq!(batch.remaining, 20);
        assert_eq!(
            batch
                .notes
                .iter()
                .map(|note| note.note.id.as_str())
                .collect::<Vec<_>>(),
            ["a-0", "a-1", "a-2"]
        );
    }
}
