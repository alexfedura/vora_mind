use super::*;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const EXPERIMENT_SCHEMA: &str = include_str!("schema.sql");
const EXPERIMENT_MIGRATION_ID: &str = "phase3-experiments-001";

#[derive(Debug)]
pub struct ExperimentError(String);

impl ExperimentError {
    pub fn message(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for ExperimentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ExperimentError {}

impl From<rusqlite::Error> for ExperimentError {
    fn from(error: rusqlite::Error) -> Self {
        Self(error.to_string())
    }
}

impl From<serde_json::Error> for ExperimentError {
    fn from(error: serde_json::Error) -> Self {
        Self(error.to_string())
    }
}

impl From<std::io::Error> for ExperimentError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ExperimentError>;

fn invalid(message: impl Into<String>) -> ExperimentError {
    ExperimentError(message.into())
}

fn migration_checksum() -> String {
    format!("{:x}", Sha256::digest(EXPERIMENT_SCHEMA.as_bytes()))
}

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS experiment_schema_migrations (
             version INTEGER PRIMARY KEY,
             migration_id TEXT NOT NULL UNIQUE,
             checksum TEXT NOT NULL,
             applied_at_us INTEGER NOT NULL
         );",
    )?;
    let foreign_keys_enabled: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    if foreign_keys_enabled != 1 {
        return Err(invalid(
            "SQLite foreign-key enforcement is required for Experiment writes.",
        ));
    }
    let applied: Option<(String, String)> = conn
        .query_row(
            "SELECT migration_id, checksum
             FROM experiment_schema_migrations WHERE version = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let checksum = migration_checksum();
    if let Some((migration_id, applied_checksum)) = applied {
        if migration_id != EXPERIMENT_MIGRATION_ID || applied_checksum != checksum {
            return Err(invalid(
                "Experiment migration 1 differs from the applied migration; add a new numbered migration.",
            ));
        }
        return Ok(());
    }
    conn.execute_batch("BEGIN IMMEDIATE")?;
    let migration_result = (|| -> Result<()> {
        conn.execute_batch(EXPERIMENT_SCHEMA)?;
        conn.execute(
            "INSERT INTO experiment_schema_migrations
                (version, migration_id, checksum, applied_at_us)
             VALUES (1, ?1, ?2, ?3)",
            params![
                EXPERIMENT_MIGRATION_ID,
                checksum,
                now_ts_us().map_err(invalid)?
            ],
        )?;
        Ok(())
    })();
    match migration_result {
        Ok(()) => conn.execute_batch("COMMIT")?,
        Err(error) => {
            conn.execute_batch("ROLLBACK")?;
            return Err(error);
        }
    }
    Ok(())
}

fn new_id() -> String {
    Uuid::now_v7().to_string()
}

fn json_string<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(Into::into)
}

fn sha256_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn query_design_version(conn: &Connection, version_id: &str) -> Result<ExperimentDesignVersionDto> {
    let row = conn.query_row(
        "SELECT v.id, v.design_id, d.project_id, COALESCE(t.title, d.title),
                v.version_no, v.description, v.protocol, v.lifecycle_status,
                s.specification_json, s.specification_sha256,
                l.parent_version_id, l.change_rationale, v.created_by, v.created_at_us,
                p.id, p.specification_sha256, p.reviewer_id, p.rationale, p.preregistered_at_us
         FROM research_experiment_design_versions v
         JOIN research_experiment_designs d ON d.id = v.design_id
         LEFT JOIN experiment_design_version_titles t ON t.design_version_id = v.id
         LEFT JOIN experiment_design_version_specs s ON s.design_version_id = v.id
         LEFT JOIN experiment_design_version_lineage l ON l.design_version_id = v.id
         LEFT JOIN experiment_preregistrations p ON p.design_version_id = v.id
         WHERE v.id = ?1",
        [version_id],
        |row| {
            let lifecycle_status = parse_design_status(&row.get::<_, String>(7)?)?;
            let specification_json: Option<String> = row.get(8)?;
            let specification = specification_json
                .as_deref()
                .map(serde_json::from_str::<ExperimentDesignSpecificationDto>)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        8,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            let preregistration_id: Option<String> = row.get(14)?;
            let preregistration = preregistration_id
                .map(|id| {
                    Ok::<ExperimentPreregistrationDto, rusqlite::Error>(
                        ExperimentPreregistrationDto {
                            id,
                            design_version_id: row.get(0)?,
                            specification_sha256: row.get(15)?,
                            reviewer_id: row.get(16)?,
                            rationale: row.get(17)?,
                            preregistered_at_us: row.get(18)?,
                        },
                    )
                })
                .transpose()?;
            Ok(ExperimentDesignVersionDto {
                id: row.get(0)?,
                design_id: row.get(1)?,
                project_id: row.get(2)?,
                title: row.get(3)?,
                version_no: row.get(4)?,
                description: row.get(5)?,
                protocol: row.get(6)?,
                lifecycle_status,
                specification,
                specification_sha256: row.get(9)?,
                parent_version_id: row.get(10)?,
                change_rationale: row.get(11)?,
                created_by: row.get(12)?,
                created_at_us: row.get(13)?,
                preregistration,
            })
        },
    )?;
    Ok(row)
}

