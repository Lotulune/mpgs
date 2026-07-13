-- Track data-quality check findings for operator review.

CREATE TABLE data_quality_findings (
    finding_id INTEGER PRIMARY KEY AUTOINCREMENT,
    check_name TEXT NOT NULL,
    severity TEXT NOT NULL CHECK (severity IN ('info', 'warn', 'error')),
    app_id INTEGER,
    entity_key TEXT,
    message TEXT NOT NULL,
    details_json TEXT,
    detected_at_ms INTEGER NOT NULL,
    resolved_at_ms INTEGER
);

CREATE INDEX idx_data_quality_open
    ON data_quality_findings (resolved_at_ms, severity, detected_at_ms);
