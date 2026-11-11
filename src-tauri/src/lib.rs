extern crate self as vora_lib;

pub mod experiment;
pub mod analysis;
pub mod learning;
pub mod model;
pub mod forge;
pub mod mind;
pub mod research;
pub mod transfer;
pub mod vora_trace;

use crate::mind::canvas_db::{AppState, BoardSummary, EdgeInput, NodeInput, SnapshotSummary};
use crate::mind::resolver::{ResolvedSnapshot, SnapshotResolver};
use futures::TryStreamExt;
use rusqlite::{Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};
use tauri_plugin_updater::UpdaterExt;

const DEEPSEEK_API_KEY_SETTING: &str = "vora_deepseek_api_key";
const DEEPSEEK_COMPLETION_URL: &str = "https://api.deepseek.com/v1/chat/completions";
const MAX_DEEPSEEK_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

enum DeepSeekStreamLine {
    Ignore,
    Done,
    Delta(String),
}

fn parse_deepseek_sse_line(line: &[u8]) -> Result<DeepSeekStreamLine, String> {
    let line = std::str::from_utf8(line)
        .map_err(|error| format!("DeepSeek returned an invalid UTF-8 stream line: {error}"))?
        .trim_end_matches('\r')
        .trim();
    let Some(data) = line.strip_prefix("data:") else {
        return Ok(DeepSeekStreamLine::Ignore);
    };
    let data = data.trim();
    if data == "[DONE]" {
        return Ok(DeepSeekStreamLine::Done);
    }

    let event: serde_json::Value = serde_json::from_str(data)
        .map_err(|error| format!("Could not parse a DeepSeek stream event: {error}"))?;
    let delta = event
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("delta"))
        .and_then(|delta| delta.get("content"))
        .and_then(serde_json::Value::as_str);
    Ok(delta.map_or(DeepSeekStreamLine::Ignore, |value| {
        DeepSeekStreamLine::Delta(value.to_string())
    }))
}

fn process_deepseek_sse_line(
    app: &tauri::AppHandle,
    line: &[u8],
    full_response: &mut String,
) -> Result<bool, String> {
    let delta = match parse_deepseek_sse_line(line)? {
        DeepSeekStreamLine::Ignore => return Ok(false),
        DeepSeekStreamLine::Done => return Ok(true),
        DeepSeekStreamLine::Delta(delta) => delta,
    };

    if full_response.len().saturating_add(delta.len()) > MAX_DEEPSEEK_RESPONSE_BYTES {
        return Err("DeepSeek response exceeded the 2 MiB limit.".to_string());
    }
    full_response.push_str(&delta);
    app.emit("deepseek-chunk", delta)
        .map_err(|error| format!("Could not forward DeepSeek response progress: {error}"))?;
    Ok(false)
}

#[cfg(test)]
mod deepseek_stream_tests {
    use super::{parse_deepseek_sse_line, DeepSeekStreamLine};

    #[test]
    fn parses_deepseek_content_deltas() {
        let line = br#"data: {"choices":[{"delta":{"content":"{\"nodes\":"}}]}"#;
        match parse_deepseek_sse_line(line).unwrap() {
            DeepSeekStreamLine::Delta(delta) => assert_eq!(delta, "{\"nodes\":"),
            _ => panic!("expected a content delta"),
        }
    }

    #[test]
    fn recognizes_stream_completion_and_ignores_metadata_lines() {
        assert!(matches!(
            parse_deepseek_sse_line(b"data: [DONE]").unwrap(),
            DeepSeekStreamLine::Done
        ));
        assert!(matches!(
            parse_deepseek_sse_line(b": keep-alive").unwrap(),
            DeepSeekStreamLine::Ignore
        ));
    }

    #[test]
    fn rejects_malformed_stream_events() {
        assert!(parse_deepseek_sse_line(b"data: {not-json}").is_err());
    }
}

#[tauri::command]
fn save_deepseek_key(state: State<AppState>, api_key: String) -> Result<(), String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("DeepSeek API key cannot be empty.".to_string());
    }
    mind::portable_db::save_setting(
        state,
        DEEPSEEK_API_KEY_SETTING.to_string(),
        api_key.to_string(),
    )
}

