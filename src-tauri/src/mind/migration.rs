use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OpenFlags, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const REQUIRED_CURRENT_TABLES: &[&str] = &[
    "sources",
    "segments",
    "extracts",
    "links",
    "research_projects",
    "research_questions",
    "research_hypotheses",
    "scientific_models",
    "model_versions",
];
const LEDGER_TABLE: &str = "vora_migration_ledger";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaProfile {
    CurrentSupported { fingerprint: String },
    Incomplete { missing_tables: Vec<String> },
    Unknown { fingerprint: String },
    Unsupported { user_version: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationOutcome {
    Published { backup_path: PathBuf },
    AlreadyApplied,
}

#[derive(Debug, Clone)]
pub struct ApprovedMigration {
    id: String,
    sql: String,
    checksum: String,
}

impl ApprovedMigration {
    pub fn new(id: impl Into<String>, sql: impl Into<String>) -> Result<Self> {
        let id = id.into();
        let sql = sql.into();
        if id.trim().is_empty() || sql.trim().is_empty() {
            bail!("Migration id and SQL must not be empty");
        }
        let checksum = format!("{:x}", Sha256::digest(sql.as_bytes()));
        Ok(Self { id, sql, checksum })
    }
}

#[derive(Debug, Clone)]
pub struct MigrationPlan {
    steps: Vec<ApprovedMigration>,
    expected_schema_fingerprint: String,
}

impl MigrationPlan {
    pub fn new(
        steps: Vec<ApprovedMigration>,
        expected_schema_fingerprint: impl Into<String>,
    ) -> Result<Self> {
        let expected_schema_fingerprint = expected_schema_fingerprint.into();
        if steps.is_empty() || expected_schema_fingerprint.trim().is_empty() {
            bail!("A migration plan needs steps and an expected schema fingerprint");
        }
        for (index, step) in steps.iter().enumerate() {
            if steps[..index].iter().any(|previous| previous.id == step.id) {
                bail!("Duplicate migration id in plan: {}", step.id);
            }
        }
        Ok(Self {
            steps,
            expected_schema_fingerprint,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryOutcome {
    Published { operation_directory: PathBuf },
    Abandoned { operation_directory: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailurePoint {
    None,
    BeforeStaging,
    AfterMigrations,
    AfterValidation,
    DuringBackupCopy,
    AfterBackupCopyBeforeJournal,
    BeforePublication,
    DuringPublication,
    AfterPublicationBeforeJournal,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum JournalEvent {
    Started,
    Staged,
    MigrationsApplied,
    SourceObserved { original_sha256: String },
    Validated { expected_fingerprint: String },
    BackupPrepared { original_sha256: String },
    Publishing,
    // Kept to recognize journal entries written by the earlier coordinator.
    SourceBackedUp,
    Published { expected_fingerprint: String },
    Abandoned,
}

struct CoordinatorLock {
    _file: File,
}

impl CoordinatorLock {
    fn acquire(source_path: &Path) -> Result<Self> {
        let lock_path = sidecar_path(source_path, "migration.lock")?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .with_context(|| format!("Open migration lock {}", lock_path.display()))?;
        file.try_lock().with_context(|| {
            format!("Another migration operation holds {}", lock_path.display())
        })?;
        Ok(Self { _file: file })
    }
}

/// Recognize a database without changing it. Only the exact schema produced by
/// the current workspace initializer is supported; no legacy profile is certified.
pub fn recognize_profile(source_path: &Path) -> Result<SchemaProfile> {
    let source = open_read_only(source_path)?;
    recognize_profile_on_connection(&source)
}

/// Return the deterministic fingerprint of a database's non-internal schema.
pub fn schema_fingerprint_for_path(source_path: &Path) -> Result<String> {
    let source = open_read_only(source_path)?;
    schema_fingerprint(&source)
}

/// Stage and publish an explicitly approved migration plan.
///
/// The trusted caller must quiesce database activity and close every connection
/// to the source before calling. Sidecar checks and the coordinator lock cannot
/// establish that external SQLite clients are absent; this API is not available
/// for production use until a trusted caller can guarantee quiescence.
/// Publication replaces the canonical file with one same-volume rename. If the
/// platform or filesystem cannot atomically replace an existing file, the call
/// fails without using a two-rename fallback.
pub fn execute_staged_migrations(
    source_path: &Path,
    plan: &MigrationPlan,
) -> Result<MigrationOutcome> {
    execute_with_failure(source_path, plan, FailurePoint::None)
}

/// Recover operations left in a source's adjacent staging directories.
///
/// A validated stage is completed only when the canonical file still matches
/// the original backup or the validated output. The trusted caller must
/// quiesce database activity and close every source connection. Sidecar checks
/// and the lock cannot establish that external SQLite clients are absent; this
/// API is not available for production use until a trusted caller can guarantee
/// quiescence.
pub fn recover_incomplete_operations(source_path: &Path) -> Result<Vec<RecoveryOutcome>> {
    let _lock = CoordinatorLock::acquire(source_path)?;
    ensure_no_sqlite_sidecars(source_path)?;
    match fs::symlink_metadata(source_path) {
        Ok(metadata) if metadata.file_type().is_file() => {}
        Ok(_) => bail!("Canonical database must be a regular, non-symlink file"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Inspect canonical database {}", source_path.display()))
        }
    }
    let (parent, source_name) = source_location(source_path)?;
    let prefix = format!(".{source_name}.migration-");
    let mut directories = fs::read_dir(parent)
        .with_context(|| format!("Read migration directory {}", parent.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    directories.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(&prefix))
            && path.is_dir()
    });
    directories.sort();

    let mut outcomes = Vec::new();
    for operation_directory in directories {
        let journal_path = operation_directory.join("journal.log");
        if !journal_path.exists() {
            bail!(
                "Migration operation directory has no journal: {}",
                operation_directory.display()
            );
        }
        let events = read_journal(&journal_path)?;
        if matches!(
            events.last(),
            Some(JournalEvent::Published { .. } | JournalEvent::Abandoned)
        ) {
            continue;
        }
        let source = source_path.to_path_buf();
        let staged = operation_directory.join("staging.sqlite");
        let backup = operation_directory.join("backup.sqlite");
        let expected_fingerprint = events.iter().rev().find_map(|event| match event {
            JournalEvent::Validated {
                expected_fingerprint,
            }
            | JournalEvent::Published {
                expected_fingerprint,
            } => Some(expected_fingerprint.clone()),
            _ => None,
        });
        let recorded_backup_hash = events.iter().rev().find_map(|event| match event {
            JournalEvent::BackupPrepared { original_sha256 } => Some(original_sha256.clone()),
            _ => None,
        });
        let observed_source_hash = events.iter().rev().find_map(|event| match event {
            JournalEvent::SourceObserved { original_sha256 } => Some(original_sha256.clone()),
            _ => None,
        });
        let legacy_backup_event = events
            .iter()
            .any(|event| matches!(event, JournalEvent::SourceBackedUp));
        let legacy_publish_attempt = events
            .iter()
            .any(|event| matches!(event, JournalEvent::Publishing));

        let Some(expected_fingerprint) = expected_fingerprint else {
            if !source.exists() && backup.exists() && legacy_backup_event {
                let backup_hash = sha256_file(&backup)?;
                validate_original_backup(&backup, &backup_hash)?;
                restore_from_backup_atomically(
                    &backup,
                    &source,
                    &operation_directory,
                    &backup_hash,
                )?;
            }
            if !source.exists() {
                bail!(
                    "Cannot recover source database for {}",
                    operation_directory.display()
                );
            }
            append_journal(&journal_path, &JournalEvent::Abandoned)?;
            outcomes.push(RecoveryOutcome::Abandoned {
                operation_directory,
            });
            continue;
        };

        let mut original_sha256 = recorded_backup_hash.or(observed_source_hash);
        if original_sha256.is_none() && legacy_backup_event && backup.exists() {
            original_sha256 = Some(sha256_file(&backup)?);
        } else if original_sha256.is_none()
            && !source.exists()
            && backup.exists()
            && staged.exists()
            && legacy_publish_attempt
        {
            if !matches!(
                recognize_profile(&backup)?,
                SchemaProfile::CurrentSupported { .. }
            ) {
                bail!("Legacy interrupted publication has an unsupported original backup");
            }
            original_sha256 = Some(sha256_file(&backup)?);
        }
        if !source.exists() {
            if !backup.exists() || original_sha256.is_none() {
                bail!(
                    "Canonical database is missing and its original backup is unverified for {}",
                    operation_directory.display()
                );
            }
            let expected_original_hash = original_sha256
                .as_deref()
                .context("Original backup checksum is unavailable")?;
            if sha256_file(&backup)? != expected_original_hash {
                bail!("Migration backup checksum mismatch: {}", backup.display());
            }
            validate_original_backup(&backup, expected_original_hash)?;
            restore_from_backup_atomically(
                &backup,
                &source,
                &operation_directory,
                expected_original_hash,
            )?;
        }

        let original_sha256 = match original_sha256 {
            Some(hash) => {
                if !backup.exists() {
                    if sha256_file(&source)? != hash {
                        bail!("Migration backup is missing and source is not the original");
                    }
                    validate_original_backup(&source, &hash)?;
                    copy_file_durably(&source, &backup)?;
                    validate_original_backup(&backup, &hash)?;
                } else if sha256_file(&backup)? != hash {
                    if sha256_file(&source)? != hash {
                        bail!("Migration backup checksum mismatch: {}", backup.display());
                    }
                    validate_original_backup(&source, &hash)?;
                    preserve_unverified_backup(&backup, &operation_directory)?;
                    copy_file_durably(&source, &backup)?;
                    validate_original_backup(&backup, &hash)?;
                } else {
                    validate_original_backup(&backup, &hash)?;
                }
                hash
            }
            None => {
                let hash = if backup.exists() {
                    let source_hash = sha256_file(&source)?;
                    if sha256_file(&backup)? != source_hash {
                        bail!("Unrecorded migration backup does not match canonical database");
                    }
                    validate_original_backup(&backup, &source_hash)?;
                    source_hash
                } else {
                    let source_hash = sha256_file(&source)?;
                    validate_original_backup(&source, &source_hash)?;
                    copy_file_durably(&source, &backup)?;
                    validate_original_backup(&backup, &source_hash)?;
                    source_hash
                };
                append_journal(
                    &journal_path,
                    &JournalEvent::BackupPrepared {
                        original_sha256: hash.clone(),
                    },
                )?;
                hash
            }
        };
        if !events
            .iter()
            .any(|event| matches!(event, JournalEvent::BackupPrepared { .. }))
        {
            append_journal(
                &journal_path,
                &JournalEvent::BackupPrepared {
                    original_sha256: original_sha256.clone(),
                },
            )?;
        }

        if sha256_file(&source)? == original_sha256 {
            if staged.exists() {
                validate_output(&staged, &expected_fingerprint)
                    .context("Validate staged database during recovery")?;
                append_journal(&journal_path, &JournalEvent::Publishing)?;
                atomic_replace(&staged, &source)
                    .context("Atomically publish validated staged database during recovery")?;
                sync_file(&source)?;
                if let Err(error) = validate_output(&source, &expected_fingerprint) {
                    restore_from_backup_atomically(
                        &backup,
                        &source,
                        &operation_directory,
                        &original_sha256,
                    )?;
                    append_journal(&journal_path, &JournalEvent::Abandoned)?;
                    return Err(error).context("Recovered publication was rolled back");
                }
                append_journal(
                    &journal_path,
                    &JournalEvent::Published {
                        expected_fingerprint,
                    },
                )?;
                outcomes.push(RecoveryOutcome::Published {
                    operation_directory,
                });
            } else {
                append_journal(&journal_path, &JournalEvent::Abandoned)?;
                outcomes.push(RecoveryOutcome::Abandoned {
                    operation_directory,
                });
            }
            continue;
        }

        validate_output(&source, &expected_fingerprint)
            .context("Canonical database matches neither original backup nor validated output")?;
        if staged.exists() {
            bail!(
                "Both canonical output and staging database exist for {}",
                operation_directory.display()
            );
        }
        sync_file(&source)?;
        append_journal(
            &journal_path,
            &JournalEvent::Published {
                expected_fingerprint,
            },
        )?;
        outcomes.push(RecoveryOutcome::Published {
            operation_directory,
        });
    }
    Ok(outcomes)
}

fn execute_with_failure(
    source_path: &Path,
    plan: &MigrationPlan,
    failure: FailurePoint,
) -> Result<MigrationOutcome> {
    let _lock = CoordinatorLock::acquire(source_path)?;
    let metadata = fs::symlink_metadata(source_path)
        .with_context(|| format!("Inspect source database {}", source_path.display()))?;
    if !metadata.file_type().is_file() {
        bail!("Source database must be a regular, non-symlink file");
    }
    ensure_no_sqlite_sidecars(source_path)?;

    {
        let source = open_read_only(source_path)?;
        if plan_is_already_applied(&source, plan)? {
            validate_output(
                &source_path.to_path_buf(),
                &plan.expected_schema_fingerprint,
            )?;
            return Ok(MigrationOutcome::AlreadyApplied);
        }
        match recognize_profile_on_connection(&source)? {
            SchemaProfile::CurrentSupported { .. } => {
                ensure_empty_or_missing_ledger(&source)?;
            }
            profile => bail!("Migration source profile is not supported: {profile:?}"),
        }
    }

    let (parent, source_name) = source_location(source_path)?;
    let operation_directory =
        parent.join(format!(".{source_name}.migration-{}", uuid::Uuid::now_v7()));
    fs::create_dir(&operation_directory).with_context(|| {
        format!(
            "Create isolated migration directory {}",
            operation_directory.display()
        )
    })?;
    let journal_path = operation_directory.join("journal.log");
    append_journal(&journal_path, &JournalEvent::Started)?;
    let original_sha256 = sha256_file(source_path)?;
    append_journal(
        &journal_path,
        &JournalEvent::SourceObserved {
            original_sha256: original_sha256.clone(),
        },
    )?;
    let staged_path = operation_directory.join("staging.sqlite");
    let backup_path = operation_directory.join("backup.sqlite");

    if failure == FailurePoint::BeforeStaging {
        bail!("Injected failure before staging");
    }
    create_staging_copy(source_path, &staged_path)?;
    append_journal(&journal_path, &JournalEvent::Staged)?;

    {
        let mut staged = open_staging(&staged_path)?;
        for (ordinal, step) in plan.steps.iter().enumerate() {
            let transaction = staged.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.execute_batch(
                "CREATE TABLE IF NOT EXISTS vora_migration_ledger (
                    ordinal INTEGER PRIMARY KEY,
                    migration_id TEXT NOT NULL UNIQUE,
                    checksum TEXT NOT NULL,
                    schema_fingerprint TEXT
                );",
            )?;
            transaction.execute_batch(&step.sql)?;
            transaction.execute(
                "INSERT INTO vora_migration_ledger
                    (ordinal, migration_id, checksum, schema_fingerprint)
                 VALUES (?1, ?2, ?3, NULL)",
                rusqlite::params![ordinal as i64 + 1, step.id, step.checksum],
            )?;
            transaction.commit()?;
        }
        append_journal(&journal_path, &JournalEvent::MigrationsApplied)?;
        if failure == FailurePoint::AfterMigrations {
            bail!("Injected failure after migrations");
        }

        validate_database(&staged, &plan.expected_schema_fingerprint)?;
        {
            let transaction = staged.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.execute(
                "UPDATE vora_migration_ledger SET schema_fingerprint = ?1",
                [&plan.expected_schema_fingerprint],
            )?;
            transaction.commit()?;
        }
        validate_ledger(&staged, plan)?;
        validate_database(&staged, &plan.expected_schema_fingerprint)?;
        staged.pragma_update(None, "journal_mode", "DELETE")?;
    }

    append_journal(
        &journal_path,
        &JournalEvent::Validated {
            expected_fingerprint: plan.expected_schema_fingerprint.clone(),
        },
    )?;
    if failure == FailurePoint::AfterValidation {
        bail!("Injected failure after validation");
    }

    if sha256_file(source_path)? != original_sha256 {
        bail!("Source database changed during staging; refusing publication");
    }
    if failure == FailurePoint::DuringBackupCopy {
        create_partial_copy_for_failure(source_path, &backup_path)?;
        bail!("Injected interruption during backup copy");
    }
    let backup_hash = copy_file_durably(source_path, &backup_path)?;
    if backup_hash != original_sha256 {
        bail!("Source database changed while its recoverable backup was being created");
    }
    validate_original_backup(&backup_path, &original_sha256)?;
    if failure == FailurePoint::AfterBackupCopyBeforeJournal {
        bail!("Injected interruption after backup copy and before journal completion");
    }
    append_journal(
        &journal_path,
        &JournalEvent::BackupPrepared { original_sha256 },
    )?;
    if failure == FailurePoint::BeforePublication {
        bail!("Injected failure before atomic publication");
    }

    append_journal(&journal_path, &JournalEvent::Publishing)?;
    if failure == FailurePoint::DuringPublication {
        let absent_staging = operation_directory.join("absent-staging.sqlite");
        atomic_replace(&absent_staging, source_path)
            .expect_err("injected replacement must fail without a source file");
        bail!("Injected atomic publication failure");
    }
    if sha256_file(source_path)? != backup_hash {
        bail!("Source database changed after backup preparation; refusing publication");
    }
    atomic_replace(&staged_path, source_path).context("Atomically publish validated stage")?;
    sync_file(source_path)?;
    if let Err(error) = validate_output(source_path, &plan.expected_schema_fingerprint) {
        restore_from_backup_atomically(
            &backup_path,
            source_path,
            &operation_directory,
            &backup_hash,
        )?;
        append_journal(&journal_path, &JournalEvent::Abandoned)?;
        return Err(error).context("Published database failed confirmation and was rolled back");
    }
    if failure == FailurePoint::AfterPublicationBeforeJournal {
        bail!("Injected interruption after atomic publication");
    }
    append_journal(
        &journal_path,
        &JournalEvent::Published {
            expected_fingerprint: plan.expected_schema_fingerprint.clone(),
        },
    )?;

    Ok(MigrationOutcome::Published { backup_path })
}

fn plan_is_already_applied(source: &Connection, plan: &MigrationPlan) -> Result<bool> {
    let table_exists: bool = source.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [LEDGER_TABLE],
        |row| row.get(0),
    )?;
    if !table_exists {
        return Ok(false);
    }
    validate_ledger_fingerprint(source, &plan.expected_schema_fingerprint)?;
    let entries = read_ledger(source)?;
    if entries.len() != plan.steps.len() || entries.is_empty() {
        return Ok(false);
    }
    for (index, (id, checksum, fingerprint)) in entries.iter().enumerate() {
        let step = &plan.steps[index];
        if id != &step.id
            || checksum != &step.checksum
            || fingerprint.as_deref() != Some(&plan.expected_schema_fingerprint)
        {
            return Ok(false);
        }
    }
    Ok(schema_fingerprint(source)? == plan.expected_schema_fingerprint)
}

fn ensure_empty_or_missing_ledger(source: &Connection) -> Result<()> {
    let table_exists: bool = source.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [LEDGER_TABLE],
        |row| row.get(0),
    )?;
    if table_exists && !read_ledger(source)?.is_empty() {
        bail!("Source contains migration ledger entries not approved by this plan");
    }
    Ok(())
}

fn read_ledger(source: &Connection) -> Result<Vec<(String, String, Option<String>)>> {
    let mut statement = source.prepare(
        "SELECT migration_id, checksum, schema_fingerprint
         FROM vora_migration_ledger ORDER BY ordinal",
    )?;
    let entries = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(entries)
}

fn validate_ledger(source: &Connection, plan: &MigrationPlan) -> Result<()> {
    let entries = read_ledger(source)?;
    if entries.len() != plan.steps.len() {
        bail!("Migration ledger entry count does not match the approved plan");
    }
    for (index, ((id, checksum, fingerprint), step)) in entries.iter().zip(&plan.steps).enumerate()
    {
        if id != &step.id
            || checksum != &step.checksum
            || fingerprint.as_deref() != Some(&plan.expected_schema_fingerprint)
        {
            bail!("Migration ledger is inconsistent at step {}", index + 1);
        }
    }
    Ok(())
}

fn create_staging_copy(source_path: &Path, staged_path: &Path) -> Result<()> {
    let source = open_read_only(source_path)?;
    let mut target = Connection::open(staged_path)?;
    enable_foreign_keys(&target)?;
    {
        let backup = rusqlite::backup::Backup::new(&source, &mut target)?;
        backup.run_to_completion(5, Duration::from_millis(100), None)?;
    }
    target.pragma_update(None, "journal_mode", "DELETE")?;
    drop(target);
    sync_file(staged_path)?;
    ensure_no_sqlite_sidecars(staged_path)?;
    Ok(())
}

fn open_read_only(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.execute_batch("PRAGMA query_only = ON;")?;
    enable_foreign_keys(&connection)?;
    let query_only: i64 = connection.query_row("PRAGMA query_only", [], |row| row.get(0))?;
    if query_only != 1 {
        bail!("SQLite read-only connection did not enable query_only");
    }
    Ok(connection)
}

fn open_staging(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    enable_foreign_keys(&connection)?;
    Ok(connection)
}

fn enable_foreign_keys(connection: &Connection) -> Result<()> {
    connection.pragma_update(None, "foreign_keys", true)?;
    let enabled: i64 = connection.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    if enabled != 1 {
        bail!("SQLite foreign-key enforcement is not enabled");
    }
    Ok(())
}

fn recognize_profile_on_connection(source: &Connection) -> Result<SchemaProfile> {
    let user_version: i64 = source.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if user_version > 1 {
        return Ok(SchemaProfile::Unsupported { user_version });
    }
    let tables = table_names(source)?;
    let missing_tables = REQUIRED_CURRENT_TABLES
        .iter()
        .filter(|required| !tables.iter().any(|table| table == **required))
        .map(|table| (*table).to_string())
        .collect::<Vec<_>>();
    if !missing_tables.is_empty() {
        return Ok(SchemaProfile::Incomplete { missing_tables });
    }
    let observed = schema_fingerprint(source)?;
    let expected = current_schema_fingerprint()?;
    if user_version == 1 && observed == expected {
        Ok(SchemaProfile::CurrentSupported {
            fingerprint: observed,
        })
    } else {
        Ok(SchemaProfile::Unknown {
            fingerprint: observed,
        })
    }
}

fn current_schema_fingerprint() -> Result<String> {
    let connection = Connection::open_in_memory()?;
    super::portable_db::initialize_workspace_schema(&connection)
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    schema_fingerprint(&connection)
}

fn schema_fingerprint(connection: &Connection) -> Result<String> {
    let mut statement = connection.prepare(
        "SELECT type, name, tbl_name, sql FROM sqlite_master
         WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' AND name <> ?1
         ORDER BY type, name, tbl_name",
    )?;
    let objects = statement
        .query_map([LEDGER_TABLE], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);
    let serialized = serde_json::to_vec(&objects)?;
    Ok(format!("{:x}", Sha256::digest(serialized)))
}

fn table_names(connection: &Connection) -> Result<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT name FROM sqlite_master
         WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )?;
    let names = statement
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(names)
}

fn validate_output(path: &Path, expected_fingerprint: &str) -> Result<()> {
    let connection = open_staging(path)?;
    validate_database(&connection, expected_fingerprint)?;
    validate_ledger_fingerprint(&connection, expected_fingerprint)?;
    Ok(())
}

fn validate_original_backup(path: &Path, expected_sha256: &str) -> Result<()> {
    ensure_no_sqlite_sidecars(path)?;
    let observed_sha256 = sha256_file(path)?;
    if observed_sha256 != expected_sha256 {
        bail!(
            "Migration backup checksum mismatch: expected {expected_sha256}, observed {observed_sha256}"
        );
    }
    let connection = open_read_only(path)
        .with_context(|| format!("Open migration backup as SQLite: {}", path.display()))?;
    validate_sqlite_integrity(&connection)
        .with_context(|| format!("Validate migration backup: {}", path.display()))?;
    match recognize_profile_on_connection(&connection)? {
        SchemaProfile::CurrentSupported { .. } => ensure_empty_or_missing_ledger(&connection),
        profile => bail!("Migration backup profile is not supported: {profile:?}"),
    }
}

fn validate_database(connection: &Connection, expected_fingerprint: &str) -> Result<()> {
    validate_sqlite_integrity(connection)?;
    let tables = table_names(connection)?;
    let missing = REQUIRED_CURRENT_TABLES
        .iter()
        .filter(|required| !tables.iter().any(|table| table == **required))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!("Required application tables are missing: {missing:?}");
    }
    let observed_fingerprint = schema_fingerprint(connection)?;
    if observed_fingerprint != expected_fingerprint {
        bail!(
            "Post-migration schema fingerprint mismatch: expected {expected_fingerprint}, observed {observed_fingerprint}"
        );
    }
    Ok(())
}

fn validate_sqlite_integrity(connection: &Connection) -> Result<()> {
    enable_foreign_keys(connection)?;
    let mut statement = connection.prepare("PRAGMA integrity_check")?;
    let integrity = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);
    if integrity.len() != 1 || integrity[0] != "ok" {
        bail!("SQLite integrity_check failed: {}", integrity.join("; "));
    }
    let mut statement = connection.prepare("PRAGMA foreign_key_check")?;
    let foreign_key_violations = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if !foreign_key_violations.is_empty() {
        bail!("SQLite foreign_key_check found violations: {foreign_key_violations:?}");
    }
    Ok(())
}

fn validate_ledger_fingerprint(connection: &Connection, expected_fingerprint: &str) -> Result<()> {
    let table_exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [LEDGER_TABLE],
        |row| row.get(0),
    )?;
    if !table_exists {
        bail!("Published migration has no migration ledger");
    }
    let mut statement = connection.prepare(
        "SELECT ordinal, migration_id, checksum, schema_fingerprint
         FROM vora_migration_ledger ORDER BY ordinal",
    )?;
    let entries = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if entries.is_empty() {
        bail!("Migration ledger is empty");
    }
    for (index, (ordinal, migration_id, checksum, fingerprint)) in entries.iter().enumerate() {
        if *ordinal != index as i64 + 1
            || migration_id.trim().is_empty()
            || checksum.len() != 64
            || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit())
            || fingerprint.as_deref() != Some(expected_fingerprint)
        {
            bail!("Migration ledger is inconsistent at step {}", index + 1);
        }
    }
    Ok(())
}

