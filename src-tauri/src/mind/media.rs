use crate::mind::canvas_db::AppState;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoraFile {
    pub id: String,
    pub original_name: String,
    pub current_path: String,
    pub file_type: String,
    pub file_hash: String,
    pub size_bytes: i64,
    pub modified_time_ms: i64,
    pub created_at: i64,
    pub duration_seconds: Option<f64>,
    pub last_position_seconds: f64,
    pub last_used_at: i64,
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaSource {
    pub id: String,
    pub path: String,
    pub fingerprint_size: i64,
    pub fingerprint_modified_ms: i64,
    pub duration_seconds: Option<f64>,
    pub last_position_seconds: f64,
    pub created_at: i64,
    pub last_used_at: i64,
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaSlice {
    pub id: String,
    pub file_id: String,
    pub title: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub note: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSliceInput {
    pub id: Option<String>,
    pub file_id: String,
    pub title: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSliceWithSource {
    pub slice: MediaSlice,
    pub source: MediaSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFileLibraryEntry {
    pub file: VoraFile,
    pub slices: Vec<MediaSlice>,
}

fn now_ms() -> Result<i64, String> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?;
    i64::try_from(elapsed.as_millis()).map_err(|error| error.to_string())
}

fn valid_range(start_seconds: f64, end_seconds: f64) -> bool {
    start_seconds.is_finite()
        && end_seconds.is_finite()
        && start_seconds >= 0.0
        && end_seconds > start_seconds
}

fn file_metadata(path: &Path) -> Result<(i64, i64), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Cannot read file metadata for {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("File is not a regular file: {}", path.display()));
    }
    let size = i64::try_from(metadata.len()).map_err(|error| error.to_string())?;
    let modified = metadata
        .modified()
        .map_err(|error| format!("Cannot read file modification time: {error}"))?
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("File modification time is invalid: {error}"))?;
    let modified_ms = i64::try_from(modified.as_millis()).map_err(|error| error.to_string())?;
    Ok((size, modified_ms))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("Cannot open file {} to fingerprint it: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("Cannot fingerprint {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn file_type_for_path(path: &Path) -> Result<String, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "The selected file has no supported extension.".to_string())?;
    let file_type = match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" => "image",
        "mp3" | "wav" | "ogg" | "m4a" | "aac" | "flac" => "audio",
        "mp4" | "webm" | "mov" | "m4v" => "video",
        "pdf" => "pdf",
        "docx" => "docx",
        "md" => "md",
        _ => return Err(format!("Unsupported file extension: .{extension}")),
    };
    Ok(file_type.to_string())
}

fn file_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<VoraFile> {
    let current_path: String = row.get(2)?;
    Ok(VoraFile {
        id: row.get(0)?,
        original_name: row.get(1)?,
        is_available: Path::new(&current_path).is_file(),
        current_path,
        file_type: row.get(3)?,
        file_hash: row.get(4)?,
        size_bytes: row.get(5)?,
        modified_time_ms: row.get(6)?,
        created_at: row.get(7)?,
        duration_seconds: row.get(8)?,
        last_position_seconds: row.get(9)?,
        last_used_at: row.get(10)?,
    })
}

const FILE_COLUMNS: &str = "id, original_name, current_path, file_type, file_hash,
    size_bytes, modified_time_ms, created_at, duration_seconds, last_position_seconds, last_used_at";

fn get_file(conn: &Connection, file_id: &str) -> rusqlite::Result<Option<VoraFile>> {
    conn.query_row(
        &format!("SELECT {FILE_COLUMNS} FROM vora_files WHERE id = ?1"),
        [file_id],
        file_from_row,
    )
    .optional()
}

pub fn ensure_file_reference(
    conn: &Connection,
    path: &Path,
    requested_type: Option<&str>,
) -> Result<VoraFile, String> {
    let canonical_path = path.canonicalize().map_err(|error| {
        format!("Cannot access file {}: {error}", path.display())
    })?;
    let (size_bytes, modified_time_ms) = file_metadata(&canonical_path)?;
    let file_hash = sha256_file(&canonical_path)?;
    let file_type = match requested_type {
        Some(file_type) => {
            if !matches!(file_type, "image" | "audio" | "video" | "pdf" | "docx" | "md") {
                return Err(format!("Unsupported file type: {file_type}"));
            }
            file_type.to_string()
        }
        None => file_type_for_path(&canonical_path)?,
    };
    if let Some(mut file) = conn
        .query_row(
            &format!(
                "SELECT {FILE_COLUMNS} FROM vora_files
                 WHERE current_path = ?1 AND file_hash = ?2 AND size_bytes = ?3"
            ),
            params![canonical_path.to_string_lossy(), file_hash, size_bytes],
            file_from_row,
        )
        .optional()
        .map_err(|error| error.to_string())?
    {
        conn.execute(
            "UPDATE vora_files SET modified_time_ms = ?1 WHERE id = ?2",
            params![modified_time_ms, file.id],
        )
        .map_err(|error| error.to_string())?;
        file.modified_time_ms = modified_time_ms;
        return Ok(file);
    }

    let id = Uuid::now_v7().to_string();
    let original_name = canonical_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "The file name cannot be represented as Unicode.".to_string())?
        .to_string();
    let created_at = now_ms()?;
    conn.execute(
        "INSERT INTO vora_files
            (id, original_name, current_path, file_type, file_hash, size_bytes,
             modified_time_ms, created_at, duration_seconds, last_position_seconds, last_used_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, 0, 0)",
        params![
            id,
            original_name,
            canonical_path.to_string_lossy(),
            file_type,
            file_hash,
            size_bytes,
            modified_time_ms,
            created_at
        ],
    )
    .map_err(|error| error.to_string())?;
    get_file(conn, &id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Could not retrieve imported file {id}."))
}

pub fn migrate_file_references(conn: &Connection) -> Result<(), String> {
    let has_legacy_sources = table_exists(conn, "mind_media_sources")?;
    if has_legacy_sources {
        migrate_legacy_sources(conn)?;
    }
    ensure_document_file_id(conn)?;
    migrate_legacy_documents(conn)?;
    migrate_legacy_slices(conn)?;
    if has_legacy_sources {
        conn.execute("DROP TABLE mind_media_sources", [])
            .map_err(|error| format!("Could not remove migrated media sources: {error}"))?;
    }
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_media_slices_file_time
            ON mind_media_slices(file_id, start_seconds, created_at);
         CREATE INDEX IF NOT EXISTS idx_vora_files_last_used
            ON vora_files(last_used_at DESC, created_at DESC);",
    )
    .map_err(|error| format!("Could not index file references: {error}"))?;
    Ok(())
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [table],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2
         )",
        params![table, column],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

