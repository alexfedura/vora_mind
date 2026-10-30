use rusqlite::{params, Connection};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};

use super::canvas_db::AppState;

const PORTABLE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS vora_items (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,
    name TEXT NOT NULL,
    data TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_vora_items_type_updated
    ON vora_items(type, updated_at DESC);

CREATE TABLE IF NOT EXISTS vora_snapshots (
    id TEXT PRIMARY KEY,
    chart_id TEXT NOT NULL,
    name TEXT NOT NULL,
    data TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (chart_id) REFERENCES vora_items(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_vora_snapshots_chart_created
    ON vora_snapshots(chart_id, created_at DESC);

CREATE TABLE IF NOT EXISTS vora_imported_items (
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    source_package_id TEXT NOT NULL,
    source_package_name TEXT NOT NULL,
    imported_at INTEGER NOT NULL,
    PRIMARY KEY (entity_type, entity_id)
);
CREATE INDEX IF NOT EXISTS idx_vora_imported_items_source
    ON vora_imported_items(source_package_id, imported_at DESC);

CREATE TABLE IF NOT EXISTS vora_imported_packages (
    package_id TEXT PRIMARY KEY,
    package_name TEXT NOT NULL,
    imported_at INTEGER NOT NULL
);
"#;

#[derive(serde::Deserialize)]
struct LegacyDatabasePathConfig {
    path: String,
}

fn legacy_database_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join("database.json"))
        .map_err(|error| error.to_string())
}

pub fn migrate_legacy_database_selector(
    app: &AppHandle,
    fixed_database_path: &Path,
) -> Result<(), String> {
    let selector_path = legacy_database_config_path(app)?;
    migrate_legacy_database_selector_file(&selector_path, fixed_database_path)
}

fn migrate_legacy_database_selector_file(
    selector_path: &Path,
    fixed_database_path: &Path,
) -> Result<(), String> {
    let contents = match fs::read_to_string(&selector_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    let selector: LegacyDatabasePathConfig =
        serde_json::from_str(&contents).map_err(|error| error.to_string())?;
    let selected_path = PathBuf::from(selector.path.trim());
    if selected_path.as_os_str().is_empty() {
        return Err("Legacy database path is empty".to_string());
    }

    let same_database = selected_path == fixed_database_path
        || fs::canonicalize(&selected_path)
            .ok()
            .zip(fs::canonicalize(fixed_database_path).ok())
            .is_some_and(|(selected, fixed)| selected == fixed);
    if !same_database {
        if !selected_path.is_file() {
            return Err(format!(
                "The previously selected database is missing: {}",
                selected_path.display()
            ));
        }
        if fixed_database_path.exists() {
            return Err(format!(
                "The fixed database already exists at {} while the legacy selector points to {}. No data was changed; resolve the two database files before continuing.",
                fixed_database_path.display(),
                selected_path.display()
            ));
        }

        let source = Connection::open(&selected_path).map_err(|error| error.to_string())?;
        initialize_workspace_schema(&source).map_err(|error| error.to_string())?;
        let settings = initialize_settings_file(&source, &selected_path, None)?;

        let temporary_path = temporary_sibling(fixed_database_path, "migration")?;
        let migration_result = (|| {
            let mut destination =
                Connection::open(&temporary_path).map_err(|error| error.to_string())?;
            {
                let backup = rusqlite::backup::Backup::new(&source, &mut destination)
                    .map_err(|error| error.to_string())?;
                backup
                    .run_to_completion(5, std::time::Duration::from_millis(100), None)
                    .map_err(|error| error.to_string())?;
            }
            initialize_workspace_schema(&destination).map_err(|error| error.to_string())?;
            destination
                .pragma_update(None, "journal_mode", "DELETE")
                .map_err(|error| error.to_string())?;
            write_settings_file(&settings_path_for_database(fixed_database_path)?, &settings)?;
            drop(destination);
            fs::rename(&temporary_path, fixed_database_path).map_err(|error| error.to_string())
        })();
        if let Err(error) = migration_result {
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }
    }

    fs::remove_file(selector_path).map_err(|error| error.to_string())
}

pub fn settings_path_for_database(path: &Path) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Database directory is unavailable".to_string())?;
    let stem = path
        .file_stem()
        .ok_or_else(|| "Database filename is unavailable".to_string())?
        .to_string_lossy();
    Ok(parent.join(format!("{stem}.settings.json")))
}

fn read_settings_file(path: &Path) -> Result<Option<HashMap<String, String>>, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let settings = serde_json::from_str(&contents)
        .map_err(|error| format!("Invalid settings file {}: {error}", path.display()))?;
    Ok(Some(settings))
}

