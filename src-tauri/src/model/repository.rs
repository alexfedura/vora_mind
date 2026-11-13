use super::*;
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const MODEL_SCHEMA: &str = include_str!("schema.sql");
const MODEL_MIGRATION_ID: &str = "phase5-model-001";

fn id() -> String {
    Uuid::now_v7().to_string()
}

fn now_us() -> Result<i64> {
    let value = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros();
    i64::try_from(value).map_err(|_| anyhow!("system timestamp exceeds the supported range"))
}

fn sha256(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

fn json<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

fn parse_json<T: for<'de> Deserialize<'de>>(value: &str) -> rusqlite::Result<T> {
    serde_json::from_str(value).map_err(|_| rusqlite::Error::InvalidQuery)
}

fn required(value: &str, name: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{name} must not be empty."));
    }
    Ok(())
}

fn valid_sha256(value: &str, name: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(anyhow!("{name} must be a 64-character SHA-256 digest."));
    }
    Ok(())
}

fn exists(conn: &Connection, sql: &str, value: &str, message: &str) -> Result<()> {
    let found: bool = conn.query_row(sql, [value], |row| row.get(0))?;
    if !found {
        return Err(anyhow!(message.to_string()));
    }
    Ok(())
}

fn ensure_project_owns(
    conn: &Connection,
    sql: &str,
    resource_id: &str,
    project_id: &str,
    message: &str,
) -> Result<()> {
    let owned: bool = conn.query_row(sql, params![resource_id, project_id], |row| row.get(0))?;
    if !owned {
        return Err(anyhow!(message.to_string()));
    }
    Ok(())
}

fn ensure_workspace_source_claim(conn: &Connection, claim_id: &str) -> Result<()> {
    exists(
        conn,
        "SELECT EXISTS(
            SELECT 1
            FROM source_claims c
            JOIN source_versions v
              ON v.id = c.source_version_id AND v.source_id = c.source_id
            JOIN knowledge_extractions e
              ON e.id = c.extraction_id
             AND e.source_id = c.source_id
             AND e.source_version_id = c.source_version_id
            WHERE c.id = ?1
         )",
        claim_id,
        "Source claim was not found.",
    )
}

fn ensure_project_writable(conn: &Connection, project_id: &str) -> Result<()> {
    let status: Option<String> = conn
        .query_row(
            "SELECT lifecycle_status FROM research_projects WHERE id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(status) = status else {
        return Err(anyhow!("Research project was not found."));
    };
    if status == "archived" {
        return Err(anyhow!("Archived research projects cannot be modified."));
    }
    Ok(())
}

fn ensure_model_identity_writable(
    conn: &Connection,
    model_id: &str,
    project_id: &str,
) -> Result<()> {
    let origin_project: Option<String> = conn.query_row(
        "SELECT origin_project_id FROM scientific_models WHERE id = ?1",
        [model_id],
        |row| row.get(0),
    )?;
    if origin_project
        .as_deref()
        .is_some_and(|origin_project| origin_project != project_id)
    {
        return Err(anyhow!(
            "A model identity can only be versioned in its origin project; use a distinct receiving-project identity for a transferred adaptation."
        ));
    }

    let has_foreign_versions: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM model_versions
            WHERE model_id = ?1 AND created_in_project_id <> ?2
         )",
        params![model_id, project_id],
        |row| row.get(0),
    )?;
    if has_foreign_versions {
        return Err(anyhow!(
            "A model identity cannot have versions created in multiple projects."
        ));
    }
    Ok(())
}

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS model_schema_migrations (
             version INTEGER PRIMARY KEY,
             migration_id TEXT NOT NULL UNIQUE,
             checksum TEXT NOT NULL,
             applied_at_us INTEGER NOT NULL
         );",
    )?;
    let enabled: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    if enabled != 1 {
        return Err(anyhow!(
            "SQLite foreign-key enforcement is required for Model writes."
        ));
    }

    conn.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| -> Result<()> {
        let checksum = sha256(MODEL_SCHEMA.as_bytes());
        let applied: Option<(String, String)> = conn
            .query_row(
                "SELECT migration_id, checksum FROM model_schema_migrations WHERE version = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((migration_id, applied_checksum)) = applied {
            if migration_id != MODEL_MIGRATION_ID || applied_checksum != checksum {
                return Err(anyhow!(
                    "Model migration 1 differs from the applied migration; add a new numbered migration."
                ));
            }
            return Ok(());
        }

        conn.execute_batch(MODEL_SCHEMA)?;
        conn.execute(
            "INSERT INTO model_schema_migrations
                (version, migration_id, checksum, applied_at_us)
             VALUES (1, ?1, ?2, ?3)",
            params![MODEL_MIGRATION_ID, checksum, now_us()?],
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

pub fn create_model(conn: &mut Connection, request: &CreateModelReq) -> Result<ModelDto> {
    required(&request.display_name, "Model display name")?;
    if let Some(project_id) = &request.origin_project_id {
        exists(
            conn,
            "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
            project_id,
            "Model origin project was not found.",
        )?;
        ensure_project_writable(conn, project_id)?;
    }
    if let Some(domain_id) = &request.domain_id {
        exists(
            conn,
            "SELECT EXISTS(SELECT 1 FROM research_domains WHERE id = ?1)",
            domain_id,
            "Research domain was not found.",
        )?;
    }
    let now = now_us()?;
    let model_id = id();
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute(
        "INSERT INTO scientific_models
            (id, display_name, description, origin_project_id, domain_id, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            model_id,
            request.display_name.trim(),
            request.description.trim(),
            request.origin_project_id,
            request.domain_id,
            request.created_by,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO model_status_events (id, model_id, status, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, 'proposed', 'Model identity created.', ?3, ?4)",
        params![id(), model_id, request.created_by, now],
    )?;
    tx.commit()?;
    query_model(conn, &model_id)
}

fn model_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ModelDto> {
    Ok(ModelDto {
        id: row.get(0)?,
        display_name: row.get(1)?,
        description: row.get(2)?,
        origin_project_id: row.get(3)?,
        domain_id: row.get(4)?,
        status: row.get(5)?,
        events: Vec::new(),
        created_by: row.get(6)?,
        created_at_us: row.get(7)?,
    })
}

pub fn query_model(conn: &Connection, model_id: &str) -> Result<ModelDto> {
    let mut model = conn.query_row(
        "SELECT m.id, m.display_name, m.description, m.origin_project_id, m.domain_id,
                COALESCE((SELECT e.status FROM model_status_events e
                          WHERE e.model_id = m.id ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1),
                         'proposed'),
                m.created_by, m.created_at_us
         FROM scientific_models m WHERE m.id = ?1",
        [model_id],
        model_from_row,
    )?;
    let mut statement = conn.prepare(
        "SELECT status, rationale, actor_id, occurred_at_us FROM model_status_events
         WHERE model_id = ?1 ORDER BY occurred_at_us, id",
    )?;
    model.events = statement
        .query_map([model_id], |row| {
            Ok(ModelLifecycleEventDto {
                status: row.get(0)?,
                rationale: row.get(1)?,
                actor_id: row.get(2)?,
                occurred_at_us: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(model)
}

pub fn transition_model(conn: &mut Connection, request: &TransitionModelReq) -> Result<ModelDto> {
    required(&request.rationale, "Model status rationale")?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let origin_project: Option<Option<String>> = tx
        .query_row(
            "SELECT origin_project_id FROM scientific_models WHERE id = ?1",
            [&request.model_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(origin_project) = origin_project else {
        return Err(anyhow!("Model identity was not found."));
    };
    if let Some(project_id) = origin_project.as_deref() {
        ensure_project_writable(&tx, project_id)?;
    }
    let current: Option<String> = tx
        .query_row(
            "SELECT status FROM model_status_events WHERE model_id = ?1
             ORDER BY occurred_at_us DESC, id DESC LIMIT 1",
            [&request.model_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(current) = current else {
        return Err(anyhow!("Model identity was not found."));
    };
    let allowed = match current.as_str() {
        "proposed" => matches!(
            request.next_status,
            ModelStatus::Active | ModelStatus::Deprecated | ModelStatus::Retired
        ),
        "active" => matches!(
            request.next_status,
            ModelStatus::Deprecated | ModelStatus::Replaced | ModelStatus::Retired
        ),
        "deprecated" => matches!(
            request.next_status,
            ModelStatus::Active | ModelStatus::Replaced | ModelStatus::Retired
        ),
        "replaced" => request.next_status == ModelStatus::Retired,
        "retired" => false,
        _ => return Err(anyhow!("Stored model status is invalid.")),
    };
    if !allowed {
        return Err(anyhow!(
            "Invalid model status transition from {current} to {}.",
            request.next_status.as_str()
        ));
    }
    tx.execute(
        "INSERT INTO model_status_events
            (id, model_id, status, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id(),
            request.model_id,
            request.next_status.as_str(),
            request.rationale.trim(),
            request.actor_id,
            now_us()?
        ],
    )?;
    tx.commit()?;
    query_model(conn, &request.model_id)
}

pub fn transition_version(
    conn: &mut Connection,
    request: &TransitionModelVersionReq,
) -> Result<ModelVersionDto> {
    required(&request.rationale, "Model version lifecycle rationale")?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let project_id: Option<String> = tx
        .query_row(
            "SELECT created_in_project_id FROM model_versions WHERE id = ?1",
            [&request.version_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(project_id) = project_id else {
        return Err(anyhow!("Model version was not found."));
    };
    ensure_project_writable(&tx, &project_id)?;
    let current: Option<String> = tx
        .query_row(
            "SELECT lifecycle_status FROM model_version_events
             WHERE model_version_id = ?1 ORDER BY occurred_at_us DESC, id DESC LIMIT 1",
            [&request.version_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(current) = current else {
        return Err(anyhow!("Model version was not found."));
    };
    let allowed = match current.as_str() {
        "draft" => matches!(
            request.next_status,
            ModelVersionLifecycleStatus::Review | ModelVersionLifecycleStatus::Withdrawn
        ),
        "review" => matches!(
            request.next_status,
            ModelVersionLifecycleStatus::Draft
                | ModelVersionLifecycleStatus::Released
                | ModelVersionLifecycleStatus::Withdrawn
        ),
        "released" => matches!(
            request.next_status,
            ModelVersionLifecycleStatus::Superseded | ModelVersionLifecycleStatus::Withdrawn
        ),
        "superseded" => request.next_status == ModelVersionLifecycleStatus::Withdrawn,
        "withdrawn" => false,
        _ => return Err(anyhow!("Stored model-version status is invalid.")),
    };
    if !allowed {
        return Err(anyhow!(
            "Invalid model-version transition from {current} to {}.",
            request.next_status.as_str()
        ));
    }
    tx.execute(
        "INSERT INTO model_version_events
            (id, model_version_id, lifecycle_status, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id(),
            request.version_id,
            request.next_status.as_str(),
            request.rationale.trim(),
            request.actor_id,
            now_us()?
        ],
    )?;
    tx.commit()?;
    query_version(conn, &request.version_id)
}

pub fn create_model_version(
    conn: &mut Connection,
    request: &CreateModelVersionReq,
) -> Result<ModelVersionDto> {
    let model_version_id = id();
    let created_at = now_us()?;
    let serialized = serde_json::to_vec(request)?;
    let content_sha256 = sha256(&serialized);

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    validate_version_request(&tx, request)?;
    let version_no: i64 = tx.query_row(
        "SELECT COALESCE(MAX(version_no), 0) + 1 FROM model_versions WHERE model_id = ?1",
        [&request.model_id],
        |row| row.get(0),
    )?;
    tx.execute(
        "INSERT INTO model_versions
            (id, model_id, version_no, created_in_project_id, change_class,
             change_rationale, expected_implications, human_specification,
             machine_specification_json, scope, dependencies_json, environment_json,
             content_sha256, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            model_version_id,
            request.model_id,
            version_no,
            request.project_id,
            request.change_class.as_str(),
            request.change_rationale.trim(),
            request.expected_implications.trim(),
            request.specification.human_readable.trim(),
            json(&request.specification.machine_readable)?,
            request.scope.trim(),
            json(&request.dependencies)?,
            json(&request.environment)?,
            content_sha256,
            request.created_by,
            created_at
        ],
    )?;
    tx.execute(
        "INSERT INTO model_version_events
            (id, model_version_id, lifecycle_status, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, 'draft', 'Immutable version created.', ?3, ?4)",
        params![id(), model_version_id, request.created_by, created_at],
    )?;

    for parent in &request.parents {
        tx.execute(
            "INSERT INTO model_version_parents (version_id, model_id, parent_version_id, rationale)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                model_version_id,
                request.model_id,
                parent.version_id,
                parent.rationale.trim()
            ],
        )?;
    }
    for assumption in &request.assumptions {
        tx.execute(
            "INSERT INTO model_version_assumptions (id, model_version_id, assumption_key, statement)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                id(),
                model_version_id,
                assumption.key.trim(),
                assumption.statement.trim()
            ],
        )?;
    }
    for parameter in &request.parameters {
        tx.execute(
            "INSERT INTO model_version_parameters
                (id, model_version_id, name, value, value_kind, unit, lower_bound,
                 upper_bound, default_value, role, tuneable)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id(),
                model_version_id,
                parameter.name.trim(),
                parameter.value,
                parameter.value_kind,
                parameter.unit,
                parameter.lower_bound,
                parameter.upper_bound,
                parameter.default_value,
                parameter.role.trim(),
                i64::from(parameter.tuneable)
            ],
        )?;
    }
    for dependency in &request.dependencies {
        tx.execute(
            "INSERT INTO model_version_dependencies
                (id, model_version_id, name, version, source_uri, checksum_sha256)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id(),
                model_version_id,
                dependency.name.trim(),
                dependency.version.trim(),
                dependency.source_uri,
                dependency.checksum_sha256
            ],
        )?;
    }
    for artifact in &request.artifacts {
        tx.execute(
            "INSERT INTO model_version_artifacts
                (id, model_version_id, artifact_kind, title, artifact_uri,
                 checksum_sha256, media_type, byte_length)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id(),
                model_version_id,
                artifact.artifact_kind.trim(),
                artifact.title.trim(),
                artifact.artifact_uri.trim(),
                artifact.checksum_sha256.to_ascii_lowercase(),
                artifact.media_type,
                artifact.byte_length
            ],
        )?;
    }
    for impact in &request.impacts {
        insert_impact(&tx, &model_version_id, impact)?;
    }
    for relation in &request.hypotheses {
        if !matches!(
            relation.relation_kind,
            ModelRelationKind::ContributesTo | ModelRelationKind::Contradicts
        ) {
            return Err(anyhow!(
                "Hypothesis relations must contribute to or contradict the model."
            ));
        }
        tx.execute(
            "INSERT INTO model_contributions
                (id, model_version_id, relation_kind, hypothesis_version_id, rationale,
                 created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id(),
                model_version_id,
                relation.relation_kind.as_str(),
                relation.target_id,
                relation.rationale.trim(),
                request.created_by,
                created_at
            ],
        )?;
    }
    for relation in &request.findings {
        if relation.relation_kind == ModelRelationKind::TestedIn {
            return Err(anyhow!("A finding cannot use the tested_in relation."));
        }
        tx.execute(
            "INSERT INTO model_contributions
                (id, model_version_id, relation_kind, finding_version_id, rationale,
                 created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id(),
                model_version_id,
                relation.relation_kind.as_str(),
                relation.target_id,
                relation.rationale.trim(),
                request.created_by,
                created_at
            ],
        )?;
    }
    for relation in &request.experiments {
        if !matches!(
            relation.relation_kind.as_str(),
            "contributes_to" | "tested_in"
        ) {
            return Err(anyhow!(
                "Experiment relations must be contributes_to or tested_in."
            ));
        }
        if relation.relation_kind == "tested_in" && relation.experiment_run_id.is_none() {
            return Err(anyhow!(
                "A tested_in relation requires an exact experiment run."
            ));
        }
        if let Some(run_id) = &relation.experiment_run_id {
            let run_design: Option<String> = tx
                .query_row(
                    "SELECT design_version_id FROM experiment_runs WHERE id = ?1",
                    [run_id],
                    |row| row.get(0),
                )
                .optional()?;
            if run_design.as_deref() != Some(relation.design_version_id.as_str()) {
                return Err(anyhow!(
                    "Experiment run does not use the referenced design version."
                ));
            }
        }
        tx.execute(
            "INSERT INTO model_contributions
                (id, model_version_id, design_version_id, experiment_run_id, relation_kind,
                 rationale, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id(),
                model_version_id,
                relation.design_version_id,
                relation.experiment_run_id,
                relation.relation_kind,
                relation.rationale.trim(),
                request.created_by,
                created_at
            ],
        )?;
    }
    for use_record in &request.dataset_uses {
        validate_dataset_role(&use_record.role)?;
        required(&use_record.rationale, "Dataset-use rationale")?;
        ensure_project_owns(
            &tx,
            "SELECT EXISTS(
                SELECT 1 FROM experiment_dataset_versions v
                JOIN experiment_datasets d ON d.id = v.dataset_id
                WHERE v.id = ?1 AND d.project_id = ?2
             )",
            &use_record.dataset_version_id,
            &request.project_id,
            "Model dataset references must belong to the requesting project.",
        )?;
        let sealed: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM experiment_dataset_versions
             WHERE id = ?1 AND lifecycle_status = 'sealed')",
            [&use_record.dataset_version_id],
            |row| row.get(0),
        )?;
        if !sealed {
            return Err(anyhow!(
                "Model dataset references must pin a sealed version."
            ));
        }
        tx.execute(
            "INSERT INTO model_contributions
                (id, model_version_id, relation_kind, dataset_version_id, dataset_role,
                 split_group_id, rationale,
                 created_by, created_at_us)
             VALUES (?1, ?2, 'dataset_use', ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id(),
                model_version_id,
                use_record.dataset_version_id,
                use_record.role,
                use_record.split_group_id,
                use_record.rationale.trim(),
                request.created_by,
                created_at
            ],
        )?;
    }
    for evidence_id in &request.evidence_ids {
        let target_exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM model_evidence WHERE id = ?1)",
            [evidence_id],
            |row| row.get(0),
        )?;
        if !target_exists {
            return Err(anyhow!("Model evidence reference was not found."));
        }
        ensure_project_owns(
            &tx,
            "SELECT EXISTS(
                SELECT 1 FROM model_evidence e
                JOIN model_versions v ON v.id = e.model_version_id
                WHERE e.id = ?1 AND v.created_in_project_id = ?2
             )",
            evidence_id,
            &request.project_id,
            "Model evidence references must belong to the requesting project.",
        )?;
        tx.execute(
            "INSERT INTO model_contributions
                (id, model_version_id, relation_kind, evidence_id, rationale, created_by, created_at_us)
             VALUES (?1, ?2, 'evidenced_by', ?3,
                 'Referenced during immutable model-version creation.', ?4, ?5)",
            params![id(), model_version_id, evidence_id, request.created_by, created_at],
        )?;
    }

    tx.commit()?;
    query_version(conn, &model_version_id)
}

fn ensure_parent_version_ownership(
    conn: &Connection,
    parent_version_id: &str,
    model_id: &str,
    project_id: &str,
) -> Result<()> {
    let parent: Option<(String, String)> = conn
        .query_row(
            "SELECT model_id, created_in_project_id FROM model_versions WHERE id = ?1",
            [parent_version_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let parent_model_id = parent
        .as_ref()
        .map(|(parent_model_id, _)| parent_model_id.as_str());
    if parent_model_id != Some(model_id) {
        return Err(anyhow!(
            "Parent versions must belong to the same model identity."
        ));
    }
    let parent_project_id = parent
        .as_ref()
        .map(|(_, parent_project_id)| parent_project_id.as_str());
    if parent_project_id != Some(project_id) {
        return Err(anyhow!(
            "Parent versions must belong to the requesting project."
        ));
    }
    Ok(())
}

fn validate_version_request(conn: &Connection, request: &CreateModelVersionReq) -> Result<()> {
    exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
        &request.project_id,
        "Research project was not found.",
    )?;
    ensure_project_writable(conn, &request.project_id)?;
    exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM scientific_models WHERE id = ?1)",
        &request.model_id,
        "Model identity was not found.",
    )?;
    ensure_model_identity_writable(conn, &request.model_id, &request.project_id)?;
    for (value, label) in [
        (&request.change_rationale, "Change rationale"),
        (&request.expected_implications, "Expected implications"),
        (
            &request.specification.human_readable,
            "Human-readable model specification",
        ),
        (&request.scope, "Model scope"),
    ] {
        required(value, label)?;
    }
    if !request.specification.machine_readable.is_object() {
        return Err(anyhow!(
            "Machine-readable model specification must be a JSON object."
        ));
    }
    if !request.environment.is_object() {
        return Err(anyhow!("Model environment identity must be a JSON object."));
    }
    if request.impacts.is_empty() {
        return Err(anyhow!(
            "A model version requires at least one affected claim or assumption."
        ));
    }
    let existing_versions: i64 = conn.query_row(
        "SELECT COUNT(*) FROM model_versions WHERE model_id = ?1",
        [&request.model_id],
        |row| row.get(0),
    )?;
    if existing_versions == 0 {
        if !request.parents.is_empty() || request.change_class != ModelChangeClass::Revision {
            return Err(anyhow!(
                "The first version is a parentless REVISION baseline."
            ));
        }
    } else if request.parents.is_empty() {
        return Err(anyhow!(
            "A successor model version must reference at least one parent version."
        ));
    }
    if request.change_class == ModelChangeClass::Replacement && request.parents.is_empty() {
        return Err(anyhow!(
            "A replacement must preserve and reference its predecessor version."
        ));
    }
    for parent in &request.parents {
        required(&parent.rationale, "Parent-version rationale")?;
        ensure_parent_version_ownership(
            conn,
            &parent.version_id,
            &request.model_id,
            &request.project_id,
        )?;
    }
    for assumption in &request.assumptions {
        required(&assumption.key, "Assumption key")?;
        required(&assumption.statement, "Assumption statement")?;
    }
    for impact in &request.impacts {
        required(&impact.rationale, "Impact rationale")?;
        match impact.impact_kind {
            ModelImpactKind::Added
                if impact
                    .current_statement
                    .as_deref()
                    .is_none_or(str::is_empty) =>
            {
                return Err(anyhow!("An added impact requires its new statement."));
            }
            ModelImpactKind::Changed
                if impact
                    .previous_statement
                    .as_deref()
                    .is_none_or(str::is_empty)
                    || impact
                        .current_statement
                        .as_deref()
                        .is_none_or(str::is_empty) =>
            {
                return Err(anyhow!(
                    "A changed impact requires both previous and current statements."
                ));
            }
            ModelImpactKind::Removed
                if impact
                    .previous_statement
                    .as_deref()
                    .is_none_or(str::is_empty) =>
            {
                return Err(anyhow!("A removed impact requires its prior statement."));
            }
            _ => {}
        }
        match &impact.target {
            ModelImpactTarget::Assumption { assumption_key } => {
                if !request
                    .assumptions
                    .iter()
                    .any(|assumption| assumption.key == *assumption_key)
                {
                    return Err(anyhow!(
                        "Impacted assumptions must be included in this immutable version."
                    ));
                }
            }
            ModelImpactTarget::SourceClaim { claim_id } => {
                ensure_workspace_source_claim(conn, claim_id)?
            }
            ModelImpactTarget::HypothesisVersion {
                hypothesis_version_id,
            } => ensure_project_owns(
                conn,
                "SELECT EXISTS(
                    SELECT 1 FROM hypothesis_definition_versions v
                    JOIN research_hypotheses h ON h.id = v.hypothesis_id
                    WHERE v.id = ?1 AND h.project_id = ?2
                 )",
                hypothesis_version_id,
                &request.project_id,
                "Affected hypothesis version must belong to the requesting project.",
            )?,
            ModelImpactTarget::FindingVersion { finding_version_id } => ensure_project_owns(
                conn,
                "SELECT EXISTS(
                    SELECT 1 FROM analysis_finding_versions v
                    JOIN analysis_findings f ON f.id = v.finding_id
                    WHERE v.id = ?1 AND f.project_id = ?2
                 )",
                finding_version_id,
                &request.project_id,
                "Affected finding version must belong to the requesting project.",
            )?,
        }
    }
    for parameter in &request.parameters {
        required(&parameter.name, "Parameter name")?;
        required(&parameter.value, "Parameter value")?;
        required(&parameter.role, "Parameter role")?;
        if !matches!(
            parameter.value_kind.as_str(),
            "decimal" | "integer" | "boolean" | "string" | "json"
        ) {
            return Err(anyhow!("Unsupported model parameter value kind."));
        }
    }
    for dependency in &request.dependencies {
        required(&dependency.name, "Dependency name")?;
        required(&dependency.version, "Dependency version")?;
        if let Some(checksum) = &dependency.checksum_sha256 {
            valid_sha256(checksum, "Dependency checksum")?;
        }
    }
    for artifact in &request.artifacts {
        required(&artifact.artifact_kind, "Artifact kind")?;
        required(&artifact.title, "Artifact title")?;
        required(&artifact.artifact_uri, "Artifact URI")?;
        valid_sha256(&artifact.checksum_sha256, "Artifact checksum")?;
        if artifact.byte_length.is_some_and(|length| length < 0) {
            return Err(anyhow!("Artifact byte length cannot be negative."));
        }
    }
    for relation in request.hypotheses.iter().chain(request.findings.iter()) {
        required(&relation.rationale, "Model relation rationale")?;
    }
    for relation in &request.hypotheses {
        ensure_project_owns(
            conn,
            "SELECT EXISTS(
                SELECT 1 FROM hypothesis_definition_versions v
                JOIN research_hypotheses h ON h.id = v.hypothesis_id
                WHERE v.id = ?1 AND h.project_id = ?2
             )",
            &relation.target_id,
            &request.project_id,
            "Hypothesis relation must target a version in the requesting project.",
        )?;
    }
    for relation in &request.findings {
        ensure_project_owns(
            conn,
            "SELECT EXISTS(
                SELECT 1 FROM analysis_finding_versions v
                JOIN analysis_findings f ON f.id = v.finding_id
                WHERE v.id = ?1 AND f.project_id = ?2
             )",
            &relation.target_id,
            &request.project_id,
            "Finding relation must target a version in the requesting project.",
        )?;
    }
    for relation in &request.experiments {
        required(&relation.rationale, "Experiment relation rationale")?;
        ensure_project_owns(
            conn,
            "SELECT EXISTS(
                SELECT 1 FROM research_experiment_design_versions v
                JOIN research_experiment_designs d ON d.id = v.design_id
                WHERE v.id = ?1 AND d.project_id = ?2
             )",
            &relation.design_version_id,
            &request.project_id,
            "Experiment design must belong to the requesting project.",
        )?;
        if let Some(run_id) = &relation.experiment_run_id {
            ensure_project_owns(
                conn,
                "SELECT EXISTS(
                    SELECT 1 FROM experiment_runs WHERE id = ?1 AND project_id = ?2
                 )",
                run_id,
                &request.project_id,
                "Experiment run must belong to the requesting project.",
            )?;
        }
    }
    Ok(())
}

