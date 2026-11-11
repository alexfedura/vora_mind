use super::*;
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const ANALYSIS_SCHEMA: &str = include_str!("schema.sql");
const ANALYSIS_MIGRATION_ID: &str = "phase4-analysis-001";

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

fn parse_json<T: for<'de> Deserialize<'de>>(value: &str) -> Result<T> {
    Ok(serde_json::from_str(value)?)
}

fn nonempty(value: &str, name: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{name} must not be empty"));
    }
    Ok(())
}

fn valid_sha256(value: &str, name: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(anyhow!("{name} must be a 64-character SHA-256 digest"));
    }
    Ok(())
}

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS analysis_schema_migrations (
             version INTEGER PRIMARY KEY,
             migration_id TEXT NOT NULL UNIQUE,
             checksum TEXT NOT NULL,
             applied_at_us INTEGER NOT NULL
         );",
    )?;
    let foreign_keys_enabled: i64 =
        conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    if foreign_keys_enabled != 1 {
        return Err(anyhow!(
            "SQLite foreign-key enforcement is required for Analysis writes."
        ));
    }

    let checksum = sha256(ANALYSIS_SCHEMA.as_bytes());
    conn.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| -> Result<()> {
        let applied: Option<(String, String)> = conn
            .query_row(
                "SELECT migration_id, checksum
                 FROM analysis_schema_migrations WHERE version = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((migration_id, applied_checksum)) = applied {
            if migration_id != ANALYSIS_MIGRATION_ID || applied_checksum != checksum {
                return Err(anyhow!(
                    "Analysis migration 1 differs from the applied migration; add a new numbered migration."
                ));
            }
            return Ok(());
        }

        conn.execute_batch(ANALYSIS_SCHEMA)?;
        conn.execute(
            "INSERT INTO analysis_schema_migrations
                (version, migration_id, checksum, applied_at_us)
             VALUES (1, ?1, ?2, ?3)",
            params![ANALYSIS_MIGRATION_ID, checksum, now_us()?],
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

fn validate_verification_report(
    total: i64,
    passed: i64,
    suite_uri: &str,
    suite_sha256: &str,
) -> Result<AnalysisVerificationStatus> {
    nonempty(suite_uri, "Test suite URI")?;
    valid_sha256(suite_sha256, "Test suite checksum")?;
    if total <= 0 || passed < 0 || passed > total {
        return Err(anyhow!(
            "Test counts must satisfy 0 <= passed <= total and total > 0."
        ));
    }
    Ok(if total == passed {
        AnalysisVerificationStatus::Passed
    } else {
        AnalysisVerificationStatus::Failed
    })
}

fn method_version_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AnalysisMethodVersionDto> {
    let verification_status: String = row.get(13)?;
    Ok(AnalysisMethodVersionDto {
        id: row.get(0)?,
        method_id: row.get(1)?,
        project_id: row.get(2)?,
        name: row.get(3)?,
        version_no: row.get(4)?,
        version_label: row.get(5)?,
        code_uri: row.get(6)?,
        code_revision: row.get(7)?,
        dependency_lock_sha256: row.get(8)?,
        environment: serde_json::from_str(&row.get::<_, String>(9)?).map_err(|_| {
            rusqlite::Error::InvalidColumnType(9, "environment_json".into(), rusqlite::types::Type::Text)
        })?,
        assumptions: serde_json::from_str(&row.get::<_, String>(10)?).map_err(|_| {
            rusqlite::Error::InvalidColumnType(10, "assumptions_json".into(), rusqlite::types::Type::Text)
        })?,
        test_suite_uri: row.get(11)?,
        test_suite_sha256: row.get(12)?,
        tests_total: row.get(14)?,
        tests_passed: row.get(15)?,
        verification_status: AnalysisVerificationStatus::parse(&verification_status)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        content_sha256: row.get(16)?,
        created_by: row.get(17)?,
        created_at_us: row.get(18)?,
    })
}

fn query_method_version(conn: &Connection, version_id: &str) -> Result<AnalysisMethodVersionDto> {
    Ok(conn.query_row(
        "SELECT v.id, m.id, m.project_id, m.name, v.version_no, v.version_label,
                v.code_uri, v.code_revision, v.dependency_lock_sha256,
                v.environment_json, v.assumptions_json, v.test_suite_uri,
                v.test_suite_sha256, v.verification_status, v.tests_total,
                v.tests_passed, v.content_sha256, v.created_by, v.created_at_us
         FROM analysis_method_versions v
         JOIN analysis_methods m ON m.id = v.method_id
         WHERE v.id = ?1",
        [version_id],
        method_version_from_row,
    )?)
}

pub fn create_method_version(
    conn: &mut Connection,
    req: &CreateAnalysisMethodVersionReq,
) -> Result<AnalysisMethodVersionDto> {
    nonempty(&req.name, "Method name")?;
    nonempty(&req.version_label, "Method version label")?;
    nonempty(&req.code_uri, "Method code URI")?;
    nonempty(&req.code_revision, "Method code revision")?;
    if !req.environment.is_object() {
        return Err(anyhow!("Method environment must be a JSON object."));
    }
    if req.assumptions.iter().any(|value| value.trim().is_empty()) {
        return Err(anyhow!("Method assumptions must not contain empty entries."));
    }
    if let Some(checksum) = req.dependency_lock_sha256.as_deref() {
        valid_sha256(checksum, "Dependency lock checksum")?;
    }
    let status = validate_verification_report(
        req.tests_total,
        req.tests_passed,
        &req.test_suite_uri,
        &req.test_suite_sha256,
    )?;
    let now = now_us()?;
    let version_id = id();
    let method_id = req.method_id.clone().unwrap_or_else(id);
    let content = json(req)?;
    let content_sha256 = sha256(content.as_bytes());
    let tx = conn.transaction()?;

    if let Some(existing_id) = req.method_id.as_deref() {
        let existing: Option<(String, String)> = tx
            .query_row(
                "SELECT project_id, name FROM analysis_methods WHERE id = ?1",
                [existing_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        match existing {
            Some((project_id, name)) if project_id == req.project_id && name == req.name => {}
            Some(_) => return Err(anyhow!("The method identity does not match this project and name.")),
            None => return Err(anyhow!("Analysis method was not found.")),
        }
    } else {
        tx.execute(
            "INSERT INTO analysis_methods (id, project_id, name, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![method_id, req.project_id, req.name.trim(), req.created_by, now],
        )?;
    }

    let version_no: i64 = tx.query_row(
        "SELECT COALESCE(MAX(version_no), 0) + 1
         FROM analysis_method_versions WHERE method_id = ?1",
        [&method_id],
        |row| row.get(0),
    )?;
    tx.execute(
        "INSERT INTO analysis_method_versions
            (id, method_id, version_no, version_label, code_uri, code_revision,
             dependency_lock_sha256, environment_json, assumptions_json,
             test_suite_uri, test_suite_sha256, tests_total, tests_passed,
             verification_status, content_sha256, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            version_id,
            method_id,
            version_no,
            req.version_label.trim(),
            req.code_uri.trim(),
            req.code_revision.trim(),
            req.dependency_lock_sha256,
            json(&req.environment)?,
            json(&req.assumptions)?,
            req.test_suite_uri.trim(),
            req.test_suite_sha256.to_ascii_lowercase(),
            req.tests_total,
            req.tests_passed,
            status.as_str(),
            content_sha256,
            req.created_by,
            now
        ],
    )?;
    tx.commit()?;
    query_method_version(conn, &version_id)
}

fn metric_version_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MetricDefinitionVersionDto> {
    let verification_status: String = row.get(12)?;
    Ok(MetricDefinitionVersionDto {
        id: row.get(0)?,
        metric_definition_id: row.get(1)?,
        project_id: row.get(2)?,
        key: row.get(3)?,
        name: row.get(4)?,
        version_no: row.get(5)?,
        version_label: row.get(6)?,
        formula: row.get(7)?,
        unit: row.get(8)?,
        test_suite_uri: row.get(9)?,
        test_suite_sha256: row.get(10)?,
        tests_total: row.get(13)?,
        tests_passed: row.get(14)?,
        verification_status: AnalysisVerificationStatus::parse(&verification_status)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        content_sha256: row.get(15)?,
        created_by: row.get(16)?,
        created_at_us: row.get(17)?,
    })
}

fn query_metric_version(conn: &Connection, version_id: &str) -> Result<MetricDefinitionVersionDto> {
    Ok(conn.query_row(
        "SELECT v.id, m.id, m.project_id, m.metric_key, m.name, v.version_no,
                v.version_label, v.formula, v.unit, v.test_suite_uri,
                v.test_suite_sha256, v.verification_status, v.tests_total,
                v.tests_passed, v.content_sha256, v.created_by, v.created_at_us,
                v.id
         FROM analysis_metric_definition_versions v
         JOIN analysis_metric_definitions m ON m.id = v.metric_definition_id
         WHERE v.id = ?1",
        [version_id],
        |row| {
            let mut dto = metric_version_from_row(row)?;
            dto.version_no = row.get(5)?;
            Ok(dto)
        },
    )?)
}

pub fn create_metric_definition_version(
    conn: &mut Connection,
    req: &CreateMetricDefinitionVersionReq,
) -> Result<MetricDefinitionVersionDto> {
    nonempty(&req.key, "Metric key")?;
    nonempty(&req.name, "Metric name")?;
    nonempty(&req.version_label, "Metric version label")?;
    nonempty(&req.formula, "Metric formula")?;
    nonempty(&req.unit, "Metric unit")?;
    let status = validate_verification_report(
        req.tests_total,
        req.tests_passed,
        &req.test_suite_uri,
        &req.test_suite_sha256,
    )?;
    let now = now_us()?;
    let metric_definition_id = req.metric_definition_id.clone().unwrap_or_else(id);
    let version_id = id();
    let content_sha256 = sha256(json(req)?.as_bytes());
    let tx = conn.transaction()?;
    if let Some(existing_id) = req.metric_definition_id.as_deref() {
        let existing: Option<(String, String, String)> = tx
            .query_row(
                "SELECT project_id, metric_key, name
                 FROM analysis_metric_definitions WHERE id = ?1",
                [existing_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        match existing {
            Some((project_id, key, name))
                if project_id == req.project_id && key == req.key && name == req.name => {}
            Some(_) => return Err(anyhow!("The metric identity does not match this project and key.")),
            None => return Err(anyhow!("Metric definition was not found.")),
        }
    } else {
        tx.execute(
            "INSERT INTO analysis_metric_definitions
                (id, project_id, metric_key, name, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                metric_definition_id,
                req.project_id,
                req.key.trim(),
                req.name.trim(),
                req.created_by,
                now
            ],
        )?;
    }
    let version_no: i64 = tx.query_row(
        "SELECT COALESCE(MAX(version_no), 0) + 1
         FROM analysis_metric_definition_versions WHERE metric_definition_id = ?1",
        [&metric_definition_id],
        |row| row.get(0),
    )?;
    tx.execute(
        "INSERT INTO analysis_metric_definition_versions
            (id, metric_definition_id, version_no, version_label, formula, unit,
             test_suite_uri, test_suite_sha256, tests_total, tests_passed,
             verification_status, content_sha256, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            version_id,
            metric_definition_id,
            version_no,
            req.version_label.trim(),
            req.formula.trim(),
            req.unit.trim(),
            req.test_suite_uri.trim(),
            req.test_suite_sha256.to_ascii_lowercase(),
            req.tests_total,
            req.tests_passed,
            status.as_str(),
            content_sha256,
            req.created_by,
            now
        ],
    )?;
    tx.commit()?;
    query_metric_version(conn, &version_id)
}

fn query_definition_version(
    conn: &Connection,
    version_id: &str,
) -> Result<AnalysisDefinitionVersionDto> {
    let base: (
        String,
        String,
        String,
        i64,
        String,
        Option<String>,
        Option<String>,
        String,
        String,
        String,
        Option<String>,
        i64,
        Option<i64>,
    ) = conn.query_row(
        "SELECT v.id, d.id, d.project_id, v.version_no, d.title, v.version_label,
                v.parent_version_id, v.change_rationale, v.method_version_id,
                v.specification_json, v.specification_sha256, v.created_by,
                v.created_at_us
         FROM analysis_definition_versions v
         JOIN analysis_definitions d ON d.id = v.definition_id
         WHERE v.id = ?1",
        [version_id],
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
                row.get(9)?,
                row.get(10)?,
                row.get(11)?,
                row.get(12)?,
            ))
        },
    )?;
    let metric_version_ids = {
        let mut statement = conn.prepare(
            "SELECT metric_definition_version_id
             FROM analysis_definition_metric_versions
             WHERE definition_version_id = ?1
             ORDER BY metric_definition_version_id",
        )?;
        statement
            .query_map([version_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    let status: String = conn.query_row(
        "SELECT status FROM analysis_definition_events
         WHERE definition_version_id = ?1
         ORDER BY occurred_at_us DESC, id DESC LIMIT 1",
        [version_id],
        |row| row.get(0),
    )?;
    let locked_at_us: Option<i64> = conn
        .query_row(
            "SELECT occurred_at_us FROM analysis_definition_events
             WHERE definition_version_id = ?1 AND status = 'locked'
             ORDER BY occurred_at_us DESC, id DESC LIMIT 1",
            [version_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(AnalysisDefinitionVersionDto {
        id: base.0,
        definition_id: base.1,
        project_id: base.2,
        version_no: base.3,
        title: base.4,
        version_label: base.5.ok_or_else(|| anyhow!("Analysis version label is missing."))?,
        parent_version_id: base.6,
        change_rationale: base.7,
        method_version_id: base.8,
        specification: parse_json(&base.9)?,
        specification_sha256: base.10,
        lifecycle_status: AnalysisDefinitionStatus::parse(&status).map_err(anyhow::Error::msg)?,
        metric_definition_version_ids: metric_version_ids,
        created_by: base.11,
        created_at_us: base.12,
        locked_at_us,
    })
}

fn validate_required_text(value: &str, field: &str) -> Result<()> {
    nonempty(value, field)
}

fn validate_applicability<T>(
    category: &str,
    control: &Applicability<T>,
    check_details: impl FnOnce(&T) -> Result<()>,
) -> Result<()> {
    match control {
        Applicability::Applicable { details } => check_details(details)?,
        Applicability::NotApplicable { reason } => nonempty(reason, &format!("{category} reason"))?,
        Applicability::NotAssessed { rationale } => {
            if rationale.as_deref().is_some_and(|value| value.trim().is_empty()) {
                return Err(anyhow!("{category} rationale must not be empty when supplied."));
            }
        }
    }
    Ok(())
}

fn validate_integrity_plan(plan: &StatisticalIntegrityPlanDto) -> Result<()> {
    validate_applicability("Overfitting", &plan.overfitting, |details| {
        validate_required_text(&details.training_tuning_separation, "Training/tuning separation")?;
        validate_required_text(&details.parameter_search_space, "Parameter search space")?;
        validate_required_text(&details.model_selection_process, "Model selection process")?;
        validate_required_text(&details.complexity_reporting, "Complexity reporting")?;
        validate_required_text(&details.sensitivity_reporting, "Sensitivity reporting")
    })?;
    validate_applicability("Data leakage", &plan.data_leakage, |details| {
        validate_required_text(&details.data_cutoffs, "Data cutoffs")?;
        validate_required_text(&details.transformation_order, "Transformation order")?;
        validate_required_text(&details.feature_selection_provenance, "Feature-selection provenance")?;
        validate_required_text(&details.evaluation_data_isolation, "Evaluation-data isolation")
    })?;
    validate_applicability("Selection bias", &plan.selection_bias, |details| {
        validate_required_text(&details.sampling_frame, "Sampling frame")?;
        validate_required_text(&details.inclusion_rules, "Inclusion rules")?;
        validate_required_text(&details.exclusion_rules, "Exclusion rules")?;
        validate_required_text(&details.filtered_and_missing_cases, "Filtered/missing cases")?;
        validate_required_text(&details.survivor_population, "Survivor population")
    })?;
    validate_applicability("Look-ahead", &plan.look_ahead, |details| {
        validate_required_text(&details.feature_availability_policy, "Feature-availability policy")?;
        validate_required_text(&details.prediction_timestamp_policy, "Prediction timestamp policy")?;
        validate_required_text(&details.decision_timestamp_policy, "Decision timestamp policy")
    })?;
    validate_applicability("Multiple testing", &plan.multiple_testing, |details| {
        nonempty(&details.test_family, "Test family")?;
        match (
            details.correction_method.as_deref(),
            details.no_correction_justification.as_deref(),
        ) {
            (Some(method), None) => nonempty(method, "Multiplicity correction method"),
            (None, Some(reason)) => nonempty(reason, "No-correction justification"),
            _ => Err(anyhow!(
                "Multiple-testing control requires exactly one correction method or no-correction justification."
            )),
        }
    })?;
    validate_applicability("Dataset separation", &plan.dataset_separation, |details| {
        nonempty(&details.selection_rationale, "Dataset-role selection rationale")
    })?;
    validate_applicability("Out-of-sample", &plan.out_of_sample, |details| {
        nonempty(&details.evaluation_period, "Evaluation period")?;
        nonempty(&details.evaluation_population, "Evaluation population")
    })?;
    validate_applicability("Walk-forward", &plan.walk_forward, |details| {
        if details.windows.is_empty() {
            return Err(anyhow!("Walk-forward control requires at least one declared window."));
        }
        nonempty(&details.refit_cadence, "Walk-forward refit cadence")?;
        nonempty(&details.aggregation_method, "Walk-forward aggregation method")?;
        let mut previous_test_end = None;
        for window in &details.windows {
            nonempty(&window.label, "Walk-forward window label")?;
            if !(window.training_start_at_us < window.training_end_at_us
                && window.training_end_at_us <= window.validation_start_at_us
                && window.validation_start_at_us < window.validation_end_at_us
                && window.validation_end_at_us <= window.test_start_at_us
                && window.test_start_at_us < window.test_end_at_us)
            {
                return Err(anyhow!(
                    "Walk-forward boundaries must be ordered with non-empty train, validation, and test periods."
                ));
            }
            if previous_test_end.is_some_and(|end| end > window.training_start_at_us) {
                return Err(anyhow!(
                    "Walk-forward windows must be chronologically ordered and non-overlapping."
                ));
            }
            previous_test_end = Some(window.test_end_at_us);
        }
        Ok(())
    })?;
    validate_applicability("Forward testing", &plan.forward_testing, |details| {
        nonempty(&details.actual_execution_conditions, "Forward execution conditions")?;
        nonempty(&details.costs_and_fees, "Forward-test costs and fees")
    })?;
    validate_applicability("Reproducibility", &plan.reproducibility, |details| {
        nonempty(&details.protocol_version, "Reproducibility protocol version")
    })?;
    Ok(())
}

fn insert_definition_event(
    tx: &Transaction<'_>,
    version_id: &str,
    status: AnalysisDefinitionStatus,
    rationale: &str,
    actor_id: Option<&str>,
    now: i64,
) -> Result<()> {
    tx.execute(
        "INSERT INTO analysis_definition_events
            (id, definition_version_id, status, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id(), version_id, status.as_str(), rationale.trim(), actor_id, now],
    )?;
    Ok(())
}

pub fn create_definition_version(
    conn: &mut Connection,
    req: &CreateAnalysisDefinitionVersionReq,
) -> Result<AnalysisDefinitionVersionDto> {
    nonempty(&req.title, "Analysis definition title")?;
    nonempty(&req.version_label, "Analysis definition version label")?;
    nonempty(&req.specification.protocol_version, "Analysis protocol version")?;
    if req.specification.required_input_roles.is_empty() {
        return Err(anyhow!("At least one exact dataset role must be declared."));
    }
    let roles: HashSet<_> = req.specification.required_input_roles.iter().collect();
    if roles.len() != req.specification.required_input_roles.len() {
        return Err(anyhow!("Required dataset roles must be unique."));
    }
    if !req.specification.parameters_schema.is_object() {
        return Err(anyhow!("Analysis parameter schema must be a JSON object."));
    }
    validate_integrity_plan(&req.specification.integrity)?;
    if req.metric_definition_version_ids.is_empty() {
        return Err(anyhow!("An analysis definition must pin at least one metric version."));
    }
    if req.metric_definition_version_ids.iter().collect::<HashSet<_>>().len()
        != req.metric_definition_version_ids.len()
    {
        return Err(anyhow!("Metric version pins must be unique."));
    }
    if req.parent_version_id.is_some() {
        nonempty(&req.change_rationale, "Analysis definition change rationale")?;
    } else if !req.change_rationale.trim().is_empty() {
        return Err(anyhow!("A change rationale is only valid for a derived definition version."));
    }

    let now = now_us()?;
    let version_id = id();
    let definition_id = req.definition_id.clone().unwrap_or_else(id);
    let specification_json = json(&req.specification)?;
    let specification_sha256 = sha256(specification_json.as_bytes());
    let tx = conn.transaction()?;

    let method_project: String = tx.query_row(
        "SELECT m.project_id FROM analysis_method_versions v
         JOIN analysis_methods m ON m.id = v.method_id WHERE v.id = ?1",
        [&req.method_version_id],
        |row| row.get(0),
    )?;
    if method_project != req.project_id {
        return Err(anyhow!("The analysis method version belongs to a different project."));
    }
    for metric_version_id in &req.metric_definition_version_ids {
        let metric_project: String = tx.query_row(
            "SELECT project_id FROM analysis_metric_definition_versions v
             JOIN analysis_metric_definitions m ON m.id = v.metric_definition_id
             WHERE v.id = ?1",
            [metric_version_id],
            |row| row.get(0),
        )?;
        if metric_project != req.project_id {
            return Err(anyhow!("A metric version belongs to a different project."));
        }
    }

    let version_no: i64;
    if let Some(existing_id) = req.definition_id.as_deref() {
        let existing: Option<String> = tx
            .query_row(
                "SELECT project_id FROM analysis_definitions WHERE id = ?1",
                [existing_id],
                |row| row.get(0),
            )
            .optional()?;
        if existing.as_deref() != Some(req.project_id.as_str()) {
            return Err(anyhow!("Analysis definition was not found in this project."));
        }
        version_no = tx.query_row(
            "SELECT COALESCE(MAX(version_no), 0) + 1
             FROM analysis_definition_versions WHERE definition_id = ?1",
            [existing_id],
            |row| row.get(0),
        )?;
    } else {
        if req.parent_version_id.is_some() {
            return Err(anyhow!("A first analysis definition version cannot have a parent."));
        }
        tx.execute(
            "INSERT INTO analysis_definitions
                (id, project_id, title, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![definition_id, req.project_id, req.title.trim(), req.created_by, now],
        )?;
        version_no = 1;
    }
    if let Some(parent_id) = req.parent_version_id.as_deref() {
        let parent_definition: String = tx.query_row(
            "SELECT definition_id FROM analysis_definition_versions WHERE id = ?1",
            [parent_id],
            |row| row.get(0),
        )?;
        if parent_definition != definition_id {
            return Err(anyhow!("Parent version must belong to this analysis definition."));
        }
    }
    tx.execute(
        "INSERT INTO analysis_definition_versions
            (id, definition_id, version_no, version_label, parent_version_id,
             change_rationale, method_version_id, specification_json,
             specification_sha256, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            version_id,
            definition_id,
            version_no,
            req.version_label.trim(),
            req.parent_version_id,
            req.parent_version_id
                .as_ref()
                .map(|_| req.change_rationale.trim()),
            req.method_version_id,
            specification_json,
            specification_sha256,
            req.created_by,
            now
        ],
    )?;
    for metric_version_id in &req.metric_definition_version_ids {
        tx.execute(
            "INSERT INTO analysis_definition_metric_versions
                (definition_version_id, metric_definition_version_id)
             VALUES (?1, ?2)",
            params![version_id, metric_version_id],
        )?;
    }
    insert_definition_event(
        &tx,
        &version_id,
        AnalysisDefinitionStatus::Draft,
        "Version created.",
        req.created_by.as_deref(),
        now,
    )?;
    tx.commit()?;
    query_definition_version(conn, &version_id)
}

pub fn transition_definition(
    conn: &mut Connection,
    req: &TransitionAnalysisDefinitionReq,
) -> Result<AnalysisDefinitionVersionDto> {
    nonempty(&req.rationale, "Lifecycle rationale")?;
    let now = now_us()?;
    let tx = conn.transaction()?;
    let current: String = tx.query_row(
        "SELECT status FROM analysis_definition_events
         WHERE definition_version_id = ?1
         ORDER BY occurred_at_us DESC, id DESC LIMIT 1",
        [&req.version_id],
        |row| row.get(0),
    )?;
    let next = req.next_status.as_str();
    let allowed = matches!(
        (current.as_str(), next),
        ("draft", "review" | "retired") | ("review", "locked" | "retired") | ("locked", "retired")
    );
    if !allowed {
        return Err(anyhow!("Invalid analysis definition lifecycle transition."));
    }
    if req.next_status == AnalysisDefinitionStatus::Locked {
        let method_status: String = tx.query_row(
            "SELECT m.verification_status
             FROM analysis_definition_versions d
             JOIN analysis_method_versions m ON m.id = d.method_version_id
             WHERE d.id = ?1",
            [&req.version_id],
            |row| row.get(0),
        )?;
        let unverified_metrics: i64 = tx.query_row(
            "SELECT COUNT(*) FROM analysis_definition_metric_versions dm
             JOIN analysis_metric_definition_versions mv
               ON mv.id = dm.metric_definition_version_id
             WHERE dm.definition_version_id = ?1 AND mv.verification_status <> 'passed'",
            [&req.version_id],
            |row| row.get(0),
        )?;
        if method_status != "passed" || unverified_metrics > 0 {
            return Err(anyhow!(
                "Locking requires a passing method test report and passing reports for every metric version."
            ));
        }
        let metrics_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM analysis_definition_metric_versions
             WHERE definition_version_id = ?1",
            [&req.version_id],
            |row| row.get(0),
        )?;
        if metrics_count == 0 {
            return Err(anyhow!("Locking requires at least one metric definition version."));
        }
        let raw_specification: String = tx.query_row(
            "SELECT specification_json FROM analysis_definition_versions WHERE id = ?1",
            [&req.version_id],
            |row| row.get(0),
        )?;
        let specification: AnalysisDefinitionSpecificationDto = parse_json(&raw_specification)?;
        validate_integrity_plan(&specification.integrity)?;
    }
    insert_definition_event(
        &tx,
        &req.version_id,
        req.next_status,
        &req.rationale,
        req.actor_id.as_deref(),
        now,
    )?;
    tx.commit()?;
    query_definition_version(conn, &req.version_id)
}

fn definition_status(conn: &Connection, version_id: &str) -> Result<String> {
    Ok(conn.query_row(
        "SELECT status FROM analysis_definition_events
         WHERE definition_version_id = ?1
         ORDER BY occurred_at_us DESC, id DESC LIMIT 1",
        [version_id],
        |row| row.get(0),
    )?)
}

fn insert_control<T: Serialize>(
    tx: &Transaction<'_>,
    run_id: &str,
    category: &str,
    control: &Applicability<T>,
) -> Result<()> {
    let (state, rationale, details) = match control {
        Applicability::Applicable { details } => {
            ("applicable", String::new(), json(details)?)
        }
        Applicability::NotApplicable { reason } => {
            ("not_applicable", reason.trim().to_string(), "{}".to_string())
        }
        Applicability::NotAssessed { rationale } => (
            "not_assessed",
            rationale.as_deref().unwrap_or_default().trim().to_string(),
            "{}".to_string(),
        ),
    };
    tx.execute(
        "INSERT INTO analysis_run_integrity_declarations
            (run_id, category, applicability, rationale, details_json)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![run_id, category, state, rationale, details],
    )?;
    if state == "not_assessed" {
        insert_integrity_finding(
            tx,
            run_id,
            category,
            &format!("control_not_assessed:{category}"),
            AnalysisIntegritySeverity::Warning,
            &format!("{category} safeguard was explicitly left not assessed."),
        )?;
    }
    Ok(())
}

fn insert_integrity_finding(
    tx: &Transaction<'_>,
    run_id: &str,
    category: &str,
    code: &str,
    severity: AnalysisIntegritySeverity,
    message: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO analysis_integrity_findings
            (id, run_id, category, code, severity, message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id(), run_id, category, code, severity.as_str(), message],
    )?;
    Ok(())
}

fn transformation_lineage(conn: &Connection, version_id: &str) -> Result<serde_json::Value> {
    let mut pending = vec![version_id.to_string()];
    let mut visited = HashSet::new();
    let mut lineage = Vec::new();
    while let Some(output_version_id) = pending.pop() {
        if !visited.insert(output_version_id.clone()) {
            return Err(anyhow!(
                "Dataset transformation lineage contains a cycle at {output_version_id}."
            ));
        }
        let transformation: Option<(String, String, String, String)> = conn
            .query_row(
                "SELECT id, method, parameters_json, provenance_json
                 FROM experiment_dataset_transformations
                 WHERE output_dataset_version_id = ?1",
                [&output_version_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        if let Some((transformation_id, method, parameters, provenance)) = transformation {
            let inputs = {
                let mut statement = conn.prepare(
                    "SELECT input_dataset_version_id, input_role
                     FROM experiment_dataset_transformation_inputs
                     WHERE transformation_id = ?1
                     ORDER BY input_dataset_version_id, input_role",
                )?;
                statement
                    .query_map([&transformation_id], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            };
            for (input_version_id, _) in &inputs {
                pending.push(input_version_id.clone());
            }
            lineage.push(serde_json::json!({
                "transformationId": transformation_id,
                "outputDatasetVersionId": output_version_id,
                "method": method,
                "parameters": parse_json::<serde_json::Value>(&parameters)?,
                "provenance": parse_json::<serde_json::Value>(&provenance)?,
                "inputs": inputs.into_iter().map(|(dataset_version_id, role)| {
                    serde_json::json!({ "datasetVersionId": dataset_version_id, "role": role })
                }).collect::<Vec<_>>()
            }));
        }
    }
    lineage.reverse();
    Ok(serde_json::Value::Array(lineage))
}

fn expected_role(role: DatasetRole) -> &'static str {
    role.as_str()
}

fn add_role(set: &mut BTreeSet<String>, role: &DatasetRole) {
    set.insert(role.as_str().to_string());
}

fn validate_temporal_sample(
    sample: &AnalysisInputSampleReq,
    look_ahead_applicable: bool,
) -> Result<()> {
    match (
        sample.feature_available_at_us,
        sample.prediction_at_us,
        sample.decision_at_us,
    ) {
        (Some(feature), Some(prediction), Some(decision)) => {
            if feature > prediction {
                return Err(anyhow!(
                    "Temporal leakage detected for sample {}: a feature became available after prediction.",
                    sample.sample_id
                ));
            }
            if prediction > decision {
                return Err(anyhow!(
                    "Sample {} has a prediction timestamp after its decision timestamp.",
                    sample.sample_id
                ));
            }
            if sample
                .outcome_available_at_us
                .is_some_and(|outcome| outcome < prediction)
            {
                return Err(anyhow!(
                    "Sample {} has an outcome available before its prediction timestamp.",
                    sample.sample_id
                ));
            }
        }
        _ if look_ahead_applicable => {
            return Err(anyhow!(
                "Look-ahead control is applicable; sample {} requires feature-availability, prediction, and decision timestamps.",
                sample.sample_id
            ));
        }
        _ => {
            if sample.outcome_available_at_us.is_some() {
                return Err(anyhow!(
                    "Sample {} cannot declare outcome availability without feature, prediction, and decision timestamps.",
                    sample.sample_id
                ));
            }
        }
    }
    Ok(())
}

fn validate_split_and_get_key(
    tx: &Transaction<'_>,
    input: &AnalysisInputReq,
    sample_id: &str,
    disjoint_required: bool,
) -> Result<Option<String>> {
    let Some(split_group_id) = input.split_group_id.as_deref() else {
        if disjoint_required
            && matches!(
                input.role,
                DatasetRole::Training | DatasetRole::Validation | DatasetRole::Test
            )
        {
            return Err(anyhow!(
                "A declared disjoint split requires an explicit persisted split group."
            ));
        }
        return Ok(None);
    };
    let group: Option<(String, bool)> = tx
        .query_row(
            "SELECT dataset_version_id, require_disjoint
             FROM experiment_dataset_split_groups WHERE id = ?1",
            [split_group_id],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? != 0)),
        )
        .optional()?;
    match group {
        Some((version_id, is_disjoint)) if version_id == input.dataset_version_id => {
            if disjoint_required && !is_disjoint {
                return Err(anyhow!(
                    "This split group does not enforce the required disjointness."
                ));
            }
        }
        Some(_) => return Err(anyhow!("The split group belongs to another dataset version.")),
        None => return Err(anyhow!("The requested dataset split group does not exist.")),
    }
    let assignment: Option<String> = tx
        .query_row(
            "SELECT split_key FROM experiment_sample_split_assignments
             WHERE split_group_id = ?1 AND dataset_version_id = ?2
               AND sample_id = ?3 AND role = ?4",
            params![
                split_group_id,
                input.dataset_version_id,
                sample_id,
                expected_role(input.role)
            ],
            |row| row.get(0),
        )
        .optional()?;
    assignment
        .map(Some)
        .ok_or_else(|| anyhow!("Sample {sample_id} is not assigned to the declared split role."))
}

fn validate_input(
    tx: &Transaction<'_>,
    project_id: &str,
    input: &AnalysisInputReq,
    look_ahead_applicable: bool,
    disjoint_required: bool,
) -> Result<(String, serde_json::Value, Vec<(AnalysisInputSampleReq, Option<String>)>)> {
    if input.samples.is_empty() {
        return Err(anyhow!("Every analysis input must pin at least one sample."));
    }
    let (dataset_project_id, lifecycle_status, checksum): (String, String, Option<String>) =
        tx.query_row(
            "SELECT d.project_id, v.lifecycle_status, v.content_sha256
             FROM experiment_dataset_versions v
             JOIN experiment_datasets d ON d.id = v.dataset_id
             WHERE v.id = ?1",
            [&input.dataset_version_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    if dataset_project_id != project_id {
        return Err(anyhow!(
            "Analysis input dataset belongs to a different project."
        ));
    }
    if lifecycle_status != "sealed" {
        return Err(anyhow!("Analysis inputs must pin sealed dataset versions."));
    }
    let checksum =
        checksum.ok_or_else(|| anyhow!("Sealed dataset version is missing its checksum."))?;
    valid_sha256(&checksum, "Dataset checksum")?;

    let mut seen_samples = HashSet::new();
    let mut assignments = Vec::with_capacity(input.samples.len());
    for sample in &input.samples {
        nonempty(&sample.sample_id, "Analysis sample ID")?;
        if !seen_samples.insert(sample.sample_id.as_str()) {
            return Err(anyhow!(
                "Sample {} is repeated in one analysis input.",
                sample.sample_id
            ));
        }
        validate_temporal_sample(sample, look_ahead_applicable)?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM experiment_dataset_samples
             WHERE dataset_version_id = ?1 AND sample_id = ?2)",
            params![input.dataset_version_id, sample.sample_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(anyhow!(
                "Sample {} is not part of the exact pinned dataset version.",
                sample.sample_id
            ));
        }
        let split_key =
            validate_split_and_get_key(tx, input, &sample.sample_id, disjoint_required)?;
        assignments.push((sample.clone(), split_key));
    }
    let lineage = transformation_lineage(tx, &input.dataset_version_id)?;
    Ok((checksum, lineage, assignments))
}

fn control_is_not_assessed<T>(control: &Applicability<T>) -> bool {
    matches!(control, Applicability::NotAssessed { .. })
}

fn insert_integrity_declarations(
    tx: &Transaction<'_>,
    run_id: &str,
    plan: &StatisticalIntegrityPlanDto,
) -> Result<()> {
    insert_control(tx, run_id, "overfitting", &plan.overfitting)?;
    insert_control(tx, run_id, "data_leakage", &plan.data_leakage)?;
    insert_control(tx, run_id, "selection_bias", &plan.selection_bias)?;
    insert_control(tx, run_id, "look_ahead", &plan.look_ahead)?;
    insert_control(tx, run_id, "multiple_testing", &plan.multiple_testing)?;
    insert_control(tx, run_id, "dataset_separation", &plan.dataset_separation)?;
    insert_control(tx, run_id, "out_of_sample", &plan.out_of_sample)?;
    insert_control(tx, run_id, "walk_forward", &plan.walk_forward)?;
    insert_control(tx, run_id, "forward_testing", &plan.forward_testing)?;
    insert_control(tx, run_id, "reproducibility", &plan.reproducibility)?;
    Ok(())
}

fn insert_run_event(
    tx: &Transaction<'_>,
    run_id: &str,
    status: AnalysisRunStatus,
    summary: &str,
    failure_summary: &str,
    actor_id: Option<&str>,
    now: i64,
) -> Result<()> {
    tx.execute(
        "INSERT INTO analysis_run_events
            (id, run_id, lifecycle_status, summary, failure_summary, actor_id, occurred_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id(),
            run_id,
            status.as_str(),
            summary.trim(),
            failure_summary.trim(),
            actor_id,
            now
        ],
    )?;
    Ok(())
}

pub fn create_run(conn: &mut Connection, req: &CreateAnalysisRunReq) -> Result<AnalysisRunDto> {
    if !req.parameters.is_object() || !req.environment.is_object() {
        return Err(anyhow!("Analysis parameters and environment must be JSON objects."));
    }
    let mut seeds_seen = HashSet::new();
    if req.random_seeds.iter().any(|seed| seed.trim().is_empty() || !seeds_seen.insert(seed)) {
        return Err(anyhow!("Random seeds must be non-empty and unique."));
    }
    if req.inputs.is_empty() {
        return Err(anyhow!("An analysis run must pin at least one dataset input."));
    }
    let now = now_us()?;
    let run_id = id();
    let tx = conn.transaction()?;

    let definition: AnalysisDefinitionVersionDto =
        query_definition_version(&tx, &req.definition_version_id)?;
    if definition.lifecycle_status != AnalysisDefinitionStatus::Locked {
        return Err(anyhow!("Analysis run requires a locked definition version."));
    }
    let method = query_method_version(&tx, &definition.method_version_id)?;
    if method.verification_status != AnalysisVerificationStatus::Passed {
        return Err(anyhow!("Analysis method test report has not passed."));
    }
    if method.environment != req.environment && req.environment.as_object().is_none_or(|value| value.is_empty()) {
        return Err(anyhow!("Run environment must be recorded."));
    }
    let project_id = definition.project_id.clone();
    if let Applicability::Applicable { details } = &definition.specification.integrity.reproducibility {
        if details.parameters_and_seeds_required && req.random_seeds.is_empty() {
            return Err(anyhow!(
                "The reproducibility safeguard requires at least one declared random seed."
            ));
        }
    }
    let look_ahead_applicable = matches!(
        definition.specification.integrity.look_ahead,
        Applicability::Applicable { .. }
    );
    let disjoint_required = match &definition.specification.integrity.dataset_separation {
        Applicability::Applicable { details } => details.require_disjoint_splits,
        _ => false,
    };

    let mut actual_roles = BTreeSet::new();
    let mut inputs_seen = HashSet::new();
    let mut selected_by_version: HashMap<(String, String), String> = HashMap::new();
    let mut checked_inputs = Vec::with_capacity(req.inputs.len());
    for input in &req.inputs {
        add_role(&mut actual_roles, &input.role);
        if !inputs_seen.insert((input.dataset_version_id.as_str(), input.role.as_str())) {
            return Err(anyhow!(
                "Dataset version and role input pins must be unique."
            ));
        }
        let (checksum, lineage, assignments) = validate_input(
            &tx,
            &project_id,
            input,
            look_ahead_applicable,
            disjoint_required,
        )?;
        if disjoint_required {
            for (sample, _) in &assignments {
                let key = (input.dataset_version_id.clone(), sample.sample_id.clone());
                if let Some(previous_role) =
                    selected_by_version.insert(key, input.role.as_str().to_string())
                {
                    if previous_role != input.role.as_str() {
                        return Err(anyhow!(
                            "Disjoint dataset separation is violated by sample {} appearing in multiple roles.",
                            sample.sample_id
                        ));
                    }
                }
            }
        }
        checked_inputs.push((input, checksum, lineage, assignments));
    }
    let required_roles: BTreeSet<String> = definition
        .specification
        .required_input_roles
        .iter()
        .map(|role| role.as_str().to_string())
        .collect();
    if actual_roles != required_roles {
        return Err(anyhow!(
            "Analysis inputs must match the locked required dataset roles exactly."
        ));
    }
    if let Applicability::Applicable { details } = &definition.specification.integrity.out_of_sample {
        if !actual_roles.contains("test") {
            return Err(anyhow!(
                "Applicable out-of-sample evaluation requires a pinned test-role input."
            ));
        }
        if details.model_selection_usage != ModelSelectionUsage::NotUsed {
            insert_integrity_finding(
                &tx,
                &run_id,
                "out_of_sample",
                "evaluation_role_used_for_selection",
                AnalysisIntegritySeverity::Warning,
                "The declared test/evaluation data was used for model selection; it is not an untouched out-of-sample evaluation.",
            )?;
        }
    }
    if matches!(
        definition.specification.integrity.walk_forward,
        Applicability::Applicable { .. }
    ) && !["training", "validation", "test"]
        .iter()
        .all(|role| actual_roles.contains(*role))
    {
        return Err(anyhow!(
            "Applicable walk-forward analysis requires training, validation, and test roles."
        ));
    }
    if let Applicability::Applicable { details } = &definition.specification.integrity.forward_testing {
        let required_role = match details.execution_kind {
            ForwardExecutionKind::Prospective => "prospective",
            ForwardExecutionKind::Paper | ForwardExecutionKind::Live => "forward",
        };
        if !actual_roles.contains(required_role) {
            return Err(anyhow!(
                "Forward testing requires a {required_role} dataset role."
            ));
        }
    }

    let created_at_us = now;
    tx.execute(
        "INSERT INTO analysis_runs
            (id, project_id, definition_version_id, parameters_json, environment_json,
             random_seeds_json, protocol_version, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            run_id,
            project_id,
            req.definition_version_id,
            json(&req.parameters)?,
            json(&req.environment)?,
            json(&req.random_seeds)?,
            definition.specification.protocol_version,
            req.created_by,
            created_at_us
        ],
    )?;
    insert_run_event(
        &tx,
        &run_id,
        AnalysisRunStatus::Queued,
        "Analysis run queued.",
        "",
        req.created_by.as_deref(),
        now,
    )?;
    insert_integrity_declarations(
        &tx,
        &run_id,
        &definition.specification.integrity,
    )?;

    for (input, checksum, lineage, assignments) in checked_inputs {
        let input_id = id();
        let manifest_json = json(&(
            &input.dataset_version_id,
            input.role.as_str(),
            &checksum,
            &lineage,
            &assignments,
        ))?;
        tx.execute(
            "INSERT INTO analysis_run_inputs
                (id, run_id, dataset_version_id, role, split_group_id, dataset_checksum,
                 sample_count, transformation_lineage_json, input_manifest_sha256)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                input_id,
                run_id,
                input.dataset_version_id,
                input.role.as_str(),
                input.split_group_id,
                checksum,
                assignments.len() as i64,
                json(&lineage)?,
                sha256(manifest_json.as_bytes())
            ],
        )?;
        for (sample, split_key) in assignments {
            tx.execute(
                "INSERT INTO analysis_run_input_samples
                    (input_id, dataset_version_id, sample_id, split_key,
                     feature_available_at_us, prediction_at_us, decision_at_us,
                     outcome_available_at_us)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    input_id,
                    input.dataset_version_id,
                    sample.sample_id,
                    split_key,
                    sample.feature_available_at_us,
                    sample.prediction_at_us,
                    sample.decision_at_us,
                    sample.outcome_available_at_us
                ],
            )?;
        }
    }
    tx.commit()?;
    query_run(conn, &run_id)
}

pub fn start_run(
    conn: &mut Connection,
    run_id: &str,
    actor_id: Option<&str>,
) -> Result<AnalysisRunDto> {
    let now = now_us()?;
    let tx = conn.transaction()?;
    insert_run_event(
        &tx,
        run_id,
        AnalysisRunStatus::Running,
        "External analysis execution started.",
        "",
        actor_id,
        now,
    )?;
    tx.commit()?;
    query_run(conn, run_id)
}

pub fn add_run_log(
    conn: &mut Connection,
    req: &AddAnalysisRunLogReq,
) -> Result<AnalysisRunLogDto> {
    nonempty(&req.message, "Analysis log message")?;
    let log_id = id();
    let recorded_at_us = now_us()?;
    conn.execute(
        "INSERT INTO analysis_run_logs
            (id, run_id, severity, message, details_json, recorded_by, recorded_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            log_id,
            req.run_id,
            req.severity.as_str(),
            req.message.trim(),
            json(&req.details)?,
            req.recorded_by,
            recorded_at_us
        ],
    )?;
    Ok(AnalysisRunLogDto {
        id: log_id,
        severity: req.severity.clone(),
        message: req.message.trim().to_string(),
        details: req.details.clone(),
        recorded_by: req.recorded_by.clone(),
        recorded_at_us,
    })
}

pub fn add_run_deviation(
    conn: &mut Connection,
    run_id: &str,
    req: &AnalysisDeviationReq,
) -> Result<AnalysisDeviationDto> {
    nonempty(&req.description, "Deviation description")?;
    nonempty(&req.rationale, "Deviation rationale")?;
    nonempty(&req.impact_assessment, "Deviation impact assessment")?;
    let deviation_id = id();
    let recorded_at_us = now_us()?;
    conn.execute(
        "INSERT INTO analysis_run_deviations
            (id, run_id, description, rationale, impact_assessment, recorded_by, recorded_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            deviation_id,
            run_id,
            req.description.trim(),
            req.rationale.trim(),
            req.impact_assessment.trim(),
            req.recorded_by,
            recorded_at_us
        ],
    )?;
    Ok(AnalysisDeviationDto {
        id: deviation_id,
        description: req.description.trim().to_string(),
        rationale: req.rationale.trim().to_string(),
        impact_assessment: req.impact_assessment.trim().to_string(),
        recorded_by: req.recorded_by.clone(),
        recorded_at_us,
    })
}