fn ensure_no_sqlite_sidecars(path: &Path) -> Result<()> {
    for suffix in ["-wal", "-shm", "-journal"] {
        let sidecar = PathBuf::from(format!("{}{suffix}", path.to_string_lossy()));
        if sidecar.exists() {
            bail!(
                "Database has an active SQLite sidecar {}; close all database connections first",
                sidecar.display()
            );
        }
    }
    Ok(())
}

fn source_location(source_path: &Path) -> Result<(&Path, &str)> {
    let parent = source_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .context("Database path must have a UTF-8 filename")?;
    Ok((parent, file_name))
}

fn sidecar_path(source_path: &Path, suffix: &str) -> Result<PathBuf> {
    let (parent, source_name) = source_location(source_path)?;
    Ok(parent.join(format!(".{source_name}.{suffix}")))
}

fn append_journal(path: &Path, event: &JournalEvent) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("Open migration journal {}", path.display()))?;
    serde_json::to_writer(&mut file, event)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

fn read_journal(path: &Path) -> Result<Vec<JournalEvent>> {
    let contents = fs::read_to_string(path)?;
    let mut events = Vec::new();
    let mut lines = contents.split_terminator('\n').peekable();
    while let Some(line) = lines.next() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str(&line) {
            Ok(event) => events.push(event),
            Err(_error) if lines.peek().is_none() && !contents.ends_with('\n') => break,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Parse migration journal {}", path.display()))
            }
        }
    }
    if events.is_empty() {
        bail!(
            "Migration journal contains no complete events: {}",
            path.display()
        );
    }
    Ok(events)
}

