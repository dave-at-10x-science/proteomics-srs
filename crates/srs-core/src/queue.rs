//! Building the review queue.
//!
//! A card enters the rotation once its note has been studied (read through in study mode). From
//! then on it is due by its own schedule:
//! 1. Learning and relearning cards that are due now, oldest first.
//! 2. Review cards due before the end of the study day, most overdue first.
//! 3. Studied cards not yet reviewed, in set order.
//!
//! Every card is scheduled on its own: cards from the same note (say an amino acid's name and its
//! one-letter code) come up whenever each is due, together or apart.
//!
//! A round ([`build_round`]) is a fixed list drawn from the queue. When more cards are due than
//! fit, the round is a random pick of them.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::CardId;
use crate::scheduler::CardState;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: CardId,
    pub set_id: String,
    pub state: CardState,
    /// Whether the note has been studied. New cards stay out of the queue until it has.
    pub studied: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct QueueOptions {
    /// Cap on review cards per day. `None` means no cap.
    pub reviews_per_day: Option<usize>,
}

/// What has already happened today, supplied by the caller from the review log.
#[derive(Debug, Clone, PartialEq)]
pub struct Today {
    pub now: DateTime<Utc>,
    /// End of the learner's study day (their local day rollover, e.g. 4am).
    pub day_end: DateTime<Utc>,
    pub reviews_done: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DailyQueue {
    pub learning: Vec<CardId>,
    pub review: Vec<CardId>,
    /// Studied cards waiting for their first review.
    pub new: Vec<CardId>,
}

impl DailyQueue {
    pub fn len(&self) -> usize {
        self.learning.len() + self.review.len() + self.new.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The order to present cards in: learning first, then reviews with new cards spread evenly
    /// among them, so a session doesn't end in a block of unfamiliar cards.
    pub fn ordered(&self) -> Vec<CardId> {
        let mut out = self.learning.clone();
        let (r, n) = (self.review.len(), self.new.len());
        let (mut ri, mut ni) = (0, 0);
        while ri < r || ni < n {
            // Take a new card when it is "behind" its share of the mixed sequence.
            if ni < n && (ri == r || ni * (r + n) <= (ri + ni) * n) {
                out.push(self.new[ni].clone());
                ni += 1;
            } else {
                out.push(self.review[ri].clone());
                ri += 1;
            }
        }
        out
    }
}

/// Pick a round of at most `size` cards from everything due. If more are due than fit, the round
/// is a random pick; either way the order is random. `random(n)` must return an index below `n`.
pub fn build_round(
    candidates: &[Candidate],
    options: &QueueOptions,
    today: &Today,
    size: usize,
    mut random: impl FnMut(usize) -> usize,
) -> Vec<CardId> {
    let mut pool = build_queue(candidates, options, today).ordered();
    // Partial Fisher-Yates: the first `size` entries end up a uniform random sample.
    let take = size.min(pool.len());
    for i in 0..take {
        let j = i + random(pool.len() - i);
        pool.swap(i, j);
    }
    pool.truncate(take);
    spread_siblings(pool)
}

/// Reorder so cards from the same note are not back to back where that can be avoided: the
/// answer to one would give away the next.
fn spread_siblings(mut cards: Vec<CardId>) -> Vec<CardId> {
    for i in 1..cards.len() {
        if cards[i].note_id != cards[i - 1].note_id {
            continue;
        }
        let prev = cards[i - 1].note_id.clone();
        if let Some(j) = (i + 1..cards.len()).find(|&j| cards[j].note_id != prev) {
            let card = cards.remove(j);
            cards.insert(i, card);
        }
    }
    cards
}

pub fn build_queue(candidates: &[Candidate], options: &QueueOptions, today: &Today) -> DailyQueue {
    let mut learning: Vec<_> = candidates
        .iter()
        .filter(|c| c.state.is_learning() && c.state.due <= today.now)
        .collect();
    learning.sort_by(|a, b| (a.state.due, &a.id).cmp(&(b.state.due, &b.id)));

    let mut reviews: Vec<_> = candidates
        .iter()
        .filter(|c| !c.state.is_new() && !c.state.is_learning() && c.state.due <= today.day_end)
        .collect();
    reviews.sort_by(|a, b| (a.state.due, &a.id).cmp(&(b.state.due, &b.id)));
    let review_room = options
        .reviews_per_day
        .map_or(usize::MAX, |cap| cap.saturating_sub(today.reviews_done));

    let ids = |cs: Vec<&Candidate>| cs.into_iter().map(|c| c.id.clone()).collect();
    DailyQueue {
        learning: ids(learning),
        review: ids(reviews.into_iter().take(review_room).collect()),
        new: ids(candidates
            .iter()
            .filter(|c| c.state.is_new() && c.studied)
            .collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::Phase;
    use chrono::{Duration, TimeZone};

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 23, 9, 0, 0).unwrap()
    }

    fn today() -> Today {
        Today {
            now: now(),
            day_end: now() + Duration::hours(19),
            reviews_done: 0,
        }
    }

    fn card(set: &str, note: &str, tmpl: &str, phase: Phase, due_in: Duration) -> Candidate {
        let mut state = CardState::new(now());
        state.phase = phase;
        state.due = now() + due_in;
        Candidate {
            id: CardId::new(note, tmpl),
            set_id: set.into(),
            state,
            studied: true,
        }
    }

    fn new(set: &str, note: &str, tmpl: &str) -> Candidate {
        card(set, note, tmpl, Phase::New, Duration::zero())
    }

    fn unstudied(set: &str, note: &str, tmpl: &str) -> Candidate {
        Candidate {
            studied: false,
            ..new(set, note, tmpl)
        }
    }

    /// A deterministic stand-in for a random number generator.
    fn first(_: usize) -> usize {
        0
    }

    #[test]
    fn learning_then_due_reviews_then_studied_new() {
        let cards = vec![
            new("aa", "gly", "name"),
            unstudied("aa", "met", "name"),
            card("aa", "ala", "name", Phase::Review, Duration::hours(3)),
            card("aa", "ser", "name", Phase::Review, Duration::days(2)), // not due today
            card("aa", "pro", "name", Phase::Learning, -Duration::minutes(1)),
            card("aa", "val", "name", Phase::Relearning, Duration::minutes(5)), // not due yet
        ];
        let q = build_queue(&cards, &QueueOptions::default(), &today());
        assert_eq!(q.learning, vec![CardId::new("pro", "name")]);
        assert_eq!(q.review, vec![CardId::new("ala", "name")]);
        assert_eq!(q.new, vec![CardId::new("gly", "name")]);
    }

    #[test]
    fn overdue_reviews_come_first() {
        let cards = vec![
            card("aa", "a", "t", Phase::Review, -Duration::days(1)),
            card("aa", "b", "t", Phase::Review, -Duration::days(5)),
            card("aa", "c", "t", Phase::Review, Duration::hours(1)),
        ];
        let q = build_queue(&cards, &QueueOptions::default(), &today());
        let notes: Vec<_> = q.review.iter().map(|c| c.note_id.as_str()).collect();
        assert_eq!(notes, ["b", "a", "c"]);
    }

    #[test]
    fn siblings_are_each_scheduled_on_their_own() {
        let cards = vec![
            card("aa", "leu", "structure-to-name", Phase::Review, Duration::zero()),
            card(
                "aa",
                "leu",
                "structure-to-one-letter",
                Phase::Review,
                Duration::zero(),
            ),
            card(
                "aa",
                "leu",
                "structure-to-three-letter",
                Phase::Review,
                Duration::days(3),
            ),
            new("aa", "ile", "structure-to-name"),
            new("aa", "ile", "structure-to-one-letter"),
        ];
        let q = build_queue(&cards, &QueueOptions::default(), &today());
        assert_eq!(q.review.len(), 2);
        assert_eq!(q.new.len(), 2);
    }

    #[test]
    fn review_cap_applies() {
        let cards: Vec<_> = (0..10)
            .map(|i| card("aa", &format!("n{i}"), "t", Phase::Review, Duration::zero()))
            .collect();
        let opts = QueueOptions {
            reviews_per_day: Some(6),
        };
        let t = Today {
            reviews_done: 2,
            ..today()
        };
        assert_eq!(build_queue(&cards, &opts, &t).review.len(), 4);
    }

    #[test]
    fn ordered_spreads_new_cards_among_reviews() {
        let id = |s: &str| CardId::new(s, "t");
        let q = DailyQueue {
            learning: vec![id("l")],
            review: vec![id("r1"), id("r2"), id("r3"), id("r4")],
            new: vec![id("n1"), id("n2")],
        };
        let order: Vec<_> = q.ordered().into_iter().map(|c| c.note_id).collect();
        assert_eq!(order, ["l", "n1", "r1", "r2", "n2", "r3", "r4"]);
    }

    #[test]
    fn rounds_take_everything_due_when_it_fits() {
        let mut cards: Vec<_> = (0..3)
            .map(|i| card("aa", &format!("r{i}"), "t", Phase::Review, -Duration::hours(1)))
            .collect();
        cards.push(new("aa", "n", "t"));
        cards.push(unstudied("aa", "u", "t"));
        let round = build_round(&cards, &QueueOptions::default(), &today(), 10, first);
        let mut notes: Vec<_> = round.iter().map(|c| c.note_id.as_str()).collect();
        notes.sort();
        assert_eq!(notes, ["n", "r0", "r1", "r2"]);
    }

    #[test]
    fn rounds_pick_randomly_when_more_are_due_than_fit() {
        let cards: Vec<_> = (0..10)
            .map(|i| {
                card(
                    "aa",
                    &format!("r{i}"),
                    "t",
                    Phase::Review,
                    -Duration::hours(10 - i),
                )
            })
            .collect();
        // The pick depends on the random draws: always drawing the last remaining card swaps it
        // forward each time.
        let round = build_round(&cards, &QueueOptions::default(), &today(), 3, |n| n - 1);
        let notes: Vec<_> = round.iter().map(|c| c.note_id.as_str()).collect();
        assert_eq!(notes, ["r9", "r0", "r1"]);
        let round = build_round(&cards, &QueueOptions::default(), &today(), 3, first);
        let notes: Vec<_> = round.iter().map(|c| c.note_id.as_str()).collect();
        assert_eq!(notes, ["r0", "r1", "r2"]);
    }

    #[test]
    fn rounds_keep_siblings_apart_when_they_can() {
        let cards = vec![
            card("aa", "leu", "a", Phase::Review, -Duration::hours(3)),
            card("aa", "leu", "b", Phase::Review, -Duration::hours(2)),
            card("aa", "ile", "a", Phase::Review, -Duration::hours(1)),
        ];
        let round = build_round(&cards, &QueueOptions::default(), &today(), 10, first);
        let notes: Vec<_> = round.iter().map(|c| c.note_id.as_str()).collect();
        assert_eq!(notes, ["leu", "ile", "leu"]);
    }
}
