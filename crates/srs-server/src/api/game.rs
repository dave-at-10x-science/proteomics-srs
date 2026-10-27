//! Matching-pairs memory game over notes the player has already been introduced to.
//!
//! Practice only: nothing here reads or writes the review schedule.

use std::collections::{BTreeMap, HashSet};

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use srs_core::model::NoteType;

use crate::error::{ApiError, ApiResult};
use crate::{AppState, LOCAL_USER, store};

const MIN_PAIRS: usize = 3;
const MAX_PAIRS: usize = 12;

/// Two fields to match, e.g. structure <-> name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pairing {
    pub a: String,
    pub b: String,
    pub label: String,
}

/// Pairings offered for a note type: each template's first prompt and answer field, deduplicated
/// regardless of direction.
pub fn pairings(note_type: &NoteType) -> Vec<Pairing> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for t in &note_type.templates {
        let (Some(a), Some(b)) = (t.prompt.first(), t.answer.first()) else {
            continue;
        };
        if a == b {
            continue;
        }
        let key = if a < b {
            (a.clone(), b.clone())
        } else {
            (b.clone(), a.clone())
        };
        if !seen.insert(key) {
            continue;
        }
        let label = |f: &str| {
            note_type
                .field(f)
                .and_then(|d| d.label.clone())
                .unwrap_or_else(|| {
                    let mut s = f.replace('_', " ");
                    s[..1].make_ascii_uppercase();
                    s
                })
        };
        out.push(Pairing {
            a: a.clone(),
            b: b.clone(),
            label: format!("{} ↔ {}", label(a), label(b)),
        });
    }
    out
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameOption {
    pub set_id: String,
    pub set_name: String,
    /// Notes in the set the player has studied at least once.
    pub introduced: usize,
    pub pairings: Vec<Pairing>,
}

