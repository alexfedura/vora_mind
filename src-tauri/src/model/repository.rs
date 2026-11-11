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
        created_by: row.get(6)?,
        created_at_us: row.get(7)?,
    })
}

pub fn query_model(conn: &Connection, model_id: &str) -> Result<ModelDto> {
    Ok(conn.query_row(
        "SELECT m.id, m.display_name, m.description, m.origin_project_id, m.domain_id,
                COALESCE((SELECT e.status FROM model_status_events e
                          WHERE e.model_id = m.id ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1),
                         'proposed'),
                m.created_by, m.created_at_us
         FROM scientific_models m WHERE m.id = ?1",
        [model_id],
        model_from_row,
    )?)
}

pub fn create_model_version(
    conn: &mut Connection,
    request: &CreateModelVersionReq,
) -> Result<ModelVersionDto> {
    validate_version_request(conn, request)?;
    let model_version_id = id();
    let created_at = now_us()?;
    let serialized = serde_json::to_vec(request)?;
    let content_sha256 = sha256(&serialized);

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version_no: i64 = tx.query_row(
        "SELECT COALESCE(MAX(version_no), 0) + 1 FROM model_versions WHERE model_id = ?1",
        [&request.model_id],
        |row| row.get(0),
    )?;
    tx.execute(
        "INSERT INTO model_versions
            (id, model_id, version_no, created_in_project_id, change_class,
             change_rationale, expected_implications, human_specification,
             machine_specification_json, assumptions_json, scope, dependencies_json,
             environment_json, content_sha256, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
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
            json(&request.assumptions)?,
            request.scope.trim(),
            json(&request.dependencies)?,
            json(&request.environment)?,
            content_sha256,
            request.created_by,
            created_at
        ],
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
            "INSERT INTO model_version_hypotheses
                (model_version_id, hypothesis_version_id, relation_kind, rationale)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                model_version_id,
                relation.target_id,
                relation.relation_kind.as_str(),
                relation.rationale.trim()
            ],
        )?;
    }
    for relation in &request.findings {
        if relation.relation_kind == ModelRelationKind::TestedIn {
            return Err(anyhow!("A finding cannot use the tested_in relation."));
        }
        tx.execute(
            "INSERT INTO model_version_findings
                (model_version_id, finding_version_id, relation_kind, rationale)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                model_version_id,
                relation.target_id,
                relation.relation_kind.as_str(),
                relation.rationale.trim()
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
            return Err(anyhow!("A tested_in relation requires an exact experiment run."));
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
            "INSERT INTO model_version_experiments
                (id, model_version_id, design_version_id, experiment_run_id, relation_kind, rationale)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id(),
                model_version_id,
                relation.design_version_id,
                relation.experiment_run_id,
                relation.relation_kind,
                relation.rationale.trim()
            ],
        )?;
    }
    for use_record in &request.dataset_uses {
        validate_dataset_role(&use_record.role)?;
        required(&use_record.rationale, "Dataset-use rationale")?;
        let sealed: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM experiment_dataset_versions
             WHERE id = ?1 AND lifecycle_status = 'sealed')",
            [&use_record.dataset_version_id],
            |row| row.get(0),
        )?;
        if !sealed {
            return Err(anyhow!("Model dataset references must pin a sealed version."));
        }
        tx.execute(
            "INSERT INTO model_version_dataset_uses
                (id, model_version_id, dataset_version_id, role, split_group_id, rationale)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id(),
                model_version_id,
                use_record.dataset_version_id,
                use_record.role,
                use_record.split_group_id,
                use_record.rationale.trim()
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
        tx.execute(
            "INSERT INTO model_version_evidence (model_version_id, evidence_id, rationale)
             VALUES (?1, ?2, 'Referenced during immutable model-version creation.')",
            params![model_version_id, evidence_id],
        )?;
    }

    tx.commit()?;
    query_version(conn, &model_version_id)
}

