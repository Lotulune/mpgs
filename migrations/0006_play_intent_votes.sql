-- Community "play intent" votes (M4+): one toggleable vote per (app, user).
-- Aggregated count feeds a versioned ranking signal so more-wanted games rank higher.

CREATE TABLE play_intent_votes (
    app_id INTEGER NOT NULL REFERENCES apps (app_id),
    user_id TEXT NOT NULL REFERENCES anonymous_users (user_id),
    created_at_ms INTEGER NOT NULL,
    PRIMARY KEY (app_id, user_id)
);

CREATE INDEX idx_play_intent_app ON play_intent_votes (app_id);
CREATE INDEX idx_play_intent_user ON play_intent_votes (user_id);

-- A monotonic revision avoids cache/cursor collisions caused by COUNT/MAX
-- epochs (for example, a withdrawal followed by a vote in the same millisecond).
CREATE TABLE play_intent_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    revision INTEGER NOT NULL CHECK (revision >= 0)
);

INSERT INTO play_intent_state (singleton, revision) VALUES (1, 0);

CREATE TRIGGER play_intent_revision_after_insert
AFTER INSERT ON play_intent_votes
BEGIN
    UPDATE play_intent_state SET revision = revision + 1 WHERE singleton = 1;
END;

CREATE TRIGGER play_intent_revision_after_delete
AFTER DELETE ON play_intent_votes
BEGIN
    UPDATE play_intent_state SET revision = revision + 1 WHERE singleton = 1;
END;