#[tauri::command]
async fn generate_board_from_prompt(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    prompt: String,
) -> Result<(), String> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("Enter a prompt before generating a Board.".to_string());
    }
    let api_key = mind::portable_db::get_setting(state, DEEPSEEK_API_KEY_SETTING.to_string())?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Save a DeepSeek API key in the generator first.".to_string())?;

    let client = reqwest::Client::new();
    let response = client
        .post(DEEPSEEK_COMPLETION_URL)
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "model": "deepseek-chat",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a graph generator. Return a JSON object with 'nodes' (array of {id, type, title, content}) and 'edges' (array of {source, target, label}). Node type must be 'voraCard' or 'voraText'."
                },
                { "role": "user", "content": prompt }
            ],
            "stream": true,
            "response_format": { "type": "json_object" }
        }))
        .send()
        .await
        .map_err(|error| format!("Could not connect to DeepSeek: {error}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("DeepSeek returned HTTP {status}: {body}"));
    }

    let mut stream = response.bytes_stream();
    let mut pending_line = Vec::new();
    let mut full_response = String::new();
    let mut stream_finished = false;
    while let Some(chunk) = stream
        .try_next()
        .await
        .map_err(|error| format!("DeepSeek response stream failed: {error}"))?
    {
        pending_line.extend_from_slice(&chunk);
        while let Some(line_end) = pending_line.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = pending_line.drain(..=line_end).collect();
            if process_deepseek_sse_line(&app, &line, &mut full_response)? {
                stream_finished = true;
                break;
            }
        }
        if stream_finished {
            break;
        }
        if pending_line.len().saturating_add(full_response.len()) > MAX_DEEPSEEK_RESPONSE_BYTES {
            return Err("DeepSeek response exceeded the 2 MiB limit.".to_string());
        }
    }

    if !stream_finished && !pending_line.is_empty() {
        process_deepseek_sse_line(&app, &pending_line, &mut full_response)?;
    }
    if full_response.trim().is_empty() {
        return Err("DeepSeek completed without returning a generated graph.".to_string());
    }
    app.emit("deepseek-done", full_response)
        .map_err(|error| format!("Could not deliver the generated Board: {error}"))?;
    Ok(())
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdaterDownloadProgress {
    downloaded_bytes: u64,
    content_length: Option<u64>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdaterFailureMarker {
    version: String,
    failed_at: u64,
}

#[tauri::command]
async fn install_update(
    app: tauri::AppHandle,
    requested_version: String,
) -> Result<Option<String>, String> {
    let updater = app.updater().map_err(|error| error.to_string())?;
    let Some(update) = updater.check().await.map_err(|error| error.to_string())? else {
        return Ok(None);
    };
    if update.version != requested_version {
        return Ok(None);
    }
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

    update
        .install(installer_bytes)
        .map_err(|error| error.to_string())?;

    Ok(Some(update_version))
}

#[tauri::command]
fn get_last_update_failure(app: tauri::AppHandle) -> Result<Option<UpdaterFailureMarker>, String> {
    let marker_path = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("updater")
        .join("last-failed-update.json");
    match fs::read(&marker_path) {
        Ok(contents) => serde_json::from_slice(&contents)
            .map(Some)
            .map_err(|error| {
                format!(
                    "Could not parse updater failure marker at {}: {error}",
                    marker_path.display()
                )
            }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "Could not read updater failure marker at {}: {error}",
            marker_path.display()
        )),
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
    pub document_id: Option<String>,
    pub highlight_id: Option<String>,
    pub source_file_id: Option<String>,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
}

#[derive(serde::Serialize)]
struct CardVersionSummary {
    id: String,
    version: String,
    title: Option<String>,
    reason: Option<String>,
    created_at: i64,
}

fn parse_card_version(version: &str) -> Result<(String, i32, i32, i32), String> {
    let trimmed = version.trim();
    let numeric_version = trimmed.strip_prefix('v').unwrap_or(trimmed);
    let parts: Vec<&str> = numeric_version.split('.').collect();
    if parts.len() != 3 {
        return Err("Card version must use three numeric components, such as v0.1.1".to_string());
    }

    let mut parsed = [0_i32; 3];
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty()
            || (part.len() > 1 && part.starts_with('0'))
            || !part.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(
                "Card version must use three numeric components, such as v0.1.1".to_string(),
            );
        }
        parsed[index] = part
            .parse::<i32>()
            .map_err(|_| "Card version components must not exceed 2147483647".to_string())?;
    }

    Ok((
        format!("v{}.{}.{}", parsed[0], parsed[1], parsed[2]),
        parsed[0],
        parsed[1],
        parsed[2],
    ))
}

fn store_card_working_content(
    conn: &Connection,
    board_id: &str,
    id: &str,
    title: &str,
    content: &str,
    version: &str,
    now: i64,
    metadata: Option<String>,
) -> Result<usize, rusqlite::Error> {
    let working_content = serde_json::json!({
        "title": title,
        "content": content,
        "version": version,
    })
    .to_string();
    conn.execute(
        "INSERT INTO objects
            (id, type, board_id, created_at, updated_at, deleted_at, metadata, working_content)
         VALUES (?1, 'card', ?2, ?3, ?3, NULL, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
             updated_at = excluded.updated_at,
             working_content = excluded.working_content,
             metadata = CASE
                 WHEN excluded.metadata IS NULL THEN objects.metadata
                 ELSE json_patch(COALESCE(objects.metadata, '{}'), excluded.metadata)
             END
         WHERE objects.board_id = excluded.board_id",
        rusqlite::params![id, board_id, now, metadata, working_content],
    )
}

fn has_committed_card_versions(conn: &Connection, id: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM object_versions
            WHERE object_id = ?1 AND summary = 'Card version commit'
        )",
        rusqlite::params![id],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod card_version_tests {
    use super::{has_committed_card_versions, parse_card_version, store_card_working_content};
    use rusqlite::Connection;

    #[test]
    fn parses_card_version_with_or_without_prefix() {
        assert_eq!(
            parse_card_version("v1.2.3").unwrap(),
            ("v1.2.3".to_string(), 1, 2, 3)
        );
        assert_eq!(
            parse_card_version("0.1.0").unwrap(),
            ("v0.1.0".to_string(), 0, 1, 0)
        );
    }

    #[test]
    fn rejects_invalid_card_versions() {
        for version in ["", "v1.2", "1.2.3.4", "v01.2.3", "1.2.-1", "1.2.2147483648"] {
            assert!(parse_card_version(version).is_err(), "accepted {version}");
        }
    }

    #[test]
    fn automatic_saves_do_not_count_as_committed_card_versions() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        conn.execute_batch(
            "CREATE TABLE object_versions (object_id TEXT NOT NULL, reason TEXT, summary TEXT);",
        )
        .expect("create object_versions table");

        assert!(!has_committed_card_versions(&conn, "card-1").expect("check empty history"));
        conn.execute(
            "INSERT INTO object_versions (object_id, reason, summary)
             VALUES ('card-1', 'Auto Save', 'Direct edit')",
            [],
        )
        .expect("insert automatic save");
        assert!(!has_committed_card_versions(&conn, "card-1").expect("check autosave history"));
        conn.execute(
            "INSERT INTO object_versions (object_id, reason, summary)
             VALUES ('card-1', 'Restored from v0.1.0', 'Card version restore')",
            [],
        )
        .expect("insert automatic restore");
        assert!(!has_committed_card_versions(&conn, "card-1").expect("check restore history"));
        conn.execute(
            "INSERT INTO object_versions (object_id, reason, summary)
             VALUES ('card-1', 'Initial Version (Baseline)', 'Card version commit')",
            [],
        )
        .expect("insert committed version");
        assert!(has_committed_card_versions(&conn, "card-1").expect("check committed history"));
    }

    #[test]
    fn saving_card_working_content_does_not_create_version_history() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        conn.execute_batch(
            "CREATE TABLE objects (
                id TEXT PRIMARY KEY,
                type TEXT NOT NULL,
                board_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                deleted_at INTEGER,
                metadata JSON,
                working_content JSON
             );
             CREATE TABLE object_versions (
                id TEXT PRIMARY KEY,
                object_id TEXT NOT NULL
             );",
        )
        .expect("create card tables");

        store_card_working_content(
            &conn,
            "board-1",
            "card-1",
            "Draft title",
            "Draft content",
            "v0.1.0",
            1,
            None,
        )
        .expect("save initial working content");
        store_card_working_content(
            &conn,
            "board-1",
            "card-1",
            "Updated title",
            "Updated content",
            "v0.1.0",
            2,
            None,
        )
        .expect("update working content");

        let (version_count, title): (i64, String) = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM object_versions),
                        json_extract(working_content, '$.title')
                 FROM objects WHERE id = 'card-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read working card state");
        assert_eq!(version_count, 0);
        assert_eq!(title, "Updated title");
    }
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

