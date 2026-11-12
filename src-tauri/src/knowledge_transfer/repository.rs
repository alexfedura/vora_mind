use super::*;
use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const KNOWLEDGE_TRANSFER_SCHEMA: &str = include_str!("schema.sql");
const KNOWLEDGE_TRANSFER_MIGRATION_ID: &str = "phase7-knowledge-transfer-001";

fn id() -> String {
    Uuid::now_v7().to_string()
}

fn now_us() -> Result<i64> {
    i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros())
        .context("system timestamp exceeds the supported range")
}

fn sha256(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

fn required(value: &str, name: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{name} must not be empty.");
    }
    Ok(())
}

fn ensure_exists(conn: &Connection, sql: &str, value: &str, message: &str) -> Result<()> {
    let found: bool = conn.query_row(sql, [value], |row| row.get(0))?;
    if !found {
        bail!("{message}");
    }
    Ok(())
}

fn encode_json<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

fn decode_json<T: DeserializeOwned>(value: &str) -> rusqlite::Result<T> {
    serde_json::from_str(value).map_err(|_| rusqlite::Error::InvalidQuery)
}

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS knowledge_transfer_schema_migrations (
             version INTEGER PRIMARY KEY,
             migration_id TEXT NOT NULL UNIQUE,
             checksum TEXT NOT NULL,
             applied_at_us INTEGER NOT NULL
         );",
    )?;
    let foreign_keys_enabled: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    if foreign_keys_enabled != 1 {
        bail!("SQLite foreign-key enforcement is required for Knowledge Transfer.");
    }

    conn.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| -> Result<()> {
        let checksum = sha256(KNOWLEDGE_TRANSFER_SCHEMA.as_bytes());
        let applied: Option<(String, String)> = conn
            .query_row(
                "SELECT migration_id, checksum
                 FROM knowledge_transfer_schema_migrations WHERE version = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((migration_id, applied_checksum)) = applied {
            if migration_id != KNOWLEDGE_TRANSFER_MIGRATION_ID || applied_checksum != checksum {
                bail!("Knowledge Transfer migration 1 differs from the applied migration; add a new numbered migration.");
            }
            return Ok(());
        }

        conn.execute_batch(KNOWLEDGE_TRANSFER_SCHEMA)?;
        conn.execute(
            "INSERT INTO knowledge_transfer_schema_migrations
                (version, migration_id, checksum, applied_at_us)
             VALUES (1, ?1, ?2, ?3)",
            params![KNOWLEDGE_TRANSFER_MIGRATION_ID, checksum, now_us()?],
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => conn.execute_batch("COMMIT")?,
        Err(error) => {
            conn.execute_batch("ROLLBACK")?;
            return Err(error);
        }
    }
    Ok(())
}

#[derive(Debug)]
struct ArtifactInfo {
    title: String,
    version_no: i64,
    owner_project_id: Option<String>,
    lifecycle_status: String,
}

fn artifact_info(conn: &Connection, artifact: &TransferArtifactRef) -> Result<ArtifactInfo> {
    let info: Option<ArtifactInfo> = match artifact {
        TransferArtifactRef::FindingVersion {
            finding_id,
            version_id,
        } => conn
            .query_row(
                "SELECT v.title, v.version_no, f.project_id,
                    COALESCE((SELECT e.status FROM analysis_finding_events e
                              WHERE e.finding_version_id = v.id
                              ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1), 'unreviewed')
                 FROM analysis_finding_versions v
                 JOIN analysis_findings f ON f.id = v.finding_id
                 WHERE v.id = ?1 AND v.finding_id = ?2",
                params![version_id, finding_id],
                |row| {
                    Ok(ArtifactInfo {
                        title: row.get(0)?,
                        version_no: row.get(1)?,
                        owner_project_id: row.get(2)?,
                        lifecycle_status: row.get(3)?,
                    })
                },
            )
            .optional()?,
        TransferArtifactRef::ConceptVersion {
            concept_id,
            version_id,
        } => conn
            .query_row(
                "SELECT v.title, v.version_no, v.created_in_project_id,
                    COALESCE((SELECT e.status FROM knowledge_concept_version_events e
                              WHERE e.version_id = v.id
                              ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1), 'unreviewed')
                 FROM knowledge_concept_versions v
                 WHERE v.id = ?1 AND v.concept_id = ?2",
                params![version_id, concept_id],
                |row| {
                    Ok(ArtifactInfo {
                        title: row.get(0)?,
                        version_no: row.get(1)?,
                        owner_project_id: row.get(2)?,
                        lifecycle_status: row.get(3)?,
                    })
                },
            )
            .optional()?,
        TransferArtifactRef::PrincipleVersion {
            principle_id,
            version_id,
        } => conn
            .query_row(
                "SELECT v.title, v.version_no, v.created_in_project_id,
                    COALESCE((SELECT e.status FROM knowledge_principle_version_events e
                              WHERE e.version_id = v.id
                              ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1), 'unreviewed')
                 FROM knowledge_principle_versions v
                 WHERE v.id = ?1 AND v.principle_id = ?2",
                params![version_id, principle_id],
                |row| {
                    Ok(ArtifactInfo {
                        title: row.get(0)?,
                        version_no: row.get(1)?,
                        owner_project_id: row.get(2)?,
                        lifecycle_status: row.get(3)?,
                    })
                },
            )
            .optional()?,
        TransferArtifactRef::FrameworkVersion {
            framework_id,
            version_id,
        } => conn
            .query_row(
                "SELECT v.title, v.version_no, NULL, v.status
                 FROM knowledge_framework_versions v
                 WHERE v.id = ?1 AND v.framework_id = ?2",
                params![version_id, framework_id],
                |row| {
                    Ok(ArtifactInfo {
                        title: row.get(0)?,
                        version_no: row.get(1)?,
                        owner_project_id: row.get(2)?,
                        lifecycle_status: row.get(3)?,
                    })
                },
            )
            .optional()?,
        TransferArtifactRef::ModelVersion {
            model_id,
            version_id,
        } => conn
            .query_row(
                "SELECT m.display_name || ' v' || v.version_no, v.version_no,
                        COALESCE(v.created_in_project_id, m.origin_project_id),
                        COALESCE((SELECT e.lifecycle_status FROM model_version_events e
                                  WHERE e.model_version_id = v.id
                                  ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1), 'unreviewed')
                 FROM model_versions v
                 JOIN scientific_models m ON m.id = v.model_id
                 WHERE v.id = ?1 AND v.model_id = ?2",
                params![version_id, model_id],
                |row| {
                    Ok(ArtifactInfo {
                        title: row.get(0)?,
                        version_no: row.get(1)?,
                        owner_project_id: row.get(2)?,
                        lifecycle_status: row.get(3)?,
                    })
                },
            )
            .optional()?,
    };
    info.ok_or_else(|| {
        anyhow!(
            "The exact {:?} artifact version was not found.",
            artifact.kind()
        )
    })
}

fn assert_transferable(artifact: &TransferArtifactRef, info: &ArtifactInfo) -> Result<()> {
    let allowed = match artifact.kind() {
        TransferArtifactKind::FindingVersion => matches!(
            info.lifecycle_status.as_str(),
            "accepted" | "contested" | "unsupported" | "inconclusive" | "superseded"
        ),
        TransferArtifactKind::ConceptVersion | TransferArtifactKind::PrincipleVersion => {
            info.lifecycle_status == "reviewed"
        }
        TransferArtifactKind::FrameworkVersion => info.lifecycle_status == "published",
        TransferArtifactKind::ModelVersion => info.lifecycle_status == "released",
    };
    if !allowed {
        bail!(
            "This exact artifact version is not transferable in its current state ({:?}).",
            info.lifecycle_status
        );
    }
    Ok(())
}

fn next_version_number(conn: &Connection, sql: &str, identity: &str) -> Result<i64> {
    Ok(conn.query_row(sql, [identity], |row| row.get::<_, i64>(0))? + 1)
}

pub fn create_concept_version(
    conn: &mut Connection,
    request: &CreateConceptVersionReq,
) -> Result<String> {
    required(&request.project_id, "Project")?;
    required(&request.title, "Concept title")?;
    required(&request.definition, "Concept definition")?;
    required(&request.rationale, "Version rationale")?;
    ensure_exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
        &request.project_id,
        "Research project was not found.",
    )?;

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let concept_id = request.concept_id.clone().unwrap_or_else(id);
    if let Some(parent_id) = &request.parent_version_id {
        let parent_project: Option<String> = tx
            .query_row(
                "SELECT created_in_project_id FROM knowledge_concept_versions
                 WHERE id = ?1 AND concept_id = ?2",
                params![parent_id, concept_id],
                |row| row.get(0),
            )
            .optional()?;
        if parent_project.as_deref() != Some(request.project_id.as_str()) {
            bail!("A concept version parent must be an existing version of the same concept in the same project.");
        }
    } else if request.concept_id.is_some() {
        let has_versions: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM knowledge_concept_versions WHERE concept_id = ?1)",
            [&concept_id],
            |row| row.get(0),
        )?;
        if has_versions {
            bail!("A non-initial concept version must reference its parent version.");
        }
    }

    let now = now_us()?;
    if request.concept_id.is_none() {
        tx.execute(
            "INSERT INTO knowledge_concepts (id, title, definition, lifecycle, created_at_us)
             VALUES (?1, ?2, ?3, 'proposed', ?4)",
            params![
                concept_id,
                request.title.trim(),
                request.definition.trim(),
                now
            ],
        )?;
    }
    let version_no = next_version_number(
        &tx,
        "SELECT COALESCE(MAX(version_no), 0) FROM knowledge_concept_versions WHERE concept_id = ?1",
        &concept_id,
    )?;
    let version_id = id();
    tx.execute(
        "INSERT INTO knowledge_concept_versions
            (id, concept_id, version_no, created_in_project_id, title, definition,
             parent_version_id, rationale, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            version_id,
            concept_id,
            version_no,
            request.project_id,
            request.title.trim(),
            request.definition.trim(),
            request.parent_version_id,
            request.rationale.trim(),
            request.created_by,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO knowledge_concept_version_events
            (id, version_id, status, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, 'draft', ?3, ?4, ?5)",
        params![
            id(),
            version_id,
            request.rationale.trim(),
            request.created_by,
            now
        ],
    )?;
    tx.commit()?;
    Ok(version_id)
}

