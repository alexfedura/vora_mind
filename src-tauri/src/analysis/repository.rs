use super::*;
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
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
    let foreign_keys_enabled: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
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
            rusqlite::Error::InvalidColumnType(
                9,
                "environment_json".into(),
                rusqlite::types::Type::Text,
            )
        })?,
        assumptions: serde_json::from_str(&row.get::<_, String>(10)?).map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                10,
                "assumptions_json".into(),
                rusqlite::types::Type::Text,
            )
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
        return Err(anyhow!(
            "Method assumptions must not contain empty entries."
        ));
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
            Some(_) => {
                return Err(anyhow!(
                    "The method identity does not match this project and name."
                ))
            }
            None => return Err(anyhow!("Analysis method was not found.")),
        }
    } else {
        tx.execute(
            "INSERT INTO analysis_methods (id, project_id, name, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                method_id,
                req.project_id,
                req.name.trim(),
                req.created_by,
                now
            ],
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

fn metric_version_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<MetricDefinitionVersionDto> {
    let verification_status: String = row.get(11)?;
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
        tests_total: row.get(12)?,
        tests_passed: row.get(13)?,
        verification_status: AnalysisVerificationStatus::parse(&verification_status)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        content_sha256: row.get(14)?,
        created_by: row.get(15)?,
        created_at_us: row.get(16)?,
    })
}

