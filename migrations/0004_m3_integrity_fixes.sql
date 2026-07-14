-- M3 correctness and security hardening. Existing deterministic sessions expire on upgrade.

ALTER TABLE anonymous_users
    ADD COLUMN access_expires_at_ms INTEGER NOT NULL DEFAULT 0;

ALTER TABLE anonymous_users
    ADD COLUMN refresh_expires_at_ms INTEGER NOT NULL DEFAULT 0;

CREATE INDEX idx_anonymous_users_access_expiry
    ON anonymous_users (access_token_hash, access_expires_at_ms);

CREATE INDEX idx_anonymous_users_refresh_expiry
    ON anonymous_users (refresh_token_hash, refresh_expires_at_ms);

ALTER TABLE feedback_events
    ADD COLUMN request_fingerprint TEXT NOT NULL DEFAULT '';

ALTER TABLE jobs
    ADD COLUMN completion_idempotency_key TEXT;

ALTER TABLE apps
    ADD COLUMN release_date_raw TEXT;

-- A missing observation is still a sample; older code stored the first one as count zero.
UPDATE player_daily
SET sample_count = 1
WHERE sample_count = 0 AND missing_rate = 1;

-- appdetails used to write localized display dates into the ISO date column.
UPDATE apps
SET release_date_raw = release_date,
    release_date = NULL,
    release_date_precision = COALESCE(release_date_precision, 'tba')
WHERE release_date IS NOT NULL
  AND (
      length(release_date) <> 10
      OR release_date NOT GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
  );
