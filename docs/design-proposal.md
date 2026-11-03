# Proteomic Pull-ups: current design and roadmap

Updated 2026-11-03. This replaces the original proposal with the implemented design and the
next priorities. **Current** describes working code; **planned** describes work still to do.
The README remains the reference for running the app, configuration, and existing endpoints.

## 1. Purpose and priorities

Proteomic Pull-ups is a 10x Science application for learning proteomics and mass spectrometry
through spaced repetition and games. A learner can have any background. The main activity is
flashcard review, supported by an introduction to new material in Study, a matching-pairs memory
game, and Browse. Topic sets organize the shared content.

The basic learning and game flows are implemented. The next priorities are multiple learners
with separate progress, a privileged content-author role, and better tools for building and
organizing content. Automated scientific validation and full offline learning can wait.

| Area | Current | Planned |
|---|---|---|
| Learning | Study batches, FSRS review rounds, Browse, and memory game | Preserve these flows while adding users and content |
| Users | One built-in local user; no authentication or roles | Multiple authenticated users; `learner` by default, `content-author` by explicit grant |
| Authoring | Create sets; create, edit, delete, draft, and publish notes; image upload | Manage sets and membership; edit note types and templates; protect existing history |
| AI drafting | Optional Claude integration creates drafts from a topic, terms, or pasted text | A future integration may use our own proteomics model |
| Validation | Field/schema checks, browser SMILES parsing, and developer seed checks | Automated chemical and scientific checks are deferred |
| Connectivity | Installable PWA with cached interface assets; API requires the server | Full offline study/review and synchronization are deferred |

## 2. Current learning experience

The app is a phone-first Nuxt SPA, installable as a PWA, with desktop browser support. The main
tabs are Today, Browse, Game, Library, and Settings. Nuxt UI supplies controls and theme support;
structures and game tiles use purpose-built views. Assets are bundled locally.

### Study introduces notes

The learner chooses which sets to study. Study shows a batch of published notes from subscribed
sets that the learner has not studied yet and that have at least one usable enabled card, with
all fields visible. The default batch size is five notes. Finishing the batch marks its notes
studied and makes their usable enabled cards eligible for review. Publishing a new note does
not mark it studied. Retired sets do not enter the Study queue.

There is no enforced daily new-card limit. A legacy `new_per_day` setting remains in storage
and the settings API, but Study batches control introduction of new material.

### Review schedules cards independently

FSRS in `srs-core` computes memory state and next due dates for each card. Target retention
defaults to 90%. The queue includes learning/relearning cards due now, review cards due by the
end of the study day, and studied cards awaiting their first review. An optional daily review
cap applies to the review portion of the queue.

A round is a fixed list, ten cards by default. If more cards are eligible than fit, it takes
a random sample. Each card appears once in that round; missed cards can return in a later
round when due. Cards from the same note have independent schedules and may appear on the
same day. Round ordering tries to separate adjacent cards from the same note, without burying
them for the day.

Answer modes are flip and self-grade, typed answer, and multiple choice. Typed and
multiple-choice mistakes record Again; correct answers offer Hard, Good, or Easy. The study
day currently uses the server's local time and `SRS_DAY_ROLLOVER_HOUR`, defaulting to 4am.

Multiple-choice answers can be text or illustrations. A name-to-picture question shows the
correct illustration plus up to three distinct distractors from published notes of the same
type in the set, with shuffled positions. Derived structures use the source SMILES and the
note type's depiction template; uploaded/bundled pictures use their image URL. Before an
answer, illustration options carry neutral option numbers without identity captions.

Fields marked `detailed` use one-column picture choices on a phone and two columns from the
`sm` breakpoint, so chemistry and instrument labels remain readable. Simpler pictures, including
amino acids and monosaccharide symbols, keep two-column choices.

### Browse and memory practice

Browse flips through published notes in a set with every field shown. It changes neither
studied status nor review schedules.

The memory game matches pairs of fields from studied, published notes in a selected set.
Completing Study is enough to make a note eligible; a first review is not required. Pairings
come from compatible single-field card directions, such as structure/name or code/name. The
UI offers four, six, eight, or ten pairs; the API clamps requests to three through twelve,
subject to enough distinct usable notes. Game results track time, flips, and personal bests
separately from scheduling. There is no additional question after a successful match.

