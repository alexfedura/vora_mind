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