fn legacy_file_reference(
    conn: &Connection,
    path: &str,
    file_type: &str,
    size_bytes: i64,
    modified_time_ms: i64,
) -> Result<VoraFile, String> {
    let source_path = Path::new(path);
    if source_path.is_file() {
        return ensure_file_reference(conn, source_path, Some(file_type));
    }
    let id = Uuid::now_v7().to_string();
    let original_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Missing file")
        .to_string();
    let created_at = now_ms()?;
    conn.execute(
        "INSERT INTO vora_files
            (id, original_name, current_path, file_type, file_hash, size_bytes,
             modified_time_ms, created_at, duration_seconds, last_position_seconds, last_used_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, 0, 0)",
        params![
            id,
            original_name,
            path,
            file_type,
            format!("legacy:{size_bytes}:{modified_time_ms}"),
            size_bytes,
            modified_time_ms,
            created_at
        ],
    )
    .map_err(|error| error.to_string())?;
    get_file(conn, &id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Could not migrate file reference {id}."))
}

fn migrate_legacy_sources(conn: &Connection) -> Result<(), String> {
    let mut statement = conn
        .prepare(
            "SELECT id, path, fingerprint_size, fingerprint_modified_ms, duration_seconds,
                    last_position_seconds, created_at, last_used_at
             FROM mind_media_sources",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<f64>>(4)?,
                row.get::<_, f64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let legacy_sources = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);

    for (id, path, size, modified, duration, position, created, last_used) in legacy_sources {
        let file_type = file_type_for_path(Path::new(&path)).unwrap_or_else(|_| "video".to_string());
        if Path::new(&path).is_file() {
            let (actual_size, actual_modified) = file_metadata(Path::new(&path))?;
            let hash = sha256_file(Path::new(&path))?;
            conn.execute(
                "INSERT OR IGNORE INTO vora_files
                    (id, original_name, current_path, file_type, file_hash, size_bytes,
                     modified_time_ms, created_at, duration_seconds, last_position_seconds, last_used_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    id,
                    Path::new(&path).file_name().and_then(|name| name.to_str()).unwrap_or("Media file"),
                    path,
                    file_type,
                    hash,
                    actual_size,
                    actual_modified,
                    created,
                    duration,
                    position,
                    last_used
                ],
            )
            .map_err(|error| error.to_string())?;
        } else {
            conn.execute(
                "INSERT OR IGNORE INTO vora_files
                    (id, original_name, current_path, file_type, file_hash, size_bytes,
                     modified_time_ms, created_at, duration_seconds, last_position_seconds, last_used_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    id,
                    Path::new(&path).file_name().and_then(|name| name.to_str()).unwrap_or("Media file"),
                    path,
                    file_type,
                    format!("legacy:{size}:{modified}"),
                    size,
                    modified,
                    created,
                    duration,
                    position,
                    last_used
                ],
            )
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn ensure_document_file_id(conn: &Connection) -> Result<(), String> {
    if !column_exists(conn, "documents", "file_id")? {
        conn.execute(
            "ALTER TABLE documents ADD COLUMN file_id TEXT REFERENCES vora_files(id)",
            [],
        )
        .map_err(|error| format!("Could not add document file identity: {error}"))?;
    }
    Ok(())
}

fn migrate_legacy_documents(conn: &Connection) -> Result<(), String> {
    if !column_exists(conn, "documents", "file_path")? {
        return Ok(());
    }
    let mut statement = conn
        .prepare(
            "SELECT id, file_path, file_type FROM documents
             WHERE file_id IS NULL OR file_id = ''",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let documents = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    for (document_id, path, file_type) in documents {
        let file = legacy_file_reference(conn, &path, &file_type, 0, 0)?;
        conn.execute(
            "UPDATE documents SET file_id = ?1 WHERE id = ?2",
            params![file.id, document_id],
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn migrate_legacy_slices(conn: &Connection) -> Result<(), String> {
    let has_source_id = column_exists(conn, "mind_media_slices", "source_id")?;
    let has_file_id = column_exists(conn, "mind_media_slices", "file_id")?;
    if !has_source_id || has_file_id {
        return Ok(());
    }

    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .map_err(|error| error.to_string())?;
    let migration = conn.execute_batch(
        "BEGIN IMMEDIATE;
         ALTER TABLE mind_media_slices RENAME TO mind_media_slices_legacy;
         CREATE TABLE mind_media_slices (
            id TEXT PRIMARY KEY,
            file_id TEXT NOT NULL REFERENCES vora_files(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            start_seconds REAL NOT NULL CHECK(start_seconds >= 0),
            end_seconds REAL NOT NULL CHECK(end_seconds > start_seconds),
            note TEXT NOT NULL DEFAULT '',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
         );
         INSERT INTO mind_media_slices
            (id, file_id, title, start_seconds, end_seconds, note, created_at, updated_at)
         SELECT id, source_id, title, start_seconds, end_seconds, note, created_at, updated_at
         FROM mind_media_slices_legacy;
         DROP TABLE mind_media_slices_legacy;
         COMMIT;",
    );
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| error.to_string())?;
    migration.map_err(|error| format!("Could not migrate media annotations: {error}"))
}

fn source_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MediaSource> {
    let path: String = row.get(1)?;
    Ok(MediaSource {
        id: row.get(0)?,
        is_available: Path::new(&path).is_file(),
        path,
        fingerprint_size: row.get(2)?,
        fingerprint_modified_ms: row.get(3)?,
        duration_seconds: row.get(4)?,
        last_position_seconds: row.get(5)?,
        created_at: row.get(6)?,
        last_used_at: row.get(7)?,
    })
}

fn source_from_file(file: VoraFile) -> MediaSource {
    MediaSource {
        id: file.id,
        path: file.current_path,
        fingerprint_size: file.size_bytes,
        fingerprint_modified_ms: file.modified_time_ms,
        duration_seconds: file.duration_seconds,
        last_position_seconds: file.last_position_seconds,
        created_at: file.created_at,
        last_used_at: file.last_used_at,
        is_available: file.is_available,
    }
}

fn get_source(conn: &Connection, file_id: &str) -> rusqlite::Result<Option<MediaSource>> {
    get_file(conn, file_id).map(|file| file.map(source_from_file))
}

fn verify_source(source: &MediaSource, file_hash: &str) -> Result<(), String> {
    let path = Path::new(&source.path);
    if !path.is_file() {
        return Err(format!("Media file is missing: {}", path.display()));
    }
    let (size, _) = file_metadata(path)?;
    if size != source.fingerprint_size || sha256_file(path)? != file_hash {
        return Err(format!(
            "Media file contents changed since import: {}.",
            path.display()
        ));
    }
    Ok(())
}

fn lock_database<'a>(
    state: &'a State<'a, AppState>,
) -> Result<std::sync::MutexGuard<'a, Connection>, String> {
    state
        .db
        .lock()
        .map_err(|error| format!("Could not lock the Mind database: {error}"))
}

#[tauri::command]
pub fn import_file_reference(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let conn = lock_database(&state)?;
    let file = ensure_file_reference(&conn, Path::new(&path), None)?;
    Ok(file.id)
}

#[tauri::command]
pub fn get_vora_file(state: State<'_, AppState>, file_id: String) -> Result<VoraFile, String> {
    let conn = lock_database(&state)?;
    get_file(&conn, &file_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("File {file_id} was not found in this workspace."))
}

#[tauri::command]
pub fn resolve_missing_file(
    state: State<'_, AppState>,
    file_id: String,
    search_dir: String,
) -> Result<String, String> {
    let search_root = PathBuf::from(&search_dir)
        .canonicalize()
        .map_err(|error| format!("Cannot access search directory {search_dir}: {error}"))?;
    if !search_root.is_dir() {
        return Err(format!("Search path is not a directory: {}", search_root.display()));
    }
    let conn = lock_database(&state)?;
    let file = get_file(&conn, &file_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("File {file_id} was not found in this workspace."))?;
    for entry in WalkDir::new(&search_root).follow_links(false) {
        let entry = entry.map_err(|error| format!("Could not search files: {error}"))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let candidate = entry.path();
        let Ok((size, modified_time_ms)) = file_metadata(candidate) else {
            continue;
        };
        if size != file.size_bytes {
            continue;
        }
        let matches = if let Some((legacy_size, legacy_modified)) = file.file_hash
            .strip_prefix("legacy:")
            .and_then(|value| value.split_once(':'))
        {
            legacy_size.parse::<i64>().ok() == Some(size)
                && legacy_modified.parse::<i64>().ok() == Some(modified_time_ms)
        } else {
            sha256_file(candidate)? == file.file_hash
        };
        if !matches {
            continue;
        }
        let canonical_path = candidate
            .canonicalize()
            .map_err(|error| format!("Could not resolve matched file path: {error}"))?;
        conn.execute(
            "UPDATE vora_files SET current_path = ?1, modified_time_ms = ?2 WHERE id = ?3",
            params![canonical_path.to_string_lossy(), modified_time_ms, file_id],
        )
        .map_err(|error| error.to_string())?;
        return canonical_path
            .to_str()
            .map(str::to_string)
            .ok_or_else(|| "Matched file path cannot be represented as Unicode.".to_string());
    }
    Err(format!(
        "Could not locate {} in {}. No file with matching content was found.",
        file.original_name,
        search_root.display()
    ))
}

#[tauri::command]
pub fn list_media_files(
    state: State<'_, AppState>,
) -> Result<Vec<MediaFileLibraryEntry>, String> {
    let conn = lock_database(&state)?;
    let files = {
        let mut statement = conn
            .prepare(&format!("SELECT {FILE_COLUMNS} FROM vora_files ORDER BY original_name COLLATE NOCASE, id"))
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], file_from_row)
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?
    };
    files
        .into_iter()
        .map(|file| {
            let slices = list_slices(&conn, &file.id)?;
            Ok(MediaFileLibraryEntry { file, slices })
        })
        .collect()
}

fn list_slices(conn: &Connection, file_id: &str) -> Result<Vec<MediaSlice>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, file_id, title, start_seconds, end_seconds, note, created_at, updated_at
             FROM mind_media_slices WHERE file_id = ?1 ORDER BY start_seconds, created_at",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([file_id], |row| {
            Ok(MediaSlice {
                id: row.get(0)?,
                file_id: row.get(1)?,
                title: row.get(2)?,
                start_seconds: row.get(3)?,
                end_seconds: row.get(4)?,
                note: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn register_media_source(
    state: State<'_, AppState>,
    path: String,
) -> Result<MediaSource, String> {
    let now = now_ms()?;
    let conn = lock_database(&state)?;
    let file = ensure_file_reference(&conn, Path::new(&path), None)?;
    conn.execute(
        "UPDATE vora_files SET last_used_at = ?1 WHERE id = ?2",
        params![now, file.id],
    )
    .map_err(|error| error.to_string())?;
    let mut source = source_from_file(file);
    source.last_used_at = now;
    Ok(source)
}

#[tauri::command]
pub fn get_media_source(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<MediaSource, String> {
    let conn = lock_database(&state)?;
    get_source(&conn, &source_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Media file {source_id} was not found in this workspace."))
}

#[tauri::command]
pub fn get_last_media_source(state: State<'_, AppState>) -> Result<Option<MediaSource>, String> {
    let conn = lock_database(&state)?;
    conn.query_row(
        "SELECT id, current_path, size_bytes, modified_time_ms, duration_seconds,
                last_position_seconds, created_at, last_used_at
         FROM vora_files WHERE file_type IN ('audio', 'video') AND last_used_at > 0
         ORDER BY last_used_at DESC, created_at DESC LIMIT 1",
        [],
        source_from_row,
    )
    .optional()
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn validate_media_source(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<MediaSource, String> {
    let conn = lock_database(&state)?;
    let file = get_file(&conn, &source_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Media file {source_id} was not found in this workspace."))?;
    let source = source_from_file(file.clone());
    verify_source(&source, &file.file_hash)?;
    Ok(source)
}

#[tauri::command]
pub fn update_media_source_duration(
    state: State<'_, AppState>,
    source_id: String,
    duration_seconds: f64,
) -> Result<(), String> {
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err("Media duration must be a finite positive number.".to_string());
    }
    let conn = lock_database(&state)?;
    let updated = conn
        .execute(
            "UPDATE vora_files SET duration_seconds = ?1 WHERE id = ?2",
            params![duration_seconds, source_id],
        )
        .map_err(|error| error.to_string())?;
    if updated == 0 {
        return Err(format!("Media file {source_id} was not found in this workspace."));
    }
    Ok(())
}

#[tauri::command]
pub fn save_media_source_position(
    state: State<'_, AppState>,
    source_id: String,
    position_seconds: f64,
) -> Result<(), String> {
    if !position_seconds.is_finite() || position_seconds < 0.0 {
        return Err("Playback position must be a finite non-negative number.".to_string());
    }
    let conn = lock_database(&state)?;
    let updated = conn
        .execute(
            "UPDATE vora_files SET last_position_seconds = ?1, last_used_at = ?2 WHERE id = ?3",
            params![position_seconds, now_ms()?, source_id],
        )
        .map_err(|error| error.to_string())?;
    if updated == 0 {
        return Err(format!("Media file {source_id} was not found in this workspace."));
    }
    Ok(())
}

#[tauri::command]
pub fn list_media_slices(
    state: State<'_, AppState>,
    file_id: String,
) -> Result<Vec<MediaSlice>, String> {
    let conn = lock_database(&state)?;
    list_slices(&conn, &file_id)
}

#[tauri::command]
pub fn save_media_slice(
    state: State<'_, AppState>,
    input: MediaSliceInput,
) -> Result<MediaSlice, String> {
    if !valid_range(input.start_seconds, input.end_seconds) {
        return Err(
            "The media reference requires finite times with end greater than start.".to_string(),
        );
    }
    let title = input.title.trim();
    if title.is_empty() {
        return Err("Media reference title cannot be empty.".to_string());
    }

    let mut conn = lock_database(&state)?;
    let file = get_file(&conn, &input.file_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("File {} was not found in this workspace.", input.file_id))?;
    verify_source(&source_from_file(file.clone()), &file.file_hash)?;
    if file
        .duration_seconds
        .is_some_and(|duration| input.end_seconds > duration)
    {
        return Err("Media reference end time exceeds the source duration.".to_string());
    }

    let now = now_ms()?;
    let id = input.id.unwrap_or_else(|| Uuid::now_v7().to_string());
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    let existing_file_id = tx
        .query_row(
            "SELECT file_id FROM mind_media_slices WHERE id = ?1",
            [&id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if existing_file_id
        .as_ref()
        .is_some_and(|existing| existing != &input.file_id)
    {
        return Err("A media reference cannot be moved to a different file.".to_string());
    }
    if existing_file_id.is_some() {
        tx.execute(
            "UPDATE mind_media_slices
             SET title = ?1, start_seconds = ?2, end_seconds = ?3, note = ?4, updated_at = ?5
             WHERE id = ?6 AND file_id = ?7",
            params![
                title,
                input.start_seconds,
                input.end_seconds,
                input.note,
                now,
                id,
                input.file_id
            ],
        )
        .map_err(|error| error.to_string())?;
    } else {
        tx.execute(
            "INSERT INTO mind_media_slices
                (id, file_id, title, start_seconds, end_seconds, note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                id,
                input.file_id,
                title,
                input.start_seconds,
                input.end_seconds,
                input.note,
                now
            ],
        )
        .map_err(|error| error.to_string())?;
    }
    let slice = tx
        .query_row(
            "SELECT id, file_id, title, start_seconds, end_seconds, note, created_at, updated_at
             FROM mind_media_slices WHERE id = ?1",
            [&id],
            |row| {
                Ok(MediaSlice {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    title: row.get(2)?,
                    start_seconds: row.get(3)?,
                    end_seconds: row.get(4)?,
                    note: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(slice)
}

#[tauri::command]
pub fn delete_media_slice(state: State<'_, AppState>, slice_id: String) -> Result<(), String> {
    let conn = lock_database(&state)?;
    let deleted = conn
        .execute("DELETE FROM mind_media_slices WHERE id = ?1", [slice_id])
        .map_err(|error| error.to_string())?;
    if deleted == 0 {
        return Err("The media reference no longer exists.".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn get_media_slice(
    state: State<'_, AppState>,
    slice_id: String,
) -> Result<MediaSliceWithSource, String> {
    let conn = lock_database(&state)?;
    let slice = conn
        .query_row(
            "SELECT id, file_id, title, start_seconds, end_seconds, note, created_at, updated_at
             FROM mind_media_slices WHERE id = ?1",
            [&slice_id],
            |row| {
                Ok(MediaSlice {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    title: row.get(2)?,
                    start_seconds: row.get(3)?,
                    end_seconds: row.get(4)?,
                    note: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Media reference {slice_id} was not found in this workspace."))?;
    let source = get_source(&conn, &slice.file_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("File {} was not found in this workspace.", slice.file_id))?;
    Ok(MediaSliceWithSource { slice, source })
}

#[cfg(test)]
mod tests {
    use super::{file_metadata, migrate_file_references, sha256_file, valid_range};
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn fingerprint_tracks_content_and_metadata() {
        let path: PathBuf = std::env::temp_dir().join(format!("vora-file-{}.bin", Uuid::now_v7()));
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(b"original").unwrap();
        drop(file);
        let original_hash = sha256_file(&path).unwrap();
        let original_metadata = file_metadata(&path).unwrap();
        fs::write(&path, b"changed-size").unwrap();
        assert_ne!(original_hash, sha256_file(&path).unwrap());
        assert_ne!(original_metadata.0, file_metadata(&path).unwrap().0);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_non_finite_and_reversed_time_ranges() {
        assert!(!valid_range(f64::NAN, 4.0));
        assert!(!valid_range(5.0, 5.0));
        assert!(!valid_range(6.0, 4.0));
        assert!(!valid_range(-1.0, 4.0));
        assert!(valid_range(0.0, 4.0));
    }

    #[test]
    fn migrates_existing_player_sources_and_slices() {
        let path = std::env::temp_dir().join(format!("vora-legacy-{}.mp4", Uuid::now_v7()));
        fs::write(&path, b"old media bytes").unwrap();
        let now = 1_700_000_000_000_i64;
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE mind_media_sources (
                    id TEXT PRIMARY KEY, path TEXT NOT NULL, fingerprint_size INTEGER NOT NULL,
                    fingerprint_modified_ms INTEGER NOT NULL, duration_seconds REAL,
                    last_position_seconds REAL NOT NULL, created_at INTEGER NOT NULL,
                    last_used_at INTEGER NOT NULL);
                 CREATE TABLE mind_media_slices (
                    id TEXT PRIMARY KEY, source_id TEXT NOT NULL, title TEXT NOT NULL,
                    start_seconds REAL NOT NULL, end_seconds REAL NOT NULL, note TEXT NOT NULL,
                    created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
                    FOREIGN KEY(source_id) REFERENCES mind_media_sources(id));
                 CREATE TABLE documents (
                    id TEXT PRIMARY KEY, title TEXT NOT NULL, file_path TEXT NOT NULL UNIQUE,
                    file_type TEXT NOT NULL, page_count INTEGER, created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL);",
            )
            .unwrap();
        let (size, modified) = file_metadata(&path).unwrap();
        connection
            .execute(
                "INSERT INTO mind_media_sources VALUES ('file-1', ?1, ?2, ?3, 60, 3, ?4, ?4)",
                rusqlite::params![path.to_string_lossy(), size, modified, now],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO mind_media_slices VALUES ('slice-1', 'file-1', 'Note', 1, 5, 'Text', ?1, ?1)",
                [now],
            )
            .unwrap();
        crate::mind::canvas_db::init_db(&connection).unwrap();
        crate::mind::canvas_db::init_db(&connection).unwrap();
        let file_hash: String = connection
            .query_row(
                "SELECT file_hash FROM vora_files WHERE id = 'file-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(file_hash, sha256_file(&path).unwrap());
        let migrated_file_id: String = connection
            .query_row(
                "SELECT file_id FROM mind_media_slices WHERE id = 'slice-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(migrated_file_id, "file-1");
        fs::remove_file(path).unwrap();
    }
}
