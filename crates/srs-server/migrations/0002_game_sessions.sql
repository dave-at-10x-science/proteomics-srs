-- Memory game results. Practice only: the scheduler never reads this table.
CREATE TABLE game_sessions (
    id           INTEGER PRIMARY KEY,
    user_id      INTEGER NOT NULL REFERENCES users(id),
    set_id       TEXT NOT NULL,
    pairing      TEXT NOT NULL,                       -- "<field a>:<field b>"
    pairs        INTEGER NOT NULL,
    flips        INTEGER NOT NULL,
    duration_ms  INTEGER NOT NULL,
    completed    INTEGER NOT NULL,
    played_at    TEXT NOT NULL
);
CREATE INDEX game_sessions_user_set ON game_sessions(user_id, set_id);
