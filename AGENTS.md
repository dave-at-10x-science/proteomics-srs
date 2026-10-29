# AGENTS.md

Quick orientation for agents working in this repo. [README.md](README.md) has the full feature
description, environment variables and API table; [docs/design-proposal.md](docs/design-proposal.md)
has the original design. Where the design doc and the code disagree (for example, it describes a
daily new-card cap and sibling burying, which study mode and independent per-card scheduling
replaced), the code and README win.

## What this is

Proteomic Pull-ups, a 10x Science app (the repo and crates keep the name `proteomics-srs`). It
is a local, single-user spaced repetition app for proteomics and mass spectrometry. A Rust server
(Axum + SQLite) serves a JSON API and a Nuxt SPA that is used mostly on a phone. Cards are
scheduled with FSRS. New notes are read in **Study**, then their cards enter **Review** rounds.
A memory game and Browse mode never touch the schedule. Optional AI drafting calls the Claude API.

## Map

| Path | What lives there |
|---|---|
| `crates/srs-core/` | Pure logic, no I/O, must build for `wasm32-unknown-unknown`: content model and set-file validation (`model.rs`, `content.rs`), FSRS wrapper (`scheduler.rs`), queue and rounds (`queue.rs`), typed-answer checking (`answer.rs`). |
| `crates/srs-server/src/` | `lib.rs` (`AppState`, DB setup, router), `main.rs` (config from env), `seed.rs` (YAML import), `store.rs` (all SQL), `api/` (one module per area), `claude.rs` (hand-written Messages API client), `error.rs` (`ApiError` to HTTP status). |
| `crates/srs-server/migrations/` | Numbered SQLite migrations, embedded with `sqlx::migrate!`. |
| `crates/srs-server/tests/api.rs` | End-to-end API tests. |
| `web/app/` | Nuxt 4 SPA (`ssr: false`, PWA) built with Nuxt UI. `pages/`, `components/`, `composables/useApi.ts` (every API call), `types/api.ts` (hand-mirrored server JSON), `assets/main.css` (Tailwind and Nuxt UI imports, brand palette, theme variables), `app.config.ts` (Nuxt UI colors and component defaults). |
| `seed/<set>/<set>.yaml` | Starter sets: one YAML file per set, holding the set, its note type and its notes. |
| `tools/render_structures.py` | Validates SMILES and masses in a set file and renders preview images. |
| `data/` | Gitignored. The user's real database (`srs.sqlite`) and uploaded media. |

## Commands

These match CI (`.github/workflows/ci.yml`). Run the relevant ones before calling a change done.

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build -p srs-core --target wasm32-unknown-unknown

(cd web && npm ci && npm run typecheck && npm run build)

