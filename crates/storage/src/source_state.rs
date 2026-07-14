use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{StorageError, StorageResult};

pub fn load_cursor(conn: &Connection, cursor_key: &str) -> StorageResult<Option<String>> {
    conn.query_row(
        "SELECT cursor_json FROM source_cursors WHERE cursor_key = ?1",
        params![cursor_key],
        |row| row.get(0),
    )
    .optional()
    .map_err(StorageError::from)
}

pub fn save_cursor(
    conn: &Connection,
    cursor_key: &str,
    source: &str,
    cursor_json: &str,
    now_ms: i64,
) -> StorageResult<()> {
    if cursor_key.trim().is_empty() || source.trim().is_empty() {
        return Err(StorageError::validation(
            "cursor_key and source are required",
        ));
    }
    serde_json::from_str::<serde_json::Value>(cursor_json)
        .map_err(|_| StorageError::validation("cursor_json must be valid JSON"))?;
    conn.execute(
        "INSERT INTO source_cursors (
            cursor_key, source, cursor_json, last_success_at_ms, updated_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?4)
         ON CONFLICT(cursor_key) DO UPDATE SET
            source = excluded.source,
            cursor_json = excluded.cursor_json,
            last_success_at_ms = excluded.last_success_at_ms,
            updated_at_ms = excluded.updated_at_ms",
        params![cursor_key, source, cursor_json, now_ms],
    )?;
    Ok(())
}

pub fn start_run(
    conn: &Connection,
    source: &str,
    task_type: &str,
    parser_version: &str,
    notes: Option<&str>,
    now_ms: i64,
) -> StorageResult<i64> {
    for (label, value) in [
        ("source", source),
        ("task_type", task_type),
        ("parser_version", parser_version),
    ] {
        if value.trim().is_empty() {
            return Err(StorageError::validation(format!("{label} is required")));
        }
    }
    conn.execute(
        "INSERT INTO source_runs (
            source, task_type, status, started_at_ms, parser_version, notes
         ) VALUES (?1, ?2, 'running', ?3, ?4, ?5)",
        params![source, task_type, now_ms, parser_version, notes],
    )?;
    Ok(conn.last_insert_rowid())
}

#[allow(clippy::too_many_arguments)]
pub fn finish_run(
    conn: &Connection,
    run_id: i64,
    status: &str,
    request_count: i64,
    success_count: i64,
    error_category: Option<&str>,
    notes: Option<&str>,
    now_ms: i64,
) -> StorageResult<()> {
    if !matches!(status, "succeeded" | "failed" | "partial") {
        return Err(StorageError::validation(
            "source run final status must be succeeded, failed, or partial",
        ));
    }
    if request_count < 0 || success_count < 0 {
        return Err(StorageError::validation(
            "source run counters must be non-negative",
        ));
    }
    let changed = conn.execute(
        "UPDATE source_runs SET
            status = ?1, finished_at_ms = ?2, request_count = ?3,
            success_count = ?4, error_category = ?5, notes = ?6
         WHERE run_id = ?7 AND status = 'running'",
        params![
            status,
            now_ms,
            request_count,
            success_count,
            error_category,
            notes,
            run_id
        ],
    )?;
    if changed != 1 {
        return Err(StorageError::conflict(format!(
            "source run {run_id} is missing or already finalized"
        )));
    }
    Ok(())
}
