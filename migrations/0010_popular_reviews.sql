CREATE TABLE popular_reviews (
    app_id INTEGER NOT NULL REFERENCES apps(app_id) ON DELETE CASCADE,
    recommendation_id TEXT NOT NULL,
    rank INTEGER NOT NULL CHECK(rank BETWEEN 1 AND 10),
    language TEXT NOT NULL,
    author_name TEXT,
    author_profile_url TEXT,
    review_text TEXT NOT NULL,
    voted_up INTEGER NOT NULL CHECK(voted_up IN (0, 1)),
    votes_up INTEGER NOT NULL DEFAULT 0 CHECK(votes_up >= 0),
    votes_funny INTEGER NOT NULL DEFAULT 0 CHECK(votes_funny >= 0),
    comment_count INTEGER NOT NULL DEFAULT 0 CHECK(comment_count >= 0),
    playtime_forever_minutes INTEGER,
    playtime_at_review_minutes INTEGER,
    created_at_s INTEGER NOT NULL,
    updated_at_s INTEGER NOT NULL,
    steam_purchase INTEGER NOT NULL CHECK(steam_purchase IN (0, 1)),
    received_for_free INTEGER NOT NULL CHECK(received_for_free IN (0, 1)),
    written_during_early_access INTEGER NOT NULL CHECK(written_during_early_access IN (0, 1)),
    parameter_hash TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    source TEXT NOT NULL,
    captured_at_ms INTEGER NOT NULL,
    PRIMARY KEY (app_id, recommendation_id)
);

CREATE UNIQUE INDEX idx_popular_reviews_app_rank
    ON popular_reviews(app_id, rank);

CREATE INDEX idx_popular_reviews_freshness
    ON popular_reviews(app_id, captured_at_ms DESC);
