# Proteomics SRS: requirements and design proposal

Status: draft v4 (2026-10-23). SMILES-first structures with one consistent orientation; runs locally for a single user for now.

## 1. Goal

A daily study tool for proteomics and mass spectrometry. It serves a short daily queue of due cards on a phone, scheduled by spaced repetition. Content is built in the app, with AI help drafting it. It runs locally for one user first; colleagues and remote hosting come later. A matching-pairs memory game gives a fun way to practise concepts you have already been introduced to.

## 2. Decisions so far

| Question | Decision |
|---|---|
| Platform | Phone-first web app (SPA, installable as a PWA). Desktop browser works too. No Tauri for now. |
| Users | Just Dave for now. The schema keeps a `user_id` on per-user tables so colleagues can be added later. |
| Authoring | In the app, with an AI assistant that drafts cards for a human to approve. |
| Structures | SMILES first: structures are drawn from SMILES in one consistent orientation. Uploaded images only for things SMILES can't describe (proteins, antibodies, instruments). |
| Memory game | Practice only. Uses cards already introduced. Never changes the schedule. |
| Hosting | Runs locally on Dave's machine for now. Remote hosting is deferred until it works well. |
| Sign-in | None while it is local and single-user. Google sign-in on the shared domain later. |
| Default set | The 20 standard amino acids. |
| Topics | Open-ended: amino acids, peptides, proteins, antibodies, enzymes (e.g. trypsin for digestion), MS instruments and lingo, and more. Two broad kinds: *identify from a picture* and *vocabulary with plain-language definitions*. |

## 3. Requirements

### Must have (v1)
- Single local user, no sign-in. Content and review progress kept in separate tables so multiple users can be added later.
- A content library of sets. The user picks which sets to study.
- Daily review queue on a phone: due cards plus a capped number of new cards per day. Large touch targets, one-handed use, works on a flaky connection.
- Scheduling engine written in Rust (FSRS).
- Typed card fields: text, Markdown with plain-language definitions, SMILES structures (rendered automatically), uploaded images, and numbers with units.
- Mix-and-match prompts: one fact generates several card directions.
- Answer modes: flip and self-grade, type the answer (with accepted alternates), and multiple choice with distractors from the same set.
- In-app authoring: create and edit note types, notes, and sets. Paste SMILES and see the structure; upload images where SMILES doesn't apply.
- AI-assisted authoring: give a topic, a list, or pasted text, and get back draft cards to review, edit, and approve.
- Matching-pairs memory game over cards the player has already seen.

### Should have (later)
- Statistics: retention, streaks, upcoming load.
- Full offline review with sync when back online.
- Colleagues: Google sign-in on the shared domain, per-user progress, remote hosting.
- More games.

### Out of scope for now
- Multiple users, remote hosting, public sign-up, native app-store builds.

## 4. Architecture

```
 Phone / desktop browser                 Dave's machine (one Rust binary)
 ┌────────────────────────┐   HTTP/JSON  ┌──────────────────────────────┐
 │ Nuxt SPA (PWA)         │ ───────────▶ │ Axum server                  │
 │ review, game, editor   │              │  serves the SPA + /api       │
 │ SMILES → SVG (RDKit.js)│              │  srs-core (FSRS, queues)     │
 └────────────────────────┘              │  AI drafting → Claude API    │
                                         │  SQLite file + media folder  │
                                         └──────────────────────────────┘
```

- **One local binary:** `cargo run` starts an Axum server that serves the built Nuxt SPA and the JSON API from the same port. Data lives in a single SQLite file plus a media folder, so backup is copying a folder.
- **Frontend:** Nuxt with `ssr: false`, installable as a PWA so it sits on the phone home screen like an app.
- **Engine (`srs-core`):** pure Rust, no I/O. The server uses it natively, and it can later be compiled to WASM for offline review.
- **Database:** SQLite via `sqlx`. The queries stay close to portable SQL so moving to Postgres for multi-user hosting later is a small change.
- **Phone access while local:** the phone talks to the server on Dave's machine. On the same Wi-Fi, use the machine's LAN address. To get HTTPS (needed to install the PWA) and access away from home, run it behind Tailscale (`tailscale serve`), which gives a private HTTPS URL with no public hosting.
- **Sign-in:** none for now; there is one built-in user. The API still takes a user id everywhere so adding Google sign-in later doesn't change the data model.
- **Later (multi-user, hosted):** Cloud Run + Postgres on GCP, or Vercel for the SPA, with Google sign-in limited to the shared domain. Not designed in detail until the local version works well.

