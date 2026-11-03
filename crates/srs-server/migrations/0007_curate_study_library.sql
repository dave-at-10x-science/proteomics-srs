-- Keep superseded seed sets and their memberships for history without offering them to learners.
ALTER TABLE sets ADD COLUMN retired INTEGER NOT NULL DEFAULT 0;
