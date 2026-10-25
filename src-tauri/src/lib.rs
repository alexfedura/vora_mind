extern crate self as vora_lib;

pub mod forge;
pub mod mind;

use crate::mind::canvas_db::{AppState, BoardSummary, EdgeInput, NodeInput, SnapshotSummary};
use crate::mind::resolver::{ResolvedSnapshot, SnapshotResolver};
use rusqlite::{Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};
use tauri_plugin_updater::UpdaterExt;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdaterDownloadProgress {
    downloaded_bytes: u64,
    content_length: Option<u64>,
}

#[tauri::command]
async fn install_update_with_helper(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let updater = app.updater().map_err(|error| error.to_string())?;
    let Some(update) = updater.check().await.map_err(|error| error.to_string())? else {
        return Ok(None);
    };
    let update_version = update.version.clone();
    let progress_app = app.clone();
    let mut downloaded_bytes = 0_u64;
    let installer_bytes = update
        .download(
            |chunk_length, content_length| {
                downloaded_bytes += chunk_length as u64;
                if let Err(error) = progress_app.emit(
                    "vora-updater-progress",
                    UpdaterDownloadProgress {
                        downloaded_bytes,
                        content_length,
                    },
                ) {
                    eprintln!("Could not report updater download progress: {error}");
                }
            },
            || {},
        )
        .await
        .map_err(|error| error.to_string())?;

    #[cfg(windows)]
    {
        launch_update_helper(&app, &installer_bytes)?;
        app.exit(0);
    }

    #[cfg(not(windows))]
    {
        update.install(installer_bytes).map_err(|error| error.to_string())?;
    }

    Ok(Some(update_version))
}