fn insert_impact(tx: &Transaction<'_>, version_id: &str, impact: &ModelImpactReq) -> Result<()> {
    let (kind, source_claim_id, hypothesis_version_id, finding_version_id, assumption_key) =
        match &impact.target {
            ModelImpactTarget::SourceClaim { claim_id } => {
                ("claim", Some(claim_id.as_str()), None, None, None)
            }
            ModelImpactTarget::HypothesisVersion {
                hypothesis_version_id,
            } => (
                "claim",
                None,
                Some(hypothesis_version_id.as_str()),
                None,
                None,
            ),
            ModelImpactTarget::FindingVersion { finding_version_id } => {
                ("claim", None, None, Some(finding_version_id.as_str()), None)
            }
            ModelImpactTarget::Assumption { assumption_key } => (
                "assumption",
                None,
                None,
                None,
                Some(assumption_key.as_str()),
            ),
        };
    tx.execute(
        "INSERT INTO model_version_impacts
            (id, model_version_id, item_kind, impact_kind, source_claim_id,
             hypothesis_version_id, finding_version_id, assumption_key,
             previous_statement, current_statement, rationale)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id(),
            version_id,
            kind,
            impact.impact_kind.as_str(),
            source_claim_id,
            hypothesis_version_id,
            finding_version_id,
            assumption_key,
            impact.previous_statement,
            impact.current_statement,
            impact.rationale.trim()
        ],
    )?;
    Ok(())
}

fn validate_dataset_role(role: &str) -> Result<()> {
    if !matches!(
        role,
        "training" | "validation" | "test" | "exploratory" | "prospective" | "forward"
    ) {
        return Err(anyhow!("Unsupported dataset role: {role}"));
    }
    Ok(())
}

