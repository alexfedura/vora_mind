use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const SOURCE_USER_VERSION: i64 = 1;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ColumnInventory {
    name: String,
    declared_type: String,
    not_null: bool,
    default_value: Option<String>,
    primary_key_position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ForeignKeyInventory {
    referenced_table: String,
    from_column: String,
    to_column: String,
    on_update: String,
    on_delete: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct IndexInventory {
    name: String,
    unique: bool,
    origin: String,
    partial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct TableInventory {
    name: String,
    columns: Vec<ColumnInventory>,
    foreign_keys: Vec<ForeignKeyInventory>,
    indexes: Vec<IndexInventory>,
    row_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct MigrationMarker {
    table: String,
    version: i64,
    migration_id: Option<String>,
    checksum: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct OwnershipRecord {
    table: String,
    fields: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
enum ProfileClassification {
    RecognizedCurrentSchema,
    Incomplete,
    Unknown,
    UnsupportedVersionMarker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ReadOnlyInventory {
    schema_fingerprint: String,
    user_version: i64,
    application_id: i64,
    tables: Vec<TableInventory>,
    migration_markers: Vec<MigrationMarker>,
    ownership_records: Vec<OwnershipRecord>,
    classification: ProfileClassification,
}

struct FixtureDirectory {
    path: PathBuf,
}

impl FixtureDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir()
            .join("vora-stage2-0-fixtures")
            .join(Uuid::now_v7().to_string());
        fs::create_dir_all(&path).expect("create isolated synthetic fixture directory");
        Self { path }
    }

    fn database(&self, name: &str) -> PathBuf {
        self.path.join(format!("{name}.sqlite"))
    }
}

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct FixtureSet {
    current: PathBuf,
    research_legacy: PathBuf,
    model_ownership: PathBuf,
    shared_source: PathBuf,
    partial: PathBuf,
    unknown: PathBuf,
}

impl FixtureSet {
    fn generate(directory: &FixtureDirectory) -> Self {
        let current = directory.database("A-current-fresh");
        let conn = Connection::open(&current).expect("create current fresh-schema fixture");
        super::initialize_workspace_schema(&conn)
            .expect("initialize isolated current fresh-schema fixture");
        conn.pragma_update(None, "journal_mode", "DELETE")
            .expect("close current fixture without WAL sidecars");
        drop(conn);

        let research_legacy = directory.database("B-legacy-research");
        clone_fixture(&current, &research_legacy);
        add_legacy_research_records(&research_legacy);

        let model_ownership = directory.database("C-model-ownership");
        clone_fixture(&current, &model_ownership);
        add_model_ownership_records(&model_ownership);

        let shared_source = directory.database("D-shared-source");
        clone_fixture(&current, &shared_source);
        add_legacy_source_records(&shared_source);

        let partial = directory.database("E-partial-initialization");
        {
            let conn = Connection::open(&partial).expect("create partial synthetic fixture");
            conn.execute_batch(
                "CREATE TABLE research_projects (id TEXT PRIMARY KEY);
                 CREATE TABLE research_questions (
                     id TEXT PRIMARY KEY,
                     project_id TEXT
                 );",
            )
            .expect("create deliberately incomplete schema");
            conn.pragma_update(None, "journal_mode", "DELETE")
                .expect("close partial fixture without WAL sidecars");
        }

        let unknown = directory.database("F-unknown-version");
        clone_fixture(&current, &unknown);
        {
            let conn = Connection::open(&unknown).expect("open unknown synthetic fixture");
            conn.pragma_update(None, "user_version", 99)
                .expect("write unsupported synthetic schema marker");
            conn.pragma_update(None, "journal_mode", "DELETE")
                .expect("close unknown fixture without WAL sidecars");
        }

        Self {
            current,
            research_legacy,
            model_ownership,
            shared_source,
            partial,
            unknown,
        }
    }
}

fn clone_fixture(source: &Path, destination: &Path) {
    fs::copy(source, destination).expect("clone isolated synthetic fixture");
}

fn open_fixture_for_generation(path: &Path) -> Connection {
    let conn = Connection::open(path).expect("open synthetic fixture for generation");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable fixture foreign keys");
    conn
}

fn finish_fixture_generation(conn: Connection) {
    conn.pragma_update(None, "journal_mode", "DELETE")
        .expect("close generated fixture without WAL sidecars");
    drop(conn);
}

fn add_legacy_research_records(path: &Path) {
    let conn = open_fixture_for_generation(path);
    conn.execute_batch(
        "INSERT INTO questions (id, title, description, created_at, updated_at)
         VALUES
            ('legacy-question-unassigned', 'Synthetic unassigned question', 'Original question', 10, 11),
            ('legacy-question-unknown', 'Synthetic unknown question', 'Original question', 12, 13);

         INSERT INTO hypotheses (
             id, inquiry_id, statement_h1, statement_h0, independent_var,
             dependent_var, metric, success_threshold, created_at, updated_at
         ) VALUES
            ('legacy-hypothesis-unassigned', 'missing-inquiry-a', 'H1 A', 'H0 A',
             'legacy variable', 'legacy outcome', 'legacy metric', 'legacy threshold', 20, 21),
            ('legacy-hypothesis-unknown', 'missing-inquiry-b', 'H1 B', 'H0 B',
             'legacy variable', 'legacy outcome', 'legacy metric', 'legacy threshold', 22, 23);

         INSERT INTO research_questions (
             id, project_id, lifecycle_status, ownership_state, migration_note, mapped_at_us
         ) VALUES
            ('legacy-question-unassigned', NULL, 'legacy_unassessed', 'unassigned',
             'Synthetic only; retained original question row.', 30),
            ('legacy-question-unknown', NULL, 'legacy_unassessed', 'unknown',
             'Synthetic only; ownership is unknown.', 31);

         INSERT INTO research_hypotheses (
             id, project_id, legacy_hypothesis_id, ownership_state,
             lifecycle_status, created_at_us
         ) VALUES
            ('typed-legacy-hypothesis-unassigned', NULL, 'legacy-hypothesis-unassigned',
             'unassigned', 'legacy_untyped', 40),
            ('typed-legacy-hypothesis-unknown', NULL, 'legacy-hypothesis-unknown',
             'unknown', 'legacy_untyped', 41);",
    )
    .expect("insert synthetic unassigned and unknown legacy research records");
    finish_fixture_generation(conn);
}

fn add_model_ownership_records(path: &Path) {
    let conn = open_fixture_for_generation(path);
    conn.execute_batch(
        "INSERT INTO research_projects (
             id, title, aim, lifecycle_status, created_at_us, updated_at_us
         ) VALUES ('synthetic-project-model', 'Synthetic model project', '',
                   'active', 50, 50);

         INSERT INTO scientific_models (
             id, display_name, description, origin_project_id, created_at_us
         ) VALUES ('synthetic-model-null-origin', 'Synthetic nullable-origin identity',
                   'Synthetic only.', NULL, 51);

         INSERT INTO model_versions (
             id, model_id, version_no, created_in_project_id, change_class,
             change_rationale, expected_implications, human_specification,
             machine_specification_json, scope, dependencies_json, environment_json,
             content_sha256, created_at_us
         ) VALUES (
             'synthetic-model-version-project-local', 'synthetic-model-null-origin', 1,
             'synthetic-project-model', 'refinement', 'Synthetic fixture version.',
             'No runtime implication.', 'Synthetic fixture specification.',
             '{}', 'Synthetic fixture scope.', '[]', '{}',
             'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 52
         );",
    )
    .expect("insert synthetic project-local ModelVersion");
    finish_fixture_generation(conn);
}

fn add_legacy_source_records(path: &Path) {
    let conn = open_fixture_for_generation(path);
    conn.execute_batch(
        "INSERT INTO sources (id, kind, title, created_at, updated_at)
         VALUES ('synthetic-shared-source', 'doc', 'Synthetic shared source', 60, 60);

         INSERT INTO segments (id, source_id, label, created_at, updated_at)
         VALUES ('synthetic-source-segment', 'synthetic-shared-source',
                 'Synthetic segment', 61, 61);

         INSERT INTO extracts (id, segment_id, transform_kind, content, created_at, updated_at)
         VALUES ('synthetic-source-extract', 'synthetic-source-segment',
                 'quoted', 'Synthetic extract content', 62, 62);

         INSERT INTO links (id, extract_id, target_kind, target_id, created_at, updated_at)
         VALUES ('synthetic-source-link', 'synthetic-source-extract',
                 'external', 'synthetic-external-target', 63, 63);

         INSERT INTO source_revisions (
             id, entity_kind, entity_id, revision_type, after_json, created_at
         ) VALUES (
             'synthetic-source-revision', 'source', 'synthetic-shared-source',
             'create', 'synthetic-source-provenance', 64
         );",
    )
    .expect("insert synthetic shared legacy Source records");
    finish_fixture_generation(conn);
}

fn open_inventory_read_only(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.execute_batch("PRAGMA query_only = ON;")?;
    let query_only: i64 = conn.query_row("PRAGMA query_only", [], |row| row.get(0))?;
    if query_only != 1 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(conn)
}

fn inspect_read_only(
    path: &Path,
    current_schema_fingerprint: &str,
) -> rusqlite::Result<ReadOnlyInventory> {
    let conn = open_inventory_read_only(path)?;
    let user_version = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let application_id = conn.pragma_query_value(None, "application_id", |row| row.get(0))?;

    let mut statement = conn.prepare(
        "SELECT name FROM sqlite_master
         WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
         ORDER BY name",
    )?;
    let table_names = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    let tables = table_names
        .iter()
        .map(|name| inspect_table(&conn, name))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let migration_markers = inspect_migration_markers(&conn, &table_names)?;
    let ownership_records = inspect_ownership_records(&conn, &tables)?;
    let schema_fingerprint = schema_fingerprint(&conn)?;
    let classification = classify_profile(
        &table_names,
        user_version,
        current_schema_fingerprint,
        &schema_fingerprint,
    );

    Ok(ReadOnlyInventory {
        schema_fingerprint,
        user_version,
        application_id,
        tables,
        migration_markers,
        ownership_records,
        classification,
    })
}

fn inspect_table(conn: &Connection, name: &str) -> rusqlite::Result<TableInventory> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({})", quote_identifier(name)))?;
    let columns = statement
        .query_map([], |row| {
            Ok(ColumnInventory {
                name: row.get(1)?,
                declared_type: row.get(2)?,
                not_null: row.get::<_, i64>(3)? != 0,
                default_value: row.get(4)?,
                primary_key_position: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    let mut statement = conn.prepare(&format!(
        "PRAGMA foreign_key_list({})",
        quote_identifier(name)
    ))?;
    let foreign_keys = statement
        .query_map([], |row| {
            Ok(ForeignKeyInventory {
                referenced_table: row.get(2)?,
                from_column: row.get(3)?,
                to_column: row.get(4)?,
                on_update: row.get(5)?,
                on_delete: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    let mut statement = conn.prepare(&format!("PRAGMA index_list({})", quote_identifier(name)))?;
    let indexes = statement
        .query_map([], |row| {
            Ok(IndexInventory {
                name: row.get(1)?,
                unique: row.get::<_, i64>(2)? != 0,
                origin: row.get(3)?,
                partial: row.get::<_, i64>(4)? != 0,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let row_count = conn.query_row(
        &format!("SELECT COUNT(*) FROM {}", quote_identifier(name)),
        [],
        |row| row.get(0),
    )?;

    Ok(TableInventory {
        name: name.to_string(),
        columns,
        foreign_keys,
        indexes,
        row_count,
    })
}

fn inspect_migration_markers(
    conn: &Connection,
    table_names: &[String],
) -> rusqlite::Result<Vec<MigrationMarker>> {
    let mut markers = Vec::new();
    for table in table_names
        .iter()
        .filter(|name| *name == "schema_migrations" || name.ends_with("_schema_migrations"))
    {
        let columns = table_columns(conn, table)?;
        if !columns.iter().any(|column| column == "version") {
            continue;
        }
        let migration_id = columns
            .iter()
            .any(|column| column == "migration_id")
            .then(|| quote_identifier("migration_id"));
        let checksum = columns
            .iter()
            .any(|column| column == "checksum")
            .then(|| quote_identifier("checksum"));
        let migration_id_expr = migration_id.unwrap_or_else(|| "NULL".to_string());
        let checksum_expr = checksum.unwrap_or_else(|| "NULL".to_string());
        let sql = format!(
            "SELECT version, {migration_id_expr}, {checksum_expr}
             FROM {} ORDER BY version",
            quote_identifier(table)
        );
        let mut statement = conn.prepare(&sql)?;
        markers.extend(
            statement
                .query_map([], |row| {
                    Ok(MigrationMarker {
                        table: table.to_string(),
                        version: row.get(0)?,
                        migration_id: row.get(1)?,
                        checksum: row.get(2)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?,
        );
    }
    Ok(markers)
}

fn inspect_ownership_records(
    conn: &Connection,
    tables: &[TableInventory],
) -> rusqlite::Result<Vec<OwnershipRecord>> {
    let projections: &[(&str, &[&str])] = &[
        ("questions", &["id", "title", "promoted_to_hypothesis_id"]),
        (
            "hypotheses",
            &[
                "id",
                "inquiry_id",
                "parent_hypothesis_id",
                "refined_from_id",
            ],
        ),
        (
            "research_questions",
            &[
                "id",
                "project_id",
                "ownership_state",
                "lifecycle_status",
                "migration_note",
            ],
        ),
        (
            "research_hypotheses",
            &[
                "id",
                "project_id",
                "ownership_state",
                "legacy_hypothesis_id",
                "lifecycle_status",
            ],
        ),
        ("scientific_models", &["id", "origin_project_id"]),
        (
            "model_versions",
            &["id", "model_id", "created_in_project_id"],
        ),
        ("sources", &["id", "kind", "title", "project_id"]),
        ("segments", &["id", "source_id", "project_id"]),
        ("extracts", &["id", "segment_id", "project_id"]),
        (
            "links",
            &["id", "extract_id", "target_kind", "target_id", "project_id"],
        ),
    ];

    let mut records = Vec::new();
    for (table, desired_fields) in projections {
        let Some(info) = tables.iter().find(|info| info.name == *table) else {
            continue;
        };
        let fields = desired_fields
            .iter()
            .filter(|field| info.columns.iter().any(|column| column.name == **field))
            .copied()
            .collect::<Vec<_>>();
        if fields.is_empty() || !fields.iter().any(|field| *field == "id") {
            continue;
        }
        let expressions = fields
            .iter()
            .map(|field| format!("CAST({} AS TEXT)", quote_identifier(field)))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT {expressions} FROM {} ORDER BY id",
            quote_identifier(table)
        );
        let mut statement = conn.prepare(&sql)?;
        let rows = statement.query_map([], |row| {
            let mut values = BTreeMap::new();
            for (index, field) in fields.iter().enumerate() {
                values.insert(field.to_string(), row.get(index)?);
            }
            Ok(OwnershipRecord {
                table: table.to_string(),
                fields: values,
            })
        })?;
        records.extend(rows.collect::<rusqlite::Result<Vec<_>>>()?);
    }
    Ok(records)
}

fn table_columns(conn: &Connection, table: &str) -> rusqlite::Result<Vec<String>> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({})", quote_identifier(table)))?;
    let columns = statement
        .query_map([], |row| row.get(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(columns)
}

fn schema_fingerprint(conn: &Connection) -> rusqlite::Result<String> {
    let mut statement = conn.prepare(
        "SELECT type, name, tbl_name, sql FROM sqlite_master
         WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%'
         ORDER BY type, name, tbl_name",
    )?;
    let objects = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let serialized = serde_json::to_vec(&objects)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    Ok(format!("{:x}", Sha256::digest(serialized)))
}

fn classify_profile(
    tables: &[String],
    user_version: i64,
    current_schema_fingerprint: &str,
    observed_schema_fingerprint: &str,
) -> ProfileClassification {
    if user_version > SOURCE_USER_VERSION {
        return ProfileClassification::UnsupportedVersionMarker;
    }
    if current_schema_fingerprint == observed_schema_fingerprint {
        return ProfileClassification::RecognizedCurrentSchema;
    }
    if REQUIRED_CURRENT_TABLES
        .iter()
        .any(|required| !tables.iter().any(|table| table == required))
    {
        return ProfileClassification::Incomplete;
    }
    ProfileClassification::Unknown
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn file_hash(path: &Path) -> String {
    format!(
        "{:x}",
        Sha256::digest(fs::read(path).expect("read fixture bytes for hash"))
    )
}

fn directory_entries(path: &Path) -> Vec<String> {
    let mut entries = fs::read_dir(path)
        .expect("read isolated fixture directory")
        .map(|entry| {
            entry
                .expect("read fixture directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

fn stable_fixture_summary(
    inventory: &ReadOnlyInventory,
) -> (String, Vec<MigrationMarker>, Vec<OwnershipRecord>) {
    (
        inventory.schema_fingerprint.clone(),
        inventory.migration_markers.clone(),
        inventory.ownership_records.clone(),
    )
}

fn current_fingerprint(path: &Path) -> String {
    let conn = open_inventory_read_only(path).expect("open fixture read-only for fingerprint");
    schema_fingerprint(&conn).expect("fingerprint fixture schema")
}

#[test]
fn synthetic_fixture_generation_and_inventory_are_deterministic_and_read_only() {
    let first_directory = FixtureDirectory::new();
    let first = FixtureSet::generate(&first_directory);
    let second_directory = FixtureDirectory::new();
    let second = FixtureSet::generate(&second_directory);

    let current_schema_fingerprint = current_fingerprint(&first.current);
    let first_baseline = inspect_read_only(&first.current, &current_schema_fingerprint)
        .expect("inspect fresh fixture");
    let first_summary = stable_fixture_summary(&first_baseline);
    let second_fingerprint = current_fingerprint(&second.current);
    let second_baseline = inspect_read_only(&second.current, &second_fingerprint)
        .expect("inspect second fresh fixture");
    assert_eq!(first_summary, stable_fixture_summary(&second_baseline));
    assert_eq!(
        first_baseline.classification,
        ProfileClassification::RecognizedCurrentSchema
    );

    let source_hash = file_hash(&first.research_legacy);
    let directory_before = directory_entries(&first_directory.path);
    let first_inspection = inspect_read_only(&first.research_legacy, &current_schema_fingerprint)
        .expect("read-only inspect synthetic research fixture");
    let second_inspection = inspect_read_only(&first.research_legacy, &current_schema_fingerprint)
        .expect("repeat read-only inspection");
    assert_eq!(first_inspection, second_inspection);
    assert_eq!(source_hash, file_hash(&first.research_legacy));
    assert_eq!(directory_before, directory_entries(&first_directory.path));
    assert_eq!(
        first_inspection.classification,
        ProfileClassification::RecognizedCurrentSchema
    );

    let read_only = open_inventory_read_only(&first.research_legacy)
        .expect("open fixture read-only for write denial check");
    assert!(read_only
        .execute("CREATE TABLE inventory_must_not_create (id TEXT)", [])
        .is_err());
}

#[test]
fn synthetic_legacy_research_records_remain_unassigned_or_unknown() {
    let directory = FixtureDirectory::new();
    let fixtures = FixtureSet::generate(&directory);
    let fingerprint = current_fingerprint(&fixtures.current);
    let inventory = inspect_read_only(&fixtures.research_legacy, &fingerprint)
        .expect("inspect legacy Research fixture");

    for id in ["legacy-question-unassigned", "legacy-question-unknown"] {
        assert!(inventory.ownership_records.iter().any(|record| {
            record.table == "questions"
                && record.fields.get("id").and_then(Option::as_deref) == Some(id)
        }));
    }
    for (id, expected_state) in [
        ("legacy-question-unassigned", "unassigned"),
        ("legacy-question-unknown", "unknown"),
    ] {
        assert!(inventory.ownership_records.iter().any(|record| {
            record.table == "research_questions"
                && record.fields.get("id").and_then(Option::as_deref) == Some(id)
                && record.fields.get("project_id").is_some_and(Option::is_none)
                && record
                    .fields
                    .get("ownership_state")
                    .and_then(Option::as_deref)
                    == Some(expected_state)
        }));
    }
    for (id, expected_state) in [
        ("legacy-hypothesis-unassigned", "unassigned"),
        ("legacy-hypothesis-unknown", "unknown"),
    ] {
        assert!(inventory.ownership_records.iter().any(|record| {
            record.table == "research_hypotheses"
                && record
                    .fields
                    .get("legacy_hypothesis_id")
                    .and_then(Option::as_deref)
                    == Some(id)
                && record.fields.get("project_id").is_some_and(Option::is_none)
                && record
                    .fields
                    .get("ownership_state")
                    .and_then(Option::as_deref)
                    == Some(expected_state)
        }));
    }
}

#[test]
fn shared_legacy_source_has_no_inferred_project_or_fabricated_version() {
    let directory = FixtureDirectory::new();
    let fixtures = FixtureSet::generate(&directory);
    let fingerprint = current_fingerprint(&fixtures.current);
    let inventory = inspect_read_only(&fixtures.shared_source, &fingerprint)
        .expect("inspect shared Source fixture");

    let source = inventory
        .ownership_records
        .iter()
        .find(|record| {
            record.table == "sources"
                && record.fields.get("id").and_then(Option::as_deref)
                    == Some("synthetic-shared-source")
        })
        .expect("synthetic source row remains visible");
    assert!(!source.fields.contains_key("project_id"));
    assert_eq!(
        inventory
            .tables
            .iter()
            .find(|table| table.name == "source_versions")
            .expect("current schema includes SourceVersion table")
            .row_count,
        0,
        "synthetic legacy Source profile must not manufacture SourceVersion rows"
    );
    assert!(inventory
        .ownership_records
        .iter()
        .any(|record| record.table == "segments"));
    assert!(inventory
        .ownership_records
        .iter()
        .any(|record| record.table == "extracts"));
    assert!(inventory
        .ownership_records
        .iter()
        .any(|record| record.table == "links"));
    assert!(inventory
        .migration_markers
        .iter()
        .any(|marker| marker.table == "schema_migrations"));
}

#[test]
fn model_fixture_keeps_nullable_identity_origin_and_project_local_version() {
    let directory = FixtureDirectory::new();
    let fixtures = FixtureSet::generate(&directory);
    let fingerprint = current_fingerprint(&fixtures.current);
    let inventory = inspect_read_only(&fixtures.model_ownership, &fingerprint)
        .expect("inspect Model ownership fixture");

    let model = inventory
        .ownership_records
        .iter()
        .find(|record| {
            record.table == "scientific_models"
                && record.fields.get("id").and_then(Option::as_deref)
                    == Some("synthetic-model-null-origin")
        })
        .expect("nullable-origin identity remains visible");
    assert!(model
        .fields
        .get("origin_project_id")
        .is_some_and(Option::is_none));
    let version = inventory
        .ownership_records
        .iter()
        .find(|record| {
            record.table == "model_versions"
                && record.fields.get("id").and_then(Option::as_deref)
                    == Some("synthetic-model-version-project-local")
        })
        .expect("project-local ModelVersion remains visible");
    assert_eq!(
        version
            .fields
            .get("created_in_project_id")
            .and_then(Option::as_deref),
        Some("synthetic-project-model")
    );
    assert!(inventory.tables.iter().any(|table| {
        table.name == "model_versions"
            && table
                .columns
                .iter()
                .any(|column| column.name == "created_in_project_id" && column.not_null)
    }));
}

#[test]
fn partial_and_unknown_schema_profiles_are_not_treated_as_supported() {
    let directory = FixtureDirectory::new();
    let fixtures = FixtureSet::generate(&directory);
    let fingerprint = current_fingerprint(&fixtures.current);
    for (path, expected) in [
        (&fixtures.partial, ProfileClassification::Incomplete),
        (
            &fixtures.unknown,
            ProfileClassification::UnsupportedVersionMarker,
        ),
    ] {
        let before = file_hash(path);
        let inventory = inspect_read_only(path, &fingerprint)
            .expect("inventory unsupported synthetic profile without repair");
        assert_eq!(inventory.classification, expected);
        assert_eq!(before, file_hash(path));
    }
}

#[test]
fn inventory_records_schema_constraints_and_migration_markers_without_initializing() {
    let directory = FixtureDirectory::new();
    let fixtures = FixtureSet::generate(&directory);
    let fingerprint = current_fingerprint(&fixtures.current);
    let inventory = inspect_read_only(&fixtures.current, &fingerprint)
        .expect("inventory current fixture read-only");

    assert_eq!(
        inventory.classification,
        ProfileClassification::RecognizedCurrentSchema
    );
    assert!(inventory.tables.iter().any(|table| {
        table.name == "segments"
            && table.foreign_keys.iter().any(|key| {
                key.referenced_table == "sources"
                    && key.from_column == "source_id"
                    && key.to_column == "id"
            })
    }));
    assert!(inventory
        .migration_markers
        .iter()
        .any(|marker| marker.table == "research_schema_migrations"
            && marker.migration_id.as_deref() == Some("research-domain-v1")));
    assert_eq!(inventory.user_version, SOURCE_USER_VERSION);
}