#[tauri::command]
fn fork_board_from_version(
    state: State<AppState>,
    source_board_id: String,
    version_id: String,
    new_board_title: String,
) -> Result<String, String> {
    let title = new_board_title.trim();
    if title.is_empty() || title.chars().count() > 80 {
        return Err("Board title must contain between 1 and 80 characters".to_string());
    }
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::canvas_db::fork_board_from_version(&mut conn, &source_board_id, &version_id, title)
        .map_err(|error| error.to_string())
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
    task_type: mind::task_flow::TaskFlowType,
) -> Result<Vec<mind::task_flow::TaskFlowItem>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::task_flow::list_tasks(&conn, task_type).map_err(|error| error.to_string())
}

#[tauri::command]
fn create_task_flow_item(
    state: State<AppState>,
    title: String,
    task_type: mind::task_flow::TaskFlowType,
) -> Result<mind::task_flow::TaskFlowItem, String> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err("Task title must contain between 1 and 200 characters".to_string());
    }
    let id = format!("task_{}", uuid::Uuid::now_v7());
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::task_flow::create_task(&mut conn, &id, title, task_type, task_flow_now()?)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn update_task_flow_item(
    state: State<AppState>,
    id: String,
    title: String,
    description: String,
) -> Result<mind::task_flow::TaskFlowItem, String> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err("Task title must contain between 1 and 200 characters".to_string());
    }
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::task_flow::update_task(&conn, &id, title, &description, task_flow_now()?)
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
    task_type: mind::task_flow::TaskFlowType,
) -> Result<Vec<mind::task_flow::TaskFlowItem>, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::task_flow::reorder_tasks(&mut conn, &ids, task_type, task_flow_now()?)
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
fn get_questions(state: State<AppState>) -> Result<Vec<mind::questions::QuestionItem>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::questions::list_questions(&conn).map_err(|error| error.to_string())
}

