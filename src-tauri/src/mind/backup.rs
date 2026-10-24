use rusqlite::{params_from_iter, types::Value, Connection, Transaction};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const ARCHIVE_FORMAT: &str = "vora-mind-database-backup";
const ARCHIVE_VERSION: u32 = 1;
const SELECTIVE_FORMAT: &str = "vora-mind-selective-package";
const OPTIONAL_ARCHIVE_TABLES: &[&str] = &["vora_imported_items", "vora_imported_packages"];
const SELECTIVE_TABLES: &[&str] = &[
    "mind_boards",
    "objects",
    "object_versions",
    "mind_board_versions",
    "mind_snapshots",
    "snapshot_object_changes",
    "snapshot_placement_changes",
    "snapshot_link_changes",
    "checkpoint_objects",
    "checkpoint_placements",
    "checkpoint_links",
    "sources",
    "segments",
    "extracts",
    "links",
    "source_revisions",
    "vora_items",
    "vora_snapshots",
    "library_item_groups",
    "library_item_group_versions",
    "chart_drawing_templates",
    "chart_drawing_alerts",
];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DatabaseArchive {
    format: String,
    version: u32,
    tables: Vec<TableArchive>,
    sequences: Vec<SequenceArchive>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    browser_storage: Option<BrowserStorageArchive>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectableRecord {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupCatalog {
    pub boards: Vec<SelectableRecord>,
    pub sources: Vec<SelectableRecord>,
    pub charts: Vec<SelectableRecord>,
    pub chart_snapshots: Vec<SelectableRecord>,
    pub library_groups: Vec<SelectableRecord>,
    pub drawing_templates: Vec<SelectableRecord>,
    pub drawing_alerts: Vec<SelectableRecord>,
    pub chart_drawings: Vec<SelectableRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageSelection {
    pub boards: Vec<String>,
    pub sources: Vec<String>,
    pub charts: Vec<String>,
    pub chart_snapshots: Vec<String>,
    pub library_groups: Vec<String>,
    pub drawing_templates: Vec<String>,
    pub drawing_alerts: Vec<String>,
    pub chart_drawings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SelectiveArchive {
    format: String,
    version: u32,
    package_id: String,
    package_name: String,
    created_at: i64,
    selection: PackageSelection,
    tables: Vec<TableArchive>,
    chart_drawings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedRecordOrigin {
    pub entity_type: String,
    pub entity_id: String,
    pub display_name: String,
    pub source_package_id: String,
    pub source_package_name: String,
    pub imported_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageImportSummary {
    pub added: usize,
    pub skipped: usize,
    pub package_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFileInfo {
    pub kind: String,
    pub package_name: Option<String>,
    pub record_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserStorageArchive {
    pub local_storage: BTreeMap<String, String>,
    #[serde(default)]
    pub settings: BTreeMap<String, String>,
    snapshots: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SequenceArchive {
    table: String,
    value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TableArchive {
    name: String,
    columns: Vec<String>,
    rows: Vec<Vec<ArchiveValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum ArchiveValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl From<Value> for ArchiveValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Integer(value) => Self::Integer(value),
            Value::Real(value) => Self::Real(value),
            Value::Text(value) => Self::Text(value),
            Value::Blob(value) => Self::Blob(value),
        }
    }
}

impl From<&ArchiveValue> for Value {
    fn from(value: &ArchiveValue) -> Self {
        match value {
            ArchiveValue::Null => Self::Null,
            ArchiveValue::Integer(value) => Self::Integer(*value),
            ArchiveValue::Real(value) => Self::Real(*value),
            ArchiveValue::Text(value) => Self::Text(value.clone()),
            ArchiveValue::Blob(value) => Self::Blob(value.clone()),
        }
    }
}

pub fn export_database(
    conn: &mut Connection,
    file_path: &str,
    browser_storage: BrowserStorageArchive,
) -> Result<(), String> {
    let target_path = validated_vr_path(file_path)?;
    let mut archive = {
        let transaction = conn.transaction().map_err(|error| error.to_string())?;
        let archive = capture_archive(&transaction)?;
        transaction.commit().map_err(|error| error.to_string())?;
        archive
    };
    archive.browser_storage = Some(browser_storage);
    let serialized = serde_json::to_vec_pretty(&archive).map_err(|error| error.to_string())?;
    write_atomically(&target_path, &serialized).map_err(|error| error.to_string())
}

pub fn get_backup_catalog(
    conn: &Connection,
    settings: &BTreeMap<String, String>,
) -> Result<BackupCatalog, String> {
    Ok(BackupCatalog {
        boards: list_selectable_records(
            conn,
            "SELECT id, title, '' FROM mind_boards ORDER BY title COLLATE NOCASE, id",
        )?,
        sources: list_selectable_records(
            conn,
            "SELECT id, title, kind FROM sources ORDER BY title COLLATE NOCASE, id",
        )?,
        charts: list_selectable_records(
            conn,
            "SELECT id, name, type FROM vora_items WHERE type = 'chart' ORDER BY name COLLATE NOCASE, id",
        )?,
        chart_snapshots: list_selectable_records(
            conn,
            "SELECT id, name, chart_id FROM vora_snapshots ORDER BY name COLLATE NOCASE, id",
        )?,
        library_groups: list_selectable_records(
            conn,
            "SELECT id, name, version FROM library_item_groups ORDER BY name COLLATE NOCASE, id",
        )?,
        drawing_templates: list_selectable_records(
            conn,
            "SELECT id, name, drawing_type FROM chart_drawing_templates ORDER BY name COLLATE NOCASE, id",
        )?,
        drawing_alerts: list_selectable_records(
            conn,
            "SELECT id, message, symbol FROM chart_drawing_alerts ORDER BY created_at DESC, id",
        )?,
        chart_drawings: settings
            .iter()
            .filter_map(|(key, _)| {
                key.strip_prefix("vora.drawings.")
                    .filter(|symbol| !symbol.is_empty())
                    .map(|symbol| SelectableRecord {
                        id: key.clone(),
                        name: symbol.to_string(),
                        detail: "Chart drawings".to_string(),
                    })
            })
            .collect(),
    })
}

fn list_selectable_records(
    conn: &Connection,
    query: &str,
) -> Result<Vec<SelectableRecord>, String> {
    let mut statement = conn.prepare(query).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(SelectableRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                detail: row.get(2)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub fn export_selective_package(
    conn: &Connection,
    file_path: &str,
    package_name: &str,
    selection: PackageSelection,
    settings: &BTreeMap<String, String>,
) -> Result<(), String> {
    let target_path = validated_vr_path(file_path)?;
    if package_name.trim().is_empty() || package_name.chars().count() > 100 {
        return Err("Package name must contain between 1 and 100 characters".to_string());
    }
    let has_selection = !selection.boards.is_empty()
        || !selection.sources.is_empty()
        || !selection.charts.is_empty()
        || !selection.chart_snapshots.is_empty()
        || !selection.library_groups.is_empty()
        || !selection.drawing_templates.is_empty()
        || !selection.drawing_alerts.is_empty()
        || !selection.chart_drawings.is_empty();
    if !has_selection {
        return Err("Select at least one item to export".to_string());
    }

    let mut tables = Vec::new();
    let board_ids = unique_ids(selection.boards.clone());
    let source_ids = unique_ids(selection.sources.clone());
    let mut chart_ids = unique_ids(selection.charts.clone());
    let chart_snapshot_ids = unique_ids(selection.chart_snapshots.clone());
    let selected_group_ids = unique_ids(selection.library_groups.clone());
    let template_ids = unique_ids(selection.drawing_templates.clone());
    let alert_ids = unique_ids(selection.drawing_alerts.clone());
    let drawing_keys = unique_ids(selection.chart_drawings.clone());
    ensure_selected_ids_exist(conn, "boards", "SELECT id FROM mind_boards WHERE id", &board_ids)?;
    ensure_selected_ids_exist(conn, "sources", "SELECT id FROM sources WHERE id", &source_ids)?;
    ensure_selected_ids_exist(
        conn,
        "charts",
        "SELECT id FROM vora_items WHERE type = 'chart' AND id",
        &unique_ids(selection.charts.clone()),
    )?;
    ensure_selected_ids_exist(
        conn,
        "chart snapshots",
        "SELECT id FROM vora_snapshots WHERE id",
        &chart_snapshot_ids,
    )?;
    ensure_selected_ids_exist(
        conn,
        "Library groups",
        "SELECT id FROM library_item_groups WHERE id",
        &selected_group_ids,
    )?;
    ensure_selected_ids_exist(
        conn,
        "drawing templates",
        "SELECT id FROM chart_drawing_templates WHERE id",
        &template_ids,
    )?;
    ensure_selected_ids_exist(
        conn,
        "drawing alerts",
        "SELECT id FROM chart_drawing_alerts WHERE id",
        &alert_ids,
    )?;
    for chart_id in query_text_values(
        conn,
        "SELECT chart_id FROM vora_snapshots WHERE id",
        &chart_snapshot_ids,
    )? {
        if !chart_id.trim().is_empty() {
            chart_ids.push(chart_id);
        }
    }
    chart_ids = unique_ids(chart_ids);

    let object_ids = query_text_ids(conn, "SELECT id FROM objects WHERE board_id", &board_ids)?;
    let board_snapshot_ids =
        query_text_ids(conn, "SELECT id FROM mind_snapshots WHERE board_id", &board_ids)?;
    append_table_by_ids(conn, &mut tables, "mind_boards", "id", &board_ids)?;
    append_table_by_ids(conn, &mut tables, "objects", "board_id", &board_ids)?;
    append_table_by_ids(conn, &mut tables, "object_versions", "object_id", &object_ids)?;
    append_table_by_ids(conn, &mut tables, "mind_board_versions", "board_id", &board_ids)?;
    append_table_by_ids(conn, &mut tables, "mind_snapshots", "board_id", &board_ids)?;
    append_table_by_ids(conn, &mut tables, "snapshot_object_changes", "snapshot_id", &board_snapshot_ids)?;
    append_table_by_ids(conn, &mut tables, "snapshot_placement_changes", "snapshot_id", &board_snapshot_ids)?;
    append_table_by_ids(conn, &mut tables, "snapshot_link_changes", "snapshot_id", &board_snapshot_ids)?;
    append_table_by_ids(conn, &mut tables, "checkpoint_objects", "checkpoint_id", &board_snapshot_ids)?;
    append_table_by_ids(conn, &mut tables, "checkpoint_placements", "checkpoint_id", &board_snapshot_ids)?;
    append_table_by_ids(conn, &mut tables, "checkpoint_links", "checkpoint_id", &board_snapshot_ids)?;
    let segment_ids = query_text_ids(conn, "SELECT id FROM segments WHERE source_id", &source_ids)?;
    let extract_ids = query_text_ids(conn, "SELECT id FROM extracts WHERE segment_id", &segment_ids)?;
    let link_ids = query_text_ids(conn, "SELECT id FROM links WHERE extract_id", &extract_ids)?;
    append_table_by_ids(conn, &mut tables, "sources", "id", &source_ids)?;
    append_table_by_ids(conn, &mut tables, "segments", "source_id", &source_ids)?;
    append_table_by_ids(conn, &mut tables, "extracts", "segment_id", &segment_ids)?;
    append_table_by_ids(conn, &mut tables, "links", "extract_id", &extract_ids)?;
    if !source_ids.is_empty() {
        append_source_revisions(
            conn,
            &mut tables,
            &source_ids,
            &segment_ids,
            &extract_ids,
            &link_ids,
        )?;
    }

    append_table_by_ids(conn, &mut tables, "vora_items", "id", &chart_ids)?;
    append_table_by_ids(conn, &mut tables, "vora_snapshots", "id", &chart_snapshot_ids)?;
    append_table_by_ids(conn, &mut tables, "library_item_groups", "id", &selected_group_ids)?;
    append_table_by_ids(
        conn,
        &mut tables,
        "library_item_group_versions",
        "group_id",
        &selected_group_ids,
    )?;
    append_table_by_ids(conn, &mut tables, "chart_drawing_templates", "id", &template_ids)?;
    append_table_by_ids(conn, &mut tables, "chart_drawing_alerts", "id", &alert_ids)?;

    let available_settings: BTreeMap<String, String> = drawing_keys
        .iter()
        .filter_map(|key| {
            key.strip_prefix("vora.drawings.")?;
            settings.get(key).map(|value| (key.clone(), value.clone()))
        })
        .collect();
    if available_settings.len() != drawing_keys.len() {
        return Err("A selected chart drawing set is no longer available".to_string());
    }

    let archive = SelectiveArchive {
        format: SELECTIVE_FORMAT.to_string(),
        version: 1,
        package_id: uuid::Uuid::now_v7().to_string(),
        package_name: package_name.trim().to_string(),
        created_at: now_millis()?,
        selection,
        tables,
        chart_drawings: available_settings,
    };
    let serialized = serde_json::to_vec_pretty(&archive).map_err(|error| error.to_string())?;
    write_atomically(&target_path, &serialized).map_err(|error| error.to_string())
}

fn unique_ids(mut ids: Vec<String>) -> Vec<String> {
    ids.retain(|id| !id.trim().is_empty());
    ids.sort();
    ids.dedup();
    ids
}

fn query_text_ids(conn: &Connection, query_prefix: &str, ids: &[String]) -> Result<Vec<String>, String> {
    query_text_values(conn, query_prefix, ids)
}

fn ensure_selected_ids_exist(
    conn: &Connection,
    category: &str,
    query_prefix: &str,
    ids: &[String],
) -> Result<(), String> {
    if query_text_ids(conn, query_prefix, ids)?.len() != ids.len() {
        return Err(format!("One or more selected {category} are no longer available"));
    }
    Ok(())
}

fn query_text_values(
    conn: &Connection,
    query_prefix: &str,
    ids: &[String],
) -> Result<Vec<String>, String> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = std::iter::repeat("?").take(ids.len()).collect::<Vec<_>>().join(", ");
    let query = format!("{query_prefix} IN ({placeholders})");
    let mut statement = conn.prepare(&query).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(
            rusqlite::params_from_iter(ids.iter()),
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

fn append_table_by_ids(
    conn: &Connection,
    tables: &mut Vec<TableArchive>,
    table: &str,
    column: &str,
    ids: &[String],
) -> Result<(), String> {
    if ids.is_empty() {
        return Ok(());
    }
    let placeholders = std::iter::repeat("?").take(ids.len()).collect::<Vec<_>>().join(", ");
    let query = format!(
        "SELECT * FROM {} WHERE {} IN ({placeholders})",
        quote_identifier(table),
        quote_identifier(column)
    );
    let columns = table_columns(conn, table)?;
    let mut statement = conn.prepare(&query).map_err(|error| error.to_string())?;
    let mut rows = statement
        .query(rusqlite::params_from_iter(ids.iter()))
        .map_err(|error| error.to_string())?;
    let mut captured = Vec::new();
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        let mut values = Vec::with_capacity(columns.len());
        for index in 0..columns.len() {
            values.push(ArchiveValue::from(
                row.get::<_, Value>(index).map_err(|error| error.to_string())?,
            ));
        }
        captured.push(values);
    }
    if !captured.is_empty() {
        tables.push(TableArchive {
            name: table.to_string(),
            columns,
            rows: captured,
        });
    }
    Ok(())
}

fn append_source_revisions(
    conn: &Connection,
    tables: &mut Vec<TableArchive>,
    source_ids: &[String],
    segment_ids: &[String],
    extract_ids: &[String],
    link_ids: &[String],
) -> Result<(), String> {
    let ids = source_ids
        .iter()
        .map(|id| ("source", id))
        .chain(segment_ids.iter().map(|id| ("segment", id)))
        .chain(extract_ids.iter().map(|id| ("extract", id)))
        .chain(link_ids.iter().map(|id| ("link", id)))
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Ok(());
    }
    let predicates = ids
        .iter()
        .map(|_| "(entity_kind = ? AND entity_id = ?)")
        .collect::<Vec<_>>()
        .join(" OR ");
    let query = format!("SELECT * FROM source_revisions WHERE {predicates}");
    let params = ids
        .iter()
        .flat_map(|(kind, id)| [*kind, id.as_str()])
        .collect::<Vec<_>>();
    let columns = table_columns(conn, "source_revisions")?;
    let mut statement = conn.prepare(&query).map_err(|error| error.to_string())?;
    let mut rows = statement
        .query(rusqlite::params_from_iter(params.iter()))
        .map_err(|error| error.to_string())?;
    let mut captured = Vec::new();
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        let mut values = Vec::with_capacity(columns.len());
        for index in 0..columns.len() {
            values.push(ArchiveValue::from(
                row.get::<_, Value>(index).map_err(|error| error.to_string())?,
            ));
        }
        captured.push(values);
    }
    if !captured.is_empty() {
        tables.push(TableArchive {
            name: "source_revisions".to_string(),
            columns,
            rows: captured,
        });
    }
    Ok(())
}

pub fn read_selective_package(file_path: &str) -> Result<(String, String), String> {
    let path = validated_vr_path(file_path)?;
    let serialized = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let archive: SelectiveArchive =
        serde_json::from_str(&serialized).map_err(|error| format!("Invalid Vora package: {error}"))?;
    validate_selective_archive(&archive)?;
    Ok((archive.package_id, archive.package_name))
}

pub fn inspect_backup_file(
    conn: &Connection,
    file_path: &str,
) -> Result<BackupFileInfo, String> {
    let path = validated_vr_path(file_path)?;
    let serialized = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let value: serde_json::Value = serde_json::from_str(&serialized)
        .map_err(|error| format!("Invalid Vora package: {error}"))?;
    match value.get("format").and_then(serde_json::Value::as_str) {
        Some(SELECTIVE_FORMAT) => {
            let archive: SelectiveArchive = serde_json::from_value(value)
                .map_err(|error| format!("Invalid selective Vora package: {error}"))?;
            validate_selective_archive(&archive)?;
            Ok(BackupFileInfo {
                kind: "selective".to_string(),
                package_name: Some(archive.package_name),
                record_count: archive
                    .tables
                    .iter()
                    .map(|table| table.rows.len())
                    .sum::<usize>()
                    + archive.chart_drawings.len(),
            })
        }
        Some(ARCHIVE_FORMAT) => {
            let archive: DatabaseArchive = serde_json::from_value(value)
                .map_err(|error| format!("Invalid legacy Vora backup: {error}"))?;
            validate_archive(&archive)?;
            validate_database_schema(conn, &archive)?;
            Ok(BackupFileInfo {
                kind: "legacy-full".to_string(),
                package_name: None,
                record_count: archive.tables.iter().map(|table| table.rows.len()).sum(),
            })
        }
        _ => Err("Unrecognized Vora backup format".to_string()),
    }
}

pub fn get_imported_item_origins(
    conn: &Connection,
) -> Result<Vec<ImportedRecordOrigin>, String> {
    let mut statement = conn
        .prepare(
            "SELECT entity_type, entity_id, display_name, source_package_id,
                    source_package_name, imported_at
             FROM vora_imported_items
             ORDER BY imported_at DESC, display_name COLLATE NOCASE",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(ImportedRecordOrigin {
                entity_type: row.get(0)?,
                entity_id: row.get(1)?,
                display_name: row.get(2)?,
                source_package_id: row.get(3)?,
                source_package_name: row.get(4)?,
                imported_at: row.get(5)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

fn validate_selective_archive(archive: &SelectiveArchive) -> Result<(), String> {
    if archive.format != SELECTIVE_FORMAT || archive.version != 1 {
        return Err("Unsupported selective Vora package format or version".to_string());
    }
    if archive.package_id.trim().is_empty()
        || archive.package_name.trim().is_empty()
        || archive.package_name.chars().count() > 100
    {
        return Err("Package metadata is invalid".to_string());
    }
    let mut names = BTreeSet::new();
    for table in &archive.tables {
        if table.name.is_empty()
            || table.name.starts_with("sqlite_")
            || !SELECTIVE_TABLES.contains(&table.name.as_str())
            || !names.insert(table.name.as_str())
            || table.columns.is_empty()
            || table.rows.iter().any(|row| row.len() != table.columns.len())
        {
            return Err("Package contains an invalid or duplicate data table".to_string());
        }
    }
    if archive.chart_drawings.keys().any(|key| !key.starts_with("vora.drawings.")) {
        return Err("Package contains a non-chart setting".to_string());
    }
    for (key, serialized) in &archive.chart_drawings {
        let drawings: Vec<serde_json::Value> = serde_json::from_str(serialized)
            .map_err(|error| format!("Chart drawing data for {key} is invalid: {error}"))?;
        if drawings.iter().any(|drawing| {
            drawing
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map_or(true, str::is_empty)
        }) {
            return Err(format!("Chart drawing data for {key} contains an invalid drawing"));
        }
    }
    Ok(())
}

pub fn merge_selective_package<F>(
    conn: &mut Connection,
    file_path: &str,
    current_settings: &BTreeMap<String, String>,
    persist_settings: F,
) -> Result<(PackageImportSummary, Vec<ImportedRecordOrigin>), String>
where
    F: FnOnce(&BTreeMap<String, String>) -> Result<(), String>,
{
    let path = validated_vr_path(file_path)?;
    let serialized = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let archive: SelectiveArchive = serde_json::from_str(&serialized)
        .map_err(|error| format!("This is not a selective Vora package: {error}"))?;
    validate_selective_archive(&archive)?;
    let already_imported: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM vora_imported_packages WHERE package_id = ?1)",
            [&archive.package_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if already_imported {
        let record_count = archive
            .tables
            .iter()
            .map(|table| table.rows.len())
            .sum::<usize>()
            + archive.chart_drawings.values().map(|value| {
                serde_json::from_str::<Vec<serde_json::Value>>(value)
                    .map_or(0, |drawings| drawings.len())
            }).sum::<usize>();
        return Ok((
            PackageImportSummary {
                added: 0,
                skipped: record_count,
                package_name: archive.package_name,
            },
            Vec::new(),
        ));
    }

    let transaction = conn.transaction().map_err(|error| error.to_string())?;
    transaction
        .pragma_update(None, "defer_foreign_keys", "ON")
        .map_err(|error| error.to_string())?;
    let mut summary = PackageImportSummary {
        added: 0,
        skipped: 0,
        package_name: archive.package_name.clone(),
    };
    let mut origins = Vec::new();
    let mut id_map = BTreeMap::<(String, String), String>::new();
    let imported_at = now_millis()?;
    let mut merged_settings = current_settings.clone();
    merge_chart_drawings(
        &archive.chart_drawings,
        &archive,
        imported_at,
        &mut merged_settings,
        &mut id_map,
        &mut origins,
        &mut summary,
    )?;
    let mut prepared_tables = Vec::with_capacity(archive.tables.len());
    for table in &archive.tables {
        let live_columns = table_columns(&transaction, &table.name)?;
        if live_columns != table.columns {
            return Err(format!(
                "Package table {} does not match this Vora version",
                table.name
            ));
        }
        let foreign_keys = table_foreign_keys(&transaction, &table.name)?;
        let primary_keys = table_primary_keys(&transaction, &table.name)?;
        if primary_keys.is_empty() {
            return Err(format!("Package table {} has no primary key", table.name));
        }
        prepared_tables.push((foreign_keys, primary_keys));
    }
    for _ in 0..=archive.tables.len() {
        let previous_mapping_count = id_map.len();
        for (table, (foreign_keys, primary_keys)) in
            archive.tables.iter().zip(&prepared_tables)
        {
            map_conflicting_ids(
                &transaction,
                table,
                primary_keys,
                foreign_keys,
                &mut id_map,
            )?;
        }
        if id_map.len() == previous_mapping_count {
            break;
        }
    }
    for (table, (foreign_keys, primary_keys)) in archive.tables.iter().zip(&prepared_tables) {
        for source_row in &table.rows {
            let mut row = source_row.clone();
            map_row_references(
                &table.name,
                &table.columns,
                &mut row,
                &foreign_keys,
                &id_map,
            );
            let original_id = row_text_value(table, source_row, "id");
            if let Some(id) = original_id.as_ref() {
                if let Some(mapped) = id_map.get(&(table.name.clone(), id.clone())) {
                    set_row_text_value(table, &mut row, "id", mapped);
                }
            }

            if row_exists_identically(&transaction, table, primary_keys, &row)? {
                summary.skipped += 1;
                continue;
            }
            if !insert_archive_row(&transaction, table, &row)? {
                summary.skipped += 1;
                continue;
            }
            summary.added += 1;

            if let Some((entity_type, display_name)) =
                provenance_label(&table.name, table, &row)
            {
                let entity_id = row_text_value(table, &row, "id")
                    .ok_or_else(|| format!("Imported {} is missing its ID", table.name))?;
                origins.push(ImportedRecordOrigin {
                    entity_type: entity_type.to_string(),
                    entity_id,
                    display_name,
                    source_package_id: archive.package_id.clone(),
                    source_package_name: archive.package_name.clone(),
                    imported_at,
                });
            }
        }
    }

    if merged_settings != *current_settings {
        persist_settings(&merged_settings)?;
    }
    for origin in &origins {
        transaction
            .execute(
                "INSERT OR IGNORE INTO vora_imported_items
                 (entity_type, entity_id, display_name, source_package_id, source_package_name, imported_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    origin.entity_type,
                    origin.entity_id,
                    origin.display_name,
                    origin.source_package_id,
                    origin.source_package_name,
                    origin.imported_at
                ],
            )
            .map_err(|error| format!("Could not record imported-item source: {error}"))?;
    }
    transaction
        .execute(
            "INSERT INTO vora_imported_packages (package_id, package_name, imported_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![archive.package_id, archive.package_name, imported_at],
        )
        .map_err(|error| format!("Could not record imported package: {error}"))?;
    transaction
        .commit()
        .map_err(|error| format!("Could not append package; workspace changes were rolled back: {error}"))?;
    Ok((summary, origins))
}

#[derive(Debug)]
struct ForeignKeyColumn {
    column: String,
    referenced_table: String,
}

fn table_primary_keys(conn: &Connection, table: &str) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({})", quote_identifier(table)))
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i32>(5)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?;
    let mut keys = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    keys.retain(|(order, _)| *order > 0);
    keys.sort_by_key(|(order, _)| *order);
    Ok(keys.into_iter().map(|(_, name)| name).collect())
}

fn table_foreign_keys(conn: &Connection, table: &str) -> Result<Vec<ForeignKeyColumn>, String> {
    let mut statement = conn
        .prepare(&format!("PRAGMA foreign_key_list({})", quote_identifier(table)))
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(ForeignKeyColumn {
                referenced_table: row.get(2)?,
                column: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

fn map_conflicting_ids(
    conn: &Connection,
    table: &TableArchive,
    primary_keys: &[String],
    foreign_keys: &[ForeignKeyColumn],
    id_map: &mut BTreeMap<(String, String), String>,
) -> Result<(), String> {
    if primary_keys != ["id"] || !table.columns.iter().any(|column| column == "id") {
        return Ok(());
    }
    let mut changed = true;
    let mut passes = 0;
    while changed && passes <= table.rows.len() {
        changed = false;
        passes += 1;
        for source_row in &table.rows {
            let Some(id) = row_text_value(table, source_row, "id") else {
                continue;
            };
            if id_map.contains_key(&(table.name.clone(), id.clone())) {
                continue;
            }
            let mut comparable_row = source_row.clone();
            map_row_references(
                &table.name,
                &table.columns,
                &mut comparable_row,
                foreign_keys,
                id_map,
            );
            let Some(existing) = find_row_by_primary_key(conn, table, primary_keys, &comparable_row)? else {
                continue;
            };
            if !archive_rows_equal(&existing, &comparable_row) {
                id_map.insert(
                    (table.name.clone(), id),
                    format!("imported_{}", uuid::Uuid::now_v7()),
                );
                changed = true;
            }
        }
    }
    Ok(())
}

fn map_row_references(
    table_name: &str,
    columns: &[String],
    row: &mut [ArchiveValue],
    foreign_keys: &[ForeignKeyColumn],
    id_map: &BTreeMap<(String, String), String>,
) {
    for foreign_key in foreign_keys {
        let Some(column_index) = columns.iter().position(|column| column == &foreign_key.column) else {
            continue;
        };
        let Some(ArchiveValue::Text(value)) = row.get(column_index) else {
            continue;
        };
        if let Some(mapped) = id_map.get(&(foreign_key.referenced_table.clone(), value.clone())) {
            row[column_index] = ArchiveValue::Text(mapped.clone());
        }
    }

    if table_name == "mind_boards" {
        for column in ["active_version_id", "branch_origin_version_id"] {
            let Some(index) = columns.iter().position(|item| item == column) else {
                continue;
            };
            if let Some(ArchiveValue::Text(value)) = row.get(index) {
                if let Some(mapped) =
                    id_map.get(&("mind_board_versions".to_string(), value.clone()))
                {
                    row[index] = ArchiveValue::Text(mapped.clone());
                }
            }
        }
    }
    if table_name == "source_revisions" {
        let kind = columns
            .iter()
            .position(|column| column == "entity_kind")
            .and_then(|index| match row.get(index) {
                Some(ArchiveValue::Text(value)) => Some(value.as_str()),
                _ => None,
            });
        let entity_index = columns.iter().position(|column| column == "entity_id");
        if let (Some(kind), Some(index), Some(ArchiveValue::Text(value))) =
            (kind, entity_index, entity_index.and_then(|index| row.get(index)))
        {
            let referenced_table = match kind {
                "source" => "sources",
                "segment" => "segments",
                "extract" => "extracts",
                "link" => "links",
                _ => return,
            };
            if let Some(mapped) = id_map.get(&(referenced_table.to_string(), value.clone())) {
                row[index] = ArchiveValue::Text(mapped.clone());
            }
        }
    }
    if table_name == "chart_drawing_alerts" {
        let symbol = columns
            .iter()
            .position(|column| column == "symbol")
            .and_then(|index| match row.get(index) {
                Some(ArchiveValue::Text(value)) => Some(value.clone()),
                _ => None,
            });
        let drawing_index = columns.iter().position(|column| column == "drawing_id");
        if let (Some(symbol), Some(index), Some(ArchiveValue::Text(value))) =
            (symbol, drawing_index, drawing_index.and_then(|index| row.get(index)))
        {
            let map_key = format!("drawing:{symbol}");
            if let Some(mapped) = id_map.get(&(map_key, value.clone())) {
                row[index] = ArchiveValue::Text(mapped.clone());
            }
        }
    }
}

fn find_row_by_primary_key(
    conn: &Connection,
    table: &TableArchive,
    primary_keys: &[String],
    row: &[ArchiveValue],
) -> Result<Option<Vec<ArchiveValue>>, String> {
    let key_indices = primary_keys
        .iter()
        .map(|key| {
            table
                .columns
                .iter()
                .position(|column| column == key)
                .ok_or_else(|| format!("Package table {} is missing key {}", table.name, key))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let predicates = primary_keys
        .iter()
        .map(|key| format!("{} = ?", quote_identifier(key)))
        .collect::<Vec<_>>()
        .join(" AND ");
    let query = format!(
        "SELECT * FROM {} WHERE {predicates}",
        quote_identifier(&table.name)
    );
    let values = key_indices
        .iter()
        .map(|index| Value::from(&row[*index]))
        .collect::<Vec<_>>();
    let mut statement = conn.prepare(&query).map_err(|error| error.to_string())?;
    let mut rows = statement
        .query(rusqlite::params_from_iter(values.iter()))
        .map_err(|error| error.to_string())?;
    let Some(found) = rows.next().map_err(|error| error.to_string())? else {
        return Ok(None);
    };
    let mut existing = Vec::with_capacity(table.columns.len());
    for index in 0..table.columns.len() {
        existing.push(ArchiveValue::from(
            found.get::<_, Value>(index).map_err(|error| error.to_string())?,
        ));
    }
    Ok(Some(existing))
}

fn row_exists_identically(
    conn: &Connection,
    table: &TableArchive,
    primary_keys: &[String],
    row: &[ArchiveValue],
) -> Result<bool, String> {
    Ok(find_row_by_primary_key(conn, table, primary_keys, row)?
        .is_some_and(|existing| archive_rows_equal(&existing, row)))
}

fn archive_rows_equal(left: &[ArchiveValue], right: &[ArchiveValue]) -> bool {
    left == right
}

fn insert_archive_row(
    conn: &Connection,
    table: &TableArchive,
    row: &[ArchiveValue],
) -> Result<bool, String> {
    let columns = table
        .columns
        .iter()
        .map(|column| quote_identifier(column))
        .collect::<Vec<_>>()
        .join(", ");
    let placeholders = std::iter::repeat("?")
        .take(table.columns.len())
        .collect::<Vec<_>>()
        .join(", ");
    let query = format!(
        "INSERT OR IGNORE INTO {} ({columns}) VALUES ({placeholders})",
        quote_identifier(&table.name)
    );
    let values = row.iter().map(Value::from).collect::<Vec<_>>();
    let changed = conn.execute(&query, rusqlite::params_from_iter(values.iter()))
        .map_err(|error| format!("Could not append {} record: {error}", table.name))?;
    Ok(changed > 0)
}

fn row_text_value(table: &TableArchive, row: &[ArchiveValue], column: &str) -> Option<String> {
    let index = table.columns.iter().position(|item| item == column)?;
    match row.get(index)? {
        ArchiveValue::Text(value) => Some(value.clone()),
        _ => None,
    }
}

fn set_row_text_value(table: &TableArchive, row: &mut [ArchiveValue], column: &str, value: &str) {
    if let Some(index) = table.columns.iter().position(|item| item == column) {
        row[index] = ArchiveValue::Text(value.to_string());
    }
}

fn provenance_label(
    table_name: &str,
    table: &TableArchive,
    row: &[ArchiveValue],
) -> Option<(&'static str, String)> {
    let (entity_type, label_column) = match table_name {
        "mind_boards" => ("Board", "title"),
        "sources" => ("Source", "title"),
        "vora_items" => ("Chart", "name"),
        "vora_snapshots" => ("Chart snapshot", "name"),
        "library_item_groups" => ("Item group", "name"),
        "chart_drawing_templates" => ("Drawing template", "name"),
        "chart_drawing_alerts" => ("Drawing alert", "message"),
        _ => return None,
    };
    Some((entity_type, row_text_value(table, row, label_column)?))
}

fn merge_chart_drawings(
    drawings: &BTreeMap<String, String>,
    archive: &SelectiveArchive,
    imported_at: i64,
    settings: &mut BTreeMap<String, String>,
    id_map: &mut BTreeMap<(String, String), String>,
    origins: &mut Vec<ImportedRecordOrigin>,
    summary: &mut PackageImportSummary,
) -> Result<(), String> {
    for (key, serialized) in drawings {
        let imported_value: serde_json::Value = serde_json::from_str(serialized)
            .map_err(|error| format!("Chart drawing data for {key} is invalid: {error}"))?;
        let imported = imported_value
            .as_array()
            .ok_or_else(|| format!("Chart drawing data for {key} must be an array"))?;
        let mut merged_value = match settings.get(key) {
            Some(current) => {
                let value: serde_json::Value = serde_json::from_str(current)
                    .map_err(|error| format!("Current chart drawing data for {key} is invalid: {error}"))?;
                value
                    .as_array()
                    .cloned()
                    .ok_or_else(|| format!("Current chart drawing data for {key} must be an array"))?
            }
            None => Vec::new(),
        };
        let symbol = key.trim_start_matches("vora.drawings.");
        for drawing in imported {
            let imported_id = drawing
                .get("id")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| format!("A drawing in {key} has no valid ID"))?;
            let existing = merged_value.iter().find(|entry| {
                entry.get("id").and_then(serde_json::Value::as_str) == Some(imported_id)
            });
            if let Some(existing) = existing {
                if existing == drawing {
                    summary.skipped += 1;
                    continue;
                }
                let new_id = format!("imported_{}", uuid::Uuid::now_v7());
                id_map.insert(
                    (format!("drawing:{symbol}"), imported_id.to_string()),
                    new_id.clone(),
                );
                let mut remapped = drawing.clone();
                let object = remapped
                    .as_object_mut()
                    .ok_or_else(|| format!("A drawing in {key} must be an object"))?;
                object.insert("id".to_string(), serde_json::Value::String(new_id.clone()));
                merged_value.push(remapped);
                origins.push(ImportedRecordOrigin {
                    entity_type: "Chart drawing".to_string(),
                    entity_id: new_id,
                    display_name: symbol.to_string(),
                    source_package_id: archive.package_id.clone(),
                    source_package_name: archive.package_name.clone(),
                    imported_at,
                });
                summary.added += 1;
            } else {
                merged_value.push(drawing.clone());
                origins.push(ImportedRecordOrigin {
                    entity_type: "Chart drawing".to_string(),
                    entity_id: imported_id.to_string(),
                    display_name: symbol.to_string(),
                    source_package_id: archive.package_id.clone(),
                    source_package_name: archive.package_name.clone(),
                    imported_at,
                });
                summary.added += 1;
            }
        }
        if settings.contains_key(key) || !merged_value.is_empty() {
            let value = serde_json::to_string(&merged_value).map_err(|error| error.to_string())?;
            settings.insert(key.clone(), value);
        }
    }
    Ok(())
}

pub fn read_browser_storage(
    conn: &Connection,
    file_path: &str,
) -> Result<Option<BrowserStorageArchive>, String> {
    let archive = read_archive(file_path)?;
    validate_archive(&archive)?;
    validate_database_schema(conn, &archive)?;
    browser_storage_from_archive(&archive)
}

pub fn import_database(
    conn: &mut Connection,
    file_path: &str,
) -> Result<Option<BrowserStorageArchive>, String> {
    import_database_with_settings(conn, file_path, |_| Ok(()))
}

pub fn import_database_with_settings<F>(
    conn: &mut Connection,
    file_path: &str,
    persist_settings: F,
) -> Result<Option<BrowserStorageArchive>, String>
where
    F: FnOnce(&Option<BrowserStorageArchive>) -> Result<(), String>,
{
    let archive = read_archive(file_path)?;
    validate_archive(&archive)?;
    validate_database_schema(conn, &archive)?;

    let transaction = conn.transaction().map_err(|error| error.to_string())?;
    restore_archive(&transaction, &archive)?;
    let browser_storage = browser_storage_from_archive(&archive)?;
    persist_settings(&browser_storage)?;
    transaction.commit().map_err(|error| {
        format!("Could not restore backup; database changes were rolled back: {error}")
    })?;
    Ok(browser_storage)
}

fn browser_storage_from_archive(
    archive: &DatabaseArchive,
) -> Result<Option<BrowserStorageArchive>, String> {
    let legacy_settings = legacy_settings_from_archive(archive)?;
    let Some(mut storage) = archive.browser_storage.clone().or_else(|| {
        (!legacy_settings.is_empty()).then(|| BrowserStorageArchive {
            local_storage: BTreeMap::new(),
            settings: BTreeMap::new(),
            snapshots: Vec::new(),
        })
    }) else {
        return Ok(None);
    };

    let mut settings = BTreeMap::new();
    for (key, value) in &storage.local_storage {
        settings.entry(key.clone()).or_insert_with(|| value.clone());
    }
    for (key, value) in legacy_settings {
        settings.insert(key, value);
    }
    settings.extend(storage.settings.clone());
    storage.settings = settings;
    Ok(Some(storage))
}

fn legacy_settings_from_archive(
    archive: &DatabaseArchive,
) -> Result<BTreeMap<String, String>, String> {
    let Some(table) = archive
        .tables
        .iter()
        .find(|table| table.name == "vora_settings")
    else {
        return Ok(BTreeMap::new());
    };
    let key_index = table
        .columns
        .iter()
        .position(|column| column == "key")
        .ok_or_else(|| "Legacy settings backup has no key column".to_string())?;
    let value_index = table
        .columns
        .iter()
        .position(|column| column == "value")
        .ok_or_else(|| "Legacy settings backup has no value column".to_string())?;
    let mut settings = BTreeMap::new();
    for row in &table.rows {
        let (ArchiveValue::Text(key), ArchiveValue::Text(value)) =
            (&row[key_index], &row[value_index])
        else {
            return Err("Legacy settings backup contains invalid values".to_string());
        };
        settings.insert(key.clone(), value.clone());
    }
    Ok(settings)
}

fn read_archive(file_path: &str) -> Result<DatabaseArchive, String> {
    let source_path = validated_vr_path(file_path)?;
    let serialized = fs::read_to_string(&source_path).map_err(|error| {
        format!(
            "Failed to read backup file {}: {error}",
            source_path.display()
        )
    })?;
    serde_json::from_str(&serialized).map_err(|error| format!("Invalid Vora backup file: {error}"))
}

fn capture_archive(conn: &Connection) -> Result<DatabaseArchive, String> {
    let table_names = application_table_names(conn)?;
    let mut tables = Vec::with_capacity(table_names.len());

    for name in table_names {
        let columns = table_columns(conn, &name)?;
        let query = format!("SELECT * FROM {}", quote_identifier(&name));
        let mut statement = conn.prepare(&query).map_err(|error| error.to_string())?;
        let column_count = statement.column_count();
        let mut rows = statement.query([]).map_err(|error| error.to_string())?;
        let mut archived_rows = Vec::new();

        while let Some(row) = rows.next().map_err(|error| error.to_string())? {
            let mut values = Vec::with_capacity(column_count);
            for index in 0..column_count {
                let value = row
                    .get::<_, Value>(index)
                    .map_err(|error| error.to_string())?;
                values.push(ArchiveValue::from(value));
            }
            archived_rows.push(values);
        }

        tables.push(TableArchive {
            name,
            columns,
            rows: archived_rows,
        });
    }

    let sequences = if has_table(conn, "sqlite_sequence")? {
        let mut statement = conn
            .prepare("SELECT name, seq FROM sqlite_sequence ORDER BY name")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(SequenceArchive {
                    table: row.get(0)?,
                    value: row.get(1)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?
    } else {
        Vec::new()
    };

    Ok(DatabaseArchive {
        format: ARCHIVE_FORMAT.to_string(),
        version: ARCHIVE_VERSION,
        tables,
        sequences,
        browser_storage: None,
    })
}

fn validate_archive(archive: &DatabaseArchive) -> Result<(), String> {
    if archive.format != ARCHIVE_FORMAT || archive.version != ARCHIVE_VERSION {
        return Err(format!(
            "Unsupported backup format or version ({} v{})",
            archive.format, archive.version
        ));
    }
    if archive.tables.is_empty() {
        return Err("The backup contains no database tables".to_string());
    }

    let mut table_names = BTreeSet::new();
    for table in &archive.tables {
        if table.name.is_empty()
            || table.name.starts_with("sqlite_")
            || !table_names.insert(table.name.as_str())
        {
            return Err("The backup contains an invalid or duplicate table".to_string());
        }
        if table.columns.is_empty() {
            return Err(format!("Table {} has no columns", table.name));
        }
        let mut columns = BTreeSet::new();
        if table
            .columns
            .iter()
            .any(|column| column.is_empty() || !columns.insert(column.as_str()))
        {
            return Err(format!(
                "Table {} contains invalid or duplicate columns",
                table.name
            ));
        }
        if table
            .rows
            .iter()
            .any(|row| row.len() != table.columns.len())
        {
            return Err(format!(
                "Table {} contains a row with an invalid number of values",
                table.name
            ));
        }
    }
    for sequence in &archive.sequences {
        if !table_names.contains(sequence.table.as_str()) || sequence.value < 0 {
            return Err("The backup contains invalid SQLite sequence data".to_string());
        }
    }
    Ok(())
}

fn validate_database_schema(conn: &Connection, archive: &DatabaseArchive) -> Result<(), String> {
    let live_tables = application_table_names(conn)?;
    let backup_tables: BTreeSet<&str> = archive
        .tables
        .iter()
        .map(|table| table.name.as_str())
        .collect();
    let mut backup_without_legacy_settings = backup_tables.clone();
    backup_without_legacy_settings.remove("vora_settings");
    let live_table_set: BTreeSet<&str> = live_tables.iter().map(String::as_str).collect();
    let live_without_optional_tables: BTreeSet<&str> = live_table_set
        .iter()
        .copied()
        .filter(|table| !OPTIONAL_ARCHIVE_TABLES.contains(table))
        .collect();
    if live_table_set != backup_tables
        && live_table_set != backup_without_legacy_settings
        && live_without_optional_tables != backup_without_legacy_settings
    {
        return Err("This backup does not match the current database schema".to_string());
    }

    for table in &archive.tables {
        if table.name == "vora_settings" && !has_table(conn, "vora_settings")? {
            continue;
        }
        let live_columns = table_columns(conn, &table.name)?;
        if live_columns != table.columns {
            return Err(format!(
                "The columns in table {} do not match this version of Vora Mind",
                table.name
            ));
        }
    }

    Ok(())
}

fn has_table(conn: &Connection, name: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
         )",
        [name],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

fn restore_archive(transaction: &Transaction<'_>, archive: &DatabaseArchive) -> Result<(), String> {
    transaction
        .execute_batch("PRAGMA defer_foreign_keys = ON;")
        .map_err(|error| error.to_string())?;

    let triggers = {
        let mut statement = transaction
            .prepare(
                "SELECT name, sql FROM sqlite_master WHERE type = 'trigger' AND sql IS NOT NULL",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?
    };
    for (name, _) in &triggers {
        transaction
            .execute_batch(&format!("DROP TRIGGER {}", quote_identifier(name)))
            .map_err(|error| error.to_string())?;
    }

    for table in archive.tables.iter().rev() {
        if table.name == "vora_settings" && !has_table(transaction, &table.name)? {
            continue;
        }
        let query = format!("DELETE FROM {}", quote_identifier(&table.name));
        transaction
            .execute(&query, [])
            .map_err(|error| format!("Could not clear table {}: {error}", table.name))?;
    }
    for table in OPTIONAL_ARCHIVE_TABLES {
        if !archive.tables.iter().any(|archived| archived.name == *table)
            && has_table(transaction, table)?
        {
            transaction
                .execute(&format!("DELETE FROM {}", quote_identifier(table)), [])
                .map_err(|error| format!("Could not clear table {table}: {error}"))?;
        }
    }

    for table in &archive.tables {
        if table.name == "vora_settings" && !has_table(transaction, &table.name)? {
            continue;
        }
        let columns = table
            .columns
            .iter()
            .map(|column| quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = std::iter::repeat("?")
            .take(table.columns.len())
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "INSERT INTO {} ({columns}) VALUES ({placeholders})",
            quote_identifier(&table.name)
        );

        for row in &table.rows {
            let values: Vec<Value> = row.iter().map(Value::from).collect();
            transaction
                .execute(&query, params_from_iter(values.iter()))
                .map_err(|error| format!("Could not restore table {}: {error}", table.name))?;
        }
    }

    for (_, sql) in triggers {
        transaction
            .execute_batch(&sql)
            .map_err(|error| format!("Could not restore a database trigger: {error}"))?;
    }

    if has_table(transaction, "sqlite_sequence")? {
        transaction
            .execute("DELETE FROM sqlite_sequence", [])
            .map_err(|error| error.to_string())?;
        for sequence in &archive.sequences {
            transaction
                .execute(
                    "INSERT INTO sqlite_sequence (name, seq) VALUES (?1, ?2)",
                    (&sequence.table, sequence.value),
                )
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn application_table_names(conn: &Connection) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare(
            "SELECT name FROM sqlite_master
             WHERE type = 'table'
               AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\'
               AND sql IS NOT NULL
               AND upper(ltrim(sql)) NOT LIKE 'CREATE VIRTUAL TABLE%' 
             ORDER BY name",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

fn table_columns(conn: &Connection, table_name: &str) -> Result<Vec<String>, String> {
    let query = format!("PRAGMA table_info({})", quote_identifier(table_name));
    let mut statement = conn.prepare(&query).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn now_millis() -> Result<i64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .map_err(|error| error.to_string())
}

fn validated_vr_path(file_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(file_path);
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("vr"))
    {
        return Err("Backup files must use the .vr extension".to_string());
    }
    Ok(path.to_path_buf())
}

fn write_atomically(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Backup path has no file name",
        )
    })?;
    let temporary_path = path.with_file_name(format!(
        ".{}.{}.tmp",
        file_name.to_string_lossy(),
        uuid::Uuid::now_v7()
    ));

    let write_result = (|| {
        let mut temporary_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        temporary_file.write_all(contents)?;
        temporary_file.sync_all()?;
        fs::rename(&temporary_path, path)
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    write_result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_archive() -> DatabaseArchive {
        DatabaseArchive {
            format: ARCHIVE_FORMAT.to_string(),
            version: ARCHIVE_VERSION,
            tables: vec![
                TableArchive {
                    name: "parents".to_string(),
                    columns: vec!["id".to_string(), "label".to_string()],
                    rows: vec![vec![
                        ArchiveValue::Integer(1),
                        ArchiveValue::Text("restored".to_string()),
                    ]],
                },
                TableArchive {
                    name: "children".to_string(),
                    columns: vec!["id".to_string(), "parent_id".to_string()],
                    rows: vec![vec![ArchiveValue::Integer(1), ArchiveValue::Integer(1)]],
                },
            ],
            sequences: Vec::new(),
            browser_storage: None,
        }
    }

    fn test_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE parents (id INTEGER PRIMARY KEY, label TEXT NOT NULL);
                 CREATE TABLE children (
                     id INTEGER PRIMARY KEY,
                     parent_id INTEGER NOT NULL REFERENCES parents(id)
                 );
                 CREATE TRIGGER protect_parent_delete
                 BEFORE DELETE ON parents
                 BEGIN
                     SELECT RAISE(ABORT, 'parent rows are immutable');
                 END;
                 INSERT INTO parents VALUES (9, 'existing');
                 INSERT INTO children VALUES (9, 9);",
            )
            .expect("create test schema");
        connection
    }

    #[test]
    fn restore_replaces_rows_and_preserves_foreign_keys() {
        let mut connection = test_connection();
        let archive = test_archive();
        validate_database_schema(&connection, &archive).expect("matching schema");

        let transaction = connection.transaction().expect("start transaction");
        restore_archive(&transaction, &archive).expect("restore archive");
        transaction.commit().expect("commit restore");

        let restored_label: String = connection
            .query_row("SELECT label FROM parents WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("read restored parent");
        let restored_child: i64 = connection
            .query_row("SELECT parent_id FROM children WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("read restored child");
        assert_eq!(restored_label, "restored");
        assert_eq!(restored_child, 1);
        assert!(connection
            .execute("DELETE FROM parents WHERE id = 1", [])
            .is_err());
        let old_rows: i64 = connection
            .query_row("SELECT COUNT(*) FROM parents WHERE id = 9", [], |row| {
                row.get(0)
            })
            .expect("count replaced rows");
        assert_eq!(old_rows, 0);
    }

    #[test]
    fn invalid_foreign_key_data_rolls_back_existing_database() {
        let mut connection = test_connection();
        let mut archive = test_archive();
        archive.tables[1].rows[0][1] = ArchiveValue::Integer(77);

        let transaction = connection.transaction().expect("start transaction");
        restore_archive(&transaction, &archive).expect("insert rows before deferred check");
        assert!(transaction.commit().is_err());

        let existing_label: String = connection
            .query_row("SELECT label FROM parents WHERE id = 9", [], |row| {
                row.get(0)
            })
            .expect("read unchanged parent");
        let existing_child: i64 = connection
            .query_row("SELECT parent_id FROM children WHERE id = 9", [], |row| {
                row.get(0)
            })
            .expect("read unchanged child");
        assert_eq!(existing_label, "existing");
        assert_eq!(existing_child, 9);
    }

    #[test]
    fn rejects_incomplete_archives_and_wrong_file_extensions() {
        let connection = test_connection();
        let mut archive = test_archive();
        archive.tables.pop();
        assert!(validate_database_schema(&connection, &archive).is_err());
        assert!(validated_vr_path("backup.json").is_err());
        assert!(connection
            .execute("INSERT INTO parents VALUES (10, 'still usable')", [])
            .is_ok());
    }

    #[test]
    fn accepts_legacy_archive_without_browser_storage() {
        let archive = test_archive();
        let serialized = serde_json::to_string(&archive).expect("serialize legacy archive");
        let restored: DatabaseArchive =
            serde_json::from_str(&serialized).expect("deserialize legacy archive");
        assert!(restored.browser_storage.is_none());
    }

    #[test]
    fn legacy_full_archive_without_provenance_tables_remains_restorable() {
        let mut connection = test_connection();
        connection
            .execute_batch(
                "CREATE TABLE vora_imported_items (
                     entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, display_name TEXT NOT NULL,
                     source_package_id TEXT NOT NULL, source_package_name TEXT NOT NULL,
                     imported_at INTEGER NOT NULL, PRIMARY KEY(entity_type, entity_id)
                 );
                 CREATE TABLE vora_imported_packages (
                     package_id TEXT PRIMARY KEY, package_name TEXT NOT NULL, imported_at INTEGER NOT NULL
                 );
                 INSERT INTO vora_imported_packages VALUES ('old-package', 'Old package', 1);",
            )
            .expect("create newer provenance schema");
        let archive = test_archive();
        validate_database_schema(&connection, &archive)
            .expect("accept archive predating optional provenance tables");

        let transaction = connection.transaction().expect("start restore transaction");
        restore_archive(&transaction, &archive).expect("restore legacy archive");
        transaction.commit().expect("commit legacy restore");
        let remaining_packages: i64 = connection
            .query_row("SELECT COUNT(*) FROM vora_imported_packages", [], |row| row.get(0))
            .expect("read provenance table retained by the new schema");
        assert_eq!(remaining_packages, 0);
    }

    #[test]
    fn export_import_round_trip_preserves_database_values() {
        let mut connection = test_connection();
        connection
            .execute("UPDATE parents SET label = 'backup value' WHERE id = 9", [])
            .expect("update source database");
        let path = std::env::temp_dir().join(format!("vora-backup-{}.vr", uuid::Uuid::now_v7()));
        let path_string = path.to_string_lossy().into_owned();

        export_database(
            &mut connection,
            &path_string,
            BrowserStorageArchive {
                local_storage: BTreeMap::from([(
                    "vora.drawings.XAUUSD".to_string(),
                    "[]".to_string(),
                )]),
                settings: BTreeMap::from([("theme".to_string(), "dark".to_string())]),
                snapshots: vec![serde_json::json!({ "id": "snapshot-1" })],
            },
        )
        .expect("export database");
        connection
            .execute(
                "UPDATE parents SET label = 'changed later' WHERE id = 9",
                [],
            )
            .expect("change database after export");
        let browser_storage =
            import_database(&mut connection, &path_string).expect("import database");
        let restored_browser_storage = browser_storage.expect("browser storage in backup");
        assert_eq!(
            restored_browser_storage.settings.get("theme"),
            Some(&"dark".to_string())
        );
        assert_eq!(
            restored_browser_storage
                .local_storage
                .get("vora.drawings.XAUUSD"),
            Some(&"[]".to_string())
        );
        assert_eq!(
            restored_browser_storage.snapshots[0]["id"],
            serde_json::json!("snapshot-1")
        );

        let restored_label: String = connection
            .query_row("SELECT label FROM parents WHERE id = 9", [], |row| {
                row.get(0)
            })
            .expect("read restored value");
        assert_eq!(restored_label, "backup value");
        fs::remove_file(path).expect("remove test backup");
    }

    #[test]
    fn imports_legacy_database_settings_without_restoring_a_settings_table() {
        let connection = test_connection();
        let mut archive = test_archive();
        archive.tables.push(TableArchive {
            name: "vora_settings".to_string(),
            columns: vec!["key".to_string(), "value".to_string()],
            rows: vec![vec![
                ArchiveValue::Text("legacy_key".to_string()),
                ArchiveValue::Text("legacy_value".to_string()),
            ]],
        });
        validate_database_schema(&connection, &archive).expect("legacy schema is supported");
        let browser_storage = browser_storage_from_archive(&archive).expect("read legacy settings");
        assert_eq!(
            browser_storage
                .expect("legacy settings produce backup storage")
                .settings
                .get("legacy_key"),
            Some(&"legacy_value".to_string())
        );

        let mut connection = connection;
        let transaction = connection.transaction().expect("start transaction");
        restore_archive(&transaction, &archive).expect("restore legacy archive");
        transaction.commit().expect("commit restored database");
        assert!(!has_table(&connection, "vora_settings").expect("check legacy table"));
    }

    #[test]
    fn settings_persistence_failure_rolls_back_database_restore() {
        let mut connection = test_connection();
        let mut archive = test_archive();
        archive.browser_storage = Some(BrowserStorageArchive {
            local_storage: BTreeMap::new(),
            settings: BTreeMap::from([("theme".to_string(), "light".to_string())]),
            snapshots: Vec::new(),
        });
        let path = std::env::temp_dir().join(format!("vora-backup-{}.vr", uuid::Uuid::now_v7()));
        fs::write(
            &path,
            serde_json::to_vec(&archive).expect("serialize archive"),
        )
        .expect("write archive");

        let result =
            import_database_with_settings(&mut connection, &path.to_string_lossy(), |_| {
                Err("settings file is read-only".to_string())
            });
        assert!(result.is_err());
        let original_label: String = connection
            .query_row("SELECT label FROM parents WHERE id = 9", [], |row| {
                row.get(0)
            })
            .expect("read unchanged row");
        assert_eq!(original_label, "existing");
        fs::remove_file(path).expect("remove test backup");
    }

    #[test]
    fn selective_import_remaps_conflicting_board_and_version_ids_and_is_idempotent() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE mind_boards (
                     id TEXT PRIMARY KEY, title TEXT NOT NULL, created_at INTEGER NOT NULL,
                     updated_at INTEGER NOT NULL, active_version_id TEXT,
                     branch_origin_version_id TEXT
                 );
                 CREATE TABLE mind_board_versions (
                     id TEXT PRIMARY KEY, board_id TEXT NOT NULL, version_major INTEGER NOT NULL,
                     version_minor INTEGER NOT NULL, version_patch INTEGER NOT NULL,
                     parent_version_id TEXT, forked_from_version_id TEXT, snapshot_json TEXT NOT NULL,
                     created_at INTEGER NOT NULL, FOREIGN KEY(board_id) REFERENCES mind_boards(id),
                     FOREIGN KEY(parent_version_id) REFERENCES mind_board_versions(id),
                     FOREIGN KEY(forked_from_version_id) REFERENCES mind_board_versions(id)
                 );
                 CREATE TABLE vora_imported_items (
                     entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, display_name TEXT NOT NULL,
                     source_package_id TEXT NOT NULL, source_package_name TEXT NOT NULL,
                     imported_at INTEGER NOT NULL, PRIMARY KEY(entity_type, entity_id)
                 );
                 CREATE TABLE vora_imported_packages (
                     package_id TEXT PRIMARY KEY, package_name TEXT NOT NULL, imported_at INTEGER NOT NULL
                 );
                 INSERT INTO mind_boards VALUES ('board-1', 'Local board', 1, 1, 'version-1', NULL);
                 INSERT INTO mind_board_versions VALUES
                     ('version-1', 'board-1', 1, 0, 0, NULL, NULL, '{}', 1);",
            )
            .expect("create selective-import schema");

        let archive = SelectiveArchive {
            format: SELECTIVE_FORMAT.to_string(),
            version: 1,
            package_id: "package-remap-test".to_string(),
            package_name: "Shared board".to_string(),
            created_at: 1,
            selection: PackageSelection {
                boards: vec!["board-1".to_string()],
                sources: Vec::new(),
                charts: Vec::new(),
                chart_snapshots: Vec::new(),
                library_groups: Vec::new(),
                drawing_templates: Vec::new(),
                drawing_alerts: Vec::new(),
                chart_drawings: Vec::new(),
            },
            tables: vec![
                TableArchive {
                    name: "mind_boards".to_string(),
                    columns: vec![
                        "id".to_string(),
                        "title".to_string(),
                        "created_at".to_string(),
                        "updated_at".to_string(),
                        "active_version_id".to_string(),
                        "branch_origin_version_id".to_string(),
                    ],
                    rows: vec![vec![
                        ArchiveValue::Text("board-1".to_string()),
                        ArchiveValue::Text("Shared board".to_string()),
                        ArchiveValue::Integer(2),
                        ArchiveValue::Integer(2),
                        ArchiveValue::Text("version-1".to_string()),
                        ArchiveValue::Null,
                    ]],
                },
                TableArchive {
                    name: "mind_board_versions".to_string(),
                    columns: vec![
                        "id".to_string(),
                        "board_id".to_string(),
                        "version_major".to_string(),
                        "version_minor".to_string(),
                        "version_patch".to_string(),
                        "parent_version_id".to_string(),
                        "forked_from_version_id".to_string(),
                        "snapshot_json".to_string(),
                        "created_at".to_string(),
                    ],
                    rows: vec![vec![
                        ArchiveValue::Text("version-1".to_string()),
                        ArchiveValue::Text("board-1".to_string()),
                        ArchiveValue::Integer(2),
                        ArchiveValue::Integer(0),
                        ArchiveValue::Integer(0),
                        ArchiveValue::Null,
                        ArchiveValue::Null,
                        ArchiveValue::Text("{\"shared\":true}".to_string()),
                        ArchiveValue::Integer(2),
                    ]],
                },
            ],
            chart_drawings: BTreeMap::new(),
        };
        let path = std::env::temp_dir().join(format!("vora-selective-{}.vr", uuid::Uuid::now_v7()));
        fs::write(&path, serde_json::to_vec(&archive).expect("serialize selective package"))
            .expect("write selective package");
        let mut connection = connection;
        let settings = BTreeMap::new();
        let (first_result, imported) = merge_selective_package(
            &mut connection,
            &path.to_string_lossy(),
            &settings,
            |_| Ok(()),
        )
        .expect("import selective package");

        assert_eq!(first_result.added, 2);
        assert_eq!(imported.len(), 1);
        let (board_count, active_version): (i64, String) = connection
            .query_row(
                "SELECT COUNT(*), MAX(active_version_id) FROM mind_boards WHERE title = 'Shared board'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read remapped imported board");
        assert_eq!(board_count, 1);
        let matching_versions: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM mind_board_versions
                 WHERE id = ?1 AND board_id = (SELECT id FROM mind_boards WHERE title = 'Shared board')",
                [&active_version],
                |row| row.get(0),
            )
            .expect("verify remapped active version reference");
        assert_eq!(matching_versions, 1);

        let (second_result, second_origins) = merge_selective_package(
            &mut connection,
            &path.to_string_lossy(),
            &settings,
            |_| Ok(()),
        )
        .expect("repeat selective import");
        assert_eq!(second_result.added, 0);
        assert_eq!(second_result.skipped, 2);
        assert!(second_origins.is_empty());
        let provenance_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM vora_imported_items", [], |row| row.get(0))
            .expect("count imported-item provenance");
        assert_eq!(provenance_count, 1);
        fs::remove_file(path).expect("remove selective package");
    }

    #[test]
    fn selective_import_rolls_back_database_when_settings_cannot_be_persisted() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch(
                "CREATE TABLE library_item_groups (
                     id TEXT PRIMARY KEY, name TEXT NOT NULL, version TEXT NOT NULL,
                     snapshot_json TEXT NOT NULL, created_at INTEGER NOT NULL,
                     updated_at INTEGER NOT NULL, deleted_at INTEGER
                 );
                 CREATE TABLE vora_imported_items (
                     entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, display_name TEXT NOT NULL,
                     source_package_id TEXT NOT NULL, source_package_name TEXT NOT NULL,
                     imported_at INTEGER NOT NULL, PRIMARY KEY(entity_type, entity_id)
                 );
                 CREATE TABLE vora_imported_packages (
                     package_id TEXT PRIMARY KEY, package_name TEXT NOT NULL, imported_at INTEGER NOT NULL
                 );",
            )
            .expect("create selective-import schema");
        let archive = SelectiveArchive {
            format: SELECTIVE_FORMAT.to_string(),
            version: 1,
            package_id: "package-rollback-test".to_string(),
            package_name: "Group package".to_string(),
            created_at: 1,
            selection: PackageSelection {
                boards: Vec::new(),
                sources: Vec::new(),
                charts: Vec::new(),
                chart_snapshots: Vec::new(),
                library_groups: vec!["group-1".to_string()],
                drawing_templates: Vec::new(),
                drawing_alerts: Vec::new(),
                chart_drawings: vec!["vora.drawings.XAUUSD".to_string()],
            },
            tables: vec![TableArchive {
                name: "library_item_groups".to_string(),
                columns: vec![
                    "id".to_string(),
                    "name".to_string(),
                    "version".to_string(),
                    "snapshot_json".to_string(),
                    "created_at".to_string(),
                    "updated_at".to_string(),
                    "deleted_at".to_string(),
                ],
                rows: vec![vec![
                    ArchiveValue::Text("group-1".to_string()),
                    ArchiveValue::Text("Group 1".to_string()),
                    ArchiveValue::Text("1.0.0".to_string()),
                    ArchiveValue::Text("{}".to_string()),
                    ArchiveValue::Integer(1),
                    ArchiveValue::Integer(1),
                    ArchiveValue::Null,
                ]],
            }],
            chart_drawings: BTreeMap::from([(
                "vora.drawings.XAUUSD".to_string(),
                "[{\"id\":\"drawing-1\"}]".to_string(),
            )]),
        };
        let path = std::env::temp_dir().join(format!("vora-selective-{}.vr", uuid::Uuid::now_v7()));
        fs::write(&path, serde_json::to_vec(&archive).expect("serialize selective package"))
            .expect("write selective package");
        let mut connection = connection;
        let result = merge_selective_package(
            &mut connection,
            &path.to_string_lossy(),
            &BTreeMap::new(),
            |_| Err("settings file is read-only".to_string()),
        );
        assert!(result.is_err());
        let group_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM library_item_groups", [], |row| row.get(0))
            .expect("count groups after rolled-back import");
        assert_eq!(group_count, 0);
        let package_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM vora_imported_packages", [], |row| row.get(0))
            .expect("count import markers after rollback");
        assert_eq!(package_count, 0);
        fs::remove_file(path).expect("remove selective package");
    }
}
