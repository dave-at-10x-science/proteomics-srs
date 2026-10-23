# proteomics-srs
Spaced repetition system for learning about proteomic and mass spectrometry

A daily study tool: a short queue of due cards on your phone, scheduled with FSRS, plus a
matching-pairs memory game for practice. Structures are drawn from SMILES. See
[docs/design-proposal.md](docs/design-proposal.md) for the full design.

## Layout

```
crates/srs-core/   Scheduling (FSRS), daily queue, content model, answer checking. No I/O; builds for WASM.
crates/srs-server/ Local server: JSON API + serves the web app. SQLite in data/.
web/               Nuxt SPA, installable as a PWA. Structures drawn from SMILES with RDKit.js.
seed/              Starter sets as YAML: the 20 amino acids (default) and MS/proteomics vocabulary.
tools/             Content helpers (SMILES validation and structure previews).
docs/              Design notes.
```

## Running it

```sh
(cd web && npm install && npm run build)   # once, and after pulling web changes
cargo run -p srs-server
```

Then open `http://localhost:8080`.

**On your phone:** on the same Wi-Fi, open `http://<your-computer's-LAN-IP>:8080`. Installing it
to the home screen as an app needs HTTPS. The easiest way while running locally is
[Tailscale](https://tailscale.com): run `tailscale serve 8080` on the computer and open the
`https://<machine>.<tailnet>.ts.net` address on the phone (also works away from home).

**Developing the web app:** run the server, then `cd web && npm run dev` for hot reload on
`http://localhost:3000` (it proxies `/api` to the server on port 8080).

This creates `data/srs.sqlite`, imports everything under `seed/` (the amino acids are
subscribed by default), and listens on `http://0.0.0.0:8080` so a phone on the same Wi-Fi can
reach it at your machine's LAN address. Back up by copying `data/`.

Add and edit cards in the app under **Library**: create sets, add notes with a live structure
preview as you type SMILES, upload images for things SMILES can't draw, and mark notes as drafts
to keep them out of review. Uploaded images are stored in `data/media/`.

**Memory game** (Game in the header): match pairs such as structure ↔ name or one-letter code ↔
name, using only notes you've already studied. Results are kept for personal bests, and the game
never changes your review schedule.

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
| POST | `/api/media` | Upload an image (raw body, `Content-Type: image/*`); returns `{url}` |
| POST | `/api/notes` | Add a note: `{set_id, note_type_id, fields, id?, status?}` |
| GET/PUT/DELETE | `/api/notes/{id}` | Read, edit, delete a note |
| GET | `/api/game/options` | Sets you can play (with studied-note counts) and their pairings |
| GET | `/api/game/board` | `?set=&a=&b=&pairs=` a shuffled board of studied notes |
| POST | `/api/game/results` | Record a game; returns personal bests |
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
