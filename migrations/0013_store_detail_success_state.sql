CREATE TABLE store_detail_refresh_state_v2 (
    app_id INTEGER NOT NULL REFERENCES apps (app_id),
    country_code TEXT NOT NULL,
    language TEXT NOT NULL,
    captured_at_ms INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('succeeded', 'not_found')),
    source TEXT NOT NULL,
    PRIMARY KEY (app_id, country_code, language)
);

INSERT INTO store_detail_refresh_state_v2(
    app_id, country_code, language, captured_at_ms, status, source
)
SELECT app_id, country_code, language, captured_at_ms, status, source
FROM store_detail_refresh_state;

DROP TABLE store_detail_refresh_state;
ALTER TABLE store_detail_refresh_state_v2 RENAME TO store_detail_refresh_state;

CREATE INDEX idx_store_detail_refresh_due
    ON store_detail_refresh_state(country_code, language, captured_at_ms, status);
