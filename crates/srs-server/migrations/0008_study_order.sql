-- Keep the existing order until a learner chooses to mix concepts across their sets.
ALTER TABLE users ADD COLUMN study_order TEXT NOT NULL DEFAULT 'set_order'
    CHECK (study_order IN ('set_order', 'random'));
