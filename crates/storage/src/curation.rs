use rusqlite::{Connection, OptionalExtension, params};

use crate::catalog::{self, set_profile_bool_field, set_profile_text_field};
use crate::error::{StorageError, StorageResult};
use crate::models::{
    CreateOverrideRequest, CurationOverride, EffectiveFeatureValue, FeatureOrigin,
};

pub fn create_override(
    conn: &Connection,
    app_id: u32,
    request: &CreateOverrideRequest,
    now_ms: i64,
) -> StorageResult<CurationOverride> {
    if request.feature_name.trim().is_empty() {
        return Err(StorageError::validation("feature_name is required"));
    }
    if request.reason.trim().is_empty() {
        return Err(StorageError::validation("reason is required"));
    }
    if request.operator.trim().is_empty() {
        return Err(StorageError::validation("operator is required"));
    }

    catalog::ensure_app_stub(conn, app_id, &format!("app-{app_id}"), now_ms)?;
    let value_json = serde_json::to_string(&request.value_json)?;

    // Revoke any existing active override for the same feature before creating a new one.
    conn.execute(
        "UPDATE curation_overrides
         SET revoked_at_ms = ?1, revoked_by = ?2, revoke_reason = 'superseded'
         WHERE app_id = ?3 AND feature_name = ?4 AND revoked_at_ms IS NULL",
        params![now_ms, request.operator, app_id, request.feature_name],
    )?;

    conn.execute(
        "INSERT INTO curation_overrides (
            app_id, feature_name, value_json, reason, external_evidence,
            operator, created_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            app_id,
            request.feature_name,
            value_json,
            request.reason,
            request.external_evidence,
            request.operator,
            now_ms
        ],
    )?;
    let override_id = conn.last_insert_rowid();

    apply_override_to_profile(
        conn,
        app_id,
        &request.feature_name,
        &request.value_json,
        now_ms,
    )?;

    conn.execute(
        "INSERT INTO audit_events (
            actor, action, entity_type, entity_key, before_json, after_json, reason, request_id, created_at_ms
         ) VALUES (?1, 'create_override', 'app_feature', ?2, NULL, ?3, ?4, ?5, ?6)",
        params![
            request.operator,
            format!("{app_id}:{}", request.feature_name),
            value_json,
            request.reason,
            request.request_id,
            now_ms
        ],
    )?;

    get_override(conn, override_id)?
        .ok_or_else(|| StorageError::not_found(format!("override {override_id}")))
}

pub fn revoke_override(
    conn: &Connection,
    override_id: i64,
    operator: &str,
    reason: &str,
    request_id: Option<&str>,
    now_ms: i64,
) -> StorageResult<CurationOverride> {
    let existing = get_override(conn, override_id)?
        .ok_or_else(|| StorageError::not_found(format!("override {override_id}")))?;
    if existing.revoked_at_ms.is_some() {
        return Err(StorageError::conflict(format!(
            "override {override_id} already revoked"
        )));
    }

    conn.execute(
        "UPDATE curation_overrides
         SET revoked_at_ms = ?1, revoked_by = ?2, revoke_reason = ?3
         WHERE override_id = ?4",
        params![now_ms, operator, reason, override_id],
    )?;

    // After revoke, recompute profile field from remaining active override or source evidence.
    recompute_feature_after_revoke(conn, existing.app_id, &existing.feature_name, now_ms)?;

    conn.execute(
        "INSERT INTO audit_events (
            actor, action, entity_type, entity_key, before_json, after_json, reason, request_id, created_at_ms
         ) VALUES (?1, 'revoke_override', 'override', ?2, ?3, NULL, ?4, ?5, ?6)",
        params![
            operator,
            override_id.to_string(),
            existing.value_json,
            reason,
            request_id,
            now_ms
        ],
    )?;

    get_override(conn, override_id)?
        .ok_or_else(|| StorageError::not_found(format!("override {override_id}")))
}

pub fn get_override(
    conn: &Connection,
    override_id: i64,
) -> StorageResult<Option<CurationOverride>> {
    conn.query_row(
        "SELECT override_id, app_id, feature_name, value_json, reason, external_evidence,
                operator, created_at_ms, revoked_at_ms
         FROM curation_overrides WHERE override_id = ?1",
        params![override_id],
        |row| {
            Ok(CurationOverride {
                override_id: row.get(0)?,
                app_id: row.get::<_, i64>(1)? as u32,
                feature_name: row.get(2)?,
                value_json: row.get(3)?,
                reason: row.get(4)?,
                external_evidence: row.get(5)?,
                operator: row.get(6)?,
                created_at_ms: row.get(7)?,
                revoked_at_ms: row.get(8)?,
            })
        },
    )
    .optional()
    .map_err(StorageError::from)
}