A magnifier on a revealed detailed picture opens `DiagramPreview` at a larger size. The modal
has the generic title **Diagram**, without the note's name. The preview button is separate
from the flip button and is absent on hidden tiles; it does not change flips, matches or timers.

Settings includes target retention, an optional review cap, theme selection, and a confirmed
progress reset. Reset clears the current user's studied notes, schedules, review log, and game
results while keeping content, subscriptions, and settings.

## 3. Current content and authoring

### Notes, templates, cards, and sets

- A **note type** defines fields, card templates, and optional structure depiction templates.
  Field kinds are text, Markdown, numbers with optional units, SMILES, and images. Images can
  be uploaded, bundled, or derived from SMILES. The optional `detailed` field hint controls
  picture-choice layout and memory-game enlargement.
- A **note** stores one learning item and its related facts under a note type. Its status is
  `draft` or `published`.
- A **card template** defines prompt fields, answer fields, and an answer mode. A **card** is
  one note paired with one template. Stable identity is `(note_id, template_id)`.
- A **set** is a named, tagged collection of notes. A note can belong to multiple sets in the
  database; the current app creates a note directly in one set and has no membership editor.
- A **subscription** selects a set for a learner's Study and Review flows. The amino-acid set
  is subscribed by default for the current local user.

Drafts already have generated card records, but stay out of Study, Review, Browse, and the
memory game. The current Library and content API expose them for editing; role restrictions
do not exist yet.

An optional picture can be omitted while drafting facts. Each card separately records whether
its prompt and answer fields have content; a blank picture withholds its visual cards from
Study and Review. Adding the picture makes those cards usable without changing their enablement
or deleting schedules. A note with another usable direction can still be studied.

### Implemented authoring

Library creates sets. Inside a set, an author can add a note using an existing note type, edit
fields, preview structures while entering SMILES, upload images, and save as draft or
published. Existing notes can be edited or deleted. Drafts can be published individually or
in a batch. Authoring is currently available to the single local user.

There is no in-app editor for note types or card templates, no set rename/delete action, and
no action to add existing notes to another set, remove a membership, or reorder a set.
Notes record status and timestamps, but no creator, editor, or publisher identity.

### Starter content and startup import

The bundled library has six sets, 88 unique notes and 299 active cards. A fresh database stores
449 card rows, including disabled directions retained for stable identity and existing history.
Existing databases can also contain archived content, authored notes and retained legacy drills.

| Content | Notes | Active cards |
|---|---:|---:|
| Standard amino acids | 20 | 140 |
| Monosaccharides | 9 | 27 |
| Post-translational modifications | 14 | 42 |
| Glycan motifs | 15 | 30 |
| Mass spec concepts | 22 | 44 |
| Peptide fragmentation | 8 | 16 |

The amino-acid set has seven enabled templates per note:

- Structure to name, three-letter code, and one-letter code: three separate typed cards.
- One-letter code to name and three-letter code to name: multiple choice.
- Name to one-letter code: typed.
- Name to structure: multiple choice among four illustrations.

Side-chain class, residue mass, pI, and supporting notes are available as facts, without
dedicated class or mass review templates in the current set.

Seed YAML imports run at every server start. They overwrite note-type definitions and set
metadata, insert new notes, and ordinarily preserve existing note fields/status. Explicit
content upgrades are exceptions with versioned, one-time guards. Deleting a seed note in the
app still allows the next startup to recreate it. A broader ownership policy is needed before
seed-derived sets and note types become fully editable in the app.

The original monosaccharide refresh is an append-only SQL migration. The PTM, MS, and glycan
refresh uses `seed_refresh.rs` with immutable original YAML in `seed_baselines/` and receipts
created by migration `0006_visual_study_sets.sql`. A second immutable snapshot supports the
later glycan salvage and fragmentation updates. Each field is replaced or removed only when
it still equals the matching snapshot value; missing new fields are filled once. Authored
replacements, status, timestamps, studied status, schedules, and review/game logs survive. Old seed drill
cards are disabled in place, while authored legacy drill cards retain their enablement.
PTMs with edited SMILES retain their existing example/context and wait for an authored diagram,
so the refresh cannot substitute a stock picture of different chemistry.

