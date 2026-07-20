CREATE TABLE popular_review_refresh_state (
    app_id INTEGER PRIMARY KEY REFERENCES apps (app_id),
    captured_at_ms INTEGER NOT NULL,
    result_count INTEGER NOT NULL CHECK (result_count >= 0),
    source TEXT NOT NULL
);

INSERT INTO popular_review_refresh_state(app_id, captured_at_ms, result_count, source)
SELECT app_id, MAX(captured_at_ms), COUNT(*), 'steam_reviews'
FROM popular_reviews
GROUP BY app_id;

CREATE TABLE store_detail_refresh_state (
    app_id INTEGER NOT NULL REFERENCES apps (app_id),
    country_code TEXT NOT NULL,
    language TEXT NOT NULL,
    captured_at_ms INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('not_found')),
    source TEXT NOT NULL,
    PRIMARY KEY (app_id, country_code, language)
);

CREATE INDEX idx_store_detail_refresh_due
    ON store_detail_refresh_state(country_code, language, captured_at_ms, status);
