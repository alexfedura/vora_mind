use crate::mind::canvas_db::AppState;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use uuid::Uuid;

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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaSlice {
    pub id: String,
    pub source_id: String,
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
    pub source_id: String,
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

fn fingerprint(path: &Path) -> Result<(i64, i64), String> {
    let metadata = fs::metadata(path).map_err(|error| {
        format!(
            "Cannot read media source metadata for {}: {error}",
            path.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!("Media source is not a file: {}", path.display()));
    }
    let size = i64::try_from(metadata.len()).map_err(|error| error.to_string())?;
    let modified = metadata
        .modified()
        .map_err(|error| format!("Cannot read media source modification time: {error}"))?
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Media source modification time is invalid: {error}"))?;
    let modified_ms = i64::try_from(modified.as_millis()).map_err(|error| error.to_string())?;
    Ok((size, modified_ms))
}

fn source_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MediaSource> {
    Ok(MediaSource {
        id: row.get(0)?,
        path: row.get(1)?,
        fingerprint_size: row.get(2)?,
        fingerprint_modified_ms: row.get(3)?,
        duration_seconds: row.get(4)?,
        last_position_seconds: row.get(5)?,
        created_at: row.get(6)?,
        last_used_at: row.get(7)?,
    })
}

fn get_source(conn: &Connection, source_id: &str) -> rusqlite::Result<Option<MediaSource>> {
    conn.query_row(
        "SELECT id, path, fingerprint_size, fingerprint_modified_ms, duration_seconds,
                last_position_seconds, created_at, last_used_at
         FROM mind_media_sources WHERE id = ?1",
        [source_id],
        source_from_row,
    )
    .optional()
}

fn verify_source(source: &MediaSource) -> Result<(), String> {
    let path = Path::new(&source.path);
    let (size, modified_ms) = fingerprint(path).map_err(|error| {
        if !path.exists() {
            format!("Media source is missing: {}", path.display())
        } else {
            error
        }
    })?;
    if size != source.fingerprint_size || modified_ms != source.fingerprint_modified_ms {
        return Err(format!(
            "Media source changed since it was referenced: {}. Reopen the current file to create a new source reference.",
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
pub fn register_media_source(
    state: State<'_, AppState>,
    path: String,
) -> Result<MediaSource, String> {
    let selected_path = Path::new(&path);
    let canonical_path = selected_path.canonicalize().map_err(|error| {
        format!(
            "Cannot open media source {}: {error}",
            selected_path.display()
        )
    })?;
    let canonical_path = canonical_path
        .to_str()
        .ok_or_else(|| "Media source path cannot be represented as Unicode.".to_string())?
        .to_string();
    let (size, modified_ms) = fingerprint(Path::new(&canonical_path))?;
    let now = now_ms()?;
    let conn = lock_database(&state)?;

    let existing = conn
        .query_row(
            "SELECT id, path, fingerprint_size, fingerprint_modified_ms, duration_seconds,
                    last_position_seconds, created_at, last_used_at
             FROM mind_media_sources
             WHERE path = ?1 AND fingerprint_size = ?2 AND fingerprint_modified_ms = ?3",
            params![canonical_path, size, modified_ms],
            source_from_row,
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if let Some(mut source) = existing {
        conn.execute(
            "UPDATE mind_media_sources SET last_used_at = ?1 WHERE id = ?2",
            params![now, source.id],
        )
        .map_err(|error| error.to_string())?;
        source.last_used_at = now;
        return Ok(source);
    }

    let source = MediaSource {
        id: Uuid::now_v7().to_string(),
        path: canonical_path,
        fingerprint_size: size,
        fingerprint_modified_ms: modified_ms,
        duration_seconds: None,
        last_position_seconds: 0.0,
        created_at: now,
        last_used_at: now,
    };
    conn.execute(
        "INSERT INTO mind_media_sources
            (id, path, fingerprint_size, fingerprint_modified_ms, duration_seconds,
             last_position_seconds, created_at, last_used_at)
         VALUES (?1, ?2, ?3, ?4, NULL, 0, ?5, ?5)",
        params![
            source.id,
            source.path,
            source.fingerprint_size,
            source.fingerprint_modified_ms,
            source.created_at
        ],
    )
    .map_err(|error| error.to_string())?;
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
        .ok_or_else(|| format!("Media source {source_id} was not found in this workspace."))
}

#[tauri::command]
pub fn get_last_media_source(state: State<'_, AppState>) -> Result<Option<MediaSource>, String> {
    let conn = lock_database(&state)?;
    conn.query_row(
        "SELECT id, path, fingerprint_size, fingerprint_modified_ms, duration_seconds,
                last_position_seconds, created_at, last_used_at
         FROM mind_media_sources ORDER BY last_used_at DESC, created_at DESC LIMIT 1",
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
    let source = get_source(&conn, &source_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Media source {source_id} was not found in this workspace."))?;
    verify_source(&source)?;
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
            "UPDATE mind_media_sources SET duration_seconds = ?1 WHERE id = ?2",
            params![duration_seconds, source_id],
        )
        .map_err(|error| error.to_string())?;
    if updated == 0 {
        return Err(format!(
            "Media source {source_id} was not found in this workspace."
        ));
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
            "UPDATE mind_media_sources SET last_position_seconds = ?1, last_used_at = ?2
             WHERE id = ?3",
            params![position_seconds, now_ms()?, source_id],
        )
        .map_err(|error| error.to_string())?;
    if updated == 0 {
        return Err(format!(
            "Media source {source_id} was not found in this workspace."
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn list_media_slices(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<Vec<MediaSlice>, String> {
    let conn = lock_database(&state)?;
    let mut statement = conn
        .prepare(
            "SELECT id, source_id, title, start_seconds, end_seconds, note, created_at, updated_at
             FROM mind_media_slices WHERE source_id = ?1 ORDER BY start_seconds, created_at",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([source_id], |row| {
            Ok(MediaSlice {
                id: row.get(0)?,
                source_id: row.get(1)?,
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
    let source = get_source(&conn, &input.source_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!(
                "Media source {} was not found in this workspace.",
                input.source_id
            )
        })?;
    verify_source(&source)?;
    if source
        .duration_seconds
        .is_some_and(|duration| input.end_seconds > duration)
    {
        return Err("Media reference end time exceeds the source duration.".to_string());
    }

    let now = now_ms()?;
    let id = input.id.unwrap_or_else(|| Uuid::now_v7().to_string());
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    let existing_source_id = tx
        .query_row(
            "SELECT source_id FROM mind_media_slices WHERE id = ?1",
            [&id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if existing_source_id
        .as_ref()
        .is_some_and(|existing| existing != &input.source_id)
    {
        return Err("A media reference cannot be moved to a different source.".to_string());
    }
    if existing_source_id.is_some() {
        tx.execute(
            "UPDATE mind_media_slices
             SET title = ?1, start_seconds = ?2, end_seconds = ?3, note = ?4, updated_at = ?5
             WHERE id = ?6 AND source_id = ?7",
            params![
                title,
                input.start_seconds,
                input.end_seconds,
                input.note,
                now,
                id,
                input.source_id
            ],
        )
        .map_err(|error| error.to_string())?;
    } else {
        tx.execute(
            "INSERT INTO mind_media_slices
                (id, source_id, title, start_seconds, end_seconds, note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                id,
                input.source_id,
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
            "SELECT id, source_id, title, start_seconds, end_seconds, note, created_at, updated_at
             FROM mind_media_slices WHERE id = ?1",
            [&id],
            |row| {
                Ok(MediaSlice {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
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
            "SELECT id, source_id, title, start_seconds, end_seconds, note, created_at, updated_at
             FROM mind_media_slices WHERE id = ?1",
            [&slice_id],
            |row| {
                Ok(MediaSlice {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
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
    let source = get_source(&conn, &slice.source_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!(
                "Media source {} was not found in this workspace.",
                slice.source_id
            )
        })?;
    Ok(MediaSliceWithSource { slice, source })
}

#[cfg(test)]
mod tests {
    use super::{fingerprint, valid_range, verify_source, MediaSource};
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn fingerprint_tracks_size_and_modification_time() {
        let path: PathBuf = std::env::temp_dir().join(format!("vora-media-{}.bin", Uuid::now_v7()));
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(b"original").unwrap();
        drop(file);
        let original = fingerprint(&path).unwrap();
        fs::write(&path, b"changed-size").unwrap();
        let changed = fingerprint(&path).unwrap();
        assert_ne!(original, changed);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn verification_rejects_a_changed_source() {
        let path: PathBuf = std::env::temp_dir().join(format!("vora-media-{}.bin", Uuid::now_v7()));
        fs::write(&path, b"original").unwrap();
        let (size, modified_ms) = fingerprint(&path).unwrap();
        let source = MediaSource {
            id: Uuid::now_v7().to_string(),
            path: path.to_string_lossy().into_owned(),
            fingerprint_size: size,
            fingerprint_modified_ms: modified_ms,
            duration_seconds: None,
            last_position_seconds: 0.0,
            created_at: 0,
            last_used_at: 0,
        };
        fs::write(&path, b"replaced source").unwrap();
        assert!(verify_source(&source).is_err());
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
    fn media_schema_initializes_idempotently() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        crate::mind::canvas_db::init_db(&connection).unwrap();
        crate::mind::canvas_db::init_db(&connection).unwrap();
        let source_table_exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'mind_media_sources')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let slice_table_exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'mind_media_slices')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(source_table_exists);
        assert!(slice_table_exists);
    }
}