fn sync_file(path: &Path) -> Result<()> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .with_context(|| format!("Open file for durable sync {}", path.display()))?
        .sync_all()
        .with_context(|| format!("Sync file {}", path.display()))
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("Open database {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn copy_file_durably(source: &Path, destination: &Path) -> Result<String> {
    if destination.exists() {
        bail!(
            "Refusing to overwrite migration copy {}",
            destination.display()
        );
    }
    let source_hash = sha256_file(source)?;
    fs::copy(source, destination)
        .with_context(|| format!("Copy {} to {}", source.display(), destination.display()))?;
    sync_file(destination)?;
    let destination_hash = sha256_file(destination)?;
    if destination_hash != source_hash {
        bail!(
            "Copied database checksum mismatch for {}",
            destination.display()
        );
    }
    Ok(destination_hash)
}

fn create_partial_copy_for_failure(source: &Path, destination: &Path) -> Result<()> {
    let source_file = File::open(source)?;
    let source_length = source_file.metadata()?.len();
    let mut source_prefix = source_file.take((source_length / 2).max(1));
    let mut destination_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)?;
    std::io::copy(&mut source_prefix, &mut destination_file)?;
    destination_file.sync_all()?;
    Ok(())
}

fn preserve_unverified_backup(backup: &Path, operation_directory: &Path) -> Result<PathBuf> {
    let preserved_path =
        operation_directory.join(format!("backup-unverified-{}.sqlite", uuid::Uuid::now_v7()));
    fs::rename(backup, &preserved_path).with_context(|| {
        format!(
            "Preserve unverified migration backup {} at {}",
            backup.display(),
            preserved_path.display()
        )
    })?;
    Ok(preserved_path)
}

