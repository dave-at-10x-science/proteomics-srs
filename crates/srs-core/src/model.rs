//! Content model: note types define fields, templates pair prompt fields with answer fields, and
//! every (note, template) pair is a card that the scheduler tracks.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// The kind of data a field holds. Controls how it is edited and rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    Markdown,
    Number,
    /// A SMILES string. Structures are drawn from it.
    Smiles,
    /// An image. Either uploaded media, or derived from another field (see [`FieldDef::from`]).
    Image,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default)]
    pub optional: bool,
    /// For derived images: the field this one is rendered from (e.g. `smiles`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// For structures: the depiction template that fixes the drawing's orientation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depiction_template: Option<String>,
}

/// A fixed 2D layout for a shared core, so every structure of a note type is drawn the same way.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepictionTemplate {
    pub smiles: String,
    /// One (x, y) per heavy atom of `smiles`, in atom order.
    pub coords: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerMode {
    /// Show the answer, the learner grades themselves.
    Flip,
    /// The learner types the answer and it is checked.
    Typed,
    /// Pick from the right answer plus distractors drawn from the same set.
    MultipleChoice,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardTemplate {
    pub id: String,
    pub prompt: Vec<String>,
    pub answer: Vec<String>,
    pub mode: AnswerMode,
    /// Fields whose values count as a correct typed answer. Defaults to `answer`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accept: Vec<String>,
    /// Allowed absolute error for numeric answers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f64>,
    #[serde(default = "default_true")]
    pub enabled_by_default: bool,
}

fn default_true() -> bool {
    true
}

impl CardTemplate {
    /// The fields whose values are accepted as a typed answer.
    pub fn accepted_fields(&self) -> &[String] {
        if self.accept.is_empty() {
            &self.answer
        } else {
            &self.accept
        }
    }

    /// Every field this template reads.
    pub fn referenced_fields(&self) -> impl Iterator<Item = &String> {
        self.prompt.iter().chain(&self.answer).chain(&self.accept)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteType {
    pub id: String,
    pub fields: Vec<FieldDef>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub depiction_templates: BTreeMap<String, DepictionTemplate>,
    pub templates: Vec<CardTemplate>,
}

impl NoteType {
    pub fn field(&self, name: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.name == name)
    }

    pub fn template(&self, id: &str) -> Option<&CardTemplate> {
        self.templates.iter().find(|t| t.id == id)
    }
}

/// One fact: a filled-in instance of a note type. Field values are kept as JSON so text and
/// numbers share one map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    #[serde(flatten)]
    pub fields: BTreeMap<String, serde_json::Value>,
}

impl Note {
    pub fn get(&self, field: &str) -> Option<&serde_json::Value> {
        self.fields.get(field).filter(|v| !v.is_null())
    }

    /// A field value as display text. Numbers are formatted as written.
    pub fn text(&self, field: &str) -> Option<String> {
        match self.get(field)? {
            serde_json::Value::String(s) => Some(s.clone()),
            other => Some(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StudySet {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A card is identified by its note and template. Stable across re-imports, so review history
/// survives content edits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CardId {
    pub note_id: String,
    pub template_id: String,
}

impl CardId {
    pub fn new(note_id: impl Into<String>, template_id: impl Into<String>) -> Self {
        Self {
            note_id: note_id.into(),
            template_id: template_id.into(),
        }
    }
}

impl fmt::Display for CardId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.note_id, self.template_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub id: CardId,
    pub set_id: String,
    pub enabled: bool,
}