The four original O-GalNAc note IDs and six reusable reference concepts move to `GlycanMotif`,
retaining legacy template IDs in disabled form. Existing MS notes keep their IDs when the five
concept topics merge; peptide fragmentation stays separate. Authored notes in the old MS sets
follow the merged set, except existing fragmentation notes follow their own set.

Subscriptions follow these moves once; later opt-outs stick. Superseded sets and twelve unused
reference concepts are retired from the learner library without deleting stored notes, card
states or logs. Custom reference notes remain available in **Your glycosylation notes**;
the old reference set itself is retired. Refresh receipts survive note deletion so
recreating an ID cannot reapply an old upgrade.

### Visual topic sets

Use a recognizable picture and a small number of short facts, following the amino-acid set.
Choose a consistent field schema for a topic family. Include names, codes, abbreviations, or
distinguishing facts only when meaningful for that family; abbreviations are not universal.
Keep optional context brief, and choose which facts merit their own cards rather than testing
every field. Each card asks for one answer. Prompt and answer values must be present before a
card is usable; optional context need not generate cards.

Pictures should distinguish the items being tested and use consistent presentation. A useful
default is picture → name, supplemented by name → picture multiple choice and short-code
directions where applicable. Memory pairs can match pictures to names or codes. Shared
categories and masses do not uniquely identify items and should not be used as identity cues.

The monosaccharide pilot has nine actual SNFG symbols, a name, abbreviation, composition class,
and a short context sentence per note. Three active directions focus on identity: name →
abbreviation, name → picture, and picture → name. Neu5Ac/Neu5Gc accept common NeuAc/NeuGc aliases.
The old mass and composition-class template IDs remain disabled to preserve their histories;
their drills are also omitted from memory pairings.

The PTM set uses fourteen representative before-and-after connectivity diagrams, common sites,
an example residue, a short chemical change and brief context. Active directions are picture
to name, name to picture, and name to common sites. Mass and MS-detail templates remain defined
but disabled for the seed notes. The drawings show connectivity, not complete reaction mechanisms
or a fixed protonation state. The GG example is explicitly an analytical remnant after digestion.

**Glycan motifs** has fifteen notes: the six O-GalNAc motifs (Tn, a Neu5Ac-containing sialyl-Tn
example, and cores 1–4), the N-glycan core, representative high-mannose/complex/hybrid N-glycans,
core α1-6 fucosylation, O-GlcNAc, LacNAc, and α2-3/α2-6 sialyl-LacNAc. SNFG pictures specify
linkages and attachment where relevant. Larger glycans are representative structures, not
claims that a glycan class has only one structure. Each note has a name, family, distinguishing
feature and brief context, with picture/name multiple choice in both directions. The former
reference paragraphs are replaced by this visual treatment where it works and retired otherwise.

The MS/proteomics notes share the `Term` schema across two sets. **Mass spec concepts** combines
reading spectra (4 notes), LC and ionization (4), analyzers (3), scans and acquisition (5), and
workflow (6). **Peptide fragmentation** has eight notes: a, b, x and y ions, a/x and b/y cleavage,
ion-trap CID and HCD. Backbone diagrams distinguish the Cα–C(=O) cleavage associated with a/x
fragments from the C(=O)–N cleavage associated with b/y fragments, and show which terminus is
retained. Complementary series names do not imply that both charged fragments are observed
from every dissociation event.

Each MS note has a schematic, short definition, function and key fact where useful;
symbols/abbreviations are optional. Picture/name multiple choice runs in both directions.
Old definition template IDs remain disabled for the seed notes. Diagrams are teaching
schematics rather than measured spectra or scale drawings; sources are recorded with the content.

## 4. Current architecture and storage

```text
Phone / desktop browser                  Rust server
  Nuxt SPA + Nuxt UI       HTTP/JSON        Axum routes and static SPA/media serving
  RDKit.js: SMILES -> SVG  <-------->       srs-core: FSRS, queues, model, answer checks
                                           SQLite via sqlx + uploaded media files
                                           Optional drafting -> Claude API
```

- `srs-core` is pure Rust with no I/O and builds for `wasm32-unknown-unknown`. The server uses
  it natively; the browser does not currently run the scheduler.
- `srs-server` serves the built SPA and JSON API from one port. SQLite and uploaded media live
  under `data/`. Local phone access uses LAN connectivity or a private HTTPS endpoint such as
  Tailscale. There is no authentication today.
