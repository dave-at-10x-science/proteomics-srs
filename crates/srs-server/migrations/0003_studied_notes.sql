-- Notes the learner has read through in study mode. A note's cards only enter review rounds once
-- it is studied.
CREATE TABLE studied_notes (
    user_id     INTEGER NOT NULL REFERENCES users(id),
    note_id     TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    studied_at  TEXT NOT NULL,
    PRIMARY KEY (user_id, note_id)
);

-- Anything already reviewed counts as studied.
INSERT INTO studied_notes (user_id, note_id, studied_at)
SELECT user_id, note_id, COALESCE(MIN(last_review), MIN(due)) FROM card_state GROUP BY user_id, note_id;