pub fn create_principle_version(
    conn: &mut Connection,
    request: &CreatePrincipleVersionReq,
) -> Result<String> {
    required(&request.project_id, "Project")?;
    required(&request.title, "Principle title")?;
    required(&request.statement, "Principle statement")?;
    required(&request.rationale, "Version rationale")?;
    ensure_exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
        &request.project_id,
        "Research project was not found.",
    )?;

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let principle_id = request.principle_id.clone().unwrap_or_else(id);
    if let Some(parent_id) = &request.parent_version_id {
        let parent_project: Option<String> = tx
            .query_row(
                "SELECT created_in_project_id FROM knowledge_principle_versions
                 WHERE id = ?1 AND principle_id = ?2",
                params![parent_id, principle_id],
                |row| row.get(0),
            )
            .optional()?;
        if parent_project.as_deref() != Some(request.project_id.as_str()) {
            bail!("A principle version parent must be an existing version of the same principle in the same project.");
        }
    } else if request.principle_id.is_some() {
        let has_versions: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM knowledge_principle_versions WHERE principle_id = ?1)",
            [&principle_id],
            |row| row.get(0),
        )?;
        if has_versions {
            bail!("A non-initial principle version must reference its parent version.");
        }
    }

    let now = now_us()?;
    if request.principle_id.is_none() {
        tx.execute(
            "INSERT INTO knowledge_principles (id, title, statement, lifecycle, created_at_us)
             VALUES (?1, ?2, ?3, 'proposed', ?4)",
            params![
                principle_id,
                request.title.trim(),
                request.statement.trim(),
                now
            ],
        )?;
    }
    let version_no = next_version_number(
        &tx,
        "SELECT COALESCE(MAX(version_no), 0) FROM knowledge_principle_versions WHERE principle_id = ?1",
        &principle_id,
    )?;
    let version_id = id();
    tx.execute(
        "INSERT INTO knowledge_principle_versions
            (id, principle_id, version_no, created_in_project_id, title, statement,
             parent_version_id, rationale, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            version_id,
            principle_id,
            version_no,
            request.project_id,
            request.title.trim(),
            request.statement.trim(),
            request.parent_version_id,
            request.rationale.trim(),
            request.created_by,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO knowledge_principle_version_events
            (id, version_id, status, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, 'draft', ?3, ?4, ?5)",
        params![
            id(),
            version_id,
            request.rationale.trim(),
            request.created_by,
            now
        ],
    )?;
    tx.commit()?;
    Ok(version_id)
}