fn write_settings_file(path: &Path, settings: &HashMap<String, String>) -> Result<(), String> {
    let contents = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
    let temporary_path = temporary_sibling(path, "settings.tmp")?;
    let backup_path = temporary_sibling(path, "settings.bak")?;
    fs::write(&temporary_path, contents).map_err(|error| error.to_string())?;

    let had_previous = path.exists();
    if had_previous {
        if let Err(error) = fs::rename(path, &backup_path) {
            let _ = fs::remove_file(&temporary_path);
            return Err(error.to_string());
        }
    }
    if let Err(error) = fs::rename(&temporary_path, path) {
        if had_previous {
            let _ = fs::rename(&backup_path, path);
        }
        let _ = fs::remove_file(&temporary_path);
        return Err(error.to_string());
    }
    if had_previous {
        fs::remove_file(backup_path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn legacy_database_settings(conn: &Connection) -> Result<HashMap<String, String>, String> {
    let has_legacy_settings: bool = conn
        .query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'vora_settings'
             )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !has_legacy_settings {
        return Ok(HashMap::new());
    }

    let mut statement = conn
        .prepare("SELECT key, value FROM vora_settings")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<HashMap<_, _>>>()
        .map_err(|error| error.to_string())
}

pub fn initialize_settings_file(
    conn: &Connection,
    database_path: &Path,
    seed_if_missing: Option<&HashMap<String, String>>,
) -> Result<HashMap<String, String>, String> {
    let settings_path = settings_path_for_database(database_path)?;
    let persisted = read_settings_file(&settings_path)?;
    let legacy_settings = legacy_database_settings(conn)?;
    let mut settings = persisted
        .clone()
        .or_else(|| (!legacy_settings.is_empty()).then(|| legacy_settings.clone()))
        .or_else(|| seed_if_missing.cloned())
        .unwrap_or_default();

    if persisted.is_some() {
        for (key, value) in &legacy_settings {
            settings.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }

    if persisted.as_ref() != Some(&settings) {
        write_settings_file(&settings_path, &settings)?;
    }
    conn.execute_batch("DROP TABLE IF EXISTS vora_settings")
        .map_err(|error| error.to_string())?;
    Ok(settings)
}

pub fn replace_settings_for_database(
    database_path: &Path,
    settings: HashMap<String, String>,
) -> Result<(), String> {
    if settings.keys().any(|key| key.trim().is_empty()) {
        return Err("Setting key cannot be empty".to_string());
    }
    write_settings_file(&settings_path_for_database(database_path)?, &settings)
}

pub fn read_settings_for_database(database_path: &Path) -> Result<HashMap<String, String>, String> {
    let path = settings_path_for_database(database_path)?;
    read_settings_file(&path)?.ok_or_else(|| format!("Settings file {} is missing", path.display()))
}

fn active_settings_path(state: &AppState) -> Result<PathBuf, String> {
    let database_path = state
        .db_path
        .lock()
        .map_err(|error| error.to_string())?
        .clone()
        .ok_or_else(|| "Active database path is unavailable".to_string())?;
    settings_path_for_database(Path::new(&database_path))
}

fn temporary_sibling(path: &Path, suffix: &str) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Database destination directory is unavailable".to_string())?;
    let name = path
        .file_name()
        .ok_or_else(|| "Database destination filename is unavailable".to_string())?
        .to_string_lossy();
    Ok(parent.join(format!(".{name}.{}.{}", uuid::Uuid::now_v7(), suffix)))
}

fn validate_imported_database(path: &Path) -> Result<(), String> {
    let conn = Connection::open(path).map_err(|error| error.to_string())?;
    let integrity: String = conn
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if integrity != "ok" {
        return Err(format!(
            "Backup database failed integrity check: {integrity}"
        ));
    }
    let has_workspace_data: bool = conn
        .query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name IN ('mind_boards', 'objects', 'vora_items')
             )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !has_workspace_data {
        return Err("Selected file is not a Vora database backup".to_string());
    }
    super::portable_db::initialize_workspace_schema(&conn).map_err(|error| error.to_string())?;
    conn.pragma_update(None, "journal_mode", "DELETE")
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn init_vora_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(PORTABLE_SCHEMA)
}

#[tauri::command]
pub fn save_setting(state: State<AppState>, key: String, value: String) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("Setting key cannot be empty".to_string());
    }
    let path = active_settings_path(&state)?;
    let mut settings = read_settings_file(&path)?
        .ok_or_else(|| format!("Settings file {} is missing", path.display()))?;
    settings.insert(key, value);
    write_settings_file(&path, &settings)
}

#[tauri::command]
pub fn get_setting(state: State<AppState>, key: String) -> Result<Option<String>, String> {
    let path = active_settings_path(&state)?;
    let settings = read_settings_file(&path)?
        .ok_or_else(|| format!("Settings file {} is missing", path.display()))?;
    Ok(settings.get(&key).cloned())
}