- Shared content tables are `note_types`, `sets`, `notes`, `set_notes`, and `cards`. Field,
  template, and depiction definitions live together as JSON in `note_types`; there is no
  separate templates table. Uploaded files are named by content hash, without a media table.
- `users` holds learner settings. Subscriptions, studied notes, card states, review logs, and
  game sessions carry `user_id`. All handlers currently use `LOCAL_USER = 1`.
- `cards.content_available` is separate from `cards.enabled`; reconciliation updates content
  availability while preserving enablement. `seed_content_refreshes` records one-time content
  upgrades and topic subscription moves.
- `sets.retired` keeps superseded collections in storage while excluding them from learner
  lists, Study, Review and Game. Retirement preserves note/card identities and existing logs.
- The server controls time through `AppState::now()`, allowing tests to use a fixed clock.
  Migrations are append-only, and API tests use in-memory databases and a mock AI endpoint.
- The PWA caches the interface and RDKit assets. API calls still require connectivity to the
  server; there is no offline answer queue or synchronization protocol.

## 5. Planned: multiple users and roles

Keep one shared content library with private progress for each learner. Roles express access
to functionality, independent of occupation, educational status, or prior knowledge.

| Capability | `learner` | `content-author` |
|---|---|---|
| Browse published sets and choose subscriptions | Yes | Yes |
| Study, Review, Game, personal settings and progress reset | Own account | Own account |
| Read drafts and use Library authoring tools | No | Yes |
| Create/edit content, sets, note types, and templates | No | Yes |
| Upload authoring media, draft with AI, and publish | No | Yes |
| Read or reset another learner's progress | No | No |

New accounts receive `learner`. A trusted operator explicitly grants `content-author`;
learners cannot grant themselves author privileges. No additional classroom, organization,
or per-set permission model is required for this milestone.

Implementation requirements:

- Resolve the authenticated account on the server and replace `LOCAL_USER` in request handling.
  Scope all progress reads/writes to that identity. Enforce author permissions in the API as
  well as the UI, including AI drafting and media uploads.
- Give learners published content only, including list responses, direct note reads, and
  content used by Browse, Study, Review, and Game. Decide how draft media is protected when
  replacing today's unrestricted static media route.
- Preserve the existing user-1 identity and history. Link the owner's authenticated account
  to that row through trusted setup and grant it `content-author`; do not reset or copy its
  progress into a new account.
- Apply default set subscriptions once when an account is created, preserving later opt-outs.
  Publishing shared content must not subscribe every learner or mark it studied for them.
- Clear or scope saved review rounds and other private browser state on account changes.
  Choose a learner time-zone/day-rollover policy before shared hosting, since the current
  study day follows the server's local time.

Authentication provider, account provisioning, session handling, and hosting remain
implementation decisions. Earlier Google/domain and cloud-provider suggestions are options,
not settled requirements. Identity and role schema changes can be added to SQLite without
changing database engines.

Acceptance includes two learners studying the same content with independent subscriptions,
studied status, schedules, settings, history, and game results; one learner's reset leaving
the other untouched; author-only endpoints rejecting learner requests; drafts remaining
invisible to learners; and existing local history surviving migration and account linking.

## 6. Planned: complete the content-author workflow

Content creation and organization are the next product focus alongside user separation.

1. **Manage sets and reuse notes.** Create, rename, and tag sets; find existing notes; add them
   to one or more sets; remove memberships; and reorder material for Study and Browse.
   Removing a membership must not delete the underlying note or its learning history. The
   same card in multiple subscribed sets should appear once in a review round.
2. **Edit note types and templates.** Add and edit fields, labels, units, answer modes,
   accepted-answer rules, card directions, and depiction templates. Preview cards and show
   the notes/cards affected before applying a schema or template change. Validate existing
   notes and reconcile all affected cards in one transaction, including notes created in the
   app as well as the original seed notes.
3. **Preserve learning history.** Keep note and template IDs stable during ordinary edits.
   Add templates under new IDs and retain schedules for existing cards. Define retirement
   and deletion behavior before exposing destructive shared edits, including the effects on
   memberships, active schedules, and retained review logs. Define whether template enablement
   changes affect only new cards or existing cards too.