pub fn review_knowledge_version(
    conn: &mut Connection,
    request: &ReviewKnowledgeVersionReq,
) -> Result<()> {
    if !matches!(
        request.status,
        KnowledgeVersionStatus::Reviewed | KnowledgeVersionStatus::Deprecated
    ) {
        bail!("Knowledge versions can only be moved to reviewed or deprecated through this operation.");
    }
    required(&request.rationale, "Review rationale")?;
    required(request.version_id.as_str(), "Version")?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let (table, event_table, status_column) = match request.kind {
        KnowledgeVersionKind::Concept => (
            "knowledge_concept_versions",
            "knowledge_concept_version_events",
            "status",
        ),
        KnowledgeVersionKind::Principle => (
            "knowledge_principle_versions",
            "knowledge_principle_version_events",
            "status",
        ),
    };
    let version_exists: bool = tx.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?1)"),
        [&request.version_id],
        |row| row.get(0),
    )?;
    if !version_exists {
        bail!("Knowledge version was not found.");
    }
    let current_status: String = tx.query_row(
        &format!(
            "SELECT {status_column} FROM {event_table}
             WHERE version_id = ?1 ORDER BY occurred_at_us DESC, id DESC LIMIT 1"
        ),
        [&request.version_id],
        |row| row.get(0),
    )?;
    if current_status != "draft" {
        bail!("Only a draft knowledge version can be reviewed or superseded.");
    }
    tx.execute(
        &format!(
            "INSERT INTO {event_table}
                (id, version_id, status, rationale, actor_id, occurred_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        ),
        params![
            id(),
            request.version_id,
            request.status.as_str(),
            request.rationale.trim(),
            request.actor_id,
            now_us()?
        ],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn create_origin(
    conn: &mut Connection,
    request: &CreateTransferOriginReq,
) -> Result<KnowledgeTransferOriginDto> {
    required(&request.origin_project_id, "Origin project")?;
    required(&request.rationale, "Origin rationale")?;
    ensure_exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
        &request.origin_project_id,
        "Origin research project was not found.",
    )?;
    let info = artifact_info(conn, &request.artifact)?;
    assert_transferable(&request.artifact, &info)?;
    if let Some(owner_project_id) = &info.owner_project_id {
        if owner_project_id != &request.origin_project_id {
            bail!("Origin project does not own this exact artifact version.");
        }
    }
    let origin_id = id();
    let now = now_us()?;
    match &request.artifact {
        TransferArtifactRef::FindingVersion { version_id, .. } => conn.execute(
            "INSERT INTO knowledge_transfer_origins
                (id, artifact_kind, finding_version_id, origin_project_id, rationale, recorded_by, recorded_at_us)
             VALUES (?1, 'finding_version', ?2, ?3, ?4, ?5, ?6)",
            params![origin_id, version_id, request.origin_project_id, request.rationale.trim(), request.recorded_by, now],
        )?,
        TransferArtifactRef::ConceptVersion { version_id, .. } => conn.execute(
            "INSERT INTO knowledge_transfer_origins
                (id, artifact_kind, concept_version_id, origin_project_id, rationale, recorded_by, recorded_at_us)
             VALUES (?1, 'concept_version', ?2, ?3, ?4, ?5, ?6)",
            params![origin_id, version_id, request.origin_project_id, request.rationale.trim(), request.recorded_by, now],
        )?,
        TransferArtifactRef::PrincipleVersion { version_id, .. } => conn.execute(
            "INSERT INTO knowledge_transfer_origins
                (id, artifact_kind, principle_version_id, origin_project_id, rationale, recorded_by, recorded_at_us)
             VALUES (?1, 'principle_version', ?2, ?3, ?4, ?5, ?6)",
            params![origin_id, version_id, request.origin_project_id, request.rationale.trim(), request.recorded_by, now],
        )?,
        TransferArtifactRef::FrameworkVersion { version_id, .. } => conn.execute(
            "INSERT INTO knowledge_transfer_origins
                (id, artifact_kind, framework_version_id, origin_project_id, rationale, recorded_by, recorded_at_us)
             VALUES (?1, 'framework_version', ?2, ?3, ?4, ?5, ?6)",
            params![origin_id, version_id, request.origin_project_id, request.rationale.trim(), request.recorded_by, now],
        )?,
        TransferArtifactRef::ModelVersion { version_id, .. } => conn.execute(
            "INSERT INTO knowledge_transfer_origins
                (id, artifact_kind, model_version_id, origin_project_id, rationale, recorded_by, recorded_at_us)
             VALUES (?1, 'model_version', ?2, ?3, ?4, ?5, ?6)",
            params![origin_id, version_id, request.origin_project_id, request.rationale.trim(), request.recorded_by, now],
        )?,
    };
    query_origin(conn, &origin_id)
}

pub fn create_eligibility(
    conn: &mut Connection,
    request: &CreateTransferEligibilityReq,
) -> Result<TransferEligibilityDto> {
    required(&request.origin_id, "Transfer origin")?;
    required(&request.license_identifier, "License identifier")?;
    required(&request.attribution_text, "Attribution text")?;
    required(&request.permission_basis, "Permission basis")?;
    required(&request.reviewed_by, "Rights reviewer")?;
    if request
        .license_identifier
        .trim()
        .eq_ignore_ascii_case("unknown")
    {
        bail!("Unknown rights must be resolved before enabling transfer.");
    }
    if !request.permission_granted {
        bail!("Transfer eligibility requires an explicit permission grant.");
    }
    if request.permitted_purposes.is_empty() {
        bail!("At least one permitted purpose is required.");
    }
    let mut purposes = request
        .permitted_purposes
        .iter()
        .map(|purpose| purpose.as_str())
        .collect::<Vec<_>>();
    purposes.sort_unstable();
    purposes.dedup();
    if purposes.len() != request.permitted_purposes.len() {
        bail!("Permitted purposes cannot contain duplicates.");
    }
    let mut project_ids = request.allowed_project_ids.clone();
    project_ids.sort();
    project_ids.dedup();
    if project_ids.len() != request.allowed_project_ids.len() {
        bail!("Recipient project list cannot contain duplicates.");
    }
    match request.access_scope {
        TransferAccessScope::Workspace if !project_ids.is_empty() => {
            bail!("Workspace-wide eligibility cannot also contain a project allowlist.")
        }
        TransferAccessScope::SelectedProjects if project_ids.is_empty() => {
            bail!("Selected-project eligibility requires at least one recipient project.")
        }
        _ => {}
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let origin = query_origin(&tx, &request.origin_id)?;
    let artifact = artifact_info(&tx, &origin.artifact)?;
    assert_transferable(&origin.artifact, &artifact)?;
    let eligibility_id = id();
    let now = now_us()?;
    let purpose_json = encode_json(
        &request
            .permitted_purposes
            .iter()
            .map(TransferPurpose::as_str)
            .collect::<Vec<_>>(),
    )?;
    tx.execute(
        "INSERT INTO transfer_eligibilities
            (id, origin_id, license_identifier, license_uri, access_scope,
             permitted_purposes_json, attribution_text, permission_basis,
             permission_granted, reviewed_by, reviewed_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10)",
        params![
            eligibility_id,
            request.origin_id,
            request.license_identifier.trim(),
            request.license_uri,
            request.access_scope.as_str(),
            purpose_json,
            request.attribution_text.trim(),
            request.permission_basis.trim(),
            request.reviewed_by.trim(),
            now
        ],
    )?;
    for project_id in project_ids {
        tx.execute(
            "INSERT INTO transfer_eligibility_projects (eligibility_id, project_id)
             VALUES (?1, ?2)",
            params![eligibility_id, project_id],
        )?;
    }
    tx.execute(
        "INSERT INTO transfer_restrictions
            (id, eligibility_id, state, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, 'eligible', 'Rights and eligibility reviewed', ?3, ?4)",
        params![id(), eligibility_id, request.reviewed_by, now],
    )?;
    tx.commit()?;
    query_eligibility(conn, &eligibility_id)
}

pub fn change_eligibility(
    conn: &mut Connection,
    request: &ChangeTransferEligibilityReq,
) -> Result<TransferEligibilityDto> {
    if request.state == TransferEligibilityState::Eligible {
        bail!("Reinstatement requires a new reviewed eligibility snapshot.");
    }
    required(&request.rationale, "Restriction rationale")?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current_state: String = tx
        .query_row(
            "SELECT state FROM transfer_restrictions
             WHERE eligibility_id = ?1
             ORDER BY occurred_at_us DESC, id DESC LIMIT 1",
            [&request.eligibility_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("Transfer eligibility was not found."))?;
    if current_state != "eligible" {
        bail!("Only currently eligible transfers can be restricted or withdrawn.");
    }
    tx.execute(
        "INSERT INTO transfer_restrictions
            (id, eligibility_id, state, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id(),
            request.eligibility_id,
            request.state.as_str(),
            request.rationale.trim(),
            request.actor_id,
            now_us()?
        ],
    )?;
    tx.commit()?;
    query_eligibility(conn, &request.eligibility_id)
}

pub fn adopt_knowledge(
    conn: &mut Connection,
    request: &AdoptKnowledgeReq,
) -> Result<KnowledgeTransferDto> {
    required(&request.receiving_project_id, "Receiving project")?;
    required(&request.attribution_text, "Attribution text")?;
    required(&request.context, "Transfer context")?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let eligibility = query_eligibility(&tx, &request.eligibility_id)?;
    if eligibility.state != TransferEligibilityState::Eligible {
        bail!("This transfer eligibility is no longer active.");
    }
    if eligibility
        .origin
        .origin_project_id
        .eq(&request.receiving_project_id)
    {
        bail!("An originating project cannot adopt its own artifact.");
    }
    if !eligibility
        .permitted_purposes
        .contains(&request.intended_purpose)
    {
        bail!("The intended purpose is not permitted by this rights record.");
    }
    if eligibility.access_scope == TransferAccessScope::SelectedProjects
        && !eligibility
            .allowed_project_ids
            .contains(&request.receiving_project_id)
    {
        bail!("The receiving project is outside the eligibility allowlist.");
    }
    let info = artifact_info(&tx, &eligibility.origin.artifact)?;
    assert_transferable(&eligibility.origin.artifact, &info)?;

    let transfer_id = id();
    let now = now_us()?;
    tx.execute(
        "INSERT INTO knowledge_transfers
            (id, origin_id, eligibility_id, origin_project_id, receiving_project_id,
             intended_purpose, requested_by, requested_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            transfer_id,
            eligibility.origin.id,
            request.eligibility_id,
            eligibility.origin.origin_project_id,
            request.receiving_project_id,
            request.intended_purpose.as_str(),
            request.requested_by,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO transfer_attributions
            (transfer_id, attribution_text, recorded_by, recorded_at_us)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            transfer_id,
            request.attribution_text.trim(),
            request.requested_by,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO transfer_contexts (transfer_id, context, recorded_by, recorded_at_us)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            transfer_id,
            request.context.trim(),
            request.requested_by,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO knowledge_transfer_events
            (id, transfer_id, state, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, 'accepted', 'Adopted by receiving project', ?3, ?4)",
        params![id(), transfer_id, request.requested_by, now],
    )?;
    tx.execute(
        "INSERT INTO research_lineage
            (id, transfer_id, adaptation_id, origin_id, origin_project_id,
             receiving_project_id, relation_kind, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, 'adopted', ?6, ?7, ?8)",
        params![
            id(),
            transfer_id,
            eligibility.origin.id,
            eligibility.origin.origin_project_id,
            request.receiving_project_id,
            request.context.trim(),
            request.requested_by,
            now
        ],
    )?;
    tx.commit()?;
    query_transfer(conn, &transfer_id)
}

pub fn create_adaptation(
    conn: &mut Connection,
    request: &CreateTransferAdaptationReq,
) -> Result<TransferAdaptationDto> {
    required(&request.rationale, "Adaptation rationale")?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let transfer = query_transfer(&tx, &request.transfer_id)?;
    if transfer.state != KnowledgeTransferState::Accepted {
        bail!("Adaptations require an active accepted transfer.");
    }
    if request.artifact.kind() != transfer.origin.artifact.kind() {
        bail!("An adaptation must preserve the artifact domain type.");
    }
    if request.artifact.version_id() == transfer.origin.artifact.version_id() {
        bail!("An adaptation must reference a distinct exact artifact version.");
    }
    let info = artifact_info(&tx, &request.artifact)?;
    if request.artifact.kind() != TransferArtifactKind::FrameworkVersion
        && info.owner_project_id.as_deref() != Some(transfer.receiving_project_id.as_str())
    {
        bail!("Adapted artifacts must be created in the receiving project.");
    }
    let adaptation_id = id();
    let now = now_us()?;
    match &request.artifact {
        TransferArtifactRef::FindingVersion { version_id, .. } => tx.execute(
            "INSERT INTO transfer_adaptations
                (id, transfer_id, origin_id, receiving_project_id, artifact_kind,
                 finding_version_id, rationale, adapted_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, 'finding_version', ?5, ?6, ?7, ?8)",
            params![
                adaptation_id,
                request.transfer_id,
                transfer.origin.id,
                transfer.receiving_project_id,
                version_id,
                request.rationale.trim(),
                request.adapted_by,
                now
            ],
        )?,
        TransferArtifactRef::ConceptVersion { version_id, .. } => tx.execute(
            "INSERT INTO transfer_adaptations
                (id, transfer_id, origin_id, receiving_project_id, artifact_kind,
                 concept_version_id, rationale, adapted_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, 'concept_version', ?5, ?6, ?7, ?8)",
            params![
                adaptation_id,
                request.transfer_id,
                transfer.origin.id,
                transfer.receiving_project_id,
                version_id,
                request.rationale.trim(),
                request.adapted_by,
                now
            ],
        )?,
        TransferArtifactRef::PrincipleVersion { version_id, .. } => tx.execute(
            "INSERT INTO transfer_adaptations
                (id, transfer_id, origin_id, receiving_project_id, artifact_kind,
                 principle_version_id, rationale, adapted_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, 'principle_version', ?5, ?6, ?7, ?8)",
            params![
                adaptation_id,
                request.transfer_id,
                transfer.origin.id,
                transfer.receiving_project_id,
                version_id,
                request.rationale.trim(),
                request.adapted_by,
                now
            ],
        )?,
        TransferArtifactRef::FrameworkVersion { version_id, .. } => tx.execute(
            "INSERT INTO transfer_adaptations
                (id, transfer_id, origin_id, receiving_project_id, artifact_kind,
                 framework_version_id, rationale, adapted_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, 'framework_version', ?5, ?6, ?7, ?8)",
            params![
                adaptation_id,
                request.transfer_id,
                transfer.origin.id,
                transfer.receiving_project_id,
                version_id,
                request.rationale.trim(),
                request.adapted_by,
                now
            ],
        )?,
        TransferArtifactRef::ModelVersion { version_id, .. } => tx.execute(
            "INSERT INTO transfer_adaptations
                (id, transfer_id, origin_id, receiving_project_id, artifact_kind,
                 model_version_id, rationale, adapted_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, 'model_version', ?5, ?6, ?7, ?8)",
            params![
                adaptation_id,
                request.transfer_id,
                transfer.origin.id,
                transfer.receiving_project_id,
                version_id,
                request.rationale.trim(),
                request.adapted_by,
                now
            ],
        )?,
    };
    tx.execute(
        "INSERT INTO research_lineage
            (id, transfer_id, adaptation_id, origin_id, origin_project_id,
             receiving_project_id, relation_kind, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            id(),
            request.transfer_id,
            adaptation_id,
            transfer.origin.id,
            transfer.origin.origin_project_id,
            transfer.receiving_project_id,
            request.relation_kind.as_str(),
            request.rationale.trim(),
            request.adapted_by,
            now
        ],
    )?;
    tx.commit()?;
    query_adaptation(conn, &adaptation_id)
}

pub fn record_downstream_usage(
    conn: &mut Connection,
    request: &RecordDownstreamUsageReq,
) -> Result<DownstreamUsageDto> {
    required(&request.use_context, "Downstream-use context")?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let transfer = query_transfer(&tx, &request.transfer_id)?;
    if transfer.state != KnowledgeTransferState::Accepted {
        bail!("Downstream use requires an active accepted transfer.");
    }
    let usage_id = id();
    let now = now_us()?;
    insert_downstream_usage(&tx, request, &usage_id, &transfer.receiving_project_id, now)?;
    tx.commit()?;
    query_usage(conn, &usage_id)
}

fn insert_downstream_usage(
    tx: &Transaction<'_>,
    request: &RecordDownstreamUsageReq,
    usage_id: &str,
    project_id: &str,
    now: i64,
) -> Result<()> {
    let (kind, finding, concept, principle, framework, model, evaluation, analysis, experiment) =
        match &request.reference {
            DownstreamUseRef::FindingVersion { version_id, .. } => (
                "finding_version",
                Some(version_id.as_str()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            DownstreamUseRef::ConceptVersion { version_id, .. } => (
                "concept_version",
                None,
                Some(version_id.as_str()),
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            DownstreamUseRef::PrincipleVersion { version_id, .. } => (
                "principle_version",
                None,
                None,
                Some(version_id.as_str()),
                None,
                None,
                None,
                None,
                None,
            ),
            DownstreamUseRef::FrameworkVersion { version_id, .. } => (
                "framework_version",
                None,
                None,
                None,
                Some(version_id.as_str()),
                None,
                None,
                None,
                None,
            ),
            DownstreamUseRef::ModelVersion { version_id, .. } => (
                "model_version",
                None,
                None,
                None,
                None,
                Some(version_id.as_str()),
                None,
                None,
                None,
            ),
            DownstreamUseRef::ModelEvaluation { id } => (
                "model_evaluation",
                None,
                None,
                None,
                None,
                None,
                Some(id.as_str()),
                None,
                None,
            ),
            DownstreamUseRef::AnalysisRun { id } => (
                "analysis_run",
                None,
                None,
                None,
                None,
                None,
                None,
                Some(id.as_str()),
                None,
            ),
            DownstreamUseRef::ExperimentRun { id } => (
                "experiment_run",
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(id.as_str()),
            ),
        };
    tx.execute(
        "INSERT INTO downstream_usage
            (id, transfer_id, project_id, usage_kind, finding_version_id,
             concept_version_id, principle_version_id, framework_version_id,
             model_version_id, model_evaluation_id, analysis_run_id, experiment_run_id,
             use_context, recorded_by, recorded_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            usage_id,
            request.transfer_id,
            project_id,
            kind,
            finding,
            concept,
            principle,
            framework,
            model,
            evaluation,
            analysis,
            experiment,
            request.use_context.trim(),
            request.recorded_by,
            now
        ],
    )?;
    Ok(())
}

fn enum_artifact(kind: &str, entity_id: String, version_id: String) -> Result<TransferArtifactRef> {
    match kind {
        "finding_version" => Ok(TransferArtifactRef::FindingVersion {
            finding_id: entity_id,
            version_id,
        }),
        "concept_version" => Ok(TransferArtifactRef::ConceptVersion {
            concept_id: entity_id,
            version_id,
        }),
        "principle_version" => Ok(TransferArtifactRef::PrincipleVersion {
            principle_id: entity_id,
            version_id,
        }),
        "framework_version" => Ok(TransferArtifactRef::FrameworkVersion {
            framework_id: entity_id,
            version_id,
        }),
        "model_version" => Ok(TransferArtifactRef::ModelVersion {
            model_id: entity_id,
            version_id,
        }),
        _ => bail!("Unsupported knowledge transfer artifact kind: {kind}"),
    }
}

fn query_origin(conn: &Connection, origin_id: &str) -> Result<KnowledgeTransferOriginDto> {
    let (kind, version_id, project_id, rationale, recorded_by, recorded_at): (
        String,
        String,
        String,
        String,
        Option<String>,
        i64,
    ) = conn.query_row(
        "SELECT o.artifact_kind,
                COALESCE(o.finding_version_id, o.concept_version_id, o.principle_version_id,
                         o.framework_version_id, o.model_version_id),
                o.origin_project_id, o.rationale, o.recorded_by, o.recorded_at_us
         FROM knowledge_transfer_origins o
         WHERE o.id = ?1",
        [origin_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        },
    )?;
    let entity_id = project_or_entity(&kind, project_id.clone(), &version_id, conn)?;
    let artifact = enum_artifact(&kind, entity_id, version_id)?;
    let project_title = conn.query_row(
        "SELECT title FROM research_projects WHERE id = ?1",
        [&project_id],
        |row| row.get(0),
    )?;
    Ok(KnowledgeTransferOriginDto {
        id: origin_id.to_string(),
        artifact,
        origin_project_id: project_id,
        origin_project_title: project_title,
        rationale,
        recorded_by,
        recorded_at_us: recorded_at,
    })
}

fn project_or_entity(
    kind: &str,
    project_id: String,
    version_id: &str,
    conn: &Connection,
) -> Result<String> {
    let _ = project_id;
    Ok(match kind {
        "finding_version" => conn.query_row(
            "SELECT finding_id FROM analysis_finding_versions WHERE id = ?1",
            [version_id],
            |row| row.get(0),
        )?,
        "concept_version" => conn.query_row(
            "SELECT concept_id FROM knowledge_concept_versions WHERE id = ?1",
            [version_id],
            |row| row.get(0),
        )?,
        "principle_version" => conn.query_row(
            "SELECT principle_id FROM knowledge_principle_versions WHERE id = ?1",
            [version_id],
            |row| row.get(0),
        )?,
        "framework_version" => conn.query_row(
            "SELECT framework_id FROM knowledge_framework_versions WHERE id = ?1",
            [version_id],
            |row| row.get(0),
        )?,
        "model_version" => conn.query_row(
            "SELECT model_id FROM model_versions WHERE id = ?1",
            [version_id],
            |row| row.get(0),
        )?,
        _ => bail!("Unsupported knowledge transfer artifact kind: {kind}"),
    })
}

fn query_eligibility(conn: &Connection, eligibility_id: &str) -> Result<TransferEligibilityDto> {
    let origin_id: String = conn
        .query_row(
            "SELECT origin_id FROM transfer_eligibilities WHERE id = ?1",
            [eligibility_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("Transfer eligibility was not found."))?;
    let origin = query_origin(conn, &origin_id)?;
    let (license, uri, scope, purposes_json, attribution, basis, reviewer, reviewed_at): (
        String,
        Option<String>,
        String,
        String,
        String,
        String,
        String,
        i64,
    ) = conn.query_row(
        "SELECT license_identifier, license_uri, access_scope, permitted_purposes_json,
                attribution_text, permission_basis, reviewed_by, reviewed_at_us
         FROM transfer_eligibilities WHERE id = ?1",
        [eligibility_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        },
    )?;
    let purpose_names: Vec<String> = decode_json(&purposes_json)?;
    let permitted_purposes = purpose_names
        .into_iter()
        .map(|purpose| match purpose.as_str() {
            "research" => Ok(TransferPurpose::Research),
            "learning" => Ok(TransferPurpose::Learning),
            "commercial" => Ok(TransferPurpose::Commercial),
            "public_distribution" => Ok(TransferPurpose::PublicDistribution),
            _ => bail!("Stored transfer purpose is invalid: {purpose}"),
        })
        .collect::<Result<Vec<_>>>()?;
    let mut stmt = conn.prepare(
        "SELECT project_id FROM transfer_eligibility_projects
         WHERE eligibility_id = ?1 ORDER BY project_id",
    )?;
    let allowed_project_ids = stmt
        .query_map([eligibility_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    let (state, state_rationale): (String, String) = conn.query_row(
        "SELECT state, rationale FROM transfer_restrictions
         WHERE eligibility_id = ?1 ORDER BY occurred_at_us DESC, id DESC LIMIT 1",
        [eligibility_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let state = match state.as_str() {
        "eligible" => TransferEligibilityState::Eligible,
        "restricted" => TransferEligibilityState::Restricted,
        "withdrawn" => TransferEligibilityState::Withdrawn,
        _ => bail!("Stored transfer eligibility state is invalid."),
    };
    let access_scope = match scope.as_str() {
        "workspace" => TransferAccessScope::Workspace,
        "selected_projects" => TransferAccessScope::SelectedProjects,
        _ => bail!("Stored transfer access scope is invalid."),
    };
    Ok(TransferEligibilityDto {
        id: eligibility_id.to_string(),
        origin,
        license_identifier: license,
        license_uri: uri,
        access_scope,
        permitted_purposes,
        allowed_project_ids,
        attribution_text: attribution,
        permission_basis: basis,
        reviewed_by: reviewer,
        reviewed_at_us: reviewed_at,
        state,
        state_rationale,
    })
}

fn query_transfer(conn: &Connection, transfer_id: &str) -> Result<KnowledgeTransferDto> {
    let (origin_id, project_id, purpose, requester, requested_at): (
        String,
        String,
        String,
        Option<String>,
        i64,
    ) = conn
        .query_row(
            "SELECT origin_id, receiving_project_id, intended_purpose, requested_by, requested_at_us
             FROM knowledge_transfers WHERE id = ?1",
            [transfer_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("Knowledge transfer was not found."))?;
    let origin = query_origin(conn, &origin_id)?;
    let (attribution, context): (String, String) = conn.query_row(
        "SELECT a.attribution_text, c.context
         FROM transfer_attributions a JOIN transfer_contexts c ON c.transfer_id = a.transfer_id
         WHERE a.transfer_id = ?1",
        [transfer_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let project_title: String = conn.query_row(
        "SELECT title FROM research_projects WHERE id = ?1",
        [&project_id],
        |row| row.get(0),
    )?;
    let mut event_stmt = conn.prepare(
        "SELECT id, state, rationale, actor_id, occurred_at_us
         FROM knowledge_transfer_events WHERE transfer_id = ?1
         ORDER BY occurred_at_us, id",
    )?;
    let events = event_stmt
        .query_map([transfer_id], |row| {
            let state: String = row.get(1)?;
            Ok((
                row.get::<_, String>(0)?,
                state,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?
        .map(|row| {
            let (id, state, rationale, actor_id, occurred_at_us) = row?;
            Ok(KnowledgeTransferEventDto {
                id,
                state: parse_transfer_state(&state)?,
                rationale,
                actor_id,
                occurred_at_us,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let current_state = events
        .last()
        .map(|event| event.state)
        .ok_or_else(|| anyhow!("Knowledge transfer has no lifecycle event."))?;
    let intended_purpose = match purpose.as_str() {
        "research" => TransferPurpose::Research,
        "learning" => TransferPurpose::Learning,
        "commercial" => TransferPurpose::Commercial,
        "public_distribution" => TransferPurpose::PublicDistribution,
        _ => bail!("Stored transfer purpose is invalid."),
    };
    Ok(KnowledgeTransferDto {
        id: transfer_id.to_string(),
        eligibility_id: conn.query_row(
            "SELECT eligibility_id FROM knowledge_transfers WHERE id = ?1",
            [transfer_id],
            |row| row.get(0),
        )?,
        origin,
        receiving_project_id: project_id,
        receiving_project_title: project_title,
        intended_purpose,
        attribution_text: attribution,
        context,
        requested_by: requester,
        requested_at_us: requested_at,
        state: current_state,
        events,
    })
}

fn parse_transfer_state(value: &str) -> Result<KnowledgeTransferState> {
    match value {
        "accepted" => Ok(KnowledgeTransferState::Accepted),
        "rejected" => Ok(KnowledgeTransferState::Rejected),
        "withdrawn" => Ok(KnowledgeTransferState::Withdrawn),
        _ => bail!("Stored knowledge transfer state is invalid: {value}"),
    }
}

fn query_adaptation(conn: &Connection, adaptation_id: &str) -> Result<TransferAdaptationDto> {
    let (
        transfer_id,
        origin_id,
        receiving_project_id,
        kind,
        version_id,
        rationale,
        adapted_by,
        created_at,
    ): (
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        i64,
    ) = conn.query_row(
        "SELECT transfer_id, origin_id, receiving_project_id, artifact_kind,
                COALESCE(finding_version_id, concept_version_id, principle_version_id,
                         framework_version_id, model_version_id),
                rationale, adapted_by, created_at_us
         FROM transfer_adaptations WHERE id = ?1",
        [adaptation_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        },
    )?;
    let entity_id = project_or_entity(&kind, receiving_project_id.clone(), &version_id, conn)?;
    let artifact = enum_artifact(&kind, entity_id, version_id)?;
    let title = artifact_info(conn, &artifact)?.title;
    Ok(TransferAdaptationDto {
        id: adaptation_id.to_string(),
        transfer_id,
        origin: query_origin(conn, &origin_id)?,
        receiving_project_id,
        artifact,
        title,
        rationale,
        adapted_by,
        created_at_us: created_at,
    })
}

fn query_usage(conn: &Connection, usage_id: &str) -> Result<DownstreamUsageDto> {
    let (transfer_id, project_id, kind, artifact_id, context, recorded_by, recorded_at): (
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        i64,
    ) = conn.query_row(
        "SELECT transfer_id, project_id, usage_kind,
                COALESCE(finding_version_id, concept_version_id, principle_version_id,
                         framework_version_id, model_version_id, model_evaluation_id,
                         analysis_run_id, experiment_run_id),
                use_context, recorded_by, recorded_at_us
         FROM downstream_usage WHERE id = ?1",
        [usage_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        },
    )?;
    let reference = match kind.as_str() {
        "finding_version" | "concept_version" | "principle_version" | "framework_version"
        | "model_version" => {
            let entity_id = project_or_entity(&kind, project_id.clone(), &artifact_id, conn)?;
            match enum_artifact(&kind, entity_id, artifact_id)? {
                TransferArtifactRef::FindingVersion {
                    finding_id,
                    version_id,
                } => DownstreamUseRef::FindingVersion {
                    finding_id,
                    version_id,
                },
                TransferArtifactRef::ConceptVersion {
                    concept_id,
                    version_id,
                } => DownstreamUseRef::ConceptVersion {
                    concept_id,
                    version_id,
                },
                TransferArtifactRef::PrincipleVersion {
                    principle_id,
                    version_id,
                } => DownstreamUseRef::PrincipleVersion {
                    principle_id,
                    version_id,
                },
                TransferArtifactRef::FrameworkVersion {
                    framework_id,
                    version_id,
                } => DownstreamUseRef::FrameworkVersion {
                    framework_id,
                    version_id,
                },
                TransferArtifactRef::ModelVersion {
                    model_id,
                    version_id,
                } => DownstreamUseRef::ModelVersion {
                    model_id,
                    version_id,
                },
            }
        }
        "model_evaluation" => DownstreamUseRef::ModelEvaluation { id: artifact_id },
        "analysis_run" => DownstreamUseRef::AnalysisRun { id: artifact_id },
        "experiment_run" => DownstreamUseRef::ExperimentRun { id: artifact_id },
        _ => bail!("Stored downstream usage kind is invalid."),
    };
    Ok(DownstreamUsageDto {
        id: usage_id.to_string(),
        transfer_id,
        project_id,
        reference,
        use_context: context,
        recorded_by,
        recorded_at_us: recorded_at,
    })
}

fn query_lineage(conn: &Connection, lineage_id: &str) -> Result<ResearchLineageDto> {
    let (
        transfer_id,
        origin_id,
        origin_project_id,
        receiving_project_id,
        relation_kind,
        adaptation_id,
        rationale,
        occurred_at,
    ): (
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        String,
        i64,
    ) = conn.query_row(
        "SELECT transfer_id, origin_id, origin_project_id, receiving_project_id,
                relation_kind, adaptation_id, rationale, occurred_at_us
         FROM research_lineage WHERE id = ?1",
        [lineage_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        },
    )?;
    let origin = query_origin(conn, &origin_id)?.artifact;
    let adaptation = adaptation_id
        .map(|id| query_adaptation(conn, &id).map(|adaptation| adaptation.artifact))
        .transpose()?;
    Ok(ResearchLineageDto {
        id: lineage_id.to_string(),
        transfer_id,
        origin,
        origin_project_id,
        receiving_project_id,
        relation_kind,
        adaptation,
        rationale,
        occurred_at_us: occurred_at,
    })
}

pub fn workspace(conn: &Connection) -> Result<TransferWorkspaceDto> {
    let mut project_stmt = conn
        .prepare("SELECT id, title, lifecycle_status FROM research_projects ORDER BY title, id")?;
    let projects = project_stmt
        .query_map([], |row| {
            Ok(ResearchProjectDto {
                id: row.get(0)?,
                title: row.get(1)?,
                lifecycle_status: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut artifact_refs = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT v.finding_id, v.id FROM analysis_finding_versions v
             JOIN analysis_finding_events e ON e.finding_version_id = v.id
             WHERE e.id = (SELECT e2.id FROM analysis_finding_events e2
                           WHERE e2.finding_version_id = v.id
                           ORDER BY e2.occurred_at_us DESC, e2.id DESC LIMIT 1)
               AND e.status IN ('accepted', 'contested', 'unsupported', 'inconclusive', 'superseded')
             ORDER BY v.created_at_us DESC",
        )?;
        artifact_refs.extend(
            stmt.query_map([], |row| {
                Ok(TransferArtifactRef::FindingVersion {
                    finding_id: row.get(0)?,
                    version_id: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?,
        );
    }
    {
        let mut stmt = conn.prepare(
            "SELECT v.concept_id, v.id FROM knowledge_concept_versions v
             JOIN knowledge_concept_version_events e ON e.version_id = v.id
             WHERE e.id = (SELECT e2.id FROM knowledge_concept_version_events e2
                           WHERE e2.version_id = v.id
                           ORDER BY e2.occurred_at_us DESC, e2.id DESC LIMIT 1)
               AND e.status = 'reviewed'",
        )?;
        artifact_refs.extend(
            stmt.query_map([], |row| {
                Ok(TransferArtifactRef::ConceptVersion {
                    concept_id: row.get(0)?,
                    version_id: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?,
        );
    }
    {
        let mut stmt = conn.prepare(
            "SELECT v.principle_id, v.id FROM knowledge_principle_versions v
             JOIN knowledge_principle_version_events e ON e.version_id = v.id
             WHERE e.id = (SELECT e2.id FROM knowledge_principle_version_events e2
                           WHERE e2.version_id = v.id
                           ORDER BY e2.occurred_at_us DESC, e2.id DESC LIMIT 1)
               AND e.status = 'reviewed'",
        )?;
        artifact_refs.extend(
            stmt.query_map([], |row| {
                Ok(TransferArtifactRef::PrincipleVersion {
                    principle_id: row.get(0)?,
                    version_id: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?,
        );
    }
    {
        let mut stmt = conn.prepare(
            "SELECT v.framework_id, v.id FROM knowledge_framework_versions v
             WHERE v.status = 'published'",
        )?;
        artifact_refs.extend(
            stmt.query_map([], |row| {
                Ok(TransferArtifactRef::FrameworkVersion {
                    framework_id: row.get(0)?,
                    version_id: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?,
        );
    }
    {
        let mut stmt = conn.prepare(
            "SELECT v.model_id, v.id FROM model_versions v
             JOIN model_version_events e ON e.model_version_id = v.id
             WHERE e.id = (SELECT e2.id FROM model_version_events e2
                           WHERE e2.model_version_id = v.id
                           ORDER BY e2.occurred_at_us DESC, e2.id DESC LIMIT 1)
               AND e.lifecycle_status = 'released'",
        )?;
        artifact_refs.extend(
            stmt.query_map([], |row| {
                Ok(TransferArtifactRef::ModelVersion {
                    model_id: row.get(0)?,
                    version_id: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?,
        );
    }
    let mut artifacts = Vec::new();
    for artifact in artifact_refs {
        let info = artifact_info(conn, &artifact)?;
        let origin_id = find_origin_id(conn, &artifact)?;
        let eligibility_ids = if let Some(origin_id) = &origin_id {
            let mut stmt = conn.prepare(
                "SELECT e.id FROM transfer_eligibilities e
                 JOIN transfer_restrictions r ON r.eligibility_id = e.id
                 WHERE e.origin_id = ?1
                   AND r.id = (SELECT r2.id FROM transfer_restrictions r2
                               WHERE r2.eligibility_id = e.id
                               ORDER BY r2.occurred_at_us DESC, r2.id DESC LIMIT 1)
                   AND r.state = 'eligible'
                 ORDER BY e.reviewed_at_us DESC",
            )?;
            let rows = stmt.query_map([origin_id], |row| row.get(0))?;
            rows.collect::<rusqlite::Result<Vec<String>>>()?
        } else {
            Vec::new()
        };
        artifacts.push(TransferArtifactCatalogDto {
            artifact,
            title: info.title,
            version_no: info.version_no,
            owner_project_id: info.owner_project_id,
            lifecycle_status: info.lifecycle_status,
            origin_id,
            eligibility_ids,
        });
    }
    let mut knowledge_versions = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT v.concept_id, v.id, v.created_in_project_id, v.title, v.definition,
                    v.rationale, v.parent_version_id,
                    COALESCE((SELECT e.status FROM knowledge_concept_version_events e
                              WHERE e.version_id = v.id
                              ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1), 'unreviewed')
             FROM knowledge_concept_versions v
             ORDER BY v.created_at_us DESC, v.id",
        )?;
        knowledge_versions.extend(
            stmt.query_map([], |row| {
                Ok(KnowledgeVersionDto {
                    kind: KnowledgeVersionKind::Concept,
                    entity_id: row.get(0)?,
                    version_id: row.get(1)?,
                    project_id: row.get(2)?,
                    title: row.get(3)?,
                    content: row.get(4)?,
                    rationale: row.get(5)?,
                    parent_version_id: row.get(6)?,
                    status: row.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?,
        );
    }
    {
        let mut stmt = conn.prepare(
            "SELECT v.principle_id, v.id, v.created_in_project_id, v.title, v.statement,
                    v.rationale, v.parent_version_id,
                    COALESCE((SELECT e.status FROM knowledge_principle_version_events e
                              WHERE e.version_id = v.id
                              ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1), 'unreviewed')
             FROM knowledge_principle_versions v
             ORDER BY v.created_at_us DESC, v.id",
        )?;
        knowledge_versions.extend(
            stmt.query_map([], |row| {
                Ok(KnowledgeVersionDto {
                    kind: KnowledgeVersionKind::Principle,
                    entity_id: row.get(0)?,
                    version_id: row.get(1)?,
                    project_id: row.get(2)?,
                    title: row.get(3)?,
                    content: row.get(4)?,
                    rationale: row.get(5)?,
                    parent_version_id: row.get(6)?,
                    status: row.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?,
        );
    }

    let origin_ids = query_strings(
        conn,
        "SELECT id FROM knowledge_transfer_origins ORDER BY recorded_at_us DESC, id",
    )?;
    let origins = origin_ids
        .iter()
        .map(|origin_id| query_origin(conn, origin_id))
        .collect::<Result<Vec<_>>>()?;
    let eligibility_ids = query_strings(
        conn,
        "SELECT id FROM transfer_eligibilities ORDER BY reviewed_at_us DESC, id",
    )?;
    let eligibilities = eligibility_ids
        .iter()
        .map(|eligibility_id| query_eligibility(conn, eligibility_id))
        .collect::<Result<Vec<_>>>()?;
    let transfer_ids = query_strings(
        conn,
        "SELECT id FROM knowledge_transfers ORDER BY requested_at_us DESC, id",
    )?;
    let transfers = transfer_ids
        .iter()
        .map(|transfer_id| query_transfer(conn, transfer_id))
        .collect::<Result<Vec<_>>>()?;
    let adaptation_ids = query_strings(
        conn,
        "SELECT id FROM transfer_adaptations ORDER BY created_at_us DESC, id",
    )?;
    let adaptations = adaptation_ids
        .iter()
        .map(|adaptation_id| query_adaptation(conn, adaptation_id))
        .collect::<Result<Vec<_>>>()?;
    let usage_ids = query_strings(
        conn,
        "SELECT id FROM downstream_usage ORDER BY recorded_at_us DESC, id",
    )?;
    let downstream_usage = usage_ids
        .iter()
        .map(|usage_id| query_usage(conn, usage_id))
        .collect::<Result<Vec<_>>>()?;
    let lineage_ids = query_strings(
        conn,
        "SELECT id FROM research_lineage ORDER BY occurred_at_us DESC, id",
    )?;
    let lineage = lineage_ids
        .iter()
        .map(|lineage_id| query_lineage(conn, lineage_id))
        .collect::<Result<Vec<_>>>()?;
    Ok(TransferWorkspaceDto {
        projects,
        artifacts,
        knowledge_versions,
        origins,
        eligibilities,
        transfers,
        adaptations,
        downstream_usage,
        lineage,
    })
}

pub fn provenance_preview(
    conn: &Connection,
    origin_id: &str,
) -> Result<TransferProvenancePreviewDto> {
    let origin = query_origin(conn, origin_id)?;
    let info = artifact_info(conn, &origin.artifact)?;
    let mut validation_scope = Vec::new();
    let mut provenance = Vec::new();

    let (scope, applicability, uncertainty, limitations) = match &origin.artifact {
        TransferArtifactRef::FindingVersion { version_id, .. } => {
            let values = conn.query_row(
                "SELECT scope, applicability, uncertainty, limitations
                 FROM analysis_finding_versions WHERE id = ?1",
                [version_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
            let mut evidence_stmt = conn.prepare(
                "SELECT e.id, e.polarity, e.scope, e.limitations,
                        COALESCE(e.artifact_analysis_output_id, e.artifact_experiment_run_id,
                                 e.artifact_dataset_version_id, e.artifact_observation_id,
                                 e.artifact_source_segment_id, e.artifact_source_claim_id),
                        COALESCE(
                            (SELECT src.title || ' — ' || seg.label || ' (' || seg.location_kind
                                    || CASE WHEN seg.page_start IS NOT NULL
                                         THEN ', pages ' || seg.page_start || '-' || COALESCE(seg.page_end, seg.page_start)
                                         ELSE '' END
                                    || CASE WHEN seg.line_start IS NOT NULL
                                         THEN ', lines ' || seg.line_start || '-' || COALESCE(seg.line_end, seg.line_start)
                                         ELSE '' END
                                    || ') · version ' || COALESCE(sv.edition_label, sv.capture_kind)
                             FROM source_version_segments seg
                             JOIN sources src ON src.id = seg.source_id
                             JOIN source_versions sv ON sv.id = seg.source_version_id
                             WHERE seg.id = e.artifact_source_segment_id),
                            (SELECT src.title || ' — ' || claim.statement || ' · version '
                                    || COALESCE(sv.edition_label, sv.capture_kind)
                             FROM source_claims claim
                             JOIN sources src ON src.id = claim.source_id
                             JOIN source_versions sv ON sv.id = claim.source_version_id
                             WHERE claim.id = e.artifact_source_claim_id),
                            'Artifact context not available for this reference'
                        )
                 FROM analysis_finding_version_evidence l
                 JOIN analysis_evidence e ON e.id = l.evidence_id
                 WHERE l.finding_version_id = ?1 ORDER BY e.created_at_us, e.id",
            )?;
            provenance.extend(
                evidence_stmt
                    .query_map([version_id], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                        ))
                    })?
                    .map(|row| {
                        let (
                            id,
                            polarity,
                            evidence_scope,
                            evidence_limitations,
                            artifact_id,
                            artifact_context,
                        ) = row?;
                        Ok(TransferProvenanceRecordDto {
                            record_kind: "evidence".to_string(),
                            record_id: id,
                            label: format!("Evidence: {polarity}"),
                            relationship: "finding_evidence".to_string(),
                            details: format!(
                                "Scope: {evidence_scope}; limitations: {evidence_limitations}; artifact: {artifact_id}; context: {artifact_context}"
                            ),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            );
            let mut run_stmt = conn.prepare(
                "SELECT r.id, COALESCE(e.lifecycle_status, 'not_started')
                 FROM analysis_finding_version_runs l
                 JOIN analysis_runs r ON r.id = l.run_id
                 LEFT JOIN analysis_run_events e ON e.id = (
                     SELECT e2.id FROM analysis_run_events e2
                     WHERE e2.run_id = r.id
                     ORDER BY e2.occurred_at_us DESC, e2.id DESC LIMIT 1
                 )
                 WHERE l.finding_version_id = ?1 ORDER BY r.created_at_us, r.id",
            )?;
            provenance.extend(
                run_stmt
                    .query_map([version_id], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })?
                    .map(|row| {
                        let (run_id, status) = row?;
                        Ok(TransferProvenanceRecordDto {
                            record_kind: "analysis_run".to_string(),
                            record_id: run_id,
                            label: format!("Analysis run: {status}"),
                            relationship: "finding_run".to_string(),
                            details: "Exact run associated with this finding version.".to_string(),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            );
            let mut validation_stmt = conn.prepare(
                "SELECT v.id, v.scope, v.population_scope, v.evaluation_method,
                        COALESCE(e.outcome, 'not_evaluated')
                 FROM model_validations v
                 LEFT JOIN model_validation_events e ON e.validation_id = v.id
                 WHERE v.target_finding_version_id = ?1 ORDER BY v.created_at_us, v.id",
            )?;
            let validation_rows = validation_stmt
                .query_map([version_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for (validation_id, validation_scope_text, population, method, outcome) in
                validation_rows
            {
                validation_scope.push(format!("{validation_scope_text} / {population} ({outcome})"));
                provenance.push(TransferProvenanceRecordDto {
                    record_kind: "validation".to_string(),
                    record_id: validation_id,
                    label: format!("Validation: {outcome}"),
                    relationship: "finding_validation".to_string(),
                    details: format!("Scope: {validation_scope_text}; population: {population}; method: {method}"),
                });
            }
            values
        }
        TransferArtifactRef::ModelVersion { version_id, .. } => {
            let values = conn.query_row(
                "SELECT v.scope, v.human_specification,
                        COALESCE((SELECT group_concat(a.statement, '; ')
                                  FROM model_version_assumptions a
                                  WHERE a.model_version_id = v.id), ''),
                        v.change_rationale
                 FROM model_versions v WHERE v.id = ?1",
                [version_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
            let mut evidence_stmt = conn.prepare(
                "SELECT e.id, e.polarity, e.scope, e.limitations,
                        COALESCE(e.artifact_analysis_output_id, e.artifact_experiment_run_id,
                                 e.artifact_dataset_version_id, e.artifact_observation_id,
                                 e.artifact_source_segment_id, e.artifact_source_claim_id,
                                 e.artifact_finding_version_id),
                        COALESCE(
                            (SELECT src.title || ' — ' || seg.label || ' (' || seg.location_kind
                                    || CASE WHEN seg.page_start IS NOT NULL
                                         THEN ', pages ' || seg.page_start || '-' || COALESCE(seg.page_end, seg.page_start)
                                         ELSE '' END
                                    || CASE WHEN seg.line_start IS NOT NULL
                                         THEN ', lines ' || seg.line_start || '-' || COALESCE(seg.line_end, seg.line_start)
                                         ELSE '' END
                                    || ') · version ' || COALESCE(sv.edition_label, sv.capture_kind)
                             FROM source_version_segments seg
                             JOIN sources src ON src.id = seg.source_id
                             JOIN source_versions sv ON sv.id = seg.source_version_id
                             WHERE seg.id = e.artifact_source_segment_id),
                            (SELECT src.title || ' — ' || claim.statement || ' · version '
                                    || COALESCE(sv.edition_label, sv.capture_kind)
                             FROM source_claims claim
                             JOIN sources src ON src.id = claim.source_id
                             JOIN source_versions sv ON sv.id = claim.source_version_id
                             WHERE claim.id = e.artifact_source_claim_id),
                            'Artifact context not available for this reference'
                        )
                 FROM model_evidence e WHERE e.model_version_id = ?1
                 ORDER BY e.created_at_us, e.id",
            )?;
            provenance.extend(
                evidence_stmt
                    .query_map([version_id], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                        ))
                    })?
                    .map(|row| {
                        let (
                            id,
                            polarity,
                            evidence_scope,
                            evidence_limitations,
                            artifact_id,
                            artifact_context,
                        ) = row?;
                        Ok(TransferProvenanceRecordDto {
                            record_kind: "model_evidence".to_string(),
                            record_id: id,
                            label: format!("Model evidence: {polarity}"),
                            relationship: "model_version_evidence".to_string(),
                            details: format!(
                                "Scope: {evidence_scope}; limitations: {evidence_limitations}; artifact: {artifact_id}; context: {artifact_context}"
                            ),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            );
            let mut evaluation_stmt = conn.prepare(
                "SELECT e.id, e.role, e.dataset_version_id, m.effect_estimate, m.unit
                 FROM model_evaluations e
                 LEFT JOIN analysis_metrics m ON m.id = e.analysis_metric_id
                 WHERE e.model_version_id = ?1 ORDER BY e.evaluated_at_us, e.id",
            )?;
            let rows = evaluation_stmt
                .query_map([version_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for (evaluation_id, role, dataset_version_id, metric, units) in rows {
                provenance.push(TransferProvenanceRecordDto {
                    record_kind: "model_evaluation".to_string(),
                    record_id: evaluation_id,
                    label: format!("Model evaluation ({role})"),
                    relationship: "model_version_evaluation".to_string(),
                    details: format!(
                        "Dataset version: {dataset_version_id}; metric: {} {}",
                        metric.unwrap_or_else(|| "not recorded".to_string()),
                        units.unwrap_or_default()
                    ),
                });
            }
            let mut validation_stmt = conn.prepare(
                "SELECT v.id, v.scope, v.population_scope, v.evaluation_method,
                        COALESCE(e.outcome, 'not_evaluated')
                 FROM model_validations v
                 LEFT JOIN model_validation_events e ON e.validation_id = v.id
                 WHERE v.target_model_version_id = ?1 ORDER BY v.created_at_us, v.id",
            )?;
            let validation_rows = validation_stmt
                .query_map([version_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for (validation_id, validation_scope_text, population, method, outcome) in
                validation_rows
            {
                validation_scope.push(format!(
                    "{validation_scope_text} / {population} ({outcome})"
                ));
                provenance.push(TransferProvenanceRecordDto {
                    record_kind: "validation".to_string(),
                    record_id: validation_id,
                    label: format!("Validation: {outcome}"),
                    relationship: "model_validation".to_string(),
                    details: format!("Scope: {validation_scope_text}; population: {population}; method: {method}"),
                });
            }
            values
        }
        TransferArtifactRef::FrameworkVersion { version_id, .. } => {
            let values = conn.query_row(
                "SELECT v.description, v.status, '', v.change_rationale
                 FROM knowledge_framework_versions v WHERE v.id = ?1",
                [version_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
            let mut item_stmt = conn.prepare(
                "SELECT item_kind,
                        COALESCE(source_claim_id, concept_id, principle_id, procedure_id),
                        membership_role, rationale
                 FROM framework_version_items WHERE framework_version_id = ?1
                 ORDER BY item_order, id",
            )?;
            provenance.extend(
                item_stmt
                    .query_map([version_id], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        ))
                    })?
                    .map(|row| {
                        let (kind, item_id, role, rationale) = row?;
                        Ok(TransferProvenanceRecordDto {
                            record_kind: kind.clone(),
                            record_id: item_id,
                            label: format!("Framework member: {kind} ({role})"),
                            relationship: "framework_membership".to_string(),
                            details: rationale,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            );
            values
        }
        TransferArtifactRef::ConceptVersion {
            concept_id,
            version_id,
        } => {
            let values = conn.query_row(
                "SELECT v.definition, v.created_in_project_id, '', v.rationale
                 FROM knowledge_concept_versions v WHERE v.id = ?1 AND v.concept_id = ?2",
                params![version_id, concept_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
            append_acquisition_provenance(conn, "concept_id", concept_id, &mut provenance)?;
            values
        }
        TransferArtifactRef::PrincipleVersion {
            principle_id,
            version_id,
        } => {
            let values = conn.query_row(
                "SELECT v.statement, v.created_in_project_id, '', v.rationale
                 FROM knowledge_principle_versions v WHERE v.id = ?1 AND v.principle_id = ?2",
                params![version_id, principle_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
            append_acquisition_provenance(conn, "principle_id", principle_id, &mut provenance)?;
            values
        }
    };

    Ok(TransferProvenancePreviewDto {
        origin,
        title: info.title,
        version_no: info.version_no,
        lifecycle_status: info.lifecycle_status,
        scope,
        applicability,
        uncertainty,
        limitations,
        validation_scope,
        provenance,
    })
}

fn append_acquisition_provenance(
    conn: &Connection,
    target_column: &str,
    target_id: &str,
    provenance: &mut Vec<TransferProvenanceRecordDto>,
) -> Result<()> {
    let sql = format!(
        "SELECT id, event_kind, reason, occurred_at_us FROM acquisition_provenance_events
         WHERE {target_column} = ?1 ORDER BY occurred_at_us, id"
    );
    let mut stmt = conn.prepare(&sql)?;
    provenance.extend(
        stmt.query_map([target_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?
        .map(|row| {
            let (id, event_kind, reason, occurred_at_us) = row?;
            Ok(TransferProvenanceRecordDto {
                record_kind: "acquisition_provenance".to_string(),
                record_id: id,
                label: event_kind,
                relationship: "knowledge_origin".to_string(),
                details: format!("{reason} (at {occurred_at_us})"),
            })
        })
        .collect::<Result<Vec<_>>>()?,
    );
    Ok(())
}

fn query_strings(conn: &Connection, sql: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn find_origin_id(conn: &Connection, artifact: &TransferArtifactRef) -> Result<Option<String>> {
    let (column, _) = match artifact.kind() {
        TransferArtifactKind::FindingVersion => ("finding_version_id", ""),
        TransferArtifactKind::ConceptVersion => ("concept_version_id", ""),
        TransferArtifactKind::PrincipleVersion => ("principle_version_id", ""),
        TransferArtifactKind::FrameworkVersion => ("framework_version_id", ""),
        TransferArtifactKind::ModelVersion => ("model_version_id", ""),
    };
    let sql = format!("SELECT id FROM knowledge_transfer_origins WHERE {column} = ?1");
    Ok(conn
        .query_row(&sql, [artifact.version_id()], |row| row.get(0))
        .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::backup::Backup;
    use std::{fs, path::PathBuf, time::Duration};

    fn create_project(conn: &Connection, id: &str, title: &str) -> Result<()> {
        conn.execute(
            "INSERT INTO research_projects
                (id, title, aim, lifecycle_status, created_by, created_at_us, updated_at_us)
             VALUES (?1, ?2, '', 'active', NULL, 1, 1)",
            params![id, title],
        )?;
        Ok(())
    }

    fn create_accepted_finding(conn: &Connection, project_id: &str, key: &str) -> Result<String> {
        let hypothesis_id = format!("hypothesis-{key}");
        let hypothesis_version_id = format!("hypothesis-version-{key}");
        let finding_id = format!("finding-{key}");
        let finding_version_id = format!("finding-version-{key}");
        conn.execute(
            "INSERT INTO research_hypotheses
                (id, project_id, research_question_id, legacy_hypothesis_id,
                 parent_hypothesis_id, ownership_state, lifecycle_status, created_by, created_at_us)
             VALUES (?1, ?2, NULL, NULL, NULL, 'project', 'legacy_untyped', NULL, 1)",
            params![hypothesis_id, project_id],
        )?;
        conn.execute(
            "INSERT INTO hypothesis_definition_versions
                (id, hypothesis_id, version_no, parent_version_id, statement_h0, statement_h1,
                 population_scope, research_scope, expected_direction, expected_effect_decimal,
                 expected_effect_unit, falsification_criteria, assumptions, lifecycle_status,
                 created_by, created_at_us, preregistered_at_us)
             VALUES (?1, ?2, 1, NULL, 'No effect', 'There is an effect', 'population',
                     'study scope', 'non_directional', NULL, NULL, 'falsification rule',
                     '', 'draft', NULL, 1, NULL)",
            params![hypothesis_version_id, hypothesis_id],
        )?;
        conn.execute(
            "INSERT INTO analysis_findings (id, project_id, created_by, created_at_us)
             VALUES (?1, ?2, NULL, 1)",
            params![finding_id, project_id],
        )?;
        conn.execute(
            "INSERT INTO analysis_finding_versions
                (id, finding_id, version_no, parent_version_id, change_rationale, title, statement,
                 target_source_claim_id, target_hypothesis_version_id, scope, applicability,
                 uncertainty, limitations, conclusion_kind, content_sha256, created_by, created_at_us)
             VALUES (?1, ?2, 1, NULL, NULL, 'Finding title', 'Finding statement', NULL, ?3,
                     'declared scope', 'declared applicability', 'uncertainty recorded',
                     'limitations recorded', 'result', ?4, NULL, 1)",
            params![
                finding_version_id,
                finding_id,
                hypothesis_version_id,
                "a".repeat(64)
            ],
        )?;
        for (index, status) in ["draft", "review", "accepted"].iter().enumerate() {
            conn.execute(
                "INSERT INTO analysis_finding_events
                    (id, finding_version_id, status, rationale, actor_id, occurred_at_us)
                 VALUES (?1, ?2, ?3, 'Lifecycle event recorded', NULL, ?4)",
                params![
                    format!("finding-event-{key}-{index}"),
                    finding_version_id,
                    status,
                    index as i64 + 1
                ],
            )?;
        }
        Ok(finding_version_id)
    }

    fn add_source_segment_evidence(
        conn: &Connection,
        project_id: &str,
        finding_version_id: &str,
        hypothesis_version_id: &str,
    ) -> Result<()> {
        conn.execute(
            "INSERT INTO sources (id, kind, path, title, author, language, duration_sec,
                                  page_count, metadata_json, notes, status, created_at, updated_at)
             VALUES ('source-study', 'book', NULL, 'Study Source', 'Author', 'en', NULL,
                     30, '{}', '', 'active', 1, 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO source_versions
                (id, source_id, capture_kind, edition_label, canonical_uri, captured_at_us,
                 content_sha256, external_asset_ref, metadata_json, created_by, created_at_us)
             VALUES ('source-version-study', 'source-study', 'edition', '2nd edition', NULL, 1,
                     NULL, NULL, '{}', 'tester', 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO source_version_segments
                (id, source_id, source_version_id, label, location_kind, page_start, page_end,
                 notes, order_index, created_at_us)
             VALUES ('source-segment-study', 'source-study', 'source-version-study',
                     'Methods section', 'page', 4, 5, '', 0, 1)",
            [],
        )?;
        conn.execute(
            "INSERT INTO analysis_evidence
                (id, project_id, target_source_claim_id, target_hypothesis_version_id,
                 target_finding_version_id, artifact_analysis_output_id, artifact_experiment_run_id,
                 artifact_dataset_version_id, artifact_observation_id, artifact_source_segment_id,
                 artifact_source_claim_id, polarity, scope, quality_rationale, limitations,
                 provenance_json, created_by, created_at_us)
             VALUES ('evidence-study', ?1, NULL, ?2, NULL, NULL, NULL, NULL, NULL,
                     'source-segment-study', NULL, 'supports', 'study scope',
                     'reproducible method', 'sampling limitations', '{}', 'tester', 1)",
            params![project_id, hypothesis_version_id],
        )?;
        conn.execute(
            "INSERT INTO analysis_finding_version_evidence
                (finding_version_id, evidence_id, synthesis_polarity, rationale)
             VALUES (?1, 'evidence-study', 'supports', 'Linked source evidence.')",
            [finding_version_id],
        )?;
        Ok(())
    }

    fn transfer_fixture() -> Result<(Connection, String, String)> {
        let conn = Connection::open_in_memory()?;
        crate::mind::portable_db::initialize_workspace_schema(&conn)
            .map_err(|error| anyhow!(error.to_string()))?;
        create_project(&conn, "project-a", "Project A")?;
        create_project(&conn, "project-b", "Project B")?;
        let source_version_id = create_accepted_finding(&conn, "project-a", "source")?;
        Ok((conn, source_version_id, "project-b".to_string()))
    }

    fn create_accepted_transfer(
        conn: &mut Connection,
        source_version_id: &str,
        receiving_project_id: &str,
    ) -> Result<KnowledgeTransferDto> {
        let origin = create_origin(
            conn,
            &CreateTransferOriginReq {
                artifact: TransferArtifactRef::FindingVersion {
                    finding_id: "finding-source".to_string(),
                    version_id: source_version_id.to_string(),
                },
                origin_project_id: "project-a".to_string(),
                rationale: "Verified project ownership and accepted finding lifecycle.".to_string(),
                recorded_by: Some("reviewer-a".to_string()),
            },
        )?;
        let eligibility = create_eligibility(
            conn,
            &CreateTransferEligibilityReq {
                origin_id: origin.id,
                license_identifier: "CC BY 4.0".to_string(),
                license_uri: Some("https://creativecommons.org/licenses/by/4.0/".to_string()),
                access_scope: TransferAccessScope::Workspace,
                permitted_purposes: vec![TransferPurpose::Research],
                allowed_project_ids: Vec::new(),
                attribution_text: "Project A, exact finding version".to_string(),
                permission_basis: "Explicit recorded research reuse permission.".to_string(),
                permission_granted: true,
                reviewed_by: "rights-reviewer".to_string(),
            },
        )?;
        adopt_knowledge(
            conn,
            &AdoptKnowledgeReq {
                eligibility_id: eligibility.id,
                receiving_project_id: receiving_project_id.to_string(),
                intended_purpose: TransferPurpose::Research,
                attribution_text: "Project A, exact finding version (attributed)".to_string(),
                context: "Used as prior evidence for Project B.".to_string(),
                requested_by: Some("researcher-b".to_string()),
            },
        )
    }

    #[test]
    fn project_a_to_project_b_adoption_adaptation_and_usage_survive_backup_restore() -> Result<()> {
        let (mut conn, source_version_id, receiving_project_id) = transfer_fixture()?;
        let transfer =
            create_accepted_transfer(&mut conn, &source_version_id, &receiving_project_id)?;
        assert_eq!(transfer.origin.origin_project_id, "project-a");
        assert_eq!(transfer.receiving_project_id, "project-b");
        assert_eq!(transfer.origin.artifact.version_id(), source_version_id);
        assert_eq!(transfer.state, KnowledgeTransferState::Accepted);
        assert_eq!(
            transfer.attribution_text,
            "Project A, exact finding version (attributed)"
        );
        add_source_segment_evidence(
            &conn,
            "project-a",
            &source_version_id,
            "hypothesis-version-source",
        )?;

        let destination_version_id = create_accepted_finding(&conn, "project-b", "adapted")?;
        let adaptation = create_adaptation(
            &mut conn,
            &CreateTransferAdaptationReq {
                transfer_id: transfer.id.clone(),
                artifact: TransferArtifactRef::FindingVersion {
                    finding_id: "finding-adapted".to_string(),
                    version_id: destination_version_id.clone(),
                },
                relation_kind: TransferLineageRelation::DerivedFrom,
                rationale: "Adapted to Project B's population and methods.".to_string(),
                adapted_by: Some("researcher-b".to_string()),
            },
        )?;
        assert_eq!(adaptation.artifact.version_id(), destination_version_id);
        assert_ne!(
            adaptation.artifact.entity_id(),
            transfer.origin.artifact.entity_id()
        );
        record_downstream_usage(
            &mut conn,
            &RecordDownstreamUsageReq {
                transfer_id: transfer.id.clone(),
                reference: DownstreamUseRef::FindingVersion {
                    finding_id: "finding-adapted".to_string(),
                    version_id: destination_version_id,
                },
                use_context: "Project B synthesis.".to_string(),
                recorded_by: Some("researcher-b".to_string()),
            },
        )?;
        let preview = provenance_preview(&conn, &transfer.origin.id)?;
        assert_eq!(preview.scope, "declared scope");
        assert_eq!(preview.provenance.len(), 1);
        assert!(preview.provenance[0].details.contains("Study Source"));
        assert!(preview.provenance[0].details.contains("pages 4-5"));

        let eligibility = workspace(&conn)?.eligibilities[0].clone();
        change_eligibility(
            &mut conn,
            &ChangeTransferEligibilityReq {
                eligibility_id: eligibility.id,
                state: TransferEligibilityState::Withdrawn,
                rationale: "Permission withdrawn for future adoption.".to_string(),
                actor_id: Some("rights-reviewer".to_string()),
            },
        )?;
        assert_eq!(
            workspace(&conn)?.transfers[0].state,
            KnowledgeTransferState::Accepted
        );

        let backup_path: PathBuf =
            std::env::temp_dir().join(format!("vora-transfer-{}.db", Uuid::now_v7()));
        {
            let mut destination = Connection::open(&backup_path)?;
            let backup = Backup::new(&conn, &mut destination)?;
            backup.run_to_completion(5, Duration::from_millis(10), None)?;
        }
        let restored = Connection::open(&backup_path)?;
        crate::mind::portable_db::initialize_workspace_schema(&restored)
            .map_err(|error| anyhow!(error.to_string()))?;
        let restored_workspace = workspace(&restored)?;
        assert_eq!(restored_workspace.transfers.len(), 1);
        assert_eq!(restored_workspace.adaptations.len(), 1);
        assert_eq!(restored_workspace.downstream_usage.len(), 1);
        assert_eq!(restored_workspace.lineage.len(), 2);
        let preserved_source_count: i64 =
            restored.query_row("SELECT COUNT(*) FROM sources", [], |row| row.get(0))?;
        let preserved_framework_adoption_count: i64 = restored.query_row(
            "SELECT COUNT(*) FROM research_framework_adoptions",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(preserved_source_count, 1);
        assert_eq!(preserved_framework_adoption_count, 0);
        assert_eq!(
            restored_workspace.transfers[0].origin.artifact.version_id(),
            source_version_id
        );
        drop(restored);
        fs::remove_file(backup_path)?;
        Ok(())
    }

    #[test]
    fn finding_origin_uses_latest_lifecycle_event_not_any_historical_event() -> Result<()> {
        let (mut conn, source_version_id, _) = transfer_fixture()?;
        conn.execute(
            "INSERT INTO analysis_finding_events
                (id, finding_version_id, status, rationale, actor_id, occurred_at_us)
             VALUES ('finding-retracted', ?1, 'retracted', 'Finding retracted.', NULL, 10)",
            [&source_version_id],
        )?;
        let result = create_origin(
            &mut conn,
            &CreateTransferOriginReq {
                artifact: TransferArtifactRef::FindingVersion {
                    finding_id: "finding-source".to_string(),
                    version_id: source_version_id,
                },
                origin_project_id: "project-a".to_string(),
                rationale: "Origin assertion.".to_string(),
                recorded_by: None,
            },
        );
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn eligibility_enforces_rights_scope_purpose_and_restriction() -> Result<()> {
        let (mut conn, source_version_id, _) = transfer_fixture()?;
        create_project(&conn, "project-c", "Project C")?;
        let origin = create_origin(
            &mut conn,
            &CreateTransferOriginReq {
                artifact: TransferArtifactRef::FindingVersion {
                    finding_id: "finding-source".to_string(),
                    version_id: source_version_id,
                },
                origin_project_id: "project-a".to_string(),
                rationale: "Verified exact source finding.".to_string(),
                recorded_by: None,
            },
        )?;
        let unknown_rights = create_eligibility(
            &mut conn,
            &CreateTransferEligibilityReq {
                origin_id: origin.id.clone(),
                license_identifier: "unknown".to_string(),
                license_uri: None,
                access_scope: TransferAccessScope::Workspace,
                permitted_purposes: vec![TransferPurpose::Research],
                allowed_project_ids: Vec::new(),
                attribution_text: "Attribution".to_string(),
                permission_basis: "Unverified".to_string(),
                permission_granted: true,
                reviewed_by: "reviewer".to_string(),
            },
        );
        assert!(unknown_rights.is_err());

        let eligibility = create_eligibility(
            &mut conn,
            &CreateTransferEligibilityReq {
                origin_id: origin.id,
                license_identifier: "CC BY 4.0".to_string(),
                license_uri: None,
                access_scope: TransferAccessScope::SelectedProjects,
                permitted_purposes: vec![TransferPurpose::Research],
                allowed_project_ids: vec!["project-b".to_string()],
                attribution_text: "Project A attribution".to_string(),
                permission_basis: "Permission reviewed for Project B research.".to_string(),
                permission_granted: true,
                reviewed_by: "rights-reviewer".to_string(),
            },
        )?;
        let disallowed_purpose = adopt_knowledge(
            &mut conn,
            &AdoptKnowledgeReq {
                eligibility_id: eligibility.id.clone(),
                receiving_project_id: "project-b".to_string(),
                intended_purpose: TransferPurpose::Commercial,
                attribution_text: "Project A".to_string(),
                context: "Commercial use".to_string(),
                requested_by: None,
            },
        );
        assert!(disallowed_purpose.is_err());
        let disallowed_project = adopt_knowledge(
            &mut conn,
            &AdoptKnowledgeReq {
                eligibility_id: eligibility.id.clone(),
                receiving_project_id: "project-c".to_string(),
                intended_purpose: TransferPurpose::Research,
                attribution_text: "Project A".to_string(),
                context: "Outside allowlist".to_string(),
                requested_by: None,
            },
        );
        assert!(disallowed_project.is_err());
        assert!(workspace(&conn)?.transfers.is_empty());

        let transfer = adopt_knowledge(
            &mut conn,
            &AdoptKnowledgeReq {
                eligibility_id: eligibility.id.clone(),
                receiving_project_id: "project-b".to_string(),
                intended_purpose: TransferPurpose::Research,
                attribution_text: "Project A".to_string(),
                context: "Permitted research use.".to_string(),
                requested_by: None,
            },
        )?;
        change_eligibility(
            &mut conn,
            &ChangeTransferEligibilityReq {
                eligibility_id: eligibility.id.clone(),
                state: TransferEligibilityState::Restricted,
                rationale: "Restrict future adoption.".to_string(),
                actor_id: None,
            },
        )?;
        let future_adoption = adopt_knowledge(
            &mut conn,
            &AdoptKnowledgeReq {
                eligibility_id: eligibility.id,
                receiving_project_id: "project-b".to_string(),
                intended_purpose: TransferPurpose::Research,
                attribution_text: "Project A".to_string(),
                context: "Should be blocked.".to_string(),
                requested_by: None,
            },
        );
        assert!(future_adoption.is_err());
        let records = workspace(&conn)?;
        assert_eq!(records.transfers.len(), 1);
        assert_eq!(records.transfers[0].id, transfer.id);
        Ok(())
    }

    #[test]
    fn unreviewed_concept_version_cannot_be_offered_as_transferable_knowledge() -> Result<()> {
        let (mut conn, _, _) = transfer_fixture()?;
        let version_id = create_concept_version(
            &mut conn,
            &CreateConceptVersionReq {
                concept_id: None,
                project_id: "project-a".to_string(),
                parent_version_id: None,
                title: "Test concept".to_string(),
                definition: "A test definition.".to_string(),
                rationale: "Initial definition.".to_string(),
                created_by: Some("author".to_string()),
            },
        )?;
        let origin_request = CreateTransferOriginReq {
            artifact: TransferArtifactRef::ConceptVersion {
                concept_id: conn.query_row(
                    "SELECT concept_id FROM knowledge_concept_versions WHERE id = ?1",
                    [&version_id],
                    |row| row.get(0),
                )?,
                version_id: version_id.clone(),
            },
            origin_project_id: "project-a".to_string(),
            rationale: "Origin is explicitly asserted.".to_string(),
            recorded_by: None,
        };
        assert!(create_origin(&mut conn, &origin_request).is_err());
        review_knowledge_version(
            &mut conn,
            &ReviewKnowledgeVersionReq {
                version_id,
                kind: KnowledgeVersionKind::Concept,
                status: KnowledgeVersionStatus::Reviewed,
                rationale: "Reviewed content quality only.".to_string(),
                actor_id: Some("reviewer".to_string()),
            },
        )?;
        assert!(create_origin(&mut conn, &origin_request).is_ok());
        Ok(())
    }
}