/// Sets that have enough introduced notes to play, with their pairings.
pub async fn options(State(state): State<AppState>) -> ApiResult<Json<Vec<GameOption>>> {
    let rows = sqlx::query(
        "SELECT s.id, s.name, n.note_type_id, COUNT(DISTINCT n.id) AS introduced
         FROM sets s
         JOIN set_notes sn ON sn.set_id = s.id
         JOIN notes n ON n.id = sn.note_id AND n.status = 'published'
         WHERE EXISTS (SELECT 1 FROM studied_notes st WHERE st.user_id = ?1 AND st.note_id = n.id)
         GROUP BY s.id, n.note_type_id
         ORDER BY s.is_default DESC, s.name",
    )
    .bind(LOCAL_USER)
    .fetch_all(&state.pool)
    .await?;
    let mut out = Vec::new();
    for r in rows {
        let note_type = store::note_type(&state.pool, r.get("note_type_id")).await?;
        out.push(GameOption {
            set_id: r.get("id"),
            set_name: r.get("name"),
            introduced: r.get::<i64, _>("introduced") as usize,
            pairings: pairings(&note_type),
        });
    }
    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
pub struct BoardQuery {
    pub set: String,
    pub a: String,
    pub b: String,
    pub pairs: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tile {
    /// Position-independent id; the two tiles of a pair share `note_id`.
    pub id: String,
    pub note_id: String,
    pub field: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Board {
    pub set_id: String,
    pub note_type: NoteType,
    /// Field values per note, for rendering tiles (structures are drawn from SMILES).
    pub notes: BTreeMap<String, BTreeMap<String, serde_json::Value>>,
    pub tiles: Vec<Tile>,
}

pub async fn board(State(state): State<AppState>, Query(q): Query<BoardQuery>) -> ApiResult<Json<Board>> {
    let want = q.pairs.unwrap_or(8).clamp(MIN_PAIRS, MAX_PAIRS);
    let introduced: HashSet<String> = sqlx::query_scalar(
        "SELECT st.note_id FROM studied_notes st
         JOIN set_notes sn ON sn.note_id = st.note_id AND sn.set_id = ?2
         WHERE st.user_id = ?1",
    )
    .bind(LOCAL_USER)
    .bind(&q.set)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .collect();

    let mut notes: Vec<_> = store::set_notes(&state.pool, &q.set)
        .await?
        .into_iter()
        .filter(|n| n.status == "published" && introduced.contains(&n.note.id))
        .collect();
    let Some(first) = notes.first() else {
        return Err(ApiError::invalid(
            "study some cards in this set first; the game only uses notes you've seen",
        ));
    };
    let note_type = store::note_type(&state.pool, &first.note_type_id).await?;
    if !pairings(&note_type)
        .iter()
        .any(|p| (p.a == q.a && p.b == q.b) || (p.a == q.b && p.b == q.a))
    {
        return Err(ApiError::invalid(format!(
            "`{}` ↔ `{}` is not a pairing for this set",
            q.a, q.b
        )));
    }

    // The value a tile shows: derived structures are identified by their source (SMILES).
    let shown = |field: &str| {
        let src = note_type
            .field(field)
            .and_then(|f| f.from.clone())
            .unwrap_or_else(|| field.to_string());
        move |n: &store::StoredNote| n.note.text(&src)
    };
    let (show_a, show_b) = (shown(&q.a), shown(&q.b));

    // Pick notes so no two tiles on the same side look alike, or matches would be ambiguous.
    notes.shuffle(&mut rand::rng());
    let (mut used_a, mut used_b) = (HashSet::new(), HashSet::new());
    let mut chosen = Vec::new();
    for n in notes {
        if chosen.len() == want {
            break;
        }
        let (Some(va), Some(vb)) = (show_a(&n), show_b(&n)) else {
            continue;
        };
        if n.note_type_id == note_type.id && used_a.insert(va) && used_b.insert(vb) {
            chosen.push(n);
        }
    }
    if chosen.len() < MIN_PAIRS {
        return Err(ApiError::invalid(format!(
            "need at least {MIN_PAIRS} distinct notes you've studied for this pairing; found {}",
            chosen.len()
        )));
    }

    let mut tiles = Vec::new();
    for n in &chosen {
        for (side, field) in [("a", &q.a), ("b", &q.b)] {
            tiles.push(Tile {
                id: format!("{}:{side}", n.note.id),
                note_id: n.note.id.clone(),
                field: field.clone(),
            });
        }
    }
    tiles.shuffle(&mut rand::rng());

    // Only send the fields tiles need, plus any field they're derived from.
    let needed: HashSet<&str> = [&q.a, &q.b]
        .into_iter()
        .flat_map(|f| {
            [
                Some(f.as_str()),
                note_type.field(f).and_then(|d| d.from.as_deref()),
            ]
        })
        .flatten()
        .collect();
    let notes = chosen
        .into_iter()
        .map(|n| {
            let fields = n
                .note
                .fields
                .into_iter()
                .filter(|(k, _)| needed.contains(k.as_str()))
                .collect();
            (n.note.id, fields)
        })
        .collect();

    Ok(Json(Board {
        set_id: q.set,
        note_type,
        notes,
        tiles,
    }))
}

#[derive(Debug, Deserialize)]
pub struct GameResult {
    pub set_id: String,
    pub a: String,
    pub b: String,
    pub pairs: u32,
    pub flips: u32,
    pub duration_ms: u64,
    pub completed: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Best {
    pub games: i64,
    pub best_ms: Option<i64>,
    pub fewest_flips: Option<i64>,
}

/// Record a finished (or abandoned) game. Returns the personal bests for that board shape.
pub async fn record(
    State(state): State<AppState>,
    Json(r): Json<GameResult>,
) -> ApiResult<(StatusCode, Json<Best>)> {
    let pairing = format!("{}:{}", r.a, r.b);
    sqlx::query(
        "INSERT INTO game_sessions (user_id, set_id, pairing, pairs, flips, duration_ms, completed, played_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(LOCAL_USER)
    .bind(&r.set_id)
    .bind(&pairing)
    .bind(r.pairs as i64)
    .bind(r.flips as i64)
    .bind(r.duration_ms as i64)
    .bind(r.completed)
    .bind(state.now())
    .execute(&state.pool)
    .await?;
    let row = sqlx::query(
        "SELECT COUNT(*) AS games,
                MIN(CASE WHEN completed THEN duration_ms END) AS best_ms,
                MIN(CASE WHEN completed THEN flips END) AS fewest_flips
         FROM game_sessions WHERE user_id = ? AND set_id = ? AND pairing = ? AND pairs = ?",
    )
    .bind(LOCAL_USER)
    .bind(&r.set_id)
    .bind(&pairing)
    .bind(r.pairs as i64)
    .fetch_one(&state.pool)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(Best {
            games: row.get("games"),
            best_ms: row.get("best_ms"),
            fewest_flips: row.get("fewest_flips"),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amino_acid_pairings_are_deduplicated() {
        let file = srs_core::content::SetFile::from_yaml(include_str!(
            "../../../../seed/amino-acids/amino-acids.yaml"
        ))
        .unwrap();
        let p = pairings(&file.note_type);
        let pairs: Vec<_> = p.iter().map(|p| (p.a.as_str(), p.b.as_str())).collect();
        // one-letter -> name and name -> one-letter collapse into one pairing.
        assert_eq!(
            pairs,
            [
                ("structure", "name"),
                ("structure", "three_letter"),
                ("structure", "one_letter"),
                ("one_letter", "name"),
                ("three_letter", "name"),
                ("structure", "side_chain_class"),
                ("name", "residue_mass_mono"),
            ]
        );
        assert_eq!(p[0].label, "Structure ↔ Name");
        assert_eq!(p[3].label, "One-letter code ↔ Name");
    }
}
