use super::*;
use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::Value;
use sha2::{Digest, Sha256};

const TRANSFER_SCHEMA_VERSION: i64 = 1;
const TRANSFER_SCHEMA_ID: &str = "knowledge-transfer-v1";
const TRANSFER_SCHEMA: &str = include_str!("schema.sql");

#[derive(Debug)]
struct OriginRecord {
    id: String,
    artifact_kind: TransferArtifactKind,
    artifact_version_id: String,
    origin_project_id: String,
    rationale: String,
}

#[derive(Debug)]
struct CatalogRecord {
    artifact_kind: TransferArtifactKind,
    artifact_id: String,
    artifact_version_id: String,
    title: String,
    version_no: i64,
    owner_project_id: Option<String>,
    scientific_status: String,
}

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS transfer_schema_migrations (
             version INTEGER PRIMARY KEY,
             migration_id TEXT NOT NULL UNIQUE,
             checksum TEXT NOT NULL,
             applied_at_us INTEGER NOT NULL
         );",
    )?;
    let enabled: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    if enabled != 1 {
        return Err(anyhow!(
            "SQLite foreign-key enforcement is required for Knowledge Transfer."
        ));
    }

    let checksum = format!("{:x}", Sha256::digest(TRANSFER_SCHEMA.as_bytes()));
    let tx = conn.unchecked_transaction()?;
    let applied = tx
        .query_row(
            "SELECT migration_id, checksum FROM transfer_schema_migrations WHERE version = ?1",
            [TRANSFER_SCHEMA_VERSION],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    match applied {
        Some((migration_id, applied_checksum))
            if migration_id != TRANSFER_SCHEMA_ID || applied_checksum != checksum =>
        {
            return Err(anyhow!(
                "Knowledge Transfer migration 1 differs from the applied migration; add a new numbered migration."
            ));
        }
        Some(_) => {}
        None => {
            tx.execute_batch(TRANSFER_SCHEMA)
                .context("apply Knowledge Transfer schema")?;
            tx.execute(
                "INSERT INTO transfer_schema_migrations
                    (version, migration_id, checksum, applied_at_us)
                 VALUES (?1, ?2, ?3, ?4)",
                params![TRANSFER_SCHEMA_VERSION, TRANSFER_SCHEMA_ID, checksum, now_us()?],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn workspace(conn: &Connection) -> Result<TransferWorkspaceDto> {
    let artifacts = query_artifacts(conn)?;
    let eligibility_ids = query_ids(
        conn,
        "SELECT id FROM knowledge_transfer_eligibility ORDER BY reviewed_at_us DESC, id",
    )?;
    let eligibility_records = eligibility_ids
        .iter()
        .map(|id| query_eligibility(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let transfer_ids = query_ids(
        conn,
        "SELECT id FROM knowledge_transfers ORDER BY requested_at_us DESC, id",
    )?;
    let transfers = transfer_ids
        .iter()
        .map(|id| query_transfer(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let adaptations = query_adaptations(conn)?;
    let downstream_usage = query_downstream_usage(conn)?;
    Ok(TransferWorkspaceDto {
        artifacts,
        eligibility_records,
        transfers,
        adaptations,
        downstream_usage,
    })
}

pub fn create_artifact_version(
    conn: &mut Connection,
    request: &CreateArtifactVersionReq,
    id: &str,
    now: i64,
) -> Result<TransferArtifactDto> {
    let tx = conn.transaction()?;
    let version_no = match &request.artifact_kind {
        TransferArtifactKind::ConceptVersion => {
            exists(&tx, "SELECT EXISTS(SELECT 1 FROM knowledge_concepts WHERE id = ?1)", &request.artifact_id, "concept")?;
            insert_concept_version(&tx, request, id, now)?
        }
        TransferArtifactKind::PrincipleVersion => {
            exists(&tx, "SELECT EXISTS(SELECT 1 FROM knowledge_principles WHERE id = ?1)", &request.artifact_id, "principle")?;
            insert_principle_version(&tx, request, id, now)?
        }
        _ => return Err(anyhow!("only concept and principle versions are created by this workflow")),
    };
    tx.commit()?;
    let kind = request.artifact_kind.as_str();
    query_artifact(conn, kind, id)?.ok_or_else(|| {
        anyhow!("created artifact version {id} (v{version_no}) could not be reloaded")
    })
}

fn insert_concept_version(
    tx: &Transaction<'_>,
    request: &CreateArtifactVersionReq,
    id: &str,
    now: i64,
) -> Result<i64> {
    let (version_no, parent_version_id) = next_version(
        tx,
        "knowledge_concept_versions",
        "concept_id",
        &request.artifact_id,
        request.parent_version_id.as_deref(),
    )?;
    tx.execute(
        "INSERT INTO knowledge_concept_versions
            (id, concept_id, version_no, created_in_project_id, title, definition,
             parent_version_id, rationale, reviewed_by, reviewed_at_us, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?10)",
        params![
            id,
            request.artifact_id,
            version_no,
            request.project_id,
            request.title.trim(),
            request.content.trim(),
            parent_version_id,
            request.rationale.trim(),
            request.reviewed_by,
            now,
            request.created_by
        ],
    )?;
    Ok(version_no)
}

fn insert_principle_version(
    tx: &Transaction<'_>,
    request: &CreateArtifactVersionReq,
    id: &str,
    now: i64,
) -> Result<i64> {
    let (version_no, parent_version_id) = next_version(
        tx,
        "knowledge_principle_versions",
        "principle_id",
        &request.artifact_id,
        request.parent_version_id.as_deref(),
    )?;
    tx.execute(
        "INSERT INTO knowledge_principle_versions
            (id, principle_id, version_no, created_in_project_id, title, statement,
             parent_version_id, rationale, reviewed_by, reviewed_at_us, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?10)",
        params![
            id,
            request.artifact_id,
            version_no,
            request.project_id,
            request.title.trim(),
            request.content.trim(),
            parent_version_id,
            request.rationale.trim(),
            request.reviewed_by,
            now,
            request.created_by
        ],
    )?;
    Ok(version_no)
}

fn next_version(
    conn: &Connection,
    table: &str,
    owner_column: &str,
    owner_id: &str,
    parent_version_id: Option<&str>,
) -> Result<(i64, Option<String>)> {
    let count_sql = format!("SELECT COUNT(*) FROM {table} WHERE {owner_column} = ?1");
    let count: i64 = conn.query_row(&count_sql, [owner_id], |row| row.get(0))?;
    match (count, parent_version_id) {
        (0, None) => Ok((1, None)),
        (0, Some(_)) => Err(anyhow!("a first artifact version cannot have a parent")),
        (_, None) => Err(anyhow!("a later artifact version must pin its parent version")),
        (_, Some(parent_id)) => {
            let parent_sql = format!(
                "SELECT version_no FROM {table} WHERE id = ?1 AND {owner_column} = ?2"
            );
            let parent_no: i64 = conn
                .query_row(&parent_sql, params![parent_id, owner_id], |row| row.get(0))
                .with_context(|| format!("parent version {parent_id} does not belong to {owner_id}"))?;
            let version_no = count + 1;
            if parent_no >= version_no {
                return Err(anyhow!("parent version must precede the new version"));
            }
            Ok((version_no, Some(parent_id.to_string())))
        }
    }
}

pub fn record_origin(
    conn: &mut Connection,
    request: &RecordTransferOriginReq,
    id: &str,
    now: i64,
) -> Result<String> {
    let tx = conn.transaction()?;
    let artifact = query_artifact(&tx, request.artifact_kind.as_str(), &request.artifact_version_id)?
        .ok_or_else(|| anyhow!("artifact version does not exist"))?;
    if let Some(owner_id) = artifact.owner_project_id.as_deref() {
        if owner_id != request.origin_project_id {
            return Err(anyhow!(
                "origin project does not match the recorded owner of this artifact version"
            ));
        }
    }
    exists(
        &tx,
        "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
        &request.origin_project_id,
        "origin research project",
    )?;

    if let Some(origin) = query_origin(&tx, &request.artifact_kind, &request.artifact_version_id)? {
        if origin.origin_project_id == request.origin_project_id {
            tx.commit()?;
            return Ok(origin.id);
        }
        return Err(anyhow!(
            "this exact artifact version already has a different immutable origin assertion"
        ));
    }

    insert_origin(
        &tx,
        id,
        &request.artifact_kind,
        &request.artifact_version_id,
        &request.origin_project_id,
        request.rationale.trim(),
        request.recorded_by.as_deref(),
        now,
    )?;
    tx.commit()?;
    Ok(id.to_string())
}

fn insert_origin(
    tx: &Transaction<'_>,
    id: &str,
    kind: &TransferArtifactKind,
    artifact_version_id: &str,
    project_id: &str,
    rationale: &str,
    actor_id: Option<&str>,
    now: i64,
) -> Result<()> {
    let target_column = target_column(kind);
    let sql = format!(
        "INSERT INTO knowledge_transfer_origins
            (id, artifact_kind, {target_column}, origin_project_id, rationale, recorded_by, recorded_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    );
    tx.execute(
        &sql,
        params![id, kind.as_str(), artifact_version_id, project_id, rationale, actor_id, now],
    )?;
    Ok(())
}

pub fn create_eligibility(
    conn: &mut Connection,
    request: &CreateTransferEligibilityReq,
    id: &str,
    event_id: &str,
    now: i64,
) -> Result<TransferEligibilityDto> {
    let purposes = unique_purposes(&request.allowed_purposes)?;
    let mut recipients = request.allowed_project_ids.clone();
    recipients.sort();
    recipients.dedup();
    if recipients.len() != request.allowed_project_ids.len() {
        return Err(anyhow!("recipient projects must not be duplicated"));
    }

    let tx = conn.transaction()?;
    let origin = query_origin_by_id(&tx, &request.origin_id)?
        .ok_or_else(|| anyhow!("origin assertion does not exist"))?;
    let artifact = query_catalog_item(&tx, &origin.artifact_kind, &origin.artifact_version_id)?
        .ok_or_else(|| anyhow!("origin artifact version does not exist"))?;
    if !is_reusable_status(&artifact.artifact_kind, &artifact.scientific_status) {
        return Err(anyhow!(
            "artifact version is not in a reviewed, published, or released state"
        ));
    }
    if request.access_scope == TransferAccessScope::Workspace && !recipients.is_empty() {
        return Err(anyhow!(
            "workspace-scoped eligibility must not list individual recipient projects"
        ));
    }
    if request.access_scope == TransferAccessScope::SelectedProjects && recipients.is_empty() {
        return Err(anyhow!(
            "selected-project eligibility must list at least one recipient project"
        ));
    }
    for recipient in &recipients {
        if recipient == &origin.origin_project_id {
            return Err(anyhow!("the origin project cannot be an adoption recipient"));
        }
        exists(
            &tx,
            "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
            recipient,
            "recipient research project",
        )?;
    }
    let purposes_json = serde_json::to_string(&purposes)?;
    tx.execute(
        "INSERT INTO knowledge_transfer_eligibility
            (id, origin_id, license_identifier, license_uri, access_scope,
             allowed_purposes_json, attribution_text, permission_basis,
             permission_granted, reviewed_by, reviewed_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10)",
        params![
            id,
            request.origin_id,
            request.license_identifier.trim(),
            request.license_uri.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            request.access_scope.as_str(),
            purposes_json,
            request.attribution_text.trim(),
            request.permission_basis.trim(),
            request.reviewed_by,
            now
        ],
    )?;
    for recipient in &recipients {
        tx.execute(
            "INSERT INTO knowledge_transfer_eligibility_projects (eligibility_id, project_id)
             VALUES (?1, ?2)",
            params![id, recipient],
        )?;
    }
    tx.execute(
        "INSERT INTO knowledge_transfer_eligibility_events
            (id, eligibility_id, status, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, 'eligible', 'Permission and transfer scope explicitly reviewed.', ?3, ?4)",
        params![event_id, id, request.reviewed_by, now],
    )?;
    tx.commit()?;
    query_eligibility(conn, id)
}

pub fn change_eligibility(
    conn: &Connection,
    request: &ChangeTransferEligibilityReq,
    event_id: &str,
    now: i64,
) -> Result<TransferEligibilityDto> {
    let tx = conn.unchecked_transaction()?;
    let current = current_eligibility_state(&tx, &request.eligibility_id)?;
    if current != TransferEligibilityState::Eligible {
        return Err(anyhow!(
            "only currently eligible transfer permissions can be restricted or withdrawn"
        ));
    }
    if request.next_state == TransferEligibilityState::Eligible {
        return Err(anyhow!(
            "eligibility cannot be restored in place; create a new reviewed eligibility record"
        ));
    }
    tx.execute(
        "INSERT INTO knowledge_transfer_eligibility_events
            (id, eligibility_id, status, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            event_id,
            request.eligibility_id,
            request.next_state.as_str(),
            request.rationale.trim(),
            request.actor_id,
            now
        ],
    )?;
    tx.commit()?;
    query_eligibility(conn, &request.eligibility_id)
}

pub fn create_transfer(
    conn: &mut Connection,
    request: &CreateKnowledgeTransferReq,
    id: &str,
    event_id: &str,
    now: i64,
) -> Result<KnowledgeTransferDto> {
    let tx = conn.transaction()?;
    let eligibility = query_eligibility(&tx, &request.eligibility_id)?;
    validate_eligibility_for_use(
        &tx,
        &eligibility,
        &request.receiving_project_id,
        &request.intended_purpose,
    )?;
    if request.receiving_project_id == eligibility.origin_project_id {
        return Err(anyhow!(
            "knowledge transfer requires a receiving project distinct from its origin"
        ));
    }
    if let Some(previous_id) = request.reasserts_transfer_id.as_deref() {
        let previous = query_transfer(&tx, previous_id)?;
        if previous.state != KnowledgeTransferState::Withdrawn
            || previous.origin_project_id != eligibility.origin_project_id
            || previous.receiving_project_id != request.receiving_project_id
            || previous.artifact_version_id != eligibility.artifact_version_id
            || previous.artifact_kind != eligibility.artifact_kind
        {
            return Err(anyhow!(
                "reasserted transfer must be a withdrawn transfer of the same version between the same projects"
            ));
        }
    }
    tx.execute(
        "INSERT INTO knowledge_transfers
            (id, eligibility_id, origin_project_id, receiving_project_id,
             intended_purpose, use_context, attribution_text, requested_by,
             requested_at_us, reasserts_transfer_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            id,
            request.eligibility_id,
            eligibility.origin_project_id,
            request.receiving_project_id,
            request.intended_purpose.as_str(),
            request.use_context.trim(),
            request.attribution_text.trim(),
            request.requested_by,
            now,
            request.reasserts_transfer_id
        ],
    )?;
    tx.execute(
        "INSERT INTO knowledge_transfer_events
            (id, transfer_id, status, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, 'proposed', 'Adoption requested.', ?3, ?4)",
        params![event_id, id, request.requested_by, now],
    )?;
    tx.commit()?;
    query_transfer(conn, id)
}

pub fn transition_transfer(
    conn: &Connection,
    request: &TransitionKnowledgeTransferReq,
    event_id: &str,
    now: i64,
) -> Result<KnowledgeTransferDto> {
    let tx = conn.unchecked_transaction()?;
    let transfer = query_transfer(&tx, &request.transfer_id)?;
    let valid = matches!(
        (&transfer.state, &request.next_state),
        (KnowledgeTransferState::Proposed, KnowledgeTransferState::Accepted)
            | (KnowledgeTransferState::Proposed, KnowledgeTransferState::Rejected)
            | (KnowledgeTransferState::Accepted, KnowledgeTransferState::Withdrawn)
    );
    if !valid {
        return Err(anyhow!(
            "invalid knowledge transfer transition from {:?} to {:?}",
            transfer.state,
            request.next_state
        ));
    }
    if request.next_state == KnowledgeTransferState::Accepted {
        let eligibility = query_eligibility(&tx, &transfer.eligibility_id)?;
        validate_eligibility_for_use(
            &tx,
            &eligibility,
            &transfer.receiving_project_id,
            &transfer.intended_purpose,
        )?;
    }
    tx.execute(
        "INSERT INTO knowledge_transfer_events
            (id, transfer_id, status, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            event_id,
            transfer.id,
            request.next_state.as_str(),
            request.rationale.trim(),
            request.actor_id,
            now
        ],
    )?;
    tx.commit()?;
    query_transfer(conn, &transfer.id)
}

pub fn create_adaptation(
    conn: &Connection,
    request: &CreateTransferAdaptationReq,
    id: &str,
    now: i64,
) -> Result<TransferAdaptationDto> {
    let tx = conn.unchecked_transaction()?;
    let transfer = query_transfer(&tx, &request.transfer_id)?;
    if transfer.state != KnowledgeTransferState::Accepted {
        return Err(anyhow!("only an accepted transfer can be adapted"));
    }
    let eligibility = query_eligibility(&tx, &transfer.eligibility_id)?;
    if eligibility.state != TransferEligibilityState::Eligible {
        return Err(anyhow!(
            "adaptation is blocked because the source permission is restricted or withdrawn"
        ));
    }
    if request.artifact_kind != transfer.artifact_kind {
        return Err(anyhow!(
            "adaptation must retain the transferred artifact's semantic kind"
        ));
    }
    let target = query_artifact(
        &tx,
        request.artifact_kind.as_str(),
        &request.artifact_version_id,
    )?
    .ok_or_else(|| anyhow!("adapted artifact version does not exist"))?;
    if let Some(owner) = target.owner_project_id.as_deref() {
        if owner != transfer.receiving_project_id {
            return Err(anyhow!(
                "adapted artifact version is not owned by the receiving project"
            ));
        }
    } else {
        let target_origin = query_origin(
            &tx,
            &request.artifact_kind,
            &request.artifact_version_id,
        )?
        .ok_or_else(|| anyhow!("adapted framework version has no explicit origin assertion"))?;
        if target_origin.origin_project_id != transfer.receiving_project_id {
            return Err(anyhow!(
                "adapted framework version origin does not match the receiving project"
            ));
        }
    }
    let target_column = target_column(&request.artifact_kind);
    let sql = format!(
        "INSERT INTO knowledge_transfer_adaptations
            (id, transfer_id, receiving_project_id, artifact_kind, {target_column},
             rationale, adapted_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
    );
    tx.execute(
        &sql,
        params![
            id,
            request.transfer_id,
            transfer.receiving_project_id,
            request.artifact_kind.as_str(),
            request.artifact_version_id,
            request.rationale.trim(),
            request.adapted_by,
            now
        ],
    )?;
    tx.commit()?;
    query_adaptation(conn, id)
}

pub fn create_downstream_usage(
    conn: &Connection,
    request: &CreateTransferUsageReq,
    id: &str,
    now: i64,
) -> Result<TransferDownstreamUsageDto> {
    let tx = conn.unchecked_transaction()?;
    let transfer = query_transfer(&tx, &request.transfer_id)?;
    if transfer.state != KnowledgeTransferState::Accepted
        || transfer.receiving_project_id != request.project_id
    {
        return Err(anyhow!(
            "downstream use must belong to the receiving project of an accepted transfer"
        ));
    }
    let eligibility = query_eligibility(&tx, &transfer.eligibility_id)?;
    if eligibility.state != TransferEligibilityState::Eligible {
        return Err(anyhow!(
            "new downstream use is blocked by the current transfer restriction"
        ));
    }
    insert_usage(&tx, request, id, now)?;
    tx.commit()?;
    query_downstream_usage_by_id(conn, id)
}

fn insert_usage(
    tx: &Transaction<'_>,
    request: &CreateTransferUsageReq,
    id: &str,
    now: i64,
) -> Result<()> {
    let column = match request.usage_kind.as_str() {
        "concept_version" => "concept_version_id",
        "principle_version" => "principle_version_id",
        "framework_version" => "framework_version_id",
        "finding_version" => "finding_version_id",
        "model_version" => "model_version_id",
        "model_evaluation" => "model_evaluation_id",
        "analysis_run" => "analysis_run_id",
        "experiment_run" => "experiment_run_id",
        _ => return Err(anyhow!("unsupported downstream usage kind")),
    };
    let sql = format!(
        "INSERT INTO knowledge_transfer_downstream_usage
            (id, transfer_id, project_id, usage_kind, {column}, use_context,
             recorded_by, recorded_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
    );
    tx.execute(
        &sql,
        params![
            id,
            request.transfer_id,
            request.project_id,
            request.usage_kind,
            request.artifact_id,
            request.use_context.trim(),
            request.recorded_by,
            now
        ],
    )?;
    Ok(())
}

fn validate_eligibility_for_use(
    conn: &Connection,
    eligibility: &TransferEligibilityDto,
    recipient_id: &str,
    purpose: &TransferPurpose,
) -> Result<()> {
    if eligibility.state != TransferEligibilityState::Eligible {
        return Err(anyhow!("the pinned transfer permission is not currently eligible"));
    }
    if !eligibility.allowed_purposes.contains(purpose) {
        return Err(anyhow!("the declared license does not permit this intended use"));
    }
    match &eligibility.access_scope {
        TransferAccessScope::Workspace => {}
        TransferAccessScope::SelectedProjects
            if eligibility.allowed_project_ids.iter().any(|id| id == recipient_id) => {}
        TransferAccessScope::SelectedProjects => {
            return Err(anyhow!(
                "the receiving project is outside the declared access scope"
            ))
        }
    }
    exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
        recipient_id,
        "receiving research project",
    )
}

fn unique_purposes(purposes: &[TransferPurpose]) -> Result<Vec<&'static str>> {
    let values = purposes.iter().map(TransferPurpose::as_str).collect::<Vec<_>>();
    if values.is_empty() {
        return Err(anyhow!("at least one allowed purpose is required"));
    }
    let unique = values.iter().copied().collect::<std::collections::BTreeSet<_>>();
    if unique.len() != values.len() {
        return Err(anyhow!("allowed purposes must not be duplicated"));
    }
    Ok(values)
}

fn is_reusable_status(kind: &TransferArtifactKind, status: &str) -> bool {
    match kind {
        TransferArtifactKind::ConceptVersion | TransferArtifactKind::PrincipleVersion => {
            status == "reviewed"
        }
        TransferArtifactKind::FrameworkVersion => status == "published",
        TransferArtifactKind::FindingVersion => {
            matches!(status, "accepted" | "contested" | "unsupported" | "inconclusive")
        }
        TransferArtifactKind::ModelVersion => status == "released",
    }
}

fn target_column(kind: &TransferArtifactKind) -> &'static str {
    match kind {
        TransferArtifactKind::ConceptVersion => "concept_version_id",
        TransferArtifactKind::PrincipleVersion => "principle_version_id",
        TransferArtifactKind::FrameworkVersion => "framework_version_id",
        TransferArtifactKind::FindingVersion => "finding_version_id",
        TransferArtifactKind::ModelVersion => "model_version_id",
    }
}

fn query_artifacts(conn: &Connection) -> Result<Vec<TransferArtifactDto>> {
    let mut statement = conn.prepare(
        "SELECT artifact_kind, artifact_version_id FROM knowledge_transfer_artifact_catalog
         ORDER BY artifact_kind, title, version_no DESC, artifact_version_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let ids = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    ids.iter()
        .map(|(kind, id)| {
            query_artifact(conn, kind, id)?.ok_or_else(|| anyhow!("catalog artifact disappeared"))
        })
        .collect()
}

fn query_artifact(
    conn: &Connection,
    kind: &str,
    version_id: &str,
) -> Result<Option<TransferArtifactDto>> {
    let Some(catalog) = query_catalog_row(conn, kind, version_id)? else {
        return Ok(None);
    };
    let origin = query_origin(conn, &catalog.artifact_kind, &catalog.artifact_version_id)?;
    let mut dto = TransferArtifactDto {
        artifact_kind: catalog.artifact_kind.clone(),
        artifact_id: catalog.artifact_id,
        artifact_version_id: catalog.artifact_version_id,
        title: catalog.title,
        version_no: catalog.version_no,
        owner_project_id: catalog.owner_project_id,
        origin_project_id: origin.as_ref().map(|record| record.origin_project_id.clone()),
        origin_rationale: origin.as_ref().map(|record| record.rationale.clone()),
        scientific_status: catalog.scientific_status.clone(),
        eligibility_id: None,
        eligibility_status: None,
        license_identifier: None,
        license_uri: None,
        access_scope: None,
        allowed_purposes: Vec::new(),
        attribution_text: None,
        permission_basis: None,
        block_reason: None,
    };
    if let Some(origin) = origin {
        if let Some(eligibility_id) = latest_eligibility_id(conn, &origin.id)? {
            let eligibility = query_eligibility(conn, &eligibility_id)?;
            dto.eligibility_id = Some(eligibility.id.clone());
            dto.eligibility_status = Some(eligibility.state.clone());
            dto.license_identifier = Some(eligibility.license_identifier.clone());
            dto.license_uri = eligibility.license_uri.clone();
            dto.access_scope = Some(eligibility.access_scope.clone());
            dto.allowed_purposes = eligibility.allowed_purposes.clone();
            dto.attribution_text = Some(eligibility.attribution_text.clone());
            dto.permission_basis = Some(eligibility.permission_basis.clone());
            if eligibility.state != TransferEligibilityState::Eligible {
                dto.block_reason = Some(format!(
                    "future transfer is {}",
                    eligibility.state.as_str()
                ));
            }
        } else {
            dto.block_reason = Some("no explicit license, access scope, and permission record".into());
        }
    } else {
        dto.block_reason = Some("origin project is unknown and has not been explicitly recorded".into());
    }
    if !is_reusable_status(&catalog.artifact_kind, &catalog.scientific_status) {
        dto.block_reason = Some(format!(
            "artifact state '{}' is not eligible for transfer",
            catalog.scientific_status
        ));
    }
    Ok(Some(dto))
}

fn query_catalog_item(
    conn: &Connection,
    kind: &TransferArtifactKind,
    version_id: &str,
) -> Result<Option<CatalogRecord>> {
    query_catalog_row(conn, kind.as_str(), version_id)
}

fn query_catalog_row(
    conn: &Connection,
    kind: &str,
    version_id: &str,
) -> Result<Option<CatalogRecord>> {
    conn.query_row(
        "SELECT artifact_kind, artifact_id, artifact_version_id, title, version_no,
                owner_project_id, scientific_status
         FROM knowledge_transfer_artifact_catalog
         WHERE artifact_kind = ?1 AND artifact_version_id = ?2",
        params![kind, version_id],
        |row| {
            let kind = row.get::<_, String>(0)?;
            Ok(CatalogRecord {
                artifact_kind: TransferArtifactKind::parse(&kind)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                artifact_id: row.get(1)?,
                artifact_version_id: row.get(2)?,
                title: row.get(3)?,
                version_no: row.get(4)?,
                owner_project_id: row.get(5)?,
                scientific_status: row.get(6)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn query_origin(
    conn: &Connection,
    kind: &TransferArtifactKind,
    version_id: &str,
) -> Result<Option<OriginRecord>> {
    let column = target_column(kind);
    let sql = format!(
        "SELECT id, artifact_kind, {column}, origin_project_id, rationale
         FROM knowledge_transfer_origins
         WHERE artifact_kind = ?1 AND {column} = ?2"
    );
    conn.query_row(&sql, params![kind.as_str(), version_id], |row| {
        let kind = row.get::<_, String>(1)?;
        Ok(OriginRecord {
            id: row.get(0)?,
            artifact_kind: TransferArtifactKind::parse(&kind)
                .map_err(|_| rusqlite::Error::InvalidQuery)?,
            artifact_version_id: row.get(2)?,
            origin_project_id: row.get(3)?,
            rationale: row.get(4)?,
        })
    })
    .optional()
    .map_err(Into::into)
}

fn query_origin_by_id(conn: &Connection, id: &str) -> Result<Option<OriginRecord>> {
    conn.query_row(
        "SELECT id, artifact_kind, concept_version_id, principle_version_id,
                framework_version_id, finding_version_id, model_version_id,
                origin_project_id, rationale
         FROM knowledge_transfer_origins WHERE id = ?1",
        [id],
        |row| {
            let kind_text = row.get::<_, String>(1)?;
            let kind = TransferArtifactKind::parse(&kind_text)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let index = match kind {
                TransferArtifactKind::ConceptVersion => 2,
                TransferArtifactKind::PrincipleVersion => 3,
                TransferArtifactKind::FrameworkVersion => 4,
                TransferArtifactKind::FindingVersion => 5,
                TransferArtifactKind::ModelVersion => 6,
            };
            Ok(OriginRecord {
                id: row.get(0)?,
                artifact_kind: kind,
                artifact_version_id: row.get(index)?,
                origin_project_id: row.get(7)?,
                rationale: row.get(8)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn latest_eligibility_id(conn: &Connection, origin_id: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT id FROM knowledge_transfer_eligibility
         WHERE origin_id = ?1 ORDER BY reviewed_at_us DESC, id DESC LIMIT 1",
        [origin_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn query_eligibility(conn: &Connection, id: &str) -> Result<TransferEligibilityDto> {
    let (origin_id, license, uri, scope, purposes_json, attribution, basis, reviewer, reviewed_at) =
        conn.query_row(
            "SELECT origin_id, license_identifier, license_uri, access_scope,
                    allowed_purposes_json, attribution_text, permission_basis,
                    reviewed_by, reviewed_at_us
             FROM knowledge_transfer_eligibility WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            },
        )?;
    let origin = query_origin_by_id(conn, &origin_id)?
        .ok_or_else(|| anyhow!("eligibility references a missing origin assertion"))?;
    let artifact = query_catalog_item(conn, &origin.artifact_kind, &origin.artifact_version_id)?
        .ok_or_else(|| anyhow!("eligibility references a missing artifact version"))?;
    let purposes = parse_purposes(&purposes_json)?;
    let allowed_project_ids = query_ids_for(
        conn,
        "SELECT project_id FROM knowledge_transfer_eligibility_projects
         WHERE eligibility_id = ?1 ORDER BY project_id",
        id,
    )?;
    let (state_text, rationale) = conn.query_row(
        "SELECT status, rationale FROM knowledge_transfer_eligibility_events
         WHERE eligibility_id = ?1 ORDER BY sequence DESC LIMIT 1",
        [id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?;
    Ok(TransferEligibilityDto {
        id: id.into(),
        artifact_kind: origin.artifact_kind,
        artifact_id: artifact.artifact_id,
        artifact_version_id: artifact.artifact_version_id,
        artifact_title: artifact.title,
        origin_id,
        origin_project_id: origin.origin_project_id,
        license_identifier: license,
        license_uri: uri,
        access_scope: parse_scope(&scope)?,
        allowed_purposes: purposes,
        allowed_project_ids,
        attribution_text: attribution,
        permission_basis: basis,
        reviewed_by: reviewer,
        reviewed_at_us: reviewed_at,
        state: parse_eligibility_state(&state_text)?,
        state_rationale: rationale,
    })
}

fn current_eligibility_state(
    conn: &Connection,
    eligibility_id: &str,
) -> Result<TransferEligibilityState> {
    let text: String = conn.query_row(
        "SELECT status FROM knowledge_transfer_eligibility_events
         WHERE eligibility_id = ?1 ORDER BY sequence DESC LIMIT 1",
        [eligibility_id],
        |row| row.get(0),
    )?;
    parse_eligibility_state(&text)
}

fn query_transfer(conn: &Connection, id: &str) -> Result<KnowledgeTransferDto> {
    let (
        eligibility_id,
        origin_project_id,
        receiving_project_id,
        purpose,
        context,
        attribution,
        requested_by,
        requested_at,
        reasserts,
    ) = conn.query_row(
        "SELECT eligibility_id, origin_project_id, receiving_project_id,
                intended_purpose, use_context, attribution_text, requested_by,
                requested_at_us, reasserts_transfer_id
         FROM knowledge_transfers WHERE id = ?1",
        [id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, Option<String>>(8)?,
            ))
        },
    )?;
    let eligibility = query_eligibility(conn, &eligibility_id)?;
    let mut event_statement = conn.prepare(
        "SELECT id, status, rationale, actor_id, occurred_at_us
         FROM knowledge_transfer_events WHERE transfer_id = ?1 ORDER BY sequence",
    )?;
    let event_rows = event_statement.query_map([id], |row| {
        let status: String = row.get(1)?;
        Ok(KnowledgeTransferEventDto {
            id: row.get(0)?,
            state: parse_transfer_state(&status).map_err(|_| rusqlite::Error::InvalidQuery)?,
            rationale: row.get(2)?,
            actor_id: row.get(3)?,
            occurred_at_us: row.get(4)?,
        })
    })?;
    let events = event_rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let state = events
        .last()
        .map(|event| event.state.clone())
        .ok_or_else(|| anyhow!("transfer has no lifecycle event"))?;
    Ok(KnowledgeTransferDto {
        id: id.into(),
        eligibility_id,
        artifact_kind: eligibility.artifact_kind,
        artifact_id: eligibility.artifact_id,
        artifact_version_id: eligibility.artifact_version_id,
        artifact_title: eligibility.artifact_title,
        origin_project_id,
        receiving_project_id,
        intended_purpose: TransferPurpose::parse(&purpose)?,
        use_context: context,
        attribution_text: attribution,
        requested_by,
        requested_at_us: requested_at,
        reasserts_transfer_id: reasserts,
        state,
        events,
    })
}

fn query_adaptations(conn: &Connection) -> Result<Vec<TransferAdaptationDto>> {
    let ids = query_ids(conn, "SELECT id FROM knowledge_transfer_adaptations ORDER BY created_at_us DESC, id")?;
    ids.iter().map(|id| query_adaptation(conn, id)).collect()
}

fn query_adaptation(conn: &Connection, id: &str) -> Result<TransferAdaptationDto> {
    let (transfer_id, project_id, kind_text, version_id, rationale, adapted_by, created_at) =
        conn.query_row(
            "SELECT transfer_id, receiving_project_id, artifact_kind,
                    COALESCE(concept_version_id, principle_version_id, framework_version_id,
                             finding_version_id, model_version_id),
                    rationale, adapted_by, created_at_us
             FROM knowledge_transfer_adaptations WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            },
        )?;
    let kind = TransferArtifactKind::parse(&kind_text)?;
    let target = query_catalog_item(conn, &kind, &version_id)?
        .ok_or_else(|| anyhow!("adapted artifact version disappeared"))?;
    Ok(TransferAdaptationDto {
        id: id.into(),
        transfer_id,
        receiving_project_id: project_id,
        artifact_kind: kind,
        artifact_id: target.artifact_id,
        artifact_version_id: version_id,
        artifact_title: target.title,
        rationale,
        adapted_by,
        created_at_us: created_at,
    })
}

fn query_downstream_usage(conn: &Connection) -> Result<Vec<TransferDownstreamUsageDto>> {
    let mut statement = conn.prepare(
        "SELECT u.id, u.transfer_id, u.project_id, u.usage_kind,
                COALESCE(u.concept_version_id, u.principle_version_id,
                         u.framework_version_id, u.finding_version_id,
                         u.model_version_id, u.model_evaluation_id,
                         u.analysis_run_id, u.experiment_run_id),
                u.use_context, u.recorded_by, u.recorded_at_us
         FROM knowledge_transfer_downstream_usage u
         ORDER BY u.recorded_at_us DESC, u.id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, i64>(7)?,
        ))
    })?;
    let records = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    records
        .iter()
        .map(|(id, transfer_id, project_id, kind, artifact_id, context, actor, time)| {
            query_downstream_usage_by_id(conn, id).map(|mut usage| {
                usage.transfer_id = transfer_id.clone();
                usage.project_id = project_id.clone();
                usage.usage_kind = kind.clone();
                usage.artifact_id = artifact_id.clone();
                usage.use_context = context.clone();
                usage.recorded_by = actor.clone();
                usage.recorded_at_us = *time;
                usage
            })
        })
        .collect()
}

fn query_downstream_usage_by_id(
    conn: &Connection,
    id: &str,
) -> Result<TransferDownstreamUsageDto> {
    let (transfer_id, project_id, kind, artifact_id, context, actor, time) = conn.query_row(
        "SELECT transfer_id, project_id, usage_kind,
                COALESCE(concept_version_id, principle_version_id,
                         framework_version_id, finding_version_id,
                         model_version_id, model_evaluation_id,
                         analysis_run_id, experiment_run_id),
                use_context, recorded_by, recorded_at_us
         FROM knowledge_transfer_downstream_usage WHERE id = ?1",
        [id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, i64>(6)?,
            ))
        },
    )?;
    let title = match kind.as_str() {
        "concept_version" => title_for(conn, "concept_version", &artifact_id)?,
        "principle_version" => title_for(conn, "principle_version", &artifact_id)?,
        "framework_version" => title_for(conn, "framework_version", &artifact_id)?,
        "finding_version" => title_for(conn, "finding_version", &artifact_id)?,
        "model_version" => title_for(conn, "model_version", &artifact_id)?,
        "model_evaluation" => format!("Model evaluation {artifact_id}"),
        "analysis_run" => format!("Analysis run {artifact_id}"),
        "experiment_run" => format!("Experiment run {artifact_id}"),
        _ => return Err(anyhow!("unsupported downstream usage kind")),
    };
    Ok(TransferDownstreamUsageDto {
        id: id.into(),
        transfer_id,
        project_id,
        usage_kind: kind,
        artifact_id,
        artifact_title: title,
        use_context: context,
        recorded_by: actor,
        recorded_at_us: time,
    })
}

fn title_for(conn: &Connection, kind: &str, version_id: &str) -> Result<String> {
    query_catalog_row(conn, kind, version_id)?
        .map(|artifact| artifact.title)
        .ok_or_else(|| anyhow!("downstream artifact version does not exist"))
}

fn query_ids(conn: &Connection, sql: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map([], |row| row.get(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

fn query_ids_for(conn: &Connection, sql: &str, value: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map([value], |row| row.get(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

fn exists(conn: &Connection, sql: &str, id: &str, label: &str) -> Result<()> {
    let found: bool = conn.query_row(sql, [id], |row| row.get(0))?;
    if !found {
        return Err(anyhow!("{label} {id} does not exist"));
    }
    Ok(())
}

fn parse_purposes(json: &str) -> Result<Vec<TransferPurpose>> {
    let values: Value = serde_json::from_str(json)?;
    let values = values
        .as_array()
        .ok_or_else(|| anyhow!("allowed purposes must be a JSON array"))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| anyhow!("allowed purpose must be a string"))
                .and_then(TransferPurpose::parse)
        })
        .collect()
}

fn parse_scope(value: &str) -> Result<TransferAccessScope> {
    match value {
        "workspace" => Ok(TransferAccessScope::Workspace),
        "selected_projects" => Ok(TransferAccessScope::SelectedProjects),
        _ => Err(anyhow!("invalid transfer access scope: {value}")),
    }
}

fn parse_eligibility_state(value: &str) -> Result<TransferEligibilityState> {
    match value {
        "eligible" => Ok(TransferEligibilityState::Eligible),
        "restricted" => Ok(TransferEligibilityState::Restricted),
        "withdrawn" => Ok(TransferEligibilityState::Withdrawn),
        _ => Err(anyhow!("invalid transfer eligibility state: {value}")),
    }
}

fn parse_transfer_state(value: &str) -> Result<KnowledgeTransferState> {
    match value {
        "proposed" => Ok(KnowledgeTransferState::Proposed),
        "accepted" => Ok(KnowledgeTransferState::Accepted),
        "rejected" => Ok(KnowledgeTransferState::Rejected),
        "withdrawn" => Ok(KnowledgeTransferState::Withdrawn),
        _ => Err(anyhow!("invalid knowledge transfer state: {value}")),
    }
}
