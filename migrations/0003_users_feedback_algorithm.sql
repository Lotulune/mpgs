-- Anonymous users, preferences, feedback, and algorithm config for public API (M3).

CREATE TABLE anonymous_users (
    user_id TEXT PRIMARY KEY,
    created_at_ms INTEGER NOT NULL,
    last_active_at_ms INTEGER NOT NULL,
    access_token_hash TEXT NOT NULL UNIQUE,
    refresh_token_hash TEXT NOT NULL UNIQUE
);

CREATE TABLE user_preferences (
    user_id TEXT PRIMARY KEY REFERENCES anonymous_users (user_id),
    version INTEGER NOT NULL DEFAULT 1,
    party_size INTEGER NOT NULL CHECK (party_size BETWEEN 1 AND 64),
    coop_competitive REAL NOT NULL CHECK (coop_competitive BETWEEN 0 AND 1),
    session_minutes_min INTEGER NOT NULL,
    session_minutes_max INTEGER NOT NULL,
    budget_currency TEXT NOT NULL,
    budget_max_each_minor INTEGER,
    platforms_json TEXT NOT NULL,
    self_hosting_willingness REAL NOT NULL CHECK (self_hosting_willingness BETWEEN 0 AND 1),
    languages_json TEXT NOT NULL,
    excluded_modes_json TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    CHECK (session_minutes_min <= session_minutes_max)
);

CREATE TABLE feedback_events (
    feedback_id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL REFERENCES anonymous_users (user_id),
    app_id INTEGER NOT NULL REFERENCES apps (app_id),
    feedback_type TEXT NOT NULL CHECK (
        feedback_type IN (
            'like', 'not_interested', 'played', 'too_competitive',
            'party_size_mismatch', 'hosting_friction', 'undo'
        )
    ),
    recommendation_run_id TEXT,
    idempotency_key TEXT NOT NULL,
    client_created_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL,
    undone_by INTEGER,
    UNIQUE (user_id, idempotency_key)
);

CREATE INDEX idx_feedback_user_app ON feedback_events (user_id, app_id, created_at_ms);

CREATE TABLE algorithm_configs (
    version TEXT PRIMARY KEY,
    schema_version INTEGER NOT NULL,
    config_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('draft', 'active', 'retired')),
    created_by TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE UNIQUE INDEX idx_algorithm_one_active
    ON algorithm_configs (status)
    WHERE status = 'active';
