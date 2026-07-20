-- M7 P1: moderation state is bound to an avatar content hash. A replacement
-- image therefore becomes visible without retaining a block on the account.
-- Every block/unblock operation is additionally recorded in audit_events.

CREATE TABLE avatar_moderation (
    user_id TEXT PRIMARY KEY REFERENCES user_accounts (user_id) ON DELETE CASCADE,
    content_hash TEXT NOT NULL,
    blocked_at_ms INTEGER NOT NULL,
    blocked_by TEXT NOT NULL,
    reason TEXT NOT NULL
);

CREATE INDEX idx_avatar_moderation_content
    ON avatar_moderation (content_hash, blocked_at_ms DESC);