#[tauri::command]
fn create_question(
    state: State<AppState>,
    title: String,
    description: String,
) -> Result<mind::questions::QuestionItem, String> {
    let id = format!("question_{}", uuid::Uuid::now_v7());
    let now = task_flow_now()?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::questions::create_question(&conn, &id, &title, &description, now)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_question(
    state: State<AppState>,
    id: String,
    title: String,
    description: String,
) -> Result<mind::questions::QuestionItem, String> {
    let now = task_flow_now()?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::questions::save_question(&conn, &id, &title, &description, now)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_question(state: State<AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::questions::delete_question(&conn, &id).map_err(|error| error.to_string())
}

#[tauri::command]
fn triage_question(
    state: State<AppState>,
    id: String,
    impact: i64,
    feasibility: i64,
    urgency: i64,
    triage_notes: Option<String>,
) -> Result<mind::questions::QuestionItem, String> {
    let now = task_flow_now()?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::questions::triage_question(
        &conn,
        &id,
        impact,
        feasibility,
        urgency,
        triage_notes.as_deref(),
        now,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn untriage_question(
    state: State<AppState>,
    id: String,
) -> Result<mind::questions::QuestionItem, String> {
    let now = task_flow_now()?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::questions::untriage_question(&conn, &id, now).map_err(|error| error.to_string())
}

#[tauri::command]
fn archive_question(
    state: State<AppState>,
    id: String,
) -> Result<mind::questions::QuestionItem, String> {
    let now = task_flow_now()?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::questions::archive_question(&conn, &id, now).map_err(|error| error.to_string())
}

#[tauri::command]
fn select_for_promotion(
    state: State<AppState>,
    id: String,
) -> Result<mind::questions::QuestionItem, String> {
    let now = task_flow_now()?;
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::questions::select_for_promotion(&mut conn, &id, now).map_err(|error| error.to_string())
}

#[tauri::command]
fn unselect_for_promotion(
    state: State<AppState>,
    id: String,
) -> Result<mind::questions::QuestionItem, String> {
    let now = task_flow_now()?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::questions::unselect_for_promotion(&conn, &id, now).map_err(|error| error.to_string())
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
    reason: String,
    custom_version: Option<String>,
    snapshot: mind::canvas_db::BoardVersionSnapshot,
) -> Result<mind::canvas_db::BoardVersionSummary, String> {
    if !matches!(increment.as_str(), "major" | "minor" | "patch") {
        return Err("Choose a Major, Minor, or Patch version increment.".to_string());
    }
    if reason.trim().is_empty() {
        return Err("Enter a reason for this Board version.".to_string());
    }
    if custom_version
        .as_deref()
        .is_some_and(|version| mind::canvas_db::parse_board_version(version).is_none())
    {
        return Err("Use a semantic version tag such as 0.1.0.".to_string());
    }
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    if custom_version.is_some()
        && !mind::canvas_db::get_board_versions(&conn, &board_id)
            .map_err(|error| error.to_string())?
            .is_empty()
    {
        return Err(
            "A custom version tag can only be used for the first Board version.".to_string(),
        );
    }
    mind::canvas_db::create_board_version(
        &mut conn,
        &board_id,
        &increment,
        &reason,
        custom_version.as_deref(),
        snapshot,
    )
    .map_err(|error| match error {
        rusqlite::Error::InvalidQuery => {
            "That version label already exists on this Board. Choose another increment.".to_string()
        }
        other => other.to_string(),
    })
}

#[tauri::command]
fn get_board_version_snapshot(
    state: State<AppState>,
    version_id: String,
) -> Result<mind::canvas_db::BoardVersionSnapshot, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::canvas_db::get_board_version_snapshot(&conn, &version_id)
        .map_err(|error| error.to_string())
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
fn get_library_definitions_for_selection(
    state: State<AppState>,
) -> Result<Vec<mind::library_groups::LibraryDefinitionSelection>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::library_groups::list_definitions_for_selection(&conn).map_err(|e| e.to_string())
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
    item_type: String,
    snapshot: mind::library_groups::LibraryItemGroupSnapshot,
) -> Result<mind::library_groups::LibraryItemGroup, String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::library_groups::create_item_group(&mut conn, &name, &version, &item_type, snapshot)
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
fn restore_library_item_group(state: State<AppState>, group_id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    mind::library_groups::restore_item_group(&conn, &group_id).map_err(|error| error.to_string())
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
fn search_library_objects(
    state: State<AppState>,
    query: String,
    status_filter: Option<String>,
) -> Result<Vec<mind::canvas_db::LibraryObjectResult>, String> {
    if let Some(status) = status_filter.as_deref() {
        if !matches!(status, "all" | "active" | "completed" | "not_needed") {
            return Err("Unsupported Library review status filter".to_string());
        }
    }
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::canvas_db::search_library_objects(&conn, &query, status_filter.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn update_object_review_status(
    state: State<AppState>,
    object_id: String,
    status: String,
) -> Result<(), String> {
    if !matches!(status.as_str(), "active" | "completed" | "not_needed") {
        return Err("Unsupported Library review status".to_string());
    }
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mind::canvas_db::update_object_review_status(&conn, &object_id, &status)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn import_document(
    state: State<AppState>,
    file_path: String,
    file_type: String,
) -> Result<String, String> {
    if !matches!(file_type.as_str(), "pdf" | "docx" | "md") {
        return Err("Supported document types are PDF, DOCX, and Markdown.".to_string());
    }
    let source = Path::new(&file_path)
        .canonicalize()
        .map_err(|error| format!("Could not access the selected document: {error}"))?;
    if !source.is_file() {
        return Err("The selected document is not a file.".to_string());
    }
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "The selected document has no supported extension.".to_string())?;
    if extension != file_type {
        return Err("The selected file extension does not match its document type.".to_string());
    }
    let title = source
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Untitled document")
        .to_string();
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let file = mind::media::ensure_file_reference(&conn, &source, Some(&file_type))?;
    let document_id = format!("document_{}", uuid::Uuid::now_v7());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as i64;
    let existing_document_id = conn
        .query_row(
            "SELECT id FROM documents WHERE file_path = ?1",
            [&file.current_path],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if let Some(document_id) = existing_document_id {
        conn.execute(
            "UPDATE documents SET title = ?1, file_id = ?2, file_type = ?3, updated_at = ?4
             WHERE id = ?5",
            rusqlite::params![title, file.id, file_type, now, document_id],
        )
        .map_err(|error| format!("Could not update imported document: {error}"))?;
        return Ok(document_id);
    }
    conn.execute(
        "INSERT INTO documents (id, title, file_path, file_id, file_type, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        rusqlite::params![
            document_id,
            title,
            file.current_path,
            file.id,
            file_type,
            now
        ],
    )
    .map_err(|error| format!("Could not register imported document: {error}"))?;
    Ok(document_id)
}

#[tauri::command]
fn get_documents(state: State<AppState>) -> Result<Vec<mind::canvas_db::DocumentDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let mut statement = conn
        .prepare(
            "SELECT id, title, file_type, page_count, created_at, updated_at
             FROM documents ORDER BY updated_at DESC, id",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(mind::canvas_db::DocumentDto {
                id: row.get(0)?,
                title: row.get(1)?,
                file_type: row.get(2)?,
                page_count: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_document_content(state: State<AppState>, document_id: String) -> Result<Vec<u8>, String> {
    let file_path: String = state
        .db
        .lock()
        .map_err(|error| error.to_string())?
        .query_row(
            "SELECT files.current_path
             FROM documents
             JOIN vora_files AS files ON files.id = documents.file_id
             WHERE documents.id = ?1",
            [&document_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not find imported document: {error}"))?;
    fs::read(&file_path).map_err(|error| format!("Could not read document {file_path}: {error}"))
}

#[tauri::command]
fn create_highlight(
    state: State<AppState>,
    document_id: String,
    content: String,
    page_number: Option<i32>,
    position_data: String,
    note: Option<String>,
    color: Option<String>,
) -> Result<String, String> {
    if content.trim().is_empty() {
        return Err("A highlight must contain selected text.".to_string());
    }
    let positions: serde_json::Value = serde_json::from_str(&position_data)
        .map_err(|error| format!("Invalid highlight position data: {error}"))?;
    if !positions.is_object() && !positions.is_array() {
        return Err("Highlight position data must be a JSON object or array.".to_string());
    }
    if page_number.is_some_and(|page| page < 1) {
        return Err("PDF page numbers start at 1.".to_string());
    }
    let color = color.unwrap_or_else(|| "yellow".to_string());
    if !matches!(color.as_str(), "yellow" | "green" | "blue" | "pink") {
        return Err("Unsupported highlight color.".to_string());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as i64;
    let highlight_id = format!("highlight_{}", uuid::Uuid::now_v7());
    state
        .db
        .lock()
        .map_err(|error| error.to_string())?
        .execute(
            "INSERT INTO document_highlights
                (id, document_id, content, page_number, position_data, color, note, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                highlight_id,
                document_id,
                content,
                page_number,
                positions.to_string(),
                color,
                note,
                now
            ],
        )
        .map_err(|error| format!("Could not save document highlight: {error}"))?;
    Ok(highlight_id)
}

#[tauri::command]
fn get_highlights(
    state: State<AppState>,
    document_id: String,
) -> Result<Vec<mind::canvas_db::HighlightDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let mut statement = conn
        .prepare(
            "SELECT id, document_id, content, page_number, position_data,
                    color, note, library_card_id, created_at
             FROM document_highlights WHERE document_id = ?1
             ORDER BY created_at, id",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([document_id], |row| {
            let position_json: String = row.get(4)?;
            let position_data = serde_json::from_str(&position_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(mind::canvas_db::HighlightDto {
                id: row.get(0)?,
                document_id: row.get(1)?,
                content: row.get(2)?,
                page_number: row.get(3)?,
                position_data,
                color: row.get(5)?,
                note: row.get(6)?,
                library_card_id: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn send_highlight_to_library(
    state: State<AppState>,
    highlight_id: String,
    board_id: String,
) -> Result<String, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    let (document_id, content, existing_card_id): (String, String, Option<String>) = tx
        .query_row(
            "SELECT document_id, content, library_card_id
             FROM document_highlights WHERE id = ?1",
            [&highlight_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| format!("Could not find document highlight: {error}"))?;
    if let Some(card_id) = existing_card_id {
        tx.commit().map_err(|error| error.to_string())?;
        return Ok(card_id);
    }
    let board_exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM mind_boards WHERE id = ?1)",
            [&board_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !board_exists {
        return Err(
            "Select or create a Board before sending a highlight to the Library.".to_string(),
        );
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as i64;
    let card_id = format!("card_{}", uuid::Uuid::now_v7());
    let title = content
        .chars()
        .take(80)
        .collect::<String>()
        .trim()
        .to_string();
    let working_content = serde_json::json!({
        "title": title,
        "content": content,
        "version": "v0.1.0"
    })
    .to_string();
    let metadata = serde_json::json!({
        "documentId": document_id,
        "highlightId": highlight_id,
    })
    .to_string();
    tx.execute(
        "INSERT INTO objects
            (id, type, board_id, created_at, updated_at, metadata, working_content)
         VALUES (?1, 'card', ?2, ?3, ?3, ?4, ?5)",
        rusqlite::params![card_id, board_id, now, metadata, working_content],
    )
    .map_err(|error| format!("Could not create Library card from highlight: {error}"))?;
    tx.execute(
        "UPDATE document_highlights SET library_card_id = ?1 WHERE id = ?2",
        rusqlite::params![card_id, highlight_id],
    )
    .map_err(|error| format!("Could not link Library card to highlight: {error}"))?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(card_id)
}

#[tauri::command]
fn get_all_cards(state: State<AppState>, board_id: String) -> Result<Vec<CardDto>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "
            SELECT o.id,
                   COALESCE(json_extract(o.working_content, '$.title'), json_extract(v.content, '$.title'), 'Untitled') as title,
                   COALESCE(json_extract(o.working_content, '$.content'), json_extract(v.content, '$.content'), '') as content,
                   COALESCE(json_extract(o.working_content, '$.version'),
                            json_extract(v.content, '$.version'),
                            v.version_major || '.' || v.version_minor || '.' || v.version_patch,
                            'v0.1.0') as version,
                   o.deleted_at,
                   json_extract(o.metadata, '$.librarySourceCardId'),
                   json_extract(o.metadata, '$.documentId'),
                   json_extract(o.metadata, '$.highlightId'),
                   json_extract(o.metadata, '$.source_file_id'),
                   json_extract(o.metadata, '$.start_time'),
                   json_extract(o.metadata, '$.end_time')
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
                document_id: r.get(6)?,
                highlight_id: r.get(7)?,
                source_file_id: r.get(8)?,
                start_time: r.get(9)?,
                end_time: r.get(10)?,
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
fn commit_card_version(
    state: State<AppState>,
    id: String,
    board_id: String,
    title: String,
    content: String,
    version: String,
    reason: String,
) -> Result<(), String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as i64;
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    let latest_version_timestamp: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(created_at), 0) FROM object_versions WHERE object_id = ?1",
            rusqlite::params![&id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let now = now.max(latest_version_timestamp.saturating_add(1));

    let board_exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM mind_boards WHERE id = ?1)",
            rusqlite::params![&board_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !board_exists {
        return Err("Board does not exist".to_string());
    }

    let stored_object_id: String = tx
        .query_row(
            "SELECT id FROM objects WHERE id = ?1",
            rusqlite::params![&id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Card does not exist; its identity cannot be changed".to_string())?;
    if stored_object_id != id {
        return Err("Card identity cannot be changed".to_string());
    }
    let belongs_to_board: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM objects WHERE id = ?1 AND board_id = ?2 AND type = 'card')",
            rusqlite::params![&id, &board_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !belongs_to_board {
        return Err("Card does not belong to the requested Board".to_string());
    }

    let has_versions = has_committed_card_versions(&tx, &id)?;
    let (version, version_major, version_minor, version_patch) = parse_card_version(&version)?;
    let reason = if has_versions {
        reason.trim()
    } else {
        "Initial Version (Baseline)"
    };
    if reason.is_empty() {
        return Err("A reason is required to commit a card version".to_string());
    }

    let parent_version_id: Option<String> = tx
        .query_row(
            "SELECT id FROM object_versions
             WHERE object_id = ?1 AND summary = 'Card version commit'
             ORDER BY created_at DESC, rowid DESC LIMIT 1",
            rusqlite::params![&id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let version_id = format!("ver_{}_{}_{}", id, now, uuid::Uuid::now_v7());
    let content_json = serde_json::json!({
        "title": title,
        "content": content,
        "version": version,
    })
    .to_string();

    tx.execute(
        "INSERT INTO object_versions
         (id, object_id, version_major, version_minor, version_patch, parent_version_id, reason, summary, content, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'Card version commit', ?8, ?9)",
        rusqlite::params![
            &version_id,
            &id,
            version_major,
            version_minor,
            version_patch,
            parent_version_id,
            reason,
            content_json,
            now
        ],
    )
    .map_err(|error| error.to_string())?;
    tx.execute(
        "UPDATE objects SET working_content = ?1, updated_at = ?2 WHERE id = ?3 AND board_id = ?4",
        rusqlite::params![content_json, now, &id, &board_id],
    )
    .map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())
}

#[tauri::command]
fn has_card_versions(state: State<AppState>, id: String) -> Result<bool, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let is_card: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM objects WHERE id = ?1 AND type = 'card')",
            rusqlite::params![&id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !is_card {
        return Err("Card does not exist".to_string());
    }
    has_committed_card_versions(&conn, &id)
}

#[tauri::command]
fn get_card_versions(
    state: State<AppState>,
    id: String,
) -> Result<Vec<CardVersionSummary>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let is_card: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM objects WHERE id = ?1 AND type = 'card')",
            rusqlite::params![&id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !is_card {
        return Err("Card does not exist".to_string());
    }

    let mut statement = conn
        .prepare(
            "SELECT id,
                    COALESCE(json_extract(content, '$.version'),
                             printf('v%d.%d.%d', version_major, version_minor, version_patch)),
                    json_extract(content, '$.title'),
                    reason,
                    created_at
             FROM object_versions
             WHERE object_id = ?1 AND summary = 'Card version commit'
             ORDER BY created_at DESC, rowid DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(rusqlite::params![&id], |row| {
            Ok(CardVersionSummary {
                id: row.get(0)?,
                version: row.get(1)?,
                title: row.get(2)?,
                reason: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.map(|row| row.map_err(|error| error.to_string()))
        .collect()
}

#[tauri::command]
fn get_card_version_content(
    state: State<AppState>,
    version_id: String,
) -> Result<serde_json::Value, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let content: String = conn
        .query_row(
            "SELECT content FROM object_versions WHERE id = ?1",
            rusqlite::params![&version_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

#[tauri::command]
fn restore_card_version(
    state: State<AppState>,
    id: String,
    version_id: String,
) -> Result<serde_json::Value, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as i64;
    let tx = conn.transaction().map_err(|error| error.to_string())?;

    let object_exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM objects WHERE id = ?1 AND type = 'card')",
            rusqlite::params![&id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !object_exists {
        return Err("Card does not exist".to_string());
    }
    let (version_major, version_minor, version_patch, content): (i32, i32, i32, String) = tx
        .query_row(
            "SELECT version_major, version_minor, version_patch, content
             FROM object_versions WHERE id = ?1 AND object_id = ?2",
            rusqlite::params![&version_id, &id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|error| error.to_string())?;
    let mut restored_content: serde_json::Value =
        serde_json::from_str(&content).map_err(|error| error.to_string())?;
    let version = restored_content
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("v{version_major}.{version_minor}.{version_patch}"));
    if let Some(content) = restored_content.as_object_mut() {
        content.insert("version".to_string(), serde_json::Value::String(version));
    }
    tx.execute(
        "UPDATE objects SET working_content = ?1, updated_at = ?2 WHERE id = ?3 AND type = 'card'",
        rusqlite::params![restored_content.to_string(), now, &id],
    )
    .map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(restored_content)
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
    mind::canvas_db::create_text_link(&mut conn, input, now, &id).map_err(|error| error.to_string())
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
    media_lineage: Option<mind::canvas_db::CardMediaLineage>,
) -> Result<(), String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as i64;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    mind::canvas_db::apply_text_link_changes(&tx, &board_id, &id, &text_change_sets, now)
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

    if let Some(lineage) = &media_lineage {
        if lineage.source_file_id.trim().is_empty()
            || !lineage.start_time.is_finite()
            || !lineage.end_time.is_finite()
            || lineage.start_time < 0.0
            || lineage.end_time <= lineage.start_time
        {
            return Err(
                "Media card lineage requires a valid source file and increasing time range."
                    .to_string(),
            );
        }
        let source_exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM vora_files WHERE id = ?1)",
                [&lineage.source_file_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if !source_exists {
            return Err("The media source file does not exist.".to_string());
        }
    }

    let mut card_metadata = serde_json::Map::new();
    if let Some(source_card_id) = &library_source_card_id {
        card_metadata.insert(
            "librarySourceCardId".to_string(),
            serde_json::Value::String(source_card_id.clone()),
        );
    }
    if let Some(lineage) = &media_lineage {
        card_metadata.insert(
            "source_file_id".to_string(),
            serde_json::Value::String(lineage.source_file_id.clone()),
        );
        card_metadata.insert(
            "start_time".to_string(),
            serde_json::json!(lineage.start_time),
        );
        card_metadata.insert("end_time".to_string(), serde_json::json!(lineage.end_time));
    }
    let metadata =
        (!card_metadata.is_empty()).then(|| serde_json::Value::Object(card_metadata).to_string());
    let rows_updated = store_card_working_content(
        &tx, &board_id, &id, &title, &content, &version, now, metadata,
    )
    .map_err(|error| error.to_string())?;
    if rows_updated == 0 {
        return Err("Card does not belong to the active Board".to_string());
    }
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

    if let Some(source_card_id) = library_source_card_id {
        if source_card_id != id {
            let source_updated = tx
                .execute(
                    "UPDATE objects SET updated_at = ?1
                     WHERE id = ?2 AND board_id = ?3 AND type = 'card'",
                    rusqlite::params![now, &source_card_id, &board_id],
                )
                .map_err(|e| e.to_string())?;
            if source_updated == 0 {
                return Err("Library source card does not belong to the active Board".to_string());
            }

            let source_version: Option<String> = tx
                .query_row(
                    "SELECT COALESCE(
                         json_extract(objects.working_content, '$.version'),
                         (SELECT COALESCE(json_extract(content, '$.version'),
                                          printf('v%d.%d.%d', version_major, version_minor, version_patch))
                          FROM object_versions
                          WHERE object_id = objects.id
                          ORDER BY created_at DESC, rowid DESC
                          LIMIT 1)
                     )
                     FROM objects WHERE id = ?1",
                    rusqlite::params![&source_card_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            let source_content_json = serde_json::json!({
                "title": title,
                "content": content,
                "version": source_version.unwrap_or_else(|| version.clone()),
            })
            .to_string();
            tx.execute(
                "UPDATE objects SET working_content = ?1
                 WHERE id = ?2 AND board_id = ?3 AND type = 'card'",
                rusqlite::params![source_content_json, &source_card_id, &board_id],
            )
            .map_err(|e| e.to_string())?;
        }
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
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                if let Err(error) = mind::player::player_kill() {
                    eprintln!("Failed to stop MPV when closing Vora Mind: {error}");
                }
            }
        })
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
            mind::player::start_position_polling(app.handle().clone());

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            install_update,
            get_last_update_failure,
            save_deepseek_key,
            generate_board_from_prompt,
            get_all_boards,
            create_board,
            get_task_flow_items,
            create_task_flow_item,
            update_task_flow_item,
            set_task_flow_item_completed,
            reorder_task_flow_items,
            delete_task_flow_item,
            vora_trace::trace_load_graph,
            vora_trace::trace_save_graph,
            vora_trace::trace_save_decision_md,
            vora_trace::trace_list_decisions,
            vora_trace::trace_scan_ecosystem,
            vora_trace::trace_git_log_for_project,
            vora_trace::trace_read_all_decisions,
            vora_trace::trace_find_commits_for_file,
            vora_trace::trace_export_graph_json,
            vora_trace::trace_import_graph_json,
            vora_trace::trace_app_version,
            vora_trace::trace_recent_decisions,
            get_journal_entries,
            save_journal_entry,
            get_questions,
            create_question,
            save_question,
            delete_question,
            research::ipc::research_get_workspace,
            transfer::ipc::transfer_get_workspace,
            transfer::ipc::transfer_create_artifact_version,
            transfer::ipc::transfer_record_origin,
            transfer::ipc::transfer_create_eligibility,
            transfer::ipc::transfer_change_eligibility,
            transfer::ipc::transfer_create_transfer,
            transfer::ipc::transfer_transition_transfer,
            transfer::ipc::transfer_create_adaptation,
            transfer::ipc::transfer_create_downstream_usage,
            research::ipc::research_create_project,
            research::ipc::research_transition_project,
            research::ipc::research_create_domain,
            research::ipc::research_assign_domain,
            research::ipc::research_adopt_framework_version,
            research::ipc::research_create_literature_review,
            research::ipc::research_create_reviewed_literature_review,
            research::ipc::research_add_literature_source,
            research::ipc::research_review_literature_review,
            research::ipc::research_create_prior_assessment,
            research::ipc::research_review_prior_assessment,
            research::ipc::research_create_gap,
            research::ipc::research_add_gap_evidence,
            research::ipc::research_review_gap_exception,
            research::ipc::research_transition_gap,
            research::ipc::research_create_question,
            research::ipc::research_transition_question,
            research::ipc::research_refine_question,
            research::ipc::research_link_question_lineage,
            research::ipc::research_create_phenomenon,
            research::ipc::research_create_observation_plan,
            research::ipc::research_review_observation_plan,
            research::ipc::research_create_dataset_version_reference,
            research::ipc::research_create_observation,
            research::ipc::research_create_pattern,
            research::ipc::research_create_idea,
            research::ipc::research_create_typed_hypothesis,
            research::ipc::research_transition_hypothesis,
            research::ipc::research_link_competing_hypotheses,
            research::ipc::research_add_hypothesis_model_reference,
            research::ipc::research_add_hypothesis_evidence,
            research::ipc::research_create_design,
            research::ipc::research_create_linked_design,
            research::ipc::research_transition_design,
            research::ipc::research_link_hypothesis_design,
            experiment::ipc::experiment_get_workspace,
            experiment::ipc::experiment_create_design_version,
            experiment::ipc::experiment_preregister_design,
            experiment::ipc::experiment_create_dataset,
            experiment::ipc::experiment_create_dataset_version,
            experiment::ipc::experiment_add_dataset_samples,
            experiment::ipc::experiment_register_dataset_asset,
            experiment::ipc::experiment_add_dataset_split_group,
            experiment::ipc::experiment_create_dataset_transformation,
            experiment::ipc::experiment_seal_dataset_version,
            experiment::ipc::experiment_verify_dataset_version,
            experiment::ipc::experiment_get_dataset_samples,
            experiment::ipc::experiment_create_run,
            experiment::ipc::experiment_transition_run,
            experiment::ipc::experiment_add_run_deviation,
            experiment::ipc::experiment_add_run_log,
            analysis::ipc::analysis_get_workspace,
            analysis::ipc::analysis_create_method_version,
            analysis::ipc::analysis_create_metric_version,
            analysis::ipc::analysis_create_definition_version,
            analysis::ipc::analysis_transition_definition,
            analysis::ipc::analysis_create_run,
            analysis::ipc::analysis_start_run,
            analysis::ipc::analysis_add_run_log,
            analysis::ipc::analysis_add_run_deviation,
            analysis::ipc::analysis_finalize_run,
            analysis::ipc::analysis_get_run,
            analysis::ipc::analysis_create_evidence,
            analysis::ipc::analysis_transition_evidence,
            analysis::ipc::analysis_create_finding_version,
            analysis::ipc::analysis_transition_finding,
            analysis::ipc::analysis_create_finding_relation,
            model::ipc::model_get_workspace,
            model::ipc::model_create,
            model::ipc::model_transition,
            model::ipc::model_create_version,
            model::ipc::model_transition_version,
            model::ipc::model_create_evidence,
            model::ipc::model_create_evaluation,
            model::ipc::model_create_validation,
            model::ipc::model_finalize_validation,
            model::ipc::model_create_replication,
            learning::ipc::learning_get_workspace,
            learning::ipc::learning_create_learner,
            learning::ipc::learning_create_project,
            learning::ipc::learning_create_objective,
            learning::ipc::learning_create_assessment_version,
            learning::ipc::learning_review_assessment,
            learning::ipc::learning_transition_assessment_version,
            learning::ipc::learning_create_attempt,
            learning::ipc::learning_save_attempt_response,
            learning::ipc::learning_submit_attempt,
            learning::ipc::learning_evaluate_attempt,
            learning::ipc::learning_create_competency,
            learning::ipc::learning_create_mastery_rule,
            learning::ipc::learning_evaluate_competency,
            learning::ipc::learning_create_reproduction_task,
            learning::ipc::learning_submit_reproduction,
            learning::ipc::learning_evaluate_reproduction,
            triage_question,
            untriage_question,
            archive_question,
            select_for_promotion,
            unselect_for_promotion,
            mind::hypothesis::promote_to_hypothesis,
            mind::hypothesis::create_hypothesis_from_source,
            mind::hypothesis::get_hypotheses,
            mind::hypothesis::link_snapshot_to_hypothesis,
            mind::hypothesis::unlink_snapshot_from_hypothesis,
            mind::hypothesis::get_hypothesis_snapshots,
            mind::hypothesis::record_verdict,
            mind::hypothesis::save_checklist_schema,
            mind::hypothesis::add_data_entry,
            mind::hypothesis::get_data_entries,
            mind::hypothesis::delete_data_entry,
            mind::hypothesis::save_hypothesis_protocol,
            mind::hypothesis::save_protocol_flowchart,
            mind::hypothesis::update_protocol_completion,
            mind::hypothesis::analyze_with_ai,
            mind::hypothesis::record_hypothesis_execution,
            mind::hypothesis::add_hypothesis_insight,
            mind::hypothesis::get_hypothesis_insights,
            mind::hypothesis::delete_hypothesis_insight,
            mind::hypothesis::graduate_to_library,
            mind::hypothesis::create_child_hypothesis,
            get_board_versions,
            create_board_version,
            fork_board_from_version,
            get_board_version_snapshot,
            checkout_board_version,
            get_library_item_groups,
            get_library_item_group_versions,
            get_library_definitions_for_selection,
            get_library_item_group_usages,
            create_library_item_group,
            commit_library_item_group_version,
            move_library_item_group_to_trash,
            restore_library_item_group,
            rename_board,
            delete_board,
            commit_snapshot,
            resolve_snapshot,
            get_all_snapshots,
            get_all_cards,
            search_library_objects,
            update_object_review_status,
            import_document,
            get_documents,
            get_document_content,
            create_highlight,
            get_highlights,
            send_highlight_to_library,
            commit_card_version,
            has_card_versions,
            get_card_versions,
            get_card_version_content,
            restore_card_version,
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
            crate::source::ipc::src_list_source_revisions,
            crate::source::acquisition_ipc::src_list_source_versions,
            crate::source::acquisition_ipc::src_create_source_version,
            crate::source::acquisition_ipc::src_list_source_locations,
            crate::source::acquisition_ipc::src_create_source_location,
            crate::source::acquisition_ipc::src_list_knowledge_extractions,
            crate::source::acquisition_ipc::src_create_knowledge_extraction,
            crate::source::acquisition_ipc::src_review_knowledge_extraction,
            crate::source::acquisition_ipc::src_correct_knowledge_extraction,
            crate::source::acquisition_ipc::src_list_source_claims,
            crate::source::acquisition_ipc::src_create_source_claim,
            crate::source::acquisition_ipc::src_list_knowledge_items,
            crate::source::acquisition_ipc::src_create_knowledge_item,
            crate::source::acquisition_ipc::src_list_claim_relations,
            crate::source::acquisition_ipc::src_create_claim_relation,
            crate::source::acquisition_ipc::src_list_frameworks,
            crate::source::acquisition_ipc::src_create_framework,
            crate::source::acquisition_ipc::src_list_framework_versions,
            crate::source::acquisition_ipc::src_list_framework_reviews,
            crate::source::acquisition_ipc::src_create_framework_version,
            crate::source::acquisition_ipc::src_add_framework_item,
            crate::source::acquisition_ipc::src_remove_framework_item,
            crate::source::acquisition_ipc::src_reorder_framework_items,
            crate::source::acquisition_ipc::src_update_framework_version,
            crate::source::acquisition_ipc::src_review_framework_version,
            crate::source::acquisition_ipc::src_publish_framework_version,
            crate::source::acquisition_ipc::src_list_provenance,
            crate::market::ipc::mkt_fetch_local_candles,
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
            mind::media::register_media_source,
            mind::media::import_file_reference,
            mind::media::get_vora_file,
            mind::media::resolve_missing_file,
            mind::media::list_media_files,
            mind::media::get_media_source,
            mind::media::get_last_media_source,
            mind::media::validate_media_source,
            mind::media::update_media_source_duration,
            mind::media::save_media_source_position,
            mind::media::list_media_slices,
            mind::media::save_media_slice,
            mind::media::delete_media_slice,
            mind::media::get_media_slice,
            mind::player::player_spawn,
            mind::player::player_kill,
            mind::player::player_load,
            mind::player::player_load_at,
            mind::player::player_play,
            mind::player::player_pause,
            mind::player::player_set_fullscreen,
            mind::player::player_seek,
            mind::player::player_set_speed,
            mind::player::player_set_volume,
            mind::player::player_get_position,
            mind::player::player_get_duration,
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
