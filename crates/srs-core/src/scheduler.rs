//! Spaced-repetition scheduling. [`Scheduler`] is the seam; [`FsrsScheduler`] is the default.

use chrono::{DateTime, Duration, Utc};
use fsrs::{FSRS, MemoryState, NextStates};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The learner's self-assessment after seeing the answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rating {
    Again = 1,
    Hard = 2,
    Good = 3,
    Easy = 4,
}

impl Rating {
    pub const ALL: [Rating; 4] = [Rating::Again, Rating::Hard, Rating::Good, Rating::Easy];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Never reviewed.
    New,
    /// Failed on first sight; shown again after a short delay.
    Learning,
    /// Graduated; scheduled in whole days.
    Review,
    /// Failed after graduating; shown again after a short delay.
    Relearning,
}

/// FSRS memory model for one card.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    pub stability: f32,
    pub difficulty: f32,
}

impl From<MemoryState> for Memory {
    fn from(m: MemoryState) -> Self {
        Self {
            stability: m.stability,
            difficulty: m.difficulty,
        }
    }
}

impl From<Memory> for MemoryState {
    fn from(m: Memory) -> Self {
        Self {
            stability: m.stability,
            difficulty: m.difficulty,
        }
    }
}

/// Everything the scheduler knows about one card for one user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardState {
    pub phase: Phase,
    pub memory: Option<Memory>,
    pub due: DateTime<Utc>,
    pub last_review: Option<DateTime<Utc>>,
    pub reps: u32,
    pub lapses: u32,
}

impl CardState {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            phase: Phase::New,
            memory: None,
            due: now,
            last_review: None,
            reps: 0,
            lapses: 0,
        }
    }

    pub fn is_new(&self) -> bool {
        self.phase == Phase::New
    }

    /// Learning and relearning cards are due within the day and are not bound to day cutoffs.
    pub fn is_learning(&self) -> bool {
        matches!(self.phase, Phase::Learning | Phase::Relearning)
    }
}

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("FSRS error: {0}")]
    Fsrs(#[from] fsrs::FSRSError),
}

pub trait Scheduler {
    /// The card's state after being answered with `rating` at `now`.
    fn review(
        &self,
        state: &CardState,
        rating: Rating,
        now: DateTime<Utc>,
    ) -> Result<CardState, SchedulerError>;

    /// The outcome of each rating, e.g. to label the answer buttons with their intervals.
    fn preview(
        &self,
        state: &CardState,
        now: DateTime<Utc>,
    ) -> Result<[(Rating, CardState); 4], SchedulerError> {
        let mut out = Rating::ALL.map(|r| (r, state.clone()));
        for (rating, next) in &mut out {
            *next = self.review(state, *rating, now)?;
        }
        Ok(out)
    }
}

/// FSRS-6 via the `fsrs` crate.
///
/// "Again" puts the card back into (re)learning and shows it again after `relearn_delay`.
/// Other ratings schedule it in whole days, with Hard < Good < Easy always strictly ordered
/// (so at the interval cap, Good is one day shorter than Easy).
#[derive(Debug, Clone)]
pub struct FsrsScheduler {
    fsrs: FSRS,
    pub desired_retention: f32,
    pub maximum_interval_days: u32,
    pub relearn_delay: Duration,
}

impl Default for FsrsScheduler {
    fn default() -> Self {
        Self::new(&[]).expect("default FSRS parameters are valid")
    }
}

impl FsrsScheduler {
    /// `parameters` may be empty to use the FSRS defaults, or a user's optimised weights.
    pub fn new(parameters: &[f32]) -> Result<Self, SchedulerError> {
        Ok(Self {
            fsrs: FSRS::new(parameters)?,
            desired_retention: 0.9,
            maximum_interval_days: 365 * 3,
            relearn_delay: Duration::minutes(10),
        })
    }

    fn next_states(&self, state: &CardState, now: DateTime<Utc>) -> Result<NextStates, SchedulerError> {
        let days_elapsed = state
            .last_review
            .map(|t| (now - t).num_days().max(0) as u32)
            .unwrap_or(0);
        Ok(self
            .fsrs
            .next_states(state.memory.map(Into::into), self.desired_retention, days_elapsed)?)
    }

    /// Whole-day intervals for Hard, Good, Easy, kept strictly increasing.
    fn day_intervals(&self, next: &NextStates) -> [u32; 3] {
        let max = self.maximum_interval_days.max(3);
        let days = |i: f32| (i.round() as u32).clamp(1, max);
        let good = days(next.good.interval);
        let hard = days(next.hard.interval).min(good);
        let good = good.max(hard + 1).min(max - 1);
        let hard = hard.min(good - 1).max(1);
        let easy = days(next.easy.interval).max(good + 1).min(max);
        [hard, good, easy]
    }
}