pub fn query_version(conn: &Connection, version_id: &str) -> Result<ModelVersionDto> {
    let row = conn.query_row(
        "SELECT id, model_id, version_no, created_in_project_id, change_class,
                change_rationale, expected_implications, human_specification,
                machine_specification_json, scope, dependencies_json,
                environment_json, content_sha256, created_by, created_at_us
         FROM model_versions WHERE id = ?1",
        [version_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, i64>(14)?,
            ))
        },
    )?;
    let mut statement = conn.prepare(
        "SELECT parent_version_id, rationale FROM model_version_parents
         WHERE version_id = ?1 ORDER BY parent_version_id",
    )?;
    let parents = statement
        .query_map([version_id], |row| {
            Ok(ModelParentReq {
                version_id: row.get(0)?,
                rationale: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut statement = conn.prepare(
        "SELECT assumption_key, statement FROM model_version_assumptions
         WHERE model_version_id = ?1 ORDER BY assumption_key",
    )?;
    let assumptions = statement
        .query_map([version_id], |row| {
            Ok(ModelAssumptionReq {
                key: row.get(0)?,
                statement: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut statement = conn.prepare(
        "SELECT name, value, value_kind, unit, lower_bound, upper_bound, default_value, role, tuneable
         FROM model_version_parameters WHERE model_version_id = ?1 ORDER BY name",
    )?;
    let parameters = statement
        .query_map([version_id], |row| {
            Ok(ModelParameterDto {
                name: row.get(0)?,
                value: row.get(1)?,
                value_kind: row.get(2)?,
                unit: row.get(3)?,
                lower_bound: row.get(4)?,
                upper_bound: row.get(5)?,
                default_value: row.get(6)?,
                role: row.get(7)?,
                tuneable: row.get::<_, i64>(8)? != 0,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut statement = conn.prepare(
        "SELECT id, artifact_kind, title, artifact_uri, checksum_sha256, media_type, byte_length
         FROM model_version_artifacts WHERE model_version_id = ?1 ORDER BY id",
    )?;
    let artifacts = statement
        .query_map([version_id], |row| {
            Ok(ModelArtifactDto {
                id: row.get(0)?,
                artifact_kind: row.get(1)?,
                title: row.get(2)?,
                artifact_uri: row.get(3)?,
                checksum_sha256: row.get(4)?,
                media_type: row.get(5)?,
                byte_length: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let lifecycle_status: Option<String> = conn
        .query_row(
            "SELECT lifecycle_status FROM model_version_events WHERE model_version_id = ?1
             ORDER BY occurred_at_us DESC, id DESC LIMIT 1",
            [version_id],
            |row| row.get(0),
        )
        .optional()?;
    let lifecycle_status = parse_model_version_status(
        lifecycle_status
            .as_deref()
            .ok_or_else(|| anyhow!("Model version lifecycle state was not found."))?,
    )?;
    let mut statement = conn.prepare(
        "SELECT lifecycle_status, rationale, actor_id, occurred_at_us
         FROM model_version_events WHERE model_version_id = ?1 ORDER BY occurred_at_us, id",
    )?;
    let lifecycle_events = statement
        .query_map([version_id], |row| {
            Ok(ModelLifecycleEventDto {
                status: row.get(0)?,
                rationale: row.get(1)?,
                actor_id: row.get(2)?,
                occurred_at_us: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let dependencies = parse_json(&row.10)?;
    let machine_readable = conn.query_row(
        "SELECT machine_specification_json FROM model_versions WHERE id = ?1",
        [version_id],
        |row| parse_json(&row.get::<_, String>(0)?),
    )?;
    let contributions = query_contributions(conn, version_id)?;
    let hypotheses = contributions
        .iter()
        .filter_map(|contribution| match &contribution.target {
            ModelContributionTarget::HypothesisVersion { id } => Some(ModelRelationReq {
                target_id: id.clone(),
                relation_kind: ModelRelationKind::parse(contribution.relation_kind.as_str())
                    .ok()?,
                rationale: contribution.rationale.clone(),
            }),
            _ => None,
        })
        .collect();
    let findings = contributions
        .iter()
        .filter_map(|contribution| match &contribution.target {
            ModelContributionTarget::FindingVersion { id } => Some(ModelRelationReq {
                target_id: id.clone(),
                relation_kind: ModelRelationKind::parse(contribution.relation_kind.as_str())
                    .ok()?,
                rationale: contribution.rationale.clone(),
            }),
            _ => None,
        })
        .collect();
    let experiments = contributions
        .iter()
        .filter_map(|contribution| match &contribution.target {
            ModelContributionTarget::Experiment {
                design_version_id,
                experiment_run_id,
            } => Some(ModelExperimentRelationReq {
                design_version_id: design_version_id.clone(),
                experiment_run_id: experiment_run_id.clone(),
                relation_kind: contribution.relation_kind.as_str().to_string(),
                rationale: contribution.rationale.clone(),
            }),
            _ => None,
        })
        .collect();
    let dataset_uses = contributions
        .iter()
        .filter_map(|contribution| match &contribution.target {
            ModelContributionTarget::Dataset {
                dataset_version_id,
                role,
                split_group_id,
            } => Some(ModelDatasetUseReq {
                dataset_version_id: dataset_version_id.clone(),
                role: role.clone(),
                split_group_id: split_group_id.clone(),
                rationale: contribution.rationale.clone(),
            }),
            _ => None,
        })
        .collect();
    let evidence_ids = contributions
        .iter()
        .filter_map(|contribution| match &contribution.target {
            ModelContributionTarget::Evidence { id } => Some(id.clone()),
            _ => None,
        })
        .collect();
    Ok(ModelVersionDto {
        id: row.0,
        model_id: row.1,
        version_no: row.2,
        created_in_project_id: row.3,
        change_class: ModelChangeClass::parse(&row.4).map_err(|error| anyhow!(error))?,
        change_rationale: row.5,
        expected_implications: row.6,
        specification: ModelSpecificationDto {
            human_readable: row.7,
            machine_readable,
        },
        assumptions,
        scope: row.9,
        dependencies,
        environment: parse_json(&row.11)?,
        parents,
        parameters,
        artifacts,
        impacts: query_impacts(conn, version_id)?,
        contributions,
        hypotheses,
        findings,
        experiments,
        dataset_uses,
        evidence_ids,
        content_sha256: row.12,
        created_by: row.13,
        created_at_us: row.14,
        lifecycle_status,
        lifecycle_events,
    })
}

fn query_contributions(conn: &Connection, version_id: &str) -> Result<Vec<ModelContributionDto>> {
    let mut statement = conn.prepare(
        "SELECT id, relation_kind, hypothesis_version_id, finding_version_id,
                design_version_id, experiment_run_id, dataset_version_id, dataset_role,
                split_group_id, evidence_id, rationale, created_by, created_at_us
         FROM model_contributions WHERE model_version_id = ?1
         ORDER BY created_at_us, id",
    )?;
    let rows = statement
        .query_map([version_id], |row| {
            let relation_kind = ModelContributionKind::parse(&row.get::<_, String>(1)?)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let hypothesis_id: Option<String> = row.get(2)?;
            let finding_id: Option<String> = row.get(3)?;
            let design_id: Option<String> = row.get(4)?;
            let dataset_id: Option<String> = row.get(6)?;
            let evidence_id: Option<String> = row.get(9)?;
            let target = if let Some(id) = hypothesis_id {
                ModelContributionTarget::HypothesisVersion { id }
            } else if let Some(id) = finding_id {
                ModelContributionTarget::FindingVersion { id }
            } else if let Some(design_version_id) = design_id {
                ModelContributionTarget::Experiment {
                    design_version_id,
                    experiment_run_id: row.get(5)?,
                }
            } else if let Some(dataset_version_id) = dataset_id {
                ModelContributionTarget::Dataset {
                    dataset_version_id,
                    role: row
                        .get::<_, Option<String>>(7)?
                        .ok_or(rusqlite::Error::InvalidQuery)?,
                    split_group_id: row.get(8)?,
                }
            } else if let Some(id) = evidence_id {
                ModelContributionTarget::Evidence { id }
            } else {
                return Err(rusqlite::Error::InvalidQuery);
            };
            Ok(ModelContributionDto {
                id: row.get(0)?,
                relation_kind,
                target,
                rationale: row.get(10)?,
                created_by: row.get(11)?,
                created_at_us: row.get(12)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn parse_model_version_status(value: &str) -> Result<ModelVersionLifecycleStatus> {
    match value {
        "draft" => Ok(ModelVersionLifecycleStatus::Draft),
        "review" => Ok(ModelVersionLifecycleStatus::Review),
        "released" => Ok(ModelVersionLifecycleStatus::Released),
        "superseded" => Ok(ModelVersionLifecycleStatus::Superseded),
        "withdrawn" => Ok(ModelVersionLifecycleStatus::Withdrawn),
        _ => Err(anyhow!("Invalid model-version status: {value}")),
    }
}

fn query_impacts(conn: &Connection, version_id: &str) -> Result<Vec<ModelImpactReq>> {
    let mut statement = conn.prepare(
        "SELECT item_kind, impact_kind, source_claim_id, hypothesis_version_id,
                finding_version_id, assumption_key, previous_statement, current_statement, rationale
         FROM model_version_impacts WHERE model_version_id = ?1 ORDER BY id",
    )?;
    let rows = statement
        .query_map([version_id], |row| {
            let item_kind: String = row.get(0)?;
            let target = match item_kind.as_str() {
                "claim" => {
                    let source_claim: Option<String> = row.get(2)?;
                    let hypothesis: Option<String> = row.get(3)?;
                    let finding: Option<String> = row.get(4)?;
                    match (source_claim, hypothesis, finding) {
                        (Some(claim_id), None, None) => ModelImpactTarget::SourceClaim { claim_id },
                        (None, Some(hypothesis_version_id), None) => {
                            ModelImpactTarget::HypothesisVersion {
                                hypothesis_version_id,
                            }
                        }
                        (None, None, Some(finding_version_id)) => {
                            ModelImpactTarget::FindingVersion { finding_version_id }
                        }
                        _ => return Err(rusqlite::Error::InvalidQuery),
                    }
                }
                "assumption" => ModelImpactTarget::Assumption {
                    assumption_key: row
                        .get::<_, Option<String>>(5)?
                        .ok_or(rusqlite::Error::InvalidQuery)?,
                },
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            let impact_kind = match row.get::<_, String>(1)?.as_str() {
                "added" => ModelImpactKind::Added,
                "changed" => ModelImpactKind::Changed,
                "removed" => ModelImpactKind::Removed,
                "affected" => ModelImpactKind::Affected,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            Ok(ModelImpactReq {
                target,
                impact_kind,
                previous_statement: row.get(6)?,
                current_statement: row.get(7)?,
                rationale: row.get(8)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn project_model_ids(conn: &Connection, project_id: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT DISTINCT m.id
         FROM scientific_models m
         LEFT JOIN model_versions v ON v.model_id = m.id
         WHERE m.origin_project_id = ?1
            OR v.created_in_project_id = ?1
         ORDER BY m.display_name, m.id",
    )?;
    let ids = statement
        .query_map([project_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ids)
}

fn query_all_models(conn: &Connection, project_id: &str) -> Result<Vec<ModelDto>> {
    let ids = project_model_ids(conn, project_id)?;
    ids.iter()
        .map(|model_id| query_model(conn, model_id))
        .collect()
}

fn query_all_versions(conn: &Connection, project_id: &str) -> Result<Vec<ModelVersionDto>> {
    let model_ids = project_model_ids(conn, project_id)?;
    if model_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = model_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT id FROM model_versions WHERE model_id IN ({placeholders}) AND created_in_project_id = ? ORDER BY model_id, version_no"
    );
    let mut params: Vec<&str> = model_ids.iter().map(String::as_str).collect();
    params.push(project_id);
    let mut statement = conn.prepare(&sql)?;
    let ids = statement
        .query_map(rusqlite::params_from_iter(params.iter().copied()), |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.iter()
        .map(|version_id| query_version(conn, version_id))
        .collect()
}

pub fn create_evidence(
    conn: &mut Connection,
    request: &CreateModelEvidenceReq,
) -> Result<ModelEvidenceDto> {
    required(&request.scope, "Evidence scope")?;
    required(&request.quality_rationale, "Evidence quality rationale")?;
    required(&request.limitations, "Evidence limitations")?;
    exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
        &request.project_id,
        "Evidence project was not found.",
    )?;
    ensure_project_writable(conn, &request.project_id)?;
    let created_in_project: Option<String> = conn
        .query_row(
            "SELECT created_in_project_id FROM model_versions WHERE id = ?1",
            [&request.model_version_id],
            |row| row.get(0),
        )
        .optional()?;
    if created_in_project.as_deref() != Some(request.project_id.as_str()) {
        return Err(anyhow!(
            "Evidence must target a model version created in the requesting project."
        ));
    }
    match &request.artifact {
        ModelArtifactRef::AnalysisOutput(output_id) => ensure_project_owns(
            conn,
            "SELECT EXISTS(
                SELECT 1 FROM analysis_outputs o
                JOIN analysis_runs r ON r.id = o.run_id
                WHERE o.id = ?1 AND r.project_id = ?2
             )",
            output_id,
            &request.project_id,
            "Evidence output must belong to the requesting project.",
        )?,
        ModelArtifactRef::ExperimentRun(run_id) => ensure_project_owns(
            conn,
            "SELECT EXISTS(
                SELECT 1 FROM experiment_runs WHERE id = ?1 AND project_id = ?2
             )",
            run_id,
            &request.project_id,
            "Evidence experiment run must belong to the requesting project.",
        )?,
        ModelArtifactRef::DatasetVersion(dataset_version_id) => ensure_project_owns(
            conn,
            "SELECT EXISTS(
                SELECT 1 FROM experiment_dataset_versions v
                JOIN experiment_datasets d ON d.id = v.dataset_id
                WHERE v.id = ?1 AND d.project_id = ?2
             )",
            dataset_version_id,
            &request.project_id,
            "Evidence dataset must belong to the requesting project.",
        )?,
        ModelArtifactRef::ResearchObservation(observation_id) => ensure_project_owns(
            conn,
            "SELECT EXISTS(
                SELECT 1 FROM research_observations
                WHERE id = ?1 AND project_id = ?2
             )",
            observation_id,
            &request.project_id,
            "Evidence observation must belong to the requesting project.",
        )?,
        ModelArtifactRef::FindingVersion(finding_version_id) => ensure_project_owns(
            conn,
            "SELECT EXISTS(
                SELECT 1 FROM analysis_finding_versions v
                JOIN analysis_findings f ON f.id = v.finding_id
                WHERE v.id = ?1 AND f.project_id = ?2
             )",
            finding_version_id,
            &request.project_id,
            "Evidence finding must belong to the requesting project.",
        )?,
        ModelArtifactRef::SourceVersionSegment(_) | ModelArtifactRef::SourceClaim(_) => {}
    }
    let evidence_id = id();
    let created_at = now_us()?;
    let artifact_columns = match &request.artifact {
        ModelArtifactRef::AnalysisOutput(value) => {
            (Some(value.as_str()), None, None, None, None, None, None)
        }
        ModelArtifactRef::ExperimentRun(value) => {
            (None, Some(value.as_str()), None, None, None, None, None)
        }
        ModelArtifactRef::DatasetVersion(value) => {
            (None, None, Some(value.as_str()), None, None, None, None)
        }
        ModelArtifactRef::ResearchObservation(value) => {
            (None, None, None, Some(value.as_str()), None, None, None)
        }
        ModelArtifactRef::SourceVersionSegment(value) => {
            (None, None, None, None, Some(value.as_str()), None, None)
        }
        ModelArtifactRef::SourceClaim(value) => {
            ensure_workspace_source_claim(conn, value)?;
            (None, None, None, None, None, Some(value.as_str()), None)
        }
        ModelArtifactRef::FindingVersion(value) => {
            (None, None, None, None, None, None, Some(value.as_str()))
        }
    };
    conn.execute(
        "INSERT INTO model_evidence
            (id, model_version_id, artifact_analysis_output_id, artifact_experiment_run_id,
             artifact_dataset_version_id, artifact_observation_id, artifact_source_segment_id,
             artifact_source_claim_id, artifact_finding_version_id, polarity, scope,
             quality_rationale, limitations, provenance_json, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            evidence_id,
            request.model_version_id,
            artifact_columns.0,
            artifact_columns.1,
            artifact_columns.2,
            artifact_columns.3,
            artifact_columns.4,
            artifact_columns.5,
            artifact_columns.6,
            request.polarity.as_str(),
            request.scope.trim(),
            request.quality_rationale.trim(),
            request.limitations.trim(),
            json(&request.provenance)?,
            request.created_by,
            created_at
        ],
    )?;
    query_evidence(conn, &evidence_id)
}

fn query_evidence(conn: &Connection, evidence_id: &str) -> Result<ModelEvidenceDto> {
    let values = conn.query_row(
        "SELECT id, model_version_id, artifact_analysis_output_id, artifact_experiment_run_id,
                artifact_dataset_version_id, artifact_observation_id, artifact_source_segment_id,
                artifact_source_claim_id, artifact_finding_version_id, polarity, scope,
                quality_rationale, limitations, provenance_json, created_by, created_at_us
         FROM model_evidence WHERE id = ?1",
        [evidence_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, i64>(15)?,
            ))
        },
    )?;
    let artifact = match (
        values.2, values.3, values.4, values.5, values.6, values.7, values.8,
    ) {
        (Some(value), None, None, None, None, None, None) => {
            ModelArtifactRef::AnalysisOutput(value)
        }
        (None, Some(value), None, None, None, None, None) => ModelArtifactRef::ExperimentRun(value),
        (None, None, Some(value), None, None, None, None) => {
            ModelArtifactRef::DatasetVersion(value)
        }
        (None, None, None, Some(value), None, None, None) => {
            ModelArtifactRef::ResearchObservation(value)
        }
        (None, None, None, None, Some(value), None, None) => {
            ModelArtifactRef::SourceVersionSegment(value)
        }
        (None, None, None, None, None, Some(value), None) => ModelArtifactRef::SourceClaim(value),
        (None, None, None, None, None, None, Some(value)) => {
            ModelArtifactRef::FindingVersion(value)
        }
        _ => return Err(anyhow!("Model evidence has an invalid artifact endpoint.")),
    };
    let polarity = match values.9.as_str() {
        "supports" => ModelEvidencePolarity::Supports,
        "contradicts" => ModelEvidencePolarity::Contradicts,
        "qualifies" => ModelEvidencePolarity::Qualifies,
        "neutral" => ModelEvidencePolarity::Neutral,
        _ => return Err(anyhow!("Model evidence has an invalid polarity.")),
    };
    Ok(ModelEvidenceDto {
        id: values.0,
        model_version_id: values.1,
        artifact,
        polarity,
        scope: values.10,
        quality_rationale: values.11,
        limitations: values.12,
        provenance: parse_json(&values.13)?,
        created_by: values.14,
        created_at_us: values.15,
    })
}

pub fn create_evaluation(
    conn: &mut Connection,
    request: &CreateModelEvaluationReq,
) -> Result<ModelEvaluationDto> {
    validate_dataset_role(&request.role)?;
    required(&request.limitations, "Evaluation limitations")?;
    exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
        &request.project_id,
        "Evaluation project was not found.",
    )?;
    ensure_project_writable(conn, &request.project_id)?;
    let created_in_project: Option<String> = conn
        .query_row(
            "SELECT created_in_project_id FROM model_versions WHERE id = ?1",
            [&request.model_version_id],
            |row| row.get(0),
        )
        .optional()?;
    if created_in_project.as_deref() != Some(request.project_id.as_str()) {
        return Err(anyhow!(
            "Evaluation model version must belong to the requesting project."
        ));
    }
    let run: Option<(String, String)> = conn
        .query_row(
            "SELECT r.project_id, e.lifecycle_status
             FROM analysis_runs r
             JOIN analysis_run_events e ON e.run_id = r.id
             WHERE r.id = ?1
             ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1",
            [&request.analysis_run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((run_project, run_status)) = run else {
        return Err(anyhow!(
            "Analysis run or its lifecycle status was not found."
        ));
    };
    if run_status != "completed" {
        return Err(anyhow!(
            "Model evaluations require a completed AnalysisRun."
        ));
    }
    if run_project != request.project_id {
        return Err(anyhow!(
            "Model evaluation project must match the AnalysisRun project."
        ));
    }
    let input: Option<(String, String, Option<String>)> = conn
        .query_row(
            "SELECT run_id, role, split_group_id FROM analysis_run_inputs
             WHERE id = ?1 AND dataset_version_id = ?2",
            params![request.analysis_run_input_id, request.dataset_version_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let Some((input_run, input_role, input_split)) = input else {
        return Err(anyhow!(
            "Evaluation input does not pin the requested dataset version."
        ));
    };
    if input_run != request.analysis_run_id
        || input_role != request.role
        || input_split != request.split_group_id
    {
        return Err(anyhow!(
            "Evaluation run, dataset, role, and split must match the exact AnalysisRun input."
        ));
    }
    let sealed: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM experiment_dataset_versions
         WHERE id = ?1 AND lifecycle_status = 'sealed')",
        [&request.dataset_version_id],
        |row| row.get(0),
    )?;
    if !sealed {
        return Err(anyhow!(
            "Model evaluation requires a sealed DatasetVersion."
        ));
    }
    let metric_run: Option<String> = conn
        .query_row(
            "SELECT run_id FROM analysis_metrics WHERE id = ?1",
            [&request.analysis_metric_id],
            |row| row.get(0),
        )
        .optional()?;
    if metric_run.as_deref() != Some(request.analysis_run_id.as_str()) {
        return Err(anyhow!(
            "Evaluation metric must belong to the exact AnalysisRun."
        ));
    }
    let evaluation_id = id();
    conn.execute(
        "INSERT INTO model_evaluations
            (id, project_id, model_version_id, analysis_run_id, analysis_run_input_id,
             dataset_version_id, analysis_metric_id, role, split_group_id, limitations,
             created_by, evaluated_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            evaluation_id,
            request.project_id,
            request.model_version_id,
            request.analysis_run_id,
            request.analysis_run_input_id,
            request.dataset_version_id,
            request.analysis_metric_id,
            request.role,
            request.split_group_id,
            request.limitations.trim(),
            request.created_by,
            now_us()?
        ],
    )?;
    query_evaluation(conn, &evaluation_id)
}

fn query_evaluation(conn: &Connection, evaluation_id: &str) -> Result<ModelEvaluationDto> {
    Ok(conn.query_row(
        "SELECT e.id, e.project_id, e.model_version_id, e.analysis_run_id,
                e.analysis_run_input_id, e.dataset_version_id, e.role, e.split_group_id,
                e.analysis_metric_id, m.metric_definition_version_id, m.estimand,
                m.sample_size, m.effect_estimate, m.unit, m.uncertainty_kind,
                m.uncertainty_lower, m.uncertainty_upper, m.uncertainty_level,
                m.uncertainty_reason, e.limitations, e.created_by, e.evaluated_at_us
         FROM model_evaluations e
         JOIN analysis_metrics m ON m.id = e.analysis_metric_id
         WHERE e.id = ?1",
        [evaluation_id],
        |row| {
            Ok(ModelEvaluationDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                model_version_id: row.get(2)?,
                analysis_run_id: row.get(3)?,
                analysis_run_input_id: row.get(4)?,
                dataset_version_id: row.get(5)?,
                role: row.get(6)?,
                split_group_id: row.get(7)?,
                analysis_metric_id: row.get(8)?,
                metric_definition_version_id: row.get(9)?,
                estimand: row.get(10)?,
                sample_size: row.get(11)?,
                effect_estimate: row.get(12)?,
                unit: row.get(13)?,
                uncertainty_kind: row.get(14)?,
                uncertainty_lower: row.get(15)?,
                uncertainty_upper: row.get(16)?,
                uncertainty_level: row.get(17)?,
                uncertainty_reason: row.get(18)?,
                limitations: row.get(19)?,
                created_by: row.get(20)?,
                evaluated_at_us: row.get(21)?,
            })
        },
    )?)
}

fn query_evaluations(conn: &Connection, project_id: &str) -> Result<Vec<ModelEvaluationDto>> {
    let mut statement = conn.prepare(
        "SELECT id FROM model_evaluations WHERE project_id = ?1 ORDER BY evaluated_at_us, id",
    )?;
    let ids = statement
        .query_map([project_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.iter()
        .map(|evaluation_id| query_evaluation(conn, evaluation_id))
        .collect()
}

pub fn create_validation(
    conn: &mut Connection,
    request: &CreateModelValidationReq,
) -> Result<ModelValidationDto> {
    validate_validation_request(conn, request)?;
    let validation_id = id();
    let criteria_json = json(&request.criteria)?;
    let criteria_sha256 = sha256(criteria_json.as_bytes());
    conn.execute(
        "INSERT INTO model_validations
            (id, project_id, target_model_version_id, target_finding_version_id,
             target_source_claim_id, criteria_json, criteria_sha256, scope, population_scope,
             evaluation_method, evaluator_identity, independence_state,
             independence_rationale, conflict_of_interest, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            validation_id,
            request.project_id,
            validation_target_column(&request.target, 0),
            validation_target_column(&request.target, 1),
            validation_target_column(&request.target, 2),
            criteria_json,
            criteria_sha256,
            request.scope.trim(),
            request.population_scope.trim(),
            request.evaluation_method.trim(),
            request.evaluator_identity.trim(),
            request.independence_state,
            request.independence_rationale.trim(),
            request.conflict_of_interest.trim(),
            request.created_by,
            now_us()?
        ],
    )?;
    query_validation(conn, &validation_id)
}

fn validate_validation_request(
    conn: &Connection,
    request: &CreateModelValidationReq,
) -> Result<()> {
    for (value, label) in [
        (&request.scope, "Validation scope"),
        (&request.population_scope, "Validation population scope"),
        (&request.evaluation_method, "Validation method"),
        (&request.evaluator_identity, "Evaluator identity"),
        (
            &request.conflict_of_interest,
            "Conflict-of-interest declaration",
        ),
    ] {
        required(value, label)?;
    }
    exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
        &request.project_id,
        "Validation research project was not found.",
    )?;
    ensure_project_writable(conn, &request.project_id)?;
    if request.criteria.is_empty()
        || request.criteria.iter().any(|criterion| match criterion {
            serde_json::Value::String(value) => value.trim().is_empty(),
            serde_json::Value::Object(value) => value
                .get("criterion")
                .and_then(serde_json::Value::as_str)
                .is_none_or(|text| text.trim().is_empty()),
            _ => true,
        })
    {
        return Err(anyhow!(
            "Validation requires non-empty predeclared criteria."
        ));
    }
    if !matches!(
        request.independence_state.as_str(),
        "independent" | "not_independent" | "unknown"
    ) {
        return Err(anyhow!("Invalid evaluator-independence state."));
    }
    if request.independence_state != "independent" {
        required(
            &request.independence_rationale,
            "Evaluator-independence rationale",
        )?;
    }
    match &request.target {
        ModelValidationTarget::ModelVersion(version_id) => {
            exists(
                conn,
                "SELECT EXISTS(SELECT 1 FROM model_versions WHERE id = ?1)",
                version_id,
                "Validation model version was not found.",
            )?;
            let created_in_project: Option<String> = conn
                .query_row(
                    "SELECT created_in_project_id FROM model_versions WHERE id = ?1",
                    [version_id],
                    |row| row.get(0),
                )
                .optional()?;
            if created_in_project.as_deref() != Some(request.project_id.as_str()) {
                return Err(anyhow!(
                    "Validation model version must belong to the requesting project."
                ));
            }
        }
        ModelValidationTarget::FindingVersion(finding_id) => {
            ensure_project_owns(
                conn,
                "SELECT EXISTS(
                    SELECT 1 FROM analysis_finding_versions v
                    JOIN analysis_findings f ON f.id = v.finding_id
                    WHERE v.id = ?1 AND f.project_id = ?2
                 )",
                finding_id,
                &request.project_id,
                "Validation finding must belong to the selected project.",
            )?;
        }
        ModelValidationTarget::SourceClaim(claim_id) => {
            ensure_workspace_source_claim(conn, claim_id)?
        }
    }
    Ok(())
}

fn validation_target_column(target: &ModelValidationTarget, column: usize) -> Option<&str> {
    match (target, column) {
        (ModelValidationTarget::ModelVersion(value), 0) => Some(value),
        (ModelValidationTarget::FindingVersion(value), 1) => Some(value),
        (ModelValidationTarget::SourceClaim(value), 2) => Some(value),
        _ => None,
    }
}

pub fn finalize_validation(
    conn: &mut Connection,
    request: &FinalizeModelValidationReq,
) -> Result<ModelValidationDto> {
    required(&request.rationale, "Validation result rationale")?;
    if request.analysis_evidence_ids.is_empty() && request.model_evidence_ids.is_empty() {
        return Err(anyhow!(
            "A validation result requires at least one underlying evidence record."
        ));
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let target: Option<(Option<String>, Option<String>, Option<String>, String)> = tx
        .query_row(
            "SELECT target_model_version_id, target_finding_version_id,
                    target_source_claim_id, project_id
             FROM model_validations WHERE id = ?1",
            [&request.validation_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let Some((model_version, finding_version, source_claim, project_id)) = target else {
        return Err(anyhow!("Validation record was not found."));
    };
    let owning_project = if let Some(version_id) = model_version.as_deref() {
        Some(
            tx.query_row(
                "SELECT created_in_project_id FROM model_versions WHERE id = ?1",
                [version_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("Validation model version was not found."))?,
        )
    } else if let Some(finding_id) = finding_version.as_deref() {
        Some(
            tx.query_row(
                "SELECT f.project_id
                 FROM analysis_finding_versions v
                 JOIN analysis_findings f ON f.id = v.finding_id
                 WHERE v.id = ?1",
                [finding_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("Validation finding was not found."))?,
        )
    } else if let Some(claim_id) = source_claim.as_deref() {
        ensure_workspace_source_claim(&tx, claim_id)?;
        None
    } else {
        return Err(anyhow!("Validation target was not found."));
    };
    if let Some(owning_project) = owning_project.as_deref() {
        if owning_project != project_id {
            return Err(anyhow!(
                "Validation target must belong to the recorded project."
            ));
        }
        ensure_project_writable(&tx, owning_project)?;
    } else {
        ensure_project_writable(&tx, &project_id)?;
    }
    let existing_result: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM model_validation_events WHERE validation_id = ?1)",
        [&request.validation_id],
        |row| row.get(0),
    )?;
    if existing_result {
        return Err(anyhow!(
            "A validation attempt is immutable after its result is recorded; create a new attempt."
        ));
    }
    for evidence_id in &request.analysis_evidence_ids {
        let evidence: Option<(String, Option<String>, Option<String>, Option<String>)> = tx
            .query_row(
                "SELECT project_id, target_source_claim_id, target_hypothesis_version_id,
                        target_finding_version_id
                 FROM analysis_evidence WHERE id = ?1",
                [evidence_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        let Some((evidence_project, evidence_claim, _, evidence_finding)) = evidence else {
            return Err(anyhow!("Validation evidence was not found."));
        };
        if evidence_project != project_id
            || (source_claim.is_some() && evidence_claim != source_claim)
            || (finding_version.is_some() && evidence_finding != finding_version)
        {
            return Err(anyhow!(
                "Validation evidence must match the target claim/finding and project."
            ));
        }
    }
    for evidence_id in &request.model_evidence_ids {
        let evidence_version: Option<String> = tx
            .query_row(
                "SELECT model_version_id FROM model_evidence WHERE id = ?1",
                [evidence_id],
                |row| row.get(0),
            )
            .optional()?;
        if model_version.as_deref().is_none()
            || evidence_version.as_deref() != model_version.as_deref()
        {
            return Err(anyhow!(
                "Model validation evidence must target the exact validated ModelVersion."
            ));
        }
    }
    tx.execute(
        "INSERT INTO model_validation_events
            (id, validation_id, outcome, rationale, deviations_json, actor_id, occurred_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id(),
            request.validation_id,
            request.outcome.as_str(),
            request.rationale.trim(),
            json(&request.deviations)?,
            request.actor_id,
            now_us()?
        ],
    )?;
    for evidence_id in &request.analysis_evidence_ids {
        tx.execute(
            "INSERT INTO model_validation_analysis_evidence (validation_id, evidence_id)
             VALUES (?1, ?2)",
            params![request.validation_id, evidence_id],
        )?;
    }
    for evidence_id in &request.model_evidence_ids {
        tx.execute(
            "INSERT INTO model_validation_model_evidence (validation_id, evidence_id)
             VALUES (?1, ?2)",
            params![request.validation_id, evidence_id],
        )?;
    }
    let validation = query_validation(&tx, &request.validation_id)?;
    tx.commit()?;
    Ok(validation)
}

fn query_validation(conn: &Connection, validation_id: &str) -> Result<ModelValidationDto> {
    let row = conn.query_row(
        "SELECT v.id, v.project_id, v.target_model_version_id, v.target_finding_version_id,
                v.target_source_claim_id, v.criteria_json, v.scope, v.population_scope,
                v.evaluation_method, v.evaluator_identity, v.independence_state,
                v.independence_rationale, v.conflict_of_interest, v.created_by, v.created_at_us,
                e.outcome, e.rationale, e.deviations_json, e.occurred_at_us
         FROM model_validations v LEFT JOIN model_validation_events e ON e.validation_id = v.id
         WHERE v.id = ?1",
        [validation_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, i64>(14)?,
                row.get::<_, Option<String>>(15)?,
                row.get::<_, Option<String>>(16)?,
                row.get::<_, Option<String>>(17)?,
                row.get::<_, Option<i64>>(18)?,
            ))
        },
    )?;
    let target = match (row.2, row.3, row.4) {
        (Some(value), None, None) => ModelValidationTarget::ModelVersion(value),
        (None, Some(value), None) => ModelValidationTarget::FindingVersion(value),
        (None, None, Some(value)) => ModelValidationTarget::SourceClaim(value),
        _ => return Err(anyhow!("Validation record has an invalid target.")),
    };
    let analysis_evidence_ids = query_ids(
        conn,
        "SELECT evidence_id FROM model_validation_analysis_evidence
         WHERE validation_id = ?1 ORDER BY evidence_id",
        validation_id,
    )?;
    let model_evidence_ids = query_ids(
        conn,
        "SELECT evidence_id FROM model_validation_model_evidence
         WHERE validation_id = ?1 ORDER BY evidence_id",
        validation_id,
    )?;
    let outcome = row
        .15
        .as_deref()
        .map(parse_validation_outcome)
        .transpose()?;
    Ok(ModelValidationDto {
        id: row.0,
        project_id: row.1,
        target,
        criteria: parse_json(&row.5)?,
        scope: row.6,
        population_scope: row.7,
        evaluation_method: row.8,
        evaluator_identity: row.9,
        independence_state: row.10,
        independence_rationale: row.11,
        conflict_of_interest: row.12,
        outcome,
        result_rationale: row.16,
        deviations: row
            .17
            .as_deref()
            .map(parse_json)
            .transpose()?
            .unwrap_or_default(),
        analysis_evidence_ids,
        model_evidence_ids,
        created_by: row.13,
        created_at_us: row.14,
        finalized_at_us: row.18,
    })
}

fn parse_validation_outcome(value: &str) -> Result<ModelValidationOutcome> {
    match value {
        "pass" => Ok(ModelValidationOutcome::Pass),
        "fail" => Ok(ModelValidationOutcome::Fail),
        "inconclusive" => Ok(ModelValidationOutcome::Inconclusive),
        _ => Err(anyhow!("Invalid validation outcome: {value}")),
    }
}

fn query_ids(conn: &Connection, sql: &str, id_value: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(sql)?;
    let ids = statement
        .query_map([id_value], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ids)
}

fn query_validations(conn: &Connection, project_id: &str) -> Result<Vec<ModelValidationDto>> {
    let ids = query_ids(
        conn,
        "SELECT id FROM model_validations WHERE project_id = ?1 ORDER BY created_at_us, id",
        project_id,
    )?;
    ids.iter()
        .map(|validation_id| query_validation(conn, validation_id))
        .collect()
}

pub fn create_replication(
    conn: &mut Connection,
    request: &CreateModelReplicationReq,
) -> Result<ModelReplicationDto> {
    validate_replication_request(conn, request)?;
    let design_version_id: String = conn.query_row(
        "SELECT design_version_id FROM experiment_runs WHERE id = ?1",
        [&request.original_experiment_run_id],
        |row| row.get(0),
    )?;
    let replication_id = id();
    let created_at = now_us()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute(
        "INSERT OR IGNORE INTO model_contributions
            (id, model_version_id, design_version_id, experiment_run_id, relation_kind,
             rationale, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, 'tested_in',
             'Pinned as the original experiment for this replication.', ?5, ?6)",
        params![
            id(),
            request.model_version_id,
            design_version_id,
            request.original_experiment_run_id,
            request.created_by,
            created_at
        ],
    )?;
    tx.execute(
        "INSERT INTO model_replications
            (id, project_id, model_version_id, original_experiment_run_id,
             original_design_version_id, replication_kind, independent_team_operator,
             protocol_context, dataset_context_json, outcome, result_summary,
             deviations_json, provenance_json, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            replication_id,
            request.project_id,
            request.model_version_id,
            request.original_experiment_run_id,
            design_version_id,
            request.replication_kind,
            request.independent_team_operator.trim(),
            request.protocol_context.trim(),
            json(&request.dataset_context)?,
            request.outcome,
            request.result_summary.trim(),
            json(&request.deviations)?,
            json(&request.provenance)?,
            request.created_by,
            now_us()?
        ],
    )?;
    for dataset in &request.datasets {
        tx.execute(
            "INSERT INTO model_replication_datasets
                (replication_id, dataset_version_id, role, split_group_id)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                replication_id,
                dataset.dataset_version_id,
                dataset.role,
                dataset.split_group_id
            ],
        )?;
    }
    for evidence_id in &request.analysis_evidence_ids {
        tx.execute(
            "INSERT INTO model_replication_analysis_evidence (replication_id, evidence_id)
             VALUES (?1, ?2)",
            params![replication_id, evidence_id],
        )?;
    }
    for evidence_id in &request.model_evidence_ids {
        tx.execute(
            "INSERT INTO model_replication_model_evidence (replication_id, evidence_id)
             VALUES (?1, ?2)",
            params![replication_id, evidence_id],
        )?;
    }
    tx.commit()?;
    query_replication(conn, &replication_id)
}

fn validate_replication_request(
    conn: &Connection,
    request: &CreateModelReplicationReq,
) -> Result<()> {
    for (value, label) in [
        (
            &request.independent_team_operator,
            "Independent team/operator",
        ),
        (&request.protocol_context, "Replication protocol context"),
        (&request.result_summary, "Replication result summary"),
    ] {
        required(value, label)?;
    }
    if !matches!(request.replication_kind.as_str(), "direct" | "conceptual") {
        return Err(anyhow!("Replication kind must be direct or conceptual."));
    }
    if !matches!(
        request.outcome.as_str(),
        "supports" | "contradicts" | "mixed" | "inconclusive"
    ) {
        return Err(anyhow!("Invalid replication outcome."));
    }
    if !request.dataset_context.is_object() || !request.provenance.is_object() {
        return Err(anyhow!(
            "Replication dataset context and provenance must be JSON objects."
        ));
    }
    if request.datasets.is_empty() {
        return Err(anyhow!(
            "A replication must pin at least one exact dataset version and role."
        ));
    }
    if request.analysis_evidence_ids.is_empty() && request.model_evidence_ids.is_empty() {
        return Err(anyhow!(
            "A replication result requires at least one underlying evidence record."
        ));
    }
    exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
        &request.project_id,
        "Replication project was not found.",
    )?;
    ensure_project_writable(conn, &request.project_id)?;
    exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM model_versions WHERE id = ?1)",
        &request.model_version_id,
        "Replication model version was not found.",
    )?;
    let created_in_project: Option<String> = conn
        .query_row(
            "SELECT created_in_project_id FROM model_versions WHERE id = ?1",
            [&request.model_version_id],
            |row| row.get(0),
        )
        .optional()?;
    if created_in_project.as_deref() != Some(request.project_id.as_str()) {
        return Err(anyhow!(
            "Replication model version must belong to the requesting project."
        ));
    }
    let original: Option<(String, String)> = conn
        .query_row(
            "SELECT project_id, lifecycle_status FROM experiment_runs WHERE id = ?1",
            [&request.original_experiment_run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((original_project, status)) = original else {
        return Err(anyhow!("Original experiment run was not found."));
    };
    if original_project != request.project_id {
        return Err(anyhow!(
            "Original experiment run must belong to the requesting project."
        ));
    }
    if !matches!(status.as_str(), "completed" | "inconclusive") {
        return Err(anyhow!(
            "Replication requires a completed or inconclusive original experiment run."
        ));
    }
    for dataset in &request.datasets {
        validate_dataset_role(&dataset.role)?;
        let dataset_context: Option<(String, String)> = conn
            .query_row(
                "SELECT d.project_id, v.lifecycle_status
                 FROM experiment_dataset_versions v
                 JOIN experiment_datasets d ON d.id = v.dataset_id
                 WHERE v.id = ?1",
                [&dataset.dataset_version_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((dataset_project, lifecycle_status)) = dataset_context else {
            return Err(anyhow!("Replication dataset version was not found."));
        };
        if dataset_project != request.project_id {
            return Err(anyhow!(
                "Replication datasets must belong to the requesting project."
            ));
        }
        if lifecycle_status != "sealed" {
            return Err(anyhow!("Replication datasets must pin sealed versions."));
        }
        let split_group: Option<String> = conn
            .query_row(
                "SELECT id FROM experiment_dataset_split_groups
                 WHERE id = ?1 AND dataset_version_id = ?2",
                params![dataset.split_group_id, dataset.dataset_version_id],
                |row| row.get(0),
            )
            .optional()?;
        if dataset.split_group_id.is_some() && split_group != dataset.split_group_id {
            return Err(anyhow!(
                "Replication split group must belong to its exact dataset version."
            ));
        }
    }
    for evidence_id in &request.analysis_evidence_ids {
        let evidence: Option<(
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = conn
            .query_row(
                "SELECT project_id, target_source_claim_id, target_hypothesis_version_id,
                        target_finding_version_id, artifact_experiment_run_id,
                        artifact_dataset_version_id, artifact_analysis_output_id,
                        artifact_observation_id, artifact_source_claim_id
                 FROM analysis_evidence WHERE id = ?1",
                [evidence_id],
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
                        row.get(8)?,
                    ))
                },
            )
            .optional()?;
        let Some((
            project,
            target_claim,
            target_hypothesis,
            target_finding,
            artifact_run,
            artifact_dataset,
            artifact_output,
            observation,
            artifact_source_claim,
        )) = evidence
        else {
            return Err(anyhow!("Replication Analysis evidence was not found."));
        };
        if let Some(claim_id) = target_claim.as_deref() {
            ensure_workspace_source_claim(conn, claim_id)?;
        }
        if let Some(hypothesis_version_id) = target_hypothesis.as_deref() {
            let hypothesis_owner: Option<(Option<String>, String)> = conn
                .query_row(
                    "SELECT h.project_id, h.ownership_state
                     FROM hypothesis_definition_versions v
                     JOIN research_hypotheses h ON h.id = v.hypothesis_id
                     WHERE v.id = ?1",
                    [hypothesis_version_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            let Some((hypothesis_project, ownership_state)) = hypothesis_owner else {
                return Err(anyhow!("Replication evidence hypothesis was not found."));
            };
            if ownership_state == "project"
                && hypothesis_project.as_deref() != Some(request.project_id.as_str())
            {
                return Err(anyhow!(
                    "Replication evidence hypothesis must belong to the requesting project."
                ));
            }
        }
        if let Some(finding_id) = target_finding.as_deref() {
            ensure_project_owns(
                conn,
                "SELECT EXISTS(
                    SELECT 1 FROM analysis_finding_versions v
                    JOIN analysis_findings f ON f.id = v.finding_id
                    WHERE v.id = ?1 AND f.project_id = ?2
                 )",
                finding_id,
                &request.project_id,
                "Replication evidence finding must belong to the requesting project.",
            )?;
        }
        if let Some(run_id) = artifact_run.as_deref() {
            ensure_project_owns(
                conn,
                "SELECT EXISTS(
                    SELECT 1 FROM experiment_runs WHERE id = ?1 AND project_id = ?2
                 )",
                run_id,
                &request.project_id,
                "Replication evidence run must belong to the requesting project.",
            )?;
        }
        if let Some(dataset_id) = artifact_dataset.as_deref() {
            ensure_project_owns(
                conn,
                "SELECT EXISTS(
                    SELECT 1 FROM experiment_dataset_versions v
                    JOIN experiment_datasets d ON d.id = v.dataset_id
                    WHERE v.id = ?1 AND d.project_id = ?2
                 )",
                dataset_id,
                &request.project_id,
                "Replication evidence dataset must belong to the requesting project.",
            )?;
        }
        if let Some(output_id) = artifact_output.as_deref() {
            ensure_project_owns(
                conn,
                "SELECT EXISTS(
                    SELECT 1 FROM analysis_outputs o
                    JOIN analysis_runs r ON r.id = o.run_id
                    WHERE o.id = ?1 AND r.project_id = ?2
                 )",
                output_id,
                &request.project_id,
                "Replication evidence output must belong to the requesting project.",
            )?;
        }
        if let Some(observation_id) = observation.as_deref() {
            ensure_project_owns(
                conn,
                "SELECT EXISTS(
                    SELECT 1 FROM research_observations
                    WHERE id = ?1 AND project_id = ?2
                 )",
                observation_id,
                &request.project_id,
                "Replication evidence observation must belong to the requesting project.",
            )?;
        }
        if let Some(claim_id) = artifact_source_claim.as_deref() {
            ensure_workspace_source_claim(conn, claim_id)?;
        }
        let dataset_pinned = artifact_dataset.as_ref().is_some_and(|dataset_id| {
            request
                .datasets
                .iter()
                .any(|dataset| dataset.dataset_version_id == *dataset_id)
        });
        let output_from_original = if let Some(output_id) = artifact_output {
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM analysis_outputs
                 WHERE id = ?1 AND run_id = ?2)",
                params![output_id, request.original_experiment_run_id],
                |row| row.get(0),
            )?
        } else {
            false
        };
        if project != request.project_id
            || (artifact_run.as_deref() != Some(request.original_experiment_run_id.as_str())
                && !dataset_pinned
                && !output_from_original
                && observation.is_none())
        {
            return Err(anyhow!(
                "Replication Analysis evidence must belong to the project and cite the original run, a pinned dataset, or its analysis output."
            ));
        }
    }
    for evidence_id in &request.model_evidence_ids {
        let version: Option<String> = conn
            .query_row(
                "SELECT model_version_id FROM model_evidence WHERE id = ?1",
                [evidence_id],
                |row| row.get(0),
            )
            .optional()?;
        if version.as_deref() != Some(request.model_version_id.as_str()) {
            return Err(anyhow!(
                "Replication model evidence must target the exact replicated version."
            ));
        }
    }
    Ok(())
}

fn query_replication(conn: &Connection, replication_id: &str) -> Result<ModelReplicationDto> {
    let row = conn.query_row(
        "SELECT id, project_id, model_version_id, original_experiment_run_id,
                original_design_version_id, replication_kind, independent_team_operator,
                protocol_context, dataset_context_json, outcome, result_summary,
                deviations_json, provenance_json, created_by, created_at_us
         FROM model_replications WHERE id = ?1",
        [replication_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, i64>(14)?,
            ))
        },
    )?;
    let mut statement = conn.prepare(
        "SELECT dataset_version_id, role, split_group_id
         FROM model_replication_datasets WHERE replication_id = ?1
         ORDER BY role, dataset_version_id",
    )?;
    let datasets = statement
        .query_map([replication_id], |row| {
            Ok(ModelReplicationDatasetReq {
                dataset_version_id: row.get(0)?,
                role: row.get(1)?,
                split_group_id: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ModelReplicationDto {
        id: row.0,
        project_id: row.1,
        model_version_id: row.2,
        original_experiment_run_id: row.3,
        original_design_version_id: row.4,
        replication_kind: row.5,
        independent_team_operator: row.6,
        protocol_context: row.7,
        dataset_context: parse_json(&row.8)?,
        outcome: row.9,
        result_summary: row.10,
        deviations: parse_json(&row.11)?,
        provenance: parse_json(&row.12)?,
        datasets,
        analysis_evidence_ids: query_ids(
            conn,
            "SELECT evidence_id FROM model_replication_analysis_evidence
             WHERE replication_id = ?1 ORDER BY evidence_id",
            replication_id,
        )?,
        model_evidence_ids: query_ids(
            conn,
            "SELECT evidence_id FROM model_replication_model_evidence
             WHERE replication_id = ?1 ORDER BY evidence_id",
            replication_id,
        )?,
        created_by: row.13,
        created_at_us: row.14,
    })
}

fn query_replications(conn: &Connection, project_id: &str) -> Result<Vec<ModelReplicationDto>> {
    let ids = query_ids(
        conn,
        "SELECT id FROM model_replications WHERE project_id = ?1 ORDER BY created_at_us, id",
        project_id,
    )?;
    ids.iter()
        .map(|replication_id| query_replication(conn, replication_id))
        .collect()
}

fn query_evidence_list(conn: &Connection, project_id: &str) -> Result<Vec<ModelEvidenceDto>> {
    let model_ids = project_model_ids(conn, project_id)?;
    if model_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = model_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT e.id
         FROM model_evidence e
         JOIN model_versions v ON v.id = e.model_version_id
         WHERE v.model_id IN ({placeholders})
           AND v.created_in_project_id = ?
         ORDER BY e.created_at_us, e.id"
    );
    let mut params: Vec<&str> = model_ids.iter().map(String::as_str).collect();
    params.push(project_id);
    let mut statement = conn.prepare(&sql)?;
    let ids = statement
        .query_map(rusqlite::params_from_iter(params.iter().copied()), |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.iter()
        .map(|evidence_id| query_evidence(conn, evidence_id))
        .collect()
}

pub fn workspace(conn: &Connection, project_id: &str) -> Result<ModelWorkspaceDto> {
    exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
        project_id,
        "Research project was not found.",
    )?;
    Ok(ModelWorkspaceDto {
        models: query_all_models(conn, project_id)?,
        versions: query_all_versions(conn, project_id)?,
        evidence: query_evidence_list(conn, project_id)?,
        evaluations: query_evaluations(conn, project_id)?,
        validations: query_validations(conn, project_id)?,
        replications: query_replications(conn, project_id)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    fn new_database_path() -> PathBuf {
        std::env::temp_dir().join(format!("vora-model-{}.db", Uuid::now_v7()))
    }

    fn initialize(conn: &Connection) {
        crate::mind::portable_db::initialize_workspace_schema(conn)
            .expect("initialize full workspace schema");
    }

    fn insert_project(conn: &Connection, project_id: &str) {
        conn.execute(
            "INSERT INTO research_projects
                (id, title, aim, lifecycle_status, created_by, created_at_us, updated_at_us)
             VALUES (?1, ?2, '', 'active', NULL, 1, 1)",
            params![project_id, format!("Project {project_id}")],
        )
        .expect("insert project");
    }

    fn insert_hypothesis(conn: &Connection, project_id: &str, suffix: &str) -> String {
        let hypothesis_id = format!("hypothesis-{suffix}");
        let version_id = format!("hypothesis-version-{suffix}");
        conn.execute(
            "INSERT INTO research_hypotheses
                (id, project_id, research_question_id, legacy_hypothesis_id,
                 parent_hypothesis_id, ownership_state, lifecycle_status, created_at_us)
             VALUES (?1, ?2, NULL, NULL, NULL, 'project', 'legacy_untyped', 1)",
            params![hypothesis_id, project_id],
        )
        .expect("insert hypothesis identity");
        conn.execute(
            "INSERT INTO hypothesis_definition_versions
                (id, hypothesis_id, version_no, parent_version_id, statement_h0,
                 statement_h1, population_scope, research_scope, expected_direction,
                 expected_effect_decimal, expected_effect_unit, falsification_criteria,
                 assumptions, lifecycle_status, created_at_us, preregistered_at_us)
             VALUES (?1, ?2, 1, NULL, 'H0', 'H1', 'population', 'scope',
                     'non_directional', NULL, NULL, 'falsification rule', '',
                     'draft', 1, NULL)",
            params![version_id, hypothesis_id],
        )
        .expect("insert hypothesis version");
        version_id
    }

    fn insert_finding_version(
        conn: &Connection,
        project_id: &str,
        suffix: &str,
        hypothesis_version_id: &str,
    ) -> String {
        let finding_id = format!("finding-{suffix}");
        let version_id = format!("finding-version-{suffix}");
        conn.execute(
            "INSERT INTO analysis_findings (id, project_id, created_by, created_at_us)
             VALUES (?1, ?2, 'test', 1)",
            params![finding_id, project_id],
        )
        .expect("insert finding");
        conn.execute(
            "INSERT INTO analysis_finding_versions
                (id, finding_id, version_no, parent_version_id, change_rationale,
                 title, statement, target_source_claim_id, target_hypothesis_version_id,
                 scope, applicability, uncertainty, limitations, conclusion_kind,
                 content_sha256, created_by, created_at_us)
             VALUES (?1, ?2, 1, NULL, NULL, 'Finding', 'Finding statement', NULL, ?3,
                     'scope', 'applicability', 'uncertainty', 'limitations', 'result',
                     ?4, 'test', 1)",
            params![
                version_id,
                finding_id,
                hypothesis_version_id,
                "6".repeat(64)
            ],
        )
        .expect("insert finding version");
        version_id
    }

    fn model_request(origin_project_id: &str) -> CreateModelReq {
        CreateModelReq {
            display_name: "Baseline model".to_string(),
            description: "Reusable test model".to_string(),
            origin_project_id: Some(origin_project_id.to_string()),
            domain_id: None,
            created_by: Some("author".to_string()),
        }
    }

    fn version_request(project_id: &str, model_id: &str) -> CreateModelVersionReq {
        CreateModelVersionReq {
            project_id: project_id.to_string(),
            model_id: model_id.to_string(),
            change_class: ModelChangeClass::Revision,
            change_rationale: "Register the initial baseline.".to_string(),
            expected_implications: "Provides the first immutable specification.".to_string(),
            specification: ModelSpecificationDto {
                human_readable: "A model used to test versioned provenance.".to_string(),
                machine_readable: serde_json::json!({"kind": "baseline"}),
            },
            assumptions: vec![ModelAssumptionReq {
                key: "scope".to_string(),
                statement: "Applies only to the declared population.".to_string(),
            }],
            scope: "Declared study population".to_string(),
            dependencies: vec![],
            environment: serde_json::json!({"runtime": "captured"}),
            parents: vec![],
            parameters: vec![ModelParameterReq {
                name: "threshold".to_string(),
                value: "0.1250000000000000001".to_string(),
                value_kind: "decimal".to_string(),
                unit: None,
                lower_bound: Some("0".to_string()),
                upper_bound: Some("1".to_string()),
                default_value: None,
                role: "decision_threshold".to_string(),
                tuneable: true,
            }],
            artifacts: vec![],
            impacts: vec![ModelImpactReq {
                target: ModelImpactTarget::Assumption {
                    assumption_key: "scope".to_string(),
                },
                impact_kind: ModelImpactKind::Added,
                previous_statement: None,
                current_statement: Some("Applies only to the declared population.".to_string()),
                rationale: "Declare the baseline scope assumption.".to_string(),
            }],
            hypotheses: vec![],
            findings: vec![],
            experiments: vec![],
            dataset_uses: vec![],
            evidence_ids: vec![],
            created_by: Some("author".to_string()),
        }
    }

    fn insert_sealed_dataset(conn: &Connection, project_id: &str) -> String {
        let dataset_id = format!("dataset-{project_id}");
        let dataset_version_id = format!("dataset-version-{project_id}");
        conn.execute(
            "INSERT INTO experiment_datasets
                (id, project_id, title, description, created_by, created_at_us)
             VALUES (?1, ?2, 'Pinned evaluation data', '', NULL, 1)",
            params![dataset_id, project_id],
        )
        .expect("insert dataset");
        conn.execute(
            "INSERT INTO experiment_dataset_versions
                (id, dataset_id, version_no, schema_id, schema_version, sample_schema_json,
                 expected_sample_count, lifecycle_status, content_sha256, created_at_us, sealed_at_us)
             VALUES (?1, ?2, 1, 'test-schema', 1, '{}', 1, 'staged', NULL, 1, NULL)",
            params![dataset_version_id, dataset_id],
        )
        .expect("insert sealed dataset version");
        conn.execute(
            "INSERT INTO experiment_dataset_samples
                (id, dataset_version_id, sample_id, sample_kind, record_json, created_at_us)
             VALUES (?1, ?2, 'sample-1', 'data', '{}', 1)",
            params![format!("sample-{project_id}"), dataset_version_id],
        )
        .expect("insert dataset sample");
        conn.execute(
            "UPDATE experiment_dataset_versions
             SET lifecycle_status = 'sealed', content_sha256 = ?2, sealed_at_us = 2
             WHERE id = ?1",
            params![dataset_version_id, "0".repeat(64)],
        )
        .expect("seal dataset version");
        dataset_version_id
    }

    fn insert_experiment(conn: &Connection, project_id: &str) -> (String, String) {
        let design_id = format!("design-{project_id}");
        let design_version_id = format!("design-version-{project_id}");
        let run_id = format!("experiment-run-{project_id}");
        conn.execute(
            "INSERT INTO research_experiment_designs
                (id, project_id, title, created_by, created_at_us)
             VALUES (?1, ?2, 'Original design', NULL, 1)",
            params![design_id, project_id],
        )
        .expect("insert experiment design");
        conn.execute(
            "INSERT INTO research_experiment_design_versions
                (id, design_id, version_no, description, protocol, lifecycle_status,
                 created_by, created_at_us, preregistered_at_us)
             VALUES (?1, ?2, 1, 'Design description', 'Protocol', 'draft', NULL, 1, NULL)",
            params![design_version_id, design_id],
        )
        .expect("insert design version");
        conn.execute(
            "INSERT INTO experiment_runs
                (id, project_id, design_version_id, lifecycle_status, created_at_us, ended_at_us)
             VALUES (?1, ?2, ?3, 'completed', 1, 2)",
            params![run_id, project_id, design_version_id],
        )
        .expect("insert completed experiment run");
        (design_version_id, run_id)
    }

    fn insert_analysis_run(
        conn: &Connection,
        project_id: &str,
        dataset_version_id: &str,
    ) -> (String, String, String) {
        let method_id = format!("method-{project_id}");
        let method_version_id = format!("method-version-{project_id}");
        let metric_id = format!("metric-{project_id}");
        let metric_version_id = format!("metric-version-{project_id}");
        let definition_id = format!("analysis-definition-{project_id}");
        let definition_version_id = format!("analysis-definition-version-{project_id}");
        let run_id = format!("analysis-run-{project_id}");
        let input_id = format!("analysis-input-{project_id}");
        let output_id = format!("analysis-output-{project_id}");
        let analysis_metric_id = format!("analysis-metric-{project_id}");
        conn.execute(
            "INSERT INTO analysis_methods (id, project_id, name, created_at_us)
             VALUES (?1, ?2, 'test method', 1)",
            params![method_id, project_id],
        )
        .expect("insert analysis method");
        conn.execute(
            "INSERT INTO analysis_method_versions
                (id, method_id, version_no, version_label, code_uri, code_revision,
                 environment_json, assumptions_json, test_suite_uri, test_suite_sha256,
                 tests_total, tests_passed, verification_status, content_sha256, created_at_us)
             VALUES (?1, ?2, 1, '1', 'file:///method', 'rev-1', '{}', '[]',
                     'file:///tests', ?3, 1, 1, 'passed', ?3, 1)",
            params![method_version_id, method_id, "1".repeat(64)],
        )
        .expect("insert method version");
        conn.execute(
            "INSERT INTO analysis_metric_definitions
                (id, project_id, metric_key, name, created_at_us)
             VALUES (?1, ?2, 'effect', 'Effect estimate', 1)",
            params![metric_id, project_id],
        )
        .expect("insert metric definition");
        conn.execute(
            "INSERT INTO analysis_metric_definition_versions
                (id, metric_definition_id, version_no, version_label, formula, unit,
                 test_suite_uri, test_suite_sha256, tests_total, tests_passed,
                 verification_status, content_sha256, created_at_us)
             VALUES (?1, ?2, 1, '1', 'difference', 'units', 'file:///tests',
                     ?3, 1, 1, 'passed', ?3, 1)",
            params![metric_version_id, metric_id, "2".repeat(64)],
        )
        .expect("insert metric version");
        conn.execute(
            "INSERT INTO analysis_definitions (id, project_id, title, created_at_us)
             VALUES (?1, ?2, 'Test analysis', 1)",
            params![definition_id, project_id],
        )
        .expect("insert analysis definition");
        conn.execute(
            "INSERT INTO analysis_definition_versions
                (id, definition_id, version_no, version_label, method_version_id,
                 specification_json, specification_sha256, created_at_us)
             VALUES (?1, ?2, 1, '1', ?3, '{}', ?4, 1)",
            params![
                definition_version_id,
                definition_id,
                method_version_id,
                "3".repeat(64)
            ],
        )
        .expect("insert analysis definition version");
        conn.execute(
            "INSERT INTO analysis_runs
                (id, project_id, definition_version_id, parameters_json, environment_json,
                 random_seeds_json, protocol_version, created_at_us)
             VALUES (?1, ?2, ?3, '{}', '{}', '[]', '1', 1)",
            params![run_id, project_id, definition_version_id],
        )
        .expect("insert analysis run");
        conn.execute(
            "INSERT INTO analysis_run_events (id, run_id, lifecycle_status, occurred_at_us)
             VALUES (?1, ?2, 'queued', 1)",
            params![format!("analysis-queued-{project_id}"), run_id],
        )
        .expect("queue analysis run");
        conn.execute(
            "INSERT INTO analysis_run_events (id, run_id, lifecycle_status, occurred_at_us)
             VALUES (?1, ?2, 'running', 2)",
            params![format!("analysis-running-{project_id}"), run_id],
        )
        .expect("start analysis run");
        conn.execute(
            "INSERT INTO analysis_run_inputs
                (id, run_id, dataset_version_id, role, dataset_checksum, sample_count,
                 transformation_lineage_json, input_manifest_sha256)
             VALUES (?1, ?2, ?3, 'test', ?4, 1, '{}', ?4)",
            params![input_id, run_id, dataset_version_id, "4".repeat(64)],
        )
        .expect("insert analysis input");
        conn.execute(
            "INSERT INTO analysis_outputs
                (id, run_id, output_key, output_kind, title, payload_json, payload_sha256, created_at_us)
             VALUES (?1, ?2, 'result', 'report', 'Report', '{}', ?3, 2)",
            params![output_id, run_id, "5".repeat(64)],
        )
        .expect("insert analysis output");
        conn.execute(
            "INSERT INTO analysis_metrics
                (id, run_id, output_id, metric_definition_version_id, method_version_id,
                 estimand, sample_size, effect_estimate, unit, uncertainty_kind,
                 uncertainty_lower, uncertainty_upper, uncertainty_level,
                 assumptions_json, exclusions_json, missing_data_json, multiplicity_treatment)
             VALUES (?1, ?2, ?3, ?4, ?5, 'mean difference', 1, '0.125', 'units',
                     'interval', '0.1', '0.15', '0.95', '[]', '[]', '{}', 'none planned')",
            params![
                analysis_metric_id,
                run_id,
                output_id,
                metric_version_id,
                method_version_id
            ],
        )
        .expect("insert analysis metric");
        conn.execute(
            "INSERT INTO analysis_run_events (id, run_id, lifecycle_status, occurred_at_us)
             VALUES (?1, ?2, 'completed', 3)",
            params![format!("analysis-completed-{project_id}"), run_id],
        )
        .expect("complete analysis run");
        (run_id, input_id, analysis_metric_id)
    }

    struct ProjectModelFixture {
        model_id: String,
        version_id: String,
        dataset_version_id: String,
        experiment_design_version_id: String,
        experiment_run_id: String,
        analysis_run_id: String,
        analysis_input_id: String,
        analysis_metric_id: String,
        evidence_id: String,
    }

    fn create_project_model_fixture(
        conn: &mut Connection,
        project_id: &str,
    ) -> ProjectModelFixture {
        let model = create_model(conn, &model_request(project_id)).expect("create project model");
        let version = create_model_version(conn, &version_request(project_id, &model.id))
            .expect("create project model version");
        let dataset_version_id = insert_sealed_dataset(conn, project_id);
        let (experiment_design_version_id, experiment_run_id) = insert_experiment(conn, project_id);
        let (analysis_run_id, analysis_input_id, analysis_metric_id) =
            insert_analysis_run(conn, project_id, &dataset_version_id);
        let evidence = create_evidence(
            conn,
            &CreateModelEvidenceReq {
                project_id: project_id.to_string(),
                model_version_id: version.id.clone(),
                artifact: ModelArtifactRef::DatasetVersion(dataset_version_id.clone()),
                polarity: ModelEvidencePolarity::Supports,
                scope: format!("{project_id} model fixture"),
                quality_rationale: "Pinned to the project's sealed dataset.".to_string(),
                limitations: "Test fixture.".to_string(),
                provenance: serde_json::json!({"project": project_id}),
                created_by: Some("test".to_string()),
            },
        )
        .expect("create project model evidence");
        ProjectModelFixture {
            model_id: model.id,
            version_id: version.id,
            dataset_version_id,
            experiment_design_version_id,
            experiment_run_id,
            analysis_run_id,
            analysis_input_id,
            analysis_metric_id,
            evidence_id: evidence.id,
        }
    }

    fn archived_project_model_fixture() -> (Connection, ProjectModelFixture) {
        let mut conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        insert_project(&conn, "project-a");
        let project = create_project_model_fixture(&mut conn, "project-a");
        conn.execute(
            "UPDATE research_projects SET lifecycle_status = 'archived' WHERE id = 'project-a'",
            [],
        )
        .expect("archive project A");
        (conn, project)
    }

    fn evaluation_request(
        project_id: &str,
        project: &ProjectModelFixture,
    ) -> CreateModelEvaluationReq {
        CreateModelEvaluationReq {
            project_id: project_id.to_string(),
            model_version_id: project.version_id.clone(),
            analysis_run_id: project.analysis_run_id.clone(),
            analysis_run_input_id: project.analysis_input_id.clone(),
            dataset_version_id: project.dataset_version_id.clone(),
            analysis_metric_id: project.analysis_metric_id.clone(),
            role: "test".to_string(),
            split_group_id: None,
            limitations: "Test fixture.".to_string(),
            created_by: None,
        }
    }

    fn validation_request(
        project_id: &str,
        project: &ProjectModelFixture,
    ) -> CreateModelValidationReq {
        CreateModelValidationReq {
            project_id: project_id.to_string(),
            target: ModelValidationTarget::ModelVersion(project.version_id.clone()),
            criteria: vec![serde_json::json!({"criterion": "threshold"})],
            scope: "Fixture model".to_string(),
            population_scope: format!("{project_id} population"),
            evaluation_method: "Compare against threshold".to_string(),
            evaluator_identity: "test reviewer".to_string(),
            independence_state: "independent".to_string(),
            independence_rationale: String::new(),
            conflict_of_interest: "None.".to_string(),
            created_by: None,
        }
    }

    fn replication_request(
        project_id: &str,
        project: &ProjectModelFixture,
    ) -> CreateModelReplicationReq {
        CreateModelReplicationReq {
            project_id: project_id.to_string(),
            model_version_id: project.version_id.clone(),
            original_experiment_run_id: project.experiment_run_id.clone(),
            replication_kind: "direct".to_string(),
            independent_team_operator: "test team".to_string(),
            protocol_context: "Test fixture.".to_string(),
            dataset_context: serde_json::json!({}),
            datasets: vec![ModelReplicationDatasetReq {
                dataset_version_id: project.dataset_version_id.clone(),
                role: "test".to_string(),
                split_group_id: None,
            }],
            outcome: "supports".to_string(),
            result_summary: "Test fixture.".to_string(),
            deviations: vec![],
            provenance: serde_json::json!({}),
            analysis_evidence_ids: vec![],
            model_evidence_ids: vec![project.evidence_id.clone()],
            created_by: None,
        }
    }

    fn insert_workspace_source_claim(conn: &Connection) -> String {
        conn.execute(
            "INSERT INTO sources (id, kind, title, created_at, updated_at)
             VALUES ('source-shared', 'document', 'Shared source', 1, 1)",
            [],
        )
        .expect("insert shared source");
        conn.execute(
            "INSERT INTO source_versions
                (id, source_id, capture_kind, captured_at_us, created_by, created_at_us)
             VALUES ('source-version-shared', 'source-shared', 'retrieval', 1, 'test', 1)",
            [],
        )
        .expect("insert shared source version");
        conn.execute(
            "INSERT INTO source_version_segments
                (id, source_id, source_version_id, label, location_kind, created_at_us)
             VALUES ('source-segment-shared', 'source-shared', 'source-version-shared',
                     'Shared excerpt', 'unknown', 1)",
            [],
        )
        .expect("insert shared source segment");
        conn.execute(
            "INSERT INTO knowledge_extractions
                (id, source_id, source_version_id, source_segment_id, kind,
                 content, created_by, created_at_us)
             VALUES ('extraction-shared', 'source-shared', 'source-version-shared',
                     'source-segment-shared', 'source_claim', 'Shared claim', 'test', 1)",
            [],
        )
        .expect("insert shared source extraction");
        conn.execute(
            "INSERT INTO extraction_reviews
                (id, extraction_id, review_status, reviewed_at_us)
             VALUES ('extraction-review-shared', 'extraction-shared', 'accepted', 1)",
            [],
        )
        .expect("accept shared source extraction");
        conn.execute(
            "INSERT INTO source_claims
                (id, source_id, source_version_id, extraction_id, statement, created_at_us)
             VALUES ('claim-shared', 'source-shared', 'source-version-shared',
                     'extraction-shared', 'A workspace-shared claim.', 1)",
            [],
        )
        .expect("insert workspace-shared claim");
        "claim-shared".to_string()
    }

    #[test]
    fn project_private_model_reads_and_writes_are_isolated_in_both_directions() {
        let mut conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        insert_project(&conn, "project-a");
        insert_project(&conn, "project-b");
        let project_a = create_project_model_fixture(&mut conn, "project-a");
        let project_b = create_project_model_fixture(&mut conn, "project-b");

        let workspace_a = workspace(&conn, "project-a").expect("load project A");
        let workspace_b = workspace(&conn, "project-b").expect("load project B");
        assert!(workspace_a
            .versions
            .iter()
            .all(|version| version.id != project_b.version_id));
        assert!(workspace_b
            .versions
            .iter()
            .all(|version| version.id != project_a.version_id));
        assert!(workspace_a
            .evidence
            .iter()
            .all(|evidence| evidence.id != project_b.evidence_id));
        assert!(workspace_b
            .evidence
            .iter()
            .all(|evidence| evidence.id != project_a.evidence_id));
        assert!(workspace_a
            .models
            .iter()
            .all(|model| model.origin_project_id.as_deref() == Some("project-a")));
        assert!(workspace_b
            .models
            .iter()
            .all(|model| model.origin_project_id.as_deref() == Some("project-b")));

        let counts_before: (i64, i64, i64, i64) = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM model_evidence),
                        (SELECT COUNT(*) FROM model_evaluations),
                        (SELECT COUNT(*) FROM model_validations),
                        (SELECT COUNT(*) FROM model_replications)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("count model records");

        for (project_id, target, local) in [
            ("project-a", &project_b, &project_a),
            ("project-b", &project_a, &project_b),
        ] {
            let mut foreign_model_version = version_request(project_id, &target.model_id);
            foreign_model_version.change_class = ModelChangeClass::Extension;
            foreign_model_version.parents = vec![ModelParentReq {
                version_id: target.version_id.clone(),
                rationale: "Negative cross-project versioning test.".to_string(),
            }];
            assert!(create_model_version(&mut conn, &foreign_model_version).is_err());
            assert!(create_evidence(
                &mut conn,
                &CreateModelEvidenceReq {
                    project_id: project_id.to_string(),
                    model_version_id: target.version_id.clone(),
                    artifact: ModelArtifactRef::DatasetVersion(local.dataset_version_id.clone()),
                    polarity: ModelEvidencePolarity::Supports,
                    scope: "unauthorized cross-project evidence".to_string(),
                    quality_rationale: "Negative isolation test.".to_string(),
                    limitations: "None.".to_string(),
                    provenance: serde_json::json!({}),
                    created_by: None,
                },
            )
            .is_err());
            assert!(create_evaluation(
                &mut conn,
                &CreateModelEvaluationReq {
                    project_id: project_id.to_string(),
                    model_version_id: target.version_id.clone(),
                    analysis_run_id: local.analysis_run_id.clone(),
                    analysis_run_input_id: local.analysis_input_id.clone(),
                    dataset_version_id: local.dataset_version_id.clone(),
                    analysis_metric_id: local.analysis_metric_id.clone(),
                    role: "test".to_string(),
                    split_group_id: None,
                    limitations: "Negative isolation test.".to_string(),
                    created_by: None,
                },
            )
            .is_err());
            assert!(create_validation(
                &mut conn,
                &CreateModelValidationReq {
                    project_id: project_id.to_string(),
                    target: ModelValidationTarget::ModelVersion(target.version_id.clone()),
                    criteria: vec![serde_json::json!({"criterion": "threshold"})],
                    scope: "Cross-project test".to_string(),
                    population_scope: "Fixture population".to_string(),
                    evaluation_method: "Compare against threshold".to_string(),
                    evaluator_identity: "test reviewer".to_string(),
                    independence_state: "independent".to_string(),
                    independence_rationale: String::new(),
                    conflict_of_interest: "None.".to_string(),
                    created_by: None,
                },
            )
            .is_err());
            assert!(create_replication(
                &mut conn,
                &CreateModelReplicationReq {
                    project_id: project_id.to_string(),
                    model_version_id: target.version_id.clone(),
                    original_experiment_run_id: local.experiment_run_id.clone(),
                    replication_kind: "direct".to_string(),
                    independent_team_operator: "test team".to_string(),
                    protocol_context: "Cross-project rejection fixture.".to_string(),
                    dataset_context: serde_json::json!({}),
                    datasets: vec![ModelReplicationDatasetReq {
                        dataset_version_id: local.dataset_version_id.clone(),
                        role: "test".to_string(),
                        split_group_id: None,
                    }],
                    outcome: "supports".to_string(),
                    result_summary: "Must not be recorded.".to_string(),
                    deviations: vec![],
                    provenance: serde_json::json!({}),
                    analysis_evidence_ids: vec![],
                    model_evidence_ids: vec![target.evidence_id.clone()],
                    created_by: None,
                },
            )
            .is_err());
        }
        assert!(create_evidence(
            &mut conn,
            &CreateModelEvidenceReq {
                project_id: "project-a".to_string(),
                model_version_id: project_a.version_id.clone(),
                artifact: ModelArtifactRef::DatasetVersion(project_b.dataset_version_id.clone()),
                polarity: ModelEvidencePolarity::Supports,
                scope: "Foreign dataset reference".to_string(),
                quality_rationale: "Negative isolation test.".to_string(),
                limitations: "None.".to_string(),
                provenance: serde_json::json!({}),
                created_by: None,
            },
        )
        .is_err());
        assert!(create_evaluation(
            &mut conn,
            &CreateModelEvaluationReq {
                project_id: "project-a".to_string(),
                model_version_id: project_a.version_id.clone(),
                analysis_run_id: project_b.analysis_run_id.clone(),
                analysis_run_input_id: project_b.analysis_input_id.clone(),
                dataset_version_id: project_b.dataset_version_id.clone(),
                analysis_metric_id: project_b.analysis_metric_id.clone(),
                role: "test".to_string(),
                split_group_id: None,
                limitations: "Foreign run reference.".to_string(),
                created_by: None,
            },
        )
        .is_err());

        let counts_after: (i64, i64, i64, i64) = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM model_evidence),
                        (SELECT COUNT(*) FROM model_evaluations),
                        (SELECT COUNT(*) FROM model_validations),
                        (SELECT COUNT(*) FROM model_replications)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("count model records after rejected writes");
        assert_eq!(counts_after, counts_before);
    }

    #[test]
    fn project_scoped_model_reads_hide_other_projects_records() {
        let mut conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        insert_project(&conn, "project-a");
        insert_project(&conn, "project-b");
        let project_a = create_project_model_fixture(&mut conn, "project-a");
        let project_b = create_project_model_fixture(&mut conn, "project-b");

        for (project_id, local) in [("project-a", &project_a), ("project-b", &project_b)] {
            create_evaluation(&mut conn, &evaluation_request(project_id, local))
                .expect("create project evaluation");
            let validation = create_validation(&mut conn, &validation_request(project_id, local))
                .expect("create project validation");
            finalize_validation(
                &mut conn,
                &FinalizeModelValidationReq {
                    validation_id: validation.id,
                    outcome: ModelValidationOutcome::Pass,
                    rationale: "Project-scoped read fixture.".to_string(),
                    deviations: vec![],
                    analysis_evidence_ids: vec![],
                    model_evidence_ids: vec![local.evidence_id.clone()],
                    actor_id: None,
                },
            )
            .expect("finalize project validation");
            create_replication(&mut conn, &replication_request(project_id, local))
                .expect("create project replication");
        }

        for (project_id, local, foreign) in [
            ("project-a", &project_a, &project_b),
            ("project-b", &project_b, &project_a),
        ] {
            let evidence = query_evidence_list(&conn, project_id).expect("query project evidence");
            assert!(evidence.iter().any(|item| item.id == local.evidence_id));
            assert!(evidence.iter().all(|item| item.id != foreign.evidence_id));

            let evaluations = query_evaluations(&conn, project_id).expect("query evaluations");
            assert_eq!(evaluations.len(), 1);
            assert_eq!(evaluations[0].project_id, project_id);
            assert_ne!(evaluations[0].model_version_id, foreign.version_id);

            let validations = query_validations(&conn, project_id).expect("query validations");
            assert_eq!(validations.len(), 1);
            assert_eq!(validations[0].project_id, project_id);
            assert_ne!(
                validations[0].target,
                ModelValidationTarget::ModelVersion(foreign.version_id.clone())
            );

            let replications = query_replications(&conn, project_id).expect("query replications");
            assert_eq!(replications.len(), 1);
            assert_eq!(replications[0].project_id, project_id);
            assert_ne!(replications[0].model_version_id, foreign.version_id);
        }
    }

    #[test]
    fn evidence_requires_same_project_dataset_and_experiment_references() {
        let mut conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        insert_project(&conn, "project-a");
        insert_project(&conn, "project-b");
        let project_a = create_project_model_fixture(&mut conn, "project-a");
        let project_b = create_project_model_fixture(&mut conn, "project-b");

        for (project_id, local, foreign) in [
            ("project-a", &project_a, &project_b),
            ("project-b", &project_b, &project_a),
        ] {
            for artifact in [
                ModelArtifactRef::DatasetVersion(foreign.dataset_version_id.clone()),
                ModelArtifactRef::ExperimentRun(foreign.experiment_run_id.clone()),
            ] {
                let error = create_evidence(
                    &mut conn,
                    &CreateModelEvidenceReq {
                        project_id: project_id.to_string(),
                        model_version_id: local.version_id.clone(),
                        artifact,
                        polarity: ModelEvidencePolarity::Supports,
                        scope: "Cross-project artifact test".to_string(),
                        quality_rationale: "Must reference a local artifact.".to_string(),
                        limitations: "Test fixture.".to_string(),
                        provenance: serde_json::json!({}),
                        created_by: None,
                    },
                )
                .expect_err("evidence cannot reference another project's data");
                assert!(error
                    .to_string()
                    .contains("must belong to the requesting project"));
            }
        }
    }

    #[test]
    fn evaluations_reject_foreign_runs_inputs_datasets_and_metrics_in_both_directions() {
        let mut conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        insert_project(&conn, "project-a");
        insert_project(&conn, "project-b");
        let project_a = create_project_model_fixture(&mut conn, "project-a");
        let project_b = create_project_model_fixture(&mut conn, "project-b");

        for (project_id, local, foreign) in [
            ("project-a", &project_a, &project_b),
            ("project-b", &project_b, &project_a),
        ] {
            let mut request = evaluation_request(project_id, local);
            request.analysis_run_id = foreign.analysis_run_id.clone();
            request.analysis_run_input_id = foreign.analysis_input_id.clone();
            request.dataset_version_id = foreign.dataset_version_id.clone();
            request.analysis_metric_id = foreign.analysis_metric_id.clone();
            assert!(create_evaluation(&mut conn, &request)
                .expect_err("evaluation run must be project-local")
                .to_string()
                .contains("must match the AnalysisRun project"));

            let mut request = evaluation_request(project_id, local);
            request.dataset_version_id = foreign.dataset_version_id.clone();
            assert!(create_evaluation(&mut conn, &request)
                .expect_err("evaluation dataset must match its exact local input")
                .to_string()
                .contains("does not pin the requested dataset version"));

            let mut request = evaluation_request(project_id, local);
            request.analysis_run_input_id = foreign.analysis_input_id.clone();
            assert!(create_evaluation(&mut conn, &request)
                .expect_err("evaluation input must belong to its exact local run")
                .to_string()
                .contains("does not pin the requested dataset version"));

            let mut request = evaluation_request(project_id, local);
            request.analysis_metric_id = foreign.analysis_metric_id.clone();
            assert!(create_evaluation(&mut conn, &request)
                .expect_err("evaluation metric must belong to its exact local run")
                .to_string()
                .contains("metric must belong to the exact AnalysisRun"));
        }
    }

    #[test]
    fn validations_reject_foreign_targets_and_model_evidence_in_both_directions() {
        let mut conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        insert_project(&conn, "project-a");
        insert_project(&conn, "project-b");
        let project_a = create_project_model_fixture(&mut conn, "project-a");
        let project_b = create_project_model_fixture(&mut conn, "project-b");
        let hypothesis_a = insert_hypothesis(&conn, "project-a", "validation-a");
        let hypothesis_b = insert_hypothesis(&conn, "project-b", "validation-b");
        let finding_a = insert_finding_version(&conn, "project-a", "validation-a", &hypothesis_a);
        let finding_b = insert_finding_version(&conn, "project-b", "validation-b", &hypothesis_b);

        for (project_id, local, foreign, foreign_finding) in [
            ("project-a", &project_a, &project_b, &finding_b),
            ("project-b", &project_b, &project_a, &finding_a),
        ] {
            let mut request = validation_request(project_id, local);
            request.target = ModelValidationTarget::ModelVersion(foreign.version_id.clone());
            assert!(create_validation(&mut conn, &request)
                .expect_err("validation target must be project-local")
                .to_string()
                .contains("must belong to the requesting project"));

            let mut request = validation_request(project_id, local);
            request.target = ModelValidationTarget::FindingVersion(foreign_finding.clone());
            assert!(create_validation(&mut conn, &request)
                .expect_err("validation finding must be project-local")
                .to_string()
                .contains("must belong to the selected project"));

            let validation = create_validation(&mut conn, &validation_request(project_id, local))
                .expect("create local validation");
            assert!(finalize_validation(
                &mut conn,
                &FinalizeModelValidationReq {
                    validation_id: validation.id,
                    outcome: ModelValidationOutcome::Pass,
                    rationale: "Foreign evidence must be rejected.".to_string(),
                    deviations: vec![],
                    analysis_evidence_ids: vec![],
                    model_evidence_ids: vec![foreign.evidence_id.clone()],
                    actor_id: None,
                },
            )
            .expect_err("validation evidence must target the exact local version")
            .to_string()
            .contains("must target the exact validated ModelVersion"));
        }
    }

    #[test]
    fn replications_reject_foreign_runs_datasets_and_evidence_in_both_directions() {
        let mut conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        insert_project(&conn, "project-a");
        insert_project(&conn, "project-b");
        let project_a = create_project_model_fixture(&mut conn, "project-a");
        let project_b = create_project_model_fixture(&mut conn, "project-b");

        for (project_id, local, foreign) in [
            ("project-a", &project_a, &project_b),
            ("project-b", &project_b, &project_a),
        ] {
            let mut request = replication_request(project_id, local);
            request.original_experiment_run_id = foreign.experiment_run_id.clone();
            assert!(create_replication(&mut conn, &request)
                .expect_err("replication run must be project-local")
                .to_string()
                .contains("must belong to the requesting project"));

            let mut request = replication_request(project_id, local);
            request.datasets[0].dataset_version_id = foreign.dataset_version_id.clone();
            assert!(create_replication(&mut conn, &request)
                .expect_err("replication dataset must be project-local")
                .to_string()
                .contains("must belong to the requesting project"));

            let mut request = replication_request(project_id, local);
            request.model_evidence_ids = vec![foreign.evidence_id.clone()];
            assert!(create_replication(&mut conn, &request)
                .expect_err("replication evidence must target the exact local version")
                .to_string()
                .contains("must target the exact replicated version"));
        }
    }

    #[test]
    fn model_evidence_accepts_workspace_shared_source_claims_for_both_projects() {
        let mut conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        insert_project(&conn, "project-a");
        insert_project(&conn, "project-b");
        let project_a = create_project_model_fixture(&mut conn, "project-a");
        let project_b = create_project_model_fixture(&mut conn, "project-b");
        let claim_id = insert_workspace_source_claim(&conn);

        for (project_id, project) in [("project-a", &project_a), ("project-b", &project_b)] {
            create_evidence(
                &mut conn,
                &CreateModelEvidenceReq {
                    project_id: project_id.to_string(),
                    model_version_id: project.version_id.clone(),
                    artifact: ModelArtifactRef::SourceClaim(claim_id.clone()),
                    polarity: ModelEvidencePolarity::Supports,
                    scope: "Shared workspace source".to_string(),
                    quality_rationale: "The source claim is workspace-shared.".to_string(),
                    limitations: "Test fixture.".to_string(),
                    provenance: serde_json::json!({}),
                    created_by: None,
                },
            )
            .expect("shared source claim can support evidence in either project");
        }
    }

    #[test]
    fn model_versions_reject_foreign_project_contribution_and_impact_targets() {
        let mut conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        insert_project(&conn, "project-a");
        insert_project(&conn, "project-b");
        let project_a = create_project_model_fixture(&mut conn, "project-a");
        let project_b = create_project_model_fixture(&mut conn, "project-b");
        let hypothesis_a = insert_hypothesis(&conn, "project-a", "ownership-a");
        let hypothesis_b = insert_hypothesis(&conn, "project-b", "ownership-b");
        let finding_a = insert_finding_version(&conn, "project-a", "ownership-a", &hypothesis_a);
        let finding_b = insert_finding_version(&conn, "project-b", "ownership-b", &hypothesis_b);
        let versions_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM model_versions", [], |row| row.get(0))
            .expect("count model versions before rejected writes");

        for (project_id, local, foreign, foreign_hypothesis, foreign_finding) in [
            (
                "project-a",
                &project_a,
                &project_b,
                &hypothesis_b,
                &finding_b,
            ),
            (
                "project-b",
                &project_b,
                &project_a,
                &hypothesis_a,
                &finding_a,
            ),
        ] {
            let new_version = || {
                let mut request = version_request(project_id, &local.model_id);
                request.change_class = ModelChangeClass::Extension;
                request.parents = vec![ModelParentReq {
                    version_id: local.version_id.clone(),
                    rationale: "Same-project predecessor.".to_string(),
                }];
                request
            };

            let mut request = new_version();
            request.hypotheses.push(ModelRelationReq {
                target_id: foreign_hypothesis.clone(),
                relation_kind: ModelRelationKind::ContributesTo,
                rationale: "Foreign hypothesis must be rejected.".to_string(),
            });
            assert!(create_model_version(&mut conn, &request).is_err());

            let mut request = new_version();
            request.findings.push(ModelRelationReq {
                target_id: foreign_finding.clone(),
                relation_kind: ModelRelationKind::Qualifies,
                rationale: "Foreign finding must be rejected.".to_string(),
            });
            assert!(create_model_version(&mut conn, &request).is_err());

            let mut request = new_version();
            request.impacts = vec![ModelImpactReq {
                target: ModelImpactTarget::HypothesisVersion {
                    hypothesis_version_id: foreign_hypothesis.clone(),
                },
                impact_kind: ModelImpactKind::Added,
                previous_statement: None,
                current_statement: Some("Foreign hypothesis impact.".to_string()),
                rationale: "Foreign project impact must be rejected.".to_string(),
            }];
            assert!(create_model_version(&mut conn, &request).is_err());

            let mut request = new_version();
            request.impacts = vec![ModelImpactReq {
                target: ModelImpactTarget::FindingVersion {
                    finding_version_id: foreign_finding.clone(),
                },
                impact_kind: ModelImpactKind::Added,
                previous_statement: None,
                current_statement: Some("Foreign finding impact.".to_string()),
                rationale: "Foreign project impact must be rejected.".to_string(),
            }];
            assert!(create_model_version(&mut conn, &request).is_err());

            let mut request = new_version();
            request.experiments.push(ModelExperimentRelationReq {
                design_version_id: foreign.experiment_design_version_id.clone(),
                experiment_run_id: Some(foreign.experiment_run_id.clone()),
                relation_kind: "tested_in".to_string(),
                rationale: "Foreign experiment must be rejected.".to_string(),
            });
            assert!(create_model_version(&mut conn, &request).is_err());

            let mut request = new_version();
            request.dataset_uses.push(ModelDatasetUseReq {
                dataset_version_id: foreign.dataset_version_id.clone(),
                role: "test".to_string(),
                split_group_id: None,
                rationale: "Foreign dataset must be rejected.".to_string(),
            });
            assert!(create_model_version(&mut conn, &request).is_err());

            let mut request = new_version();
            request.evidence_ids.push(foreign.evidence_id.clone());
            assert!(create_model_version(&mut conn, &request).is_err());
        }

        let versions_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM model_versions", [], |row| row.get(0))
            .expect("count model versions after rejected writes");
        assert_eq!(versions_after, versions_before);
    }

    #[test]
    fn parent_version_ownership_is_checked_after_model_identity_matches() {
        let mut conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        insert_project(&conn, "project-a");
        insert_project(&conn, "project-b");
        let project_a = create_project_model_fixture(&mut conn, "project-a");
        conn.execute(
            "INSERT INTO model_versions
                (id, model_id, version_no, created_in_project_id, change_class,
                 change_rationale, expected_implications, human_specification,
                 machine_specification_json, scope, dependencies_json, environment_json,
                 content_sha256, created_by, created_at_us)
             VALUES ('foreign-parent-version', ?1, 2, 'project-b', 'revision',
                     'Foreign fixture version.', 'Test parent ownership.',
                     'Test specification.', '{}', 'Test scope', '[]', '{}',
                     ?2, 'test', 2)",
            params![project_a.model_id, "a".repeat(64)],
        )
        .expect("insert foreign-project version for the same model identity");

        let error = ensure_parent_version_ownership(
            &conn,
            "foreign-parent-version",
            &project_a.model_id,
            "project-a",
        )
        .expect_err("a foreign-project version cannot be a parent");
        assert_eq!(
            error.to_string(),
            "Parent versions must belong to the requesting project."
        );
    }

    #[test]
    fn archived_projects_reject_model_lifecycle_and_validation_writes() {
        let mut conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        insert_project(&conn, "project-a");
        insert_project(&conn, "project-b");
        let project_a = create_project_model_fixture(&mut conn, "project-a");
        let model_id: String = conn
            .query_row(
                "SELECT model_id FROM model_versions WHERE id = ?1",
                [&project_a.version_id],
                |row| row.get(0),
            )
            .expect("get project A model");

        transition_version(
            &mut conn,
            &TransitionModelVersionReq {
                version_id: project_a.version_id.clone(),
                next_status: ModelVersionLifecycleStatus::Review,
                rationale: "Active-project transition.".to_string(),
                actor_id: None,
            },
        )
        .expect("active project version transition");
        transition_model(
            &mut conn,
            &TransitionModelReq {
                model_id: model_id.clone(),
                next_status: ModelStatus::Active,
                rationale: "Active-project model transition.".to_string(),
                actor_id: None,
            },
        )
        .expect("active project model transition");

        let validation_request = CreateModelValidationReq {
            project_id: "project-a".to_string(),
            target: ModelValidationTarget::ModelVersion(project_a.version_id.clone()),
            criteria: vec![serde_json::json!({"criterion": "threshold"})],
            scope: "Fixture model".to_string(),
            population_scope: "Project A population".to_string(),
            evaluation_method: "Compare against threshold".to_string(),
            evaluator_identity: "test reviewer".to_string(),
            independence_state: "independent".to_string(),
            independence_rationale: String::new(),
            conflict_of_interest: "None.".to_string(),
            created_by: None,
        };
        let completed_validation = create_validation(&mut conn, &validation_request)
            .expect("create active-project validation");
        finalize_validation(
            &mut conn,
            &FinalizeModelValidationReq {
                validation_id: completed_validation.id,
                outcome: ModelValidationOutcome::Pass,
                rationale: "Active-project finalization.".to_string(),
                deviations: vec![],
                analysis_evidence_ids: vec![],
                model_evidence_ids: vec![project_a.evidence_id.clone()],
                actor_id: None,
            },
        )
        .expect("active project validation finalization");

        let archived_validation = create_validation(&mut conn, &validation_request)
            .expect("create second active-project validation");
        conn.execute(
            "UPDATE research_projects SET lifecycle_status = 'archived' WHERE id = 'project-a'",
            [],
        )
        .expect("archive project A");
        let transition_events_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM model_version_events WHERE model_version_id = ?1",
                [&project_a.version_id],
                |row| row.get(0),
            )
            .expect("count version transition events");
        assert!(transition_version(
            &mut conn,
            &TransitionModelVersionReq {
                version_id: project_a.version_id.clone(),
                next_status: ModelVersionLifecycleStatus::Released,
                rationale: "Must be rejected for archived project.".to_string(),
                actor_id: None,
            },
        )
        .is_err());
        let transition_events_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM model_version_events WHERE model_version_id = ?1",
                [&project_a.version_id],
                |row| row.get(0),
            )
            .expect("count version transition events after rejection");
        assert_eq!(transition_events_after, transition_events_before);

        let model_events_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM model_status_events WHERE model_id = ?1",
                [&model_id],
                |row| row.get(0),
            )
            .expect("count model lifecycle events");
        assert!(transition_model(
            &mut conn,
            &TransitionModelReq {
                model_id: model_id.clone(),
                next_status: ModelStatus::Deprecated,
                rationale: "Must be rejected for archived origin project.".to_string(),
                actor_id: None,
            },
        )
        .is_err());
        let model_events_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM model_status_events WHERE model_id = ?1",
                [&model_id],
                |row| row.get(0),
            )
            .expect("count model lifecycle events after rejection");
        assert_eq!(model_events_after, model_events_before);

        let validation_events_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM model_validation_events WHERE validation_id = ?1",
                [&archived_validation.id],
                |row| row.get(0),
            )
            .expect("count validation result events");
        assert!(finalize_validation(
            &mut conn,
            &FinalizeModelValidationReq {
                validation_id: archived_validation.id.clone(),
                outcome: ModelValidationOutcome::Pass,
                rationale: "Must be rejected for archived project.".to_string(),
                deviations: vec![],
                analysis_evidence_ids: vec![],
                model_evidence_ids: vec![project_a.evidence_id.clone()],
                actor_id: None,
            },
        )
        .is_err());
        let validation_events_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM model_validation_events WHERE validation_id = ?1",
                [&archived_validation.id],
                |row| row.get(0),
            )
            .expect("count validation result events after rejection");
        assert_eq!(validation_events_after, validation_events_before);

        let criteria_json =
            serde_json::json!([{"criterion": "Forged project context."}]).to_string();
        let criteria_sha256 = sha256(criteria_json.as_bytes());
        conn.execute(
            "INSERT INTO model_validations
                (id, project_id, target_model_version_id, target_finding_version_id,
                 target_source_claim_id, criteria_json, criteria_sha256, scope,
                 population_scope, evaluation_method, evaluator_identity,
                 independence_state, independence_rationale, conflict_of_interest,
                 created_by, created_at_us)
             VALUES ('forged-validation', 'project-b', ?1, NULL, NULL, ?2, ?3,
                     'scope', 'population', 'method', 'reviewer', 'independent',
                     '', 'None.', NULL, 1)",
            params![project_a.version_id, criteria_json, criteria_sha256],
        )
        .expect("insert mismatched stored validation context");
        assert!(finalize_validation(
            &mut conn,
            &FinalizeModelValidationReq {
                validation_id: "forged-validation".to_string(),
                outcome: ModelValidationOutcome::Pass,
                rationale: "Mismatched stored project must be rejected.".to_string(),
                deviations: vec![],
                analysis_evidence_ids: vec![],
                model_evidence_ids: vec![project_a.evidence_id],
                actor_id: None,
            },
        )
        .is_err());
        let forged_events: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM model_validation_events
                 WHERE validation_id = 'forged-validation'",
                [],
                |row| row.get(0),
            )
            .expect("count forged validation events");
        assert_eq!(forged_events, 0);
    }

    #[test]
    fn archived_projects_reject_model_creation() {
        let (mut conn, _) = archived_project_model_fixture();
        let error = create_model(&mut conn, &model_request("project-a"))
            .expect_err("archived projects cannot create model identities");
        assert_eq!(
            error.to_string(),
            "Archived research projects cannot be modified."
        );
    }

    #[test]
    fn archived_projects_reject_model_version_creation() {
        let (mut conn, project) = archived_project_model_fixture();
        let mut request = version_request("project-a", &project.model_id);
        request.change_class = ModelChangeClass::Extension;
        request.parents = vec![ModelParentReq {
            version_id: project.version_id,
            rationale: "Existing same-project parent.".to_string(),
        }];
        let error = create_model_version(&mut conn, &request)
            .expect_err("archived projects cannot create model versions");
        assert_eq!(
            error.to_string(),
            "Archived research projects cannot be modified."
        );
    }

    #[test]
    fn archived_projects_reject_evidence_creation() {
        let (mut conn, project) = archived_project_model_fixture();
        let error = create_evidence(
            &mut conn,
            &CreateModelEvidenceReq {
                project_id: "project-a".to_string(),
                model_version_id: project.version_id,
                artifact: ModelArtifactRef::DatasetVersion(project.dataset_version_id),
                polarity: ModelEvidencePolarity::Supports,
                scope: "Archived-project evidence".to_string(),
                quality_rationale: "Valid fixture request.".to_string(),
                limitations: "None.".to_string(),
                provenance: serde_json::json!({}),
                created_by: None,
            },
        )
        .expect_err("archived projects cannot create evidence");
        assert_eq!(
            error.to_string(),
            "Archived research projects cannot be modified."
        );
    }

    #[test]
    fn archived_projects_reject_evaluation_creation() {
        let (mut conn, project) = archived_project_model_fixture();
        let error = create_evaluation(&mut conn, &evaluation_request("project-a", &project))
            .expect_err("archived projects cannot create evaluations");
        assert_eq!(
            error.to_string(),
            "Archived research projects cannot be modified."
        );
    }

    #[test]
    fn archived_projects_reject_validation_creation() {
        let (mut conn, project) = archived_project_model_fixture();
        let error = create_validation(&mut conn, &validation_request("project-a", &project))
            .expect_err("archived projects cannot create validations");
        assert_eq!(
            error.to_string(),
            "Archived research projects cannot be modified."
        );
    }

    #[test]
    fn archived_projects_reject_replication_creation() {
        let (mut conn, project) = archived_project_model_fixture();
        let error = create_replication(&mut conn, &replication_request("project-a", &project))
            .expect_err("archived projects cannot create replications");
        assert_eq!(
            error.to_string(),
            "Archived research projects cannot be modified."
        );
    }

    #[test]
    fn validations_preserve_workspace_shared_source_claim_provenance() {
        let mut conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        insert_project(&conn, "project-a");
        insert_project(&conn, "project-b");
        let claim_id = insert_workspace_source_claim(&conn);
        let base_request = |project_id: &str| CreateModelValidationReq {
            project_id: project_id.to_string(),
            target: ModelValidationTarget::SourceClaim(claim_id.clone()),
            criteria: vec![serde_json::json!({"criterion": "Source attribution retained."})],
            scope: "Shared source claim".to_string(),
            population_scope: "Workspace source context".to_string(),
            evaluation_method: "Compare against source".to_string(),
            evaluator_identity: "test reviewer".to_string(),
            independence_state: "independent".to_string(),
            independence_rationale: String::new(),
            conflict_of_interest: "None.".to_string(),
            created_by: None,
        };

        assert!(create_validation(
            &mut conn,
            &CreateModelValidationReq {
                target: ModelValidationTarget::SourceClaim("forged-claim-id".to_string()),
                ..base_request("project-a")
            },
        )
        .is_err());

        for project_id in ["project-a", "project-b"] {
            let validation = create_validation(&mut conn, &base_request(project_id))
                .expect("create validation for workspace-shared claim");
            let evidence_id = format!("source-claim-evidence-{project_id}");
            conn.execute(
                "INSERT INTO analysis_evidence
                    (id, project_id, target_source_claim_id, artifact_source_claim_id,
                     polarity, scope, quality_rationale, limitations, provenance_json,
                     created_at_us)
                 VALUES (?1, ?2, ?3, ?3, 'supports', 'source scope',
                         'Source provenance retained.', 'Test fixture.', '{}', 1)",
                params![evidence_id, project_id, claim_id],
            )
            .expect("insert source-claim analysis evidence");
            let finalized = finalize_validation(
                &mut conn,
                &FinalizeModelValidationReq {
                    validation_id: validation.id,
                    outcome: ModelValidationOutcome::Pass,
                    rationale: "Workspace-shared source claim validated.".to_string(),
                    deviations: vec![],
                    analysis_evidence_ids: vec![evidence_id],
                    model_evidence_ids: vec![],
                    actor_id: None,
                },
            )
            .expect("finalize validation with shared source provenance");
            assert_eq!(
                finalized.target,
                ModelValidationTarget::SourceClaim(claim_id.clone())
            );
        }
    }

    #[test]
    fn replications_reject_foreign_project_runs_datasets_and_evidence_references() {
        let mut conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        insert_project(&conn, "project-a");
        insert_project(&conn, "project-b");
        let project_a = create_project_model_fixture(&mut conn, "project-a");
        let project_b = create_project_model_fixture(&mut conn, "project-b");
        let request = |original_experiment_run_id: String,
                       dataset_version_id: String,
                       analysis_evidence_ids: Vec<String>| {
            let model_evidence_ids = if analysis_evidence_ids.is_empty() {
                vec![project_a.evidence_id.clone()]
            } else {
                vec![]
            };
            CreateModelReplicationReq {
                project_id: "project-a".to_string(),
                model_version_id: project_a.version_id.clone(),
                original_experiment_run_id,
                replication_kind: "direct".to_string(),
                independent_team_operator: "test team".to_string(),
                protocol_context: "Reference ownership test.".to_string(),
                dataset_context: serde_json::json!({}),
                datasets: vec![ModelReplicationDatasetReq {
                    dataset_version_id,
                    role: "test".to_string(),
                    split_group_id: None,
                }],
                outcome: "supports".to_string(),
                result_summary: "Rejected reference.".to_string(),
                deviations: vec![],
                provenance: serde_json::json!({}),
                analysis_evidence_ids,
                model_evidence_ids,
                created_by: None,
            }
        };
        let hypothesis_a = insert_hypothesis(&conn, "project-a", "replication-a");
        let foreign_dataset_evidence = "evidence-foreign-dataset";
        conn.execute(
            "INSERT INTO analysis_evidence
                (id, project_id, target_hypothesis_version_id, artifact_dataset_version_id,
                 polarity, scope, quality_rationale, limitations, provenance_json, created_at_us)
             VALUES (?1, 'project-a', ?2, ?3, 'supports', 'test scope',
                     'Test rationale.', 'Test limitation.', '{}', 1)",
            params![
                foreign_dataset_evidence,
                hypothesis_a,
                project_b.dataset_version_id
            ],
        )
        .expect("insert legacy evidence with foreign dataset reference");
        let replications_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM model_replications", [], |row| {
                row.get(0)
            })
            .expect("count replications");

        assert!(create_replication(
            &mut conn,
            &request(
                project_b.experiment_run_id.clone(),
                project_a.dataset_version_id.clone(),
                vec![],
            ),
        )
        .is_err());
        assert!(create_replication(
            &mut conn,
            &request(
                project_a.experiment_run_id.clone(),
                project_b.dataset_version_id.clone(),
                vec![],
            ),
        )
        .is_err());
        assert!(create_replication(
            &mut conn,
            &request(
                project_a.experiment_run_id.clone(),
                project_a.dataset_version_id.clone(),
                vec![foreign_dataset_evidence.to_string()],
            ),
        )
        .is_err());

        let replications_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM model_replications", [], |row| {
                row.get(0)
            })
            .expect("count replications after rejection");
        assert_eq!(replications_after, replications_before);
    }

    #[test]
    fn model_versions_evaluations_validation_and_replications_survive_reopen() {
        let path = new_database_path();
        let (
            model_a_id,
            model_b_id,
            initial_version_id,
            project_b_initial_version_id,
            successor_version_id,
            evidence_id,
        ) = {
            let mut conn = Connection::open(&path).expect("open workspace");
            initialize(&conn);
            insert_project(&conn, "project-a");
            insert_project(&conn, "project-b");
            let hypothesis_b = insert_hypothesis(&conn, "project-b", "b");
            let model = create_model(&mut conn, &model_request("project-a")).expect("create model");
            let mut initial_request = version_request("project-a", &model.id);
            initial_request.hypotheses.push(ModelRelationReq {
                target_id: insert_hypothesis(&conn, "project-a", "a"),
                relation_kind: ModelRelationKind::ContributesTo,
                rationale: "Model is considered by this independent hypothesis.".to_string(),
            });
            let initial = create_model_version(&mut conn, &initial_request)
                .expect("create initial revision baseline");
            assert_eq!(initial.version_no, 1);
            assert_eq!(initial.lifecycle_status, ModelVersionLifecycleStatus::Draft);
            assert_ne!(initial.model_id, "hypothesis-a");
            assert_eq!(initial.hypotheses.len(), 1);
            let hypothesis_contribution = initial
                .contributions
                .iter()
                .find(|contribution| {
                    contribution.relation_kind == ModelContributionKind::ContributesTo
                })
                .expect("typed hypothesis contribution");
            assert!(!hypothesis_contribution.id.is_empty());
            assert_eq!(
                hypothesis_contribution.created_by.as_deref(),
                Some("author")
            );
            assert!(matches!(
                &hypothesis_contribution.target,
                ModelContributionTarget::HypothesisVersion { id } if id == &initial.hypotheses[0].target_id
            ));

            transition_version(
                &mut conn,
                &TransitionModelVersionReq {
                    version_id: initial.id.clone(),
                    next_status: ModelVersionLifecycleStatus::Review,
                    rationale: "Reviewed.".to_string(),
                    actor_id: Some("reviewer".to_string()),
                },
            )
            .expect("transition to review");
            transition_version(
                &mut conn,
                &TransitionModelVersionReq {
                    version_id: initial.id.clone(),
                    next_status: ModelVersionLifecycleStatus::Released,
                    rationale: "Release immutable baseline.".to_string(),
                    actor_id: Some("reviewer".to_string()),
                },
            )
            .expect("release baseline");

            let model_b = create_model(&mut conn, &model_request("project-b"))
                .expect("create project B model");
            let project_b_initial =
                create_model_version(&mut conn, &version_request("project-b", &model_b.id))
                    .expect("create project B baseline");
            let dataset_version_id = insert_sealed_dataset(&conn, "project-b");
            let (design_version_id, experiment_run_id) = insert_experiment(&conn, "project-b");
            let (analysis_run_id, analysis_input_id, analysis_metric_id) =
                insert_analysis_run(&conn, "project-b", &dataset_version_id);
            let mut successor_request = version_request("project-b", &model_b.id);
            successor_request.change_class = ModelChangeClass::Replacement;
            successor_request.change_rationale =
                "Replace the baseline following new evidence.".to_string();
            successor_request.parents = vec![ModelParentReq {
                version_id: project_b_initial.id.clone(),
                rationale: "Predecessor retained for comparison.".to_string(),
            }];
            successor_request.impacts = vec![ModelImpactReq {
                target: ModelImpactTarget::Assumption {
                    assumption_key: "scope".to_string(),
                },
                impact_kind: ModelImpactKind::Changed,
                previous_statement: Some("Applies only to the declared population.".to_string()),
                current_statement: Some(
                    "Applies only to the preregistered population.".to_string(),
                ),
                rationale: "Narrowed the applicability scope.".to_string(),
            }];
            successor_request.hypotheses = vec![ModelRelationReq {
                target_id: hypothesis_b,
                relation_kind: ModelRelationKind::Contradicts,
                rationale: "Conflicting hypothesis retained.".to_string(),
            }];
            successor_request.experiments = vec![ModelExperimentRelationReq {
                design_version_id,
                experiment_run_id: Some(experiment_run_id.clone()),
                relation_kind: "tested_in".to_string(),
                rationale: "Exact run assessed this model version.".to_string(),
            }];
            successor_request.dataset_uses = vec![ModelDatasetUseReq {
                dataset_version_id: dataset_version_id.clone(),
                role: "test".to_string(),
                split_group_id: None,
                rationale: "Exact sealed evaluation dataset.".to_string(),
            }];
            let successor = create_model_version(&mut conn, &successor_request)
                .expect("create replacement in project B");
            assert_eq!(successor.version_no, 2);
            assert_eq!(successor.created_in_project_id, "project-b");
            assert_eq!(
                successor.hypotheses[0].relation_kind,
                ModelRelationKind::Contradicts
            );
            assert!(successor.contributions.iter().any(|contribution| {
                contribution.relation_kind == ModelContributionKind::TestedIn
                    && matches!(
                        &contribution.target,
                        ModelContributionTarget::Experiment {
                            experiment_run_id: Some(run_id),
                            ..
                        } if run_id == &experiment_run_id
                    )
            }));
            assert!(successor.contributions.iter().any(|contribution| {
                contribution.relation_kind == ModelContributionKind::DatasetUse
                    && matches!(
                        &contribution.target,
                        ModelContributionTarget::Dataset {
                            dataset_version_id: id,
                            role,
                            ..
                        } if id == &dataset_version_id && role == "test"
                    )
            }));

            let evaluation = create_evaluation(
                &mut conn,
                &CreateModelEvaluationReq {
                    project_id: "project-b".to_string(),
                    model_version_id: successor.id.clone(),
                    analysis_run_id: analysis_run_id.clone(),
                    analysis_run_input_id: analysis_input_id.clone(),
                    dataset_version_id: dataset_version_id.clone(),
                    analysis_metric_id: analysis_metric_id.clone(),
                    role: "test".to_string(),
                    split_group_id: None,
                    limitations: "Single-sample fixture.".to_string(),
                    created_by: Some("evaluator".to_string()),
                },
            )
            .expect("record exact evaluation");
            assert_eq!(
                evaluation.metric_definition_version_id,
                "metric-version-project-b"
            );
            assert_eq!(evaluation.effect_estimate, "0.125");
            assert!(
                create_evaluation(
                    &mut conn,
                    &CreateModelEvaluationReq {
                        role: "training".to_string(),
                        ..CreateModelEvaluationReq {
                            project_id: "project-b".to_string(),
                            model_version_id: successor.id.clone(),
                            analysis_run_id,
                            analysis_run_input_id: analysis_input_id,
                            dataset_version_id: dataset_version_id.clone(),
                            analysis_metric_id,
                            role: "test".to_string(),
                            split_group_id: None,
                            limitations: "fixture".to_string(),
                            created_by: None,
                        }
                    }
                )
                .is_err(),
                "a role that differs from the exact run input is rejected"
            );

            let evidence = create_evidence(
                &mut conn,
                &CreateModelEvidenceReq {
                    project_id: "project-b".to_string(),
                    model_version_id: successor.id.clone(),
                    artifact: ModelArtifactRef::DatasetVersion(dataset_version_id.clone()),
                    polarity: ModelEvidencePolarity::Supports,
                    scope: "Project B evaluation dataset".to_string(),
                    quality_rationale: "The dataset version is immutable and pinned.".to_string(),
                    limitations: "Small fixture.".to_string(),
                    provenance: serde_json::json!({"run": "external capture"}),
                    created_by: Some("reviewer".to_string()),
                },
            )
            .expect("create typed model evidence");
            let validation = create_validation(
                &mut conn,
                &CreateModelValidationReq {
                    project_id: "project-b".to_string(),
                    target: ModelValidationTarget::ModelVersion(successor.id.clone()),
                    criteria: vec![serde_json::json!({"criterion": "Effect remains within declared interval."})],
                    scope: "This metric and exact model version".to_string(),
                    population_scope: "Project B fixture population".to_string(),
                    evaluation_method: "Compare against predeclared interval".to_string(),
                    evaluator_identity: "Independent reviewer".to_string(),
                    independence_state: "independent".to_string(),
                    independence_rationale: String::new(),
                    conflict_of_interest: "None declared.".to_string(),
                    created_by: Some("reviewer".to_string()),
                },
            )
            .expect("create predeclared validation");
            let finalized = finalize_validation(
                &mut conn,
                &FinalizeModelValidationReq {
                    validation_id: validation.id.clone(),
                    outcome: ModelValidationOutcome::Inconclusive,
                    rationale: "The sample is insufficient for a pass/fail conclusion.".to_string(),
                    deviations: vec!["One sample in fixture.".to_string()],
                    analysis_evidence_ids: vec![],
                    model_evidence_ids: vec![evidence.id.clone()],
                    actor_id: Some("reviewer".to_string()),
                },
            )
            .expect("record inconclusive validation with evidence");
            assert_eq!(
                finalized.outcome,
                Some(ModelValidationOutcome::Inconclusive)
            );
            assert!(finalize_validation(
                &mut conn,
                &FinalizeModelValidationReq {
                    validation_id: validation.id,
                    outcome: ModelValidationOutcome::Pass,
                    rationale: "Cannot replace a prior result.".to_string(),
                    deviations: vec![],
                    analysis_evidence_ids: vec![],
                    model_evidence_ids: vec![evidence.id.clone()],
                    actor_id: None,
                }
            )
            .is_err());

            for outcome in ["contradicts", "supports"] {
                create_replication(
                    &mut conn,
                    &CreateModelReplicationReq {
                        project_id: "project-b".to_string(),
                        model_version_id: successor.id.clone(),
                        original_experiment_run_id: experiment_run_id.clone(),
                        replication_kind: if outcome == "supports" {
                            "conceptual".to_string()
                        } else {
                            "direct".to_string()
                        },
                        independent_team_operator: "Independent team".to_string(),
                        protocol_context: "Protocol compared to original design.".to_string(),
                        dataset_context: serde_json::json!({"population": "Project B"}),
                        datasets: vec![ModelReplicationDatasetReq {
                            dataset_version_id: dataset_version_id.clone(),
                            role: "test".to_string(),
                            split_group_id: None,
                        }],
                        outcome: outcome.to_string(),
                        result_summary: format!("Reported {outcome} result."),
                        deviations: vec![],
                        provenance: serde_json::json!({"replication": true}),
                        analysis_evidence_ids: vec![],
                        model_evidence_ids: vec![evidence.id.clone()],
                        created_by: Some("operator".to_string()),
                    },
                )
                .expect("retain replication result");
            }
            assert_eq!(
                query_replications(&conn, "project-b")
                    .expect("replications")
                    .len(),
                2
            );

            let mut later_request = version_request("project-b", &model_b.id);
            later_request.change_class = ModelChangeClass::Extension;
            later_request.change_rationale =
                "Add a capability after the recorded evaluation.".to_string();
            later_request.parents = vec![ModelParentReq {
                version_id: successor.id.clone(),
                rationale: "Preserve the evaluated predecessor.".to_string(),
            }];
            later_request.impacts = vec![ModelImpactReq {
                target: ModelImpactTarget::Assumption {
                    assumption_key: "scope".to_string(),
                },
                impact_kind: ModelImpactKind::Changed,
                previous_statement: Some("Applies only to the declared population.".to_string()),
                current_statement: Some(
                    "Applies to the declared population and its extension.".to_string(),
                ),
                rationale: "Record the declared scope extension.".to_string(),
            }];
            create_model_version(&mut conn, &later_request)
                .expect("create a later immutable version");

            let legacy_hypothesis_id = "legacy-hypothesis";
            conn.execute(
                "INSERT INTO research_hypotheses
                    (id, project_id, research_question_id, legacy_hypothesis_id,
                     parent_hypothesis_id, ownership_state, lifecycle_status, created_at_us)
                 VALUES (?1, 'project-a', NULL, NULL, NULL, 'project', 'legacy_untyped', 1)",
                [legacy_hypothesis_id],
            )
            .expect("insert legacy hypothesis");
            conn.execute(
                "INSERT INTO hypothesis_model_references
                    (id, hypothesis_id, model_id, model_version_id, label, rationale,
                     resolution_status, created_at_us)
                 VALUES ('legacy-ref', ?1, 'unresolved-model', 'unresolved-version',
                         'legacy note', 'not migrated speculatively', 'unresolved', 1)",
                [legacy_hypothesis_id],
            )
            .expect("insert unresolved legacy model link");
            conn.execute(
                "UPDATE model_version_parameters SET value = 'changed' WHERE model_version_id = ?1",
                [&initial.id],
            )
            .expect_err("model parameter history must be immutable");
            conn.execute("DELETE FROM model_versions WHERE id = ?1", [&initial.id])
                .expect_err("predecessor model version must not be deleted");
            (
                model.id,
                model_b.id,
                initial.id,
                project_b_initial.id,
                successor.id,
                evidence.id,
            )
        };

        {
            let conn = Connection::open(&path).expect("reopen workspace");
            initialize(&conn);
            let project_a_state =
                workspace(&conn, "project-a").expect("load project-a workspace after restart");
            assert_eq!(project_a_state.models.len(), 1);
            assert!(project_a_state
                .models
                .iter()
                .any(|model| model.id == model_a_id));
            assert_eq!(project_a_state.versions.len(), 1);
            assert!(project_a_state
                .versions
                .iter()
                .all(|version| version.created_in_project_id == "project-a"));
            assert!(project_a_state
                .versions
                .iter()
                .any(|version| version.id == initial_version_id));
            assert!(project_a_state.evidence.is_empty());
            assert!(project_a_state.evaluations.is_empty());
            assert!(project_a_state.validations.is_empty());
            assert!(project_a_state.replications.is_empty());

            let state = workspace(&conn, "project-b").expect("load model workspace after restart");
            assert_eq!(state.models.len(), 1);
            assert!(state.models.iter().any(|model| model.id == model_b_id));
            assert!(state.models.iter().all(|model| model.id != model_a_id));
            assert_eq!(state.versions.len(), 3);
            assert!(state
                .versions
                .iter()
                .all(|version| version.created_in_project_id == "project-b"));
            assert!(!state
                .versions
                .iter()
                .any(|version| version.id == initial_version_id));
            assert!(state
                .versions
                .iter()
                .any(|version| version.id == project_b_initial_version_id));
            assert!(state.versions.iter().any(|version| {
                version.id == successor_version_id
                    && version.parents[0].version_id == project_b_initial_version_id
            }));
            assert!(state
                .evidence
                .iter()
                .any(|evidence| evidence.id == evidence_id));
            assert_eq!(state.evaluations.len(), 1);
            assert_eq!(state.evaluations[0].model_version_id, successor_version_id);
            assert_eq!(state.validations.len(), 1);
            assert_eq!(state.replications.len(), 2);
            assert!(state
                .replications
                .iter()
                .any(|replication| replication.outcome == "supports"));
            assert!(state
                .replications
                .iter()
                .any(|replication| replication.outcome == "contradicts"));
            let legacy: (String, String, String) = conn
                .query_row(
                    "SELECT model_id, model_version_id, resolution_status
                     FROM hypothesis_model_references WHERE id = 'legacy-ref'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("legacy unresolved reference remains");
            assert_eq!(
                legacy,
                (
                    "unresolved-model".to_string(),
                    "unresolved-version".to_string(),
                    "unresolved".to_string()
                )
            );
            let foreign_key_violations: i64 = conn
                .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                    row.get(0)
                })
                .expect("check foreign keys");
            assert_eq!(foreign_key_violations, 0);
        }
        fs::remove_file(path).expect("remove test database");
    }

    #[test]
    fn initial_version_requires_revision_and_explicit_impact() {
        let mut conn = Connection::open_in_memory().expect("open in-memory database");
        initialize(&conn);
        insert_project(&conn, "project");
        let model = create_model(&mut conn, &model_request("project")).expect("create model");
        let mut request = version_request("project", &model.id);
        request.change_class = ModelChangeClass::Extension;
        assert!(create_model_version(&mut conn, &request).is_err());
        request.change_class = ModelChangeClass::Replacement;
        assert!(create_model_version(&mut conn, &request).is_err());
        request.change_class = ModelChangeClass::Revision;
        request.impacts.clear();
        assert!(create_model_version(&mut conn, &request).is_err());
        request.impacts = vec![ModelImpactReq {
            target: ModelImpactTarget::Assumption {
                assumption_key: "scope".to_string(),
            },
            impact_kind: ModelImpactKind::Changed,
            previous_statement: None,
            current_statement: Some("New assumption".to_string()),
            rationale: "Invalid without previous value.".to_string(),
        }];
        assert!(create_model_version(&mut conn, &request).is_err());

        request = version_request("project", &model.id);
        let version = create_model_version(&mut conn, &request).expect("create baseline");
        let mut validation_request = CreateModelValidationReq {
            project_id: "project".to_string(),
            target: ModelValidationTarget::ModelVersion(version.id),
            criteria: vec![],
            scope: "scope".to_string(),
            population_scope: "population".to_string(),
            evaluation_method: "method".to_string(),
            evaluator_identity: "reviewer".to_string(),
            independence_state: "unknown".to_string(),
            independence_rationale: String::new(),
            conflict_of_interest: "None declared.".to_string(),
            created_by: None,
        };
        assert!(create_validation(&mut conn, &validation_request).is_err());
        validation_request.criteria = vec![serde_json::json!({"criterion": "threshold"})];
        validation_request.conflict_of_interest.clear();
        assert!(create_validation(&mut conn, &validation_request).is_err());
        validation_request.conflict_of_interest = "None declared.".to_string();
        assert!(create_validation(&mut conn, &validation_request).is_err());
    }
}
