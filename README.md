# proteomics-srs
**Proteomic Pull-ups** by 10x Science: spaced repetition for learning about proteomics and mass
spectrometry.

A daily study tool: a short queue of due cards on your phone, scheduled with FSRS, plus a
matching-pairs memory game for practice. Structures are drawn from SMILES. See
[docs/design-proposal.md](docs/design-proposal.md) for the full design.

## Layout

```
crates/srs-core/   Scheduling (FSRS), daily queue, content model, answer checking. No I/O; builds for WASM.
crates/srs-server/ Local server: JSON API + serves the web app. SQLite in data/.
web/               Nuxt SPA built with Nuxt UI, installable as a PWA. Structures drawn from SMILES
                   with RDKit.js.
seed/              Starter sets as YAML: the 20 amino acids (default), MS/proteomics vocabulary,
                   PTMs, monosaccharides, and N-/O-glycosylation.
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

**Study, then review.** New material starts in **Study**: read through a batch of notes you
haven't seen (5 by default) with every field shown. When you finish the batch, all of those
notes' cards join your review rotation. Each card is scheduled on its own, so an amino acid's
name, three-letter code and one-letter code come due independently.

**Review** happens in rounds. Pick a size (10 by default); the round is drawn from the studied
cards that are due, at random if more are due than fit. Each card is shown once per round, and one
you miss comes back in a later round. Do as many rounds as you like; due cards never expire, they
just wait. Typed and multiple-choice answers grade themselves: a wrong answer is recorded as
Again, and a right one offers Hard / Good / Easy with Good focused, so Enter moves on.

**Start over** from Settings to forget all progress (studied notes, schedules, review history,
game results) and begin as if for the first time. Sets, notes and settings are kept. Settings
also picks the theme: light, dark, or the system's.

**Browse** (in the tab bar) flips through a set like flash cards, with every field shown. It
doesn't change your review schedule.

**Memory game** (Game in the tab bar): match pairs such as structure ↔ name or one-letter code ↔
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
| `ANTHROPIC_API_KEY` | unset | Turns on AI drafting |
| `SRS_CLAUDE_MODEL` | `claude-opus-5-5` | Model used for drafting |
| `SRS_ANTHROPIC_BASE_URL` | `https://api.anthropic.com` | Claude API endpoint (tests point it at a mock) |

### AI drafting

With `ANTHROPIC_API_KEY` set, each set's page has **Draft with AI**. Describe a topic, list
some terms, or paste text from a paper, and Claude drafts notes that fit the set's note type
(structured output against the note type's fields). Drafts land in the set marked `draft`:
they are never studied until you open each one, fix what needs fixing, and publish it, or
press **Publish all**. Anything that duplicates a note already in your library, or doesn't fit
the note type, is skipped and listed. Requests opt into server-side fallbacks
(`fallbacks: "default"`), so if the primary model declines a request the API can retry it on
a fallback model instead of returning the refusal.


### API

| Method | Path | |
|---|---|---|
| GET | `/api/study/batch` | `?size=` (default 5) the next notes not studied yet, and how many remain |
| POST | `/api/study/studied` | `{note_ids}` mark notes studied, putting their cards into review |
| GET | `/api/review/round` | `?size=` (default 10) counts of what's due, notes not studied yet, and a random round of due cards |
| GET | `/api/review/card` | `?note_id=&template_id=` one card, with everything needed to render it |
| GET | `/api/review/next` | Today's counts and the next card, with everything needed to render it |
| POST | `/api/review/check` | Check a typed answer: `{note_id, template_id, input}` |
| POST | `/api/review/answer` | Submit a rating (`again`/`hard`/`good`/`easy`); returns the next card |
| GET/POST | `/api/sets` | List sets, create a set |
| PUT | `/api/sets/{id}/subscription` | `{subscribed: bool}` |
| GET | `/api/sets/{id}/notes` | Notes in a set |
| POST | `/api/sets/{id}/draft` | Draft notes with Claude: `{source, count?, note_type_id?}`; returns `{created, skipped}` |
| POST | `/api/sets/{id}/publish-drafts` | Publish every draft in the set |
| GET | `/api/draft/status` | Whether drafting is configured, and the model |
| GET | `/api/note-types`, `/api/note-types/{id}` | Field and template definitions |
| POST | `/api/media` | Upload an image (raw body, `Content-Type: image/*`); returns `{url}` |
| POST | `/api/notes` | Add a note: `{set_id, note_type_id, fields, id?, status?}` |
| GET/PUT/DELETE | `/api/notes/{id}` | Read, edit, delete a note |
| GET | `/api/game/options` | Sets you can play (with studied-note counts) and their pairings |
| GET | `/api/game/board` | `?set=&a=&b=&pairs=` a shuffled board of studied notes |
| POST | `/api/game/results` | Record a game; returns personal bests |
| GET/PUT | `/api/settings` | Review cap, target retention |
| POST | `/api/progress/reset` | Start over: clears schedules, review history, studied notes and game results |

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
