use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{StorageError, StorageResult};
use crate::models::{EnqueueJob, JobRecord};

pub fn enqueue_job(conn: &Connection, job: &EnqueueJob, now_ms: i64) -> StorageResult<i64> {
    if job.idempotency_key.trim().is_empty() {
        return Err(StorageError::validation("idempotency_key is required"));
    }
    // Idempotent enqueue: return existing job id when key already present.
    if let Some(existing) = get_job_by_idempotency(conn, &job.idempotency_key)? {
        return Ok(existing.job_id);
    }

    conn.execute(
        "INSERT INTO jobs (
            source, task_type, entity_key, priority, attempts, max_attempts,
            due_at_ms, status, idempotency_key, payload_json, created_at_ms, updated_at_ms
         ) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, 'pending', ?7, ?8, ?9, ?9)",
        params![
            job.source,
            job.task_type,
            job.entity_key,
            job.priority,
            job.max_attempts,
            job.due_at_ms,
            job.idempotency_key,
            job.payload_json,
            now_ms
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn lease_jobs(
    conn: &Connection,
    owner: &str,
    limit: i64,
    lease_ms: i64,
    now_ms: i64,
    source_filter: Option<&str>,
) -> StorageResult<Vec<JobRecord>> {
    if owner.trim().is_empty() {
        return Err(StorageError::validation("lease owner is required"));
    }
    if limit <= 0 {
        return Ok(Vec::new());
    }

    // Recover expired leases first.
    conn.execute(
        "UPDATE jobs
         SET status = 'pending', lease_owner = NULL, lease_expires_at_ms = NULL, updated_at_ms = ?1
         WHERE status = 'leased' AND lease_expires_at_ms IS NOT NULL AND lease_expires_at_ms < ?1",
        params![now_ms],
    )?;

    let sql = if source_filter.is_some() {
        "SELECT job_id FROM jobs
         WHERE status = 'pending' AND due_at_ms <= ?1 AND source = ?2
         ORDER BY priority ASC, due_at_ms ASC, job_id ASC
         LIMIT ?3"
    } else {
        "SELECT job_id FROM jobs
         WHERE status = 'pending' AND due_at_ms <= ?1
         ORDER BY priority ASC, due_at_ms ASC, job_id ASC
         LIMIT ?2"
    };

    let ids: Vec<i64> = if let Some(source) = source_filter {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params![now_ms, source, limit], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    } else {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params![now_ms, limit], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let mut leased = Vec::new();
    let expires = now_ms.saturating_add(lease_ms);
    for id in ids {
        let changed = conn.execute(
            "UPDATE jobs
             SET status = 'leased', lease_owner = ?1, lease_expires_at_ms = ?2,
                 attempts = attempts + 1, updated_at_ms = ?3
             WHERE job_id = ?4 AND status = 'pending'",
            params![owner, expires, now_ms, id],
        )?;
        if changed == 1
            && let Some(job) = get_job(conn, id)?
        {
            leased.push(job);
        }
    }
    Ok(leased)
}

pub fn complete_job(
    conn: &Connection,
    job_id: i64,
    owner: &str,
    idempotency_key: &str,
    now_ms: i64,
) -> StorageResult<JobRecord> {
    let job =
        get_job(conn, job_id)?.ok_or_else(|| StorageError::not_found(format!("job {job_id}")))?;
    if job.status == "completed" {
        if job.idempotency_key == idempotency_key {
            return Ok(job);
        }
        return Err(StorageError::conflict(
            "job already completed with different idempotency context",
        ));
    }
    if job.status != "leased" {
        return Err(StorageError::lease(format!(
            "job {job_id} is not leased (status={})",
            job.status
        )));
    }
    if job.lease_owner.as_deref() != Some(owner) {
        return Err(StorageError::lease(format!(
            "job {job_id} is leased by another owner"
        )));
    }
    if job.lease_expires_at_ms.is_some_and(|exp| exp < now_ms) {
        return Err(StorageError::lease(format!("job {job_id} lease expired")));
    }

    conn.execute(
        "UPDATE jobs
         SET status = 'completed', lease_owner = NULL, lease_expires_at_ms = NULL, updated_at_ms = ?1
         WHERE job_id = ?2",
        params![now_ms, job_id],
    )?;
    get_job(conn, job_id)?.ok_or_else(|| StorageError::not_found(format!("job {job_id}")))
}

pub fn fail_job(
    conn: &Connection,
    job_id: i64,
    owner: &str,
    error_category: &str,
    retry_delay_ms: i64,
    now_ms: i64,
) -> StorageResult<JobRecord> {
    let job =
        get_job(conn, job_id)?.ok_or_else(|| StorageError::not_found(format!("job {job_id}")))?;
    if job.status != "leased" || job.lease_owner.as_deref() != Some(owner) {
        return Err(StorageError::lease(format!(
            "job {job_id} cannot be failed by {owner}"
        )));
    }

    let permanent = matches!(
        error_category,
        "auth" | "not_found" | "invalid_payload" | "parse_changed"
    );
    let dead = permanent || job.attempts >= job.max_attempts;
    if dead {
        conn.execute(
            "UPDATE jobs
             SET status = 'dead', last_error_category = ?1,
                 lease_owner = NULL, lease_expires_at_ms = NULL, updated_at_ms = ?2
             WHERE job_id = ?3",
            params![error_category, now_ms, job_id],
        )?;
    } else {
        let due = now_ms.saturating_add(retry_delay_ms.max(1));
        conn.execute(
            "UPDATE jobs
             SET status = 'pending', last_error_category = ?1, due_at_ms = ?2,
                 lease_owner = NULL, lease_expires_at_ms = NULL, updated_at_ms = ?3
             WHERE job_id = ?4",
            params![error_category, due, now_ms, job_id],
        )?;
    }
    get_job(conn, job_id)?.ok_or_else(|| StorageError::not_found(format!("job {job_id}")))
}

pub fn get_job(conn: &Connection, job_id: i64) -> StorageResult<Option<JobRecord>> {
    conn.query_row(
        "SELECT job_id, source, task_type, entity_key, priority, attempts, max_attempts,
                due_at_ms, status, lease_owner, lease_expires_at_ms, idempotency_key,
                payload_json, last_error_category
         FROM jobs WHERE job_id = ?1",
        params![job_id],
        map_job,
    )
    .optional()
    .map_err(StorageError::from)
}

pub fn get_job_by_idempotency(conn: &Connection, key: &str) -> StorageResult<Option<JobRecord>> {
    conn.query_row(
        "SELECT job_id, source, task_type, entity_key, priority, attempts, max_attempts,
                due_at_ms, status, lease_owner, lease_expires_at_ms, idempotency_key,
                payload_json, last_error_category
         FROM jobs WHERE idempotency_key = ?1",
        params![key],
        map_job,
    )
    .optional()
    .map_err(StorageError::from)
}

pub fn count_jobs_by_status(conn: &Connection, status: &str) -> StorageResult<i64> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM jobs WHERE status = ?1",
        params![status],
        |row| row.get(0),
    )?;
    Ok(n)
}

fn map_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobRecord> {
    Ok(JobRecord {
        job_id: row.get(0)?,
        source: row.get(1)?,
        task_type: row.get(2)?,
        entity_key: row.get(3)?,
        priority: row.get(4)?,
        attempts: row.get(5)?,
        max_attempts: row.get(6)?,
        due_at_ms: row.get(7)?,
        status: row.get(8)?,
        lease_owner: row.get(9)?,
        lease_expires_at_ms: row.get(10)?,
        idempotency_key: row.get(11)?,
        payload_json: row.get(12)?,
        last_error_category: row.get(13)?,
    })
}
