//! Loading and validating set files: one YAML document holding a set, its note type, and notes.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::{Card, CardId, FieldType, Note, NoteType, StudySet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetFile {
    pub set: StudySet,
    pub note_type: NoteType,
    pub notes: Vec<Note>,
}

#[derive(Debug, Error)]
pub enum ContentError {
    #[error("could not parse set file: {0}")]
    Parse(#[from] serde_yaml_ng::Error),
    #[error("set file is invalid:\n  - {}", .0.join("\n  - "))]
    Invalid(Vec<String>),
}

/// Problems with one note against its note type: unknown fields, missing required fields, and
/// values of the wrong kind.
pub fn validate_note(note_type: &NoteType, note: &Note) -> Vec<String> {
    let mut errors = Vec::new();
    if note.id.trim().is_empty() {
        errors.push("note id must not be empty".to_string());
    }
    for key in note.fields.keys() {
        if note_type.field(key).is_none() {
            errors.push(format!("note `{}` has unknown field `{key}`", note.id));
        }
    }
    for f in &note_type.fields {
        // Derived fields are rendered, not stored.
        if f.from.is_some() {
            continue;
        }
        match note.get(&f.name) {
            None if !f.optional => errors.push(format!(
                "note `{}` is missing required field `{}`",
                note.id, f.name
            )),
            Some(v) if f.field_type == FieldType::Number && !v.is_number() => {
                errors.push(format!("note `{}` field `{}` must be a number", note.id, f.name))
            }
            Some(v) if f.field_type != FieldType::Number && !v.is_string() => {
                errors.push(format!("note `{}` field `{}` must be text", note.id, f.name))
            }
            _ => {}
        }
    }
    errors
}

impl SetFile {
    /// Parse and validate a set file.
    pub fn from_yaml(src: &str) -> Result<Self, ContentError> {
        let file: SetFile = serde_yaml_ng::from_str(src)?;
        file.validate()?;
        Ok(file)
    }

    /// Check the file is internally consistent. Reports every problem at once.
    pub fn validate(&self) -> Result<(), ContentError> {
        let mut errors = Vec::new();
        let nt = &self.note_type;

        let mut field_names = HashSet::new();
        for f in &nt.fields {
            if !field_names.insert(f.name.as_str()) {
                errors.push(format!("field `{}` is declared twice", f.name));
            }
        }
        for f in &nt.fields {
            if let Some(src) = &f.from
                && nt.field(src).is_none()
            {
                errors.push(format!(
                    "field `{}` is derived from unknown field `{src}`",
                    f.name
                ));
            }
            if let Some(dt) = &f.depiction_template
                && !nt.depiction_templates.contains_key(dt)
            {
                errors.push(format!(
                    "field `{}` uses unknown depiction template `{dt}`",
                    f.name
                ));
            }
        }

        let mut template_ids = HashSet::new();
        for t in &nt.templates {
            if !template_ids.insert(t.id.as_str()) {
                errors.push(format!("template `{}` is declared twice", t.id));
            }
            if t.prompt.is_empty() || t.answer.is_empty() {
                errors.push(format!(
                    "template `{}` needs at least one prompt and one answer field",
                    t.id
                ));
            }
            for name in t.referenced_fields() {
                if nt.field(name).is_none() {
                    errors.push(format!("template `{}` references unknown field `{name}`", t.id));
                }
            }
        }

        let mut note_ids = HashSet::new();
        for note in &self.notes {
            if !note_ids.insert(note.id.as_str()) {
                errors.push(format!("note `{}` is declared twice", note.id));
            }
            errors.extend(validate_note(nt, note));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ContentError::Invalid(errors))
        }
    }

    /// Every card this file generates: one per note per template.
    pub fn cards(&self) -> Vec<Card> {
        self.notes
            .iter()
            .flat_map(|note| {
                self.note_type.templates.iter().map(move |t| Card {
                    id: CardId::new(&note.id, &t.id),
                    set_id: self.set.id.clone(),
                    enabled: t.enabled_by_default,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AMINO_ACIDS: &str = include_str!("../../../seed/amino-acids/amino-acids.yaml");

    #[test]
    fn amino_acid_seed_is_valid() {
        let file = SetFile::from_yaml(AMINO_ACIDS).expect("seed should load");
        assert_eq!(file.notes.len(), 20);
        assert!(file.set.default);

        let cards = file.cards();
        assert_eq!(cards.len(), 20 * file.note_type.templates.len());
        assert_eq!(cards.iter().filter(|c| c.enabled).count(), 100);
    }

    #[test]
    fn every_seed_set_is_valid() {
        let seed = concat!(env!("CARGO_MANIFEST_DIR"), "/../../seed");
        let mut checked = 0;
        for dir in std::fs::read_dir(seed).unwrap() {
            for f in std::fs::read_dir(dir.unwrap().path()).unwrap() {
                let path = f.unwrap().path();
                if path.extension().is_some_and(|e| e == "yaml") {
                    let src = std::fs::read_to_string(&path).unwrap();
                    if let Err(e) = SetFile::from_yaml(&src) {
                        panic!("{}: {e}", path.display());
                    }
                    checked += 1;
                }
            }
        }
        assert!(checked >= 2);
    }

    #[test]
    fn amino_acid_codes_are_unique() {
        let file = SetFile::from_yaml(AMINO_ACIDS).unwrap();
        for field in ["name", "three_letter", "one_letter", "smiles"] {
            let values: HashSet<_> = file.notes.iter().map(|n| n.text(field).unwrap()).collect();
            assert_eq!(values.len(), 20, "`{field}` values should be unique");
        }
    }

    #[test]
    fn reports_every_problem() {
        let src = r#"
set: {id: s, name: S}
note_type:
  id: T
  fields:
    - {name: name, type: text}
    - {name: mass, type: number}
  templates:
    - {id: a, prompt: [name], answer: [nope], mode: typed}
notes:
  - {id: n1, name: X, mass: "heavy"}
  - {id: n1, name: Y, mass: 1.0, extra: 2}
  - {id: n2, mass: 2.0}
"#;
        let ContentError::Invalid(errors) = SetFile::from_yaml(src).unwrap_err() else {
            panic!("expected validation errors");
        };
        let joined = errors.join("\n");
        assert!(joined.contains("unknown field `nope`"), "{joined}");
        assert!(joined.contains("`n1` is declared twice"), "{joined}");
        assert!(joined.contains("`mass` must be a number"), "{joined}");
        assert!(joined.contains("unknown field `extra`"), "{joined}");
        assert!(
            joined.contains("`n2` is missing required field `name`"),
            "{joined}"
        );
    }
}
