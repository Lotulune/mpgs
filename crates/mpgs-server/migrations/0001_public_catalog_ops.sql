CREATE SCHEMA IF NOT EXISTS public_catalog;
CREATE SCHEMA IF NOT EXISTS ops;

CREATE TABLE IF NOT EXISTS public_catalog.games (
    appid INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    review_status TEXT NOT NULL DEFAULT 'needs_review',
    visibility TEXT NOT NULL DEFAULT 'hidden',
    recommendation_score DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS public_catalog.game_analysis (
    appid INTEGER PRIMARY KEY REFERENCES public_catalog.games(appid) ON DELETE CASCADE,
    report_json JSONB NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS public_catalog.public_catalog_state (
    id BOOLEAN PRIMARY KEY DEFAULT TRUE,
    revision BIGINT NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'empty',
    last_generated_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT public_catalog_state_singleton CHECK (id)
);

INSERT INTO public_catalog.public_catalog_state (id)
VALUES (TRUE)
ON CONFLICT (id) DO NOTHING;

CREATE TABLE IF NOT EXISTS ops.service_config_state (
    id BOOLEAN PRIMARY KEY DEFAULT TRUE,
    active_config_version TEXT,
    pending_config_version TEXT,
    restart_required BOOLEAN NOT NULL DEFAULT FALSE,
    last_startup_status TEXT NOT NULL DEFAULT 'ok',
    last_startup_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT service_config_state_singleton CHECK (id)
);

INSERT INTO ops.service_config_state (id)
VALUES (TRUE)
ON CONFLICT (id) DO NOTHING;