#[tauri::command]
pub fn get_all_settings(state: State<AppState>) -> Result<Value, String> {
    let path = active_settings_path(&state)?;
    let settings = read_settings_file(&path)?
        .ok_or_else(|| format!("Settings file {} is missing", path.display()))?;
    serde_json::to_value(settings).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_setting(state: State<AppState>, key: String) -> Result<(), String> {
    let path = active_settings_path(&state)?;
    let mut settings = read_settings_file(&path)?
        .ok_or_else(|| format!("Settings file {} is missing", path.display()))?;
    settings.remove(&key);
    write_settings_file(&path, &settings)
}

#[tauri::command]
pub fn replace_all_settings(
    state: State<AppState>,
    settings: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let database_path = state
        .db_path
        .lock()
        .map_err(|error| error.to_string())?
        .clone()
        .ok_or_else(|| "Active database path is unavailable".to_string())?;
    replace_settings_for_database(Path::new(&database_path), settings)
}

fn now_ms() -> Result<i64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as i64)
}

fn validate_item_type(item_type: &str) -> Result<(), String> {
    match item_type {
        "board" | "chart" | "library" => Ok(()),
        _ => Err("Item type must be board, chart, or library".to_string()),
    }
}

#[tauri::command]
pub fn save_vora_item(
    state: State<AppState>,
    id: String,
    item_type: String,
    name: String,
    data: String,
) -> Result<(), String> {
    validate_item_type(&item_type)?;
    let _: Value = serde_json::from_str(&data).map_err(|error| error.to_string())?;
    let now = now_ms()?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT INTO vora_items (id, type, name, data, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)
         ON CONFLICT(id) DO UPDATE SET
             type = excluded.type,
             name = excluded.name,
             data = excluded.data,
             updated_at = excluded.updated_at",
        params![id, item_type, name, data, now],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_vora_items(state: State<AppState>, item_type: String) -> Result<Vec<Value>, String> {
    validate_item_type(&item_type)?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let mut statement = conn
        .prepare(
            "SELECT id, type, name, data, created_at, updated_at
             FROM vora_items WHERE type = ?1 ORDER BY updated_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([item_type], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(|error| error.to_string())?;

    rows.map(|row| {
        let (id, item_type, name, data, created_at, updated_at) =
            row.map_err(|error| error.to_string())?;
        let data: Value = serde_json::from_str(&data).map_err(|error| error.to_string())?;
        Ok(serde_json::json!({
            "id": id,
            "type": item_type,
            "name": name,
            "data": data,
            "created_at": created_at,
            "updated_at": updated_at,
        }))
    })
    .collect()
}

#[tauri::command]
pub fn save_vora_snapshot(
    state: State<AppState>,
    id: String,
    chart_id: String,
    name: String,
    data: String,
) -> Result<(), String> {
    let _: Value = serde_json::from_str(&data).map_err(|error| error.to_string())?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let now = now_ms()?;
    tx.execute(
        "INSERT INTO vora_items (id, type, name, data, created_at, updated_at)
         VALUES (?1, 'chart', ?2, '{}', ?3, ?3)
         ON CONFLICT(id) DO NOTHING",
        params![chart_id, name, now],
    )
    .map_err(|error| error.to_string())?;
    tx.execute(
        "INSERT INTO vora_snapshots (id, chart_id, name, data, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
             chart_id = excluded.chart_id,
             name = excluded.name,
             data = excluded.data",
        params![id, chart_id, name, data, now],
    )
    .map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_vora_snapshots(state: State<AppState>, chart_id: String) -> Result<Vec<Value>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let mut statement = conn
        .prepare(
            "SELECT id, chart_id, name, data, created_at
             FROM vora_snapshots WHERE chart_id = ?1 ORDER BY created_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([chart_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|error| error.to_string())?;

    rows.map(|row| {
        let (id, chart_id, name, data, created_at) = row.map_err(|error| error.to_string())?;
        let snapshot: Value = serde_json::from_str(&data).map_err(|error| error.to_string())?;
        let _ = (id, chart_id, name, created_at);
        Ok(snapshot)
    })
    .collect()
}

#[tauri::command]
pub fn get_all_vora_snapshots(state: State<AppState>) -> Result<Vec<Value>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let mut statement = conn
        .prepare("SELECT data FROM vora_snapshots ORDER BY created_at DESC")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    rows.map(|row| {
        let data = row.map_err(|error| error.to_string())?;
        serde_json::from_str(&data).map_err(|error| error.to_string())
    })
    .collect()
}

#[tauri::command]
pub fn replace_vora_snapshots(
    state: State<AppState>,
    snapshots: Vec<String>,
) -> Result<(), String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    tx.execute("DELETE FROM vora_snapshots", [])
        .map_err(|error| error.to_string())?;
    let now = now_ms()?;

    for raw_snapshot in snapshots {
        let snapshot: Value =
            serde_json::from_str(&raw_snapshot).map_err(|error| error.to_string())?;
        let id = snapshot
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "Snapshot is missing its id".to_string())?;
        let name = snapshot
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| "Snapshot is missing its name".to_string())?;
        let chart = snapshot
            .get("chart")
            .ok_or_else(|| "Snapshot is missing chart metadata".to_string())?;
        let symbol = chart
            .get("symbol")
            .and_then(Value::as_str)
            .ok_or_else(|| "Snapshot chart is missing its symbol".to_string())?;
        let interval = chart
            .get("interval")
            .and_then(Value::as_str)
            .ok_or_else(|| "Snapshot chart is missing its interval".to_string())?;
        let chart_id = format!("chart:{symbol}:{interval}");
        tx.execute(
            "INSERT INTO vora_items (id, type, name, data, created_at, updated_at)
             VALUES (?1, 'chart', ?2, ?3, ?4, ?4)
             ON CONFLICT(id) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at",
            params![chart_id, symbol, chart.to_string(), now],
        )
        .map_err(|error| error.to_string())?;
        tx.execute(
            "INSERT INTO vora_snapshots (id, chart_id, name, data, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, chart_id, name, raw_snapshot, now],
        )
        .map_err(|error| error.to_string())?;
    }

    tx.commit().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_vora_snapshot(state: State<AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    conn.execute("DELETE FROM vora_snapshots WHERE id = ?1", [id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn clear_vora_snapshots(state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    conn.execute("DELETE FROM vora_snapshots", [])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_current_db_path(app: AppHandle, state: State<AppState>) -> Result<String, String> {
    if let Some(path) = state
        .db_path
        .lock()
        .map_err(|error| error.to_string())?
        .as_ref()
    {
        return Ok(path.clone());
    }
    app.path()
        .app_data_dir()
        .map(|directory| {
            directory
                .join("vora_data.vora")
                .to_string_lossy()
                .into_owned()
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_settings_file_path(state: State<AppState>) -> Result<String, String> {
    Ok(active_settings_path(&state)?.to_string_lossy().into_owned())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsImportSummary {
    pub added: usize,
    pub already_present: usize,
}

fn merge_missing_settings(
    mut current: HashMap<String, String>,
    imported: HashMap<String, String>,
) -> (HashMap<String, String>, SettingsImportSummary) {
    let mut summary = SettingsImportSummary {
        added: 0,
        already_present: 0,
    };
    for (key, value) in imported {
        if let std::collections::hash_map::Entry::Vacant(entry) = current.entry(key) {
            entry.insert(value);
            summary.added += 1;
        } else {
            summary.already_present += 1;
        }
    }
    (current, summary)
}

fn require_extension(path: &str, extension: &str, label: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path.trim());
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
    {
        return Err(format!("{label} files must use the .{extension} extension"));
    }
    Ok(path)
}

#[tauri::command]
pub fn export_database_backup(state: State<AppState>, backup_path: String) -> Result<(), String> {
    let destination = require_extension(&backup_path, "vora", "Database backup")?;
    let active_path = state
        .db_path
        .lock()
        .map_err(|error| error.to_string())?
        .clone()
        .ok_or_else(|| "Active database path is unavailable".to_string())?;
    let active_path = PathBuf::from(active_path);
    if destination == active_path {
        return Err("Choose a different path from the active workspace database".to_string());
    }
    let temporary_path = temporary_sibling(&destination, "export")?;
    let export_result = (|| {
        let source = state.db.lock().map_err(|error| error.to_string())?;
        create_database_backup(&source, &temporary_path)?;
        drop(source);

        let previous_path = temporary_sibling(&destination, "previous")?;
        let had_previous = destination.exists();
        if had_previous {
            fs::rename(&destination, &previous_path).map_err(|error| error.to_string())?;
        }
        if let Err(error) = fs::rename(&temporary_path, &destination) {
            if had_previous {
                fs::rename(&previous_path, &destination).map_err(|rollback| {
                    format!("{error}; could not restore previous backup: {rollback}")
                })?;
            }
            return Err(error.to_string());
        }
        if had_previous {
            fs::remove_file(previous_path).map_err(|error| error.to_string())?;
        }
        Ok(())
    })();
    if export_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    export_result
}

fn create_database_backup(source: &Connection, destination: &Path) -> Result<(), String> {
    let mut target = Connection::open(destination).map_err(|error| error.to_string())?;
    {
        let backup = rusqlite::backup::Backup::new(source, &mut target)
            .map_err(|error| error.to_string())?;
        backup
            .run_to_completion(5, std::time::Duration::from_millis(100), None)
            .map_err(|error| error.to_string())?;
    }
    target
        .pragma_update(None, "journal_mode", "DELETE")
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn export_settings_json(state: State<AppState>, file_path: String) -> Result<(), String> {
    let destination = require_extension(&file_path, "json", "Settings export")?;
    let settings = read_settings_file(&active_settings_path(&state)?)?
        .ok_or_else(|| "The active workspace settings file is missing".to_string())?;
    write_settings_file(&destination, &settings)
}

#[tauri::command]
pub fn import_settings_json(
    state: State<AppState>,
    file_path: String,
) -> Result<SettingsImportSummary, String> {
    let source_path = require_extension(&file_path, "json", "Settings import")?;
    let imported = read_settings_file(&source_path)?
        .ok_or_else(|| format!("Settings file {} does not exist", source_path.display()))?;
    if imported.keys().any(|key| key.trim().is_empty()) {
        return Err("Settings file contains an empty key".to_string());
    }
    let active_path = active_settings_path(&state)?;
    let mut current = read_settings_file(&active_path)?
        .ok_or_else(|| format!("Settings file {} is missing", active_path.display()))?;
    let (merged, summary) = merge_missing_settings(current, imported);
    current = merged;
    write_settings_file(&active_path, &current)?;
    Ok(summary)
}

#[tauri::command]
pub fn import_database_backup(state: State<AppState>, backup_path: String) -> Result<(), String> {
    let source = require_extension(&backup_path, "vora", "Database backup")?;
    if !source.is_file() {
        return Err("Backup file does not exist".to_string());
    }
    let source_canonical = fs::canonicalize(&source).map_err(|error| error.to_string())?;
    let active_path = state
        .db_path
        .lock()
        .map_err(|error| error.to_string())?
        .clone();
    let active_path = PathBuf::from(
        active_path.ok_or_else(|| "Active database path is unavailable".to_string())?,
    );
    if fs::canonicalize(&active_path).map_err(|error| error.to_string())? == source_canonical {
        return Err("The selected backup is already the active database".to_string());
    }

    let staged_path = temporary_sibling(&active_path, "import")?;
    fs::copy(&source, &staged_path).map_err(|error| error.to_string())?;
    if let Err(error) = validate_imported_database(&staged_path) {
        let _ = fs::remove_file(&staged_path);
        return Err(error);
    }

    let staged_settings_path = settings_path_for_database(&staged_path)?;
    let active_settings_path = settings_path_for_database(&active_path)?;
    let current_settings = read_settings_file(&active_settings_path)?;
    let staged_connection = Connection::open(&staged_path).map_err(|error| error.to_string())?;
    initialize_settings_file(&staged_connection, &staged_path, current_settings.as_ref())?;
    drop(staged_connection);

    let previous_path = temporary_sibling(&active_path, "previous")?;
    let previous_settings_path = temporary_sibling(&active_settings_path, "previous")?;
    let had_previous_settings = active_settings_path.exists();
    let mut active_connection = state.db.lock().map_err(|error| error.to_string())?;
    let placeholder = Connection::open_in_memory().map_err(|error| error.to_string())?;
    let old_connection = std::mem::replace(&mut *active_connection, placeholder);
    drop(old_connection);

    let mut database_moved = false;
    let mut settings_moved = false;
    let mut database_installed = false;
    let mut settings_installed = false;
    let install_result = (|| {
        fs::rename(&active_path, &previous_path).map_err(|error| error.to_string())?;
        database_moved = true;
        if had_previous_settings {
            fs::rename(&active_settings_path, &previous_settings_path)
                .map_err(|error| error.to_string())?;
            settings_moved = true;
        }
        fs::rename(&staged_path, &active_path).map_err(|error| error.to_string())?;
        database_installed = true;
        fs::rename(&staged_settings_path, &active_settings_path)
            .map_err(|error| error.to_string())?;
        settings_installed = true;
        let connection = Connection::open(&active_path).map_err(|error| error.to_string())?;
        initialize_workspace_schema(&connection).map_err(|error| error.to_string())?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(|error| error.to_string())?;
        connection
            .pragma_update(None, "journal_mode", "DELETE")
            .map_err(|error| error.to_string())?;
        Ok(connection)
    })();

    match install_result {
        Ok(connection) => {
            *active_connection = connection;
            fs::remove_file(previous_path).map_err(|error| error.to_string())?;
            if had_previous_settings {
                fs::remove_file(previous_settings_path).map_err(|error| error.to_string())?;
            }
        }
        Err(error) => {
            let db_rollback = if database_installed {
                fs::remove_file(&active_path).and_then(|()| {
                    if database_moved {
                        fs::rename(&previous_path, &active_path)
                    } else {
                        Ok(())
                    }
                })
            } else if database_moved {
                fs::rename(&previous_path, &active_path)
            } else {
                Ok(())
            };
            let settings_rollback = if settings_installed {
                fs::remove_file(&active_settings_path).and_then(|()| {
                    if settings_moved {
                        fs::rename(&previous_settings_path, &active_settings_path)
                    } else {
                        Ok(())
                    }
                })
            } else if settings_moved {
                fs::rename(&previous_settings_path, &active_settings_path)
            } else {
                Ok(())
            };
            let reopened = Connection::open(&active_path);
            match (db_rollback, settings_rollback, reopened) {
                (Ok(()), Ok(()), Ok(connection)) => {
                    *active_connection = connection;
                    let _ = fs::remove_file(&staged_path);
                    let _ = fs::remove_file(&staged_settings_path);
                    if database_installed && !database_moved {
                        let _ = fs::remove_file(&active_path);
                    }
                    return Err(error);
                }
                (db_result, settings_result, connection_result) => {
                    return Err(format!(
                        "{error}; restore rollback failed (database: {}, settings: {}, reopen: {})",
                        db_result
                            .err()
                            .map(|cause| cause.to_string())
                            .unwrap_or_else(|| "ok".to_string()),
                        settings_result
                            .err()
                            .map(|cause| cause.to_string())
                            .unwrap_or_else(|| "ok".to_string()),
                        connection_result
                            .err()
                            .map(|cause| cause.to_string())
                            .unwrap_or_else(|| "ok".to_string())
                    ));
                }
            }
        }
    }
    Ok(())
}

pub fn initialize_workspace_schema(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    super::canvas_db::init_db(conn)?;
    super::task_flow::init_schema(conn)?;
    super::journal::init_schema(conn)?;
    super::questions::init_schema(conn)?;
    super::hypothesis::init_schema(conn)?;
    crate::market::db::init_schema(conn)?;
    crate::source::db::init_schema(conn)?;
    crate::forge::index::CodeIndex::initialize_schema(conn)?;
    init_vora_schema(conn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        create_database_backup, init_vora_schema, initialize_settings_file, merge_missing_settings,
        migrate_legacy_database_selector_file, replace_settings_for_database,
        settings_path_for_database, validate_imported_database,
    };
    use rusqlite::Connection;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    fn temporary_database_path() -> PathBuf {
        std::env::temp_dir().join(format!("vora-test-{}.vora", uuid::Uuid::now_v7()))
    }

    #[test]
    fn full_database_backup_captures_database_rows() {
        let source = Connection::open_in_memory().expect("open source database");
        source
            .execute_batch(
                "CREATE TABLE authored_data (id TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO authored_data VALUES ('item-1', 'saved');",
            )
            .expect("create source data");
        let backup_path = temporary_database_path();

        create_database_backup(&source, &backup_path).expect("create complete backup");

        let backup = Connection::open(&backup_path).expect("open database backup");
        let value: String = backup
            .query_row(
                "SELECT value FROM authored_data WHERE id = 'item-1'",
                [],
                |row| row.get(0),
            )
            .expect("read backed up row");
        assert_eq!(value, "saved");
        drop(backup);
        fs::remove_file(backup_path).expect("remove backup");
    }

    #[test]
    fn settings_import_adds_only_missing_keys() {
        let current = HashMap::from([
            ("shared".to_string(), "local-value".to_string()),
            ("local-only".to_string(), "keep".to_string()),
        ]);
        let imported = HashMap::from([
            ("shared".to_string(), "imported-value".to_string()),
            ("imported-only".to_string(), "add".to_string()),
        ]);

        let (merged, summary) = merge_missing_settings(current, imported);

        assert_eq!(summary.added, 1);
        assert_eq!(summary.already_present, 1);
        assert_eq!(
            merged.get("shared").map(String::as_str),
            Some("local-value")
        );
        assert_eq!(merged.get("imported-only").map(String::as_str), Some("add"));
        assert_eq!(merged.get("local-only").map(String::as_str), Some("keep"));
    }

    #[test]
    fn legacy_selector_for_fixed_database_is_removed() {
        let directory =
            std::env::temp_dir().join(format!("vora-selector-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&directory).expect("create selector test directory");
        let database_path = directory.join("vora_data.vora");
        let selector_path = directory.join("database.json");
        Connection::open(&database_path).expect("create fixed database");
        fs::write(
            &selector_path,
            serde_json::to_vec(&serde_json::json!({
                "path": database_path.to_string_lossy()
            }))
            .expect("serialize selector"),
        )
        .expect("write legacy selector");

        migrate_legacy_database_selector_file(&selector_path, &database_path)
            .expect("remove selector for fixed database");

        assert!(database_path.is_file());
        assert!(!selector_path.exists());
        fs::remove_dir_all(directory).expect("remove selector test directory");
    }

    #[test]
    fn legacy_selected_database_moves_to_fixed_location_with_its_settings() {
        let directory =
            std::env::temp_dir().join(format!("vora-selector-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&directory).expect("create selector test directory");
        let fixed_path = directory.join("vora_data.vora");
        let selected_path = directory.join("selected.vora");
        let selector_path = directory.join("database.json");
        let source = Connection::open(&selected_path).expect("create selected database");
        super::initialize_workspace_schema(&source).expect("initialize selected database");
        let settings = HashMap::from([("theme".to_string(), "dark".to_string())]);
        super::initialize_settings_file(&source, &selected_path, Some(&settings))
            .expect("initialize selected settings");
        source
            .execute(
                "INSERT INTO vora_items (id, type, name, data, created_at, updated_at)
                 VALUES ('board-1', 'board', 'Board', '{}', 1, 1)",
                [],
            )
            .expect("insert selected workspace data");
        drop(source);
        fs::write(
            &selector_path,
            serde_json::to_vec(&serde_json::json!({
                "path": selected_path.to_string_lossy()
            }))
            .expect("serialize selector"),
        )
        .expect("write legacy selector");

        migrate_legacy_database_selector_file(&selector_path, &fixed_path)
            .expect("migrate selected database to fixed location");

        let migrated = Connection::open(&fixed_path).expect("open migrated database");
        let item_count: i64 = migrated
            .query_row("SELECT COUNT(*) FROM vora_items", [], |row| row.get(0))
            .expect("count migrated workspace data");
        assert_eq!(item_count, 1);
        let migrated_settings =
            super::read_settings_for_database(&fixed_path).expect("read migrated settings");
        assert_eq!(migrated_settings, settings);
        assert!(!selector_path.exists());
        assert!(selected_path.is_file());
        drop(migrated);
        fs::remove_dir_all(directory).expect("remove selector test directory");
    }

    #[test]
    fn conflicting_legacy_database_is_not_overwritten_or_unselected() {
        let directory =
            std::env::temp_dir().join(format!("vora-selector-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&directory).expect("create selector test directory");
        let fixed_path = directory.join("vora_data.vora");
        let selected_path = directory.join("other.vora");
        let selector_path = directory.join("database.json");
        Connection::open(&fixed_path).expect("create fixed database");
        Connection::open(&selected_path).expect("create selected database");
        fs::write(
            &selector_path,
            serde_json::to_vec(&serde_json::json!({
                "path": selected_path.to_string_lossy()
            }))
            .expect("serialize selector"),
        )
        .expect("write legacy selector");

        assert!(migrate_legacy_database_selector_file(&selector_path, &fixed_path).is_err());

        assert!(fixed_path.is_file());
        assert!(selected_path.is_file());
        assert!(selector_path.is_file());
        fs::remove_dir_all(directory).expect("remove selector test directory");
    }

    #[test]
    fn portable_schema_creates_snapshot_foreign_key() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");
        init_vora_schema(&conn).expect("initialize portable schema");

        let settings_table_exists: bool = conn
            .query_row(
                "SELECT EXISTS(
                     SELECT 1 FROM sqlite_master
                     WHERE type = 'table' AND name = 'vora_settings'
                 )",
                [],
                |row| row.get(0),
            )
            .expect("check settings table");
        assert!(!settings_table_exists);

        let missing_parent = conn.execute(
            "INSERT INTO vora_snapshots (id, chart_id, name, data, created_at)
             VALUES ('snapshot-1', 'missing-chart', 'Snapshot', '{}', 1)",
            [],
        );
        assert!(missing_parent.is_err());

        conn.execute(
            "INSERT INTO vora_items (id, type, name, data, created_at, updated_at)
             VALUES ('chart-1', 'chart', 'Chart', '{}', 1, 1)",
            [],
        )
        .expect("insert chart parent");
        conn.execute(
            "INSERT INTO vora_snapshots (id, chart_id, name, data, created_at)
             VALUES ('snapshot-1', 'chart-1', 'Snapshot', '{}', 1)",
            [],
        )
        .expect("insert snapshot");
    }

    #[test]
    fn portable_schema_can_be_initialized_repeatedly_without_a_settings_table() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        init_vora_schema(&conn).expect("initialize portable schema");
        init_vora_schema(&conn).expect("reinitialize portable schema");

        let settings_table_exists: bool = conn
            .query_row(
                "SELECT EXISTS(
                     SELECT 1 FROM sqlite_master
                     WHERE type = 'table' AND name = 'vora_settings'
                 )",
                [],
                |row| row.get(0),
            )
            .expect("check settings table");
        assert!(!settings_table_exists);
    }

    #[test]
    fn companion_settings_win_conflicts_and_fill_missing_legacy_keys() {
        let path = temporary_database_path();
        let conn = Connection::open(&path).expect("open test database");
        init_vora_schema(&conn).expect("initialize Vora schema");
        conn.execute_batch(
            "CREATE TABLE vora_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO vora_settings VALUES ('shared', 'legacy');
             INSERT INTO vora_settings VALUES ('legacy_only', 'preserved');",
        )
        .expect("create legacy settings");

        let companion_path = settings_path_for_database(&path).expect("derive settings path");
        let companion = HashMap::from([("shared".to_string(), "json".to_string())]);
        fs::write(
            &companion_path,
            serde_json::to_vec(&companion).expect("serialize settings"),
        )
        .expect("write settings sidecar");

        let settings = initialize_settings_file(&conn, &path, None).expect("migrate settings");
        assert_eq!(settings.get("shared").map(String::as_str), Some("json"));
        assert_eq!(
            settings.get("legacy_only").map(String::as_str),
            Some("preserved")
        );
        let legacy_table_exists: bool = conn
            .query_row(
                "SELECT EXISTS(
                     SELECT 1 FROM sqlite_master
                     WHERE type = 'table' AND name = 'vora_settings'
                 )",
                [],
                |row| row.get(0),
            )
            .expect("check legacy table");
        assert!(!legacy_table_exists);

        drop(conn);
        fs::remove_file(companion_path).expect("remove settings file");
        fs::remove_file(path).expect("remove test database");
    }

    #[test]
    fn legacy_database_settings_migrate_when_no_companion_exists() {
        let path = temporary_database_path();
        let conn = Connection::open(&path).expect("open test database");
        init_vora_schema(&conn).expect("initialize Vora schema");
        conn.execute_batch(
            "CREATE TABLE vora_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO vora_settings VALUES ('chart.interval', '1h');",
        )
        .expect("create legacy settings");

        let settings =
            initialize_settings_file(&conn, &path, None).expect("migrate legacy settings");
        assert_eq!(
            settings.get("chart.interval").map(String::as_str),
            Some("1h")
        );

        drop(conn);
        let companion_path = settings_path_for_database(&path).expect("derive settings path");
        fs::remove_file(companion_path).expect("remove settings file");
        fs::remove_file(path).expect("remove test database");
    }

    #[test]
    fn invalid_companion_settings_do_not_drop_legacy_data() {
        let path = temporary_database_path();
        let conn = Connection::open(&path).expect("open test database");
        init_vora_schema(&conn).expect("initialize Vora schema");
        conn.execute_batch(
            "CREATE TABLE vora_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO vora_settings VALUES ('important', 'keep');",
        )
        .expect("create legacy settings");
        let companion_path = settings_path_for_database(&path).expect("derive settings path");
        fs::write(&companion_path, "{invalid json").expect("write invalid settings");

        assert!(initialize_settings_file(&conn, &path, None).is_err());
        let preserved_value: String = conn
            .query_row(
                "SELECT value FROM vora_settings WHERE key = 'important'",
                [],
                |row| row.get(0),
            )
            .expect("legacy value remains");
        assert_eq!(preserved_value, "keep");

        drop(conn);
        fs::remove_file(companion_path).expect("remove settings file");
        fs::remove_file(path).expect("remove test database");
    }

    #[test]
    fn missing_companion_settings_are_seeded_and_written_atomically() {
        let path = temporary_database_path();
        let conn = Connection::open(&path).expect("open test database");
        init_vora_schema(&conn).expect("initialize Vora schema");
        let seed = HashMap::from([("theme".to_string(), "dark".to_string())]);
        let settings = initialize_settings_file(&conn, &path, Some(&seed)).expect("seed settings");
        assert_eq!(settings, seed);

        let replacement = HashMap::from([("theme".to_string(), "light".to_string())]);
        replace_settings_for_database(&path, replacement.clone()).expect("replace settings");
        let companion_path = settings_path_for_database(&path).expect("derive settings path");
        let persisted = super::read_settings_file(&companion_path)
            .expect("read settings")
            .expect("settings file exists");
        assert_eq!(persisted, replacement);

        drop(conn);
        fs::remove_file(companion_path).expect("remove settings file");
        fs::remove_file(path).expect("remove test database");
    }

    #[test]
    fn import_validation_rejects_non_sqlite_files() {
        let path = temporary_database_path();
        fs::write(&path, b"not a SQLite database").expect("write invalid database file");

        assert!(validate_imported_database(&path).is_err());

        fs::remove_file(path).expect("remove invalid test database");
    }

    #[test]
    fn import_validation_accepts_vora_databases() {
        let path = temporary_database_path();
        let conn = Connection::open(&path).expect("open test database");
        init_vora_schema(&conn).expect("initialize Vora schema");
        drop(conn);

        validate_imported_database(&path).expect("validate Vora database");

        fs::remove_file(path).expect("remove test database");
    }
}
