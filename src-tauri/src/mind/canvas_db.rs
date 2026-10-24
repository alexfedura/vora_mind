use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub db_path: Mutex<Option<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardSummary {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub active_version_id: String,
    pub active_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInput {
    pub id: String,
    #[serde(default = "default_node_type")]
    pub node_type: String,
    pub title: String,
    pub content: String,
    pub version: String,
    #[serde(default)]
    pub data: serde_json::Value,
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub style: Option<String>,
}

fn default_node_type() -> String {
    "voraCard".to_string()
}

fn object_type_for_node(node_type: &str) -> &'static str {
    match node_type {
        "voraCard" => "card",
        "voraText" => "text",
        "voraShape" => "shape",
        _ => "canvas_node",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeInput {
    pub id: String,
    pub source: String,
    pub target: String,
    pub link_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardVersionCard {
    pub id: String,
    pub title: String,
    pub content: String,
    pub version: String,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BoardVersionSnapshot {
    pub nodes: Vec<NodeInput>,
    pub edges: Vec<EdgeInput>,
    pub cards: Vec<BoardVersionCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardVersionSummary {
    pub id: String,
    pub board_id: String,
    pub major: i32,
    pub minor: i32,
    pub patch: i32,
    pub version: String,
    pub parent_version_id: Option<String>,
    pub forked_from_version_id: Option<String>,
    pub forked_from_version: Option<String>,
    pub created_at: i64,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub name: String,
    pub parent_snapshot_id: Option<String>,
    pub depth: i32,
    pub is_checkpoint: i32,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextLink {
    pub id: String,
    pub source_board_id: String,
    pub source_card_id: String,
    pub source_field: String,
    pub source_start: i64,
    pub source_end: i64,
    pub target_board_id: String,
    pub target_card_id: String,
    pub target_field: String,
    pub target_start: i64,
    pub target_end: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextLinkInput {
    pub source_board_id: String,
    pub source_card_id: String,
    pub source_field: String,
    pub source_start: i64,
    pub source_end: i64,
    pub target_board_id: String,
    pub target_card_id: String,
    pub target_field: String,
    pub target_start: i64,
    pub target_end: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChangeRange {
    pub from: i64,
    pub to: i64,
    pub inserted_length: i64,
    pub inserted_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChangeSet {
    pub field: String,
    pub changes: Vec<TextChangeRange>,
}

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA temp_store = MEMORY;

        CREATE TABLE IF NOT EXISTS mind_boards (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            active_version_id TEXT,
            branch_origin_version_id TEXT,
            library_item_group_id TEXT
        );

        CREATE TABLE IF NOT EXISTS mind_board_versions (
            id TEXT PRIMARY KEY,
            board_id TEXT NOT NULL,
            version_major INTEGER NOT NULL,
            version_minor INTEGER NOT NULL,
            version_patch INTEGER NOT NULL,
            parent_version_id TEXT,
            forked_from_version_id TEXT,
            snapshot_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(board_id) REFERENCES mind_boards(id),
            FOREIGN KEY(parent_version_id) REFERENCES mind_board_versions(id),
            FOREIGN KEY(forked_from_version_id) REFERENCES mind_board_versions(id)
        );

        CREATE TABLE IF NOT EXISTS library_item_groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            snapshot_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            deleted_at INTEGER
        );

        CREATE TABLE IF NOT EXISTS library_item_group_versions (
            id TEXT PRIMARY KEY,
            group_id TEXT NOT NULL,
            version TEXT NOT NULL,
            snapshot_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            UNIQUE(group_id, version),
            FOREIGN KEY(group_id) REFERENCES library_item_groups(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS objects (
            id TEXT PRIMARY KEY,
            type TEXT NOT NULL,
            board_id TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            deleted_at INTEGER NULL,
            metadata JSON
        );

        CREATE TABLE IF NOT EXISTS object_versions (
            id TEXT PRIMARY KEY,
            object_id TEXT NOT NULL,
            version_major INTEGER NOT NULL,
            version_minor INTEGER NOT NULL,
            version_patch INTEGER NOT NULL,
            parent_version_id TEXT,
            reason TEXT,
            summary TEXT,
            content JSON NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(object_id) REFERENCES objects(id),
            FOREIGN KEY(parent_version_id) REFERENCES object_versions(id)
        );

        CREATE TABLE IF NOT EXISTS mind_snapshots (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            parent_snapshot_id TEXT,
            board_id TEXT NOT NULL,
            depth INTEGER NOT NULL,
            replay_cost INTEGER NOT NULL DEFAULT 0,
            is_checkpoint INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(parent_snapshot_id) REFERENCES mind_snapshots(id)
        );

        CREATE TABLE IF NOT EXISTS snapshot_object_changes (
            snapshot_id TEXT NOT NULL,
            object_id TEXT NOT NULL,
            operation TEXT NOT NULL,
            object_version_id TEXT,
            PRIMARY KEY (snapshot_id, object_id),
            FOREIGN KEY(snapshot_id) REFERENCES mind_snapshots(id),
            FOREIGN KEY(object_id) REFERENCES objects(id)
        ) WITHOUT ROWID;

        CREATE TABLE IF NOT EXISTS snapshot_placement_changes (
            snapshot_id TEXT NOT NULL,
            object_id TEXT NOT NULL,
            operation TEXT NOT NULL,
            x REAL,
            y REAL,
            width REAL,
            height REAL,
            rotation REAL DEFAULT 0,
            z_index INTEGER DEFAULT 0,
            style JSON,
            PRIMARY KEY (snapshot_id, object_id),
            FOREIGN KEY(snapshot_id) REFERENCES mind_snapshots(id),
            FOREIGN KEY(object_id) REFERENCES objects(id)
        ) WITHOUT ROWID;

        CREATE TABLE IF NOT EXISTS snapshot_link_changes (
            snapshot_id TEXT NOT NULL,
            link_id TEXT NOT NULL,
            operation TEXT NOT NULL,
            source_object_id TEXT,
            target_object_id TEXT,
            link_type TEXT,
            PRIMARY KEY (snapshot_id, link_id),
            FOREIGN KEY(snapshot_id) REFERENCES mind_snapshots(id)
        ) WITHOUT ROWID;

        CREATE TABLE IF NOT EXISTS checkpoint_objects (
            checkpoint_id TEXT NOT NULL,
            object_id TEXT NOT NULL,
            object_version_id TEXT NOT NULL,
            PRIMARY KEY (checkpoint_id, object_id),
            FOREIGN KEY(checkpoint_id) REFERENCES mind_snapshots(id)
        ) WITHOUT ROWID;

        CREATE TABLE IF NOT EXISTS checkpoint_placements (
            checkpoint_id TEXT NOT NULL,
            object_id TEXT NOT NULL,
            x REAL NOT NULL,
            y REAL NOT NULL,
            width REAL NOT NULL,
            height REAL NOT NULL,
            rotation REAL DEFAULT 0,
            z_index INTEGER DEFAULT 0,
            style JSON,
            PRIMARY KEY (checkpoint_id, object_id),
            FOREIGN KEY(checkpoint_id) REFERENCES mind_snapshots(id)
        ) WITHOUT ROWID;

        CREATE TABLE IF NOT EXISTS checkpoint_links (
            checkpoint_id TEXT NOT NULL,
            link_id TEXT NOT NULL,
            source_object_id TEXT NOT NULL,
            target_object_id TEXT NOT NULL,
            link_type TEXT NOT NULL,
            PRIMARY KEY (checkpoint_id, link_id),
            FOREIGN KEY(checkpoint_id) REFERENCES mind_snapshots(id)
        ) WITHOUT ROWID;

        CREATE TABLE IF NOT EXISTS mind_text_links (
            id TEXT PRIMARY KEY,
            source_board_id TEXT NOT NULL,
            source_card_id TEXT NOT NULL,
            source_field TEXT NOT NULL CHECK(source_field IN ('title', 'content')),
            source_start INTEGER NOT NULL,
            source_end INTEGER NOT NULL,
            target_board_id TEXT NOT NULL,
            target_card_id TEXT NOT NULL,
            target_field TEXT NOT NULL CHECK(target_field IN ('title', 'content')),
            target_start INTEGER NOT NULL,
            target_end INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            CHECK(source_start >= 0 AND source_end > source_start),
            CHECK(target_start >= 0 AND target_end > target_start)
        );

        CREATE INDEX IF NOT EXISTS idx_snapshots_parent ON mind_snapshots(parent_snapshot_id);
        CREATE INDEX IF NOT EXISTS idx_snapshots_checkpoint ON mind_snapshots(is_checkpoint);
        CREATE INDEX IF NOT EXISTS idx_snapshots_depth ON mind_snapshots(depth);
        "#,
    )?;
    ensure_board_column(conn, "objects")?;
    ensure_board_column(conn, "mind_snapshots")?;
    ensure_board_version_columns(conn)?;
    ensure_library_item_group_board_column(conn)?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_objects_board ON objects(board_id);
         CREATE INDEX IF NOT EXISTS idx_snapshots_board ON mind_snapshots(board_id, created_at);
         CREATE INDEX IF NOT EXISTS idx_board_versions_history ON mind_board_versions(board_id, created_at, id);
         CREATE UNIQUE INDEX IF NOT EXISTS idx_mind_boards_library_group ON mind_boards(library_item_group_id)
             WHERE library_item_group_id IS NOT NULL;
         CREATE INDEX IF NOT EXISTS idx_library_item_groups_created ON library_item_groups(created_at, id);
         CREATE INDEX IF NOT EXISTS idx_library_item_group_versions_history ON library_item_group_versions(group_id, created_at, id);
         CREATE INDEX IF NOT EXISTS idx_text_links_source ON mind_text_links(source_board_id, source_card_id, source_field, source_start);
         CREATE INDEX IF NOT EXISTS idx_text_links_target ON mind_text_links(target_board_id, target_card_id, target_field, target_start);",
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO library_item_group_versions
            (id, group_id, version, snapshot_json, created_at)
         SELECT 'item_group_version_' || id || '_' || version,
                id, version, snapshot_json, updated_at
         FROM library_item_groups",
        [],
    )?;
    let has_legacy_board_content: bool = conn.query_row(
        "SELECT
            EXISTS(SELECT 1 FROM objects WHERE board_id = 'board-1')
            OR EXISTS(SELECT 1 FROM mind_snapshots WHERE board_id = 'board-1')
            OR EXISTS(SELECT 1 FROM mind_board_versions WHERE board_id = 'board-1')
            OR EXISTS(SELECT 1 FROM mind_text_links
                WHERE source_board_id = 'board-1' OR target_board_id = 'board-1')",
        [],
        |row| row.get(0),
    )?;
    if has_legacy_board_content {
        conn.execute(
            "INSERT OR IGNORE INTO mind_boards (id, title, created_at, updated_at)
             VALUES ('board-1', 'My desk', 0, 0)",
            [],
        )?;
    }
    ensure_board_version_baselines(conn)?;
    Ok(())
}

fn card_field_text(
    conn: &Connection,
    board_id: &str,
    card_id: &str,
    field: &str,
) -> Result<String> {
    if !matches!(field, "title" | "content") {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let json_path = if field == "title" {
        "$.title"
    } else {
        "$.content"
    };
    let query = format!(
        "SELECT COALESCE(json_extract(v.content, ?1), '')
         FROM objects o
         LEFT JOIN object_versions v ON v.id = (
             SELECT id FROM object_versions WHERE object_id = o.id ORDER BY created_at DESC LIMIT 1
         )
         WHERE o.id = ?2 AND o.board_id = ?3 AND o.type = 'card' AND o.deleted_at IS NULL"
    );
    conn.query_row(&query, params![json_path, card_id, board_id], |row| {
        row.get(0)
    })
}

fn utf16_offset_to_byte(text: &str, offset: i64) -> Option<usize> {
    if offset < 0 {
        return None;
    }
    let mut units = 0_i64;
    for (byte, character) in text.char_indices() {
        if units == offset {
            return Some(byte);
        }
        units += character.len_utf16() as i64;
        if units > offset {
            return None;
        }
    }
    (units == offset).then_some(text.len())
}

fn validate_text_range(text: &str, start: i64, end: i64) -> Result<()> {
    if end <= start
        || utf16_offset_to_byte(text, start).is_none()
        || utf16_offset_to_byte(text, end).is_none()
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

fn apply_validated_text_changes(text: &str, changes: &[TextChangeRange]) -> Result<String> {
    let mut previous_end = -1_i64;
    let mut byte_ranges = Vec::with_capacity(changes.len());
    for change in changes {
        if change.from < previous_end
            || change.from < 0
            || change.to < change.from
            || change.inserted_length < 0
            || change.inserted_text.encode_utf16().count() as i64 != change.inserted_length
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let from = utf16_offset_to_byte(text, change.from).ok_or(rusqlite::Error::InvalidQuery)?;
        let to = utf16_offset_to_byte(text, change.to).ok_or(rusqlite::Error::InvalidQuery)?;
        byte_ranges.push((from, to, change.inserted_text.as_str()));
        previous_end = change.to;
    }

    let mut updated = text.to_string();
    for (from, to, inserted_text) in byte_ranges.into_iter().rev() {
        updated.replace_range(from..to, inserted_text);
    }
    Ok(updated)
}

pub fn create_text_link(
    conn: &mut Connection,
    input: TextLinkInput,
    now: i64,
    id: &str,
) -> Result<TextLink> {
    let tx = conn.transaction()?;
    let source_text = card_field_text(
        &tx,
        &input.source_board_id,
        &input.source_card_id,
        &input.source_field,
    )?;
    let target_text = card_field_text(
        &tx,
        &input.target_board_id,
        &input.target_card_id,
        &input.target_field,
    )?;
    validate_text_range(&source_text, input.source_start, input.source_end)?;
    validate_text_range(&target_text, input.target_start, input.target_end)?;

    tx.execute(
        "INSERT INTO mind_text_links (
            id, source_board_id, source_card_id, source_field, source_start, source_end,
            target_board_id, target_card_id, target_field, target_start, target_end,
            created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
        params![
            id,
            input.source_board_id,
            input.source_card_id,
            input.source_field,
            input.source_start,
            input.source_end,
            input.target_board_id,
            input.target_card_id,
            input.target_field,
            input.target_start,
            input.target_end,
            now
        ],
    )?;
    tx.commit()?;
    Ok(TextLink {
        id: id.to_string(),
        source_board_id: input.source_board_id,
        source_card_id: input.source_card_id,
        source_field: input.source_field,
        source_start: input.source_start,
        source_end: input.source_end,
        target_board_id: input.target_board_id,
        target_card_id: input.target_card_id,
        target_field: input.target_field,
        target_start: input.target_start,
        target_end: input.target_end,
        created_at: now,
        updated_at: now,
    })
}

fn read_text_links_for_card(
    conn: &Connection,
    board_id: &str,
    card_id: &str,
) -> Result<Vec<TextLink>> {
    let mut statement = conn.prepare(
        "SELECT id, source_board_id, source_card_id, source_field, source_start, source_end,
                target_board_id, target_card_id, target_field, target_start, target_end,
                created_at, updated_at
         FROM mind_text_links
         WHERE (source_board_id = ?1 AND source_card_id = ?2)
            OR (target_board_id = ?1 AND target_card_id = ?2)
         ORDER BY created_at, id",
    )?;
    let rows = statement.query_map(params![board_id, card_id], |row| {
        Ok(TextLink {
            id: row.get(0)?,
            source_board_id: row.get(1)?,
            source_card_id: row.get(2)?,
            source_field: row.get(3)?,
            source_start: row.get(4)?,
            source_end: row.get(5)?,
            target_board_id: row.get(6)?,
            target_card_id: row.get(7)?,
            target_field: row.get(8)?,
            target_start: row.get(9)?,
            target_end: row.get(10)?,
            created_at: row.get(11)?,
            updated_at: row.get(12)?,
        })
    })?;
    rows.collect()
}

pub fn get_text_links_for_card(
    conn: &Connection,
    board_id: &str,
    card_id: &str,
) -> Result<Vec<TextLink>> {
    read_text_links_for_card(conn, board_id, card_id)
}

pub fn delete_text_link(conn: &Connection, id: &str) -> Result<()> {
    let changed = conn.execute("DELETE FROM mind_text_links WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

fn remap_text_range(start: i64, end: i64, changes: &[TextChangeRange]) -> Option<(i64, i64)> {
    let mut delta = 0_i64;
    for change in changes {
        if change.from == change.to {
            if change.from > start && change.from < end {
                return None;
            }
            if change.from <= start {
                delta += change.inserted_length;
            }
        } else if change.from < end && change.to > start {
            return None;
        } else if change.to <= start {
            delta += change.inserted_length - (change.to - change.from);
        }
    }
    Some((start + delta, end + delta))
}

pub fn apply_text_link_changes(
    tx: &rusqlite::Transaction<'_>,
    board_id: &str,
    card_id: &str,
    change_sets: &[TextChangeSet],
    now: i64,
) -> Result<()> {
    if change_sets.is_empty() {
        return Ok(());
    }
    let mut links = read_text_links_for_card(tx, board_id, card_id)?;
    let mut deleted_link_ids = Vec::new();
    let mut field_texts = std::collections::HashMap::<String, String>::new();
    for change_set in change_sets {
        if !matches!(change_set.field.as_str(), "title" | "content") {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if !field_texts.contains_key(&change_set.field) {
            let text = card_field_text(tx, board_id, card_id, &change_set.field)?;
            field_texts.insert(change_set.field.clone(), text);
        }
        let text = field_texts
            .get(&change_set.field)
            .ok_or(rusqlite::Error::InvalidQuery)?;
        let updated_text = apply_validated_text_changes(text, &change_set.changes)?;
        field_texts.insert(change_set.field.clone(), updated_text);

        for link in &mut links {
            let source_matches = link.source_board_id == board_id
                && link.source_card_id == card_id
                && link.source_field == change_set.field;
            let target_matches = link.target_board_id == board_id
                && link.target_card_id == card_id
                && link.target_field == change_set.field;

            if source_matches {
                if let Some((start, end)) =
                    remap_text_range(link.source_start, link.source_end, &change_set.changes)
                {
                    link.source_start = start;
                    link.source_end = end;
                } else {
                    deleted_link_ids.push(link.id.clone());
                    link.id.clear();
                    continue;
                }
            }
            if target_matches {
                if let Some((start, end)) =
                    remap_text_range(link.target_start, link.target_end, &change_set.changes)
                {
                    link.target_start = start;
                    link.target_end = end;
                } else {
                    deleted_link_ids.push(link.id.clone());
                    link.id.clear();
                }
            }
        }
        links.retain(|link| !link.id.is_empty());
    }

    for link_id in deleted_link_ids {
        tx.execute("DELETE FROM mind_text_links WHERE id = ?1", [link_id])?;
    }

    for link in links {
        tx.execute(
            "UPDATE mind_text_links
             SET source_start = ?1, source_end = ?2, target_start = ?3, target_end = ?4, updated_at = ?5
             WHERE id = ?6",
            params![
                link.source_start,
                link.source_end,
                link.target_start,
                link.target_end,
                now,
                link.id
            ],
        )?;
    }
    Ok(())
}

fn ensure_board_version_columns(conn: &Connection) -> Result<()> {
    let mut statement = conn.prepare("PRAGMA table_info(mind_boards)")?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    let columns = columns.collect::<Result<Vec<_>>>()?;
    if !columns.iter().any(|column| column == "active_version_id") {
        conn.execute(
            "ALTER TABLE mind_boards ADD COLUMN active_version_id TEXT",
            [],
        )?;
    }
    if !columns
        .iter()
        .any(|column| column == "branch_origin_version_id")
    {
        conn.execute(
            "ALTER TABLE mind_boards ADD COLUMN branch_origin_version_id TEXT",
            [],
        )?;
    }
    Ok(())
}

fn ensure_library_item_group_board_column(conn: &Connection) -> Result<()> {
    let mut statement = conn.prepare("PRAGMA table_info(mind_boards)")?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    let has_library_group_id = columns
        .collect::<Result<Vec<_>>>()?
        .iter()
        .any(|column| column == "library_item_group_id");
    if !has_library_group_id {
        conn.execute(
            "ALTER TABLE mind_boards ADD COLUMN library_item_group_id TEXT",
            [],
        )?;
    }

    conn.execute_batch("SAVEPOINT migrate_library_item_group_boards")?;
    let migration = (|| {
        let mut statement = conn.prepare(
            "SELECT groups.id, groups.name, groups.version, groups.snapshot_json,
                    groups.created_at, groups.updated_at
             FROM library_item_groups groups
             WHERE NOT EXISTS (
                 SELECT 1 FROM mind_boards boards
                 WHERE boards.library_item_group_id = groups.id
             )
             ORDER BY groups.created_at, groups.id",
        )?;
        let missing_groups = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })?
            .collect::<Result<Vec<_>>>()?;
        for (group_id, name, version, snapshot_json, created_at, updated_at) in missing_groups {
            let (major, minor, patch) = parse_board_version(&version).unwrap_or((0, 1, 0));
            let board_id = format!("library_board_{group_id}");
            let version_id = format!("library_board_version_{}", uuid::Uuid::now_v7());
            conn.execute(
                "INSERT INTO mind_boards
                    (id, title, created_at, updated_at, library_item_group_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![board_id, name, created_at, updated_at, group_id],
            )?;
            conn.execute(
                "INSERT INTO mind_board_versions
                    (id, board_id, version_major, version_minor, version_patch, snapshot_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![version_id, board_id, major, minor, patch, snapshot_json, updated_at],
            )?;
            conn.execute(
                "UPDATE mind_boards SET active_version_id = ?1 WHERE id = ?2",
                params![version_id, board_id],
            )?;
        }
        Ok(())
    })();
    match migration {
        Ok(()) => conn.execute_batch("RELEASE SAVEPOINT migrate_library_item_group_boards"),
        Err(error) => {
            conn.execute_batch(
                "ROLLBACK TO SAVEPOINT migrate_library_item_group_boards;
                 RELEASE SAVEPOINT migrate_library_item_group_boards",
            )?;
            Err(error)
        }
    }
}

fn ensure_board_column(conn: &Connection, table: &str) -> Result<()> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    let has_board_id = columns
        .collect::<Result<Vec<_>>>()?
        .iter()
        .any(|column| column == "board_id");
    if !has_board_id {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN board_id TEXT NOT NULL DEFAULT 'board-1'"),
            [],
        )?;
    }
    Ok(())
}

pub fn ensure_board_version_baselines(conn: &Connection) -> Result<()> {
    let mut statement = conn.prepare(
        "SELECT id FROM mind_boards WHERE active_version_id IS NULL ORDER BY created_at ASC, id ASC",
    )?;
    let board_ids = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>>>()?;
    for board_id in board_ids {
        let payload = snapshot_current_board(conn, &board_id)?;
        let snapshot_json = serde_json::to_string(&payload)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let now = now_millis()?;
        let version_id = format!("board_version_{}", uuid::Uuid::now_v7());
        conn.execute(
            "INSERT INTO mind_board_versions
                (id, board_id, version_major, version_minor, version_patch, snapshot_json, created_at)
             VALUES (?1, ?2, 0, 1, 0, ?3, ?4)",
            params![version_id, board_id, snapshot_json, now],
        )?;
        conn.execute(
            "UPDATE mind_boards SET active_version_id = ?1 WHERE id = ?2",
            params![version_id, board_id],
        )?;
    }
    Ok(())
}

fn snapshot_current_board(conn: &Connection, board_id: &str) -> Result<BoardVersionSnapshot> {
    let latest_snapshot_id = conn
        .query_row(
            "SELECT id FROM mind_snapshots WHERE board_id = ?1 ORDER BY created_at DESC, rowid DESC LIMIT 1",
            [board_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let resolved = latest_snapshot_id
        .map(|snapshot_id| {
            crate::mind::resolver::SnapshotResolver::resolve(conn, board_id, &snapshot_id)
        })
        .transpose()?;

    let nodes = resolved
        .as_ref()
        .map(|snapshot| {
            snapshot
                .placements
                .iter()
                .map(|(id, placement)| {
                    let node = snapshot.node_data.get(id);
                    NodeInput {
                        id: id.clone(),
                        node_type: node
                            .map(|value| value.node_type.clone())
                            .unwrap_or_else(default_node_type),
                        title: node
                            .and_then(|value| value.title.clone())
                            .unwrap_or_else(|| "Untitled".to_string()),
                        content: node
                            .and_then(|value| value.content.clone())
                            .unwrap_or_default(),
                        version: node
                            .and_then(|value| value.version.clone())
                            .unwrap_or_else(|| "v0.1.0".to_string()),
                        data: node.map(|value| value.data.clone()).unwrap_or(Value::Null),
                        x: placement.x,
                        y: placement.y,
                        width: Some(placement.width),
                        height: Some(placement.height),
                        style: placement.style.clone(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let edges = resolved
        .as_ref()
        .map(|snapshot| {
            snapshot
                .links
                .iter()
                .map(|(id, link)| EdgeInput {
                    id: id.clone(),
                    source: link.source_object_id.clone(),
                    target: link.target_object_id.clone(),
                    link_type: Some(link.link_type.clone()),
                })
                .collect()
        })
        .unwrap_or_default();

    let mut statement = conn.prepare(
        "SELECT o.id,
                COALESCE(json_extract(v.content, '$.title'), 'Untitled'),
                COALESCE(json_extract(v.content, '$.content'), ''),
                COALESCE(json_extract(v.content, '$.version'), 'v0.1.0'),
                o.deleted_at
         FROM objects o
         LEFT JOIN object_versions v ON v.id = (
             SELECT id FROM object_versions
             WHERE object_id = o.id
             ORDER BY created_at DESC, rowid DESC
             LIMIT 1
         )
         WHERE o.type = 'card' AND o.board_id = ?1
         ORDER BY o.created_at ASC, o.id ASC",
    )?;
    let cards = statement
        .query_map([board_id], |row| {
            Ok(BoardVersionCard {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                version: row.get(3)?,
                deleted_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    Ok(BoardVersionSnapshot {
        nodes,
        edges,
        cards,
    })
}

fn now_millis() -> Result<i64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .map_err(|_| rusqlite::Error::InvalidQuery)
}

fn version_label(major: i32, minor: i32, patch: i32) -> String {
    format!("{major}.{minor}.{patch}")
}

fn load_board_version_summary(
    conn: &Connection,
    board_id: &str,
    version_id: &str,
    active_version_id: &str,
) -> Result<BoardVersionSummary> {
    conn.query_row(
        "SELECT v.id, v.board_id, v.version_major, v.version_minor, v.version_patch,
                v.parent_version_id, v.forked_from_version_id, v.created_at,
                origin.version_major || '.' || origin.version_minor || '.' || origin.version_patch
         FROM mind_board_versions v
         LEFT JOIN mind_board_versions origin ON origin.id = v.forked_from_version_id
         WHERE v.board_id = ?1 AND v.id = ?2",
        params![board_id, version_id],
        |row| {
            let major = row.get(2)?;
            let minor = row.get(3)?;
            let patch = row.get(4)?;
            Ok(BoardVersionSummary {
                id: row.get(0)?,
                board_id: row.get(1)?,
                major,
                minor,
                patch,
                version: version_label(major, minor, patch),
                parent_version_id: row.get(5)?,
                forked_from_version_id: row.get(6)?,
                created_at: row.get(7)?,
                forked_from_version: row.get(8)?,
                is_active: row.get::<_, String>(0)? == active_version_id,
            })
        },
    )
}

pub fn get_all_boards(conn: &Connection) -> Result<Vec<BoardSummary>> {
    let mut statement = conn.prepare(
        "SELECT b.id, b.title, b.created_at, b.updated_at, b.active_version_id,
                v.version_major, v.version_minor, v.version_patch,
                COALESCE(origin.version_major || '.' || origin.version_minor || '.' || origin.version_patch,
                         branch.version_major || '.' || branch.version_minor || '.' || branch.version_patch)
         FROM mind_boards b
         LEFT JOIN mind_board_versions v ON v.id = b.active_version_id
         LEFT JOIN mind_board_versions origin ON origin.id = v.forked_from_version_id
         LEFT JOIN mind_board_versions branch ON branch.id = b.branch_origin_version_id
         WHERE b.library_item_group_id IS NULL
         ORDER BY b.created_at ASC, b.id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        let active_version_id: Option<String> = row.get(4)?;
        let major: Option<i32> = row.get(5)?;
        let minor: Option<i32> = row.get(6)?;
        let patch: Option<i32> = row.get(7)?;
        let forked_from_version: Option<String> = row.get(8)?;
        let active_version = match (major, minor, patch) {
            (Some(major), Some(minor), Some(patch)) => {
                let label = version_label(major, minor, patch);
                forked_from_version.map_or(label.clone(), |parent| format!("{label} - {parent}"))
            }
            _ => String::new(),
        };
        Ok(BoardSummary {
            id: row.get(0)?,
            title: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            active_version_id: active_version_id.unwrap_or_default(),
            active_version,
        })
    })?;
    rows.collect()
}

pub(crate) fn parse_board_version(version: &str) -> Option<(i32, i32, i32)> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

pub fn create_board(
    conn: &mut Connection,
    title: &str,
    now: i64,
    id: &str,
) -> Result<BoardSummary> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO mind_boards (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![id, title, now],
    )?;
    let version_id = format!("board_version_{}", uuid::Uuid::now_v7());
    let payload = serde_json::to_string(&BoardVersionSnapshot::default())
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    tx.execute(
        "INSERT INTO mind_board_versions
            (id, board_id, version_major, version_minor, version_patch, snapshot_json, created_at)
         VALUES (?1, ?2, 0, 1, 0, ?3, ?4)",
        params![version_id, id, payload, now],
    )?;
    tx.execute(
        "UPDATE mind_boards SET active_version_id = ?1 WHERE id = ?2",
        params![version_id, id],
    )?;
    tx.commit()?;
    get_all_boards(conn)?
        .into_iter()
        .find(|board| board.id == id)
        .ok_or(rusqlite::Error::InvalidQuery)
}

pub fn get_board_versions(conn: &Connection, board_id: &str) -> Result<Vec<BoardVersionSummary>> {
    let active_version_id: String = conn.query_row(
        "SELECT active_version_id FROM mind_boards WHERE id = ?1",
        [board_id],
        |row| row.get(0),
    )?;
    let mut statement = conn.prepare(
        "SELECT id FROM mind_board_versions WHERE board_id = ?1 ORDER BY created_at ASC, rowid ASC",
    )?;
    let version_ids = statement
        .query_map([board_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>>>()?;
    version_ids
        .iter()
        .map(|version_id| {
            load_board_version_summary(conn, board_id, version_id, &active_version_id)
        })
        .collect()
}

pub fn create_board_version(
    conn: &mut Connection,
    board_id: &str,
    increment: &str,
    snapshot: BoardVersionSnapshot,
) -> Result<BoardVersionSummary> {
    let tx = conn.transaction()?;
    let (active_version_id, forked_from_version_id): (String, Option<String>) = tx.query_row(
        "SELECT active_version_id, branch_origin_version_id FROM mind_boards WHERE id = ?1",
        [board_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let (major, minor, patch): (i32, i32, i32) = tx.query_row(
        "SELECT version_major, version_minor, version_patch
         FROM mind_board_versions WHERE id = ?1 AND board_id = ?2",
        params![active_version_id, board_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    let (next_major, next_minor, next_patch) = match increment {
        "major" => (major + 1, 0, 0),
        "minor" => (major, minor + 1, 0),
        "patch" => (major, minor, patch + 1),
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    let version_exists: bool = tx.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM mind_board_versions
            WHERE board_id = ?1 AND version_major = ?2 AND version_minor = ?3 AND version_patch = ?4
              AND COALESCE(forked_from_version_id, '') = COALESCE(?5, '')
         )",
        params![
            board_id,
            next_major,
            next_minor,
            next_patch,
            forked_from_version_id
        ],
        |row| row.get(0),
    )?;
    if version_exists {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let now = now_millis()?;
    let version_id = format!("board_version_{}", uuid::Uuid::now_v7());
    let snapshot_json = serde_json::to_string(&snapshot)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    tx.execute(
        "INSERT INTO mind_board_versions
            (id, board_id, version_major, version_minor, version_patch,
             parent_version_id, forked_from_version_id, snapshot_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            version_id,
            board_id,
            next_major,
            next_minor,
            next_patch,
            active_version_id,
            forked_from_version_id,
            snapshot_json,
            now
        ],
    )?;
    tx.execute(
        "UPDATE mind_boards SET active_version_id = ?1, updated_at = ?2 WHERE id = ?3",
        params![version_id, now, board_id],
    )?;
    tx.commit()?;
    load_board_version_summary(conn, board_id, &version_id, &version_id)
}

pub fn checkout_board_version(
    conn: &mut Connection,
    board_id: &str,
    version_id: &str,
) -> Result<BoardVersionSnapshot> {
    let tx = conn.transaction()?;
    let snapshot_json: String = tx.query_row(
        "SELECT snapshot_json FROM mind_board_versions WHERE board_id = ?1 AND id = ?2",
        params![board_id, version_id],
        |row| row.get(0),
    )?;
    let snapshot: BoardVersionSnapshot = serde_json::from_str(&snapshot_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let latest_version_id: String = tx.query_row(
        "SELECT id FROM mind_board_versions WHERE board_id = ?1 ORDER BY created_at DESC, rowid DESC LIMIT 1",
        [board_id],
        |row| row.get(0),
    )?;
    let selected_fork_point: Option<String> = tx.query_row(
        "SELECT forked_from_version_id FROM mind_board_versions WHERE board_id = ?1 AND id = ?2",
        params![board_id, version_id],
        |row| row.get(0),
    )?;
    let fork_point = if latest_version_id == version_id {
        selected_fork_point
    } else {
        Some(latest_version_id)
    };

    let now = now_millis()?;
    tx.execute(
        "UPDATE objects SET deleted_at = ?1 WHERE board_id = ?2 AND type = 'card'",
        params![now, board_id],
    )?;
    for card in &snapshot.cards {
        let changed = tx.execute(
            "INSERT INTO objects (id, type, board_id, created_at, updated_at, deleted_at)
             VALUES (?1, 'card', ?2, ?3, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET updated_at = excluded.updated_at, deleted_at = excluded.deleted_at
             WHERE objects.board_id = excluded.board_id",
            params![card.id, board_id, now, card.deleted_at],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let content = serde_json::json!({
            "title": card.title,
            "content": card.content,
            "version": card.version,
            "node_type": "voraCard",
            "data": {
                "cardId": card.id,
                "title": card.title,
                "label": card.title,
                "content": card.content,
                "version": card.version
            }
        });
        tx.execute(
            "INSERT INTO object_versions
                (id, object_id, version_major, version_minor, version_patch, reason, summary, content, created_at)
             VALUES (?1, ?2, 0, 1, 0, 'board-version-checkout', NULL, ?3, ?4)",
            params![
                format!("card_version_{}", uuid::Uuid::now_v7()),
                card.id,
                content.to_string(),
                now
            ],
        )?;
    }
    tx.execute(
        "UPDATE mind_boards SET active_version_id = ?1, branch_origin_version_id = ?2, updated_at = ?3
         WHERE id = ?4",
        params![version_id, fork_point, now, board_id],
    )?;
    tx.commit()?;
    Ok(snapshot)
}

pub fn rename_board(conn: &Connection, board_id: &str, title: &str, now: i64) -> Result<()> {
    let changed = conn.execute(
        "UPDATE mind_boards SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![title, now, board_id],
    )?;
    if changed == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

pub fn delete_board(conn: &mut Connection, board_id: &str) -> Result<()> {
    let tx = conn.transaction()?;
    let board_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM mind_boards WHERE id = ?1)",
        params![board_id],
        |row| row.get(0),
    )?;
    if !board_exists {
        return Err(rusqlite::Error::InvalidQuery);
    }
    tx.execute(
        "DELETE FROM mind_text_links WHERE source_board_id = ?1 OR target_board_id = ?1",
        params![board_id],
    )?;
    tx.execute(
        "DELETE FROM snapshot_link_changes WHERE snapshot_id IN (SELECT id FROM mind_snapshots WHERE board_id = ?1)",
        params![board_id],
    )?;
    tx.execute(
        "DELETE FROM snapshot_placement_changes WHERE snapshot_id IN (SELECT id FROM mind_snapshots WHERE board_id = ?1)",
        params![board_id],
    )?;
    tx.execute(
        "DELETE FROM snapshot_object_changes WHERE snapshot_id IN (SELECT id FROM mind_snapshots WHERE board_id = ?1)",
        params![board_id],
    )?;
    tx.execute(
        "DELETE FROM checkpoint_links WHERE checkpoint_id IN (SELECT id FROM mind_snapshots WHERE board_id = ?1)",
        params![board_id],
    )?;
    tx.execute(
        "DELETE FROM checkpoint_placements WHERE checkpoint_id IN (SELECT id FROM mind_snapshots WHERE board_id = ?1)",
        params![board_id],
    )?;
    tx.execute(
        "DELETE FROM checkpoint_objects WHERE checkpoint_id IN (SELECT id FROM mind_snapshots WHERE board_id = ?1)",
        params![board_id],
    )?;
    tx.execute(
        "UPDATE mind_snapshots SET parent_snapshot_id = NULL WHERE board_id = ?1",
        params![board_id],
    )?;
    tx.execute(
        "DELETE FROM object_versions WHERE object_id IN (SELECT id FROM objects WHERE board_id = ?1)",
        params![board_id],
    )?;
    tx.execute("DELETE FROM objects WHERE board_id = ?1", params![board_id])?;
    tx.execute(
        "DELETE FROM mind_snapshots WHERE board_id = ?1",
        params![board_id],
    )?;
    tx.execute(
        "UPDATE mind_board_versions SET parent_version_id = NULL, forked_from_version_id = NULL WHERE board_id = ?1",
        params![board_id],
    )?;
    tx.execute(
        "DELETE FROM mind_board_versions WHERE board_id = ?1",
        params![board_id],
    )?;
    tx.execute("DELETE FROM mind_boards WHERE id = ?1", params![board_id])?;
    tx.commit()
}

pub fn save_semantic_snapshot(
    conn: &mut Connection,
    board_id: String,
    snapshot_name: String,
    parent_snapshot_id: Option<String>,
    reason: Option<String>,
    summary: Option<String>,
    nodes: Vec<NodeInput>,
    edges: Vec<EdgeInput>,
) -> Result<String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| rusqlite::Error::InvalidQuery)?
        .as_millis() as i64;

    let new_snap_id = format!("snap_{}_{}", now, uuid::Uuid::now_v7());

    let tx = conn.transaction()?;
    let board_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM mind_boards WHERE id = ?1)",
        params![board_id],
        |row| row.get(0),
    )?;
    if !board_exists {
        return Err(rusqlite::Error::InvalidQuery);
    }

    let (parent_depth, is_checkpoint, parent_snapshot) = if let Some(ref p_id) = parent_snapshot_id
    {
        let (depth, parent_board_id): (i32, String) = tx.query_row(
            "SELECT depth, board_id FROM mind_snapshots WHERE id = ?1",
            params![p_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if parent_board_id != board_id {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let next_depth = depth + 1;
        let should_checkpoint = if next_depth % 50 == 0 { 1 } else { 0 };
        (
            next_depth,
            should_checkpoint,
            Some(crate::mind::resolver::SnapshotResolver::resolve(
                &tx, &board_id, p_id,
            )?),
        )
    } else {
        (0, 1, None)
    };

    tx.execute(
        "INSERT INTO mind_snapshots (id, name, parent_snapshot_id, board_id, depth, replay_cost, is_checkpoint, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            &new_snap_id,
            snapshot_name,
            parent_snapshot_id,
            board_id,
            parent_depth,
            0,
            is_checkpoint,
            now
        ],
    )?;

    for node in &nodes {
        let affected = tx.execute(
            "INSERT INTO objects (id, type, board_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(id) DO UPDATE SET type = excluded.type, updated_at = excluded.updated_at
             WHERE objects.board_id = excluded.board_id",
            params![&node.id, object_type_for_node(&node.node_type), &board_id, now],
        )?;
        if affected == 0 {
            return Err(rusqlite::Error::InvalidQuery);
        }

        let ver_id = format!("ver_{}_{}_{}", node.id, now, uuid::Uuid::now_v7());
        let content_json = serde_json::json!({
            "title": node.title,
            "content": node.content,
            "version": node.version,
            "node_type": node.node_type,
            "data": node.data,
        })
        .to_string();

        tx.execute(
            "INSERT INTO object_versions (id, object_id, version_major, version_minor, version_patch, reason, summary, content, created_at)
             VALUES (?1, ?2, 0, 1, 0, ?3, ?4, ?5, ?6)",
            params![&ver_id, &node.id, reason, summary, content_json, now],
        )?;

        if is_checkpoint == 1 {
            tx.execute(
                "INSERT INTO checkpoint_objects (checkpoint_id, object_id, object_version_id) VALUES (?1, ?2, ?3)",
                params![&new_snap_id, &node.id, &ver_id],
            )?;

            tx.execute(
                "INSERT INTO checkpoint_placements (checkpoint_id, object_id, x, y, width, height, style)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &new_snap_id,
                    &node.id,
                    node.x,
                    node.y,
                    node.width.unwrap_or(280.0),
                    node.height.unwrap_or(160.0),
                    &node.style
                ],
            )?;
        } else {
            let operation = if parent_snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.objects.contains_key(&node.id))
            {
                "UPDATE_VERSION"
            } else {
                "ADD"
            };
            tx.execute(
                "INSERT INTO snapshot_object_changes (snapshot_id, object_id, operation, object_version_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![&new_snap_id, &node.id, operation, &ver_id],
            )?;

            let placement_operation = if parent_snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.placements.contains_key(&node.id))
            {
                "MOVE"
            } else {
                "ADD"
            };
            tx.execute(
                "INSERT INTO snapshot_placement_changes (snapshot_id, object_id, operation, x, y, width, height, style)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    &new_snap_id,
                    &node.id,
                    placement_operation,
                    node.x,
                    node.y,
                    node.width.unwrap_or(280.0),
                    node.height.unwrap_or(160.0),
                    &node.style
                ],
            )?;
        }
    }

    if is_checkpoint == 0 {
        if let Some(previous) = &parent_snapshot {
            for object_id in previous.objects.keys() {
                if !nodes.iter().any(|node| &node.id == object_id) {
                    tx.execute(
                        "INSERT INTO snapshot_object_changes (snapshot_id, object_id, operation, object_version_id)
                         VALUES (?1, ?2, 'REMOVE', NULL)",
                        params![&new_snap_id, object_id],
                    )?;
                    tx.execute(
                        "INSERT INTO snapshot_placement_changes (snapshot_id, object_id, operation)
                         VALUES (?1, ?2, 'REMOVE')",
                        params![&new_snap_id, object_id],
                    )?;
                }
            }
        }
    }

    for edge in &edges {
        let l_type = edge
            .link_type
            .clone()
            .unwrap_or_else(|| "connects_to".to_string());
        if is_checkpoint == 1 {
            tx.execute(
                "INSERT INTO checkpoint_links (checkpoint_id, link_id, source_object_id, target_object_id, link_type)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![&new_snap_id, &edge.id, &edge.source, &edge.target, &l_type],
            )?;
        } else {
            let previous_link = parent_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.links.get(&edge.id));
            let (operation, unchanged) = match previous_link {
                Some(link)
                    if link.source_object_id == edge.source
                        && link.target_object_id == edge.target
                        && link.link_type == l_type =>
                {
                    ("UPDATE", true)
                }
                Some(_) => ("UPDATE", false),
                None => ("ADD", false),
            };
            if !unchanged {
                tx.execute(
                    "INSERT INTO snapshot_link_changes (snapshot_id, link_id, operation, source_object_id, target_object_id, link_type)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![&new_snap_id, &edge.id, operation, &edge.source, &edge.target, &l_type],
                )?;
            }
        }
    }

    if is_checkpoint == 0 {
        if let Some(previous) = &parent_snapshot {
            for link_id in previous.links.keys() {
                if !edges.iter().any(|edge| &edge.id == link_id) {
                    tx.execute(
                        "INSERT INTO snapshot_link_changes (snapshot_id, link_id, operation)
                         VALUES (?1, ?2, 'REMOVE')",
                        params![&new_snap_id, link_id],
                    )?;
                }
            }
        }
    }

    tx.commit()?;
    Ok(new_snap_id)
}

#[cfg(test)]
mod board_version_tests {
    use super::{
        checkout_board_version, create_board, create_board_version, get_all_boards,
        get_board_versions, init_db, BoardVersionCard, BoardVersionSnapshot, EdgeInput, NodeInput,
    };
    use rusqlite::Connection;
    use serde_json::json;

    fn board_snapshot(title: &str) -> BoardVersionSnapshot {
        BoardVersionSnapshot {
            nodes: vec![NodeInput {
                id: "card-a".to_string(),
                node_type: "voraCard".to_string(),
                title: title.to_string(),
                content: format!("content for {title}"),
                version: "v0.1.0".to_string(),
                data: json!({"cardId": "card-a", "title": title}),
                x: 30.0,
                y: 50.0,
                width: Some(320.0),
                height: Some(180.0),
                style: None,
            }],
            edges: vec![EdgeInput {
                id: "edge-a".to_string(),
                source: "card-a".to_string(),
                target: "card-b".to_string(),
                link_type: Some("connects_to".to_string()),
            }],
            cards: vec![BoardVersionCard {
                id: "card-a".to_string(),
                title: title.to_string(),
                content: format!("content for {title}"),
                version: "v0.1.0".to_string(),
                deleted_at: None,
            }],
        }
    }

    #[test]
    fn fresh_database_starts_without_a_default_board() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        assert!(get_all_boards(&conn).unwrap().is_empty());
    }

    #[cfg(test)]
    mod text_link_tests {
        use super::super::{
            apply_text_link_changes, create_board, create_text_link, get_text_links_for_card,
            init_db, TextChangeRange, TextChangeSet, TextLinkInput,
        };
        use rusqlite::{params, Connection};
        use serde_json::json;

        fn add_card(conn: &Connection, board_id: &str, card_id: &str, title: &str, content: &str) {
            conn.execute(
                "INSERT INTO objects (id, type, board_id, created_at, updated_at)
                     VALUES (?1, 'card', ?2, 1, 1)",
                params![card_id, board_id],
            )
            .unwrap();
            conn.execute(
                    "INSERT INTO object_versions
                     (id, object_id, version_major, version_minor, version_patch, content, created_at)
                     VALUES (?1, ?2, 0, 1, 0, ?3, 1)",
                    params![
                        format!("{card_id}-version"),
                        card_id,
                        json!({"title": title, "content": content}).to_string()
                    ],
                )
                .unwrap();
        }

        fn seed_link_database() -> Connection {
            let mut conn = Connection::open_in_memory().unwrap();
            init_db(&conn).unwrap();
            create_board(&mut conn, "My desk", 1, "board-1").unwrap();
            create_board(&mut conn, "Other board", 1, "board-2").unwrap();
            add_card(&conn, "board-1", "source", "Source", "alpha and omega");
            add_card(&conn, "board-2", "target", "Target", "beta phrase");
            conn
        }

        fn cross_board_link(conn: &mut Connection, id: &str) {
            create_text_link(
                conn,
                TextLinkInput {
                    source_board_id: "board-1".to_string(),
                    source_card_id: "source".to_string(),
                    source_field: "content".to_string(),
                    source_start: 0,
                    source_end: 5,
                    target_board_id: "board-2".to_string(),
                    target_card_id: "target".to_string(),
                    target_field: "content".to_string(),
                    target_start: 0,
                    target_end: 4,
                },
                2,
                id,
            )
            .unwrap();
        }

        #[test]
        fn supports_cross_board_links_and_multiple_destinations_for_one_anchor() {
            let mut conn = seed_link_database();
            cross_board_link(&mut conn, "link-a");
            cross_board_link(&mut conn, "link-b");

            let links = get_text_links_for_card(&conn, "board-1", "source").unwrap();
            assert_eq!(links.len(), 2);
            assert!(links.iter().all(|link| link.target_board_id == "board-2"));
            assert!(links
                .iter()
                .all(|link| link.source_start == 0 && link.source_end == 5));
        }

        #[test]
        fn shifts_anchors_before_text_and_deletes_intersected_links() {
            let mut conn = seed_link_database();
            cross_board_link(&mut conn, "link-a");
            {
                let tx = conn.transaction().unwrap();
                apply_text_link_changes(
                    &tx,
                    "board-1",
                    "source",
                    &[TextChangeSet {
                        field: "content".to_string(),
                        changes: vec![TextChangeRange {
                            from: 0,
                            to: 0,
                            inserted_length: 2,
                            inserted_text: "XX".to_string(),
                        }],
                    }],
                    3,
                )
                .unwrap();
                tx.commit().unwrap();
            }
            let shifted = get_text_links_for_card(&conn, "board-1", "source").unwrap();
            assert_eq!((shifted[0].source_start, shifted[0].source_end), (2, 7));
            conn.execute(
                    "INSERT INTO object_versions
                     (id, object_id, version_major, version_minor, version_patch, content, created_at)
                     VALUES ('source-updated', 'source', 0, 1, 1, ?1, 3)",
                    [json!({"title": "Source", "content": "XXalpha and omega"}).to_string()],
                )
                .unwrap();

            {
                let tx = conn.transaction().unwrap();
                apply_text_link_changes(
                    &tx,
                    "board-1",
                    "source",
                    &[TextChangeSet {
                        field: "content".to_string(),
                        changes: vec![TextChangeRange {
                            from: 4,
                            to: 5,
                            inserted_length: 0,
                            inserted_text: String::new(),
                        }],
                    }],
                    4,
                )
                .unwrap();
                tx.commit().unwrap();
            }
            assert!(get_text_links_for_card(&conn, "board-1", "source")
                .unwrap()
                .is_empty());
        }

        #[test]
        fn rejects_invalid_utf16_edit_boundaries_and_inserted_lengths() {
            let mut conn = Connection::open_in_memory().unwrap();
            init_db(&conn).unwrap();
            create_board(&mut conn, "My desk", 1, "board-1").unwrap();
            add_card(&conn, "board-1", "unicode", "Title", "A😀Z");
            let tx = conn.transaction().unwrap();
            let split_surrogate = apply_text_link_changes(
                &tx,
                "board-1",
                "unicode",
                &[TextChangeSet {
                    field: "content".to_string(),
                    changes: vec![TextChangeRange {
                        from: 2,
                        to: 2,
                        inserted_length: 1,
                        inserted_text: "x".to_string(),
                    }],
                }],
                5,
            );
            assert!(split_surrogate.is_err());
            drop(tx);
        }
    }

    #[test]
    fn board_versions_capture_baseline_branch_origin_and_checkout_state() {
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        create_board(&mut conn, "My desk", 1, "board-1").unwrap();

        let boards = get_all_boards(&conn).unwrap();
        assert_eq!(boards[0].active_version, "0.1.0");
        let baseline_id = boards[0].active_version_id.clone();
        let v0_2 =
            create_board_version(&mut conn, "board-1", "minor", board_snapshot("v0.2.0")).unwrap();
        assert_eq!(v0_2.version, "0.2.0");
        let v0_3 =
            create_board_version(&mut conn, "board-1", "minor", board_snapshot("v0.3.0")).unwrap();
        assert_eq!(v0_3.version, "0.3.0");

        let restored = checkout_board_version(&mut conn, "board-1", &v0_2.id).unwrap();
        assert_eq!(restored.nodes[0].title, "v0.2.0");
        assert_eq!(restored.edges[0].id, "edge-a");
        assert_eq!(restored.cards[0].content, "content for v0.2.0");
        assert_eq!(
            get_all_boards(&conn).unwrap()[0].active_version,
            "0.2.0 - 0.3.0"
        );

        let branch = create_board_version(&mut conn, "board-1", "patch", restored).unwrap();
        assert_eq!(branch.version, "0.2.1");
        assert_eq!(branch.parent_version_id.as_deref(), Some(v0_2.id.as_str()));
        assert_eq!(branch.forked_from_version.as_deref(), Some("0.3.0"));
        assert_ne!(
            branch.parent_version_id.as_deref(),
            Some(baseline_id.as_str())
        );

        let history = get_board_versions(&conn, "board-1").unwrap();
        assert_eq!(history.len(), 4);
        assert!(history.last().is_some_and(|version| version.is_active));
        assert!(checkout_board_version(&mut conn, "board-2", &branch.id).is_err());
    }

    #[test]
    fn existing_board_baseline_captures_its_current_canvas() {
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        create_board(&mut conn, "My desk", 1, "board-1").unwrap();
        conn.execute("DELETE FROM mind_board_versions", []).unwrap();
        conn.execute(
            "UPDATE mind_boards SET active_version_id = NULL WHERE id = 'board-1'",
            [],
        )
        .unwrap();
        super::save_semantic_snapshot(
            &mut conn,
            "board-1".to_string(),
            "Existing canvas".to_string(),
            None,
            None,
            None,
            vec![NodeInput {
                id: "existing-card".to_string(),
                node_type: "voraCard".to_string(),
                title: "Existing card".to_string(),
                content: "Existing content".to_string(),
                version: "v1.0.0".to_string(),
                data: json!({"cardId": "existing-card"}),
                x: 25.0,
                y: 40.0,
                width: Some(300.0),
                height: Some(170.0),
                style: None,
            }],
            Vec::new(),
        )
        .unwrap();

        super::ensure_board_version_baselines(&conn).unwrap();
        let board = get_all_boards(&conn).unwrap().remove(0);
        assert_eq!(board.active_version, "0.1.0");
        let baseline = get_board_versions(&conn, "board-1").unwrap().remove(0);
        let snapshot = checkout_board_version(&mut conn, "board-1", &baseline.id).unwrap();
        assert_eq!(snapshot.nodes[0].title, "Existing card");
        assert_eq!(snapshot.nodes[0].x, 25.0);
        assert_eq!(snapshot.cards[0].content, "Existing content");
    }
}
