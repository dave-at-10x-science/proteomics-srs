//! Core logic for the proteomics spaced-repetition system.
//!
//! This crate has no I/O so it can run natively on the server and, later, compiled to WASM in
//! the browser. It covers:
//!
//! - [`model`]: note types, templates, notes, and the cards generated from them.
//! - [`content`]: loading and validating set files such as `seed/amino-acids/amino-acids.yaml`.
//! - [`scheduler`]: the [`Scheduler`] trait and its FSRS implementation.
//! - [`queue`]: building the daily review queue.
//! - [`answer`]: checking typed answers.

pub mod answer;
pub mod content;
pub mod model;
pub mod queue;
pub mod scheduler;

pub use model::{Card, CardId, CardTemplate, Note, NoteType, StudySet};
pub use scheduler::{CardState, FsrsScheduler, Phase, Rating, Scheduler};
