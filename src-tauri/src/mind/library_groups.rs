use super::resolver::SnapshotResolver;
use rusqlite::{params, Connection, OptionalExtension, Result, Transaction};
use serde::{Deserialize, Serialize};

use super::canvas_db::{
    parse_board_version, BoardVersionCard, BoardVersionSnapshot, EdgeInput, NodeInput,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryItemGroupSnapshot {
    pub nodes: Vec<NodeInput>,
    pub edges: Vec<EdgeInput>,
    pub cards: Vec<BoardVersionCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryItemGroup {
    pub id: String,
    pub board_id: String,
    pub name: String,
    pub version: String,
    pub snapshot: LibraryItemGroupSnapshot,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryItemGroupVersion {
    pub id: String,
    pub group_id: String,
    pub version: String,
    pub snapshot: LibraryItemGroupSnapshot,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDefinitionSelection {
    pub group_id: String,
    pub version_id: String,
    pub name: String,
    pub version_string: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryItemGroupUsage {
    pub board_id: String,
    pub board_title: String,
    pub node_id: String,
    pub segment_id: String,
    pub segment_name: String,
    pub start_x: f64,
    pub start_y: f64,
    pub end_x: f64,
    pub end_y: f64,
}

pub fn list_item_groups(conn: &Connection) -> Result<Vec<LibraryItemGroup>> {
    let mut statement = conn.prepare(
        "SELECT groups.id, groups.name, groups.version, groups.snapshot_json,
                groups.created_at, groups.updated_at, groups.deleted_at, boards.id
         FROM library_item_groups groups
         JOIN mind_boards boards ON boards.library_item_group_id = groups.id
         ORDER BY groups.created_at DESC, groups.id DESC",
    )?;
    let rows = statement.query_map([], read_group)?;
    rows.collect()
}

pub fn list_item_group_usages(
    conn: &Connection,
    group_id: &str,
) -> Result<Vec<LibraryItemGroupUsage>> {
    let mut statement = conn.prepare(
        "SELECT boards.id, boards.title, snapshots.id
         FROM mind_boards boards
         JOIN mind_snapshots snapshots ON snapshots.id = (
             SELECT id FROM mind_snapshots
             WHERE board_id = boards.id
             ORDER BY created_at DESC, rowid DESC
             LIMIT 1
         )
         WHERE boards.library_item_group_id IS NULL
         ORDER BY boards.title COLLATE NOCASE, boards.id",
    )?;
    let boards = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>>>()?;

    let mut usages = Vec::new();
    for (board_id, board_title, snapshot_id) in boards {
        let snapshot = SnapshotResolver::resolve(conn, &board_id, &snapshot_id)?;
        for (node_id, node_data) in snapshot.node_data {
            let Some(segments) = node_data
                .data
                .get("segments")
                .and_then(serde_json::Value::as_array)
            else {
                continue;
            };
            let Some(points) = node_data
                .data
                .get("points")
                .and_then(serde_json::Value::as_array)
            else {
                continue;
            };
            if let Some(placement) = snapshot.placements.get(&node_id) {
                for (segment_index, segment) in
                    segments.iter().enumerate().filter(|(_, segment)| {
                        segment
                            .get("libraryItemGroupId")
                            .and_then(serde_json::Value::as_str)
                            == Some(group_id)
                    })
                {
                    let (Some(segment_id), Some(segment_name)) = (
                        segment.get("id").and_then(serde_json::Value::as_str),
                        segment.get("name").and_then(serde_json::Value::as_str),
                    ) else {
                        continue;
                    };
                    let (Some(start), Some(end)) =
                        (points.get(segment_index), points.get(segment_index + 1))
                    else {
                        continue;
                    };
                    let normalized_coordinate = |point: &serde_json::Value, axis: &str| {
                        point.get(axis).and_then(serde_json::Value::as_f64)
                    };
                    let (Some(start_x), Some(start_y), Some(end_x), Some(end_y)) = (
                        normalized_coordinate(start, "x"),
                        normalized_coordinate(start, "y"),
                        normalized_coordinate(end, "x"),
                        normalized_coordinate(end, "y"),
                    ) else {
                        continue;
                    };
                    usages.push(LibraryItemGroupUsage {
                        board_id: board_id.clone(),
                        board_title: board_title.clone(),
                        node_id: node_id.clone(),
                        segment_id: segment_id.to_string(),
                        segment_name: segment_name.to_string(),
                        start_x: placement.x + start_x * placement.width,
                        start_y: placement.y + start_y * placement.height,
                        end_x: placement.x + end_x * placement.width,
                        end_y: placement.y + end_y * placement.height,
                    });
                }
            }
        }
    }
    Ok(usages)
}

pub fn list_item_group_versions(
    conn: &Connection,
    group_id: &str,
) -> Result<Vec<LibraryItemGroupVersion>> {
    let mut statement = conn.prepare(
        "SELECT id, group_id, version, snapshot_json, created_at
         FROM library_item_group_versions
         WHERE group_id = ?1
         ORDER BY created_at DESC, rowid DESC",
    )?;
    let rows = statement.query_map([group_id], read_group_version)?;
    rows.collect()
}

pub fn list_definitions_for_selection(
    conn: &Connection,
) -> Result<Vec<LibraryDefinitionSelection>> {
    let mut statement = conn.prepare(
        "SELECT groups.id, versions.id, groups.name, versions.version
         FROM library_item_groups groups
         JOIN library_item_group_versions versions ON versions.group_id = groups.id
         WHERE groups.deleted_at IS NULL
         ORDER BY groups.name COLLATE NOCASE, versions.created_at DESC, versions.rowid DESC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(LibraryDefinitionSelection {
            group_id: row.get(0)?,
            version_id: row.get(1)?,
            name: row.get(2)?,
            version_string: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn create_item_group(
    conn: &mut Connection,
    name: &str,
    version: &str,
    snapshot: LibraryItemGroupSnapshot,
) -> Result<LibraryItemGroup> {
    let now = now_millis()?;
    let transaction = conn.transaction()?;
    let group = create_item_group_in_transaction(&transaction, name, version, snapshot, now)?;
    transaction.commit()?;
    Ok(group)
}

pub(crate) fn create_item_group_in_transaction(
    transaction: &Transaction<'_>,
    name: &str,
    version: &str,
    snapshot: LibraryItemGroupSnapshot,
    now: i64,
) -> Result<LibraryItemGroup> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(rusqlite::Error::InvalidParameterName(
            "Item group name must contain between 1 and 80 characters".to_string(),
        ));
    }
    if !is_semantic_version(version) {
        return Err(rusqlite::Error::InvalidParameterName(
            "Item group version must use the format 0.1.0".to_string(),
        ));
    }
    if snapshot.nodes.is_empty() {
        return Err(rusqlite::Error::InvalidParameterName(
            "Select at least one canvas item to create an item group".to_string(),
        ));
    }

    let id = format!("item_group_{}", uuid::Uuid::now_v7());
    let snapshot_json = serde_json::to_string(&snapshot)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    transaction.execute(
        "INSERT INTO library_item_groups
            (id, name, version, snapshot_json, created_at, updated_at, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, NULL)",
        params![id, name, version, snapshot_json, now],
    )?;
    transaction.execute(
        "INSERT INTO library_item_group_versions (id, group_id, version, snapshot_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            format!("item_group_version_{}", uuid::Uuid::now_v7()),
            id,
            version,
            snapshot_json,
            now
        ],
    )?;
    let board_id = format!("library_board_{id}");
    let board_version_id = format!("library_board_version_{}", uuid::Uuid::now_v7());
    let (major, minor, patch) =
        parse_board_version(version).ok_or(rusqlite::Error::InvalidQuery)?;
    let board_snapshot = BoardVersionSnapshot {
        nodes: snapshot.nodes.clone(),
        edges: snapshot.edges.clone(),
        cards: snapshot.cards.clone(),
    };
    let board_snapshot_json = serde_json::to_string(&board_snapshot)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    transaction.execute(
        "INSERT INTO mind_boards
            (id, title, created_at, updated_at, active_version_id, library_item_group_id)
         VALUES (?1, ?2, ?3, ?3, ?4, ?5)",
        params![board_id, name, now, board_version_id, id],
    )?;
    transaction.execute(
        "INSERT INTO mind_board_versions
            (id, board_id, version_major, version_minor, version_patch, snapshot_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            board_version_id,
            board_id,
            major,
            minor,
            patch,
            board_snapshot_json,
            now
        ],
    )?;
    let group = get_item_group(&transaction, &id)?.ok_or(rusqlite::Error::InvalidQuery)?;
    Ok(group)
}

pub fn commit_item_group_version(
    conn: &mut Connection,
    group_id: &str,
    version: &str,
    snapshot: LibraryItemGroupSnapshot,
) -> Result<LibraryItemGroup> {
    let now = now_millis()?;
    let transaction = conn.transaction()?;
    let group =
        commit_item_group_version_in_transaction(&transaction, group_id, version, snapshot, now)?;
    transaction.commit()?;
    Ok(group)
}

pub(crate) fn commit_item_group_version_in_transaction(
    transaction: &Transaction<'_>,
    group_id: &str,
    version: &str,
    snapshot: LibraryItemGroupSnapshot,
    now: i64,
) -> Result<LibraryItemGroup> {
    if !is_semantic_version(version) {
        return Err(rusqlite::Error::InvalidParameterName(
            "Item group version must use the format 0.1.0".to_string(),
        ));
    }
    if snapshot.nodes.is_empty() {
        return Err(rusqlite::Error::InvalidParameterName(
            "An item group version must contain at least one canvas item".to_string(),
        ));
    }
    let snapshot_json = serde_json::to_string(&snapshot)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let (board_id, active_board_version_id, name): (String, String, String) = transaction
        .query_row(
            "SELECT boards.id, boards.active_version_id, groups.name
             FROM mind_boards boards
             JOIN library_item_groups groups ON groups.id = boards.library_item_group_id
             WHERE groups.id = ?1",
            [group_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    let changed = transaction.execute(
        "UPDATE library_item_groups SET version = ?1, snapshot_json = ?2, updated_at = ?3
         WHERE id = ?4 AND deleted_at IS NULL",
        params![version, snapshot_json, now, group_id],
    )?;
    if changed == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    transaction.execute(
        "INSERT INTO library_item_group_versions (id, group_id, version, snapshot_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            format!("item_group_version_{}", uuid::Uuid::now_v7()),
            group_id,
            version,
            snapshot_json,
            now
        ],
    )?;
    let (major, minor, patch) =
        parse_board_version(version).ok_or(rusqlite::Error::InvalidQuery)?;
    let duplicate_board_version: bool = transaction.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM mind_board_versions
            WHERE board_id = ?1 AND version_major = ?2 AND version_minor = ?3
              AND version_patch = ?4
        )",
        params![board_id, major, minor, patch],
        |row| row.get(0),
    )?;
    if duplicate_board_version {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let board_version_id = format!("library_board_version_{}", uuid::Uuid::now_v7());
    let board_snapshot = BoardVersionSnapshot {
        nodes: snapshot.nodes.clone(),
        edges: snapshot.edges.clone(),
        cards: snapshot.cards.clone(),
    };
    let board_snapshot_json = serde_json::to_string(&board_snapshot)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    transaction.execute(
        "INSERT INTO mind_board_versions
            (id, board_id, version_major, version_minor, version_patch,
             parent_version_id, snapshot_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            board_version_id,
            board_id,
            major,
            minor,
            patch,
            active_board_version_id,
            board_snapshot_json,
            now
        ],
    )?;
    transaction.execute(
        "UPDATE mind_boards
         SET title = ?1, active_version_id = ?2, updated_at = ?3
         WHERE id = ?4",
        params![name, board_version_id, now, board_id],
    )?;
    let group = get_item_group(&transaction, group_id)?.ok_or(rusqlite::Error::InvalidQuery)?;
    Ok(group)
}

pub fn move_item_group_to_trash(conn: &Connection, id: &str) -> Result<()> {
    let changed = conn.execute(
        "UPDATE library_item_groups SET deleted_at = ?1, updated_at = ?1
         WHERE id = ?2 AND deleted_at IS NULL",
        params![now_millis()?, id],
    )?;
    if changed == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

pub fn restore_item_group(conn: &Connection, id: &str) -> Result<()> {
    let changed = conn.execute(
        "UPDATE library_item_groups SET deleted_at = NULL, updated_at = ?1
         WHERE id = ?2 AND deleted_at IS NOT NULL",
        params![now_millis()?, id],
    )?;
    if changed == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

fn get_item_group(conn: &Connection, id: &str) -> Result<Option<LibraryItemGroup>> {
    conn.query_row(
        "SELECT groups.id, groups.name, groups.version, groups.snapshot_json,
                groups.created_at, groups.updated_at, groups.deleted_at, boards.id
         FROM library_item_groups groups
         JOIN mind_boards boards ON boards.library_item_group_id = groups.id
         WHERE groups.id = ?1",
        [id],
        read_group,
    )
    .optional()
}

fn read_group(row: &rusqlite::Row<'_>) -> Result<LibraryItemGroup> {
    let snapshot_json: String = row.get(3)?;
    let snapshot = serde_json::from_str(&snapshot_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(LibraryItemGroup {
        id: row.get(0)?,
        name: row.get(1)?,
        version: row.get(2)?,
        snapshot,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        deleted_at: row.get(6)?,
        board_id: row.get(7)?,
    })
}

fn read_group_version(row: &rusqlite::Row<'_>) -> Result<LibraryItemGroupVersion> {
    let snapshot_json: String = row.get(3)?;
    let snapshot = serde_json::from_str(&snapshot_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(LibraryItemGroupVersion {
        id: row.get(0)?,
        group_id: row.get(1)?,
        version: row.get(2)?,
        snapshot,
        created_at: row.get(4)?,
    })
}

fn is_semantic_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == &"0" || !part.starts_with('0'))
                && part.parse::<u32>().is_ok()
        })
}

fn now_millis() -> Result<i64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .map_err(|_| rusqlite::Error::InvalidQuery)
}

#[cfg(test)]
mod tests {
    use super::{
        commit_item_group_version, create_item_group, list_definitions_for_selection,
        list_item_group_usages, list_item_group_versions, list_item_groups,
        move_item_group_to_trash, restore_item_group,
    };
    use crate::mind::canvas_db::{
        create_board, get_all_boards, init_db, save_semantic_snapshot, BoardVersionCard,
        BoardVersionSnapshot, EdgeInput, NodeInput,
    };
    use rusqlite::Connection;
    use serde_json::json;

    fn sample_snapshot() -> super::LibraryItemGroupSnapshot {
        super::LibraryItemGroupSnapshot {
            nodes: vec![NodeInput {
                id: "node-a".to_string(),
                node_type: "voraText".to_string(),
                title: String::new(),
                content: String::new(),
                version: "v0.1.0".to_string(),
                data: json!({"text": "example"}),
                x: 0.0,
                y: 0.0,
                width: Some(240.0),
                height: Some(120.0),
                style: None,
            }],
            edges: vec![EdgeInput {
                id: "edge-a".to_string(),
                source: "node-a".to_string(),
                target: "node-b".to_string(),
                link_type: Some("connects_to".to_string()),
            }],
            cards: vec![BoardVersionCard {
                id: "card-a".to_string(),
                title: "Card".to_string(),
                content: "Saved content".to_string(),
                version: "v0.1.0".to_string(),
                deleted_at: None,
            }],
        }
    }

    #[test]
    fn groups_are_global_persisted_and_can_be_trashed_and_restored() {
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let created =
            create_item_group(&mut conn, "Shared group", "0.1.0", sample_snapshot()).unwrap();

        let groups = list_item_groups(&conn).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].board_id, format!("library_board_{}", created.id));
        assert_eq!(groups[0].snapshot.cards[0].content, "Saved content");
        assert_eq!(groups[0].snapshot.edges[0].source, "node-a");
        assert_eq!(
            list_item_group_versions(&conn, &created.id).unwrap().len(),
            1
        );
        let owned_board_id = format!("library_board_{}", created.id);
        let (owned_group_id, board_snapshot_json): (String, String) = conn
            .query_row(
                "SELECT boards.library_item_group_id, versions.snapshot_json
                 FROM mind_boards boards
                 JOIN mind_board_versions versions ON versions.id = boards.active_version_id
                 WHERE boards.id = ?1",
                [&owned_board_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(owned_group_id, created.id);
        assert!(get_all_boards(&conn).unwrap().is_empty());
        let board_snapshot: BoardVersionSnapshot =
            serde_json::from_str(&board_snapshot_json).unwrap();
        assert_eq!(board_snapshot.nodes[0].id, created.snapshot.nodes[0].id);

        move_item_group_to_trash(&conn, &created.id).unwrap();
        assert!(list_item_groups(&conn).unwrap()[0].deleted_at.is_some());
        restore_item_group(&conn, &created.id).unwrap();
        assert_eq!(list_item_groups(&conn).unwrap()[0].deleted_at, None);
    }

    #[test]
    fn definition_selection_lists_each_available_version_and_skips_trashed_groups() {
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let created = create_item_group(&mut conn, "Flag", "0.1.0", sample_snapshot()).unwrap();
        let versions = list_item_group_versions(&conn, &created.id).unwrap();

        let definitions = list_definitions_for_selection(&conn).unwrap();
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].group_id, created.id);
        assert_eq!(definitions[0].version_id, versions[0].id);
        assert_eq!(definitions[0].name, "Flag");
        assert_eq!(definitions[0].version_string, "0.1.0");

        move_item_group_to_trash(&conn, &created.id).unwrap();
        assert!(list_definitions_for_selection(&conn).unwrap().is_empty());
    }

    #[test]
    fn path_segment_usages_report_only_current_ordinary_board_locations() {
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let group =
            create_item_group(&mut conn, "Path section", "0.1.0", sample_snapshot()).unwrap();
        let board = create_board(&mut conn, "Active Board", 1, "board-active").unwrap();
        let referenced_group_id = group.id.clone();
        let path_node = NodeInput {
            id: "path-node".to_string(),
            node_type: "voraShape".to_string(),
            title: "Path".to_string(),
            content: String::new(),
            version: "v0.1.0".to_string(),
            data: json!({
                "shape": "path",
                "points": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}],
                "segments": [{
                    "id": "section-1",
                    "name": "Entry",
                    "libraryItemGroupId": referenced_group_id
                }]
            }),
            x: 125.0,
            y: 240.0,
            width: Some(100.0),
            height: Some(60.0),
            style: None,
        };
        let first_snapshot_id = save_semantic_snapshot(
            &mut conn,
            board.id.clone(),
            "Path use".to_string(),
            None,
            None,
            None,
            vec![path_node],
            Vec::new(),
        )
        .unwrap();

        let usages = list_item_group_usages(&conn, &group.id).unwrap();
        assert_eq!(usages.len(), 1);
        assert_eq!(usages[0].board_title, "Active Board");
        assert_eq!(usages[0].segment_name, "Entry");
        assert_eq!(usages[0].start_x, 125.0);
        assert_eq!(usages[0].start_y, 240.0);
        assert_eq!(usages[0].end_x, 225.0);
        assert_eq!(usages[0].end_y, 300.0);

        save_semantic_snapshot(
            &mut conn,
            board.id,
            "Path removed".to_string(),
            Some(first_snapshot_id),
            None,
            None,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        assert!(list_item_group_usages(&conn, &group.id).unwrap().is_empty());
    }

    #[test]
    fn group_versions_are_immutable_and_version_labels_must_be_unique() {
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let created =
            create_item_group(&mut conn, "Versioned group", "0.1.0", sample_snapshot()).unwrap();
        let mut next_snapshot = sample_snapshot();
        next_snapshot.nodes[0].data = json!({"text": "updated"});
        let updated =
            commit_item_group_version(&mut conn, &created.id, "0.2.0", next_snapshot).unwrap();

        assert_eq!(updated.version, "0.2.0");
        assert_eq!(updated.snapshot.nodes[0].data["text"], "updated");
        let versions = list_item_group_versions(&conn, &created.id).unwrap();
        assert_eq!(versions.len(), 2);
        assert!(versions.iter().any(|version| {
            version.version == "0.1.0" && version.snapshot.nodes[0].data["text"] == "example"
        }));
        let owned_board_id = format!("library_board_{}", created.id);
        let board_versions: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mind_board_versions WHERE board_id = ?1",
                [&owned_board_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(board_versions, 2);
        assert!(
            commit_item_group_version(&mut conn, &created.id, "0.1.0", sample_snapshot()).is_err()
        );
    }

    #[test]
    fn existing_groups_are_backfilled_to_hidden_owned_boards() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        let snapshot_json = serde_json::to_string(&sample_snapshot()).unwrap();
        conn.execute(
            "INSERT INTO library_item_groups
                (id, name, version, snapshot_json, created_at, updated_at)
             VALUES ('legacy-group', 'Legacy group', '1.2.3', ?1, 10, 20)",
            [&snapshot_json],
        )
        .unwrap();

        init_db(&conn).unwrap();

        let (board_id, active_version_id): (String, String) = conn
            .query_row(
                "SELECT id, active_version_id FROM mind_boards WHERE library_item_group_id = 'legacy-group'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(board_id, "library_board_legacy-group");
        assert!(!active_version_id.is_empty());
        assert!(get_all_boards(&conn).unwrap().is_empty());
    }

    #[test]
    fn group_creation_rejects_invalid_version_and_empty_selection() {
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        assert!(create_item_group(&mut conn, "Group", "0.1", sample_snapshot()).is_err());
        let empty = super::LibraryItemGroupSnapshot {
            nodes: Vec::new(),
            edges: Vec::new(),
            cards: Vec::new(),
        };
        assert!(create_item_group(&mut conn, "Empty", "0.1.0", empty).is_err());
    }
}