impl Scheduler for FsrsScheduler {
    fn review(
        &self,
        state: &CardState,
        rating: Rating,
        now: DateTime<Utc>,
    ) -> Result<CardState, SchedulerError> {
        let next = self.next_states(state, now)?;
        let [hard, good, easy] = self.day_intervals(&next);
        let (memory, phase, due, lapse) = match rating {
            Rating::Again => {
                let phase = match state.phase {
                    Phase::New | Phase::Learning => Phase::Learning,
                    Phase::Review | Phase::Relearning => Phase::Relearning,
                };
                (
                    next.again.memory,
                    phase,
                    now + self.relearn_delay,
                    state.phase == Phase::Review,
                )
            }
            Rating::Hard => (
                next.hard.memory,
                Phase::Review,
                now + Duration::days(hard.into()),
                false,
            ),
            Rating::Good => (
                next.good.memory,
                Phase::Review,
                now + Duration::days(good.into()),
                false,
            ),
            Rating::Easy => (
                next.easy.memory,
                Phase::Review,
                now + Duration::days(easy.into()),
                false,
            ),
        };
        Ok(CardState {
            phase,
            memory: Some(memory.into()),
            due,
            last_review: Some(now),
            reps: state.reps + 1,
            lapses: state.lapses + u32::from(lapse),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 23, 9, 0, 0).unwrap()
    }

    #[test]
    fn new_card_good_graduates_to_days() {
        let s = FsrsScheduler::default();
        let next = s.review(&CardState::new(t0()), Rating::Good, t0()).unwrap();
        assert_eq!(next.phase, Phase::Review);
        assert!(next.due >= t0() + Duration::days(1));
        assert_eq!(next.reps, 1);
        assert_eq!(next.lapses, 0);
        assert!(next.memory.is_some());
    }

    #[test]
    fn again_relearns_soon_and_counts_lapse_only_after_graduating() {
        let s = FsrsScheduler::default();
        let learning = s.review(&CardState::new(t0()), Rating::Again, t0()).unwrap();
        assert_eq!(learning.phase, Phase::Learning);
        assert_eq!(learning.due, t0() + Duration::minutes(10));
        assert_eq!(learning.lapses, 0);

        let graduated = s.review(&learning, Rating::Good, learning.due).unwrap();
        let later = graduated.due;
        let lapsed = s.review(&graduated, Rating::Again, later).unwrap();
        assert_eq!(lapsed.phase, Phase::Relearning);
        assert_eq!(lapsed.lapses, 1);
        assert!(lapsed.memory.unwrap().stability < graduated.memory.unwrap().stability);
    }

    #[test]
    fn preview_intervals_are_strictly_ordered() {
        let s = FsrsScheduler::default();
        let mut state = CardState::new(t0());
        let mut now = t0();
        // Walk a card through several successful reviews and check ordering at each step.
        for _ in 0..8 {
            let p = s.preview(&state, now).unwrap();
            let due = |i: usize| p[i].1.due;
            assert!(due(0) < due(1) && due(1) < due(2) && due(2) < due(3), "{p:?}");
            state = p[2].1.clone();
            now = state.due;
        }
    }

    #[test]
    fn successful_reviews_grow_intervals_until_capped() {
        let s = FsrsScheduler {
            maximum_interval_days: 60,
            ..FsrsScheduler::default()
        };
        let mut state = CardState::new(t0());
        let mut now = t0();
        let mut last_gap = Duration::zero();
        for _ in 0..12 {
            state = s.review(&state, Rating::Good, now).unwrap();
            let gap = state.due - now;
            assert!(gap >= last_gap);
            assert!(gap <= Duration::days(60));
            last_gap = gap;
            now = state.due;
        }
        // Good stops one day short of the cap so Easy can stay strictly longer.
        assert_eq!(last_gap, Duration::days(59));
    }

    #[test]
    fn higher_retention_means_shorter_intervals() {
        let relaxed = FsrsScheduler {
            desired_retention: 0.8,
            ..FsrsScheduler::default()
        };
        let strict = FsrsScheduler {
            desired_retention: 0.95,
            ..FsrsScheduler::default()
        };
        let s0 = CardState::new(t0());
        let a = relaxed.review(&s0, Rating::Good, t0()).unwrap();
        let a = relaxed.review(&a, Rating::Good, a.due).unwrap();
        let b = strict.review(&s0, Rating::Good, t0()).unwrap();
        let b = strict.review(&b, Rating::Good, b.due).unwrap();
        assert!(b.due - b.last_review.unwrap() < a.due - a.last_review.unwrap());
    }
}
