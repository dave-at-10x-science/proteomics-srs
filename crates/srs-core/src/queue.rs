//! Building the daily review queue.
//!
//! Order of precedence:
//! 1. Learning and relearning cards that are due now, oldest first. These are in progress, so
//!    they are never buried.
//! 2. Review cards due before the end of the study day, most overdue first.
//! 3. New cards, up to the daily limit, taken round-robin across sets.
//!
//! Siblings (cards from the same note) are buried: at most one card per note per day, counting
//! notes already reviewed earlier in the day.
//!
//! A study round ([`build_round`]) is a fixed slice of that queue: due cards first, then new cards
//! to fill it up to the chosen size.

use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::CardId;
use crate::scheduler::CardState;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: CardId,
    pub set_id: String,
    pub state: CardState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueOptions {
    pub new_per_day: usize,
    /// Cap on review cards per day. `None` means no cap.
    pub reviews_per_day: Option<usize>,
}

impl Default for QueueOptions {
    fn default() -> Self {
        Self {
            new_per_day: 10,
            reviews_per_day: None,
        }
    }
}

/// What has already happened today, supplied by the caller from the review log.
#[derive(Debug, Clone, PartialEq)]
pub struct Today {
    pub now: DateTime<Utc>,
    /// End of the learner's study day (their local day rollover, e.g. 4am).
    pub day_end: DateTime<Utc>,
    pub new_introduced: usize,
    pub reviews_done: usize,
    /// Notes with a card already answered today, for sibling burying.
    pub notes_seen: HashSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DailyQueue {
    pub learning: Vec<CardId>,
    pub review: Vec<CardId>,
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

/// Pick a round of at most `size` cards: learning and review cards that are due come first, then
/// new cards fill whatever room is left. The result is in presentation order.
pub fn build_round(
    candidates: &[Candidate],
    options: &QueueOptions,
    today: &Today,
    size: usize,
) -> Vec<CardId> {
    let mut queue = build_queue(candidates, options, today);
    queue.learning.truncate(size);
    queue.review.truncate(size - queue.learning.len());
    queue
        .new
        .truncate(size - queue.learning.len() - queue.review.len());
    queue.ordered()
}

pub fn build_queue(candidates: &[Candidate], options: &QueueOptions, today: &Today) -> DailyQueue {
    let mut queue = DailyQueue::default();
    let mut notes_taken: HashSet<&str> = today.notes_seen.iter().map(String::as_str).collect();

    let mut learning: Vec<_> = candidates
        .iter()
        .filter(|c| c.state.is_learning() && c.state.due <= today.now)
        .collect();
    learning.sort_by(|a, b| (a.state.due, &a.id).cmp(&(b.state.due, &b.id)));
    for c in learning {
        notes_taken.insert(&c.id.note_id);
        queue.learning.push(c.id.clone());
    }

    let mut reviews: Vec<_> = candidates
        .iter()
        .filter(|c| !c.state.is_new() && !c.state.is_learning() && c.state.due <= today.day_end)
        .collect();
    reviews.sort_by(|a, b| (a.state.due, &a.id).cmp(&(b.state.due, &b.id)));
    let review_room = options
        .reviews_per_day
        .map_or(usize::MAX, |cap| cap.saturating_sub(today.reviews_done));
    for c in reviews {
        if queue.review.len() >= review_room {
            break;
        }
        if notes_taken.insert(&c.id.note_id) {
            queue.review.push(c.id.clone());
        }
    }

    // New cards: keep each set's own order, and alternate between sets.
    let mut by_set: BTreeMap<&str, Vec<&Candidate>> = BTreeMap::new();
    for c in candidates.iter().filter(|c| c.state.is_new()) {
        by_set.entry(&c.set_id).or_default().push(c);
    }
    let new_room = options.new_per_day.saturating_sub(today.new_introduced);
    let mut cursors: Vec<_> = by_set.into_values().map(|v| v.into_iter()).collect();
    while queue.new.len() < new_room && !cursors.is_empty() {
        cursors.retain_mut(|cursor| {
            if queue.new.len() >= new_room {
                return true;
            }
            for c in cursor.by_ref() {
                if notes_taken.insert(&c.id.note_id) {
                    queue.new.push(c.id.clone());
                    return true;
                }
            }
            false
        });
    }

    queue
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
            new_introduced: 0,
            reviews_done: 0,
            notes_seen: HashSet::new(),
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
        }
    }

