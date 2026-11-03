# proteomics-srs
**Proteomic Pull-ups** by 10x Science: spaced repetition for learning about proteomics and mass
spectrometry.

A daily study tool: a short queue of due cards on your phone, scheduled with FSRS, plus a
matching-pairs memory game for practice. Amino-acid and monosaccharide structures are drawn
from SMILES; other topics use local diagrams. See
[docs/design-proposal.md](docs/design-proposal.md) for the current design and roadmap.

The current app runs locally for one learner, with authoring available to that user. Next are
multiple users with separate progress, a default `learner` role and a privileged `content-author`
role, and fuller content tools for sets, note types and card templates. Learners can have any
background. Automated scientific validation and full offline study/review are deferred; the
installed app caches its interface assets but still needs the server for learning sessions.

## Layout

```
crates/srs-core/   Scheduling (FSRS), daily queue, content model, answer checking. No I/O; builds for WASM.
crates/srs-server/ Local server: JSON API + serves the web app. SQLite in data/.
web/               Nuxt SPA built with Nuxt UI, installable as a PWA. Structures drawn from SMILES
                   with RDKit.js.
seed/              Six topic sets as YAML: amino acids (default), monosaccharides, PTMs,
                   glycan motifs, mass spec concepts, and peptide fragmentation.
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

Build cards from notes in the app under **Library**: create sets, add and edit notes with a live
structure preview as you type SMILES, upload images for things SMILES can't draw, and mark notes
as drafts to keep them out of review. Uploaded images are stored in `data/media/`.

Sets collect notes; templates generate the card directions for each note. Set membership
management and in-app note-type/template editing are planned, rather than current features.

**Study, then review.** New material starts in **Study**: read through a batch of notes you
haven't seen (5 by default) with every field shown. When you finish the batch, all of those
notes' cards join your review rotation. Each card is scheduled on its own, so an amino acid's
name, three-letter code and one-letter code come due independently.

Settings → **Study → New concept order** chooses how new batches are assembled. **Set order**
is the default and groups concepts by set. **Random** shuffles all eligible unstudied concepts
across your selected sets before taking the batch, so a batch can mix topics. Random mode gives
each new batch a fresh shuffle; the current batch stays in order while you study it.

Switching a set off keeps your progress. When you switch it back on and it has previous Study
or Review progress, choose **Resume**, **Start over**, or **Cancel**. Resume keeps the existing
schedules. Start over clears your studied status, schedules, and review history for that set,
so its concepts return to Study; cards, edits, settings, and memory-game bests stay. Cancel
leaves the set off. A concept shared by multiple sets is reset in all of its memberships.

**Review** happens in rounds. Pick a size (10 by default); the round is drawn from the studied
cards that are due, at random if more are due than fit. Each card is shown once per round, and one
you miss comes back in a later round. Do as many rounds as you like; due cards never expire, they
just wait. Typed and multiple-choice answers grade themselves: a wrong answer is recorded as
Again, and a right one offers Hard / Good / Easy with Good focused, so Enter moves on.

Multiple choice can use illustrations as answers. The amino-acid **name → structure** cards
show four pictures: the correct structure and up to three distinct, randomly chosen
distractors from the same set. Their positions are shuffled. The 20 amino-acid notes now
generate 140 cards, keeping the original directions alongside this new one.

The monosaccharide set uses nine locally bundled [SNFG symbols](https://www.ncbi.nlm.nih.gov/glycans/snfg.html),
with names, abbreviations, composition classes, and short context. Each also shows a skeletal
drawing rendered from stereochemical SMILES. **Form shown** names the illustrated free cyclic
form, including its anomer and stereochemistry; the SNFG symbol represents the monosaccharide
identity. Study and Browse label both pictures. Its three active directions
are name → abbreviation, name → picture, and picture → name. NeuAc/NeuGc are accepted alongside
Neu5Ac/Neu5Gc. Abbreviation cards belong only in note types where abbreviations are useful.
The former mass and composition-class drills are disabled, with their schedules/history kept.

The same picture-and-short-facts approach covers PTMs, glycan motifs, and MS concepts.
PTMs use representative before-and-after chemistry diagrams and common modification sites.
The glycan motifs build on the learned SNFG symbols, showing N-glycan cores and classes,
O-GalNAc cores, O-GlcNAc, and terminal motifs with explicit linkages. Useful material from the
former glycosylation reference is now taught through diagrams; the remaining reference notes
are retired from the starter library.

The bundled library starts with 88 notes and 299 active cards across six sets:

| Content | Notes | Active cards |
|---|---:|---:|
| Standard amino acids | 20 | 140 |
| Monosaccharides | 9 | 27 |
| Post-translational modifications | 14 | 42 |
| Glycan motifs | 15 | 30 |
| Mass spec concepts | 22 | 44 |
| Peptide fragmentation | 8 | 16 |

**Mass spec concepts** combines reading spectra, LC and ionization, analyzers, acquisition,
and proteomics workflow. **Peptide fragmentation** keeps its own set, with a, b, x and y ions,
the different backbone bonds behind a/x and b/y cleavage, and CID/HCD methods.

Detailed diagrams get full-width answer choices on a phone. In the memory game, the magnifier
on a revealed detailed picture opens a larger view without showing the answer's name.
Optional pictures can be added after drafting; visual cards wait until their picture is present.

**Start over** from Settings to forget all progress (studied notes, schedules, review history,
game results) and begin as if for the first time. Sets, notes and settings are kept. Settings
also picks the theme: light, dark, or the system's.

The app uses installed serif fonts (Georgia first) to distinguish capital I from lowercase l,
especially in scientific labels such as `pI`. No external font downloads are needed.

**Browse** (in the tab bar) flips through a set like flash cards, with every field shown. It
doesn't change your review schedule.

**Memory game** (Game in the tab bar): match pairs such as structure ↔ name or one-letter code ↔
name, using only notes you've already studied. Results are kept for personal bests, and the game
never changes your review schedule.

Seed import runs on every start. Note types and sets are updated from the files, new notes are
added, and ordinary imports preserve existing note fields and status.

A one-time migration refreshes the original monosaccharide seed fields in existing libraries.
Each field changes only if it still matches its old seed value; authored replacements survive.
No progress reset is needed. As with other migrations, it runs automatically at server startup.
The added skeletal structures use a guarded one-time field refresh as well, preserving authored
fields and existing study/review progress. The nine notes and 27 active cards keep their identities;
the new drawings are supporting facts rather than additional quiz directions.

The PTM, MS, and glycan upgrades also apply once at startup, comparing each existing field with
an immutable copy of the old seed. Original values are refreshed and missing new fields filled;
authored replacements, status, timestamps, schedules, and logs survive. Reused glycan and MS
notes keep their IDs. Existing MS subscriptions follow the combined concepts set or peptide
fragmentation as appropriate, once, preserving later opt-outs. Superseded sets and unused
reference notes remain archived in existing databases, outside the learner library and queues.
Authored MS notes follow the combined set; custom reference notes remain available in
**Your glycosylation notes**. Old seed drills are disabled in place; authored legacy drill
cards are retained.
An edited PTM SMILES keeps its existing example and context; its picture cards wait for an
authored diagram rather than receiving a stock picture of different chemistry.

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
the note type, is skipped and listed. Newly published notes with usable cards become available
in Study; their cards enter Review after Study completion. Add optional diagrams by hand before
their visual cards can be studied. The server validates field data, but automated
scientific checks and a future integration with our own proteomics model are deferred.
Requests opt into server-side fallbacks
(`fallbacks: "default"`), so if the primary model declines a request the API can retry it on
a fallback model instead of returning the refusal.


### API

| Method | Path | |
|---|---|---|
| GET | `/api/study/batch` | `?size=` (default 5) the next notes not studied yet, ordered by the learner's Study setting, and how many remain |
| POST | `/api/study/studied` | `{note_ids}` mark notes studied, putting their cards into review |
| GET | `/api/review/round` | `?size=` (default 10) counts of what's due, notes not studied yet, and a random round of due cards |
| GET | `/api/review/card` | `?note_id=&template_id=` one card with shuffled choices; illustration choices contain image URLs or source SMILES, rendered using the answer field definition |
| GET | `/api/review/next` | Today's counts and the next card, with everything needed to render it |
| POST | `/api/review/check` | Check an answer: `{note_id, template_id, input}`; derived illustrations use their source value as input |
| POST | `/api/review/answer` | Submit a rating (`again`/`hard`/`good`/`easy`); returns the next card |
| GET/POST | `/api/sets` | List current sets (retired collections omitted), create a set; `studied_count` counts distinct notes with Study status, card state, or review history for the current user |
| PUT | `/api/sets/{id}/subscription` | `{subscribed: bool, progress?: "resume" \| "restart"}`; defaults to resume; restart resets this user's Study/Review progress only when switching an off set on |
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
| GET/PUT | `/api/settings` | Review cap, target retention, and `study_order` (`set_order` or `random`; default `set_order`) |
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

Regenerate the bundled PTM connectivity diagrams with the native Rust tool:

```sh
cargo run -p srs-server --bin render_ptm_diagrams
# Optional output directory:
cargo run -p srs-server --bin render_ptm_diagrams -- /tmp/ptm-diagrams
```

The default output is `web/public/study/ptms/`. `cargo run -p srs-server` still starts the app.
