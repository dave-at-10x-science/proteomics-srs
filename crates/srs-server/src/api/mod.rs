//! JSON API. Every handler acts as [`crate::LOCAL_USER`] until accounts exist.

mod content;
mod review;

use axum::Router;
use axum::routing::{get, post, put};
use serde_json::{Value, json};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/health",
            get(|| async { axum::Json(json!({ "ok": true })) as axum::Json<Value> }),
        )
        .route("/settings", get(content::get_settings).put(content::put_settings))
        .route("/sets", get(content::list_sets).post(content::create_set))
        .route("/sets/{id}/subscription", put(content::set_subscription))
        .route("/sets/{id}/notes", get(content::list_set_notes))
        .route("/note-types", get(content::list_note_types))
        .route("/note-types/{id}", get(content::get_note_type))
        .route("/notes", post(content::create_note))
        .route(
            "/notes/{id}",
            get(content::get_note)
                .put(content::update_note)
                .delete(content::delete_note),
        )
        .route("/review/next", get(review::next))
        .route("/review/check", post(review::check))
        .route("/review/answer", post(review::answer))
}

/// Lowercase ASCII slug for generated ids.
pub(crate) fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}