fn parse_design_status(value: &str) -> rusqlite::Result<crate::research::ResearchDesignStatus> {
    match value {
        "draft" => Ok(crate::research::ResearchDesignStatus::Draft),
        "review" => Ok(crate::research::ResearchDesignStatus::Review),
        "preregistered" => Ok(crate::research::ResearchDesignStatus::Preregistered),
        "withdrawn" => Ok(crate::research::ResearchDesignStatus::Withdrawn),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn validate_design_targets(
    tx: &Transaction<'_>,
    project_id: &str,
    specification: &ExperimentDesignSpecificationDto,
) -> Result<()> {
    if let Some(question_id) = specification.target_research_question_id.as_deref() {
        let question_project: Option<String> = tx.query_row(
            "SELECT project_id FROM research_questions WHERE id = ?1",
            [question_id],
            |row| row.get(0),
        )?;
        if question_project.as_deref() != Some(project_id) {
            return Err(invalid(
                "The target research question belongs to another or unassigned project.",
            ));
        }
    }
    if let Some(hypothesis_version_id) = specification.target_hypothesis_version_id.as_deref() {
        let hypothesis_project: String = tx.query_row(
            "SELECT h.project_id
             FROM hypothesis_definition_versions v
             JOIN research_hypotheses h ON h.id = v.hypothesis_id
             WHERE v.id = ?1",
            [hypothesis_version_id],
            |row| row.get(0),
        )?;
        if hypothesis_project != project_id {
            return Err(invalid(
                "The target hypothesis version belongs to another project.",
            ));
        }
    }
    Ok(())
}

fn insert_design_dataset_pins(
    tx: &Transaction<'_>,
    version_id: &str,
    project_id: &str,
    pins: &[DatasetRolePinDto],
) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for pin in pins {
        if !seen.insert((pin.dataset_version_id.as_str(), pin.role.as_str())) {
            return Err(invalid(
                "A dataset version and role can only be pinned once.",
            ));
        }
        let (dataset_project, status): (String, String) = tx.query_row(
            "SELECT d.project_id, v.lifecycle_status
             FROM experiment_dataset_versions v
             JOIN experiment_datasets d ON d.id = v.dataset_id
             WHERE v.id = ?1",
            [&pin.dataset_version_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if dataset_project != project_id {
            return Err(invalid(
                "Design dataset pins must belong to the same research project.",
            ));
        }
        if status != "sealed" {
            return Err(invalid("A design can only pin a sealed dataset version."));
        }
        tx.execute(
            "INSERT INTO experiment_design_dataset_pins
                (design_version_id, dataset_version_id, role, rationale)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                version_id,
                pin.dataset_version_id,
                pin.role.as_str(),
                pin.rationale
            ],
        )?;
    }
    Ok(())
}

pub fn create_design_version(
    conn: &mut Connection,
    req: &CreateExperimentDesignVersionReq,
) -> Result<ExperimentDesignVersionDto> {
    if req.title.trim().is_empty()
        || req.description.trim().is_empty()
        || req.protocol.trim().is_empty()
    {
        return Err(invalid(
            "Design title, description, and intended protocol are required.",
        ));
    }
    let specification_json = json_string(&req.specification)?;
    let specification_sha256 = sha256_text(&specification_json);
    let now = now_ts_us().map_err(invalid)?;
    let tx = conn.transaction()?;

    let (design_id, version_no, parent_version_id) =
        if let Some(parent_id) = req.parent_version_id.as_deref() {
            if req.change_rationale.trim().is_empty() {
                return Err(invalid("A design revision requires a change rationale."));
            }
            let (design_id, project_id, parent_version_no, latest_version_id): (
                String,
                String,
                i64,
                String,
            ) = tx.query_row(
                "SELECT d.id, d.project_id, parent.version_no,
                        (SELECT id FROM research_experiment_design_versions
                         WHERE design_id = d.id ORDER BY version_no DESC LIMIT 1)
                 FROM research_experiment_design_versions parent
                 JOIN research_experiment_designs d ON d.id = parent.design_id
                 WHERE parent.id = ?1",
                [parent_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
            if project_id != req.project_id {
                return Err(invalid(
                    "A design revision must remain in its original project.",
                ));
            }
            if latest_version_id != parent_id {
                return Err(invalid(
                    "A design revision must be based on the latest version.",
                ));
            }
            if req.design_id.as_deref().is_some_and(|id| id != design_id) {
                return Err(invalid(
                    "The supplied design identity does not match the parent version.",
                ));
            }
            (design_id, parent_version_no + 1, Some(parent_id.to_owned()))
        } else {
            if req.design_id.is_some() {
                return Err(invalid(
                    "A new design cannot provide an existing design identity.",
                ));
            }
            let design_id = new_id();
            tx.execute(
                "INSERT INTO research_experiment_designs
                    (id, project_id, title, created_by, created_at_us)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    design_id,
                    req.project_id,
                    req.title.trim(),
                    req.created_by,
                    now
                ],
            )?;
            (design_id, 1, None)
        };

    validate_design_targets(&tx, &req.project_id, &req.specification)?;
    let version_id = new_id();
    tx.execute(
        "INSERT INTO research_experiment_design_versions
            (id, design_id, version_no, description, protocol, lifecycle_status, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, 'draft', ?6, ?7)",
        params![
            version_id,
            design_id,
            version_no,
            req.description,
            req.protocol,
            req.created_by,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO experiment_design_version_titles (design_version_id, title)
         VALUES (?1, ?2)",
        params![version_id, req.title.trim()],
    )?;
    tx.execute(
        "INSERT INTO experiment_design_version_specs
            (design_version_id, format_version, specification_json,
             specification_sha256, created_at_us)
         VALUES (?1, 1, ?2, ?3, ?4)",
        params![version_id, specification_json, specification_sha256, now],
    )?;
    if let Some(parent_id) = parent_version_id.as_deref() {
        tx.execute(
            "INSERT INTO experiment_design_version_lineage
                (design_version_id, design_id, parent_version_id, change_rationale,
                 created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                version_id,
                design_id,
                parent_id,
                req.change_rationale.trim(),
                req.created_by,
                now
            ],
        )?;
    }
    insert_design_dataset_pins(
        &tx,
        &version_id,
        &req.project_id,
        &req.specification.datasets,
    )?;
    if let Some(hypothesis_version_id) = req.specification.target_hypothesis_version_id.as_deref() {
        tx.execute(
            "INSERT INTO hypothesis_design_links
                (hypothesis_version_id, design_version_id, relationship_kind,
                 rationale, created_by, created_at_us)
             VALUES (?1, ?2, 'tests', ?3, ?4, ?5)",
            params![
                hypothesis_version_id,
                version_id,
                if req.change_rationale.trim().is_empty() {
                    "Design targets the specified hypothesis version."
                } else {
                    req.change_rationale.trim()
                },
                req.created_by,
                now
            ],
        )?;
    }
    tx.execute(
        "INSERT INTO research_provenance_events
            (id, target_kind, target_id, event_kind, actor_id, rationale, occurred_at_us)
         VALUES (?1, 'experiment_design', ?2, 'typed_version_created', ?3, ?4, ?5)",
        params![
            new_id(),
            design_id,
            req.created_by,
            req.change_rationale.trim(),
            now
        ],
    )?;
    tx.commit()?;
    query_design_version(conn, &version_id)
}

pub fn preregister_design(
    conn: &mut Connection,
    req: &PreregisterExperimentDesignReq,
) -> Result<ExperimentDesignVersionDto> {
    if req.rationale.trim().is_empty() {
        return Err(invalid("Preregistration requires a review rationale."));
    }
    let version = query_design_version(conn, &req.design_version_id)?;
    if version.lifecycle_status != crate::research::ResearchDesignStatus::Review {
        return Err(invalid(
            "Only a reviewed draft design can be preregistered.",
        ));
    }
    let specification = version
        .specification
        .as_ref()
        .ok_or_else(|| invalid("Legacy design versions must be revised with a typed specification before preregistration."))?;
    specification
        .validate_for_preregistration()
        .map_err(invalid)?;
    let now = now_ts_us().map_err(invalid)?;
    let tx = conn.transaction()?;
    validate_design_targets(&tx, &version.project_id, specification)?;
    for pin in &specification.datasets {
        let (project_id, status): (String, String) = tx.query_row(
            "SELECT d.project_id, v.lifecycle_status
             FROM experiment_dataset_versions v
             JOIN experiment_datasets d ON d.id = v.dataset_id
             WHERE v.id = ?1",
            [&pin.dataset_version_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if project_id != version.project_id || status != "sealed" {
            return Err(invalid(
                "Preregistration requires every exact dataset version to be sealed and project-compatible.",
            ));
        }
    }
    let preregistration_id = new_id();
    let checksum = version
        .specification_sha256
        .as_deref()
        .ok_or_else(|| invalid("The typed design specification checksum is missing."))?;
    tx.execute(
        "INSERT INTO experiment_preregistrations
            (id, design_version_id, specification_sha256, reviewer_id, rationale, preregistered_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            preregistration_id,
            req.design_version_id,
            checksum,
            req.reviewer_id,
            req.rationale.trim(),
            now
        ],
    )?;
    tx.execute(
        "UPDATE research_experiment_design_versions
         SET lifecycle_status = 'preregistered', preregistered_at_us = ?1
         WHERE id = ?2 AND lifecycle_status = 'review'",
        params![now, req.design_version_id],
    )?;
    if tx.changes() != 1 {
        return Err(invalid(
            "Design preregistration state changed; reload and retry.",
        ));
    }
    tx.execute(
        "INSERT INTO research_provenance_events
            (id, target_kind, target_id, event_kind, actor_id, rationale, occurred_at_us)
         VALUES (?1, 'experiment_design', ?2, 'preregistered', ?3, ?4, ?5)",
        params![
            new_id(),
            version.design_id,
            req.reviewer_id,
            req.rationale.trim(),
            now
        ],
    )?;
    tx.commit()?;
    query_design_version(conn, &req.design_version_id)
}

pub fn create_dataset(
    conn: &mut Connection,
    req: &CreateDatasetReq,
) -> Result<ExperimentDatasetDto> {
    let title = req.title.trim();
    if title.is_empty() || title.len() > 200 {
        return Err(invalid(
            "Dataset title must contain between 1 and 200 characters.",
        ));
    }
    let now = now_ts_us().map_err(invalid)?;
    let id = new_id();
    conn.execute(
        "INSERT INTO experiment_datasets
            (id, project_id, title, description, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            req.project_id,
            title,
            req.description,
            req.created_by,
            now
        ],
    )?;
    query_dataset(conn, &id)
}

pub fn create_dataset_version(
    conn: &mut Connection,
    req: &CreateDatasetVersionReq,
) -> Result<ExperimentDatasetVersionDto> {
    validate_sample_schema(&req.schema).map_err(invalid)?;
    if req.expected_sample_count < 0 {
        return Err(invalid("Expected dataset sample count cannot be negative."));
    }
    let schema_json = json_string(&req.schema)?;
    let now = now_ts_us().map_err(invalid)?;
    let tx = conn.transaction()?;
    let version_no: i64 = tx.query_row(
        "SELECT COALESCE(MAX(version_no), 0) + 1
         FROM experiment_dataset_versions WHERE dataset_id = ?1",
        [&req.dataset_id],
        |row| row.get(0),
    )?;
    let id = new_id();
    tx.execute(
        "INSERT INTO experiment_dataset_versions
            (id, dataset_id, version_no, schema_id, schema_version, sample_schema_json,
             expected_sample_count, lifecycle_status, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'staged', ?8, ?9)",
        params![
            id,
            req.dataset_id,
            version_no,
            req.schema.schema_id,
            req.schema.schema_version,
            schema_json,
            req.expected_sample_count,
            req.created_by,
            now
        ],
    )?;
    tx.commit()?;
    query_dataset_version(conn, &id)
}

fn query_dataset_version(
    conn: &Connection,
    version_id: &str,
) -> Result<ExperimentDatasetVersionDto> {
    let mut version = conn.query_row(
        "SELECT id, dataset_id, version_no, sample_schema_json, expected_sample_count,
                lifecycle_status, content_sha256, created_by, created_at_us, sealed_at_us,
                (SELECT COUNT(*) FROM experiment_dataset_samples s
                 WHERE s.dataset_version_id = v.id)
         FROM experiment_dataset_versions v WHERE id = ?1",
        [version_id],
        |row| {
            let schema_json: String = row.get(3)?;
            Ok(ExperimentDatasetVersionDto {
                id: row.get(0)?,
                dataset_id: row.get(1)?,
                version_no: row.get(2)?,
                schema: serde_json::from_str(&schema_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?,
                expected_sample_count: row.get(4)?,
                lifecycle_status: row.get(5)?,
                content_sha256: row.get(6)?,
                created_by: row.get(7)?,
                created_at_us: row.get(8)?,
                sealed_at_us: row.get(9)?,
                actual_sample_count: row.get(10)?,
                assets: Vec::new(),
            })
        },
    )?;
    version.assets = query_dataset_assets(conn, version_id)?;
    Ok(version)
}

pub fn get_dataset_version(
    conn: &Connection,
    version_id: &str,
) -> Result<ExperimentDatasetVersionDto> {
    query_dataset_version(conn, version_id)
}

pub fn run_dataset_version_ids(conn: &Connection, run_id: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT dataset_version_id FROM experiment_run_dataset_pins
         WHERE run_id = ?1 ORDER BY usage_kind, dataset_version_id",
    )?;
    let rows = statement
        .query_map([run_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn query_dataset_assets(
    conn: &Connection,
    version_id: &str,
) -> Result<Vec<ExperimentDatasetAssetDto>> {
    let mut statement = conn.prepare(
        "SELECT id, dataset_version_id, storage_key, original_name, media_type,
                byte_length, content_sha256
         FROM experiment_dataset_assets WHERE dataset_version_id = ?1
         ORDER BY id",
    )?;
    let assets = statement
        .query_map([version_id], |row| {
            Ok(ExperimentDatasetAssetDto {
                id: row.get(0)?,
                dataset_version_id: row.get(1)?,
                storage_key: row.get(2)?,
                original_name: row.get(3)?,
                media_type: row.get(4)?,
                byte_length: row.get(5)?,
                content_sha256: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(assets)
}

fn query_dataset(conn: &Connection, dataset_id: &str) -> Result<ExperimentDatasetDto> {
    let mut dataset = conn.query_row(
        "SELECT id, project_id, title, description, created_by, created_at_us
         FROM experiment_datasets WHERE id = ?1",
        [dataset_id],
        |row| {
            Ok(ExperimentDatasetDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                created_by: row.get(4)?,
                created_at_us: row.get(5)?,
                versions: Vec::new(),
            })
        },
    )?;
    let version_ids = {
        let mut statement = conn.prepare(
            "SELECT id FROM experiment_dataset_versions
             WHERE dataset_id = ?1 ORDER BY version_no",
        )?;
        let rows = statement
            .query_map([dataset_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    dataset.versions = version_ids
        .iter()
        .map(|version_id| query_dataset_version(conn, version_id))
        .collect::<Result<Vec<_>>>()?;
    Ok(dataset)
}

pub fn add_dataset_samples(conn: &mut Connection, req: &AddDatasetSamplesReq) -> Result<i64> {
    let tx = conn.transaction()?;
    let (schema_json, expected_count, status): (String, i64, String) = tx.query_row(
        "SELECT sample_schema_json, expected_sample_count, lifecycle_status
         FROM experiment_dataset_versions WHERE id = ?1",
        [&req.dataset_version_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    if status != "staged" {
        return Err(invalid(
            "Samples can only be added to staged dataset versions.",
        ));
    }
    let schema: DatasetSampleSchemaDto = serde_json::from_str(&schema_json)?;
    let current_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM experiment_dataset_samples WHERE dataset_version_id = ?1",
        [&req.dataset_version_id],
        |row| row.get(0),
    )?;
    if current_count
        + i64::try_from(req.samples.len()).map_err(|error| invalid(error.to_string()))?
        > expected_count
    {
        return Err(invalid("Dataset samples exceed the declared sample count."));
    }
    let mut seen = std::collections::HashSet::new();
    for sample in &req.samples {
        validate_sample(&schema, sample).map_err(invalid)?;
        let base = sample.data_sample();
        if base.dataset_version_id != req.dataset_version_id {
            return Err(invalid(
                "Every sample must reference the exact staged dataset version.",
            ));
        }
        if !seen.insert(base.sample_id.as_str()) {
            return Err(invalid(
                "Sample IDs must be unique within the dataset version.",
            ));
        }
        tx.execute(
            "INSERT INTO experiment_dataset_samples
                (id, dataset_version_id, sample_id, sample_kind, occurred_at_us, record_json, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                new_id(),
                req.dataset_version_id,
                base.sample_id,
                sample.kind(),
                base.occurred_at_us,
                json_string(sample)?,
                now_ts_us().map_err(invalid)?
            ],
        )?;
    }
    let new_count = current_count + req.samples.len() as i64;
    tx.commit()?;
    Ok(new_count)
}

pub fn add_dataset_split_group(
    conn: &mut Connection,
    req: &AddDatasetSplitGroupReq,
) -> Result<ExperimentDatasetSplitGroupDto> {
    if req.split_group_key.trim().is_empty() || req.assignments.is_empty() {
        return Err(invalid(
            "A split group requires a name and at least one sample assignment.",
        ));
    }
    let mut seen = std::collections::HashSet::new();
    if req.require_disjoint
        && req
            .assignments
            .iter()
            .any(|assignment| !seen.insert(assignment.sample_id.as_str()))
    {
        return Err(invalid(
            "A sample may only belong to one split in a given split group.",
        ));
    }
    let tx = conn.transaction()?;
    let status: String = tx.query_row(
        "SELECT lifecycle_status FROM experiment_dataset_versions WHERE id = ?1",
        [&req.dataset_version_id],
        |row| row.get(0),
    )?;
    if status != "staged" {
        return Err(invalid(
            "Split groups can only be added to staged dataset versions.",
        ));
    }
    let group_id = new_id();
    let now = now_ts_us().map_err(invalid)?;
    tx.execute(
        "INSERT INTO experiment_dataset_split_groups
            (id, dataset_version_id, split_group_key, require_disjoint, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            group_id,
            req.dataset_version_id,
            req.split_group_key.trim(),
            req.require_disjoint,
            now
        ],
    )?;
    for assignment in &req.assignments {
        if assignment.split_key.trim().is_empty() {
            return Err(invalid("Split keys cannot be blank."));
        }
        tx.execute(
            "INSERT INTO experiment_sample_split_assignments
                (id, split_group_id, dataset_version_id, sample_id, role, split_key, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                new_id(),
                group_id,
                req.dataset_version_id,
                assignment.sample_id,
                assignment.role.as_str(),
                assignment.split_key.trim(),
                now
            ],
        )?;
    }
    tx.commit()?;
    query_split_group(conn, &group_id)
}

fn query_split_group(conn: &Connection, group_id: &str) -> Result<ExperimentDatasetSplitGroupDto> {
    let (id, dataset_version_id, split_group_key, require_disjoint): (
        String,
        String,
        String,
        bool,
    ) = conn.query_row(
        "SELECT id, dataset_version_id, split_group_key, require_disjoint
         FROM experiment_dataset_split_groups WHERE id = ?1",
        [group_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let mut statement = conn.prepare(
        "SELECT id, sample_id, role, split_key
         FROM experiment_sample_split_assignments
         WHERE split_group_id = ?1 ORDER BY role, split_key, sample_id",
    )?;
    let assignments = statement
        .query_map([group_id], |row| {
            let role = row.get::<_, String>(2)?;
            Ok(ExperimentSampleSplitAssignmentDto {
                id: row.get(0)?,
                sample_id: row.get(1)?,
                role: parse_dataset_role(&role)?,
                split_key: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ExperimentDatasetSplitGroupDto {
        id,
        dataset_version_id,
        split_group_key,
        require_disjoint,
        assignments,
    })
}

fn parse_dataset_role(value: &str) -> rusqlite::Result<DatasetRole> {
    match value {
        "training" => Ok(DatasetRole::Training),
        "validation" => Ok(DatasetRole::Validation),
        "test" => Ok(DatasetRole::Test),
        "exploratory" => Ok(DatasetRole::Exploratory),
        "prospective" => Ok(DatasetRole::Prospective),
        "forward" => Ok(DatasetRole::Forward),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

pub struct PreparedDatasetAsset {
    id: String,
    dataset_version_id: String,
    storage_key: String,
    staged_path: PathBuf,
    final_path: PathBuf,
    original_name: String,
    media_type: String,
    byte_length: i64,
    content_sha256: String,
}

impl PreparedDatasetAsset {
    pub fn discard(self) {
        let _ = fs::remove_file(self.staged_path);
        let _ = fs::remove_file(self.final_path);
    }
}

fn hash_file(path: &Path) -> Result<(i64, String)> {
    let mut source = File::open(path)?;
    let metadata = source.metadata()?;
    if !metadata.is_file() {
        return Err(invalid("Dataset assets must be regular files."));
    }
    let byte_length = i64::try_from(metadata.len()).map_err(|error| invalid(error.to_string()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok((byte_length, format!("{:x}", hasher.finalize())))
}

pub fn prepare_dataset_asset(
    req: &RegisterDatasetAssetReq,
    assets_root: &Path,
) -> Result<PreparedDatasetAsset> {
    if req.original_name.trim().is_empty() || req.media_type.trim().is_empty() {
        return Err(invalid("Dataset asset name and media type are required."));
    }
    let source_path = Path::new(&req.source_path)
        .canonicalize()
        .map_err(|error| invalid(format!("Cannot access dataset asset: {error}")))?;
    let metadata = fs::metadata(&source_path)?;
    if !metadata.is_file() {
        return Err(invalid("Dataset asset must be a regular file."));
    }
    let asset_dir = assets_root.join("dataset-assets");
    fs::create_dir_all(&asset_dir)?;
    let id = new_id();
    let staged_path = asset_dir.join(format!(".stage-{id}"));
    let final_path = asset_dir.join(&id);
    let mut input = File::open(source_path)?;
    let mut output = File::create(&staged_path)?;
    let mut hasher = Sha256::new();
    let mut byte_length = 0_i64;
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        output.write_all(&buffer[..count])?;
        hasher.update(&buffer[..count]);
        byte_length = byte_length
            .checked_add(i64::try_from(count).map_err(|error| invalid(error.to_string()))?)
            .ok_or_else(|| invalid("Dataset asset is too large."))?;
    }
    output.sync_all()?;
    drop(output);
    let content_sha256 = format!("{:x}", hasher.finalize());
    if req
        .expected_sha256
        .as_deref()
        .is_some_and(|expected| expected != content_sha256)
    {
        fs::remove_file(&staged_path)?;
        return Err(invalid(
            "Dataset asset checksum does not match the supplied checksum.",
        ));
    }
    if let Err(error) = fs::rename(&staged_path, &final_path) {
        let _ = fs::remove_file(&staged_path);
        return Err(error.into());
    }
    Ok(PreparedDatasetAsset {
        id,
        dataset_version_id: req.dataset_version_id.clone(),
        storage_key: format!(
            "dataset-assets/{}",
            final_path.file_name().unwrap_or_default().to_string_lossy()
        ),
        staged_path,
        final_path,
        original_name: req.original_name.trim().to_owned(),
        media_type: req.media_type.trim().to_owned(),
        byte_length,
        content_sha256,
    })
}

pub fn attach_prepared_dataset_asset(
    conn: &Connection,
    prepared: &PreparedDatasetAsset,
) -> Result<ExperimentDatasetAssetDto> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO experiment_dataset_assets
            (id, dataset_version_id, storage_key, original_name, media_type,
             byte_length, content_sha256, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            prepared.id,
            prepared.dataset_version_id,
            prepared.storage_key,
            prepared.original_name,
            prepared.media_type,
            prepared.byte_length,
            prepared.content_sha256,
            now_ts_us().map_err(invalid)?
        ],
    )?;
    tx.commit()?;
    Ok(ExperimentDatasetAssetDto {
        id: prepared.id.clone(),
        dataset_version_id: prepared.dataset_version_id.clone(),
        storage_key: prepared.storage_key.clone(),
        original_name: prepared.original_name.clone(),
        media_type: prepared.media_type.clone(),
        byte_length: prepared.byte_length,
        content_sha256: prepared.content_sha256.clone(),
    })
}

#[derive(Debug, Clone)]
pub struct DatasetSealMaterial {
    pub version_id: String,
    pub staged_revision: i64,
    pub lifecycle_status: String,
    pub schema_json: String,
    pub expected_sample_count: i64,
    pub content_sha256: Option<String>,
    pub samples: Vec<String>,
    pub assets: Vec<ExperimentDatasetAssetDto>,
}

pub fn dataset_seal_material(conn: &Connection, version_id: &str) -> Result<DatasetSealMaterial> {
    let (staged_revision, schema_json, expected_sample_count, lifecycle_status, content_sha256): (
        i64,
        String,
        i64,
        String,
        Option<String>,
    ) = conn.query_row(
        "SELECT staged_revision, sample_schema_json, expected_sample_count, lifecycle_status,
                content_sha256
         FROM experiment_dataset_versions WHERE id = ?1",
        [version_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )?;
    let mut statement = conn.prepare(
        "SELECT record_json FROM experiment_dataset_samples
         WHERE dataset_version_id = ?1 ORDER BY sample_id",
    )?;
    let samples = statement
        .query_map([version_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if samples.len() as i64 != expected_sample_count {
        return Err(invalid(format!(
            "Declared sample count {expected_sample_count} does not match {} stored samples.",
            samples.len()
        )));
    }
    let schema: DatasetSampleSchemaDto = serde_json::from_str(&schema_json)?;
    for record in &samples {
        let sample: DatasetSampleDto = serde_json::from_str(record)?;
        validate_sample(&schema, &sample).map_err(invalid)?;
    }
    Ok(DatasetSealMaterial {
        version_id: version_id.to_owned(),
        staged_revision,
        lifecycle_status,
        schema_json,
        expected_sample_count,
        content_sha256,
        samples,
        assets: query_dataset_assets(conn, version_id)?,
    })
}

pub fn verify_dataset_assets(
    assets_root: &Path,
    assets: &[ExperimentDatasetAssetDto],
) -> Result<()> {
    for asset in assets {
        let relative = Path::new(&asset.storage_key);
        let mut components = relative.components();
        let first = components.next();
        let second = components.next();
        if !matches!(
            (first, second, components.next()),
            (
                Some(std::path::Component::Normal(directory)),
                Some(std::path::Component::Normal(_)),
                None
            ) if directory == "dataset-assets"
        ) {
            return Err(invalid("Dataset asset storage key is invalid."));
        }
        let asset_root = fs::canonicalize(assets_root.join("dataset-assets"))?;
        let path = fs::canonicalize(assets_root.join(relative))?;
        if !path.starts_with(&asset_root) || !path.is_file() {
            return Err(invalid(
                "Dataset asset is not contained in its managed asset directory.",
            ));
        }
        let (byte_length, content_sha256) = hash_file(&path)?;
        if byte_length != asset.byte_length || content_sha256 != asset.content_sha256 {
            return Err(invalid(format!(
                "Dataset asset '{}' failed checksum verification.",
                asset.original_name
            )));
        }
    }
    Ok(())
}

pub fn compute_dataset_checksum(material: &DatasetSealMaterial) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"vora-dataset-version-v1\0");
    hasher.update(material.version_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(material.schema_json.as_bytes());
    hasher.update(b"\0");
    hasher.update(material.expected_sample_count.to_be_bytes());
    for sample in &material.samples {
        hasher.update(b"\0sample\0");
        hasher.update(sample.as_bytes());
    }
    let mut assets = material.assets.clone();
    assets.sort_by(|left, right| left.storage_key.cmp(&right.storage_key));
    for asset in assets {
        hasher.update(b"\0asset\0");
        hasher.update(asset.storage_key.as_bytes());
        hasher.update(asset.byte_length.to_be_bytes());
        hasher.update(asset.content_sha256.as_bytes());
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn seal_dataset_version(
    conn: &mut Connection,
    version_id: &str,
    expected_staged_revision: i64,
    content_sha256: &str,
) -> Result<ExperimentDatasetVersionDto> {
    if content_sha256.len() != 64
        || content_sha256
            .bytes()
            .any(|byte| !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase())
    {
        return Err(invalid("Invalid dataset version checksum."));
    }
    let tx = conn.transaction()?;
    let (status, revision, expected_count): (String, i64, i64) = tx.query_row(
        "SELECT lifecycle_status, staged_revision, expected_sample_count
         FROM experiment_dataset_versions WHERE id = ?1",
        [version_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    if status != "staged" || revision != expected_staged_revision {
        return Err(invalid(
            "Dataset version changed while it was being verified; verify it again.",
        ));
    }
    let actual_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM experiment_dataset_samples WHERE dataset_version_id = ?1",
        [version_id],
        |row| row.get(0),
    )?;
    if actual_count != expected_count {
        return Err(invalid("Dataset sample count changed during verification."));
    }
    let now = now_ts_us().map_err(invalid)?;
    tx.execute(
        "UPDATE experiment_dataset_versions
         SET lifecycle_status = 'sealed', content_sha256 = ?1, sealed_at_us = ?2
         WHERE id = ?3 AND lifecycle_status = 'staged' AND staged_revision = ?4",
        params![content_sha256, now, version_id, expected_staged_revision],
    )?;
    if tx.changes() != 1 {
        return Err(invalid(
            "Dataset version sealing raced with another update.",
        ));
    }
    tx.commit()?;
    query_dataset_version(conn, version_id)
}

pub fn create_dataset_transformation(
    conn: &mut Connection,
    req: &CreateDatasetTransformationReq,
) -> Result<String> {
    if req.method.trim().is_empty() || req.inputs.is_empty() {
        return Err(invalid(
            "A dataset transformation requires a method and at least one exact input version.",
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for input in &req.inputs {
        if !seen.insert(input.dataset_version_id.as_str()) {
            return Err(invalid(
                "A transformation input version can only be declared once.",
            ));
        }
        let status: String = conn.query_row(
            "SELECT lifecycle_status FROM experiment_dataset_versions WHERE id = ?1",
            [&input.dataset_version_id],
            |row| row.get(0),
        )?;
        if status != "sealed" {
            return Err(invalid(
                "Transformations require sealed input dataset versions.",
            ));
        }
        if input.dataset_version_id == req.output_dataset_version_id {
            return Err(invalid(
                "A dataset transformation cannot consume its own output version.",
            ));
        }
    }
    let tx = conn.transaction()?;
    let output_status: String = tx.query_row(
        "SELECT lifecycle_status FROM experiment_dataset_versions WHERE id = ?1",
        [&req.output_dataset_version_id],
        |row| row.get(0),
    )?;
    if output_status != "staged" {
        return Err(invalid(
            "A transformation output must be a staged dataset version.",
        ));
    }
    let id = new_id();
    tx.execute(
        "INSERT INTO experiment_dataset_transformations
            (id, output_dataset_version_id, method, parameters_json, provenance_json,
             created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            req.output_dataset_version_id,
            req.method.trim(),
            json_string(&req.parameters)?,
            json_string(&req.provenance)?,
            req.created_by,
            now_ts_us().map_err(invalid)?
        ],
    )?;
    for input in &req.inputs {
        tx.execute(
            "INSERT INTO experiment_dataset_transformation_inputs
                (transformation_id, input_dataset_version_id, input_role)
             VALUES (?1, ?2, ?3)",
            params![id, input.dataset_version_id, input.role.as_str()],
        )?;
    }
    tx.execute(
        "UPDATE experiment_dataset_versions SET staged_revision = staged_revision + 1
         WHERE id = ?1 AND lifecycle_status = 'staged'",
        [&req.output_dataset_version_id],
    )?;
    tx.commit()?;
    Ok(id)
}

fn insert_run_event(
    tx: &Transaction<'_>,
    run_id: &str,
    event_kind: &str,
    from_status: Option<&str>,
    to_status: Option<&str>,
    actor_id: Option<&str>,
    details: &str,
    now: i64,
) -> Result<()> {
    tx.execute(
        "INSERT INTO experiment_run_events
            (id, run_id, event_kind, from_status, to_status, actor_id, details, occurred_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            new_id(),
            run_id,
            event_kind,
            from_status,
            to_status,
            actor_id,
            details,
            now
        ],
    )?;
    Ok(())
}

pub fn create_run(conn: &mut Connection, req: &CreateExperimentRunReq) -> Result<ExperimentRunDto> {
    let design = query_design_version(conn, &req.design_version_id)?;
    if design.lifecycle_status != crate::research::ResearchDesignStatus::Preregistered
        || design.preregistration.is_none()
    {
        return Err(invalid(
            "Runs can only use an actively preregistered design version.",
        ));
    }
    let specification = design
        .specification
        .as_ref()
        .ok_or_else(|| invalid("The design has no typed specification."))?;
    specification
        .validate_for_preregistration()
        .map_err(invalid)?;
    let expected_pins: std::collections::BTreeSet<_> = specification
        .datasets
        .iter()
        .map(|pin| (pin.dataset_version_id.as_str(), pin.role.as_str()))
        .collect();
    let supplied_pins: std::collections::BTreeSet<_> = req
        .dataset_pins
        .iter()
        .map(|pin| (pin.dataset_version_id.as_str(), pin.role.as_str()))
        .collect();
    if expected_pins.len() != specification.datasets.len()
        || supplied_pins.len() != req.dataset_pins.len()
        || expected_pins != supplied_pins
    {
        return Err(invalid(
            "A run must pin the exact dataset-version/role set declared in its design version.",
        ));
    }
    let now = now_ts_us().map_err(invalid)?;
    let run_id = new_id();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO experiment_runs
            (id, project_id, design_version_id, lifecycle_status, created_by, created_at_us)
         VALUES (?1, ?2, ?3, 'created', ?4, ?5)",
        params![
            run_id,
            design.project_id,
            req.design_version_id,
            req.created_by,
            now
        ],
    )?;
    for pin in &req.dataset_pins {
        let (dataset_project, status): (String, String) = tx.query_row(
            "SELECT d.project_id, v.lifecycle_status
             FROM experiment_dataset_versions v
             JOIN experiment_datasets d ON d.id = v.dataset_id
             WHERE v.id = ?1",
            [&pin.dataset_version_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if dataset_project != design.project_id || status != "sealed" {
            return Err(invalid(
                "Run dataset pins must be sealed versions owned by the design's project.",
            ));
        }
        tx.execute(
            "INSERT INTO experiment_run_dataset_pins
                (run_id, dataset_version_id, role, usage_kind)
             VALUES (?1, ?2, ?3, 'input')",
            params![run_id, pin.dataset_version_id, pin.role.as_str()],
        )?;
    }
    insert_run_event(
        &tx,
        &run_id,
        "created",
        None,
        Some("created"),
        req.created_by.as_deref(),
        "",
        now,
    )?;
    tx.commit()?;
    query_run(conn, &run_id)
}

pub fn transition_run(
    conn: &mut Connection,
    req: &TransitionExperimentRunReq,
) -> Result<ExperimentRunDto> {
    let now = now_ts_us().map_err(invalid)?;
    let tx = conn.transaction()?;
    let (current, project_id): (String, String) = tx.query_row(
        "SELECT lifecycle_status, project_id FROM experiment_runs WHERE id = ?1",
        [&req.run_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let next = req.next_status.as_str();
    let allowed = matches!(
        (current.as_str(), next),
        ("created", "ready" | "cancelled")
            | ("ready", "running" | "cancelled")
            | (
                "running",
                "completed" | "failed" | "cancelled" | "inconclusive"
            )
    );
    if !allowed {
        return Err(invalid(format!(
            "Invalid experiment run transition from '{current}' to '{next}'."
        )));
    }
    if next == "completed" && req.completion_summary.as_deref().is_none_or(str::is_empty) {
        return Err(invalid("Completed runs require a completion summary."));
    }
    if next == "failed" && req.failure_details.as_deref().is_none_or(str::is_empty) {
        return Err(invalid("Failed runs require failure details."));
    }
    if !req.output_dataset_pins.is_empty() && next != "completed" {
        return Err(invalid(
            "Output dataset versions may only be pinned when completing a run.",
        ));
    }
    if next == "completed" {
        for pin in &req.output_dataset_pins {
            let allowed_output_role = matches!(
                pin.role,
                DatasetRole::Exploratory | DatasetRole::Prospective | DatasetRole::Forward
            );
            if !allowed_output_role {
                return Err(invalid(
                    "Run outputs require an explicit exploratory, prospective, or forward role.",
                ));
            }
            let (dataset_project, status): (String, String) = tx.query_row(
                "SELECT d.project_id, v.lifecycle_status
                 FROM experiment_dataset_versions v
                 JOIN experiment_datasets d ON d.id = v.dataset_id
                 WHERE v.id = ?1",
                [&pin.dataset_version_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            if dataset_project != project_id || status != "sealed" {
                return Err(invalid(
                    "Run outputs must reference sealed dataset versions in the same project.",
                ));
            }
            tx.execute(
                "INSERT INTO experiment_run_dataset_pins
                    (run_id, dataset_version_id, role, usage_kind)
                 VALUES (?1, ?2, ?3, 'output')",
                params![req.run_id, pin.dataset_version_id, pin.role.as_str()],
            )?;
        }
    }
    let started_at = if next == "running" {
        Some(now)
    } else {
        let existing: Option<i64> = tx.query_row(
            "SELECT started_at_us FROM experiment_runs WHERE id = ?1",
            [&req.run_id],
            |row| row.get(0),
        )?;
        existing
    };
    let ended_at = if req.next_status.is_terminal() {
        Some(now)
    } else {
        None
    };
    tx.execute(
        "UPDATE experiment_runs
         SET lifecycle_status = ?1, started_at_us = ?2, ended_at_us = ?3,
             completion_summary = ?4, failure_details = ?5
         WHERE id = ?6 AND lifecycle_status = ?7",
        params![
            next,
            started_at,
            ended_at,
            req.completion_summary,
            req.failure_details,
            req.run_id,
            current
        ],
    )?;
    if tx.changes() != 1 {
        return Err(invalid("Experiment run state changed; reload and retry."));
    }
    insert_run_event(
        &tx,
        &req.run_id,
        next,
        Some(&current),
        Some(next),
        req.actor_id.as_deref(),
        &req.details,
        now,
    )?;
    tx.commit()?;
    query_run(conn, &req.run_id)
}

pub fn add_run_deviation(
    conn: &mut Connection,
    req: &AddExperimentRunDeviationReq,
) -> Result<ExperimentRunDeviationDto> {
    if req.description.trim().is_empty() || req.rationale.trim().is_empty() {
        return Err(invalid(
            "Run deviations require a description and rationale.",
        ));
    }
    let now = now_ts_us().map_err(invalid)?;
    let tx = conn.transaction()?;
    let status: String = tx.query_row(
        "SELECT lifecycle_status FROM experiment_runs WHERE id = ?1",
        [&req.run_id],
        |row| row.get(0),
    )?;
    if status != "running" {
        return Err(invalid(
            "Deviations must be recorded while the run is executing.",
        ));
    }
    let id = new_id();
    tx.execute(
        "INSERT INTO experiment_run_deviations
            (id, run_id, description, rationale, impact_assessment, recorded_by, recorded_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            req.run_id,
            req.description.trim(),
            req.rationale.trim(),
            req.impact_assessment,
            req.recorded_by,
            now
        ],
    )?;
    tx.commit()?;
    Ok(ExperimentRunDeviationDto {
        id,
        description: req.description.trim().to_owned(),
        rationale: req.rationale.trim().to_owned(),
        impact_assessment: req.impact_assessment.clone(),
        recorded_by: req.recorded_by.clone(),
        recorded_at_us: now,
    })
}

pub fn add_run_log(
    conn: &mut Connection,
    req: &AddExperimentRunLogReq,
) -> Result<ExperimentRunLogDto> {
    if req.message.trim().is_empty() {
        return Err(invalid("Run log messages cannot be blank."));
    }
    let now = now_ts_us().map_err(invalid)?;
    let tx = conn.transaction()?;
    let status: String = tx.query_row(
        "SELECT lifecycle_status FROM experiment_runs WHERE id = ?1",
        [&req.run_id],
        |row| row.get(0),
    )?;
    if ExperimentRunStatus::parse(&status)
        .map_err(invalid)?
        .is_terminal()
    {
        return Err(invalid(
            "Run logs cannot be appended after a run is terminal.",
        ));
    }
    let id = new_id();
    tx.execute(
        "INSERT INTO experiment_run_logs
            (id, run_id, severity, message, details_json, recorded_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            req.run_id,
            req.severity.as_str(),
            req.message.trim(),
            json_string(&req.details)?,
            now
        ],
    )?;
    tx.commit()?;
    Ok(ExperimentRunLogDto {
        id,
        severity: match req.severity {
            RunLogSeverity::Debug => RunLogSeverity::Debug,
            RunLogSeverity::Info => RunLogSeverity::Info,
            RunLogSeverity::Warning => RunLogSeverity::Warning,
            RunLogSeverity::Error => RunLogSeverity::Error,
        },
        message: req.message.trim().to_owned(),
        details: req.details.clone(),
        recorded_at_us: now,
    })
}

fn query_run(conn: &Connection, run_id: &str) -> Result<ExperimentRunDto> {
    let mut run = conn.query_row(
        "SELECT id, project_id, design_version_id, lifecycle_status, created_by,
                created_at_us, started_at_us, ended_at_us, completion_summary, failure_details
         FROM experiment_runs WHERE id = ?1",
        [run_id],
        |row| {
            Ok(ExperimentRunDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                design_version_id: row.get(2)?,
                lifecycle_status: ExperimentRunStatus::parse(&row.get::<_, String>(3)?).map_err(
                    |error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::other(error)),
                        )
                    },
                )?,
                created_by: row.get(4)?,
                created_at_us: row.get(5)?,
                started_at_us: row.get(6)?,
                ended_at_us: row.get(7)?,
                completion_summary: row.get(8)?,
                failure_details: row.get(9)?,
                dataset_pins: Vec::new(),
                events: Vec::new(),
                deviations: Vec::new(),
                logs: Vec::new(),
            })
        },
    )?;
    run.dataset_pins = query_run_dataset_pins(conn, run_id)?;
    run.events = query_run_events(conn, run_id)?;
    run.deviations = query_run_deviations(conn, run_id)?;
    run.logs = query_run_logs(conn, run_id)?;
    Ok(run)
}

fn query_run_dataset_pins(
    conn: &Connection,
    run_id: &str,
) -> Result<Vec<ExperimentRunDatasetPinDto>> {
    let mut statement = conn.prepare(
        "SELECT p.dataset_version_id, d.id, d.title, v.version_no, p.role, p.usage_kind
         FROM experiment_run_dataset_pins p
         JOIN experiment_dataset_versions v ON v.id = p.dataset_version_id
         JOIN experiment_datasets d ON d.id = v.dataset_id
         WHERE p.run_id = ?1 ORDER BY p.usage_kind, p.role, d.title, v.version_no",
    )?;
    let pins = statement
        .query_map([run_id], |row| {
            Ok(ExperimentRunDatasetPinDto {
                dataset_version_id: row.get(0)?,
                dataset_id: row.get(1)?,
                dataset_title: row.get(2)?,
                version_no: row.get(3)?,
                role: parse_dataset_role(&row.get::<_, String>(4)?)?,
                usage_kind: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(pins)
}

fn query_run_events(conn: &Connection, run_id: &str) -> Result<Vec<ExperimentRunEventDto>> {
    let mut statement = conn.prepare(
        "SELECT id, event_kind, from_status, to_status, actor_id, details, occurred_at_us
         FROM experiment_run_events WHERE run_id = ?1 ORDER BY occurred_at_us, id",
    )?;
    let events = statement
        .query_map([run_id], |row| {
            let from_status: Option<String> = row.get(2)?;
            let to_status: Option<String> = row.get(3)?;
            Ok(ExperimentRunEventDto {
                id: row.get(0)?,
                event_kind: row.get(1)?,
                from_status: from_status
                    .as_deref()
                    .map(ExperimentRunStatus::parse)
                    .transpose()
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::other(error)),
                        )
                    })?,
                to_status: to_status
                    .as_deref()
                    .map(ExperimentRunStatus::parse)
                    .transpose()
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::other(error)),
                        )
                    })?,
                actor_id: row.get(4)?,
                details: row.get(5)?,
                occurred_at_us: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(events)
}

fn query_run_deviations(conn: &Connection, run_id: &str) -> Result<Vec<ExperimentRunDeviationDto>> {
    let mut statement = conn.prepare(
        "SELECT id, description, rationale, impact_assessment, recorded_by, recorded_at_us
         FROM experiment_run_deviations WHERE run_id = ?1 ORDER BY recorded_at_us, id",
    )?;
    let deviations = statement
        .query_map([run_id], |row| {
            Ok(ExperimentRunDeviationDto {
                id: row.get(0)?,
                description: row.get(1)?,
                rationale: row.get(2)?,
                impact_assessment: row.get(3)?,
                recorded_by: row.get(4)?,
                recorded_at_us: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(deviations)
}

fn query_run_logs(conn: &Connection, run_id: &str) -> Result<Vec<ExperimentRunLogDto>> {
    let mut statement = conn.prepare(
        "SELECT id, severity, message, details_json, recorded_at_us
         FROM experiment_run_logs WHERE run_id = ?1 ORDER BY recorded_at_us, id",
    )?;
    let logs = statement
        .query_map([run_id], |row| {
            let severity = match row.get::<_, String>(1)?.as_str() {
                "debug" => RunLogSeverity::Debug,
                "info" => RunLogSeverity::Info,
                "warning" => RunLogSeverity::Warning,
                "error" => RunLogSeverity::Error,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            let details_json: String = row.get(3)?;
            Ok(ExperimentRunLogDto {
                id: row.get(0)?,
                severity,
                message: row.get(2)?,
                details: serde_json::from_str(&details_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?,
                recorded_at_us: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(logs)
}

pub fn get_dataset_samples(
    conn: &Connection,
    dataset_version_id: &str,
    query: Option<&str>,
    offset: i64,
    limit: i64,
) -> Result<DatasetSamplePageDto> {
    if offset < 0 || !(1..=500).contains(&limit) {
        return Err(invalid(
            "Sample page offset must be non-negative and limit must be 1..=500.",
        ));
    }
    let total_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM experiment_dataset_samples
         WHERE dataset_version_id = ?1 AND (?2 IS NULL OR sample_id LIKE '%' || ?2 || '%')",
        params![dataset_version_id, query],
        |row| row.get(0),
    )?;
    let mut statement = conn.prepare(
        "SELECT record_json FROM experiment_dataset_samples
         WHERE dataset_version_id = ?1 AND (?2 IS NULL OR sample_id LIKE '%' || ?2 || '%')
         ORDER BY sample_id LIMIT ?3 OFFSET ?4",
    )?;
    let samples = statement
        .query_map(params![dataset_version_id, query, limit, offset], |row| {
            let sample_json: String = row.get(0)?;
            serde_json::from_str(&sample_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(DatasetSamplePageDto {
        dataset_version_id: dataset_version_id.to_owned(),
        samples,
        total_count,
        offset,
        limit,
    })
}

fn query_dataset_transformation(
    conn: &Connection,
    transformation_id: &str,
) -> Result<ExperimentDatasetTransformationDto> {
    let mut transformation = conn.query_row(
        "SELECT id, output_dataset_version_id, method, parameters_json,
                provenance_json, created_by, created_at_us
         FROM experiment_dataset_transformations WHERE id = ?1",
        [transformation_id],
        |row| {
            let parameters_json: String = row.get(3)?;
            let provenance_json: String = row.get(4)?;
            Ok(ExperimentDatasetTransformationDto {
                id: row.get(0)?,
                output_dataset_version_id: row.get(1)?,
                method: row.get(2)?,
                parameters: serde_json::from_str(&parameters_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?,
                provenance: serde_json::from_str(&provenance_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?,
                inputs: Vec::new(),
                created_by: row.get(5)?,
                created_at_us: row.get(6)?,
            })
        },
    )?;
    let mut statement = conn.prepare(
        "SELECT input_dataset_version_id, input_role
         FROM experiment_dataset_transformation_inputs
         WHERE transformation_id = ?1 ORDER BY input_dataset_version_id",
    )?;
    transformation.inputs = statement
        .query_map([transformation_id], |row| {
            Ok(DatasetTransformationInputDto {
                dataset_version_id: row.get(0)?,
                role: parse_dataset_role(&row.get::<_, String>(1)?)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(transformation)
}

pub fn workspace(conn: &Connection, project_id: &str) -> Result<ExperimentWorkspaceDto> {
    let design_ids = {
        let mut statement = conn.prepare(
            "SELECT v.id FROM research_experiment_design_versions v
             JOIN research_experiment_designs d ON d.id = v.design_id
             WHERE d.project_id = ?1 ORDER BY d.created_at_us, v.version_no",
        )?;
        let rows = statement
            .query_map([project_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    let design_versions = design_ids
        .iter()
        .map(|id| query_design_version(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let run_ids = {
        let mut statement = conn.prepare(
            "SELECT id FROM experiment_runs WHERE project_id = ?1 ORDER BY created_at_us, id",
        )?;
        let rows = statement
            .query_map([project_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    let runs = run_ids
        .iter()
        .map(|id| query_run(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let dataset_ids = {
        let mut statement = conn.prepare(
            "SELECT id FROM experiment_datasets WHERE project_id = ?1 ORDER BY created_at_us, id",
        )?;
        let rows = statement
            .query_map([project_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    let datasets = dataset_ids
        .iter()
        .map(|id| query_dataset(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let split_group_ids = {
        let mut statement = conn.prepare(
            "SELECT g.id FROM experiment_dataset_split_groups g
             JOIN experiment_dataset_versions v ON v.id = g.dataset_version_id
             JOIN experiment_datasets d ON d.id = v.dataset_id
             WHERE d.project_id = ?1 ORDER BY g.created_at_us, g.id",
        )?;
        let rows = statement
            .query_map([project_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    let split_groups = split_group_ids
        .iter()
        .map(|id| query_split_group(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let transformation_ids = {
        let mut statement = conn.prepare(
            "SELECT t.id FROM experiment_dataset_transformations t
             JOIN experiment_dataset_versions v ON v.id = t.output_dataset_version_id
             JOIN experiment_datasets d ON d.id = v.dataset_id
             WHERE d.project_id = ?1 ORDER BY t.created_at_us, t.id",
        )?;
        let rows = statement
            .query_map([project_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    let transformations = transformation_ids
        .iter()
        .map(|id| query_dataset_transformation(conn, id))
        .collect::<Result<Vec<_>>>()?;
    Ok(ExperimentWorkspaceDto {
        design_versions,
        runs,
        datasets,
        split_groups,
        transformations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    fn temporary_workspace_path() -> PathBuf {
        std::env::temp_dir().join(format!("vora-experiment-{}.vora", Uuid::now_v7()))
    }

    fn initialize(conn: &Connection) {
        crate::mind::portable_db::initialize_workspace_schema(conn)
            .expect("initialize complete workspace schema");
    }

    fn project(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO research_projects
                (id, title, aim, lifecycle_status, created_at_us, updated_at_us)
             VALUES (?1, ?2, '', 'active', 1, 1)",
            params![id, format!("Project {id}")],
        )
        .expect("create project");
    }

    fn schema() -> DatasetSampleSchemaDto {
        DatasetSampleSchemaDto {
            schema_id: "market.observation".into(),
            schema_version: 1,
            fields: vec![DatasetFieldDefinitionDto {
                key: "price".into(),
                value_kind: DatasetSampleValueKind::Decimal,
                required: true,
                unit: Some("USD".into()),
            }],
            allow_additional_fields: false,
        }
    }

    fn data_sample(version_id: &str, sample_id: &str, value: &str) -> DatasetSampleDto {
        DatasetSampleDto::DataSample(DataSampleDto {
            sample_id: sample_id.into(),
            dataset_version_id: version_id.into(),
            occurred_at_us: Some(1_700_000_000_000_000),
            entity_ref: None,
            values: BTreeMap::from([(
                "price".into(),
                DatasetSampleValue::Decimal {
                    value: value.into(),
                    unit: "USD".into(),
                    uncertainty: None,
                },
            )]),
            provenance: vec![],
        })
    }

    fn valid_specification(
        datasets: Vec<DatasetRolePinDto>,
    ) -> ExperimentDesignSpecificationDto {
        ExperimentDesignSpecificationDto {
            target_research_question_id: None,
            target_hypothesis_version_id: None,
            target_model_version_id: Some("unresolved-model-version-v1".into()),
            estimand: "Difference in mean outcome".into(),
            study_type: StudyType::Other,
            intervention: Some("Treatment A".into()),
            control: Some("Control".into()),
            comparison_groups: vec![
                ComparisonGroupDto {
                    name: "treatment".into(),
                    description: "Receives Treatment A".into(),
                    intervention: true,
                },
                ComparisonGroupDto {
                    name: "control".into(),
                    description: "Receives the control".into(),
                    intervention: false,
                },
            ],
            factors: vec![],
            outcomes: vec![OperationalDefinitionDto {
                name: "primary outcome".into(),
                definition: "Observed outcome under the declared protocol".into(),
                unit: Some("USD".into()),
                measurement_method: Some("Recorded measurement".into()),
            }],
            covariates: vec![],
            unit_of_analysis: "individual observation".into(),
            sampling_frame: "Eligible study observations".into(),
            eligibility_rules: vec!["Consent recorded".into()],
            exclusion_rules: vec![],
            planned_sample_size: Some(100),
            sample_size_rationale: "Predeclared target".into(),
            stopping_rule: "Stop at the planned sample size".into(),
            randomization: Some("Random assignment within blocks".into()),
            blinding: Some("Outcome assessor blinded".into()),
            instruments: vec!["instrument-v1".into()],
            operational_definitions: vec![],
            data_collection_schedule: vec![DataCollectionScheduleEntryDto {
                stage: "baseline".into(),
                schedule: "Before intervention".into(),
                instrument_ids: vec!["instrument-v1".into()],
            }],
            primary_outcomes: vec!["primary outcome".into()],
            secondary_outcomes: vec!["safety outcome".into()],
            preregistered_analysis_plan: "Estimate the declared estimand".into(),
            multiplicity_strategy: "One primary outcome; secondary outcomes exploratory".into(),
            deviations_policy: "Record deviations before interpretation".into(),
            datasets,
            dataset_plan_rationale: "Prospective data plan is documented".into(),
            code_environment: CodeEnvironmentIdentityDto {
                code_uri: "repo://vora/experiment".into(),
                code_revision: Some("abc123".into()),
                environment: "Vora test environment".into(),
                dependency_lock_sha256: None,
            },
            random_seeds: vec!["seed-2026".into()],
            reproducibility_instructions: "Use the pinned code, environment, and seed".into(),
            market_context: None,
        }
    }

    fn create_typed_design(
        conn: &mut Connection,
        project_id: &str,
        title: &str,
        specification: ExperimentDesignSpecificationDto,
    ) -> ExperimentDesignVersionDto {
        create_design_version(
            conn,
            &CreateExperimentDesignVersionReq {
                project_id: project_id.into(),
                design_id: None,
                parent_version_id: None,
                title: title.into(),
                description: "Design intent".into(),
                protocol: "Intended procedure".into(),
                change_rationale: String::new(),
                specification,
                created_by: Some("researcher".into()),
            },
        )
        .expect("create typed design")
    }

    fn review_design(conn: &Connection, version_id: &str) {
        crate::research::repository::transition_research_design(
            conn,
            version_id,
            "review",
            Some("reviewer"),
            now_ts_us().expect("timestamp"),
        )
        .expect("review typed design");
    }

    #[test]
    fn preregistered_design_versions_are_immutable_and_failed_runs_persist() {
        let database_path = temporary_workspace_path();
        let mut conn = Connection::open(&database_path).expect("open workspace");
        initialize(&conn);
        project(&conn, "run-project");
        let first = create_typed_design(
            &mut conn,
            "run-project",
            "First design",
            valid_specification(vec![]),
        );
        review_design(&conn, &first.id);
        let preregistered = preregister_design(
            &mut conn,
            &PreregisterExperimentDesignReq {
                design_version_id: first.id.clone(),
                reviewer_id: Some("reviewer".into()),
                rationale: "Specification and required fields reviewed.".into(),
            },
        )
        .expect("preregister design");
        assert!(preregistered.preregistration.is_some());
        assert_eq!(
            preregistered.lifecycle_status,
            crate::research::ResearchDesignStatus::Preregistered
        );
        assert!(
            conn.execute(
                "UPDATE experiment_design_version_specs
                 SET specification_json = '{}' WHERE design_version_id = ?1",
                [&first.id]
            )
            .is_err(),
            "preregistered design spec cannot be changed"
        );
        assert!(
            crate::research::repository::transition_research_design(
                &conn,
                &first.id,
                "preregistered",
                Some("reviewer"),
                now_ts_us().expect("timestamp")
            )
            .is_err(),
            "the legacy transition cannot bypass preregistration validation"
        );

        let failed_start = create_run(
            &mut conn,
            &CreateExperimentRunReq {
                design_version_id: first.id.clone(),
                dataset_pins: vec![],
                created_by: Some("operator".into()),
            },
        )
        .expect("create independent run");
        assert_eq!(failed_start.lifecycle_status, ExperimentRunStatus::Created);
        assert!(
            transition_run(
                &mut conn,
                &TransitionExperimentRunReq {
                    run_id: failed_start.id.clone(),
                    next_status: ExperimentRunStatus::Running,
                    actor_id: Some("operator".into()),
                    details: "invalid skip".into(),
                    completion_summary: None,
                    failure_details: None,
                    output_dataset_pins: vec![],
                }
            )
            .is_err(),
            "created runs cannot skip the ready state"
        );
        transition_run(
            &mut conn,
            &TransitionExperimentRunReq {
                run_id: failed_start.id.clone(),
                next_status: ExperimentRunStatus::Ready,
                actor_id: Some("operator".into()),
                details: "inputs checked".into(),
                completion_summary: None,
                failure_details: None,
                output_dataset_pins: vec![],
            },
        )
        .expect("mark run ready");
        transition_run(
            &mut conn,
            &TransitionExperimentRunReq {
                run_id: failed_start.id.clone(),
                next_status: ExperimentRunStatus::Running,
                actor_id: Some("operator".into()),
                details: "execution started".into(),
                completion_summary: None,
                failure_details: None,
                output_dataset_pins: vec![],
            },
        )
        .expect("persist running before work");
        add_run_deviation(
            &mut conn,
            &AddExperimentRunDeviationReq {
                run_id: failed_start.id.clone(),
                description: "Instrument reset during capture".into(),
                rationale: "The reading procedure was interrupted".into(),
                impact_assessment: "Affected one planned measurement".into(),
                recorded_by: Some("operator".into()),
            },
        )
        .expect("record deviation");
        add_run_log(
            &mut conn,
            &AddExperimentRunLogReq {
                run_id: failed_start.id.clone(),
                severity: RunLogSeverity::Error,
                message: "Worker stopped unexpectedly".into(),
                details: serde_json::json!({"exitCode": 17}),
            },
        )
        .expect("record failed execution log");
        let failed = transition_run(
            &mut conn,
            &TransitionExperimentRunReq {
                run_id: failed_start.id.clone(),
                next_status: ExperimentRunStatus::Failed,
                actor_id: Some("operator".into()),
                details: "Worker exited before completion".into(),
                completion_summary: None,
                failure_details: Some("worker exit code 17".into()),
                output_dataset_pins: vec![],
            },
        )
        .expect("retain failed run");
        assert_eq!(failed.lifecycle_status, ExperimentRunStatus::Failed);
        assert_eq!(failed.deviations.len(), 1);
        assert_eq!(failed.logs.len(), 1);
        assert!(conn
            .execute("DELETE FROM experiment_runs WHERE id = ?1", [&failed.id])
            .is_err());

        let revision = create_design_version(
            &mut conn,
            &CreateExperimentDesignVersionReq {
                project_id: "run-project".into(),
                design_id: Some(first.design_id.clone()),
                parent_version_id: Some(first.id.clone()),
                title: "Revised design".into(),
                description: "Revised design intent".into(),
                protocol: "Revised intended procedure".into(),
                change_rationale: "Clarified the protocol without changing the prior snapshot."
                    .into(),
                specification: valid_specification(vec![]),
                created_by: Some("researcher".into()),
            },
        )
        .expect("create immutable design revision");
        assert_eq!(revision.version_no, 2);
        assert_eq!(revision.parent_version_id.as_deref(), Some(first.id.as_str()));
        assert_eq!(query_design_version(&conn, &first.id).unwrap().title, "First design");
        assert_eq!(failed.design_version_id, first.id);

        drop(conn);
        let reopened = Connection::open(&database_path).expect("reopen workspace");
        initialize(&reopened);
        let restored = workspace(&reopened, "run-project").expect("restore workspace");
        assert_eq!(restored.runs.len(), 1);
        assert_eq!(restored.runs[0].lifecycle_status, ExperimentRunStatus::Failed);
        assert_eq!(restored.design_versions.len(), 2);
        drop(reopened);
        fs::remove_file(database_path).expect("remove temporary workspace database");
    }

    #[test]
    fn dataset_versions_validate_assets_samples_splits_and_exact_run_pins() {
        let database_path = temporary_workspace_path();
        let assets_root = std::env::temp_dir().join(format!("vora-assets-{}", Uuid::now_v7()));
        fs::create_dir_all(&assets_root).expect("create test asset directory");
        let source_file = assets_root.join("market.csv");
        fs::write(&source_file, b"sample,price\ns1,10.5\ns2,11.0\n")
            .expect("write fixture asset");
        let file_hash = format!(
            "{:x}",
            Sha256::digest(fs::read(&source_file).expect("read source asset"))
        );
        let db = Mutex::new(Connection::open(&database_path).expect("open workspace"));
        initialize(&db.lock().expect("lock").as_ref());
        project(&db.lock().expect("lock"), "dataset-project");

        let dataset = {
            let mut conn = db.lock().expect("lock");
            create_dataset(
                &mut conn,
                &CreateDatasetReq {
                    project_id: "dataset-project".into(),
                    title: "Market observations".into(),
                    description: "Version-pinned sample dataset".into(),
                    created_by: Some("researcher".into()),
                },
            )
            .expect("create dataset")
        };
        let version = {
            let mut conn = db.lock().expect("lock");
            create_dataset_version(
                &mut conn,
                &CreateDatasetVersionReq {
                    dataset_id: dataset.id.clone(),
                    schema: schema(),
                    expected_sample_count: 2,
                    created_by: Some("researcher".into()),
                },
            )
            .expect("create staged dataset version")
        };
        {
            let mut conn = db.lock().expect("lock");
            add_dataset_samples(
                &mut conn,
                &AddDatasetSamplesReq {
                    dataset_version_id: version.id.clone(),
                    samples: vec![
                        data_sample(&version.id, "s1", "10.5"),
                        data_sample(&version.id, "s2", "11.0"),
                    ],
                },
            )
            .expect("insert queryable samples");
            assert!(
                add_dataset_samples(
                    &mut conn,
                    &AddDatasetSamplesReq {
                        dataset_version_id: version.id.clone(),
                        samples: vec![data_sample(&version.id, "s1", "10.5")],
                    }
                )
                .is_err(),
                "duplicate stable sample IDs must be rejected"
            );
            assert!(
                add_dataset_split_group(
                    &mut conn,
                    &AddDatasetSplitGroupReq {
                        dataset_version_id: version.id.clone(),
                        split_group_key: "leakage-checked".into(),
                        require_disjoint: true,
                        assignments: vec![
                            DatasetSplitAssignmentReq {
                                sample_id: "s1".into(),
                                role: DatasetRole::Training,
                                split_key: "train".into(),
                            },
                            DatasetSplitAssignmentReq {
                                sample_id: "s1".into(),
                                role: DatasetRole::Test,
                                split_key: "test".into(),
                            },
                        ],
                    }
                )
                .is_err(),
                "disjoint split membership rejects duplicate sample assignment"
            );
            add_dataset_split_group(
                &mut conn,
                &AddDatasetSplitGroupReq {
                    dataset_version_id: version.id.clone(),
                    split_group_key: "leakage-checked".into(),
                    require_disjoint: true,
                    assignments: vec![
                        DatasetSplitAssignmentReq {
                            sample_id: "s1".into(),
                            role: DatasetRole::Training,
                            split_key: "train".into(),
                        },
                        DatasetSplitAssignmentReq {
                            sample_id: "s2".into(),
                            role: DatasetRole::Test,
                            split_key: "test".into(),
                        },
                    ],
                },
            )
            .expect("create disjoint train/test split");
        }
        assert!(
            service::register_dataset_asset(
                &db,
                &RegisterDatasetAssetReq {
                    dataset_version_id: version.id.clone(),
                    source_path: source_file.to_string_lossy().into_owned(),
                    original_name: "market.csv".into(),
                    media_type: "text/csv".into(),
                    expected_sha256: Some("0".repeat(64)),
                },
                &assets_root
            )
            .is_err(),
            "asset checksum mismatch must fail before manifest registration"
        );
        let asset = service::register_dataset_asset(
            &db,
            &RegisterDatasetAssetReq {
                dataset_version_id: version.id.clone(),
                source_path: source_file.to_string_lossy().into_owned(),
                original_name: "market.csv".into(),
                media_type: "text/csv".into(),
                expected_sha256: Some(file_hash.clone()),
            },
            &assets_root,
        )
        .expect("register and hash managed dataset asset");
        let sealed = service::seal_dataset_version(&db, &version.id, &assets_root)
            .expect("verify and seal dataset version");
        assert_eq!(sealed.actual_sample_count, 2);
        assert!(sealed.content_sha256.is_some());
        service::verify_dataset_version(&db, &version.id, &assets_root)
            .expect("verify sealed dataset assets and manifest");

        let pin = DatasetRolePinDto {
            dataset_version_id: version.id.clone(),
            role: DatasetRole::Training,
            rationale: "Pinned source version".into(),
        };
        let design = {
            let mut conn = db.lock().expect("lock");
            create_typed_design(
                &mut conn,
                "dataset-project",
                "Pinned dataset design",
                valid_specification(vec![pin.clone()]),
            )
        };
        {
            let conn = db.lock().expect("lock");
            review_design(&conn, &design.id);
        }
        {
            let mut conn = db.lock().expect("lock");
            preregister_design(
                &mut conn,
                &PreregisterExperimentDesignReq {
                    design_version_id: design.id.clone(),
                    reviewer_id: Some("reviewer".into()),
                    rationale: "Exact dataset version and analysis plan reviewed.".into(),
                },
            )
            .expect("preregister exact dataset pin");
        }
        let run = service::create_run(
            &db,
            &CreateExperimentRunReq {
                design_version_id: design.id.clone(),
                dataset_pins: vec![pin.clone()],
                created_by: Some("operator".into()),
            },
            &assets_root,
        )
        .expect("create run with exact sealed dataset version");
        assert_eq!(run.dataset_pins.len(), 1);
        assert_eq!(run.dataset_pins[0].dataset_version_id, version.id);
        assert!(
            service::create_run(
                &db,
                &CreateExperimentRunReq {
                    design_version_id: design.id.clone(),
                    dataset_pins: vec![DatasetRolePinDto {
                        dataset_version_id: pin.dataset_version_id.clone(),
                        role: DatasetRole::Validation,
                        rationale: "mismatched role".into(),
                    }],
                    created_by: None,
                },
                &assets_root,
            )
            .is_err(),
            "run cannot replace a design-pinned dataset role"
        );
        let managed_asset = assets_root.join(&asset.storage_key);
        fs::write(&managed_asset, b"modified asset").expect("modify managed file");
        assert!(
            service::verify_dataset_version(&db, &version.id, &assets_root).is_err(),
            "changed asset contents fail checksum verification"
        );
        assert!(
            service::transition_run(
                &db,
                &TransitionExperimentRunReq {
                    run_id: run.id.clone(),
                    next_status: ExperimentRunStatus::Ready,
                    actor_id: Some("operator".into()),
                    details: "validate inputs".into(),
                    completion_summary: None,
                    failure_details: None,
                    output_dataset_pins: vec![],
                },
                &assets_root,
            )
            .is_err(),
            "a run cannot become ready with a changed pinned asset"
        );
        fs::write(&managed_asset, fs::read(&source_file).expect("read source asset"))
            .expect("restore managed file");
        let samples = {
            let conn = db.lock().expect("lock");
            get_dataset_samples(&conn, &version.id, Some("s"), 0, 20)
                .expect("query samples by stable ID")
        };
        assert_eq!(samples.total_count, 2);
        assert_eq!(samples.samples[0].data_sample().sample_id, "s1");
        let workspace_state = {
            let conn = db.lock().expect("lock");
            workspace(&conn, "dataset-project").expect("query experimental workspace")
        };
        assert_eq!(
            workspace_state.split_groups[0].assignments.len(),
            2
        );
        assert!(workspace_state.transformations.is_empty());

        drop(db);
        let reopened = Connection::open(&database_path).expect("reopen workspace");
        initialize(&reopened);
        let restored = workspace(&reopened, "dataset-project").expect("restore dataset state");
        assert_eq!(restored.datasets[0].versions[0].content_sha256, sealed.content_sha256);
        assert_eq!(restored.runs[0].dataset_pins[0].dataset_version_id, version.id);
        assert_eq!(
            get_dataset_samples(&reopened, &version.id, None, 0, 20)
                .expect("query restored samples")
                .total_count,
            2
        );
        drop(reopened);
        fs::remove_file(database_path).expect("remove temporary workspace database");
        fs::remove_dir_all(assets_root).expect("remove temporary asset directory");
    }

    #[test]
    fn untyped_legacy_designs_remain_visible_but_cannot_start_runs() {
        let conn = Connection::open_in_memory().expect("open in-memory workspace");
        initialize(&conn);
        project(&conn, "legacy-design-project");
        conn.execute(
            "INSERT INTO research_experiment_designs
                (id, project_id, title, created_at_us)
             VALUES ('legacy-design', 'legacy-design-project', 'Legacy design', 1);
             INSERT INTO research_experiment_design_versions
                (id, design_id, version_no, description, protocol, lifecycle_status,
                 created_at_us, preregistered_at_us)
             VALUES ('legacy-design-v1', 'legacy-design', 1, 'Legacy description',
                     'Legacy protocol', 'preregistered', 1, 1);",
            [],
        )
        .expect("insert legacy design");
        initialize_schema(&conn).expect("repeat experiment migration");
        let designs = workspace(&conn, "legacy-design-project").expect("read legacy design");
        assert_eq!(designs.design_versions.len(), 1);
        assert!(designs.design_versions[0].specification.is_none());
        assert!(designs.design_versions[0].preregistration.is_none());
        assert!(
            create_run(
                &mut conn.unchecked_transaction().unwrap(),
                &CreateExperimentRunReq {
                    design_version_id: "legacy-design-v1".into(),
                    dataset_pins: vec![],
                    created_by: None,
                }
            )
            .is_err()
        );
    }

    #[test]
    fn decimals_and_trade_sample_fields_preserve_units_and_time_order() {
        assert!(validate_decimal("-0.003").is_ok());
        assert!(validate_decimal("+1.0").is_err());
        assert!(validate_decimal("01.0").is_err());
        let trade = DatasetSampleDto::TradeSample(TradeSampleDto {
            sample: DataSampleDto {
                sample_id: "trade-1".into(),
                dataset_version_id: "dataset-v1".into(),
                occurred_at_us: Some(10),
                entity_ref: Some("instrument:XYZ".into()),
                values: BTreeMap::from([(
                    "price".into(),
                    DatasetSampleValue::Decimal {
                        value: "12.5".into(),
                        unit: "USD".into(),
                        uncertainty: None,
                    },
                )]),
                provenance: vec![],
            },
            instrument: "XYZ".into(),
            order_side: Some(MarketOrderSide::Buy),
            order_type: Some(MarketOrderType::Limit),
            order_quantity: Some(DecimalMeasurement {
                value: "3".into(),
                unit: "shares".into(),
            }),
            execution_price: Some(DecimalMeasurement {
                value: "12.5".into(),
                unit: "USD/share".into(),
            }),
            fees: Some(DecimalMeasurement {
                value: "0.10".into(),
                unit: "USD".into(),
            }),
            slippage: Some(DecimalMeasurement {
                value: "0.02".into(),
                unit: "USD/share".into(),
            }),
            venue: Some("exchange".into()),
            market_session: Some("regular".into()),
            session_assumptions: vec!["regular session only".into()],
            decision_at_us: Some(9),
            execution_at_us: Some(10),
        });
        assert!(validate_sample(&schema(), &trade).is_ok());
        let DatasetSampleDto::TradeSample(mut invalid_trade) = trade else {
            unreachable!();
        };
        invalid_trade.execution_at_us = Some(8);
        assert!(validate_sample(
            &schema(),
            &DatasetSampleDto::TradeSample(invalid_trade)
        )
        .is_err());
    }
}