4. **Make authored content persist.** Replace unconditional seed overwrites with explicit,
   author-controlled updates. Seed content can initialize a library; subsequent imports must
   not silently undo edits or recreate intentionally removed content. Show conflicts rather
   than applying them invisibly.
5. **Improve drafting and publishing.** Keep draft review, editing, publishing, and discard
   available to content authors. Record creator/editor/publisher attribution as accounts are
   introduced. In-app source fields and AI-assisted enrichment of existing notes are later
   authoring improvements; they are not implemented today.

Acceptance includes authoring a new topic set through the app, reusing a note without
duplicating its card identity, previewing a new card direction, preserving existing schedules
after an ordinary edit, and retaining author changes across restarts. Removing a set or note
must have explicit, confirmed semantics before those broader management actions ship.

## 7. AI, structures, and validation

### Current behavior

The optional Claude integration takes a topic, terms, or pasted text and asks for structured
notes using the selected note type. Configure it through process environment variables; the
server does not automatically load a `.env` file. Invalid field data and duplicate notes are
skipped and reported. Created notes remain drafts until published. Published notes with usable
cards become available in Study, with Review following each learner's Study completion. Optional
picture fields let AI draft the facts first; a human adds the diagram before visual cards run.

SMILES are the source for small-molecule drawings. RDKit.js renders them in the browser and
can align a shared backbone using a note type's depiction template. Uploaded images cover
other visual material. The manual note editor parses SMILES for its live preview and prevents
saving a detected parse failure. Developer seed tooling parses structures, checks amino-acid
residue masses where provided, and generates previews.

PTM schematics are generated by the native Rust CLI
`cargo run -p srs-server --bin render_ptm_diagrams -- [output-directory]`, defaulting to
`web/public/study/ptms/`. The crate keeps the server as its default binary. Glycan and MS
schematics are local SVG assets under `web/public/study/`; they do not require runtime services.

The common server validator checks required/unknown fields, scalar types, and length limits.
Set-file checks also validate internal references and unique IDs. These checks support data
integrity; the server does not parse molecular structures, verify stereochemistry, calculate
masses, or judge the scientific truth of a definition when saving or publishing content.

### Deferred aspirations

Automated scientific validation would apply during content creation or editing, whether
manual or AI-assisted: molecular parsing on the server, checking stereochemistry, comparing
calculated and entered masses, or checking scientific statements. These checks are future
work and do not block the user/authoring milestones.

Future drafting may use our own proteomics model. Model/provider integration, enrichment of
existing notes, and any additional automated verification can be designed when that work is
prioritized. The current author review and draft/publish workflow remains in place.

## 8. Delivery order and deferred work

1. **Implemented foundation:** Rust scheduler and server, Study/Review, Browse, memory game,
   six topic sets, visual multiple choice, detailed-diagram enlargement, guarded content
   refreshes, basic note authoring, uploads, and optional AI drafting.
2. **Next: users and access.** Authentication, `learner`/`content-author`, isolated progress,
   learner content visibility, and migration of the existing local account.
3. **Next: content tools.** Set management and membership, note-type/template editing,
   history-preserving changes, and an import policy that preserves author edits. Prepare this
   work alongside roles, with new authoring actions protected when released for shared use.
4. **Later:** our own proteomics-model integration, richer authoring assistance, source and
   publication history, learning statistics, personalized FSRS fitting, and additional games.
5. **Deferred:** automated scientific validation and full offline Study/Review with sync.
   Native app-store builds, classrooms, public sign-up, and a particular hosting stack have
   no committed scope in this design.

## 9. Repository map

```text
crates/srs-core/              Model, validation, FSRS, queues, and answer checks
crates/srs-server/src/        Axum API, storage, seed import, and Claude client
crates/srs-server/src/bin/    Native content tools, including the PTM diagram generator
crates/srs-server/src/seed_baselines/ Immutable originals for guarded one-time content upgrades
crates/srs-server/migrations/ Append-only SQLite migrations
crates/srs-server/tests/      In-memory API integration tests
web/app/                     Nuxt pages, components, API client, types, and theme
web/public/study/             Bundled SVG study diagrams and symbols
seed/<set>/<set>.yaml         Starter set, note type, and notes
tools/                       Existing developer content-validation/rendering tools
docs/                        Current design and roadmap
data/                        Local database and uploaded media; never edit by hand
```