pub fn active_override(
    conn: &Connection,
    app_id: u32,
    feature_name: &str,
) -> StorageResult<Option<CurationOverride>> {
    conn.query_row(
        "SELECT override_id, app_id, feature_name, value_json, reason, external_evidence,
                operator, created_at_ms, revoked_at_ms
         FROM curation_overrides
         WHERE app_id = ?1 AND feature_name = ?2 AND revoked_at_ms IS NULL
         ORDER BY created_at_ms DESC, override_id DESC
         LIMIT 1",
        params![app_id, feature_name],
        |row| {
            Ok(CurationOverride {
                override_id: row.get(0)?,
                app_id: row.get::<_, i64>(1)? as u32,
                feature_name: row.get(2)?,
                value_json: row.get(3)?,
                reason: row.get(4)?,
                external_evidence: row.get(5)?,
                operator: row.get(6)?,
                created_at_ms: row.get(7)?,
                revoked_at_ms: row.get(8)?,
            })
        },
    )
    .optional()
    .map_err(StorageError::from)
}

pub fn has_active_override(
    conn: &Connection,
    app_id: u32,
    feature_name: &str,
) -> StorageResult<bool> {
    Ok(active_override(conn, app_id, feature_name)?.is_some())
}

pub fn resolve_effective_feature(
    conn: &Connection,
    app_id: u32,
    feature_name: &str,
) -> StorageResult<EffectiveFeatureValue> {
    if let Some(over) = active_override(conn, app_id, feature_name)? {
        let value_json = serde_json::from_str(&over.value_json)?;
        return Ok(EffectiveFeatureValue {
            app_id,
            feature_name: feature_name.to_owned(),
            value_json,
            origin: FeatureOrigin::HumanOverride,
            override_id: Some(over.override_id),
        });
    }

    if let Some((value_json, _)) = latest_active_evidence(conn, app_id, feature_name)? {
        return Ok(EffectiveFeatureValue {
            app_id,
            feature_name: feature_name.to_owned(),
            value_json: serde_json::from_str(&value_json)?,
            origin: FeatureOrigin::SourceEvidence,
            override_id: None,
        });
    }

    Ok(EffectiveFeatureValue {
        app_id,
        feature_name: feature_name.to_owned(),
        value_json: serde_json::Value::Null,
        origin: FeatureOrigin::Missing,
        override_id: None,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn insert_feature_evidence(
    conn: &Connection,
    app_id: u32,
    feature_name: &str,
    value_json: &serde_json::Value,
    source_type: &str,
    source_ref: &str,
    confidence: f64,
    now_ms: i64,
) -> StorageResult<i64> {
    catalog::ensure_app_stub(conn, app_id, &format!("app-{app_id}"), now_ms)?;
    // Deactivate previous active evidence for same feature/source_type pair is optional;
    // keep history active markers simplified: deactivate older same feature from same source.
    conn.execute(
        "UPDATE feature_evidence SET is_active = 0
         WHERE app_id = ?1 AND feature_name = ?2 AND source_type = ?3 AND is_active = 1",
        params![app_id, feature_name, source_type],
    )?;
    conn.execute(
        "INSERT INTO feature_evidence (
            app_id, feature_name, value_json, source_type, source_ref,
            confidence, observed_at_ms, is_active
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
        params![
            app_id,
            feature_name,
            serde_json::to_string(value_json)?,
            source_type,
            source_ref,
            confidence,
            now_ms
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

fn latest_active_evidence(
    conn: &Connection,
    app_id: u32,
    feature_name: &str,
) -> StorageResult<Option<(String, i64)>> {
    conn.query_row(
        "SELECT value_json, evidence_id FROM feature_evidence
         WHERE app_id = ?1 AND feature_name = ?2 AND is_active = 1
         ORDER BY observed_at_ms DESC, evidence_id DESC
         LIMIT 1",
        params![app_id, feature_name],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(StorageError::from)
}

fn apply_override_to_profile(
    conn: &Connection,
    app_id: u32,
    feature_name: &str,
    value: &serde_json::Value,
    now_ms: i64,
) -> StorageResult<()> {
    match feature_name {
        "private_session" | "online_coop" | "self_hosted_server" | "drop_in_out" | "crossplay" => {
            let b = value.as_bool();
            set_profile_bool_field(conn, app_id, feature_name, b, now_ms)
        }
        "dominant_mode" => {
            let s = value.as_str();
            set_profile_text_field(conn, app_id, feature_name, s, now_ms)
        }
        _ => Ok(()),
    }
}

fn recompute_feature_after_revoke(
    conn: &Connection,
    app_id: u32,
    feature_name: &str,
    now_ms: i64,
) -> StorageResult<()> {
    // Prefer another active human override, else latest source evidence, else NULL.
    if let Some(over) = active_override(conn, app_id, feature_name)? {
        let value: serde_json::Value = serde_json::from_str(&over.value_json)?;
        return apply_override_to_profile(conn, app_id, feature_name, &value, now_ms);
    }
    if let Some((value_json, _)) = latest_active_evidence(conn, app_id, feature_name)? {
        let value: serde_json::Value = serde_json::from_str(&value_json)?;
        return apply_override_to_profile(conn, app_id, feature_name, &value, now_ms);
    }
    apply_override_to_profile(conn, app_id, feature_name, &serde_json::Value::Null, now_ms)
}