fn query_metric_version(conn: &Connection, version_id: &str) -> Result<MetricDefinitionVersionDto> {
    Ok(conn.query_row(
        "SELECT v.id, m.id, m.project_id, m.metric_key, m.name, v.version_no,
                v.version_label, v.formula, v.unit, v.test_suite_uri,
                v.test_suite_sha256, v.verification_status, v.tests_total,
                v.tests_passed, v.content_sha256, v.created_by, v.created_at_us
         FROM analysis_metric_definition_versions v
         JOIN analysis_metric_definitions m ON m.id = v.metric_definition_id
         WHERE v.id = ?1",
        [version_id],
        metric_version_from_row,
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
            Some(_) => {
                return Err(anyhow!(
                    "The metric identity does not match this project and key."
                ))
            }
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
        String,
        Option<String>,
        Option<String>,
        String,
        String,
        String,
        Option<String>,
        i64,
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
        let rows = statement.query_map([version_id], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
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
        version_label: base.5,
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

fn json_value_matches_type(value: &serde_json::Value, expected: &str) -> bool {
    match expected {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.as_f64().is_some(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => false,
    }
}

fn validate_parameter_schema(schema: &serde_json::Value) -> Result<()> {
    let Some(schema) = schema.as_object() else {
        return Err(anyhow!("Analysis parameter schema must be a JSON object."));
    };
    if schema
        .get("type")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|kind| kind != "object")
    {
        return Err(anyhow!(
            "Analysis parameters schema must describe an object."
        ));
    }
    if schema
        .get("properties")
        .is_some_and(|properties| !properties.is_object())
    {
        return Err(anyhow!("Parameter schema properties must be an object."));
    }
    if let Some(required) = schema.get("required") {
        let Some(required) = required.as_array() else {
            return Err(anyhow!("Parameter schema required must be an array."));
        };
        if required.iter().any(|name| name.as_str().is_none()) {
            return Err(anyhow!("Every required parameter name must be a string."));
        }
    }
    if let Some(properties) = schema.get("properties").and_then(|value| value.as_object()) {
        for (name, definition) in properties {
            if !definition.is_object() {
                return Err(anyhow!("Parameter schema for {name} must be an object."));
            }
            if let Some(kind) = definition.get("type").and_then(serde_json::Value::as_str) {
                if !matches!(
                    kind,
                    "object" | "array" | "string" | "number" | "integer" | "boolean" | "null"
                ) {
                    return Err(anyhow!(
                        "Parameter schema for {name} declares unsupported type {kind}."
                    ));
                }
            }
            if definition
                .get("enum")
                .is_some_and(|values| !values.is_array())
            {
                return Err(anyhow!(
                    "Parameter schema enum for {name} must be an array."
                ));
            }
        }
    }
    Ok(())
}

fn validate_parameters(schema: &serde_json::Value, parameters: &serde_json::Value) -> Result<()> {
    validate_parameter_schema(schema)?;
    let Some(parameters) = parameters.as_object() else {
        return Err(anyhow!("Analysis parameters must be a JSON object."));
    };
    let schema = schema
        .as_object()
        .ok_or_else(|| anyhow!("Analysis parameter schema must be a JSON object."))?;
    if let Some(required) = schema.get("required").and_then(serde_json::Value::as_array) {
        for name in required.iter().filter_map(serde_json::Value::as_str) {
            if !parameters.contains_key(name) {
                return Err(anyhow!("Required analysis parameter {name} is missing."));
            }
        }
    }
    if let Some(properties) = schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
    {
        for (name, value) in parameters {
            let Some(property_schema) = properties.get(name) else {
                if schema
                    .get("additionalProperties")
                    .and_then(serde_json::Value::as_bool)
                    == Some(false)
                {
                    return Err(anyhow!("Unexpected analysis parameter {name}."));
                }
                continue;
            };
            if let Some(expected_type) = property_schema
                .get("type")
                .and_then(serde_json::Value::as_str)
            {
                if !json_value_matches_type(value, expected_type) {
                    return Err(anyhow!(
                        "Analysis parameter {name} does not match required type {expected_type}."
                    ));
                }
            }
            if let Some(allowed) = property_schema
                .get("enum")
                .and_then(serde_json::Value::as_array)
            {
                if !allowed.contains(value) {
                    return Err(anyhow!(
                        "Analysis parameter {name} is outside its declared allowed values."
                    ));
                }
            }
        }
    }
    Ok(())
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
            if rationale
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
            {
                return Err(anyhow!(
                    "{category} rationale must not be empty when supplied."
                ));
            }
        }
    }
    Ok(())
}

fn validate_integrity_plan(plan: &StatisticalIntegrityPlanDto) -> Result<()> {
    validate_applicability("Overfitting", &plan.overfitting, |details| {
        validate_required_text(
            &details.training_tuning_separation,
            "Training/tuning separation",
        )?;
        validate_required_text(&details.parameter_search_space, "Parameter search space")?;
        validate_required_text(&details.model_selection_process, "Model selection process")?;
        validate_required_text(&details.complexity_reporting, "Complexity reporting")?;
        validate_required_text(&details.sensitivity_reporting, "Sensitivity reporting")
    })?;
    validate_applicability("Data leakage", &plan.data_leakage, |details| {
        validate_required_text(&details.data_cutoffs, "Data cutoffs")?;
        validate_required_text(&details.transformation_order, "Transformation order")?;
        validate_required_text(
            &details.feature_selection_provenance,
            "Feature-selection provenance",
        )?;
        validate_required_text(
            &details.evaluation_data_isolation,
            "Evaluation-data isolation",
        )
    })?;
    validate_applicability("Selection bias", &plan.selection_bias, |details| {
        validate_required_text(&details.sampling_frame, "Sampling frame")?;
        validate_required_text(&details.inclusion_rules, "Inclusion rules")?;
        validate_required_text(&details.exclusion_rules, "Exclusion rules")?;
        validate_required_text(
            &details.filtered_and_missing_cases,
            "Filtered/missing cases",
        )?;
        validate_required_text(&details.survivor_population, "Survivor population")
    })?;
    validate_applicability("Look-ahead", &plan.look_ahead, |details| {
        validate_required_text(
            &details.feature_availability_policy,
            "Feature-availability policy",
        )?;
        validate_required_text(
            &details.prediction_timestamp_policy,
            "Prediction timestamp policy",
        )?;
        validate_required_text(
            &details.decision_timestamp_policy,
            "Decision timestamp policy",
        )
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
        nonempty(
            &details.selection_rationale,
            "Dataset-role selection rationale",
        )
    })?;
    validate_applicability("Out-of-sample", &plan.out_of_sample, |details| {
        nonempty(&details.evaluation_period, "Evaluation period")?;
        nonempty(&details.evaluation_population, "Evaluation population")
    })?;
    validate_applicability("Walk-forward", &plan.walk_forward, |details| {
        if details.windows.is_empty() {
            return Err(anyhow!(
                "Walk-forward control requires at least one declared window."
            ));
        }
        nonempty(&details.refit_cadence, "Walk-forward refit cadence")?;
        nonempty(
            &details.aggregation_method,
            "Walk-forward aggregation method",
        )?;
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
        nonempty(
            &details.actual_execution_conditions,
            "Forward execution conditions",
        )?;
        nonempty(&details.costs_and_fees, "Forward-test costs and fees")
    })?;
    validate_applicability("Reproducibility", &plan.reproducibility, |details| {
        nonempty(
            &details.protocol_version,
            "Reproducibility protocol version",
        )
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
        params![
            id(),
            version_id,
            status.as_str(),
            rationale.trim(),
            actor_id,
            now
        ],
    )?;
    Ok(())
}

pub fn create_definition_version(
    conn: &mut Connection,
    req: &CreateAnalysisDefinitionVersionReq,
) -> Result<AnalysisDefinitionVersionDto> {
    nonempty(&req.title, "Analysis definition title")?;
    nonempty(&req.version_label, "Analysis definition version label")?;
    nonempty(
        &req.specification.protocol_version,
        "Analysis protocol version",
    )?;
    if req.specification.required_input_roles.is_empty() {
        return Err(anyhow!("At least one exact dataset role must be declared."));
    }
    let roles: HashSet<_> = req
        .specification
        .required_input_roles
        .iter()
        .map(DatasetRole::as_str)
        .collect();
    if roles.len() != req.specification.required_input_roles.len() {
        return Err(anyhow!("Required dataset roles must be unique."));
    }
    validate_parameter_schema(&req.specification.parameters_schema)?;
    if req.definition_id.is_some() != req.parent_version_id.is_some() {
        return Err(anyhow!(
            "A revised analysis definition must specify both its identity and parent version."
        ));
    }
    validate_integrity_plan(&req.specification.integrity)?;
    if req.metric_definition_version_ids.is_empty() {
        return Err(anyhow!(
            "An analysis definition must pin at least one metric version."
        ));
    }
    if req
        .metric_definition_version_ids
        .iter()
        .collect::<HashSet<_>>()
        .len()
        != req.metric_definition_version_ids.len()
    {
        return Err(anyhow!("Metric version pins must be unique."));
    }
    if req.parent_version_id.is_some() {
        nonempty(
            &req.change_rationale,
            "Analysis definition change rationale",
        )?;
    } else if !req.change_rationale.trim().is_empty() {
        return Err(anyhow!(
            "A change rationale is only valid for a derived definition version."
        ));
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
        return Err(anyhow!(
            "The analysis method version belongs to a different project."
        ));
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
            return Err(anyhow!(
                "Analysis definition was not found in this project."
            ));
        }
        version_no = tx.query_row(
            "SELECT COALESCE(MAX(version_no), 0) + 1
             FROM analysis_definition_versions WHERE definition_id = ?1",
            [existing_id],
            |row| row.get(0),
        )?;
    } else {
        if req.parent_version_id.is_some() {
            return Err(anyhow!(
                "A first analysis definition version cannot have a parent."
            ));
        }
        tx.execute(
            "INSERT INTO analysis_definitions
                (id, project_id, title, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                definition_id,
                req.project_id,
                req.title.trim(),
                req.created_by,
                now
            ],
        )?;
        version_no = 1;
    }
    if let Some(parent_id) = req.parent_version_id.as_deref() {
        let parent_definition: String = tx.query_row(
            "SELECT definition_id FROM analysis_definition_versions WHERE id = ?1",
            [parent_id],
            |row| row.get(0),
        )?;
        let latest_parent: String = tx.query_row(
            "SELECT id FROM analysis_definition_versions
             WHERE definition_id = ?1 ORDER BY version_no DESC LIMIT 1",
            [&definition_id],
            |row| row.get(0),
        )?;
        if parent_definition != definition_id || latest_parent != parent_id {
            return Err(anyhow!(
                "Parent version must belong to this analysis definition."
            ));
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
            return Err(anyhow!(
                "Locking requires at least one metric definition version."
            ));
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

fn insert_control<T: Serialize>(
    tx: &Transaction<'_>,
    run_id: &str,
    category: &str,
    control: &Applicability<T>,
) -> Result<()> {
    let (state, rationale, details) = match control {
        Applicability::Applicable { details } => ("applicable", String::new(), json(details)?),
        Applicability::NotApplicable { reason } => (
            "not_applicable",
            reason.trim().to_string(),
            "{}".to_string(),
        ),
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
                let rows = statement.query_map([&transformation_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
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
                .is_some_and(|outcome| outcome < decision)
            {
                return Err(anyhow!(
                    "Sample {} has an outcome available before its decision timestamp.",
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
        Some(_) => {
            return Err(anyhow!(
                "The split group belongs to another dataset version."
            ))
        }
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
) -> Result<(
    String,
    serde_json::Value,
    Vec<(AnalysisInputSampleReq, Option<String>)>,
)> {
    if input.samples.is_empty() {
        return Err(anyhow!(
            "Every analysis input must pin at least one sample."
        ));
    }
    let (dataset_project_id, lifecycle_status, checksum): (String, String, Option<String>) = tx
        .query_row(
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
        return Err(anyhow!(
            "Analysis parameters and environment must be JSON objects."
        ));
    }
    let mut seeds_seen = HashSet::new();
    if req
        .random_seeds
        .iter()
        .any(|seed| seed.trim().is_empty() || !seeds_seen.insert(seed))
    {
        return Err(anyhow!("Random seeds must be non-empty and unique."));
    }
    if req.inputs.is_empty() {
        return Err(anyhow!(
            "An analysis run must pin at least one dataset input."
        ));
    }
    let now = now_us()?;
    let run_id = id();
    let tx = conn.transaction()?;

    let definition: AnalysisDefinitionVersionDto =
        query_definition_version(&tx, &req.definition_version_id)?;
    if definition.lifecycle_status != AnalysisDefinitionStatus::Locked {
        return Err(anyhow!(
            "Analysis run requires a locked definition version."
        ));
    }
    validate_parameters(&definition.specification.parameters_schema, &req.parameters)?;
    let method = query_method_version(&tx, &definition.method_version_id)?;
    if method.verification_status != AnalysisVerificationStatus::Passed {
        return Err(anyhow!("Analysis method test report has not passed."));
    }
    if req
        .environment
        .as_object()
        .is_some_and(serde_json::Map::is_empty)
    {
        return Err(anyhow!(
            "Run environment must contain the captured runtime identity."
        ));
    }
    let project_id = definition.project_id.clone();
    if let Applicability::Applicable { details } =
        &definition.specification.integrity.reproducibility
    {
        if details.parameters_and_seeds_required && req.random_seeds.is_empty() {
            return Err(anyhow!(
                "The reproducibility safeguard requires at least one declared random seed."
            ));
        }
    }
    let look_ahead_applicable = matches!(
        &definition.specification.integrity.look_ahead,
        Applicability::Applicable { .. }
    );
    let disjoint_required = match &definition.specification.integrity.dataset_separation {
        Applicability::Applicable { details } => details.require_disjoint_splits,
        _ => false,
    };

    let mut actual_roles = BTreeSet::new();
    let mut inputs_seen = HashSet::new();
    let mut selected_by_version: HashMap<(String, String), String> = HashMap::new();
    let mut pending_findings = Vec::new();
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
    if let Applicability::Applicable { details } = &definition.specification.integrity.out_of_sample
    {
        if !actual_roles.contains("test") {
            return Err(anyhow!(
                "Applicable out-of-sample evaluation requires a pinned test-role input."
            ));
        }
        if details.model_selection_usage != ModelSelectionUsage::NotUsed {
            pending_findings.push((
                "out_of_sample",
                "evaluation_role_used_for_selection",
                "The declared test/evaluation data was used for model selection; it is not an untouched out-of-sample evaluation.",
            ));
        }
    }
    if matches!(
        &definition.specification.integrity.walk_forward,
        Applicability::Applicable { .. }
    ) && !["training", "validation", "test"]
        .iter()
        .all(|role| actual_roles.contains(*role))
    {
        return Err(anyhow!(
            "Applicable walk-forward analysis requires training, validation, and test roles."
        ));
    }
    if let Applicability::Applicable { details } =
        &definition.specification.integrity.forward_testing
    {
        let required_role = match &details.execution_kind {
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
    insert_integrity_declarations(&tx, &run_id, &definition.specification.integrity)?;
    for (category, code, message) in pending_findings {
        insert_integrity_finding(
            &tx,
            &run_id,
            category,
            code,
            AnalysisIntegritySeverity::Warning,
            message,
        )?;
    }

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

pub fn add_run_log(conn: &mut Connection, req: &AddAnalysisRunLogReq) -> Result<AnalysisRunLogDto> {
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

fn validate_numeric(value: &str, field: &str) -> Result<f64> {
    let number: f64 = value
        .trim()
        .parse()
        .map_err(|_| anyhow!("{field} must be a finite numeric value."))?;
    if !number.is_finite() {
        return Err(anyhow!("{field} must be finite."));
    }
    Ok(number)
}

fn validate_metric_report(report: &AnalysisMetricReportDto, max_sample_size: i64) -> Result<()> {
    nonempty(&report.output_key, "Metric output key")?;
    nonempty(&report.estimand, "Metric estimand")?;
    nonempty(&report.unit, "Metric unit")?;
    nonempty(&report.multiplicity_treatment, "Multiplicity treatment")?;
    if report.sample_size <= 0 || report.sample_size > max_sample_size {
        return Err(anyhow!(
            "Metric sample size must be positive and cannot exceed the pinned input sample count."
        ));
    }
    if report.missing_data.missing_count < 0
        || report.missing_data.missing_count > report.sample_size
    {
        return Err(anyhow!(
            "Missing-data count must be between zero and the reported sample size."
        ));
    }
    nonempty(&report.missing_data.policy, "Missing-data policy")?;
    nonempty(&report.missing_data.rationale, "Missing-data rationale")?;
    let _effect = validate_numeric(&report.effect_estimate, "Effect estimate")?;
    if let Some(p_value) = report.p_value.as_deref() {
        let value = validate_numeric(p_value, "P-value")?;
        if !(0.0..=1.0).contains(&value) {
            return Err(anyhow!("P-values must be between zero and one."));
        }
    }
    if report
        .assumptions
        .iter()
        .any(|value| value.trim().is_empty())
        || report
            .exclusions
            .iter()
            .any(|value| value.trim().is_empty())
    {
        return Err(anyhow!(
            "Metric assumptions and exclusions must not contain empty entries."
        ));
    }
    match &report.uncertainty {
        UncertaintyReportDto::Interval { interval } => {
            let lower = validate_numeric(&interval.lower, "Uncertainty lower bound")?;
            let upper = validate_numeric(&interval.upper, "Uncertainty upper bound")?;
            let level = validate_numeric(&interval.level, "Uncertainty level")?;
            if lower > upper || !(0.0..=1.0).contains(&level) {
                return Err(anyhow!(
                    "Uncertainty intervals require lower <= upper and a level between zero and one."
                ));
            }
        }
        UncertaintyReportDto::NotMeaningful { reason } => {
            nonempty(reason, "Uncertainty interval rationale")?;
        }
    }
    Ok(())
}

fn parse_dataset_role(value: &str) -> Result<DatasetRole> {
    match value {
        "training" => Ok(DatasetRole::Training),
        "validation" => Ok(DatasetRole::Validation),
        "test" => Ok(DatasetRole::Test),
        "exploratory" => Ok(DatasetRole::Exploratory),
        "prospective" => Ok(DatasetRole::Prospective),
        "forward" => Ok(DatasetRole::Forward),
        _ => Err(anyhow!("Invalid analysis dataset role: {value}")),
    }
}

pub fn finalize_run(conn: &mut Connection, req: &FinalizeAnalysisRunReq) -> Result<AnalysisRunDto> {
    if !matches!(
        req.next_status,
        AnalysisRunStatus::Completed
            | AnalysisRunStatus::Failed
            | AnalysisRunStatus::Cancelled
            | AnalysisRunStatus::Inconclusive
    ) {
        return Err(anyhow!("Analysis finalization requires a terminal status."));
    }
    nonempty(&req.summary, "Analysis completion summary")?;
    if req.next_status == AnalysisRunStatus::Failed && req.errors.is_empty() {
        return Err(anyhow!(
            "A failed analysis must retain at least one error record."
        ));
    }
    if req.next_status == AnalysisRunStatus::Completed && req.outputs.is_empty() {
        return Err(anyhow!(
            "A completed analysis must retain at least one immutable output."
        ));
    }
    let now = now_us()?;
    let tx = conn.transaction()?;
    let (project_id, definition_version_id, current_status): (String, String, String) = tx
        .query_row(
            "SELECT r.project_id, r.definition_version_id,
                    (SELECT lifecycle_status FROM analysis_run_events
                     WHERE run_id = r.id ORDER BY occurred_at_us DESC, id DESC LIMIT 1)
             FROM analysis_runs r WHERE r.id = ?1",
            [&req.run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    if AnalysisRunStatus::parse(&current_status).map_err(anyhow::Error::msg)?
        != AnalysisRunStatus::Running
    {
        return Err(anyhow!("Only a running analysis can be finalized."));
    }
    let definition = query_definition_version(&tx, &definition_version_id)?;
    let method_version_id = definition.method_version_id;
    let max_sample_size: i64 = tx.query_row(
        "SELECT COALESCE(SUM(sample_count), 0)
         FROM analysis_run_inputs WHERE run_id = ?1",
        [&req.run_id],
        |row| row.get(0),
    )?;
    let mut output_ids = HashMap::new();
    let mut output_keys = HashSet::new();
    for output in &req.outputs {
        nonempty(&output.output_key, "Analysis output key")?;
        nonempty(&output.output_kind, "Analysis output kind")?;
        nonempty(&output.title, "Analysis output title")?;
        if !output_keys.insert(output.output_key.as_str()) {
            return Err(anyhow!("Analysis output keys must be unique within a run."));
        }
        match (
            output.artifact_uri.as_deref(),
            output.artifact_sha256.as_deref(),
        ) {
            (Some(uri), Some(checksum)) => {
                nonempty(uri, "External output artifact URI")?;
                valid_sha256(checksum, "Output artifact checksum")?;
            }
            (None, None) => {}
            _ => {
                return Err(anyhow!(
                    "External output artifacts require both a URI and checksum."
                ))
            }
        }
        if output.byte_length.is_some_and(|length| length < 0) {
            return Err(anyhow!("Output artifact byte length cannot be negative."));
        }
        let output_id = id();
        tx.execute(
            "INSERT INTO analysis_outputs
                (id, run_id, output_key, output_kind, title, payload_json,
                 payload_sha256, artifact_uri, artifact_sha256, media_type,
                 byte_length, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                output_id,
                req.run_id,
                output.output_key.trim(),
                output.output_kind.trim(),
                output.title.trim(),
                json(&output.payload)?,
                sha256(json(&output.payload)?.as_bytes()),
                output.artifact_uri,
                output
                    .artifact_sha256
                    .as_deref()
                    .map(str::to_ascii_lowercase),
                output.media_type,
                output.byte_length,
                now
            ],
        )?;
        output_ids.insert(output.output_key.as_str(), output_id);
    }
    for error in &req.errors {
        nonempty(&error.error_kind, "Analysis error kind")?;
        nonempty(&error.message, "Analysis error message")?;
        tx.execute(
            "INSERT INTO analysis_run_errors
                (id, run_id, error_kind, message, details_json, recorded_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id(),
                req.run_id,
                error.error_kind.trim(),
                error.message.trim(),
                json(&error.details)?,
                now
            ],
        )?;
    }
    if req.next_status == AnalysisRunStatus::Failed && req.errors.is_empty() {
        return Err(anyhow!("Failed analysis must have a persisted error."));
    }
    for report in &req.metrics {
        validate_metric_report(report, max_sample_size)?;
        let output_id = output_ids
            .get(report.output_key.as_str())
            .ok_or_else(|| anyhow!("Metric references an unknown output key."))?;
        let metric_version = query_metric_version(&tx, &report.metric_definition_version_id)?;
        if metric_version.project_id != project_id
            || !definition
                .metric_definition_version_ids
                .contains(&report.metric_definition_version_id)
        {
            return Err(anyhow!(
                "Metric version must be pinned by this analysis definition and belong to this project."
            ));
        }
        if report.unit != metric_version.unit {
            return Err(anyhow!(
                "Metric report unit must match the pinned metric definition version."
            ));
        }
        let uncertainty = match &report.uncertainty {
            UncertaintyReportDto::Interval { interval } => (
                "interval",
                Some(interval.lower.as_str()),
                Some(interval.upper.as_str()),
                Some(interval.level.as_str()),
                None,
            ),
            UncertaintyReportDto::NotMeaningful { reason } => {
                ("not_meaningful", None, None, None, Some(reason.as_str()))
            }
        };
        tx.execute(
            "INSERT INTO analysis_metrics
                (id, run_id, output_id, metric_definition_version_id,
                 method_version_id, estimand, sample_size, effect_estimate, unit,
                 uncertainty_kind, uncertainty_lower, uncertainty_upper,
                 uncertainty_level, uncertainty_reason, assumptions_json,
                 exclusions_json, missing_data_json, multiplicity_treatment, p_value)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                     ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                id(),
                req.run_id,
                output_id,
                report.metric_definition_version_id,
                method_version_id,
                report.estimand.trim(),
                report.sample_size,
                report.effect_estimate.trim(),
                report.unit.trim(),
                uncertainty.0,
                uncertainty.1,
                uncertainty.2,
                uncertainty.3,
                uncertainty.4,
                json(&report.assumptions)?,
                json(&report.exclusions)?,
                json(&report.missing_data)?,
                report.multiplicity_treatment.trim(),
                report.p_value
            ],
        )?;
    }
    let failure_summary = if req.next_status == AnalysisRunStatus::Failed {
        req.errors
            .iter()
            .map(|error| error.message.trim())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        String::new()
    };
    insert_run_event(
        &tx,
        &req.run_id,
        req.next_status,
        &req.summary,
        &failure_summary,
        req.actor_id.as_deref(),
        now,
    )?;
    tx.commit()?;
    query_run(conn, &req.run_id)
}

fn query_run_inputs(conn: &Connection, run_id: &str) -> Result<Vec<AnalysisInputDto>> {
    let rows = {
        let mut statement = conn.prepare(
            "SELECT id, dataset_version_id, dataset_checksum, role, split_group_id,
                    sample_count, transformation_lineage_json
             FROM analysis_run_inputs WHERE run_id = ?1
             ORDER BY role, dataset_version_id",
        )?;
        let rows = statement.query_map([run_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    rows.into_iter()
        .map(
            |(
                input_id,
                dataset_version_id,
                dataset_checksum,
                role,
                split_group_id,
                sample_count,
                lineage,
            )| {
                let samples = {
                    let mut statement = conn.prepare(
                        "SELECT sample_id, split_key, feature_available_at_us,
                                prediction_at_us, decision_at_us, outcome_available_at_us
                         FROM analysis_run_input_samples WHERE input_id = ?1
                         ORDER BY sample_id",
                    )?;
                    let rows = statement.query_map([&input_id], |row| {
                        Ok(AnalysisInputSampleDto {
                            sample_id: row.get(0)?,
                            split_key: row.get(1)?,
                            feature_available_at_us: row.get(2)?,
                            prediction_at_us: row.get(3)?,
                            decision_at_us: row.get(4)?,
                            outcome_available_at_us: row.get(5)?,
                        })
                    })?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()?
                };
                Ok(AnalysisInputDto {
                    id: input_id,
                    dataset_version_id,
                    dataset_checksum,
                    role: parse_dataset_role(&role)?,
                    split_group_id,
                    sample_count,
                    transformation_lineage: parse_json(&lineage)?,
                    samples,
                })
            },
        )
        .collect()
}

fn query_run_outputs(conn: &Connection, run_id: &str) -> Result<Vec<AnalysisOutputDto>> {
    let mut statement = conn.prepare(
        "SELECT id, output_key, output_kind, title, payload_json, payload_sha256,
                artifact_uri, artifact_sha256, media_type, byte_length, created_at_us
         FROM analysis_outputs WHERE run_id = ?1 ORDER BY created_at_us, id",
    )?;
    let rows = statement
        .query_map([run_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, i64>(10)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(
            |(
                id,
                output_key,
                output_kind,
                title,
                payload,
                payload_sha256,
                artifact_uri,
                artifact_sha256,
                media_type,
                byte_length,
                created_at_us,
            )| {
                Ok(AnalysisOutputDto {
                    id,
                    output_key,
                    output_kind,
                    title,
                    payload: parse_json(&payload)?,
                    payload_sha256,
                    artifact_uri,
                    artifact_sha256,
                    media_type,
                    byte_length,
                    created_at_us,
                })
            },
        )
        .collect()
}

fn query_run_metrics(conn: &Connection, run_id: &str) -> Result<Vec<AnalysisMetricDto>> {
    let mut statement = conn.prepare(
        "SELECT id, output_id, metric_definition_version_id, method_version_id,
                estimand, sample_size, effect_estimate, unit, uncertainty_kind,
                uncertainty_lower, uncertainty_upper, uncertainty_level,
                uncertainty_reason, assumptions_json, exclusions_json,
                missing_data_json, multiplicity_treatment, p_value
         FROM analysis_metrics WHERE run_id = ?1 ORDER BY id",
    )?;
    let rows = statement
        .query_map([run_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, String>(14)?,
                row.get::<_, String>(15)?,
                row.get::<_, String>(16)?,
                row.get::<_, Option<String>>(17)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(
            |(
                id,
                output_id,
                metric_definition_version_id,
                method_version_id,
                estimand,
                sample_size,
                effect_estimate,
                unit,
                uncertainty_kind,
                lower,
                upper,
                level,
                reason,
                assumptions,
                exclusions,
                missing_data,
                multiplicity_treatment,
                p_value,
            )| {
                let uncertainty = match uncertainty_kind.as_str() {
                    "interval" => UncertaintyReportDto::Interval {
                        interval: AnalysisUncertaintyIntervalDto {
                            lower: lower.ok_or_else(|| {
                                anyhow!("Stored uncertainty interval is missing its lower bound.")
                            })?,
                            upper: upper.ok_or_else(|| {
                                anyhow!("Stored uncertainty interval is missing its upper bound.")
                            })?,
                            level: level.ok_or_else(|| {
                                anyhow!("Stored uncertainty interval is missing its level.")
                            })?,
                        },
                    },
                    "not_meaningful" => UncertaintyReportDto::NotMeaningful {
                        reason: reason
                            .ok_or_else(|| anyhow!("Stored uncertainty reason is missing."))?,
                    },
                    _ => return Err(anyhow!("Invalid stored uncertainty report kind.")),
                };
                Ok(AnalysisMetricDto {
                    id,
                    output_id,
                    metric_definition_version_id,
                    method_version_id,
                    estimand,
                    sample_size,
                    effect_estimate,
                    unit,
                    uncertainty,
                    assumptions: parse_json(&assumptions)?,
                    exclusions: parse_json(&exclusions)?,
                    missing_data: parse_json(&missing_data)?,
                    multiplicity_treatment,
                    p_value,
                })
            },
        )
        .collect()
}

fn query_run_logs(conn: &Connection, run_id: &str) -> Result<Vec<AnalysisRunLogDto>> {
    let mut statement = conn.prepare(
        "SELECT id, severity, message, details_json, recorded_by, recorded_at_us
         FROM analysis_run_logs WHERE run_id = ?1 ORDER BY recorded_at_us, id",
    )?;
    let rows = statement
        .query_map([run_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(
            |(id, severity, message, details, recorded_by, recorded_at_us)| {
                Ok(AnalysisRunLogDto {
                    id,
                    severity: AnalysisLogSeverity::parse(&severity).map_err(anyhow::Error::msg)?,
                    message,
                    details: parse_json(&details)?,
                    recorded_by,
                    recorded_at_us,
                })
            },
        )
        .collect()
}

fn query_run_errors(conn: &Connection, run_id: &str) -> Result<Vec<AnalysisErrorDto>> {
    let mut statement = conn.prepare(
        "SELECT id, error_kind, message, details_json, recorded_at_us
         FROM analysis_run_errors WHERE run_id = ?1 ORDER BY recorded_at_us, id",
    )?;
    let rows = statement
        .query_map([run_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|(id, error_kind, message, details, recorded_at_us)| {
            Ok(AnalysisErrorDto {
                id,
                error_kind,
                message,
                details: parse_json(&details)?,
                recorded_at_us,
            })
        })
        .collect()
}

fn query_run_deviations(conn: &Connection, run_id: &str) -> Result<Vec<AnalysisDeviationDto>> {
    let mut statement = conn.prepare(
        "SELECT id, description, rationale, impact_assessment, recorded_by, recorded_at_us
         FROM analysis_run_deviations WHERE run_id = ?1 ORDER BY recorded_at_us, id",
    )?;
    let rows = statement
        .query_map([run_id], |row| {
            Ok(AnalysisDeviationDto {
                id: row.get(0)?,
                description: row.get(1)?,
                rationale: row.get(2)?,
                impact_assessment: row.get(3)?,
                recorded_by: row.get(4)?,
                recorded_at_us: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn query_run_integrity_findings(
    conn: &Connection,
    run_id: &str,
) -> Result<Vec<AnalysisIntegrityFindingDto>> {
    let mut statement = conn.prepare(
        "SELECT id, category, code, severity, message
         FROM analysis_integrity_findings WHERE run_id = ?1 ORDER BY category, id",
    )?;
    let rows = statement
        .query_map([run_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|(id, category, code, severity, message)| {
            let severity = match severity.as_str() {
                "warning" => AnalysisIntegritySeverity::Warning,
                "blocking" => AnalysisIntegritySeverity::Blocking,
                _ => return Err(anyhow!("Invalid stored integrity finding severity.")),
            };
            Ok(AnalysisIntegrityFindingDto {
                id,
                category,
                code,
                severity,
                message,
            })
        })
        .collect()
}

fn query_run_integrity_declarations(
    conn: &Connection,
    run_id: &str,
) -> Result<Vec<AnalysisIntegrityDeclarationDto>> {
    let mut statement = conn.prepare(
        "SELECT category, applicability, rationale, details_json
         FROM analysis_run_integrity_declarations WHERE run_id = ?1
         ORDER BY category",
    )?;
    let rows = statement
        .query_map([run_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|(category, applicability, rationale, details)| {
            Ok(AnalysisIntegrityDeclarationDto {
                category,
                applicability,
                rationale,
                details: parse_json(&details)?,
            })
        })
        .collect()
}

pub fn query_run(conn: &Connection, run_id: &str) -> Result<AnalysisRunDto> {
    let row: (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        i64,
        String,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<String>,
    ) = conn.query_row(
        "SELECT r.id, r.project_id, r.definition_version_id, r.parameters_json,
                r.environment_json, r.random_seeds_json, r.protocol_version,
                r.created_by, r.created_at_us,
                (SELECT lifecycle_status FROM analysis_run_events e
                 WHERE e.run_id = r.id ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1),
                (SELECT MIN(occurred_at_us) FROM analysis_run_events
                 WHERE run_id = r.id AND lifecycle_status = 'running'),
                (SELECT occurred_at_us FROM analysis_run_events
                 WHERE run_id = r.id AND lifecycle_status IN
                    ('completed', 'failed', 'cancelled', 'inconclusive')
                 ORDER BY occurred_at_us DESC, id DESC LIMIT 1),
                (SELECT summary FROM analysis_run_events
                 WHERE run_id = r.id ORDER BY occurred_at_us DESC, id DESC LIMIT 1),
                (SELECT NULLIF(failure_summary, '') FROM analysis_run_events
                 WHERE run_id = r.id ORDER BY occurred_at_us DESC, id DESC LIMIT 1)
         FROM analysis_runs r WHERE r.id = ?1",
        [run_id],
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
                row.get(13)?,
            ))
        },
    )?;
    let lifecycle_status = AnalysisRunStatus::parse(&row.9).map_err(anyhow::Error::msg)?;
    Ok(AnalysisRunDto {
        id: row.0,
        project_id: row.1,
        definition_version_id: row.2,
        lifecycle_status,
        parameters: parse_json(&row.3)?,
        environment: parse_json(&row.4)?,
        random_seeds: parse_json(&row.5)?,
        protocol_version: row.6,
        created_by: row.7,
        created_at_us: row.8,
        started_at_us: row.10,
        ended_at_us: row.11,
        completion_summary: row.12.filter(|summary| !summary.trim().is_empty()),
        failure_summary: row.13,
        inputs: query_run_inputs(conn, run_id)?,
        integrity_declarations: query_run_integrity_declarations(conn, run_id)?,
        integrity_findings: query_run_integrity_findings(conn, run_id)?,
        outputs: query_run_outputs(conn, run_id)?,
        metrics: query_run_metrics(conn, run_id)?,
        logs: query_run_logs(conn, run_id)?,
        errors: query_run_errors(conn, run_id)?,
        deviations: query_run_deviations(conn, run_id)?,
    })
}

fn project_exists(conn: &Connection, project_id: &str) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM research_projects WHERE id = ?1)",
        [project_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(anyhow!("Research project was not found."));
    }
    Ok(())
}

fn validate_proposition_project(
    conn: &Connection,
    project_id: &str,
    proposition: &PropositionRefDto,
) -> Result<()> {
    match proposition {
        PropositionRefDto::SourceClaim(id) => {
            let exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM source_claims WHERE id = ?1)",
                [id],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(anyhow!("Evidence proposition source claim was not found."));
            }
        }
        PropositionRefDto::HypothesisVersion(id) => {
            let owning_project: Option<String> = conn
                .query_row(
                    "SELECT h.project_id FROM hypothesis_definition_versions v
                     JOIN research_hypotheses h ON h.id = v.hypothesis_id
                     WHERE v.id = ?1",
                    [id],
                    |row| row.get(0),
                )
                .optional()?;
            if owning_project.as_deref() != Some(project_id) {
                return Err(anyhow!(
                    "Evidence hypothesis version must belong to this research project."
                ));
            }
        }
        PropositionRefDto::FindingVersion(id) => {
            let owning_project: Option<String> = conn
                .query_row(
                    "SELECT project_id FROM analysis_finding_versions WHERE id = ?1",
                    [id],
                    |row| row.get(0),
                )
                .optional()?;
            if owning_project.as_deref() != Some(project_id) {
                return Err(anyhow!(
                    "Evidence finding version must belong to this research project."
                ));
            }
        }
    }
    Ok(())
}

fn validate_artifact_project(
    conn: &Connection,
    project_id: &str,
    artifact: &EvidenceArtifactRefDto,
) -> Result<()> {
    let owning_project: Option<String> = match artifact {
        EvidenceArtifactRefDto::AnalysisOutput(id) => conn
            .query_row(
                "SELECT r.project_id FROM analysis_outputs o
                 JOIN analysis_runs r ON r.id = o.run_id WHERE o.id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?,
        EvidenceArtifactRefDto::ExperimentRun(id) => conn
            .query_row(
                "SELECT project_id FROM experiment_runs WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?,
        EvidenceArtifactRefDto::DatasetVersion(id) => conn
            .query_row(
                "SELECT d.project_id FROM experiment_dataset_versions v
                 JOIN experiment_datasets d ON d.id = v.dataset_id WHERE v.id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?,
        EvidenceArtifactRefDto::ResearchObservation(id) => conn
            .query_row(
                "SELECT project_id FROM research_observations WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?,
        EvidenceArtifactRefDto::SourceVersionSegment(id) => {
            let exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM source_version_segments WHERE id = ?1)",
                [id],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(anyhow!("Evidence source segment was not found."));
            }
            return Ok(());
        }
        EvidenceArtifactRefDto::SourceClaim(id) => {
            let exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM source_claims WHERE id = ?1)",
                [id],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(anyhow!("Evidence source claim was not found."));
            }
            return Ok(());
        }
    };
    if owning_project.as_deref() != Some(project_id) {
        return Err(anyhow!(
            "Evidence artifact must belong to this research project."
        ));
    }
    Ok(())
}

fn insert_evidence_event(
    tx: &Transaction<'_>,
    evidence_id: &str,
    status: &EvidenceStatus,
    rationale: &str,
    actor_id: Option<&str>,
    now: i64,
) -> Result<()> {
    tx.execute(
        "INSERT INTO analysis_evidence_events
            (id, evidence_id, status, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id(),
            evidence_id,
            status.as_str(),
            rationale.trim(),
            actor_id,
            now
        ],
    )?;
    Ok(())
}

fn proposition_target_values(
    proposition: &PropositionRefDto,
) -> (Option<&str>, Option<&str>, Option<&str>) {
    match proposition {
        PropositionRefDto::SourceClaim(id) => (Some(id), None, None),
        PropositionRefDto::HypothesisVersion(id) => (None, Some(id), None),
        PropositionRefDto::FindingVersion(id) => (None, None, Some(id)),
    }
}

fn artifact_target_values(
    artifact: &EvidenceArtifactRefDto,
) -> (
    Option<&str>,
    Option<&str>,
    Option<&str>,
    Option<&str>,
    Option<&str>,
    Option<&str>,
) {
    match artifact {
        EvidenceArtifactRefDto::AnalysisOutput(id) => (Some(id), None, None, None, None, None),
        EvidenceArtifactRefDto::ExperimentRun(id) => (None, Some(id), None, None, None, None),
        EvidenceArtifactRefDto::DatasetVersion(id) => (None, None, Some(id), None, None, None),
        EvidenceArtifactRefDto::ResearchObservation(id) => (None, None, None, Some(id), None, None),
        EvidenceArtifactRefDto::SourceVersionSegment(id) => {
            (None, None, None, None, Some(id), None)
        }
        EvidenceArtifactRefDto::SourceClaim(id) => (None, None, None, None, None, Some(id)),
    }
}

fn parse_proposition_refs(
    source_claim_id: Option<String>,
    hypothesis_version_id: Option<String>,
    finding_version_id: Option<String>,
) -> Result<PropositionRefDto> {
    match (source_claim_id, hypothesis_version_id, finding_version_id) {
        (Some(id), None, None) => Ok(PropositionRefDto::SourceClaim(id)),
        (None, Some(id), None) => Ok(PropositionRefDto::HypothesisVersion(id)),
        (None, None, Some(id)) => Ok(PropositionRefDto::FindingVersion(id)),
        _ => Err(anyhow!("Stored evidence proposition endpoint is invalid.")),
    }
}

fn parse_artifact_refs(
    output_id: Option<String>,
    experiment_run_id: Option<String>,
    dataset_version_id: Option<String>,
    observation_id: Option<String>,
    source_segment_id: Option<String>,
    source_claim_id: Option<String>,
) -> Result<EvidenceArtifactRefDto> {
    match (
        output_id,
        experiment_run_id,
        dataset_version_id,
        observation_id,
        source_segment_id,
        source_claim_id,
    ) {
        (Some(id), None, None, None, None, None) => Ok(EvidenceArtifactRefDto::AnalysisOutput(id)),
        (None, Some(id), None, None, None, None) => Ok(EvidenceArtifactRefDto::ExperimentRun(id)),
        (None, None, Some(id), None, None, None) => Ok(EvidenceArtifactRefDto::DatasetVersion(id)),
        (None, None, None, Some(id), None, None) => {
            Ok(EvidenceArtifactRefDto::ResearchObservation(id))
        }
        (None, None, None, None, Some(id), None) => {
            Ok(EvidenceArtifactRefDto::SourceVersionSegment(id))
        }
        (None, None, None, None, None, Some(id)) => Ok(EvidenceArtifactRefDto::SourceClaim(id)),
        _ => Err(anyhow!("Stored evidence artifact endpoint is invalid.")),
    }
}

fn query_evidence_events(conn: &Connection, evidence_id: &str) -> Result<Vec<EvidenceEventDto>> {
    let mut statement = conn.prepare(
        "SELECT id, status, rationale, actor_id, occurred_at_us
         FROM analysis_evidence_events WHERE evidence_id = ?1 ORDER BY occurred_at_us, id",
    )?;
    let rows = statement
        .query_map([evidence_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|(id, status, rationale, actor_id, occurred_at_us)| {
            Ok(EvidenceEventDto {
                id,
                status: EvidenceStatus::parse(&status).map_err(anyhow::Error::msg)?,
                rationale,
                actor_id,
                occurred_at_us,
            })
        })
        .collect()
}

pub fn query_evidence(conn: &Connection, evidence_id: &str) -> Result<EvidenceDto> {
    let row = conn.query_row(
        "SELECT id, project_id, target_source_claim_id, target_hypothesis_version_id,
                target_finding_version_id, artifact_analysis_output_id,
                artifact_experiment_run_id, artifact_dataset_version_id,
                artifact_observation_id, artifact_source_segment_id,
                artifact_source_claim_id, polarity, scope, quality_rationale,
                limitations, provenance_json, created_by, created_at_us,
                (SELECT status FROM analysis_evidence_events e
                 WHERE e.evidence_id = analysis_evidence.id
                 ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1)
         FROM analysis_evidence WHERE id = ?1",
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
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, String>(14)?,
                row.get::<_, String>(15)?,
                row.get::<_, Option<String>>(16)?,
                row.get::<_, i64>(17)?,
                row.get::<_, String>(18)?,
            ))
        },
    )?;
    Ok(EvidenceDto {
        id: row.0,
        project_id: row.1,
        proposition: parse_proposition_refs(row.2, row.3, row.4)?,
        artifact: parse_artifact_refs(row.5, row.6, row.7, row.8, row.9, row.10)?,
        polarity: EvidencePolarity::parse(&row.11).map_err(anyhow::Error::msg)?,
        scope: row.12,
        quality_rationale: row.13,
        limitations: row.14,
        provenance: parse_json(&row.15)?,
        created_by: row.16,
        created_at_us: row.17,
        status: EvidenceStatus::parse(&row.18).map_err(anyhow::Error::msg)?,
        events: query_evidence_events(conn, evidence_id)?,
    })
}

pub fn create_evidence(conn: &mut Connection, req: &CreateEvidenceReq) -> Result<EvidenceDto> {
    nonempty(&req.scope, "Evidence scope")?;
    nonempty(&req.quality_rationale, "Evidence quality rationale")?;
    nonempty(&req.limitations, "Evidence limitations")?;
    project_exists(conn, &req.project_id)?;
    validate_proposition_project(conn, &req.project_id, &req.proposition)?;
    validate_artifact_project(conn, &req.project_id, &req.artifact)?;
    if !req.provenance.is_object() {
        return Err(anyhow!("Evidence provenance must be a JSON object."));
    }
    let (target_source_claim_id, target_hypothesis_version_id, target_finding_version_id) =
        proposition_target_values(&req.proposition);
    let (
        artifact_analysis_output_id,
        artifact_experiment_run_id,
        artifact_dataset_version_id,
        artifact_observation_id,
        artifact_source_segment_id,
        artifact_source_claim_id,
    ) = artifact_target_values(&req.artifact);
    let evidence_id = id();
    let now = now_us()?;
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO analysis_evidence
            (id, project_id, target_source_claim_id, target_hypothesis_version_id,
             target_finding_version_id, artifact_analysis_output_id,
             artifact_experiment_run_id, artifact_dataset_version_id,
             artifact_observation_id, artifact_source_segment_id,
             artifact_source_claim_id, polarity, scope, quality_rationale,
             limitations, provenance_json, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                 ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
        params![
            evidence_id,
            req.project_id,
            target_source_claim_id,
            target_hypothesis_version_id,
            target_finding_version_id,
            artifact_analysis_output_id,
            artifact_experiment_run_id,
            artifact_dataset_version_id,
            artifact_observation_id,
            artifact_source_segment_id,
            artifact_source_claim_id,
            req.polarity.as_str(),
            req.scope.trim(),
            req.quality_rationale.trim(),
            req.limitations.trim(),
            json(&req.provenance)?,
            req.created_by,
            now
        ],
    )?;
    insert_evidence_event(
        &tx,
        &evidence_id,
        &EvidenceStatus::Recorded,
        "Evidence recorded.",
        req.created_by.as_deref(),
        now,
    )?;
    tx.commit()?;
    query_evidence(conn, &evidence_id)
}

pub fn transition_evidence(
    conn: &mut Connection,
    req: &TransitionEvidenceReq,
) -> Result<EvidenceDto> {
    nonempty(&req.rationale, "Evidence lifecycle rationale")?;
    let current = query_evidence(conn, &req.evidence_id)?.status;
    let allowed = matches!(
        (&current, &req.next_status),
        (EvidenceStatus::Recorded, EvidenceStatus::Assessed)
            | (EvidenceStatus::Recorded, EvidenceStatus::Contested)
            | (EvidenceStatus::Recorded, EvidenceStatus::Rejected)
            | (EvidenceStatus::Recorded, EvidenceStatus::Retracted)
            | (EvidenceStatus::Assessed, EvidenceStatus::Accepted)
            | (EvidenceStatus::Assessed, EvidenceStatus::Contested)
            | (EvidenceStatus::Assessed, EvidenceStatus::Rejected)
            | (EvidenceStatus::Assessed, EvidenceStatus::Retracted)
            | (EvidenceStatus::Accepted, EvidenceStatus::Contested)
            | (EvidenceStatus::Accepted, EvidenceStatus::Superseded)
            | (EvidenceStatus::Accepted, EvidenceStatus::Retracted)
            | (EvidenceStatus::Contested, EvidenceStatus::Accepted)
            | (EvidenceStatus::Contested, EvidenceStatus::Rejected)
            | (EvidenceStatus::Contested, EvidenceStatus::Superseded)
            | (EvidenceStatus::Contested, EvidenceStatus::Retracted)
    );
    if !allowed {
        return Err(anyhow!("Invalid evidence lifecycle transition."));
    }
    let tx = conn.transaction()?;
    insert_evidence_event(
        &tx,
        &req.evidence_id,
        &req.next_status,
        &req.rationale,
        req.actor_id.as_deref(),
        now_us()?,
    )?;
    tx.commit()?;
    query_evidence(conn, &req.evidence_id)
}

pub fn evidence_for_project(conn: &Connection, project_id: &str) -> Result<Vec<EvidenceDto>> {
    let mut statement = conn.prepare(
        "SELECT id FROM analysis_evidence WHERE project_id = ?1
         ORDER BY created_at_us, id",
    )?;
    let ids = statement
        .query_map([project_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.iter().map(|id| query_evidence(conn, id)).collect()
}

fn insert_finding_event(
    tx: &Transaction<'_>,
    version_id: &str,
    status: &FindingStatus,
    rationale: &str,
    actor_id: Option<&str>,
    now: i64,
) -> Result<()> {
    tx.execute(
        "INSERT INTO analysis_finding_events
            (id, finding_version_id, status, rationale, actor_id, occurred_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id(),
            version_id,
            status.as_str(),
            rationale.trim(),
            actor_id,
            now
        ],
    )?;
    Ok(())
}

fn finding_status_transition_is_valid(current: &FindingStatus, next: &FindingStatus) -> bool {
    matches!(
        (current, next),
        (FindingStatus::Draft, FindingStatus::Review)
            | (FindingStatus::Draft, FindingStatus::Retracted)
            | (FindingStatus::Review, FindingStatus::Accepted)
            | (FindingStatus::Review, FindingStatus::Contested)
            | (FindingStatus::Review, FindingStatus::Unsupported)
            | (FindingStatus::Review, FindingStatus::Inconclusive)
            | (FindingStatus::Review, FindingStatus::Retracted)
            | (FindingStatus::Accepted, FindingStatus::Contested)
            | (FindingStatus::Accepted, FindingStatus::Superseded)
            | (FindingStatus::Accepted, FindingStatus::Retracted)
            | (FindingStatus::Contested, FindingStatus::Accepted)
            | (FindingStatus::Contested, FindingStatus::Unsupported)
            | (FindingStatus::Contested, FindingStatus::Inconclusive)
            | (FindingStatus::Contested, FindingStatus::Superseded)
            | (FindingStatus::Contested, FindingStatus::Retracted)
            | (FindingStatus::Unsupported, FindingStatus::Contested)
            | (FindingStatus::Unsupported, FindingStatus::Superseded)
            | (FindingStatus::Unsupported, FindingStatus::Retracted)
            | (FindingStatus::Inconclusive, FindingStatus::Contested)
            | (FindingStatus::Inconclusive, FindingStatus::Superseded)
            | (FindingStatus::Inconclusive, FindingStatus::Retracted)
    )
}

fn query_finding_events(conn: &Connection, version_id: &str) -> Result<Vec<FindingEventDto>> {
    let mut statement = conn.prepare(
        "SELECT id, status, rationale, actor_id, occurred_at_us
         FROM analysis_finding_events WHERE finding_version_id = ?1
         ORDER BY occurred_at_us, id",
    )?;
    let rows = statement
        .query_map([version_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|(id, status, rationale, actor_id, occurred_at_us)| {
            Ok(FindingEventDto {
                id,
                status: FindingStatus::parse(&status).map_err(anyhow::Error::msg)?,
                rationale,
                actor_id,
                occurred_at_us,
            })
        })
        .collect()
}

fn query_finding_evidence(
    conn: &Connection,
    version_id: &str,
) -> Result<Vec<FindingEvidenceLinkDto>> {
    let mut statement = conn.prepare(
        "SELECT evidence_id, synthesis_polarity, rationale
         FROM analysis_finding_version_evidence WHERE finding_version_id = ?1
         ORDER BY evidence_id",
    )?;
    let rows = statement
        .query_map([version_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|(evidence_id, polarity, rationale)| {
            Ok(FindingEvidenceLinkDto {
                evidence_id,
                synthesis_polarity: EvidencePolarity::parse(&polarity)
                    .map_err(anyhow::Error::msg)?,
                rationale,
            })
        })
        .collect()
}

fn query_finding_relations(conn: &Connection, version_id: &str) -> Result<Vec<FindingRelationDto>> {
    let mut statement = conn.prepare(
        "SELECT id, target_version_id, relation_kind, rationale, created_by, created_at_us
         FROM analysis_finding_relations WHERE source_version_id = ?1
         ORDER BY created_at_us, id",
    )?;
    let rows = statement
        .query_map([version_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(
            |(id, target_version_id, relation_kind, rationale, created_by, created_at_us)| {
                let relation_kind = match relation_kind.as_str() {
                    "contests" => FindingRelationKind::Contests,
                    "contradicts" => FindingRelationKind::Contradicts,
                    "qualifies" => FindingRelationKind::Qualifies,
                    "supersedes" => FindingRelationKind::Supersedes,
                    _ => return Err(anyhow!("Invalid stored finding relation kind.")),
                };
                Ok(FindingRelationDto {
                    id,
                    target_version_id,
                    relation_kind,
                    rationale,
                    created_by,
                    created_at_us,
                })
            },
        )
        .collect()
}

pub fn query_finding_version(conn: &Connection, version_id: &str) -> Result<FindingVersionDto> {
    let row = conn.query_row(
        "SELECT v.id, f.id, f.project_id, v.version_no, v.parent_version_id,
                v.change_rationale, v.title, v.statement, v.target_source_claim_id,
                v.target_hypothesis_version_id, v.scope, v.applicability, v.uncertainty,
                v.limitations, v.conclusion_kind, v.content_sha256, v.created_by,
                v.created_at_us,
                (SELECT status FROM analysis_finding_events e
                 WHERE e.finding_version_id = v.id
                 ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1)
         FROM analysis_finding_versions v
         JOIN analysis_findings f ON f.id = v.finding_id
         WHERE v.id = ?1",
        [version_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, String>(14)?,
                row.get::<_, String>(15)?,
                row.get::<_, Option<String>>(16)?,
                row.get::<_, i64>(17)?,
                row.get::<_, String>(18)?,
            ))
        },
    )?;
    let analysis_run_ids = {
        let mut statement = conn.prepare(
            "SELECT run_id FROM analysis_finding_version_runs
             WHERE finding_version_id = ?1 ORDER BY run_id",
        )?;
        let rows = statement.query_map([version_id], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let conclusion_kind = match row.14.as_str() {
        "result" => FindingConclusionKind::Result,
        "null_result" => FindingConclusionKind::NullResult,
        "contradictory" => FindingConclusionKind::Contradictory,
        "inconclusive" => FindingConclusionKind::Inconclusive,
        _ => return Err(anyhow!("Invalid stored finding conclusion kind.")),
    };
    Ok(FindingVersionDto {
        id: row.0,
        finding_id: row.1,
        project_id: row.2,
        version_no: row.3,
        parent_version_id: row.4,
        change_rationale: row.5,
        title: row.6,
        statement: row.7,
        proposition: parse_proposition_refs(row.8, row.9, None)?,
        scope: row.10,
        applicability: row.11,
        uncertainty: row.12,
        limitations: row.13,
        conclusion_kind,
        analysis_run_ids,
        evidence: query_finding_evidence(conn, version_id)?,
        status: FindingStatus::parse(&row.18).map_err(anyhow::Error::msg)?,
        events: query_finding_events(conn, version_id)?,
        relations: query_finding_relations(conn, version_id)?,
        content_sha256: row.15,
        created_by: row.16,
        created_at_us: row.17,
    })
}

fn insert_finding_evidence_links(
    tx: &Transaction<'_>,
    version_id: &str,
    evidence: &[FindingEvidenceLinkReq],
) -> Result<()> {
    for link in evidence {
        nonempty(&link.rationale, "Finding evidence synthesis rationale")?;
        tx.execute(
            "INSERT INTO analysis_finding_version_evidence
                (finding_version_id, evidence_id, synthesis_polarity, rationale)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                version_id,
                link.evidence_id,
                link.synthesis_polarity.as_str(),
                link.rationale.trim()
            ],
        )?;
    }
    Ok(())
}

pub fn create_finding_version(
    conn: &mut Connection,
    req: &CreateFindingVersionReq,
) -> Result<FindingVersionDto> {
    nonempty(&req.title, "Finding title")?;
    nonempty(&req.statement, "Finding statement")?;
    nonempty(&req.scope, "Finding scope")?;
    nonempty(&req.applicability, "Finding applicability")?;
    nonempty(&req.uncertainty, "Finding uncertainty")?;
    nonempty(&req.limitations, "Finding limitations")?;
    if req.analysis_run_ids.is_empty() || req.evidence.is_empty() {
        return Err(anyhow!(
            "Findings require at least one analysis lineage record and evidence link."
        ));
    }
    if req.analysis_run_ids.iter().collect::<HashSet<_>>().len() != req.analysis_run_ids.len()
        || req
            .evidence
            .iter()
            .map(|link| &link.evidence_id)
            .collect::<HashSet<_>>()
            .len()
            != req.evidence.len()
    {
        return Err(anyhow!(
            "Finding run and evidence references must be unique."
        ));
    }
    if req.finding_id.is_some() != req.parent_version_id.is_some() {
        return Err(anyhow!(
            "A revised finding must specify both its identity and parent version."
        ));
    }
    if req.parent_version_id.is_some() {
        nonempty(&req.change_rationale, "Finding change rationale")?;
    } else if !req.change_rationale.trim().is_empty() {
        return Err(anyhow!(
            "Finding change rationale is only valid for a revised version."
        ));
    }
    project_exists(conn, &req.project_id)?;
    validate_proposition_project(conn, &req.project_id, &req.proposition)?;

    let now = now_us()?;
    let finding_id = req.finding_id.clone().unwrap_or_else(id);
    let version_id = id();
    let tx = conn.transaction()?;
    let version_no = if let Some(parent_id) = req.parent_version_id.as_deref() {
        let (parent_finding, parent_project): (String, String) = tx.query_row(
            "SELECT v.finding_id, f.project_id
             FROM analysis_finding_versions v
             JOIN analysis_findings f ON f.id = v.finding_id
             WHERE v.id = ?1",
            [parent_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if parent_finding != finding_id || parent_project != req.project_id {
            return Err(anyhow!(
                "Finding parent version must belong to this finding and project."
            ));
        }
        tx.query_row(
            "SELECT COALESCE(MAX(version_no), 0) + 1
             FROM analysis_finding_versions WHERE finding_id = ?1",
            [&finding_id],
            |row| row.get(0),
        )?
    } else {
        tx.execute(
            "INSERT INTO analysis_findings (id, project_id, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4)",
            params![finding_id, req.project_id, req.created_by, now],
        )?;
        1
    };

    let (target_source_claim_id, target_hypothesis_version_id, target_finding_version_id) =
        proposition_target_values(&req.proposition);
    if target_finding_version_id.is_some() {
        return Err(anyhow!(
            "Findings can only assert source claims or hypothesis versions, not other findings."
        ));
    }
    let content_sha256 = sha256(json(req)?.as_bytes());
    tx.execute(
        "INSERT INTO analysis_finding_versions
            (id, finding_id, version_no, parent_version_id, change_rationale, title,
             statement, target_source_claim_id, target_hypothesis_version_id,
             scope, applicability, uncertainty, limitations, conclusion_kind,
             content_sha256, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            version_id,
            finding_id,
            version_no,
            req.parent_version_id,
            req.parent_version_id
                .as_ref()
                .map(|_| req.change_rationale.trim()),
            req.title.trim(),
            req.statement.trim(),
            target_source_claim_id,
            target_hypothesis_version_id,
            req.scope.trim(),
            req.applicability.trim(),
            req.uncertainty.trim(),
            req.limitations.trim(),
            match req.conclusion_kind {
                FindingConclusionKind::Result => "result",
                FindingConclusionKind::NullResult => "null_result",
                FindingConclusionKind::Contradictory => "contradictory",
                FindingConclusionKind::Inconclusive => "inconclusive",
            },
            content_sha256,
            req.created_by,
            now
        ],
    )?;
    for run_id in &req.analysis_run_ids {
        let (run_project, run_status): (String, String) = tx.query_row(
            "SELECT r.project_id,
                    (SELECT lifecycle_status FROM analysis_run_events e
                     WHERE e.run_id = r.id ORDER BY e.occurred_at_us DESC, e.id DESC LIMIT 1)
             FROM analysis_runs r WHERE r.id = ?1",
            [run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if run_project != req.project_id
            || !matches!(run_status.as_str(), "completed" | "inconclusive")
        {
            return Err(anyhow!(
                "Finding analysis lineage must reference completed or inconclusive runs in this project."
            ));
        }
        tx.execute(
            "INSERT INTO analysis_finding_version_runs (finding_version_id, run_id)
             VALUES (?1, ?2)",
            params![version_id, run_id],
        )?;
    }
    for link in &req.evidence {
        let evidence = query_evidence(&tx, &link.evidence_id)?;
        if evidence.project_id != req.project_id || evidence.proposition != req.proposition {
            return Err(anyhow!(
                "Finding evidence must reference the same proposition in this project."
            ));
        }
    }
    insert_finding_evidence_links(&tx, &version_id, &req.evidence)?;
    insert_finding_event(
        &tx,
        &version_id,
        &FindingStatus::Draft,
        "Finding version created.",
        req.created_by.as_deref(),
        now,
    )?;
    tx.commit()?;
    query_finding_version(conn, &version_id)
}

pub fn transition_finding(
    conn: &mut Connection,
    req: &TransitionFindingReq,
) -> Result<FindingVersionDto> {
    nonempty(&req.rationale, "Finding lifecycle rationale")?;
    let current = query_finding_version(conn, &req.version_id)?.status;
    if !finding_status_transition_is_valid(&current, &req.next_status) {
        return Err(anyhow!("Invalid finding lifecycle transition."));
    }
    let tx = conn.transaction()?;
    insert_finding_event(
        &tx,
        &req.version_id,
        &req.next_status,
        &req.rationale,
        req.actor_id.as_deref(),
        now_us()?,
    )?;
    tx.commit()?;
    query_finding_version(conn, &req.version_id)
}

pub fn create_finding_relation(
    conn: &mut Connection,
    req: &CreateFindingRelationReq,
) -> Result<FindingRelationDto> {
    nonempty(&req.rationale, "Finding relation rationale")?;
    if req.source_version_id == req.target_version_id {
        return Err(anyhow!("A finding version cannot relate to itself."));
    }
    let source = query_finding_version(conn, &req.source_version_id)?;
    let target = query_finding_version(conn, &req.target_version_id)?;
    if source.project_id != target.project_id {
        return Err(anyhow!(
            "Finding relations cannot cross project boundaries."
        ));
    }
    if req.relation_kind == FindingRelationKind::Supersedes
        && source.finding_id != target.finding_id
    {
        return Err(anyhow!(
            "Supersession is only valid between versions of the same finding identity."
        ));
    }
    let relation_id = id();
    let created_at_us = now_us()?;
    conn.execute(
        "INSERT INTO analysis_finding_relations
            (id, source_version_id, target_version_id, relation_kind,
             rationale, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            relation_id,
            req.source_version_id,
            req.target_version_id,
            req.relation_kind.as_str(),
            req.rationale.trim(),
            req.created_by,
            created_at_us
        ],
    )?;
    Ok(FindingRelationDto {
        id: relation_id,
        target_version_id: req.target_version_id.clone(),
        relation_kind: req.relation_kind.clone(),
        rationale: req.rationale.trim().to_string(),
        created_by: req.created_by.clone(),
        created_at_us,
    })
}

pub fn findings_for_project(conn: &Connection, project_id: &str) -> Result<Vec<FindingVersionDto>> {
    let mut statement = conn.prepare(
        "SELECT v.id FROM analysis_finding_versions v
         JOIN analysis_findings f ON f.id = v.finding_id
         WHERE f.project_id = ?1 ORDER BY v.created_at_us, v.id",
    )?;
    let ids = statement
        .query_map([project_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.iter()
        .map(|version_id| query_finding_version(conn, version_id))
        .collect()
}

pub fn workspace(conn: &Connection, project_id: &str) -> Result<AnalysisWorkspaceDto> {
    project_exists(conn, project_id)?;

    let method_ids = {
        let mut statement = conn.prepare(
            "SELECT v.id FROM analysis_method_versions v
             JOIN analysis_methods m ON m.id = v.method_id
             WHERE m.project_id = ?1 ORDER BY m.name, v.version_no",
        )?;
        let rows = statement.query_map([project_id], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let methods = method_ids
        .iter()
        .map(|version_id| query_method_version(conn, version_id))
        .collect::<Result<Vec<_>>>()?;

    let metric_ids = {
        let mut statement = conn.prepare(
            "SELECT v.id FROM analysis_metric_definition_versions v
             JOIN analysis_metric_definitions m ON m.id = v.metric_definition_id
             WHERE m.project_id = ?1 ORDER BY m.metric_key, v.version_no",
        )?;
        let rows = statement.query_map([project_id], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let metrics = metric_ids
        .iter()
        .map(|version_id| query_metric_version(conn, version_id))
        .collect::<Result<Vec<_>>>()?;

    let definition_ids = {
        let mut statement = conn.prepare(
            "SELECT v.id FROM analysis_definition_versions v
             JOIN analysis_definitions d ON d.id = v.definition_id
             WHERE d.project_id = ?1 ORDER BY d.title, v.version_no",
        )?;
        let rows = statement.query_map([project_id], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let definitions = definition_ids
        .iter()
        .map(|version_id| query_definition_version(conn, version_id))
        .collect::<Result<Vec<_>>>()?;

    let run_ids = {
        let mut statement = conn.prepare(
            "SELECT id FROM analysis_runs WHERE project_id = ?1
             ORDER BY created_at_us, id",
        )?;
        let rows = statement.query_map([project_id], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let runs = run_ids
        .iter()
        .map(|run_id| query_run(conn, run_id))
        .collect::<Result<Vec<_>>>()?;

    Ok(AnalysisWorkspaceDto {
        methods,
        metrics,
        definitions,
        runs,
        evidence: evidence_for_project(conn, project_id)?,
        findings: findings_for_project(conn, project_id)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment::{
        AddDatasetSamplesReq, AddDatasetSplitGroupReq, CreateDatasetReq, CreateDatasetVersionReq,
        DataSampleDto, DatasetFieldDefinitionDto, DatasetSampleDto, DatasetSampleSchemaDto,
        DatasetSampleValueKind, DatasetSplitAssignmentReq,
    };
    use std::collections::BTreeMap;
    use std::fs;

    fn test_project(conn: &Connection, project_id: &str) {
        conn.execute(
            "INSERT INTO research_projects
                (id, title, aim, lifecycle_status, created_by, created_at_us, updated_at_us)
             VALUES (?1, 'Analysis integration project', '', 'active', NULL, 1, 1)",
            [project_id],
        )
        .expect("insert test project");
    }

    fn integrity_plan() -> StatisticalIntegrityPlanDto {
        StatisticalIntegrityPlanDto {
            overfitting: Applicability::NotAssessed { rationale: None },
            data_leakage: Applicability::NotAssessed { rationale: None },
            selection_bias: Applicability::NotAssessed { rationale: None },
            look_ahead: Applicability::Applicable {
                details: LookAheadSafeguardDto {
                    feature_availability_policy: "Features available by prediction time only."
                        .to_string(),
                    prediction_timestamp_policy: "UTC timestamp at prediction.".to_string(),
                    decision_timestamp_policy: "UTC timestamp at decision.".to_string(),
                },
            },
            multiple_testing: Applicability::NotAssessed { rationale: None },
            dataset_separation: Applicability::Applicable {
                details: DatasetSeparationSafeguardDto {
                    require_disjoint_splits: true,
                    selection_rationale: "Split membership is frozen in the dataset version."
                        .to_string(),
                },
            },
            out_of_sample: Applicability::NotAssessed { rationale: None },
            walk_forward: Applicability::NotAssessed { rationale: None },
            forward_testing: Applicability::NotAssessed { rationale: None },
            reproducibility: Applicability::Applicable {
                details: ReproducibilitySafeguardDto {
                    protocol_version: "analysis-test-1".to_string(),
                    raw_input_checksums_required: true,
                    transformation_lineage_required: true,
                    code_and_environment_required: true,
                    parameters_and_seeds_required: true,
                    timestamps_and_deviations_required: true,
                },
            },
        }
    }

    fn create_locked_definition(
        conn: &mut Connection,
        project_id: &str,
    ) -> (AnalysisDefinitionVersionDto, MetricDefinitionVersionDto) {
        let method = create_method_version(
            conn,
            &CreateAnalysisMethodVersionReq {
                project_id: project_id.to_string(),
                method_id: None,
                name: "Test estimator".to_string(),
                version_label: "1.0".to_string(),
                code_uri: "file:///analysis.py".to_string(),
                code_revision: "git:abc123".to_string(),
                dependency_lock_sha256: None,
                environment: serde_json::json!({"python": "3.12"}),
                assumptions: vec!["Independent observations.".to_string()],
                test_suite_uri: "file:///tests/test_analysis.py".to_string(),
                test_suite_sha256: "a".repeat(64),
                tests_total: 4,
                tests_passed: 4,
                created_by: Some("reviewer".to_string()),
            },
        )
        .expect("create method version");
        let metric = create_metric_definition_version(
            conn,
            &CreateMetricDefinitionVersionReq {
                project_id: project_id.to_string(),
                metric_definition_id: None,
                key: "mean_difference".to_string(),
                name: "Mean difference".to_string(),
                version_label: "1.0".to_string(),
                formula: "mean(treatment) - mean(control)".to_string(),
                unit: "score".to_string(),
                test_suite_uri: "file:///tests/test_metric.py".to_string(),
                test_suite_sha256: "c".repeat(64),
                tests_total: 3,
                tests_passed: 3,
                created_by: Some("reviewer".to_string()),
            },
        )
        .expect("create metric version");
        let definition = create_definition_version(
            conn,
            &CreateAnalysisDefinitionVersionReq {
                project_id: project_id.to_string(),
                definition_id: None,
                parent_version_id: None,
                title: "Treatment effect".to_string(),
                version_label: "1.0".to_string(),
                change_rationale: String::new(),
                method_version_id: method.id,
                metric_definition_version_ids: vec![metric.id.clone()],
                specification: AnalysisDefinitionSpecificationDto {
                    protocol_version: "analysis-test-1".to_string(),
                    required_input_roles: vec![DatasetRole::Exploratory],
                    parameters_schema: serde_json::json!({
                        "type": "object",
                        "properties": {"threshold": {"type": "number"}},
                        "required": ["threshold"],
                        "additionalProperties": false
                    }),
                    integrity: integrity_plan(),
                },
                created_by: Some("reviewer".to_string()),
            },
        )
        .expect("create definition version");
        transition_definition(
            conn,
            &TransitionAnalysisDefinitionReq {
                version_id: definition.id.clone(),
                next_status: AnalysisDefinitionStatus::Review,
                rationale: "Ready for review.".to_string(),
                actor_id: Some("reviewer".to_string()),
            },
        )
        .expect("submit definition for review");
        let definition = transition_definition(
            conn,
            &TransitionAnalysisDefinitionReq {
                version_id: definition.id,
                next_status: AnalysisDefinitionStatus::Locked,
                rationale: "Method and metric versions passed their declared tests.".to_string(),
                actor_id: Some("reviewer".to_string()),
            },
        )
        .expect("lock definition");
        (definition, metric)
    }

    fn create_dataset(conn: &mut Connection, project_id: &str) -> String {
        let dataset = crate::experiment::repository::create_dataset(
            conn,
            &CreateDatasetReq {
                project_id: project_id.to_string(),
                title: "Analysis input".to_string(),
                description: String::new(),
                created_by: None,
            },
        )
        .expect("create dataset");
        let version = crate::experiment::repository::create_dataset_version(
            conn,
            &CreateDatasetVersionReq {
                dataset_id: dataset.id,
                schema: DatasetSampleSchemaDto {
                    schema_id: "analysis-test".to_string(),
                    schema_version: 1,
                    fields: vec![DatasetFieldDefinitionDto {
                        key: "value".to_string(),
                        value_kind: DatasetSampleValueKind::Decimal,
                        required: true,
                        unit: Some("score".to_string()),
                    }],
                    allow_additional_fields: false,
                },
                expected_sample_count: 1,
                created_by: None,
            },
        )
        .expect("create dataset version");
        crate::experiment::repository::add_dataset_samples(
            conn,
            &AddDatasetSamplesReq {
                dataset_version_id: version.id.clone(),
                samples: vec![DatasetSampleDto::DataSample(DataSampleDto {
                    sample_id: "sample-1".to_string(),
                    dataset_version_id: version.id.clone(),
                    occurred_at_us: Some(110),
                    entity_ref: None,
                    values: BTreeMap::from([(
                        "value".to_string(),
                        crate::experiment::DatasetSampleValue::Decimal {
                            value: "2.5".to_string(),
                            unit: "score".to_string(),
                            uncertainty: None,
                        },
                    )]),
                    provenance: Vec::new(),
                })],
            },
        )
        .expect("add sample");
        let split = crate::experiment::repository::add_dataset_split_group(
            conn,
            &AddDatasetSplitGroupReq {
                dataset_version_id: version.id.clone(),
                split_group_key: "analysis-test-splits".to_string(),
                require_disjoint: true,
                assignments: vec![DatasetSplitAssignmentReq {
                    sample_id: "sample-1".to_string(),
                    role: DatasetRole::Exploratory,
                    split_key: "all".to_string(),
                }],
            },
        )
        .expect("create dataset split group");
        let material = crate::experiment::repository::dataset_seal_material(conn, &version.id)
            .expect("load dataset seal material");
        crate::experiment::repository::seal_dataset_version(
            conn,
            &version.id,
            material.staged_revision,
            &"d".repeat(64),
        )
        .expect("seal dataset version");
        serde_json::json!({
            "datasetVersionId": version.id,
            "splitGroupId": split.id
        })
        .to_string()
    }

    fn create_source_claim(conn: &Connection) -> String {
        let source_id = id();
        let version_id = id();
        let segment_id = id();
        let extraction_id = id();
        let claim_id = id();
        let now = now_us().expect("clock");
        conn.execute(
            "INSERT INTO sources (id, kind, title, created_at, updated_at)
             VALUES (?1, 'document', 'Test source', ?2, ?2)",
            params![source_id, now],
        )
        .expect("insert source");
        conn.execute(
            "INSERT INTO source_versions
                (id, source_id, capture_kind, edition_label, captured_at_us,
                 metadata_json, created_by, created_at_us)
             VALUES (?1, ?2, 'edition', '1st edition', ?3, '{}', 'test', ?3)",
            params![version_id, source_id, now],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO source_version_segments
                (id, source_id, source_version_id, label, location_kind, created_at_us)
             VALUES (?1, ?2, ?3, 'Page 1', 'page', ?4)",
            params![segment_id, source_id, version_id, now],
        )
        .expect("insert source segment");
        conn.execute(
            "INSERT INTO knowledge_extractions
                (id, source_id, source_version_id, source_segment_id, kind, content,
                 review_status, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, 'source_claim', 'Claim text',
                     'unreviewed', 'test', ?5)",
            params![extraction_id, source_id, version_id, segment_id, now],
        )
        .expect("insert source extraction");
        conn.execute(
            "INSERT INTO extraction_reviews
                (id, extraction_id, review_status, reviewer_id, reviewed_at_us, rationale)
             VALUES (?1, ?2, 'accepted', 'reviewer', ?3, 'Extraction quality reviewed.')",
            params![id(), extraction_id, now],
        )
        .expect("review source extraction");
        conn.execute(
            "INSERT INTO source_claims
                (id, source_id, source_version_id, extraction_id, statement, created_at_us)
             VALUES (?1, ?2, ?3, ?4, 'Claim text', ?5)",
            params![claim_id, source_id, version_id, extraction_id, now],
        )
        .expect("insert reviewed source claim");
        claim_id
    }

    fn create_run_request(
        definition_version_id: &str,
        dataset_version_id: &str,
        split_group_id: &str,
    ) -> CreateAnalysisRunReq {
        CreateAnalysisRunReq {
            definition_version_id: definition_version_id.to_string(),
            parameters: serde_json::json!({"threshold": 1.0}),
            environment: serde_json::json!({"runtime": "python 3.12"}),
            random_seeds: vec!["seed-1".to_string()],
            inputs: vec![AnalysisInputReq {
                dataset_version_id: dataset_version_id.to_string(),
                role: DatasetRole::Exploratory,
                split_group_id: Some(split_group_id.to_string()),
                samples: vec![AnalysisInputSampleReq {
                    sample_id: "sample-1".to_string(),
                    feature_available_at_us: Some(100),
                    prediction_at_us: Some(110),
                    decision_at_us: Some(120),
                    outcome_available_at_us: Some(130),
                }],
            }],
            created_by: Some("analyst".to_string()),
        }
    }

    #[test]
    fn migration_is_idempotent_and_preserves_analysis_provenance_after_reopen() {
        let database_path =
            std::env::temp_dir().join(format!("vora-analysis-{}.sqlite", Uuid::now_v7()));
        let result = (|| -> Result<()> {
            let mut conn = Connection::open(&database_path)?;
            crate::mind::portable_db::initialize_workspace_schema(&conn)
                .map_err(|error| anyhow!(error.to_string()))?;
            crate::mind::portable_db::initialize_workspace_schema(&conn)
                .map_err(|error| anyhow!(error.to_string()))?;
            let project_id = id();
            test_project(&conn, &project_id);

            let (definition, metric) = create_locked_definition(&mut conn, &project_id);
            let dataset_json = create_dataset(&mut conn, &project_id);
            let dataset_pin: serde_json::Value = parse_json(&dataset_json)?;
            let dataset_version_id = dataset_pin["datasetVersionId"]
                .as_str()
                .ok_or_else(|| anyhow!("dataset version ID missing"))?;
            let split_group_id = dataset_pin["splitGroupId"]
                .as_str()
                .ok_or_else(|| anyhow!("split group ID missing"))?;

            let run_req = create_run_request(&definition.id, dataset_version_id, split_group_id);
            let mut duplicate_sample_req = run_req.clone();
            let duplicate_sample = duplicate_sample_req.inputs[0].samples[0].clone();
            duplicate_sample_req.inputs[0]
                .samples
                .push(duplicate_sample);
            assert!(create_run(&mut conn, &duplicate_sample_req).is_err());

            let queued = create_run(&mut conn, &run_req)?;
            assert_eq!(queued.lifecycle_status, AnalysisRunStatus::Queued);
            assert_eq!(queued.inputs[0].dataset_version_id, dataset_version_id);
            assert_eq!(queued.inputs[0].samples[0].sample_id, "sample-1");
            assert_eq!(queued.definition_version_id, definition.id);
            assert!(queued
                .integrity_findings
                .iter()
                .any(|finding| { finding.code == "control_not_assessed:overfitting" }));
            assert!(transition_definition(
                &mut conn,
                &TransitionAnalysisDefinitionReq {
                    version_id: definition.id.clone(),
                    next_status: AnalysisDefinitionStatus::Draft,
                    rationale: "Attempt to mutate a locked definition.".to_string(),
                    actor_id: Some("reviewer".to_string()),
                }
            )
            .is_err());

            let running = start_run(&mut conn, &queued.id, Some("analyst"))?;
            assert_eq!(running.lifecycle_status, AnalysisRunStatus::Running);
            let completed = finalize_run(
                &mut conn,
                &FinalizeAnalysisRunReq {
                    run_id: queued.id.clone(),
                    next_status: AnalysisRunStatus::Completed,
                    summary: "External estimate captured.".to_string(),
                    outputs: vec![AnalysisOutputReq {
                        output_key: "estimate".to_string(),
                        output_kind: "report".to_string(),
                        title: "Effect estimate".to_string(),
                        payload: serde_json::json!({"source": "external"}),
                        artifact_uri: Some("file:///external/estimate.json".to_string()),
                        artifact_sha256: Some("e".repeat(64)),
                        media_type: Some("application/json".to_string()),
                        byte_length: Some(128),
                    }],
                    metrics: vec![AnalysisMetricReportDto {
                        output_key: "estimate".to_string(),
                        metric_definition_version_id: metric.id.clone(),
                        estimand: "Mean treatment effect".to_string(),
                        sample_size: 1,
                        effect_estimate: "0.5".to_string(),
                        unit: "score".to_string(),
                        uncertainty: UncertaintyReportDto::Interval {
                            interval: AnalysisUncertaintyIntervalDto {
                                lower: "0.1".to_string(),
                                upper: "0.8".to_string(),
                                level: "0.95".to_string(),
                            },
                        },
                        assumptions: vec!["Independent observations.".to_string()],
                        exclusions: Vec::new(),
                        missing_data: MissingDataReportDto {
                            policy: "complete_case".to_string(),
                            missing_count: 0,
                            rationale: "No values were missing.".to_string(),
                        },
                        multiplicity_treatment: "No correction; single primary metric.".to_string(),
                        p_value: Some("0.03".to_string()),
                    }],
                    errors: Vec::new(),
                    actor_id: Some("analyst".to_string()),
                },
            )?;
            assert_eq!(completed.lifecycle_status, AnalysisRunStatus::Completed);
            assert_eq!(completed.metrics[0].sample_size, 1);
            assert_eq!(completed.inputs[0].dataset_checksum, "d".repeat(64));
            assert!(finalize_run(
                &mut conn,
                &FinalizeAnalysisRunReq {
                    run_id: completed.id.clone(),
                    next_status: AnalysisRunStatus::Completed,
                    summary: "Attempt to overwrite a finalized run.".to_string(),
                    outputs: Vec::new(),
                    metrics: Vec::new(),
                    errors: Vec::new(),
                    actor_id: Some("analyst".to_string()),
                }
            )
            .is_err());

            let failed_run = create_run(&mut conn, &run_req)?;
            start_run(&mut conn, &failed_run.id, Some("analyst"))?;
            let failed = finalize_run(
                &mut conn,
                &FinalizeAnalysisRunReq {
                    run_id: failed_run.id.clone(),
                    next_status: AnalysisRunStatus::Failed,
                    summary: "External analysis failed.".to_string(),
                    outputs: Vec::new(),
                    metrics: Vec::new(),
                    errors: vec![AnalysisErrorReq {
                        error_kind: "runtime".to_string(),
                        message: "Numerical solver did not converge.".to_string(),
                        details: serde_json::json!({"attempt": 1}),
                    }],
                    actor_id: Some("analyst".to_string()),
                },
            )?;
            assert_eq!(failed.lifecycle_status, AnalysisRunStatus::Failed);
            assert_eq!(failed.errors.len(), 1);
            assert_ne!(completed.id, failed.id);

            let source_claim_id = create_source_claim(&conn);
            let evidence_request = |polarity| CreateEvidenceReq {
                project_id: project_id.clone(),
                proposition: PropositionRefDto::SourceClaim(source_claim_id.clone()),
                artifact: EvidenceArtifactRefDto::AnalysisOutput(completed.outputs[0].id.clone()),
                polarity,
                scope: "The tested population and period.".to_string(),
                quality_rationale: "Method report and input pins are inspectable.".to_string(),
                limitations: "External execution is not independently recomputed.".to_string(),
                provenance: serde_json::json!({"runId": completed.id}),
                created_by: Some("analyst".to_string()),
            };
            let supports =
                create_evidence(&mut conn, &evidence_request(EvidencePolarity::Supports))?;
            let contradicts =
                create_evidence(&mut conn, &evidence_request(EvidencePolarity::Contradicts))?;
            assert_eq!(supports.polarity, EvidencePolarity::Supports);
            assert_eq!(contradicts.polarity, EvidencePolarity::Contradicts);

            let finding = create_finding_version(
                &mut conn,
                &CreateFindingVersionReq {
                    project_id: project_id.clone(),
                    finding_id: None,
                    parent_version_id: None,
                    title: "Mixed evidence".to_string(),
                    statement: "Evidence does not support a settled direction.".to_string(),
                    proposition: PropositionRefDto::SourceClaim(source_claim_id),
                    scope: "Test population.".to_string(),
                    applicability: "Exploratory scope only.".to_string(),
                    uncertainty: "Wide uncertainty interval.".to_string(),
                    limitations: "Only one dataset sample.".to_string(),
                    conclusion_kind: FindingConclusionKind::Contradictory,
                    analysis_run_ids: vec![completed.id.clone()],
                    evidence: vec![
                        FindingEvidenceLinkReq {
                            evidence_id: supports.id,
                            synthesis_polarity: EvidencePolarity::Supports,
                            rationale: "The point estimate aligns.".to_string(),
                        },
                        FindingEvidenceLinkReq {
                            evidence_id: contradicts.id,
                            synthesis_polarity: EvidencePolarity::Contradicts,
                            rationale: "The evidence scope limits the claim.".to_string(),
                        },
                    ],
                    change_rationale: String::new(),
                    created_by: Some("analyst".to_string()),
                },
            )?;
            let reviewed = transition_finding(
                &mut conn,
                &TransitionFindingReq {
                    version_id: finding.id.clone(),
                    next_status: FindingStatus::Review,
                    rationale: "Reviewing synthesis.".to_string(),
                    actor_id: Some("reviewer".to_string()),
                },
            )?;
            assert_eq!(reviewed.status, FindingStatus::Review);
            let accepted_assessment = transition_finding(
                &mut conn,
                &TransitionFindingReq {
                    version_id: finding.id.clone(),
                    next_status: FindingStatus::Accepted,
                    rationale: "Accepted as a scoped finding, not a scientific fact.".to_string(),
                    actor_id: Some("reviewer".to_string()),
                },
            )?;
            assert_eq!(accepted_assessment.status, FindingStatus::Accepted);
            let revised = create_finding_version(
                &mut conn,
                &CreateFindingVersionReq {
                    project_id: project_id.clone(),
                    finding_id: Some(finding.finding_id.clone()),
                    parent_version_id: Some(finding.id.clone()),
                    title: "Mixed evidence".to_string(),
                    statement: "A revised scoped assessment retains both evidence directions."
                        .to_string(),
                    proposition: finding.proposition.clone(),
                    scope: finding.scope.clone(),
                    applicability: finding.applicability.clone(),
                    uncertainty: finding.uncertainty.clone(),
                    limitations: finding.limitations.clone(),
                    conclusion_kind: FindingConclusionKind::Contradictory,
                    analysis_run_ids: finding.analysis_run_ids.clone(),
                    evidence: finding
                        .evidence
                        .iter()
                        .map(|link| FindingEvidenceLinkReq {
                            evidence_id: link.evidence_id.clone(),
                            synthesis_polarity: link.synthesis_polarity.clone(),
                            rationale: link.rationale.clone(),
                        })
                        .collect(),
                    change_rationale: "Clarified scope without resolving conflict.".to_string(),
                    created_by: Some("analyst".to_string()),
                },
            )?;
            assert_eq!(revised.version_no, 2);
            assert_eq!(revised.parent_version_id, Some(finding.id.clone()));

            let workspace = workspace(&conn, &project_id)?;
            assert_eq!(workspace.runs.len(), 2);
            assert_eq!(workspace.evidence.len(), 2);
            assert_eq!(workspace.findings.len(), 2);

            drop(conn);
            let reopened = Connection::open(&database_path)?;
            crate::mind::portable_db::initialize_workspace_schema(&reopened)
                .map_err(|error| anyhow!(error.to_string()))?;
            let reopened_run = query_run(&reopened, &completed.id)?;
            assert_eq!(
                reopened_run.outputs[0].payload,
                serde_json::json!({"source": "external"})
            );
            assert_eq!(reopened_run.inputs[0].samples[0].sample_id, "sample-1");
            assert_eq!(
                query_finding_version(&reopened, &revised.id)?.parent_version_id,
                Some(finding.id)
            );
            assert_eq!(evidence_for_project(&reopened, &project_id)?.len(), 2);
            Ok(())
        })();
        let _ = fs::remove_file(&database_path);
        result.expect("analysis migration and lifecycle integration should succeed");
    }

    #[test]
    fn temporal_leakage_and_incomplete_statistical_reports_are_rejected() {
        let incomplete_plan: std::result::Result<StatisticalIntegrityPlanDto, _> =
            serde_json::from_value(serde_json::json!({}));
        assert!(incomplete_plan.is_err());

        let mut plan = integrity_plan();
        plan.overfitting = Applicability::NotApplicable {
            reason: "   ".to_string(),
        };
        assert!(validate_integrity_plan(&plan).is_err());

        let mut sample = AnalysisInputSampleReq {
            sample_id: "sample".to_string(),
            feature_available_at_us: Some(20),
            prediction_at_us: Some(10),
            decision_at_us: Some(30),
            outcome_available_at_us: Some(40),
        };
        assert!(validate_temporal_sample(&sample, true).is_err());
        sample.feature_available_at_us = None;
        assert!(validate_temporal_sample(&sample, true).is_err());
        sample.feature_available_at_us = Some(10);
        sample.prediction_at_us = Some(20);
        sample.decision_at_us = Some(30);
        sample.outcome_available_at_us = Some(25);
        assert!(validate_temporal_sample(&sample, true).is_err());
        let report = AnalysisMetricReportDto {
            output_key: "out".to_string(),
            metric_definition_version_id: "metric".to_string(),
            estimand: "Effect".to_string(),
            sample_size: 1,
            effect_estimate: "NaN".to_string(),
            unit: "score".to_string(),
            uncertainty: UncertaintyReportDto::NotMeaningful {
                reason: "none".to_string(),
            },
            assumptions: Vec::new(),
            exclusions: Vec::new(),
            missing_data: MissingDataReportDto {
                policy: "complete_case".to_string(),
                missing_count: 0,
                rationale: "No missing values.".to_string(),
            },
            multiplicity_treatment: "single metric".to_string(),
            p_value: None,
        };
        assert!(validate_metric_report(&report, 1).is_err());
    }
}
