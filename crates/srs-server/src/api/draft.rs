//! Drafting notes with Claude. Drafts land in the set with status `draft`, so nothing is studied
//! until it has been reviewed and published.

use std::collections::{BTreeMap, HashSet};

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use srs_core::Note;
use srs_core::answer::normalize;
use srs_core::content::validate_note;
use srs_core::model::{FieldDef, FieldType, NoteType};

use super::slug;
use crate::claude::ClaudeError;
use crate::error::{ApiError, ApiResult};
use crate::store::{self, StoredNote};
use crate::{AppState, LOCAL_USER};

pub const MAX_COUNT: u32 = 30;

#[derive(Debug, Serialize)]
pub struct Status {
    pub available: bool,
    pub model: Option<String>,
}

pub async fn status(State(state): State<AppState>) -> Json<Status> {
    Json(Status {
        available: state.claude.is_some(),
        model: state.claude.as_ref().map(|c| c.model.clone()),
    })
}

#[derive(Debug, Deserialize)]
pub struct DraftRequest {
    /// Defaults to the note type already used in the set.
    pub note_type_id: Option<String>,
    /// A topic, a list of terms, or pasted text to draw from.
    pub source: String,
    #[serde(default = "default_count")]
    pub count: u32,
}

fn default_count() -> u32 {
    10
}

#[derive(Debug, Serialize)]
pub struct Skipped {
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct DraftResult {
    pub created: Vec<StoredNote>,
    pub skipped: Vec<Skipped>,
}

pub async fn draft(
    State(state): State<AppState>,
    Path(set_id): Path<String>,
    Json(req): Json<DraftRequest>,
) -> ApiResult<Json<DraftResult>> {
    let claude = state.claude.clone().ok_or_else(|| {
        ApiError::Unavailable("AI drafting is off. Start the server with ANTHROPIC_API_KEY set.".into())
    })?;
    let source = req.source.trim();
    if source.is_empty() {
        return Err(ApiError::invalid(
            "describe a topic or paste some text to draft from",
        ));
    }
    if !(1..=MAX_COUNT).contains(&req.count) {
        return Err(ApiError::invalid(format!(
            "count must be between 1 and {MAX_COUNT}"
        )));
    }
    let set_name: String = sqlx::query_scalar("SELECT name FROM sets WHERE id = ?")
        .bind(&set_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("set `{set_id}`")))?;
    let existing = store::set_notes(&state.pool, &set_id).await?;
    let note_type_id = match (req.note_type_id, existing.first()) {
        (Some(id), _) => id,
        (None, Some(n)) => n.note_type_id.clone(),
        (None, None) => return Err(ApiError::invalid("choose a note type for an empty set")),
    };
    let note_type = store::note_type(&state.pool, &note_type_id).await?;
    let titles: Vec<String> = existing.iter().map(|n| title(&note_type, &n.note)).collect();

    let reply = claude
        .structured(
            &system_prompt(&note_type, &set_name, &titles),
            &format!("Draft up to {} notes from this:\n\n{source}", req.count),
            schema(&note_type),
        )
        .await
        .map_err(|e| match e {
            ClaudeError::Refused | ClaudeError::Truncated => ApiError::invalid(e.to_string()),
            ClaudeError::Api { .. } => ApiError::Upstream(e.to_string()),
            ClaudeError::Other(e) => ApiError::Internal(e),
        })?;

    let drafted = reply["notes"].as_array().cloned().unwrap_or_default();
    let mut result = DraftResult {
        created: Vec::new(),
        skipped: Vec::new(),
    };
    // Duplicates are caught by title across the whole library, not only by id, since
    // hand-written seed ids don't follow the generated scheme.
    let mut known: HashSet<String> = HashSet::new();
    for fields in sqlx::query_scalar::<_, String>("SELECT fields FROM notes WHERE note_type_id = ?")
        .bind(&note_type.id)
        .fetch_all(&state.pool)
        .await?
    {
        let note = Note {
            id: String::new(),
            fields: serde_json::from_str(&fields)?,
        };
        known.insert(normalize(&title(&note_type, &note)));
    }

    let mut tx = state.pool.begin().await?;
    for fields in drafted.into_iter().take(req.count as usize) {
        let note = to_note(&note_type, fields);
        let title = title(&note_type, &note);
        let errors = validate_note(&note_type, &note);
        if !errors.is_empty() {
            result.skipped.push(Skipped {
                title,
                reason: errors.join("; "),
            });
            continue;
        }
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes WHERE id = ?")
            .bind(&note.id)
            .fetch_one(&mut *tx)
            .await?;
        if exists > 0 || !known.insert(normalize(&title)) {
            result.skipped.push(Skipped {
                title,
                reason: "already in your library".into(),
            });
            continue;
        }
        store::upsert_note(&mut tx, &note_type, &note, "draft", state.now()).await?;
        store::add_to_set(&mut tx, &set_id, &note.id).await?;
        result.created.push(store::note(&mut *tx, &note.id).await?);
    }
    tx.commit().await?;
    tracing::info!(
        set = %set_id,
        created = result.created.len(),
        skipped = result.skipped.len(),
        "drafted notes"
    );
    Ok(Json(result))
}

#[derive(Debug, Serialize)]
pub struct Published {
    pub published: u64,
}

/// Publish every draft in the set, so its cards enter the study queue.
pub async fn publish_drafts(
    State(state): State<AppState>,
    Path(set_id): Path<String>,
) -> ApiResult<Json<Published>> {
    let published = sqlx::query(
        "UPDATE notes SET status = 'published'
         WHERE status = 'draft' AND id IN (SELECT note_id FROM set_notes WHERE set_id = ?)",
    )
    .bind(&set_id)
    .execute(&state.pool)
    .await?
    .rows_affected();
    // Publishing is the moment you've decided to study the set.
    if published > 0 {
        sqlx::query("INSERT INTO subscriptions (user_id, set_id) VALUES (?, ?) ON CONFLICT DO NOTHING")
            .bind(LOCAL_USER)
            .bind(&set_id)
            .execute(&state.pool)
            .await?;
    }
    Ok(Json(Published { published }))
}

