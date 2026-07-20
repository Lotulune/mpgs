-- M7: persistent accounts, multi-device account sessions, public profiles,
-- per-account AI settings, game media, and observable data-refresh state.
-- Existing anonymous subjects remain intact so their preferences, feedback and
-- votes can be merged transactionally when a user registers or signs in.

CREATE TABLE user_accounts (
    user_id TEXT PRIMARY KEY REFERENCES anonymous_users (user_id),
    username_normalized TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    password_scheme TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'frozen', 'deleted')),
    avatar_public_id TEXT NOT NULL UNIQUE,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    CHECK (length(username_normalized) BETWEEN 3 AND 32),
    CHECK (length(display_name) BETWEEN 1 AND 160)
);

CREATE INDEX idx_user_accounts_status ON user_accounts (status, created_at_ms);

CREATE TABLE account_sessions (
    session_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES user_accounts (user_id) ON DELETE CASCADE,
    access_token_hash TEXT NOT NULL UNIQUE,
    refresh_token_hash TEXT NOT NULL UNIQUE,
    device_label TEXT NOT NULL,
    issued_at_ms INTEGER NOT NULL,
    expires_at_ms INTEGER NOT NULL,
    refresh_expires_at_ms INTEGER NOT NULL,
    revoked_at_ms INTEGER,
    replaced_by_session_id TEXT REFERENCES account_sessions (session_id)
);

CREATE INDEX idx_account_sessions_user ON account_sessions (user_id, revoked_at_ms, refresh_expires_at_ms);

CREATE INDEX idx_play_intent_app_created_at
    ON play_intent_votes (app_id, created_at_ms DESC);

CREATE TABLE user_avatars (
    user_id TEXT PRIMARY KEY REFERENCES user_accounts (user_id) ON DELETE CASCADE,
    version INTEGER NOT NULL CHECK (version > 0),
    content_hash TEXT NOT NULL,
    media_type TEXT NOT NULL CHECK (media_type = 'image/webp'),
    storage_key TEXT NOT NULL UNIQUE,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE user_ai_credentials (
    user_id TEXT PRIMARY KEY REFERENCES user_accounts (user_id) ON DELETE CASCADE,
    mode TEXT NOT NULL CHECK (mode IN ('builtin', 'custom', 'off')),
    provider TEXT,
    base_url TEXT,
    model TEXT,
    encrypted_api_key BLOB,
    key_version INTEGER,
    updated_at_ms INTEGER NOT NULL,
    CHECK (
        (mode = 'custom' AND provider IS NOT NULL AND base_url IS NOT NULL AND model IS NOT NULL
             AND encrypted_api_key IS NOT NULL AND key_version IS NOT NULL)
        OR mode IN ('builtin', 'off')
    )
);

-- The account quota is durable across server restarts. It records only a
-- request count, never prompts, model credentials, or response content.
CREATE TABLE account_ai_usage (
    user_id TEXT NOT NULL REFERENCES user_accounts (user_id) ON DELETE CASCADE,
    day_utc INTEGER NOT NULL,
    builtin_requests INTEGER NOT NULL DEFAULT 0 CHECK (builtin_requests >= 0),
    PRIMARY KEY (user_id, day_utc)
);

CREATE TABLE app_media (
    app_id INTEGER PRIMARY KEY REFERENCES apps (app_id) ON DELETE CASCADE,
    capsule_url TEXT,
    source TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE data_refresh_state (
    task_name TEXT PRIMARY KEY,
    last_success_at_ms INTEGER,
    next_run_at_ms INTEGER,
    last_error_category TEXT,
    cursor_value TEXT,
    coverage_ratio REAL CHECK (coverage_ratio IS NULL OR (coverage_ratio >= 0 AND coverage_ratio <= 1)),
    updated_at_ms INTEGER NOT NULL
);

-- Account status changes and avatar changes must invalidate community snapshots
-- even when their vote rows themselves do not change.
CREATE TRIGGER user_accounts_play_intent_revision_after_update
AFTER UPDATE OF status ON user_accounts
WHEN OLD.status <> NEW.status
BEGIN
    UPDATE play_intent_state SET revision = revision + 1 WHERE singleton = 1;
END;

CREATE TRIGGER user_avatars_play_intent_revision_after_insert
AFTER INSERT ON user_avatars
BEGIN
    UPDATE play_intent_state SET revision = revision + 1 WHERE singleton = 1;
END;

CREATE TRIGGER user_avatars_play_intent_revision_after_update
AFTER UPDATE OF version, storage_key ON user_avatars
BEGIN
    UPDATE play_intent_state SET revision = revision + 1 WHERE singleton = 1;
END;

CREATE TRIGGER user_avatars_play_intent_revision_after_delete
AFTER DELETE ON user_avatars
BEGIN
    UPDATE play_intent_state SET revision = revision + 1 WHERE singleton = 1;
END;
