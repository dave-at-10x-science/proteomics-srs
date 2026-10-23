# proteomics-srs
Spaced repetition system for learning about proteomic and mass spectrometry

A daily study tool: a short queue of due cards on your phone, scheduled with FSRS, plus a
matching-pairs memory game for practice. Structures are drawn from SMILES. See
[docs/design-proposal.md](docs/design-proposal.md) for the full design.

## Layout

```
crates/srs-core/   Scheduling (FSRS), daily queue, content model, answer checking. No I/O; builds for WASM.
crates/srs-server/ Local server: JSON API + serves the web app. SQLite in data/.
seed/              Starter sets as YAML. The 20 amino acids are the default set.
tools/             Content helpers (SMILES validation and structure previews).
docs/              Design notes.
```

Coming next: `web/` (Nuxt SPA, installable as a PWA).

## Running it

```sh
cargo run -p srs-server
```

This creates `data/srs.sqlite`, imports everything under `seed/` (the amino acids are
subscribed by default), and listens on `http://0.0.0.0:8080` so a phone on the same Wi-Fi can
reach it at your machine's LAN address. Back up by copying `data/`.

Seed import runs on every start. Note types and sets are updated from the files, new notes are
added, and notes you've edited in the app are left alone.

| Variable | Default | |
|---|---|---|
| `SRS_BIND` | `0.0.0.0:8080` | Address to listen on |
| `SRS_DATA_DIR` | `data` | Where the database lives |
| `SRS_SEED_DIR` | `seed` | Set files to import |
| `SRS_WEB_DIR` | `web/.output/public` | Built web app to serve |
| `SRS_DAY_ROLLOVER_HOUR` | `4` | Local hour a new study day starts |

### API

| Method | Path | |
|---|---|---|
| GET | `/api/review/next` | Today's counts and the next card, with everything needed to render it |
| POST | `/api/review/check` | Check a typed answer: `{note_id, template_id, input}` |
| POST | `/api/review/answer` | Submit a rating (`again`/`hard`/`good`/`easy`); returns the next card |
| GET/POST | `/api/sets` | List sets, create a set |
| PUT | `/api/sets/{id}/subscription` | `{subscribed: bool}` |
| GET | `/api/sets/{id}/notes` | Notes in a set |
| GET | `/api/note-types`, `/api/note-types/{id}` | Field and template definitions |
| POST | `/api/notes` | Add a note: `{set_id, note_type_id, fields, id?, status?}` |
| GET/PUT/DELETE | `/api/notes/{id}` | Read, edit, delete a note |
| GET/PUT | `/api/settings` | New cards per day, review cap, target retention |

## Development

```sh
cargo test                     # unit + API tests, including validation of the seed sets
cargo clippy --all-targets
cargo build -p srs-core --target wasm32-unknown-unknown   # check the engine still builds for the browser
```

Validate SMILES and regenerate structure previews after editing a set:

```sh
pip install rdkit pyyaml
python tools/render_structures.py seed/amino-acids/amino-acids.yaml
```
