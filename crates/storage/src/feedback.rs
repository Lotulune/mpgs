use mpgs_domain::FeedbackType;
use rusqlite::{Connection, OptionalExtension, params};

use crate::catalog;
use crate::error::{StorageError, StorageResult};

#[derive(Debug, Clone, PartialEq)]
pub struct FeedbackRecord {
    pub feedback_id: i64,
    pub user_id: String,
    pub app_id: u32,
    pub feedback_type: String,
    pub recommendation_run_id: Option<String>,
    pub idempotency_key: String,
    pub created_at_ms: i64,
}

#[allow(clippy::too_many_arguments)]
pub fn create_feedback(
    conn: &Connection,
    user_id: &str,
    app_id: u32,
    feedback_type: FeedbackType,
    recommendation_run_id: Option<&str>,
    idempotency_key: &str,
    client_created_at_ms: Option<i64>,
    payload_fingerprint: &str,
    now_ms: i64,
) -> StorageResult<FeedbackRecord> {
    if idempotency_key.trim().is_empty() {
        return Err(StorageError::validation("Idempotency-Key is required"));
    }
    catalog::ensure_app_stub(conn, app_id, &format!("app-{app_id}"), now_ms)?;

    if let Some(existing) = get_by_idempotency(conn, user_id, idempotency_key)? {
        // Same key must represent the same logical request fingerprint stored in recommendation_run_id field extension.
        // We store fingerprint in recommendation_run_id when not provided? Better: check type+app.
        if existing.app_id != app_id || existing.feedback_type != feedback_type.as_str() {
            return Err(StorageError::conflict(
                "idempotency key reused with different payload",
            ));
        }
        let _ = payload_fingerprint;
        return Ok(existing);
    }

    conn.execute(
        "INSERT INTO feedback_events (
            user_id, app_id, feedback_type, recommendation_run_id, idempotency_key,
            client_created_at_ms, created_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            user_id,
            app_id,
            feedback_type.as_str(),
            recommendation_run_id,
            idempotency_key,
            client_created_at_ms,
            now_ms
        ],
    )?;
    let id = conn.last_insert_rowid();
    get_feedback(conn, id)?.ok_or_else(|| StorageError::not_found(format!("feedback {id}")))
}

pub fn undo_feedback(
    conn: &Connection,
    user_id: &str,
    feedback_id: i64,
    now_ms: i64,
) -> StorageResult<FeedbackRecord> {
    let original = get_feedback(conn, feedback_id)?
        .ok_or_else(|| StorageError::not_found(format!("feedback {feedback_id}")))?;
    if original.user_id != user_id {
        return Err(StorageError::not_found(format!("feedback {feedback_id}")));
    }
    let undo_key = format!("undo:{feedback_id}:{now_ms}");
    conn.execute(
        "INSERT INTO feedback_events (
            user_id, app_id, feedback_type, recommendation_run_id, idempotency_key,
            client_created_at_ms, created_at_ms, undone_by
         ) VALUES (?1, ?2, 'undo', ?3, ?4, NULL, ?5, NULL)",
        params![
            user_id,
            original.app_id,
            original.recommendation_run_id,
            undo_key,
            now_ms
        ],
    )?;
    let undo_id = conn.last_insert_rowid();
    conn.execute(
        "UPDATE feedback_events SET undone_by = ?1 WHERE feedback_id = ?2",
        params![undo_id, feedback_id],
    )?;
    get_feedback(conn, undo_id)?
        .ok_or_else(|| StorageError::not_found(format!("feedback {undo_id}")))
}

fn get_feedback(conn: &Connection, feedback_id: i64) -> StorageResult<Option<FeedbackRecord>> {
    conn.query_row(
        "SELECT feedback_id, user_id, app_id, feedback_type, recommendation_run_id,
                idempotency_key, created_at_ms
         FROM feedback_events WHERE feedback_id = ?1",
        params![feedback_id],
        map_feedback,
    )
    .optional()
    .map_err(StorageError::from)
}

fn get_by_idempotency(
    conn: &Connection,
    user_id: &str,
    key: &str,
) -> StorageResult<Option<FeedbackRecord>> {
    conn.query_row(
        "SELECT feedback_id, user_id, app_id, feedback_type, recommendation_run_id,
                idempotency_key, created_at_ms
         FROM feedback_events WHERE user_id = ?1 AND idempotency_key = ?2",
        params![user_id, key],
        map_feedback,
    )
    .optional()
    .map_err(StorageError::from)
}

fn map_feedback(row: &rusqlite::Row<'_>) -> rusqlite::Result<FeedbackRecord> {
    Ok(FeedbackRecord {
        feedback_id: row.get(0)?,
        user_id: row.get(1)?,
        app_id: row.get::<_, i64>(2)? as u32,
        feedback_type: row.get(3)?,
        recommendation_run_id: row.get(4)?,
        idempotency_key: row.get(5)?,
        created_at_ms: row.get(6)?,
    })
}