fn restore_from_backup_atomically(
    backup: &Path,
    source: &Path,
    operation_directory: &Path,
    expected_sha256: &str,
) -> Result<()> {
    validate_original_backup(backup, expected_sha256)?;
    let restore_path = operation_directory.join(format!("restore-{}.sqlite", uuid::Uuid::now_v7()));
    let backup_hash = copy_file_durably(backup, &restore_path)?;
    if backup_hash != expected_sha256 {
        bail!("Migration backup changed while being restored");
    }
    validate_original_backup(&restore_path, expected_sha256)?;
    atomic_replace(&restore_path, source).context("Atomically restore original database")?;
    sync_file(source)?;
    validate_original_backup(source, &backup_hash)
        .with_context(|| format!("Validate restored database at {}", source.display()))?;
    Ok(())
}

#[cfg(any(unix, windows))]
fn atomic_replace(staged: &Path, canonical: &Path) -> Result<()> {
    // Same-volume rename replaces the target atomically on Unix and on Windows
    // filesystems that support replace-existing rename semantics. Errors are
    // returned as-is; this function never emulates replacement with two renames.
    fs::rename(staged, canonical).with_context(|| {
        format!(
            "Atomic same-volume replacement from {} to {} failed",
            staged.display(),
            canonical.display()
        )
    })
}

