-- Steam store media gallery: one-to-many screenshots and trailers per app.
-- Cover capsule remains on app_media; this table holds ordered gallery assets.
-- Empty after upgrade is normal; existing covers must keep working.

CREATE TABLE app_media_assets (
    app_id INTEGER NOT NULL REFERENCES apps (app_id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('screenshot', 'movie')),
    source_id TEXT NOT NULL,
    sort_order INTEGER NOT NULL CHECK (sort_order >= 0),
    title TEXT,
    thumbnail_url TEXT NOT NULL,
    full_url TEXT,
    mp4_url TEXT,
    hls_h264_url TEXT,
    dash_h264_url TEXT,
    is_highlight INTEGER NOT NULL DEFAULT 0 CHECK (is_highlight IN (0, 1)),
    source TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (app_id, kind, source_id),
    CHECK (
        (kind = 'screenshot'
            AND full_url IS NOT NULL
            AND mp4_url IS NULL
            AND hls_h264_url IS NULL
            AND dash_h264_url IS NULL)
        OR
        (kind = 'movie'
            AND full_url IS NULL
            AND (mp4_url IS NOT NULL OR hls_h264_url IS NOT NULL OR dash_h264_url IS NOT NULL))
    )
);

CREATE INDEX idx_app_media_assets_order
    ON app_media_assets (app_id, kind, sort_order, source_id);
