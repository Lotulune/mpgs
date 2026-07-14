-- M3 recommendation inputs that are not represented by the core Steam catalog row.

CREATE TABLE app_availability (
    app_id INTEGER PRIMARY KEY REFERENCES apps (app_id),
    platforms_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(platforms_json)),
    languages_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(languages_json)),
    typical_session_minutes_min INTEGER CHECK (
        typical_session_minutes_min IS NULL
        OR typical_session_minutes_min BETWEEN 0 AND 1440
    ),
    typical_session_minutes_max INTEGER CHECK (
        typical_session_minutes_max IS NULL
        OR typical_session_minutes_max BETWEEN 0 AND 1440
    ),
    is_free INTEGER CHECK (is_free IS NULL OR is_free IN (0, 1)),
    updated_at_ms INTEGER NOT NULL,
    CHECK (
        typical_session_minutes_min IS NULL
        OR typical_session_minutes_max IS NULL
        OR typical_session_minutes_min <= typical_session_minutes_max
    )
);

CREATE INDEX idx_price_snapshots_latest
    ON price_snapshots (app_id, currency, captured_at_ms DESC);