#[cfg(not(any(unix, windows)))]
fn atomic_replace(_staged: &Path, _canonical: &Path) -> Result<()> {
    bail!("Atomic database replacement is unsupported on this operating system");
}

#[cfg(test)]
mod tests {
    use super::{
        current_schema_fingerprint, execute_with_failure, recognize_profile,
        recover_incomplete_operations, schema_fingerprint, validate_original_backup,
        ApprovedMigration, FailurePoint, JournalEvent, MigrationOutcome, MigrationPlan,
        RecoveryOutcome, SchemaProfile,
    };
    use rusqlite::Connection;
    use std::fs;
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    struct FixtureDirectory(PathBuf);

    impl FixtureDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join("vora-stage2-1-fixtures")
                .join(Uuid::now_v7().to_string());
            fs::create_dir_all(&path).expect("create isolated fixture directory");
            Self(path)
        }

        fn database(&self) -> PathBuf {
            self.0.join("workspace.sqlite")
        }
    }

    impl Drop for FixtureDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn current_fixture(directory: &FixtureDirectory) -> PathBuf {
        let path = directory.database();
        let connection = Connection::open(&path).expect("create current synthetic fixture");
        super::super::portable_db::initialize_workspace_schema(&connection)
            .expect("initialize current synthetic fixture");
        connection
            .pragma_update(None, "journal_mode", "DELETE")
            .expect("disable fixture WAL");
        drop(connection);
        path
    }

    fn plan_for(path: &Path, sql: &str) -> MigrationPlan {
        let connection = Connection::open(path).expect("open fixture to fingerprint plan");
        connection
            .execute_batch(sql)
            .expect("apply migration to fingerprint fixture");
        let fingerprint = schema_fingerprint(&connection).expect("fingerprint expected schema");
        drop(connection);
        let step = ApprovedMigration::new("synthetic-v1", sql).expect("create test migration");
        MigrationPlan::new(vec![step], fingerprint).expect("create test plan")
    }

    fn plan_with_expected_schema(
        expected_path: &Path,
        expected_sql: &str,
        migration_sql: &str,
    ) -> MigrationPlan {
        let connection = Connection::open(expected_path).expect("open expected fixture");
        connection
            .execute_batch(expected_sql)
            .expect("apply expected migration schema");
        let fingerprint = schema_fingerprint(&connection).expect("fingerprint expected schema");
        drop(connection);
        let step =
            ApprovedMigration::new("synthetic-v1", migration_sql).expect("create test migration");
        MigrationPlan::new(vec![step], fingerprint).expect("create test plan")
    }

    fn database_hash(path: &Path) -> String {
        format!(
            "{:x}",
            sha2::Sha256::digest(fs::read(path).expect("read database fixture"))
        )
    }

    fn table_exists(connection: &Connection, table: &str) -> bool {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [table],
                |row| row.get(0),
            )
            .expect("check synthetic fixture table")
    }

    use sha2::Digest;

    const SYNTHETIC_SQL: &str =
        "CREATE TABLE stage2_1_probe (id TEXT PRIMARY KEY, value TEXT NOT NULL);";

    #[test]
    fn recognizes_current_unknown_incomplete_and_unsupported_profiles() {
        let directory = FixtureDirectory::new();
        let current = current_fixture(&directory);
        assert!(matches!(
            recognize_profile(&current).expect("recognize current profile"),
            SchemaProfile::CurrentSupported { .. }
        ));
        let unknown = directory.0.join("unknown.sqlite");
        fs::copy(&current, &unknown).expect("clone current fixture");
        let connection = Connection::open(&unknown).expect("open unknown fixture");
        connection
            .execute_batch("CREATE TABLE synthetic_unknown (id INTEGER);")
            .expect("alter unknown fixture");
        drop(connection);
        assert!(matches!(
            recognize_profile(&unknown).expect("recognize unknown profile"),
            SchemaProfile::Unknown { .. }
        ));

        let incomplete = directory.0.join("incomplete.sqlite");
        let connection = Connection::open(&incomplete).expect("create incomplete fixture");
        connection
            .execute_batch("CREATE TABLE research_projects (id TEXT PRIMARY KEY);")
            .expect("create partial schema");
        drop(connection);
        assert!(matches!(
            recognize_profile(&incomplete).expect("recognize incomplete profile"),
            SchemaProfile::Incomplete { .. }
        ));

        let unsupported = directory.0.join("unsupported.sqlite");
        fs::copy(&current, &unsupported).expect("clone current fixture");
        let connection = Connection::open(&unsupported).expect("open unsupported fixture");
        connection
            .pragma_update(None, "user_version", 99)
            .expect("set synthetic unsupported marker");
        drop(connection);
        assert!(matches!(
            recognize_profile(&unsupported).expect("recognize unsupported profile"),
            SchemaProfile::Unsupported { user_version: 99 }
        ));

        let expected = directory.0.join("expected.sqlite");
        fs::copy(&current, &expected).expect("clone current fixture for plan");
        let plan = plan_for(&expected, SYNTHETIC_SQL);
        let unknown_before = database_hash(&unknown);
        let unsupported_before = database_hash(&unsupported);
        assert!(super::execute_staged_migrations(&unsupported, &plan).is_err());
        assert!(super::execute_staged_migrations(&unknown, &plan).is_err());
        assert_eq!(database_hash(&unknown), unknown_before);
        assert_eq!(database_hash(&unsupported), unsupported_before);
    }

    #[test]
    fn stages_and_publishes_only_after_validation_and_keeps_source_backup() {
        let directory = FixtureDirectory::new();
        let source = current_fixture(&directory);
        let expected = directory.0.join("expected.sqlite");
        fs::copy(&source, &expected).expect("clone expected fixture");
        let plan = plan_for(&expected, SYNTHETIC_SQL);
        let source_hash = database_hash(&source);

        let outcome = super::execute_staged_migrations(&source, &plan)
            .expect("publish validated synthetic migration");
        let backup_path = match outcome {
            MigrationOutcome::Published { backup_path } => backup_path,
            MigrationOutcome::AlreadyApplied => panic!("first execution must publish"),
        };
        assert_eq!(database_hash(&backup_path), source_hash);
        validate_original_backup(&backup_path, &source_hash)
            .expect("published backup is a valid current SQLite database");
        let migrated = Connection::open(&source).expect("open published fixture");
        let probe_exists: bool = migrated
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='stage2_1_probe')",
                [],
                |row| row.get(0),
            )
            .expect("check published migration");
        assert!(probe_exists);
        assert_eq!(
            schema_fingerprint(&migrated).expect("fingerprint published schema"),
            plan.expected_schema_fingerprint
        );
    }

    #[test]
    fn migration_failure_leaves_source_unchanged_and_recovery_abandons_stage() {
        let directory = FixtureDirectory::new();
        let source = current_fixture(&directory);
        let expected = directory.0.join("expected.sqlite");
        fs::copy(&source, &expected).expect("clone expected fixture");
        let plan = plan_with_expected_schema(
            &expected,
            "CREATE TABLE stage2_1_probe (id TEXT PRIMARY KEY);",
            "CREATE TABLE stage2_1_probe (id TEXT PRIMARY KEY); SELECT * FROM absent_table;",
        );
        let before = database_hash(&source);
        assert!(execute_with_failure(&source, &plan, FailurePoint::None).is_err());
        assert_eq!(database_hash(&source), before);
        let recovery = recover_incomplete_operations(&source).expect("recover failed migration");
        assert!(matches!(
            recovery.as_slice(),
            [RecoveryOutcome::Abandoned { .. }]
        ));
        assert_eq!(database_hash(&source), before);
    }

    #[test]
    fn injected_interruptions_before_staging_after_migration_and_validation_are_recoverable() {
        for failure in [
            FailurePoint::BeforeStaging,
            FailurePoint::AfterMigrations,
            FailurePoint::AfterValidation,
        ] {
            let directory = FixtureDirectory::new();
            let source = current_fixture(&directory);
            let expected = directory.0.join("expected.sqlite");
            fs::copy(&source, &expected).expect("clone expected fixture");
            let plan = plan_for(&expected, SYNTHETIC_SQL);
            let before = database_hash(&source);
            assert!(execute_with_failure(&source, &plan, failure).is_err());
            assert_eq!(database_hash(&source), before);
            if failure == FailurePoint::AfterMigrations {
                let operation_directory = directory
                    .0
                    .read_dir()
                    .expect("read fixture directory")
                    .filter_map(std::result::Result::ok)
                    .map(|entry| entry.path())
                    .find(|path| {
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.contains(".migration-"))
                    })
                    .expect("staging operation directory exists");
                let staged = Connection::open(operation_directory.join("staging.sqlite"))
                    .expect("open isolated staging copy");
                let staged_probe_exists: bool = staged
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='stage2_1_probe')",
                        [],
                        |row| row.get(0),
                    )
                    .expect("check isolated migration");
                assert!(staged_probe_exists);
                let original = Connection::open(&source).expect("open unchanged source");
                let source_probe_exists: bool = original
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='stage2_1_probe')",
                        [],
                        |row| row.get(0),
                    )
                    .expect("check source schema");
                assert!(!source_probe_exists);
            }
            let recovery =
                recover_incomplete_operations(&source).expect("recover interrupted migration");
            if failure == FailurePoint::AfterValidation {
                assert!(matches!(
                    recovery.as_slice(),
                    [RecoveryOutcome::Published { .. }]
                ));
            } else {
                assert!(matches!(
                    recovery.as_slice(),
                    [RecoveryOutcome::Abandoned { .. }]
                ));
                assert_eq!(database_hash(&source), before);
            }
        }
    }

    #[test]
    fn atomic_publication_recovery_covers_each_interruption_and_is_repeatable() {
        for failure in [
            FailurePoint::BeforePublication,
            FailurePoint::DuringPublication,
            FailurePoint::AfterPublicationBeforeJournal,
        ] {
            let directory = FixtureDirectory::new();
            let source = current_fixture(&directory);
            let expected = directory.0.join("expected.sqlite");
            fs::copy(&source, &expected).expect("clone expected fixture");
            let plan = plan_for(&expected, SYNTHETIC_SQL);
            let original_hash = database_hash(&source);

            assert!(execute_with_failure(&source, &plan, failure).is_err());
            assert!(
                source.is_file(),
                "canonical path disappeared at {failure:?}"
            );
            if failure == FailurePoint::AfterPublicationBeforeJournal {
                let published = Connection::open(&source).expect("open atomically published DB");
                assert!(table_exists(&published, "stage2_1_probe"));
            } else {
                assert_eq!(database_hash(&source), original_hash);
                let unchanged = Connection::open(&source).expect("open original canonical DB");
                assert!(!table_exists(&unchanged, "stage2_1_probe"));
            }

            let recovered =
                recover_incomplete_operations(&source).expect("recover interrupted publication");
            assert!(matches!(
                recovered.as_slice(),
                [RecoveryOutcome::Published { .. }]
            ));
            assert!(source.is_file());
            let published = Connection::open(&source).expect("open recovered canonical DB");
            assert!(table_exists(&published, "stage2_1_probe"));

            let [RecoveryOutcome::Published {
                operation_directory,
            }] = recovered.as_slice()
            else {
                panic!("recovery should report one published operation");
            };
            let backup = operation_directory.join("backup.sqlite");
            assert_eq!(database_hash(&backup), original_hash);
            assert!(recover_incomplete_operations(&source)
                .expect("repeat recovery")
                .is_empty());
            assert_eq!(database_hash(&backup), original_hash);
        }
    }

    #[test]
    fn backup_copy_interruptions_preserve_artifacts_and_recover_repeatably() {
        for failure in [
            FailurePoint::DuringBackupCopy,
            FailurePoint::AfterBackupCopyBeforeJournal,
        ] {
            let directory = FixtureDirectory::new();
            let source = current_fixture(&directory);
            let expected = directory.0.join("expected.sqlite");
            fs::copy(&source, &expected).expect("clone expected fixture");
            let plan = plan_for(&expected, SYNTHETIC_SQL);
            let original_hash = database_hash(&source);

            assert!(execute_with_failure(&source, &plan, failure).is_err());
            assert_eq!(database_hash(&source), original_hash);

            let operation_directory = directory
                .0
                .read_dir()
                .expect("read fixture directory")
                .filter_map(std::result::Result::ok)
                .map(|entry| entry.path())
                .find(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with(".workspace.sqlite.migration-"))
                })
                .expect("interrupted operation directory exists");
            let backup = operation_directory.join("backup.sqlite");
            assert!(
                backup.is_file(),
                "copy interruption leaves a backup artifact"
            );

            let recovered = recover_incomplete_operations(&source)
                .expect("recover interrupted backup preparation");
            assert!(matches!(
                recovered.as_slice(),
                [RecoveryOutcome::Published { .. }]
            ));

            let retained_backup = operation_directory.join("backup.sqlite");
            validate_original_backup(&retained_backup, &original_hash)
                .expect("recovery retains a valid original SQLite backup");
            assert_eq!(database_hash(&retained_backup), original_hash);
            if failure == FailurePoint::DuringBackupCopy {
                let preserved = operation_directory
                    .read_dir()
                    .expect("read interrupted operation artifacts")
                    .filter_map(std::result::Result::ok)
                    .map(|entry| entry.path())
                    .find(|path| {
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.starts_with("backup-unverified-"))
                    })
                    .expect("partial copy is preserved instead of deleted");
                assert!(preserved.is_file());
            }
            assert!(recover_incomplete_operations(&source)
                .expect("repeat recovery after backup interruption")
                .is_empty());
            validate_original_backup(&retained_backup, &original_hash)
                .expect("repeated recovery preserves the validated backup");
        }
    }

    #[test]
    fn hash_matching_invalid_sqlite_backup_fails_closed_and_is_preserved() {
        let directory = FixtureDirectory::new();
        let source = current_fixture(&directory);
        let source_hash = database_hash(&source);
        let operation_directory = directory.0.join(".workspace.sqlite.migration-invalid");
        fs::create_dir(&operation_directory).expect("create isolated recovery directory");
        let backup = operation_directory.join("backup.sqlite");
        fs::copy(&source, &backup).expect("clone isolated current fixture");
        let invalid = Connection::open(&backup).expect("open isolated backup fixture");
        invalid
            .execute_batch(
                "CREATE TABLE backup_integrity_probe (value INTEGER CHECK(value > 0));
                 PRAGMA ignore_check_constraints = ON;
                 INSERT INTO backup_integrity_probe VALUES (-1);",
            )
            .expect("create synthetic SQLite integrity failure");
        drop(invalid);
        let backup_hash = database_hash(&backup);
        assert_ne!(backup_hash, source_hash);

        let journal = operation_directory.join("journal.log");
        super::append_journal(&journal, &JournalEvent::Started).expect("journal start");
        super::append_journal(
            &journal,
            &JournalEvent::SourceObserved {
                original_sha256: backup_hash.clone(),
            },
        )
        .expect("journal observed source hash");
        super::append_journal(
            &journal,
            &JournalEvent::Validated {
                expected_fingerprint: current_schema_fingerprint()
                    .expect("current synthetic schema fingerprint"),
            },
        )
        .expect("journal synthetic validation");
        super::append_journal(
            &journal,
            &JournalEvent::BackupPrepared {
                original_sha256: backup_hash.clone(),
            },
        )
        .expect("journal synthetic backup hash");

        let source_before = database_hash(&source);
        assert!(recover_incomplete_operations(&source).is_err());
        assert_eq!(database_hash(&source), source_before);
        assert_eq!(database_hash(&backup), backup_hash);
        assert!(super::restore_from_backup_atomically(
            &backup,
            &source,
            &operation_directory,
            &backup_hash,
        )
        .is_err());
        assert_eq!(database_hash(&source), source_before);
        assert_eq!(database_hash(&backup), backup_hash);
        assert!(journal.is_file());
        assert!(recover_incomplete_operations(&source).is_err());
        assert_eq!(database_hash(&source), source_before);
        assert_eq!(database_hash(&backup), backup_hash);
    }

    #[test]
    fn completed_plan_is_idempotent() {
        let directory = FixtureDirectory::new();
        let source = current_fixture(&directory);
        let expected = directory.0.join("expected.sqlite");
        fs::copy(&source, &expected).expect("clone expected fixture");
        let plan = plan_for(&expected, SYNTHETIC_SQL);
        super::execute_staged_migrations(&source, &plan).expect("first execution");
        let before = database_hash(&source);
        assert_eq!(
            super::execute_staged_migrations(&source, &plan).expect("repeat execution"),
            MigrationOutcome::AlreadyApplied
        );
        assert_eq!(database_hash(&source), before);
    }

    #[test]
    fn concurrent_coordinator_call_is_rejected_without_changing_source() {
        let directory = FixtureDirectory::new();
        let source = current_fixture(&directory);
        let expected = directory.0.join("expected.sqlite");
        fs::copy(&source, &expected).expect("clone expected fixture");
        let plan = plan_for(&expected, SYNTHETIC_SQL);
        let before = database_hash(&source);
        let _lock = super::CoordinatorLock::acquire(&source).expect("hold coordinator lock");

        assert!(super::execute_staged_migrations(&source, &plan).is_err());
        assert_eq!(database_hash(&source), before);
    }

    #[test]
    fn integrity_and_foreign_key_gates_reject_invalid_synthetic_databases() {
        let directory = FixtureDirectory::new();
        let source = current_fixture(&directory);
        let bad_integrity = directory.0.join("bad-integrity.sqlite");
        fs::copy(&source, &bad_integrity).expect("clone fixture");
        let connection = Connection::open(&bad_integrity).expect("open integrity fixture");
        connection
            .execute_batch(
                "CREATE TABLE integrity_probe (value INTEGER CHECK(value > 0));
                 PRAGMA ignore_check_constraints = ON;
                 INSERT INTO integrity_probe VALUES (-1);",
            )
            .expect("create logical integrity violation");
        drop(connection);
        let connection =
            Connection::open(&bad_integrity).expect("reopen integrity fixture for validation");
        let expected = current_schema_fingerprint().expect("get current fingerprint");
        assert!(super::validate_database(&connection, &expected)
            .expect_err("integrity gate must fail")
            .to_string()
            .contains("integrity_check"));
        drop(connection);

        let bad_foreign_key = directory.0.join("bad-fk.sqlite");
        fs::copy(&source, &bad_foreign_key).expect("clone fixture");
        let connection = Connection::open(&bad_foreign_key).expect("open FK fixture");
        connection
            .execute_batch(
                "PRAGMA foreign_keys = OFF;
                 CREATE TABLE fk_parent (id INTEGER PRIMARY KEY);
                 CREATE TABLE fk_child (
                     parent_id INTEGER REFERENCES fk_parent(id)
                 );
                 INSERT INTO fk_child VALUES (404);",
            )
            .expect("create synthetic foreign-key violation");
        assert_eq!(
            connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .expect("check initial foreign-key setting"),
            0
        );
        let enforced = super::open_staging(&bad_foreign_key)
            .expect("open staging connection with foreign keys enabled");
        assert_eq!(
            enforced
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .expect("check staging foreign-key enforcement"),
            1
        );
        assert!(enforced
            .execute("INSERT INTO fk_child VALUES (405)", [])
            .is_err());
        assert!(super::validate_database(&connection, &expected)
            .expect_err("foreign-key gate must fail")
            .to_string()
            .contains("foreign_key_check"));
    }

    #[test]
    fn wrong_expected_fingerprint_prevents_publication() {
        let directory = FixtureDirectory::new();
        let source = current_fixture(&directory);
        let step =
            ApprovedMigration::new("synthetic-v1", SYNTHETIC_SQL).expect("create test migration");
        let plan = MigrationPlan::new(vec![step], "not-the-schema-fingerprint")
            .expect("create mismatched plan");
        let before = database_hash(&source);
        assert!(super::execute_staged_migrations(&source, &plan).is_err());
        assert_eq!(database_hash(&source), before);
        assert!(source.is_file());
    }
}
