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
sets that the learner has not studied yet, with all fields visible. The default batch size is
five notes. Finishing the batch marks its notes studied and makes their enabled cards eligible
for review. Publishing a new note makes it available for Study; it does not mark it studied.

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

### Browse and memory practice

Browse flips through published notes in a set with every field shown. It changes neither
studied status nor review schedules.

The memory game matches pairs of fields from studied, published notes in a selected set.
Completing Study is enough to make a note eligible; a first review is not required. Pairings
come from compatible single-field card directions, such as structure/name or code/name. The
UI offers four, six, eight, or ten pairs; the API clamps requests to three through twelve,
subject to enough distinct usable notes. Game results track time, flips, and personal bests
separately from scheduling. There is no additional question after a successful match.

Settings includes target retention, an optional review cap, theme selection, and a confirmed
progress reset. Reset clears the current user's studied notes, schedules, review log, and game
results while keeping content, subscriptions, and settings.

## 3. Current content and authoring

### Notes, templates, cards, and sets

- A **note type** defines fields, card templates, and optional structure depiction templates.
  Field kinds are text, Markdown, numbers with optional units, SMILES, and images. Images can
  be uploaded or derived from SMILES.
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

### Implemented authoring

Library creates sets. Inside a set, an author can add a note using an existing note type, edit
fields, preview structures while entering SMILES, upload images, and save as draft or
published. Existing notes can be edited or deleted. Drafts can be published individually or
in a batch. Authoring is currently available to the single local user.

There is no in-app editor for note types or card templates, no set rename/delete action, and
no action to add existing notes to another set, remove a membership, or reorder a set.
Notes record status and timestamps, but no creator, editor, or publisher identity.

### Starter content and startup import

The five starter sets are the 20 standard amino acids, MS/proteomics vocabulary, PTMs,
monosaccharides, and N-/O-glycosylation. The amino-acid set has 20 notes and seven enabled card
templates per note, giving 140 cards:

- Structure to name, three-letter code, and one-letter code: three separate typed cards.
- One-letter code to name and three-letter code to name: multiple choice.
- Name to one-letter code: typed.
- Name to structure: multiple choice among four illustrations.

Side-chain class, residue mass, pI, and supporting notes are available as facts, without
dedicated class or mass review templates in the current set.

Seed YAML imports run at every server start. They overwrite note-type definitions and set
metadata, insert new notes, and preserve existing note fields/status. Deleting a seed note in
the app allows the next startup to recreate it. The monosaccharide refresh has a one-time
migration that replaces only fields still matching their original seed values, preserving
authored replacements. A broader ownership policy is still needed before seed-derived sets
and note types become fully editable in the app.

### Visual set pattern and monosaccharide pilot

Use a recognizable picture and a small number of short facts, following the amino-acid set.
Choose a consistent field schema for a topic family. Include names, codes, abbreviations, or
distinguishing facts only when meaningful for that family; abbreviations are not universal.
Keep optional context brief, and choose which facts merit their own cards rather than testing
every field. Each card asks for one answer. Required card fields must be present on every note
using that template; optional context need not generate cards.

Pictures should distinguish the items being tested and use consistent presentation. A useful
default is picture → name, supplemented by name → picture multiple choice and short-code
directions where applicable. Memory pairs can match pictures to names or codes. Shared
categories and masses do not uniquely identify items and should not be used as identity cues.

The monosaccharide pilot has nine actual SNFG symbols, a name, abbreviation, composition class,
and a short context sentence per note. Three active directions focus on identity: name →
abbreviation, name → picture, and picture → name. Neu5Ac/Neu5Gc accept common NeuAc/NeuGc aliases.
The old mass and composition-class template IDs remain disabled to preserve their histories;
their drills are also omitted from memory pairings. Other starter sets await review of this pilot.

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
   introduced. Source references and AI-assisted enrichment of existing notes are later
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
skipped and reported. Created notes remain drafts until published. Publishing new content
then makes it available in Study, with Review following each learner's Study completion.

SMILES are the source for small-molecule drawings. RDKit.js renders them in the browser and
can align a shared backbone using a note type's depiction template. Uploaded images cover
other visual material. The manual note editor parses SMILES for its live preview and prevents
saving a detected parse failure. Developer seed tooling parses structures, checks amino-acid
residue masses where provided, and generates previews.

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
   starter sets, basic note authoring, uploads, and optional AI drafting.
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
crates/srs-server/migrations/ Append-only SQLite migrations
crates/srs-server/tests/      In-memory API integration tests
web/app/                     Nuxt pages, components, API client, types, and theme
seed/<set>/<set>.yaml         Starter set, note type, and notes
tools/                       Existing developer content-validation/rendering tools
docs/                        Current design and roadmap
data/                        Local database and uploaded media; never edit by hand
```
