//! The review loop: get the next card, optionally check a typed answer, submit a rating.

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::State;
use chrono::Duration;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use srs_core::model::{AnswerMode, CardId, CardTemplate, NoteType};
use srs_core::queue::{QueueOptions, build_queue};
use srs_core::scheduler::{CardState, FsrsScheduler, Phase, Rating, Scheduler};
use srs_core::{Note, answer};

use crate::error::{ApiError, ApiResult};
use crate::{AppState, LOCAL_USER, store};

/// When nothing else is left, learning cards due within this window are shown early.
const LEARN_AHEAD: Duration = Duration::minutes(20);
const DISTRACTORS: usize = 3;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Counts {
    pub learning: usize,
    pub review: usize,
    pub new: usize,
}

/// Seconds until the card would be due again for each rating, to label the buttons.
#[derive(Debug, Serialize, Deserialize)]
pub struct Intervals {
    pub again: i64,
    pub hard: i64,
    pub good: i64,
    pub easy: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewCard {
    pub note_id: String,
    pub template_id: String,
    pub set_id: String,
    pub phase: Phase,
    pub note_type: NoteType,
    pub template: CardTemplate,
    pub fields: BTreeMap<String, serde_json::Value>,
    /// Options for multiple-choice cards, already shuffled.
    pub choices: Option<Vec<String>>,
    pub intervals: Intervals,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NextResponse {
    pub counts: Counts,
    pub card: Option<ReviewCard>,
}

async fn scheduler(state: &AppState) -> ApiResult<FsrsScheduler> {
    let settings = store::settings(&state.pool, LOCAL_USER).await?;
    Ok(FsrsScheduler::default().with_retention(settings.desired_retention))
}

pub async fn next(State(state): State<AppState>) -> ApiResult<Json<NextResponse>> {
    Ok(Json(next_card(&state).await?))
}

async fn next_card(state: &AppState) -> ApiResult<NextResponse> {
    let now = state.now();
    let (day_start, day_end) = state.study_day(now);
    let settings = store::settings(&state.pool, LOCAL_USER).await?;
    let candidates = store::candidates(&state.pool, LOCAL_USER, now).await?;
    let today = store::today(&state.pool, LOCAL_USER, now, day_start, day_end).await?;
    let options = QueueOptions {
        new_per_day: settings.new_per_day as usize,
        reviews_per_day: settings.reviews_per_day.map(|v| v as usize),
    };
    let queue = build_queue(&candidates, &options, &today);
    let mut counts = Counts {
        learning: queue.learning.len(),
        review: queue.review.len(),
        new: queue.new.len(),
    };

    let mut next = queue.ordered().into_iter().next();
    if next.is_none() {
        next = candidates
            .iter()
            .filter(|c| c.state.is_learning() && c.state.due <= now + LEARN_AHEAD)
            .min_by_key(|c| c.state.due)
            .map(|c| c.id.clone());
        counts.learning += usize::from(next.is_some());
    }
    let Some(id) = next else {
        return Ok(NextResponse { counts, card: None });
    };
    let candidate = candidates
        .iter()
        .find(|c| c.id == id)
        .expect("queued card is a candidate");
    let card = review_card(state, &id, &candidate.set_id, &candidate.state).await?;
    Ok(NextResponse {
        counts,
        card: Some(card),
    })
}

async fn review_card(
    state: &AppState,
    id: &CardId,
    set_id: &str,
    card_state: &CardState,
) -> ApiResult<ReviewCard> {
    let now = state.now();
    let stored = store::note(&state.pool, &id.note_id).await?;
    let note_type = store::note_type(&state.pool, &stored.note_type_id).await?;
    let template = note_type
        .template(&id.template_id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound(format!("template `{}`", id.template_id)))?;

    let choices = match template.mode {
        AnswerMode::MultipleChoice => Some(choices(state, set_id, &stored.note, &template).await?),
        _ => None,
    };

    let preview = scheduler(state).await?.preview(card_state, now)?;
    let secs = |i: usize| (preview[i].1.due - now).num_seconds();
    let intervals = Intervals {
        again: secs(0),
        hard: secs(1),
        good: secs(2),
        easy: secs(3),
    };

    Ok(ReviewCard {
        note_id: id.note_id.clone(),
        template_id: id.template_id.clone(),
        set_id: set_id.to_string(),
        phase: card_state.phase,
        note_type,
        template,
        fields: stored.note.fields,
        choices,
        intervals,
    })
}

/// The correct answer plus distractors: the same field's values from other notes in the set.
async fn choices(
    state: &AppState,
    set_id: &str,
    note: &Note,
    template: &CardTemplate,
) -> ApiResult<Vec<String>> {
    let field = &template.answer[0];
    let correct = note.text(field).unwrap_or_default();
    let mut others: Vec<String> = store::set_notes(&state.pool, set_id)
        .await?
        .into_iter()
        .filter(|n| n.status == "published")
        .filter_map(|n| n.note.text(field))
        .filter(|v| *v != correct)
        .collect();
    others.sort();
    others.dedup();
    let mut rng = rand::rng();
    others.shuffle(&mut rng);
    others.truncate(DISTRACTORS);
    others.push(correct);
    others.shuffle(&mut rng);
    Ok(others)
}

#[derive(Debug, Deserialize)]
pub struct CheckRequest {
    pub note_id: String,
    pub template_id: String,
    pub input: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckResponse {
    pub correct: bool,
    /// The answer fields' values, to show after answering.
    pub expected: BTreeMap<String, serde_json::Value>,
}

pub async fn check(
    State(state): State<AppState>,
    Json(req): Json<CheckRequest>,
) -> ApiResult<Json<CheckResponse>> {
    let stored = store::note(&state.pool, &req.note_id).await?;
    let note_type = store::note_type(&state.pool, &stored.note_type_id).await?;
    let template = note_type
        .template(&req.template_id)
        .ok_or_else(|| ApiError::NotFound(format!("template `{}`", req.template_id)))?;
    let expected = template
        .answer
        .iter()
        .filter_map(|f| Some((f.clone(), stored.note.get(f)?.clone())))
        .collect();
    Ok(Json(CheckResponse {
        correct: answer::check(template, &stored.note, &req.input),
        expected,
    }))
}

#[derive(Debug, Deserialize)]
pub struct AnswerRequest {
    pub note_id: String,
    pub template_id: String,
    pub rating: Rating,
    pub duration_ms: Option<i64>,
    /// What the learner typed or picked, kept in the review log.
    pub answer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnswerResponse {
    pub state: CardState,
    pub next: NextResponse,
}

pub async fn answer(
    State(state): State<AppState>,
    Json(req): Json<AnswerRequest>,
) -> ApiResult<Json<AnswerResponse>> {
    let id = CardId::new(req.note_id, req.template_id);
    if !store::card_exists(&state.pool, &id).await? {
        return Err(ApiError::NotFound(format!("card `{id}`")));
    }
    let now = state.now();
    let before = store::card_state(&state.pool, LOCAL_USER, &id, now).await?;
    let after = scheduler(&state).await?.review(&before, req.rating, now)?;
    store::record_review(
        &state.pool,
        store::ReviewRecord {
            user: LOCAL_USER,
            card: &id,
            rating: req.rating,
            before: &before,
            after: &after,
            at: now,
            duration_ms: req.duration_ms,
            answer: req.answer.as_deref(),
        },
    )
    .await?;
    Ok(Json(AnswerResponse {
        state: after,
        next: next_card(&state).await?,
    }))
}