    fn new(set: &str, note: &str, tmpl: &str) -> Candidate {
        card(set, note, tmpl, Phase::New, Duration::zero())
    }

    #[test]
    fn learning_then_due_reviews_then_new() {
        let cards = vec![
            new("aa", "gly", "name"),
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
    fn new_card_limit_counts_what_was_already_introduced() {
        let cards: Vec<_> = (0..20).map(|i| new("aa", &format!("n{i}"), "t")).collect();
        let opts = QueueOptions {
            new_per_day: 5,
            reviews_per_day: None,
        };
        assert_eq!(build_queue(&cards, &opts, &today()).new.len(), 5);

        let t = Today {
            new_introduced: 3,
            ..today()
        };
        assert_eq!(build_queue(&cards, &opts, &t).new.len(), 2);
    }

    #[test]
    fn siblings_are_buried() {
        let cards = vec![
            card("aa", "leu", "structure-to-name", Phase::Review, Duration::zero()),
            card("aa", "leu", "name-to-one-letter", Phase::Review, Duration::zero()),
            new("aa", "leu", "one-letter-to-name"),
            new("aa", "ile", "structure-to-name"),
            new("aa", "ile", "one-letter-to-name"),
            new("aa", "met", "structure-to-name"),
        ];
        let mut t = today();
        t.notes_seen.insert("met".into());
        let q = build_queue(&cards, &QueueOptions::default(), &t);
        assert_eq!(q.review.len(), 1);
        assert_eq!(q.new, vec![CardId::new("ile", "structure-to-name")]);
    }

    #[test]
    fn new_cards_alternate_between_sets() {
        let cards = vec![
            new("aa", "a1", "t"),
            new("aa", "a2", "t"),
            new("aa", "a3", "t"),
            new("vocab", "v1", "t"),
        ];
        let opts = QueueOptions {
            new_per_day: 3,
            reviews_per_day: None,
        };
        let q = build_queue(&cards, &opts, &today());
        let notes: Vec<_> = q.new.iter().map(|c| c.note_id.as_str()).collect();
        assert_eq!(notes, ["a1", "v1", "a2"]);
    }

    #[test]
    fn review_cap_applies() {
        let cards: Vec<_> = (0..10)
            .map(|i| card("aa", &format!("n{i}"), "t", Phase::Review, Duration::zero()))
            .collect();
        let opts = QueueOptions {
            new_per_day: 0,
            reviews_per_day: Some(6),
        };
        let t = Today {
            reviews_done: 2,
            ..today()
        };
        assert_eq!(build_queue(&cards, &opts, &t).review.len(), 4);
    }

    #[test]
    fn rounds_take_due_cards_first_then_fill_with_new() {
        let mut cards: Vec<_> = (0..3)
            .map(|i| card("aa", &format!("r{i}"), "t", Phase::Review, -Duration::hours(1)))
            .collect();
        cards.push(card("aa", "l", "t", Phase::Learning, -Duration::minutes(1)));
        cards.extend((0..10).map(|i| new("aa", &format!("n{i}"), "t")));
        let opts = QueueOptions::default();

        let round = build_round(&cards, &opts, &today(), 6);
        let notes: Vec<_> = round.iter().map(|c| c.note_id.as_str()).collect();
        assert_eq!(notes.len(), 6);
        assert_eq!(notes[0], "l");
        assert_eq!(notes.iter().filter(|n| n.starts_with('r')).count(), 3);
        assert_eq!(notes.iter().filter(|n| n.starts_with('n')).count(), 2);

        // A small round is all due cards, no new ones.
        let round = build_round(&cards, &opts, &today(), 2);
        assert!(round.iter().all(|c| !c.note_id.starts_with('n')));

        // A big round is limited by what is due plus the daily new-card limit.
        assert_eq!(build_round(&cards, &opts, &today(), 50).len(), 14);
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
}