/// Fields Claude fills in. Images are uploaded or rendered, never drafted.
fn draftable(note_type: &NoteType) -> impl Iterator<Item = &FieldDef> {
    note_type
        .fields
        .iter()
        .filter(|f| f.from.is_none() && f.field_type != FieldType::Image)
}

/// The note's display title: its first stored field.
fn title(note_type: &NoteType, note: &Note) -> String {
    draftable(note_type)
        .next()
        .and_then(|f| note.text(&f.name))
        .unwrap_or_default()
}

/// JSON schema for `{"notes": [...]}`. Structured outputs need every property listed as
/// required, so optional fields are nullable instead.
pub fn schema(note_type: &NoteType) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for f in draftable(note_type) {
        let base = if f.field_type == FieldType::Number {
            "number"
        } else {
            "string"
        };
        let ty = if f.optional {
            json!([base, "null"])
        } else {
            json!(base)
        };
        let mut prop = json!({ "type": ty });
        if let Some(d) = describe(f) {
            prop["description"] = json!(d);
        }
        properties.insert(f.name.clone(), prop);
        required.push(json!(f.name));
    }
    json!({
        "type": "object",
        "properties": {
            "notes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": properties,
                    "required": required,
                    "additionalProperties": false,
                },
            },
        },
        "required": ["notes"],
        "additionalProperties": false,
    })
}

fn describe(f: &FieldDef) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(label) = &f.label {
        parts.push(label.clone());
    }
    match f.field_type {
        FieldType::Markdown => parts.push("Markdown, one to three sentences".into()),
        FieldType::Smiles => parts.push("SMILES string with stereochemistry, neutral form".into()),
        _ => {}
    }
    if let Some(unit) = &f.unit {
        parts.push(format!("in {unit}"));
    }
    (!parts.is_empty()).then(|| parts.join(". "))
}

fn system_prompt(note_type: &NoteType, set_name: &str, existing: &[String]) -> String {
    let fields: Vec<String> = draftable(note_type)
        .map(|f| {
            let mut line = format!("- `{}`", f.name);
            if let Some(d) = describe(f) {
                line.push_str(&format!(": {d}"));
            }
            if f.optional {
                line.push_str(" (optional: null when it would not help)");
            }
            line
        })
        .collect();
    let existing = if existing.is_empty() {
        "The set is empty so far.".to_string()
    } else {
        format!(
            "The set already has these, so do not repeat them: {}.",
            existing.join(", ")
        )
    };
    format!(
        "You draft flashcard notes for a spaced repetition app. The learner is a software \
         engineer who is new to proteomics and mass spectrometry. Every note will be reviewed \
         by the learner before it is studied.\n\n\
         The notes go into the set \"{set_name}\" and use the note type `{note_type}`, with these \
         fields:\n{fields}\n\n\
         {existing}\n\n\
         Write plain-language explanations first, and keep technical detail for fields meant \
         for it. Each note should test one idea. Be accurate: leave out an item rather than \
         guess at a fact, number, or structure you are not sure of. Use standard names and \
         spellings, since typed answers are checked against them.",
        note_type = note_type.id,
        fields = fields.join("\n"),
    )
}

/// Turn one drafted object into a note, dropping nulls and blank strings, with the same id
/// scheme as notes made in the editor.
fn to_note(note_type: &NoteType, fields: Value) -> Note {
    let fields: BTreeMap<String, Value> = match fields {
        Value::Object(map) => map
            .into_iter()
            .filter(|(_, v)| !v.is_null() && v.as_str().is_none_or(|s| !s.trim().is_empty()))
            .map(|(k, v)| match v {
                Value::String(s) => (k, Value::String(s.trim().to_string())),
                v => (k, v),
            })
            .collect(),
        _ => BTreeMap::new(),
    };
    let mut note = Note {
        id: String::new(),
        fields,
    };
    note.id = format!("{}-{}", slug(&note_type.id), slug(&title(note_type, &note)));
    note
}

#[cfg(test)]
mod tests {
    use super::*;
    use srs_core::content::SetFile;

    fn vocabulary() -> NoteType {
        let src = include_str!("../../../../seed/ms-vocabulary/ms-vocabulary.yaml");
        SetFile::from_yaml(src).unwrap().note_type
    }

    #[test]
    fn schema_covers_stored_fields_and_nullable_optionals() {
        let s = schema(&vocabulary());
        let item = &s["properties"]["notes"]["items"];
        assert_eq!(item["additionalProperties"], false);
        assert_eq!(
            item["required"],
            json!(["term", "definition", "technical", "example"])
        );
        assert_eq!(item["properties"]["term"]["type"], "string");
        assert_eq!(item["properties"]["technical"]["type"], json!(["string", "null"]));
        assert!(item["properties"].get("image").is_none());
    }

    #[test]
    fn drafted_object_becomes_a_note() {
        let nt = vocabulary();
        let note = to_note(
            &nt,
            json!({"term": " Peptide spectrum match ", "definition": "A match.", "technical": null, "example": ""}),
        );
        assert_eq!(note.id, "term-peptide-spectrum-match");
        assert_eq!(note.text("term").as_deref(), Some("Peptide spectrum match"));
        assert!(note.get("technical").is_none());
        assert!(note.get("example").is_none());
        assert!(validate_note(&nt, &note).is_empty());
    }
}