#[cfg(windows)]
fn launch_update_helper(app: &tauri::AppHandle, installer_bytes: &[u8]) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Stdio;
    use tauri::path::BaseDirectory;
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS};

    let app_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("updater");
    std::fs::create_dir_all(&app_data_dir).map_err(|error| error.to_string())?;

    let update_id = uuid::Uuid::now_v7();
    let helper_source = app
        .path()
        .resolve("vora_updater_helper.exe", BaseDirectory::Resource)
        .map_err(|error| error.to_string())?;
    let helper_path = app_data_dir.join(format!("vora_updater_helper-{update_id}.exe"));
    std::fs::copy(&helper_source, &helper_path).map_err(|error| {
        format!(
            "Could not copy updater helper from {}: {error}",
            helper_source.display()
        )
    })?;

    let installer_extension = if installer_bytes.starts_with(b"MZ") {
        "exe"
    } else {
        "msi"
    };
    let installer_path = app_data_dir.join(format!("VoraMind-update-{update_id}.{installer_extension}"));
    std::fs::write(&installer_path, installer_bytes).map_err(|error| error.to_string())?;

    let app_executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let process_id = std::process::id();
    let ready_path = app_data_dir.join(format!("updater-helper-{update_id}.ready"));
    let log_path = app_data_dir.join("updater-helper.log");

    let mut helper = Command::new(&helper_path)
        .arg(&installer_path)
        .arg(process_id.to_string())
        .arg(&app_executable)
        .arg(&ready_path)
        .arg(&log_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .map_err(|error| format!("Could not start detached updater helper: {error}"))?;

    let handshake_deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        if ready_path.is_file() {
            let _ = std::fs::remove_file(&ready_path);
            return Ok(());
        }
        if let Some(status) = helper
            .try_wait()
            .map_err(|error| format!("Could not check updater helper startup: {error}"))?
        {
            return Err(format!(
                "Updater helper exited before attaching to the app process ({status})."
            ));
        }
        if std::time::Instant::now() >= handshake_deadline {
            return Err("Updater helper did not confirm its process wait within 15 seconds.".to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CardDto {
    pub id: String,
    pub title: String,
    pub content: String,
    pub version: String,
    pub deleted_at: Option<i64>,
    pub library_source_card_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GitCommitDto {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TestResultDto {
    pub success: bool,
    pub output: String,
    pub exit_code: Option<i32>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LineageHistoryEntryDto {
    pub snapshot_id: String,
    pub symbol_id: String,
    pub confidence: Option<f64>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TransactionExecutionResultDto {
    pub is_committed: bool,
    pub commit_message: String,
    pub patched_files: Vec<String>,
    pub changes_count: usize,
    pub file_moves_count: usize,
    pub snapshot_id: Option<String>,
}

fn active_database_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let state = app.state::<AppState>();
    if let Some(path) = state
        .db_path
        .lock()
        .map_err(|error| error.to_string())?
        .as_ref()
    {
        return Ok(PathBuf::from(path));
    }
    app.path()
        .app_data_dir()
        .map(|directory| directory.join("vora_data.vora"))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_all_boards(state: State<AppState>) -> Result<Vec<BoardSummary>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::canvas_db::ensure_board_version_baselines(&conn).map_err(|e| e.to_string())?;
    mind::canvas_db::get_all_boards(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_board(state: State<AppState>, title: String) -> Result<BoardSummary, String> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 80 {
        return Err("Board title must contain between 1 and 80 characters".to_string());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as i64;
    let id = format!("board_{}", uuid::Uuid::now_v7());
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::canvas_db::create_board(&mut conn, title, now, &id).map_err(|e| e.to_string())
}

fn task_flow_now() -> Result<i64, String> {
    let milliseconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    i64::try_from(milliseconds).map_err(|error| error.to_string())
}

#[tauri::command]
fn get_task_flow_items(
    state: State<AppState>,
) -> Result<Vec<mind::task_flow::TaskFlowItem>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::task_flow::list_tasks(&conn).map_err(|error| error.to_string())
}

#[tauri::command]
fn create_task_flow_item(
    state: State<AppState>,
    title: String,
) -> Result<mind::task_flow::TaskFlowItem, String> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err("Task title must contain between 1 and 200 characters".to_string());
    }
    let id = format!("task_{}", uuid::Uuid::now_v7());
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::task_flow::create_task(&mut conn, &id, title, task_flow_now()?)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_task_flow_item_completed(
    state: State<AppState>,
    id: String,
    completed: bool,
) -> Result<mind::task_flow::TaskFlowItem, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::task_flow::set_completed(&conn, &id, completed, task_flow_now()?)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn reorder_task_flow_items(
    state: State<AppState>,
    ids: Vec<String>,
) -> Result<Vec<mind::task_flow::TaskFlowItem>, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::task_flow::reorder_tasks(&mut conn, &ids, task_flow_now()?)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_task_flow_item(state: State<AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::task_flow::delete_task(&conn, &id).map_err(|error| error.to_string())
}

#[tauri::command]
fn get_journal_entries(
    state: State<AppState>,
    start_date: String,
    end_date: String,
) -> Result<Vec<mind::journal::JournalEntry>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::journal::list_entries(&conn, &start_date, &end_date).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_journal_entry(
    state: State<AppState>,
    date: String,
    content: String,
) -> Result<mind::journal::JournalEntry, String> {
    let now = task_flow_now()?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::journal::save_entry(&conn, &date, &content, now).map_err(|error| error.to_string())
}

#[tauri::command]
fn get_board_versions(
    state: State<AppState>,
    board_id: String,
) -> Result<Vec<mind::canvas_db::BoardVersionSummary>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::canvas_db::get_board_versions(&conn, &board_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_board_version(
    state: State<AppState>,
    board_id: String,
    increment: String,
    snapshot: mind::canvas_db::BoardVersionSnapshot,
) -> Result<mind::canvas_db::BoardVersionSummary, String> {
    if !matches!(increment.as_str(), "major" | "minor" | "patch") {
        return Err("Choose a Major, Minor, or Patch version increment.".to_string());
    }
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::canvas_db::create_board_version(&mut conn, &board_id, &increment, snapshot)
        .map_err(|error| match error {
            rusqlite::Error::InvalidQuery => {
                "That version label already exists on this Board. Choose another increment."
                    .to_string()
            }
            other => other.to_string(),
        })
}

#[tauri::command]
fn checkout_board_version(
    state: State<AppState>,
    board_id: String,
    version_id: String,
) -> Result<mind::canvas_db::BoardVersionSnapshot, String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::canvas_db::checkout_board_version(&mut conn, &board_id, &version_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_library_item_groups(
    state: State<AppState>,
) -> Result<Vec<mind::library_groups::LibraryItemGroup>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::library_groups::list_item_groups(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_library_item_group_versions(
    state: State<AppState>,
    group_id: String,
) -> Result<Vec<mind::library_groups::LibraryItemGroupVersion>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::library_groups::list_item_group_versions(&conn, &group_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_library_item_group_usages(
    state: State<AppState>,
    group_id: String,
) -> Result<Vec<mind::library_groups::LibraryItemGroupUsage>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::library_groups::list_item_group_usages(&conn, &group_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_library_item_group(
    state: State<AppState>,
    name: String,
    version: String,
    snapshot: mind::library_groups::LibraryItemGroupSnapshot,
) -> Result<mind::library_groups::LibraryItemGroup, String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::library_groups::create_item_group(&mut conn, &name, &version, snapshot)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn commit_library_item_group_version(
    state: State<AppState>,
    group_id: String,
    version: String,
    snapshot: mind::library_groups::LibraryItemGroupSnapshot,
) -> Result<mind::library_groups::LibraryItemGroup, String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::library_groups::commit_item_group_version(&mut conn, &group_id, &version, snapshot)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn move_library_item_group_to_trash(
    state: State<AppState>,
    group_id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::library_groups::move_item_group_to_trash(&conn, &group_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn restore_library_item_group(
    state: State<AppState>,
    group_id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::library_groups::restore_item_group(&conn, &group_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn rename_board(state: State<AppState>, board_id: String, title: String) -> Result<(), String> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 80 {
        return Err("Board title must contain between 1 and 80 characters".to_string());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as i64;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::canvas_db::rename_board(&conn, &board_id, title, now).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_board(state: State<AppState>, board_id: String) -> Result<(), String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::canvas_db::delete_board(&mut conn, &board_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_board_to_file(
    app: tauri::AppHandle,
    file_path: String,
    browser_storage: mind::backup::BrowserStorageArchive,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::backup::export_database(&mut conn, &file_path, browser_storage)
}

#[tauri::command]
fn read_board_backup_storage(
    app: tauri::AppHandle,
    file_path: String,
) -> Result<Option<mind::backup::BrowserStorageArchive>, String> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::backup::read_browser_storage(&conn, &file_path)
}

#[tauri::command]
fn import_board_from_file(app: tauri::AppHandle, file_path: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let database_path = state
        .db_path
        .lock()
        .map_err(|error| error.to_string())?
        .clone()
        .ok_or_else(|| "Active database path is unavailable".to_string())?;
    let database_path_buf = PathBuf::from(&database_path);
    let previous_settings = mind::portable_db::read_settings_for_database(&database_path_buf)?;
    let mut settings_write_attempted = false;
    let import_result = {
        let mut conn = state.db.lock().map_err(|e| e.to_string())?;
        mind::backup::import_database_with_settings(&mut conn, &file_path, |restored_storage| {
            let settings = restored_storage
                .as_ref()
                .map(|storage| storage.settings.clone().into_iter().collect())
                .unwrap_or_default();
            settings_write_attempted = true;
            mind::portable_db::replace_settings_for_database(&database_path_buf, settings)
        })
    };
    if let Err(error) = import_result {
        if settings_write_attempted {
            if let Err(rollback_error) = mind::portable_db::replace_settings_for_database(
                &database_path_buf,
                previous_settings,
            ) {
                return Err(format!(
                    "{error}; settings rollback also failed: {rollback_error}"
                ));
            }
        }
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
fn get_backup_catalog(app: tauri::AppHandle) -> Result<mind::backup::BackupCatalog, String> {
    let database_path = active_database_path(&app)?;
    let settings = mind::portable_db::read_settings_for_database(&database_path)?;
    let state = app.state::<AppState>();
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::backup::get_backup_catalog(&conn, &settings.into_iter().collect())
}

#[tauri::command]
fn export_selective_package(
    app: tauri::AppHandle,
    file_path: String,
    package_name: String,
    selection: mind::backup::PackageSelection,
) -> Result<(), String> {
    let database_path = active_database_path(&app)?;
    let settings = mind::portable_db::read_settings_for_database(&database_path)?;
    let state = app.state::<AppState>();
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::backup::export_selective_package(
        &conn,
        &file_path,
        &package_name,
        selection,
        &settings.into_iter().collect(),
    )
}

#[tauri::command]
fn inspect_backup_file(
    app: tauri::AppHandle,
    file_path: String,
) -> Result<mind::backup::BackupFileInfo, String> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::backup::inspect_backup_file(&conn, &file_path)
}

#[tauri::command]
fn import_selective_package(
    app: tauri::AppHandle,
    file_path: String,
) -> Result<mind::backup::PackageImportSummary, String> {
    let database_path = active_database_path(&app)?;
    let database_path_buf = database_path.clone();
    let previous_settings = mind::portable_db::read_settings_for_database(&database_path)?;
    let current_settings = previous_settings.iter().map(|(key, value)| (key.clone(), value.clone())).collect();
    let mut settings_write_attempted = false;
    let result = {
        let state = app.state::<AppState>();
        let mut conn = state.db.lock().map_err(|error| error.to_string())?;
        mind::backup::merge_selective_package(
            &mut conn,
            &file_path,
            &current_settings,
            |settings| {
                settings_write_attempted = true;
                mind::portable_db::replace_settings_for_database(
                    &database_path_buf,
                    settings.iter().map(|(key, value)| (key.clone(), value.clone())).collect(),
                )
            },
        )
        .map(|(summary, _)| summary)
    };
    match result {
        Ok(summary) => Ok(summary),
        Err(error) => {
            if settings_write_attempted {
                if let Err(rollback_error) = mind::portable_db::replace_settings_for_database(
                    &database_path,
                    previous_settings,
                ) {
                    return Err(format!("{error}; settings rollback also failed: {rollback_error}"));
                }
            }
            Err(error)
        }
    }
}

#[tauri::command]
fn get_imported_item_origins(
    app: tauri::AppHandle,
) -> Result<Vec<mind::backup::ImportedRecordOrigin>, String> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::backup::get_imported_item_origins(&conn)
}

#[tauri::command]
fn get_all_cards(state: State<AppState>, board_id: String) -> Result<Vec<CardDto>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "
            SELECT o.id,
                   COALESCE(json_extract(v.content, '$.title'), 'Untitled') as title,
                   COALESCE(json_extract(v.content, '$.content'), '') as content,
                   COALESCE(json_extract(v.content, '$.version'), v.version_major || '.' || v.version_minor || '.' || v.version_patch, '0.1.0') as version,
                   o.deleted_at,
                   json_extract(o.metadata, '$.librarySourceCardId')
            FROM objects o
            LEFT JOIN object_versions v ON v.id = (
                SELECT id FROM object_versions
                WHERE object_id = o.id
                ORDER BY created_at DESC
                LIMIT 1
            )
            WHERE o.type = 'card'
              AND o.board_id = ?1
            ORDER BY o.created_at ASC
        ",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([board_id], |r| {
            Ok(CardDto {
                id: r.get(0)?,
                title: r.get(1)?,
                content: r.get(2)?,
                version: r.get(3)?,
                deleted_at: r.get(4)?,
                library_source_card_id: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
fn get_text_links_for_card(
    state: State<AppState>,
    board_id: String,
    card_id: String,
) -> Result<Vec<mind::canvas_db::TextLink>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::canvas_db::get_text_links_for_card(&conn, &board_id, &card_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn create_text_link(
    state: State<AppState>,
    input: mind::canvas_db::TextLinkInput,
) -> Result<mind::canvas_db::TextLink, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as i64;
    let id = format!("text-link-{}", uuid::Uuid::now_v7());
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::canvas_db::create_text_link(&mut conn, input, now, &id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_text_link(state: State<AppState>, link_id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::canvas_db::delete_text_link(&conn, &link_id).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_card_direct(
    state: State<AppState>,
    board_id: String,
    id: String,
    title: String,
    content: String,
    version: String,
    text_change_sets: Vec<mind::canvas_db::TextChangeSet>,
    library_source_card_id: Option<String>,
) -> Result<(), String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as i64;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    mind::canvas_db::apply_text_link_changes(
        &tx,
        &board_id,
        &id,
        &text_change_sets,
        now,
    )
    .map_err(|error| error.to_string())?;

    let board_exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM mind_boards WHERE id = ?1)",
            rusqlite::params![&board_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !board_exists {
        return Err("Board does not exist".to_string());
    }

    let metadata = library_source_card_id.map(|source_card_id| {
        serde_json::json!({ "librarySourceCardId": source_card_id }).to_string()
    });
    tx.execute(
        "INSERT INTO objects (id, type, board_id, created_at, updated_at, deleted_at, metadata) VALUES (?1, 'card', ?2, ?3, ?3, NULL, ?4)
         ON CONFLICT(id) DO UPDATE SET
             updated_at = excluded.updated_at,
             metadata = CASE
                 WHEN excluded.metadata IS NULL THEN objects.metadata
                 ELSE json_patch(COALESCE(objects.metadata, '{}'), excluded.metadata)
             END
         WHERE objects.board_id = excluded.board_id",
        rusqlite::params![&id, &board_id, now, metadata],
    ).map_err(|e| e.to_string())?;
    let belongs_to_board: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM objects WHERE id = ?1 AND board_id = ?2)",
            rusqlite::params![&id, &board_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !belongs_to_board {
        return Err("Card does not belong to the active Board".to_string());
    }

    let ver_id = format!("ver_{}_{}_{}", id, now, uuid::Uuid::now_v7());
    let content_json = serde_json::json!({
        "title": title,
        "content": content,
        "version": version,
    })
    .to_string();

    let draft_version_id: Option<String> = tx
        .query_row(
            "SELECT id FROM object_versions WHERE object_id = ?1 AND reason = 'Auto Save' AND summary = 'Direct edit'
             ORDER BY created_at DESC LIMIT 1",
            rusqlite::params![&id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(existing_version_id) = draft_version_id {
        tx.execute(
            "UPDATE object_versions SET content = ?1, created_at = ?2 WHERE id = ?3",
            rusqlite::params![content_json, now, existing_version_id],
        )
        .map_err(|e| e.to_string())?;
    } else {
        tx.execute(
            "INSERT INTO object_versions (id, object_id, version_major, version_minor, version_patch, reason, summary, content, created_at)
             VALUES (?1, ?2, 0, 1, 0, 'Auto Save', 'Direct edit', ?3, ?4)",
            rusqlite::params![&ver_id, &id, content_json, now],
        ).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn move_card_to_bin(state: State<AppState>, board_id: String, id: String) -> Result<(), String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as i64;

    let changed = tx
        .execute(
            "UPDATE objects SET deleted_at = ?1 WHERE id = ?2 AND board_id = ?3",
            rusqlite::params![now, &id, &board_id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Card does not belong to the active Board".to_string());
    }

    tx.execute(
        "DELETE FROM mind_text_links
         WHERE (source_board_id = ?1 AND source_card_id = ?2)
            OR (target_board_id = ?1 AND target_card_id = ?2)",
        rusqlite::params![&board_id, &id],
    )
    .map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn restore_card_from_bin(
    state: State<AppState>,
    board_id: String,
    id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let changed = conn
        .execute(
            "UPDATE objects SET deleted_at = NULL WHERE id = ?1 AND board_id = ?2",
            rusqlite::params![&id, &board_id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Card does not belong to the active Board".to_string());
    }

    Ok(())
}

#[tauri::command]
fn commit_snapshot(
    state: State<AppState>,
    board_id: String,
    snapshot_name: String,
    parent_snapshot_id: Option<String>,
    reason: Option<String>,
    summary: Option<String>,
    nodes: Vec<NodeInput>,
    edges: Vec<EdgeInput>,
) -> Result<String, String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::canvas_db::save_semantic_snapshot(
        &mut conn,
        board_id,
        snapshot_name,
        parent_snapshot_id,
        reason,
        summary,
        nodes,
        edges,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn resolve_snapshot(
    state: State<AppState>,
    board_id: String,
    snapshot_id: String,
) -> Result<ResolvedSnapshot, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    SnapshotResolver::resolve(&conn, &board_id, &snapshot_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_all_snapshots(
    state: State<AppState>,
    board_id: String,
) -> Result<Vec<SnapshotSummary>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, parent_snapshot_id, depth, is_checkpoint, created_at FROM mind_snapshots WHERE board_id = ?1 ORDER BY created_at ASC, rowid ASC")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([board_id], |r| {
            Ok(SnapshotSummary {
                id: r.get(0)?,
                name: r.get(1)?,
                parent_snapshot_id: r.get(2)?,
                depth: r.get(3)?,
                is_checkpoint: r.get(4)?,
                created_at: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
fn parse_code_ast(file_path: String, content: String) -> Result<serde_json::Value, String> {
    let ast_payload = forge::ast_engine::AstEngine::parse_source_code(&file_path, &content);
    serde_json::to_value(ast_payload).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_project_file(file_path: String) -> Result<String, String> {
    std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file {}: {}", file_path, e))
}

#[tauri::command]
fn get_forge_tools() -> Vec<forge::tools::ToolDefinition> {
    forge::tools::ForgeToolRegistry::get_tool_definitions()
}

#[tauri::command]
fn execute_forge_tool(
    app: tauri::AppHandle,
    tool: String,
    arguments: serde_json::Value,
) -> forge::tools::ToolCallResponse {
    let db_path = active_database_path(&app).unwrap_or_else(|_| {
        app.path()
            .app_data_dir()
            .unwrap_or_default()
            .join("vora_data.vora")
    });
    let mut index_opt = forge::index::CodeIndex::new(db_path).ok();

    forge::tools::ForgeToolRegistry::execute_tool(
        forge::tools::ToolCallRequest { tool, arguments },
        index_opt.as_mut(),
    )
}

#[tauri::command]
fn apply_patch_and_validate(file_path: String, content: String) -> Result<bool, String> {
    let validation_payload = forge::ast_engine::AstEngine::parse_source_code(&file_path, &content);
    if validation_payload.symbols.is_empty() && !content.trim().is_empty() {
        return Err(
            "Post-patch AST validation failed: File contains unparseable blocks.".to_string(),
        );
    }

    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write file {}: {}", file_path, e))?;
    Ok(true)
}

#[tauri::command]
fn create_git_commit(
    file_path: String,
    symbol_name: String,
    reason: String,
) -> Result<String, String> {
    let add_output = Command::new("git")
        .args(["add", &file_path])
        .output()
        .map_err(|e| format!("Failed to run git add: {}", e))?;

    if !add_output.status.success() {
        return Err(String::from_utf8_lossy(&add_output.stderr).to_string());
    }

    let commit_message = format!(
        "refactor(ast): patch symbol '{}' in {}\n\nReason: {}",
        symbol_name, file_path, reason
    );

    let commit_output = Command::new("git")
        .args(["commit", "-m", &commit_message])
        .output()
        .map_err(|e| format!("Failed to run git commit: {}", e))?;

    if !commit_output.status.success() {
        return Err(String::from_utf8_lossy(&commit_output.stderr).to_string());
    }

    Ok(String::from_utf8_lossy(&commit_output.stdout).to_string())
}

#[tauri::command]
fn scan_workspace(directory_path: String) -> Result<serde_json::Value, String> {
    let payload = forge::ast_engine::AstEngine::scan_workspace_directory(&directory_path);
    serde_json::to_value(payload).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_git_commit_history(max_count: usize) -> Result<Vec<GitCommitDto>, String> {
    let output = Command::new("git")
        .args([
            "log",
            &format!("-n{}", max_count),
            "--pretty=format:%H|%h|%s|%an|%ad",
            "--date=iso",
        ])
        .output()
        .map_err(|e| format!("Failed to execute git log: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 5 {
            commits.push(GitCommitDto {
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                message: parts[2].to_string(),
                author: parts[3].to_string(),
                timestamp: parts[4].to_string(),
            });
        }
    }

    Ok(commits)
}

#[tauri::command]
fn run_project_tests(
    test_command: Option<String>,
    working_dir: Option<String>,
) -> Result<TestResultDto, String> {
    let cmd = test_command.unwrap_or_else(|| "cargo test".to_string());

    let mut command = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", &cmd]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", &cmd]);
        c
    };

    if let Some(dir) = working_dir {
        command.current_dir(Path::new(&dir));
    }

    let output = command
        .output()
        .map_err(|e| format!("Failed to execute test command '{}': {}", cmd, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined_output = format!("{}{}", stdout, stderr);

    Ok(TestResultDto {
        success: output.status.success(),
        output: combined_output,
        exit_code: output.status.code(),
    })
}

#[tauri::command]
fn forge_test_candidate_patch(
    file_path: String,
    symbol_id: String,
    new_symbol_code: String,
    test_command: Option<String>,
) -> Result<TestResultDto, String> {
    let original_content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read target file {}: {}", file_path, e))?;

    let patch_res = forge::patcher_v2::SurgicalPatcher::apply_patch(
        &file_path,
        &symbol_id,
        &new_symbol_code,
        true,
    )?;

    if !patch_res.is_syntax_valid {
        return Ok(TestResultDto {
            success: false,
            output: "Candidate patch failed Tree-sitter AST validation before test execution."
                .to_string(),
            exit_code: Some(1),
        });
    }

    let start_byte = patch_res.original_byte_range.0;
    let end_byte = patch_res.original_byte_range.1;

    let patched_content =
        if start_byte <= original_content.len() && end_byte <= original_content.len() {
            let before = &original_content[..start_byte];
            let after = &original_content[end_byte..];
            format!("{}{}{}", before, new_symbol_code, after)
        } else {
            original_content.clone()
        };

    let _ = fs::write(&file_path, &patched_content);
    let result = run_project_tests(test_command, None);
    let _ = fs::write(&file_path, &original_content);

    result
}

#[tauri::command]
fn forge_test_multi_candidate_patches(
    patches: Vec<forge::commit::TransactionPatch>,
    test_command: Option<String>,
) -> Result<TestResultDto, String> {
    let mut original_files_backup: Vec<(String, String)> = Vec::new();

    for patch in &patches {
        let orig = std::fs::read_to_string(&patch.file_path)
            .map_err(|e| format!("Failed to read file {}: {}", patch.file_path, e))?;
        original_files_backup.push((patch.file_path.clone(), orig.clone()));

        let patch_res = forge::patcher_v2::SurgicalPatcher::apply_patch(
            &patch.file_path,
            &patch.symbol_id,
            &patch.new_symbol_code,
            true,
        )?;

        if !patch_res.is_syntax_valid {
            for (path, orig_content) in original_files_backup {
                let _ = fs::write(&path, &orig_content);
            }
            return Ok(TestResultDto {
                success: false,
                output: format!(
                    "Patch for symbol '{}' in file '{}' failed AST syntax validation.",
                    patch.symbol_id, patch.file_path
                ),
                exit_code: Some(1),
            });
        }

        let start_byte = patch_res.original_byte_range.0;
        let end_byte = patch_res.original_byte_range.1;

        let patched_content = if start_byte <= orig.len() && end_byte <= orig.len() {
            let before = &orig[..start_byte];
            let after = &orig[end_byte..];
            format!("{}{}{}", before, patch.new_symbol_code, after)
        } else {
            orig.clone()
        };

        let _ = fs::write(&patch.file_path, &patched_content);
    }

    let result = run_project_tests(test_command, None);

    for (path, orig_content) in original_files_backup {
        let _ = fs::write(&path, &orig_content);
    }

    result
}

#[tauri::command]
fn forge_rollback_to_snapshot(
    app: tauri::AppHandle,
    snapshot_id: String,
) -> Result<Vec<String>, String> {
    let db_path = active_database_path(&app)?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "
            SELECT sf.path_at_snapshot
            FROM snapshot_files sf
            WHERE sf.snapshot_id = ?1
            ",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([&snapshot_id], |r| {
            let path: String = r.get(0)?;
            Ok(path)
        })
        .map_err(|e| e.to_string())?;

    let mut restored_files = Vec::new();
    for r in rows {
        let path = r.map_err(|e| e.to_string())?;
        restored_files.push(path);
    }

    Ok(restored_files)
}

#[tauri::command]
fn resolve_strict_edges(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let db_path = active_database_path(&app)?;
    let index = forge::index::CodeIndex::new(db_path).map_err(|e| e.to_string())?;
    let edges = forge::resolver::SemanticResolver::resolve_strict_semantic_edges(&index)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(edges).map_err(|e| e.to_string())
}

#[tauri::command]
fn simulate_semantic_change(
    app: tauri::AppHandle,
    change: forge::model::SemanticChange,
) -> Result<forge::graph::ImpactAnalysisReport, String> {
    let db_path = active_database_path(&app)?;
    let index = forge::index::CodeIndex::new(db_path).map_err(|e| e.to_string())?;
    let strict_edges = forge::resolver::SemanticResolver::resolve_strict_semantic_edges(&index)
        .map_err(|e| e.to_string())?;
    let mut dep_graph = forge::graph::DependencyGraph::new();
    dep_graph.build_from_strict_semantic_edges(&strict_edges);
    Ok(dep_graph.analyze_semantic_change_impact(&change))
}

#[tauri::command]
fn create_semantic_plan(
    repo_root: String,
    change: forge::model::SemanticChange,
) -> Result<forge::planner::ExecutionPlan, String> {
    let context =
        forge::project::ProjectContext::load_from_repo(&repo_root).map_err(|e| e.to_string())?;
    let planner = forge::planner::DynamicPlanner::new(&context, &repo_root);
    Ok(planner.create_plan_for_semantic_change(&change))
}

#[tauri::command]
fn forge_compute_ast_semantic_diff(
    file_path: String,
    target_symbol_name: String,
    old_code: String,
    new_code: String,
) -> Result<forge::model::SemanticChange, String> {
    Ok(forge::differ::SemanticDiffer::extract_ast_semantic_change(
        &file_path,
        &target_symbol_name,
        &old_code,
        &new_code,
    ))
}

#[tauri::command]
fn forge_repair_and_expand_plan(
    repo_root: String,
    current_plan: forge::planner::ExecutionPlan,
    discovered_symbol_names: Vec<String>,
) -> Result<forge::planner::ExecutionPlan, String> {
    let context =
        forge::project::ProjectContext::load_from_repo(&repo_root).map_err(|e| e.to_string())?;
    let planner = forge::planner::DynamicPlanner::new(&context, &repo_root);
    Ok(planner.repair_and_expand_plan(&current_plan, &discovered_symbol_names))
}

#[tauri::command]
fn forge_find_symbol(
    app: tauri::AppHandle,
    query: String,
) -> Result<Vec<forge::tools::SymbolSearchResult>, String> {
    let db_path = active_database_path(&app)?;
    let index = forge::index::CodeIndex::new(db_path).map_err(|e| e.to_string())?;
    forge::tools::ForgeToolRegistry::find_symbol(&query, &index).map_err(|e| e.to_string())
}

#[tauri::command]
fn forge_get_impact_radius(
    app: tauri::AppHandle,
    symbol_id: String,
) -> Result<forge::graph::ImpactAnalysisReport, String> {
    let db_path = active_database_path(&app)?;
    let index = forge::index::CodeIndex::new(db_path).map_err(|e| e.to_string())?;
    forge::tools::ForgeToolRegistry::get_impact_radius(&symbol_id, &index)
        .map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn forge_execute_patch_transaction(
    app: tauri::AppHandle,
    base_commit: String,
    task_id: String,
    file_path: String,
    base_content: String,
    symbol_id: String,
    new_symbol_code: String,
    intent: String,
) -> Result<TransactionExecutionResultDto, String> {
    let db_path = active_database_path(&app)?;
    let mut index = forge::index::CodeIndex::new(db_path).map_err(|e| e.to_string())?;
    let mut overlay = forge::overlay::CodeOverlayManager::new(&base_commit, &task_id);

    let res = forge::tools::ForgeToolRegistry::execute_atomic_patch_transaction(
        &base_commit,
        &task_id,
        &mut overlay,
        &mut index,
        &file_path,
        &base_content,
        &symbol_id,
        &new_symbol_code,
        &intent,
    )?;

    Ok(TransactionExecutionResultDto {
        is_committed: res.is_committed,
        commit_message: res.commit_message,
        patched_files: res.patched_files,
        changes_count: res.diff_report.changes.len(),
        file_moves_count: res.diff_report.file_moves.len(),
        snapshot_id: res.snapshot_id.map(|s| s.0),
    })
}

#[tauri::command]
fn forge_execute_multi_patch_transaction(
    app: tauri::AppHandle,
    base_commit: String,
    task_id: String,
    base_files: Vec<(String, String)>,
    patches: Vec<forge::commit::TransactionPatch>,
    intent: String,
) -> Result<TransactionExecutionResultDto, String> {
    let db_path = active_database_path(&app)?;
    let mut index = forge::index::CodeIndex::new(db_path).map_err(|e| e.to_string())?;
    let mut overlay = forge::overlay::CodeOverlayManager::new(&base_commit, &task_id);

    let base_files_refs: Vec<(&str, &str)> = base_files
        .iter()
        .map(|(p, c)| (p.as_str(), c.as_str()))
        .collect();

    let res = forge::tools::ForgeToolRegistry::execute_multi_patch_transaction(
        &base_commit,
        &task_id,
        &mut overlay,
        &mut index,
        &base_files_refs,
        &patches,
        &intent,
    )?;

    Ok(TransactionExecutionResultDto {
        is_committed: res.is_committed,
        commit_message: res.commit_message,
        patched_files: res.patched_files,
        changes_count: res.diff_report.changes.len(),
        file_moves_count: res.diff_report.file_moves.len(),
        snapshot_id: res.snapshot_id.map(|s| s.0),
    })
}

#[tauri::command]
fn forge_get_lineage_history(
    app: tauri::AppHandle,
    lineage_id: String,
) -> Result<Vec<LineageHistoryEntryDto>, String> {
    let db_path = active_database_path(&app)?;
    let index = forge::index::CodeIndex::new(db_path).map_err(|e| e.to_string())?;
    let history = index
        .get_lineage_history(&forge::model::LineageId::new(lineage_id))
        .map_err(|e| e.to_string())?;

    Ok(history
        .into_iter()
        .map(|(snap, sym, conf)| LineageHistoryEntryDto {
            snapshot_id: snap.0,
            symbol_id: sym.0,
            confidence: conf,
        })
        .collect())
}

#[tauri::command]
fn forge_get_canonical_entities(
    app: tauri::AppHandle,
) -> Result<Vec<forge::model::CanonicalSymbolRecord>, String> {
    let db_path = active_database_path(&app)?;
    let index = forge::index::CodeIndex::new(db_path).map_err(|e| e.to_string())?;
    index.get_canonical_entities().map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn forge_record_audit_entry(
    app: tauri::AppHandle,
    task_id: String,
    intent: String,
    target_symbol: String,
    target_file: String,
    base_snapshot_id: Option<String>,
    committed_snapshot_id: Option<String>,
    healing_attempts: usize,
    status: String,
    error_payload: Option<String>,
) -> Result<i64, String> {
    let db_path = active_database_path(&app)?;
    let index = forge::index::CodeIndex::new(db_path).map_err(|e| e.to_string())?;
    let ws_id = index
        .get_or_create_workspace(".")
        .map_err(|e| e.to_string())?;

    let base_snap = base_snapshot_id.map(forge::model::SnapshotId);
    let committed_snap = committed_snapshot_id.map(forge::model::SnapshotId);

    index
        .record_audit_entry(
            &task_id,
            &ws_id,
            &intent,
            &target_symbol,
            &target_file,
            base_snap.as_ref(),
            committed_snap.as_ref(),
            healing_attempts,
            &status,
            error_payload.as_deref(),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn forge_get_task_audit_history(
    app: tauri::AppHandle,
    task_id: String,
) -> Result<Vec<forge::index::AuditTrailRecord>, String> {
    let db_path = active_database_path(&app)?;
    let index = forge::index::CodeIndex::new(db_path).map_err(|e| e.to_string())?;
    index
        .get_task_audit_history(&task_id)
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            fs::create_dir_all(&app_dir).expect("Failed to create app data directory");

            let db_path = app_dir.join("vora_data.vora");
            let legacy_default_path = app_dir.join("vora_workspace.db");
            mind::portable_db::migrate_legacy_database_selector(&app.handle(), &db_path)
                .expect("Failed to migrate legacy database location");
            let database_existed = db_path.is_file();
            let mut conn =
                Connection::open(&db_path).expect("Failed to open default database in AppData");
            if !database_existed && legacy_default_path.is_file() {
                let source = Connection::open(&legacy_default_path)
                    .expect("Failed to open legacy default database");
                let backup = rusqlite::backup::Backup::new(&source, &mut conn)
                    .expect("Failed to prepare legacy database migration");
                backup
                    .run_to_completion(5, std::time::Duration::from_millis(100), None)
                    .expect("Failed to migrate legacy default database");
            }
            mind::portable_db::initialize_workspace_schema(&conn)
                .expect("Failed to initialize default database schema");
            conn.execute_batch("PRAGMA foreign_keys = ON;")
                .expect("Failed to enable foreign keys");
            conn.pragma_update(None, "journal_mode", "DELETE")
                .expect("Failed to configure portable database journal mode");
            mind::portable_db::initialize_settings_file(&conn, &db_path, None)
                .expect("Failed to initialize workspace settings file");

            app.manage(AppState {
                db: Mutex::new(conn),
                db_path: Mutex::new(Some(db_path.to_string_lossy().into_owned())),
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            install_update_with_helper,
            get_all_boards,
            create_board,
            get_task_flow_items,
            create_task_flow_item,
            set_task_flow_item_completed,
            reorder_task_flow_items,
            delete_task_flow_item,
            get_journal_entries,
            save_journal_entry,
            get_board_versions,
            create_board_version,
            checkout_board_version,
            get_library_item_groups,
            get_library_item_group_versions,
            get_library_item_group_usages,
            create_library_item_group,
            commit_library_item_group_version,
            move_library_item_group_to_trash,
            restore_library_item_group,
            rename_board,
            delete_board,
            export_board_to_file,
            read_board_backup_storage,
            import_board_from_file,
            get_backup_catalog,
            export_selective_package,
            inspect_backup_file,
            import_selective_package,
            get_imported_item_origins,
            commit_snapshot,
            resolve_snapshot,
            get_all_snapshots,
            get_all_cards,
            get_text_links_for_card,
            create_text_link,
            delete_text_link,
            save_card_direct,
            move_card_to_bin,
            restore_card_from_bin,
            parse_code_ast,
            read_project_file,
            get_forge_tools,
            execute_forge_tool,
            apply_patch_and_validate,
            create_git_commit,
            scan_workspace,
            get_git_commit_history,
            run_project_tests,
            forge_test_candidate_patch,
            forge_test_multi_candidate_patches,
            forge_rollback_to_snapshot,
            resolve_strict_edges,
            simulate_semantic_change,
            create_semantic_plan,
            forge_compute_ast_semantic_diff,
            forge_repair_and_expand_plan,
            forge_find_symbol,
            forge_get_impact_radius,
            forge_execute_patch_transaction,
            forge_execute_multi_patch_transaction,
            forge_get_lineage_history,
            forge_get_canonical_entities,
            forge_record_audit_entry,
            forge_get_task_audit_history,
            mind::portable_db::get_settings_file_path,
            mind::portable_db::get_current_db_path,
            mind::portable_db::export_database_backup,
            mind::portable_db::export_settings_json,
            mind::portable_db::import_settings_json,
            mind::portable_db::import_database_backup,
            mind::portable_db::save_setting,
            mind::portable_db::get_setting,
            mind::portable_db::get_all_settings,
            mind::portable_db::delete_setting,
            mind::portable_db::replace_all_settings,
            mind::portable_db::save_vora_item,
            mind::portable_db::get_vora_items,
            mind::portable_db::save_vora_snapshot,
            mind::portable_db::get_vora_snapshots,
            mind::portable_db::get_all_vora_snapshots,
            mind::portable_db::replace_vora_snapshots,
            mind::portable_db::delete_vora_snapshot,
            mind::portable_db::clear_vora_snapshots,
            crate::source::ipc::src_list_sources,
            crate::source::ipc::src_get_source_bundle,
            crate::source::ipc::src_list_extracts,
            crate::source::ipc::src_get_extract,
            crate::source::ipc::src_create_source,
            crate::source::ipc::src_create_segment,
            crate::source::ipc::src_create_extract,
            crate::source::ipc::src_create_link,
            crate::source::ipc::src_mark_extract_reviewed,
            crate::market::ipc::mkt_fetch_historical_duka,
            crate::market::ipc::mkt_detect_boxes,
            crate::market::ipc::mkt_cancel_request,
            crate::market::ipc::mkt_get_symbol_origin,
            crate::market::ipc::mkt_refresh_symbol_origin,
            crate::market::ipc::mkt_get_scale_registry,
            crate::market::ipc::mkt_set_scale_registry,
            crate::market::ipc::mkt_list_scale_registries,
            crate::market::ipc::mkt_list_drawing_templates,
            crate::market::ipc::mkt_save_drawing_template,
            crate::market::ipc::mkt_delete_drawing_template,
            crate::market::ipc::mkt_list_drawing_alerts,
            crate::market::ipc::mkt_save_drawing_alert,
            crate::market::ipc::mkt_update_drawing_alert,
            crate::market::ipc::mkt_delete_drawing_alert,
            crate::market::ipc::mkt_delete_drawing_alerts_for_drawings,
            crate::market::ipc::mkt_list_pivots,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub mod market;
pub mod source;

/// Resolve the active SQLite path for standalone CLI and IPC helpers.
pub fn default_db_path() -> Result<String, Box<dyn std::error::Error>> {
    let data_dir = if let Some(base) = std::env::var_os("APPDATA") {
        let mut path = std::path::PathBuf::from(base);
        path.push("com.alexfedura.voramind");
        path
    } else if let Some(base) =
        std::env::var_os("LOCALAPPDATA").or_else(|| std::env::var_os("XDG_DATA_HOME"))
    {
        let mut path = std::path::PathBuf::from(base);
        path.push("com.alexfedura.voramind");
        path
    } else if let Some(home) = std::env::var_os("HOME") {
        let mut path = std::path::PathBuf::from(home);
        path.push("Library/Application Support/com.alexfedura.voramind");
        path
    } else {
        return Err("cannot determine data directory".into());
    };

    Ok(data_dir
        .join("vora_data.vora")
        .to_string_lossy()
        .into_owned())
}

/// Open default SQLite connection for CLI or IPC helpers.
pub fn open_default_db_connection() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
    let path = default_db_path()?;
    let conn = rusqlite::Connection::open(&path)?;
    crate::market::db::init_schema(&conn)?;
    Ok(conn)
}