## 5. Scheduling engine

**FSRS, via the `fsrs` crate**, which is the maintained reference implementation in Rust and is what Anki now uses. It typically needs 20–30% fewer reviews than SM-2 for the same retention. Each user can set a target retention (default 90%), and their FSRS weights can later be optimised from their own review history. We wrap it behind a `Scheduler` trait so another scheduler can be swapped in.

Engine scope:
- Given a card's memory state and a rating, return the new state and next due date.
- Queue building: due cards, a new-card limit per day, interleaving across sets, and sibling burying (don't show "structure → name" and "name → structure" for the same amino acid on the same day).
- An append-only review log per user, so schedules can be recomputed.

## 6. Data model

**Content** (one copy, shared by all users once there is more than one):
- **Note type**: a schema of fields, such as `AminoAcid` (name, 3-letter, 1-letter, structure SVG, SMILES, side-chain class, residue mass, pI), `Term` (term, plain-language definition, technical definition, image, example, related terms), or `Enzyme` (name, cleavage rule, use in sample prep, image).
- **Card template**: prompt fields → answer fields plus an answer mode. Adding a template creates a new card for every note of that type.
- **Note**: one fact. **Card**: one note × one template.
- **Set**: a named, tagged collection of notes (e.g. "20 amino acids", "Proteases", "MS lingo").
- **Media**: uploaded images stored by content hash, for things SMILES can't describe.
- Every note records who created and last edited it, and whether it is `draft` (e.g. AI-generated, not yet approved) or `published`. Edit rights are wide open; roles can be added later.
- A set can be flagged `default`. The amino acids set is the default, and new users get default sets automatically.

**Per-user state:**
- **Subscription**: user follows a set.
- **Card state**: FSRS stability, difficulty, due date, lapses, and whether the card has been introduced. Keyed by (user, card).
- **Review log**: (user, card, rating, time, duration).
- **Game log**: kept separately and never read by the scheduler.

Main tables (all per-user tables carry `user_id`, even with one user): `users`, `note_types`, `templates`, `notes`, `cards`, `sets`, `set_notes`, `media`, `subscriptions`, `card_state`, `review_log`, `game_sessions`.

Two kinds of content fit naturally:
- **Identify from a picture**: amino acids, protein folds, antibody structure, instrument diagrams. Prompt is an image or rendered SMILES. Answer is a name, code, or property.
- **Vocabulary and lingo**: prompt is the term, answer is the plain-language definition (or the reverse), with an optional technical definition shown after answering.

## 7. AI-assisted authoring

A "Draft with AI" screen in the editor:

1. Pick a note type and a set, then give a source: a topic ("common proteases used in bottom-up proteomics"), a list of terms, or pasted text from a paper or protocol.
2. The server calls the Claude API with the note type's field schema and asks for structured output (JSON matching the fields, one object per note), including SMILES where it applies.
3. The drafts appear as `draft` notes in a review table. You edit, approve, or discard each one. Only approved notes become cards.
4. The AI can also fill gaps in existing notes ("add plain-language definitions to these 30 terms") or suggest related terms you haven't covered.

Guardrails: AI output never goes straight to study, every note shows who approved it, and SMILES is validated by parsing before saving. Scientific accuracy still depends on human review, so the approval step is the important part.

Needs: an Anthropic API key in a local `.env` file.

## 8. Structures: SMILES first

- **SMILES is the primary source for structures.** A note stores the SMILES string, and the app draws the structure from it. No hand-drawing needed.
- **Rendering:** RDKit.js (RDKit compiled to WASM) draws an SVG in the browser. The server uses RDKit (or the same WASM build) to validate SMILES before saving, check stereochemistry, and compute masses to cross-check entered values.
- **Consistent orientation:** each note type can define a depiction template, a fixed 2D layout for a shared core that every structure is aligned to. For amino acids the template is the backbone laid out left to right (H₂N–Cα–COOH) with the side chain pointing up from Cα. That makes amino acids directly comparable side by side. RDKit.js supports this via aligned coordinates (`generate_aligned_coords`), so it works in the app as well as in seed rendering.
- **Fallback images:** things SMILES can't sensibly describe (proteins, antibodies, enzymes, instrument diagrams) use uploaded images (SVG/PNG), possibly rendered from PDB structures later.
- The AI drafter supplies SMILES for small molecules, and every SMILES is validated before it reaches a card.

## 9. Memory game

Classic concentration: a grid of face-down tiles. Flip two, and keep them if they match.

- Boards are drawn only from cards the player has already been introduced to in review.
- A board comes from a set plus a pairing of two fields, e.g. structure ↔ 1-letter code, enzyme ↔ cleavage rule, or term ↔ plain-language definition.
- Difficulty comes from board size (6–20 pairs), deliberately similar items (Leu/Ile, Asp/Asn, trypsin/Lys-C), and harder pairings.
- Optional "prove it" step: after a match, type the name or pick a property before the pair clears.
- **No impact on spaced practice.** Results go to `game_sessions` for fun stats (best time, fewest flips) and are never read by the scheduler.
- Phone-friendly: tap to flip, with a grid sized to the screen (e.g. 4×4 or 4×5 on a phone).

## 10. First card set: the 20 amino acids

Drafted in `seed/amino-acids/amino-acids.yaml`. It is the default set and has one `AminoAcid` note per amino acid with:
name, 3-letter and 1-letter codes, isomeric SMILES (L-form), side-chain class, monoisotopic residue mass, pI, and a short proteomics note where useful (Leu/Ile isobaric, trypsin cleaves after K/R, Met oxidation, Cys alkylation, etc.).

Templates generated from it: structure → name (typed, accepts name or either code), 1-letter → name, name → 1-letter, 3-letter → name, structure → side-chain class, and name → residue mass (off by default). That gives 100 cards to start and 120 with mass turned on.

All SMILES were checked with RDKit: computed residue masses match the table to 0.00001 Da and stereocentres are correct (L-forms, Thr 2S,3R, Ile 2S,3S). Structures are rendered from SMILES with the backbone template from section 8, so all 20 have the same orientation (see `seed/amino-acids/preview.png`). The SVGs in `seed/amino-acids/media/` are only previews; the app renders from SMILES at runtime.

## 11. Repo layout

```
proteomics-srs/
  Cargo.toml              # workspace
  crates/
    srs-core/             # FSRS wrapper, queues, domain types (no I/O, wasm-safe)
    srs-server/           # Axum API + static SPA, SQLite (sqlx), AI drafting
  web/                    # Nuxt SPA / PWA
  migrations/             # SQL migrations
  seed/                   # starter sets (amino acids, proteases, MS lingo) as YAML
  docs/
```

## 12. Milestones

1. **Core engine**: `srs-core` with FSRS, domain types, queue building, unit tests.
2. **Local server**: Axum, SQLite schema and migrations, content and review endpoints, amino-acid seed import. One `cargo run` to start.
3. **Phone review UI**: Nuxt PWA with the daily review screen (flip, typed, multiple choice) and SMILES rendering with aligned orientation. Phone access over LAN or Tailscale.
4. **Editor**: note types, notes, sets, SMILES entry with live preview, image upload.
5. **AI drafting**: draft-and-approve flow.
6. **Memory game**.
7. Stats, offline review via WASM. Then multi-user and remote hosting.

## 13. Open questions

None blocking. The next step is milestone 1, the `srs-core` engine crate.
