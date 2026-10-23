-- Content is shared; everything keyed by user_id is per user. There is one user for now, but
-- per-user tables carry user_id so more can be added without reshaping data.

CREATE TABLE users (
    id                INTEGER PRIMARY KEY,
    name              TEXT NOT NULL,
    new_per_day       INTEGER NOT NULL DEFAULT 10,
    reviews_per_day   INTEGER,                       -- NULL = no cap
    desired_retention REAL NOT NULL DEFAULT 0.9,
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Fields, templates, and depiction templates are stored as one JSON document
-- (srs_core::NoteType) so they can evolve without migrations.
CREATE TABLE note_types (
    id         TEXT PRIMARY KEY,
    definition TEXT NOT NULL
);

CREATE TABLE sets (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    tags       TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE notes (
    id           TEXT PRIMARY KEY,
    note_type_id TEXT NOT NULL REFERENCES note_types(id),
    fields       TEXT NOT NULL,                       -- JSON object of field name -> value
    status       TEXT NOT NULL DEFAULT 'published' CHECK (status IN ('draft', 'published')),
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE set_notes (
    set_id   TEXT NOT NULL REFERENCES sets(id) ON DELETE CASCADE,
    note_id  TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    PRIMARY KEY (set_id, note_id)
);

CREATE TABLE cards (
    note_id     TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    template_id TEXT NOT NULL,
    position    INTEGER NOT NULL,                     -- template order within the note type
    enabled     INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (note_id, template_id)
);

CREATE TABLE subscriptions (
    user_id INTEGER NOT NULL REFERENCES users(id),
    set_id  TEXT NOT NULL REFERENCES sets(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, set_id)
);

-- Absent row = new card.
CREATE TABLE card_state (
    user_id     INTEGER NOT NULL REFERENCES users(id),
    note_id     TEXT NOT NULL,
    template_id TEXT NOT NULL,
    phase       TEXT NOT NULL,
    stability   REAL,
    difficulty  REAL,
    due         TEXT NOT NULL,
    last_review TEXT,
    reps        INTEGER NOT NULL,
    lapses      INTEGER NOT NULL,
    PRIMARY KEY (user_id, note_id, template_id),
    FOREIGN KEY (note_id, template_id) REFERENCES cards(note_id, template_id) ON DELETE CASCADE
);
CREATE INDEX card_state_due ON card_state(user_id, due);

-- Append-only. Enough to recompute schedules and optimise FSRS weights later.
CREATE TABLE review_log (
    id           INTEGER PRIMARY KEY,
    user_id      INTEGER NOT NULL REFERENCES users(id),
    note_id      TEXT NOT NULL,
    template_id  TEXT NOT NULL,
    rating       INTEGER NOT NULL CHECK (rating BETWEEN 1 AND 4),
    phase_before TEXT NOT NULL,
    reviewed_at  TEXT NOT NULL,
    duration_ms  INTEGER,
    answer       TEXT
);
CREATE INDEX review_log_user_time ON review_log(user_id, reviewed_at);

INSERT INTO users (id, name) VALUES (1, 'me');