pip install rdkit pyyaml
for f in seed/*/*.yaml; do python tools/render_structures.py "$f"; done
```

To run the app: build the web app once, then `cargo run -p srs-server` and open
`http://localhost:8080`. For hot reload, also run `cd web && npm run dev` (port 3000, proxies
`/api` to 8080).

## Rules

- **Always work on a branch.** Never commit directly to `main`. Before the first commit, create
  a branch off `main` (a feature branch, a fix branch, whatever fits the work). Changes reach
  `main` through pull requests.
- **Leave `data/` alone.** It holds the user's real study history. Don't delete, reset or
  hand-edit it. Tests use an in-memory database.
- **Migrations are append-only.** Add a new numbered file and never edit one that has been
  committed, because it has already run against the user's database. There is no `build.rs`, so
  cargo may not notice a new migration file; touch `crates/srs-server/src/lib.rs` if it doesn't
  get picked up.
- **Keep `srs-core` free of I/O.** No filesystem, network, database or async runtime there. It
  has to keep building for the browser.

## Things that are easy to get wrong

- **Card identity is `(note_id, template_id)`.** Review history hangs off it. Renaming a note id
  or template id in a seed file orphans the history for those cards.
- **Seed import runs on every start.** Note types and sets are overwritten from the YAML, new
  notes are inserted, and existing notes are never updated (so edits made in the app survive).
  Changing an existing note's fields in YAML won't change an existing database.
- **Adding a seed set:** `every_seed_set_is_valid` in `srs-core` checks it, CI runs
  `render_structures.py` on it, and `TestApp::build` in `tests/api.rs` asserts the number of
  imported sets, so update that count. SMILES are the source of truth for structures; the SVG
  and PNG files under `seed/*/` are generated previews, so regenerate them rather than editing
  them by hand.
- **API changes touch four places:** the handler and route (`api/`), `web/app/types/api.ts`,
  `web/app/composables/useApi.ts`, and the API table in the README.
- **Time comes from `AppState::now()`,** not `Utc::now()`, in request handling. Tests run on a
  fixed clock and advance it. The study day rolls over at `SRS_DAY_ROLLOVER_HOUR` local time.
- **One built-in user (`LOCAL_USER`),** but per-user tables keep `user_id` and the SQL stays
  close to portable so Postgres and more users can come later.
- **Validation errors** are `ApiError::Invalid(Vec<String>)`, returned as 422 with an `errors`
  list that the web app shows via `apiErrors()`. Report every problem at once, not just the
  first one.
- **Tests never call the real Claude API.** `TestApp::with_claude` points the client at a local
  mock server.

## Web UI

The web app uses [Nuxt UI](https://ui.nuxt.com) v4 on Tailwind CSS v4. Don't invent styling.

- **Nuxt UI components first** for buttons, inputs, selects, forms (`UFormField` around every
  input), cards, alerts, badges, switches, modals and toasts. Lay out with Tailwind utilities.
- **No `<style>` blocks and no new CSS files.** Theme-wide values live in `app/assets/main.css`
  and `app/app.config.ts`. Change one component's look with its props (`color`, `variant`,
  `size`) or its `ui` and `class` props.
- **Semantic colors only:** `primary`, `neutral`, `success`, `warning`, `error`, `info`, and
  utilities like `text-muted`, `bg-elevated`, `border-default`. No palette classes like
  `text-gray-500`. Ratings map to colors: Again is `error`, Hard is `warning`, Good is `success`,
  Easy is `info`. The exceptions are deliberate: structures are drawn on white "paper" in both
  themes, and the Today panel is white on the brand gradient.
- **Bespoke visuals** (game tiles, structure drawings, the three-across facts grid) are plain
  elements styled with Tailwind utilities.
- **Shared pieces:** `ErrorAlert` shows server errors, `ConfirmModal` (opened with
  `useOverlay()`) asks before anything destructive, `SegmentedPicker` picks a size, and
  `AppTabBar` is the main navigation. Confirmations like "Saved" are toasts (`useToast()`).
  Never use `window.confirm` or `alert`.
- **Branding is 10x Science.** The primary color is the logo's purple. The header shows the full
  logo (`public/10x-science-logo-light.svg` on light, `10x-science-logo.svg` on dark, swapped by
  `UColorModeImage`), and `BrandMark` draws the mark for decoration. The app icons in `public/`
  are made from the 10x Science icon: `apple-touch-icon.png` and `icon-maskable-512.png` are
  square and opaque, because iOS and Android mask them and show transparency as black.
- **Icons are `i-lucide-*` names written literally in `.vue` files.** They are bundled at build
  time and never fetched (`icon.provider: 'none'`), so a name built at runtime or kept in a
  `.ts` file won't render.
- **Look up the API, don't recall it.** Nuxt UI changed a lot between v2, v3 and v4. Use the
  Nuxt UI MCP server (configured in `.mcp.json`; `https://ui.nuxt.com/mcp` for other tools), any
  docs page as Markdown (`https://ui.nuxt.com/raw/docs/components/<name>.md`), or the generated
  theme files in `web/.nuxt/ui/<component>.ts` for slot names and default classes.
- **Phone first.** Check new screens at 320px wide in light and dark mode. Controls default to
  size `xl` (44px tall).
- **Keep it working offline.** No web fonts, CDNs or runtime fetches beyond `/api`.

## Conventions

- Rust: `rustfmt.toml` sets `max_width = 110`. SQL is written by hand with bound parameters, using
  runtime `sqlx::query` (no compile-time query macros, so no `DATABASE_URL` is needed).
- Tests: unit tests sit in each file's `mod tests`. API tests use `TestApp` (in-memory SQLite, the
  real seed sets, a controllable clock). Test names are sentences that describe behavior, for
  example `cards_enter_review_only_once_their_note_is_studied`.
- Comments and docs are short and plain, and say why rather than what. Match the surrounding
  density.
- Commits: an imperative subject that describes the user-visible change, and a body that
  explains what changed and why. Keep the README current when behavior, endpoints or env vars
  change.