fn validate_version_request(conn: &Connection, request: &CreateModelVersionReq) -> Result<()> {
    exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
        &request.project_id,
        "Research project was not found.",
    )?;
    exists(
        conn,
        "SELECT EXISTS(SELECT 1 FROM scientific_models WHERE id = ?1)",
        &request.model_id,
        "Model identity was not found.",
    )?;
    for (value, label) in [
        (&request.change_rationale, "Change rationale"),
        (&request.expected_implications, "Expected implications"),
        (&request.specification.human_readable, "Human-readable model specification"),
        (&request.scope, "Model scope"),
    ] {
        required(value, label)?;
    }
    if !request.specification.machine_readable.is_object() {
        return Err(anyhow!("Machine-readable model specification must be a JSON object."));
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
        let parent_model: Option<String> = conn
            .query_row(
                "SELECT model_id FROM model_versions WHERE id = ?1",
                [&parent.version_id],
                |row| row.get(0),
            )
            .optional()?;
        if parent_model.as_deref() != Some(request.model_id.as_str()) {
            return Err(anyhow!(
                "Parent versions must belong to the same model identity."
            ));
        }
    }
    for assumption in &request.assumptions {
        required(&assumption.key, "Assumption key")?;
        required(&assumption.statement, "Assumption statement")?;
    }
    for impact in &request.impacts {
        required(&impact.rationale, "Impact rationale")?;
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
            ModelImpactTarget::SourceClaim { claim_id } => exists(
                conn,
                "SELECT EXISTS(SELECT 1 FROM source_claims WHERE id = ?1)",
                claim_id,
                "Affected source claim was not found.",
            )?,
            ModelImpactTarget::HypothesisVersion {
                hypothesis_version_id,
            } => exists(
                conn,
                "SELECT EXISTS(SELECT 1 FROM hypothesis_definition_versions WHERE id = ?1)",
                hypothesis_version_id,
                "Affected hypothesis version was not found.",
            )?,
            ModelImpactTarget::FindingVersion { finding_version_id } => exists(
                conn,
                "SELECT EXISTS(SELECT 1 FROM analysis_finding_versions WHERE id = ?1)",
                finding_version_id,
                "Affected finding version was not found.",
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
    for relation in &request.experiments {
        required(&relation.rationale, "Experiment relation rationale")?;
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
            ModelImpactTarget::Assumption { assumption_key } => {
                ("assumption", None, None, None, Some(assumption_key.as_str()))
            }
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
                machine_specification_json, assumptions_json, scope, dependencies_json,
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
                row.get::<_, String>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, i64>(15)?,
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

    let dependencies = conn.query_row(
        "SELECT dependencies_json FROM model_versions WHERE id = ?1",
        [version_id],
        |row| parse_json(&row.get::<_, String>(0)?),
    )?;
    let machine_readable = conn.query_row(
        "SELECT machine_specification_json FROM model_versions WHERE id = ?1",
        [version_id],
        |row| parse_json(&row.get::<_, String>(0)?),
    )?;
    let assumption_values = row.9;
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
        assumptions: parse_json(&assumption_values)?,
        scope: row.10,
        dependencies,
        environment: parse_json(&row.12)?,
        parents,
        parameters,
        artifacts,
        impacts: query_impacts(conn, version_id)?,
        content_sha256: row.13,
        created_by: row.14,
        created_at_us: row.15,
    })
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
                    assumption_key: row.get::<_, Option<String>>(5)?.ok_or(
                        rusqlite::Error::InvalidQuery,
                    )?,
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

fn query_all_models(conn: &Connection) -> Result<Vec<ModelDto>> {
    let mut statement = conn.prepare("SELECT id FROM scientific_models ORDER BY display_name, id")?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.iter().map(|model_id| query_model(conn, model_id)).collect()
}

fn query_all_versions(conn: &Connection) -> Result<Vec<ModelVersionDto>> {
    let mut statement = conn.prepare(
        "SELECT id FROM model_versions ORDER BY model_id, version_no",
    )?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.iter().map(|version_id| query_version(conn, version_id)).collect()
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
        "SELECT EXISTS(SELECT 1 FROM model_versions WHERE id = ?1)",
        &request.model_version_id,
        "Evidence target model version was not found.",
    )?;
    let evidence_id = id();
    let created_at = now_us()?;
    let artifact_columns = match &request.artifact {
        ModelArtifactRef::AnalysisOutput(value) => (Some(value.as_str()), None, None, None, None, None, None),
        ModelArtifactRef::ExperimentRun(value) => (None, Some(value.as_str()), None, None, None, None, None),
        ModelArtifactRef::DatasetVersion(value) => (None, None, Some(value.as_str()), None, None, None, None),
        ModelArtifactRef::ResearchObservation(value) => (None, None, None, Some(value.as_str()), None, None, None),
        ModelArtifactRef::SourceVersionSegment(value) => (None, None, None, None, Some(value.as_str()), None, None),
        ModelArtifactRef::SourceClaim(value) => (None, None, None, None, None, Some(value.as_str()), None),
        ModelArtifactRef::FindingVersion(value) => (None, None, None, None, None, None, Some(value.as_str())),
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
        (Some(value), None, None, None, None, None, None) => ModelArtifactRef::AnalysisOutput(value),
        (None, Some(value), None, None, None, None, None) => ModelArtifactRef::ExperimentRun(value),
        (None, None, Some(value), None, None, None, None) => ModelArtifactRef::DatasetVersion(value),
        (None, None, None, Some(value), None, None, None) => ModelArtifactRef::ResearchObservation(value),
        (None, None, None, None, Some(value), None, None) => ModelArtifactRef::SourceVersionSegment(value),
        (None, None, None, None, None, Some(value), None) => ModelArtifactRef::SourceClaim(value),
        (None, None, None, None, None, None, Some(value)) => ModelArtifactRef::FindingVersion(value),
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
        return Err(anyhow!("Analysis run or its lifecycle status was not found."));
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
        return Err(anyhow!("Model evaluation requires a sealed DatasetVersion."));
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
    ids.iter().map(|evaluation_id| query_evaluation(conn, evaluation_id)).collect()
}
