# proteomics-srs
Spaced repetition system for learning about proteomic and mass spectrometry

A daily study tool: a short queue of due cards on your phone, scheduled with FSRS, plus a
matching-pairs memory game for practice. Structures are drawn from SMILES. See
[docs/design-proposal.md](docs/design-proposal.md) for the full design.

## Layout

```
crates/srs-core/   Scheduling (FSRS), daily queue, content model, answer checking. No I/O; builds for WASM.
seed/              Starter sets as YAML. The 20 amino acids are the default set.
tools/             Content helpers (SMILES validation and structure previews).
docs/              Design notes.
```

Coming next: `crates/srs-server` (Axum + SQLite, serves the app and API locally) and `web/`
(Nuxt SPA, installable as a PWA).

## Development

```sh
cargo test                     # unit tests, including validation of the seed sets
cargo clippy --all-targets
cargo build -p srs-core --target wasm32-unknown-unknown   # check the engine still builds for the browser
```

Validate SMILES and regenerate structure previews after editing a set:

```sh
pip install rdkit pyyaml
python tools/render_structures.py seed/amino-acids/amino-acids.yaml
```
