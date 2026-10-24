use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    pub z_index: i32,
    pub style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub source_object_id: String,
    pub target_object_id: String,
    pub link_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotNodeData {
    pub node_type: String,
    pub data: serde_json::Value,
    pub title: Option<String>,
    pub content: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedSnapshot {
    pub snapshot_id: String,
    pub objects: HashMap<String, String>, // object_id -> object_version_id
    pub node_data: HashMap<String, SnapshotNodeData>,
    pub placements: HashMap<String, Placement>, // object_id -> Placement
    pub links: HashMap<String, Link>,           // link_id -> Link
}

pub struct SnapshotResolver;

impl SnapshotResolver {
    pub fn resolve(
        conn: &Connection,
        board_id: &str,
        target_snapshot_id: &str,
    ) -> Result<ResolvedSnapshot> {
        // ۱. پیدا کردن مسیر از اسنپ‌شات به سمت عقب تا رسیدن به نزدیک‌ترین چک‌پوینت
        let mut chain = Vec::new();
        let mut curr = target_snapshot_id.to_string();

        let nearest_checkpoint_id: String;

        loop {
            let mut stmt = conn.prepare(
                "SELECT parent_snapshot_id, is_checkpoint, board_id FROM mind_snapshots WHERE id = ?1",
            )?;
            let row = stmt.query_row(params![curr], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, i32>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?;

            if row.2 != board_id {
                return Err(rusqlite::Error::InvalidQuery);
            }
            if row.1 == 1 || row.0.is_none() {
                nearest_checkpoint_id = curr;
                break;
            }

            chain.push(curr.clone());
            curr = row.0.ok_or(rusqlite::Error::InvalidQuery)?;
        }

        // زنجیره دلتاها باید از قدیمی به جدید اعمال شوند
        chain.reverse();

        // ۲. لود داده‌های ماتریالایز شده چک‌پوینت
        let mut objects: HashMap<String, String> = HashMap::new();
        let mut placements: HashMap<String, Placement> = HashMap::new();
        let mut links: HashMap<String, Link> = HashMap::new();

        {
            let mut stmt = conn.prepare(
                "SELECT object_id, object_version_id FROM checkpoint_objects WHERE checkpoint_id = ?1"
            )?;
            let rows = stmt.query_map(params![nearest_checkpoint_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?;
            for r in rows {
                let (k, v) = r?;
                objects.insert(k, v);
            }
        }

        {
            let mut stmt = conn.prepare(
                "SELECT object_id, x, y, width, height, rotation, z_index, style FROM checkpoint_placements WHERE checkpoint_id = ?1"
            )?;
            let rows = stmt.query_map(params![nearest_checkpoint_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    Placement {
                        x: r.get(1)?,
                        y: r.get(2)?,
                        width: r.get(3)?,
                        height: r.get(4)?,
                        rotation: r.get(5)?,
                        z_index: r.get(6)?,
                        style: r.get(7)?,
                    },
                ))
            })?;
            for r in rows {
                let (k, v) = r?;
                placements.insert(k, v);
            }
        }

        {
            let mut stmt = conn.prepare(
                "SELECT link_id, source_object_id, target_object_id, link_type FROM checkpoint_links WHERE checkpoint_id = ?1"
            )?;
            let rows = stmt.query_map(params![nearest_checkpoint_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    Link {
                        source_object_id: r.get(1)?,
                        target_object_id: r.get(2)?,
                        link_type: r.get(3)?,
                    },
                ))
            })?;
            for r in rows {
                let (k, v) = r?;
                links.insert(k, v);
            }
        }

        // ۳. اعمال دلتاها (Delta Coalescing & State Resolution)
        for snap_id in chain {
            // اعمال تغییرات آبجکت‌ها
            let mut stmt_obj = conn.prepare(
                "SELECT object_id, operation, object_version_id FROM snapshot_object_changes WHERE snapshot_id = ?1"
            )?;
            let obj_rows = stmt_obj.query_map(params![snap_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })?;

            for r in obj_rows {
                let (obj_id, op, ver_id) = r?;
                match op.as_str() {
                    "ADD" | "UPDATE_VERSION" => {
                        if let Some(v) = ver_id {
                            objects.insert(obj_id, v);
                        }
                    }
                    "REMOVE" => {
                        objects.remove(&obj_id);
                        placements.remove(&obj_id);
                    }
                    _ => {}
                }
            }

            // اعمال تغییرات چیدمان (Placement Deltas)
            let mut stmt_plc = conn.prepare(
                "SELECT object_id, operation, x, y, width, height, rotation, z_index, style FROM snapshot_placement_changes WHERE snapshot_id = ?1"
            )?;
            let plc_rows = stmt_plc.query_map(params![snap_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<f64>>(2)?,
                    r.get::<_, Option<f64>>(3)?,
                    r.get::<_, Option<f64>>(4)?,
                    r.get::<_, Option<f64>>(5)?,
                    r.get::<_, Option<f64>>(6)?,
                    r.get::<_, Option<i32>>(7)?,
                    r.get::<_, Option<String>>(8)?,
                ))
            })?;

            for r in plc_rows {
                let (obj_id, op, x, y, w, h, rot, z, style) = r?;
                if op == "REMOVE" {
                    placements.remove(&obj_id);
                } else if let Some(current) = placements.get_mut(&obj_id) {
                    if let Some(val) = x {
                        current.x = val;
                    }
                    if let Some(val) = y {
                        current.y = val;
                    }
                    if let Some(val) = w {
                        current.width = val;
                    }
                    if let Some(val) = h {
                        current.height = val;
                    }
                    if let Some(val) = rot {
                        current.rotation = val;
                    }
                    if let Some(val) = z {
                        current.z_index = val;
                    }
                    if style.is_some() {
                        current.style = style;
                    }
                } else if op == "ADD" {
                    placements.insert(
                        obj_id,
                        Placement {
                            x: x.unwrap_or(0.0),
                            y: y.unwrap_or(0.0),
                            width: w.unwrap_or(200.0),
                            height: h.unwrap_or(150.0),
                            rotation: rot.unwrap_or(0.0),
                            z_index: z.unwrap_or(0),
                            style,
                        },
                    );
                }
            }

            let mut stmt_links = conn.prepare(
                "SELECT link_id, operation, source_object_id, target_object_id, link_type
                 FROM snapshot_link_changes WHERE snapshot_id = ?1",
            )?;
            let link_rows = stmt_links.query_map(params![snap_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            })?;
            for row in link_rows {
                let (link_id, operation, source, target, link_type) = row?;
                if operation == "REMOVE" {
                    links.remove(&link_id);
                } else if let (Some(source_object_id), Some(target_object_id), Some(link_type)) =
                    (source, target, link_type)
                {
                    links.insert(
                        link_id,
                        Link {
                            source_object_id,
                            target_object_id,
                            link_type,
                        },
                    );
                }
            }
        }

        let mut node_data = HashMap::new();
        for (object_id, version_id) in &objects {
            let content_json: String = conn.query_row(
                "SELECT content FROM object_versions WHERE id = ?1",
                params![version_id],
                |row| row.get(0),
            )?;
            let content = serde_json::from_str::<serde_json::Value>(&content_json)
                .unwrap_or(serde_json::Value::Null);
            let node_type = content
                .get("node_type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("voraCard")
                .to_string();
            node_data.insert(
                object_id.clone(),
                SnapshotNodeData {
                    node_type,
                    data: content
                        .get("data")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                    title: content
                        .get("title")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string),
                    content: content
                        .get("content")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string),
                    version: content
                        .get("version")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string),
                },
            );
        }

        Ok(ResolvedSnapshot {
            snapshot_id: target_snapshot_id.to_string(),
            objects,
            node_data,
            placements,
            links,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SnapshotResolver;
    use crate::mind::canvas_db::{
        create_board, delete_board, get_all_boards, init_db, save_semantic_snapshot, EdgeInput,
        NodeInput,
    };
    use rusqlite::Connection;
    use serde_json::json;

    #[test]
    fn shape_node_type_and_data_round_trip_through_snapshot() {
        let mut connection = Connection::open_in_memory().unwrap();
        init_db(&connection).unwrap();
        create_board(&mut connection, "My desk", 1, "board-1").unwrap();
        let shape_data = json!({
            "shape": "path",
            "label": "Path",
            "points": [{ "x": 0.0, "y": 0.0 }, { "x": 1.0, "y": 1.0 }],
            "startArrow": false,
            "endArrow": true
        });

        let snapshot_id = save_semantic_snapshot(
            &mut connection,
            "board-1".to_string(),
            "shape snapshot".to_string(),
            None,
            None,
            None,
            vec![NodeInput {
                id: "path-1".to_string(),
                node_type: "voraShape".to_string(),
                title: "Path".to_string(),
                content: String::new(),
                version: "v1".to_string(),
                data: shape_data.clone(),
                x: 42.0,
                y: 84.0,
                width: Some(240.0),
                height: Some(160.0),
                style: None,
            }],
            Vec::<EdgeInput>::new(),
        )
        .unwrap();

        let resolved = SnapshotResolver::resolve(&connection, "board-1", &snapshot_id).unwrap();
        let restored = resolved.node_data.get("path-1").unwrap();
        assert_eq!(restored.node_type, "voraShape");
        assert_eq!(restored.data, shape_data);
        assert_eq!(resolved.placements.get("path-1").unwrap().x, 42.0);
        assert_eq!(resolved.placements.get("path-1").unwrap().width, 240.0);
        let object_type: String = connection
            .query_row("SELECT type FROM objects WHERE id = 'path-1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(object_type, "shape");
    }

    #[test]
    fn board_snapshots_isolate_nodes_and_restore_incremental_adds_and_removals() {
        let mut connection = Connection::open_in_memory().unwrap();
        init_db(&connection).unwrap();
        create_board(&mut connection, "My desk", 1, "board-1").unwrap();
        crate::mind::canvas_db::create_board(&mut connection, "Second board", 1, "board-2")
            .unwrap();

        let first = save_semantic_snapshot(
            &mut connection,
            "board-1".to_string(),
            "Initial".to_string(),
            None,
            None,
            None,
            vec![test_card("card-a", "First")],
            vec![],
        )
        .unwrap();
        let second = save_semantic_snapshot(
            &mut connection,
            "board-1".to_string(),
            "Added card and connection".to_string(),
            Some(first.clone()),
            None,
            None,
            vec![test_card("card-a", "First"), test_card("card-b", "Second")],
            vec![EdgeInput {
                id: "edge-a-b".to_string(),
                source: "card-a".to_string(),
                target: "card-b".to_string(),
                link_type: None,
            }],
        )
        .unwrap();
        let removed = save_semantic_snapshot(
            &mut connection,
            "board-1".to_string(),
            "Removed second card".to_string(),
            Some(second.clone()),
            None,
            None,
            vec![test_card("card-a", "First")],
            vec![],
        )
        .unwrap();

        let restored_initial = SnapshotResolver::resolve(&connection, "board-1", &first).unwrap();
        let restored_second = SnapshotResolver::resolve(&connection, "board-1", &second).unwrap();
        let restored_removed = SnapshotResolver::resolve(&connection, "board-1", &removed).unwrap();

        assert_eq!(restored_initial.placements.len(), 1);
        assert_eq!(restored_second.placements.len(), 2);
        assert!(restored_second.links.contains_key("edge-a-b"));
        assert_eq!(restored_removed.placements.len(), 1);
        assert!(restored_removed.links.is_empty());

        let other_board_snapshot = save_semantic_snapshot(
            &mut connection,
            "board-2".to_string(),
            "Independent board".to_string(),
            None,
            None,
            None,
            vec![test_card("board-2-card", "Independent")],
            vec![],
        )
        .unwrap();
        let other_board =
            SnapshotResolver::resolve(&connection, "board-2", &other_board_snapshot).unwrap();
        assert_eq!(other_board.placements.len(), 1);
        assert!(!other_board.placements.contains_key("card-a"));
        assert!(SnapshotResolver::resolve(&connection, "board-1", &other_board_snapshot).is_err());

        let other_board_card_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM objects WHERE board_id = 'board-2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(other_board_card_count, 1);
    }

    fn test_card(id: &str, title: &str) -> NodeInput {
        NodeInput {
            id: id.to_string(),
            node_type: "voraCard".to_string(),
            title: title.to_string(),
            content: String::new(),
            version: "v0.1.0".to_string(),
            data: json!({ "title": title, "content": "" }),
            x: 10.0,
            y: 20.0,
            width: Some(280.0),
            height: Some(160.0),
            style: None,
        }
    }

    #[test]
    fn old_node_payload_defaults_to_card() {
        let payload = json!({
            "id": "card-1",
            "title": "Card",
            "content": "Legacy content",
            "version": "v0.1.0",
            "x": 0.0,
            "y": 0.0
        });
        let node: NodeInput = serde_json::from_value(payload).unwrap();

        assert_eq!(node.node_type, "voraCard");
        assert!(node.data.is_null());
    }

    #[test]
    fn migration_assigns_existing_data_to_default_board_and_board_delete_is_safe() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE objects (
                    id TEXT PRIMARY KEY, type TEXT NOT NULL, created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL, deleted_at INTEGER NULL, metadata JSON
                 );
                 CREATE TABLE mind_snapshots (
                    id TEXT PRIMARY KEY, name TEXT NOT NULL, parent_snapshot_id TEXT,
                    depth INTEGER NOT NULL, replay_cost INTEGER NOT NULL DEFAULT 0,
                    is_checkpoint INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL
                 );
                 INSERT INTO objects (id, type, created_at, updated_at) VALUES ('legacy-card', 'card', 1, 1);
                 INSERT INTO mind_snapshots (id, name, depth, is_checkpoint, created_at)
                    VALUES ('legacy-snapshot', 'Legacy', 0, 1, 1);",
            )
            .unwrap();

        init_db(&connection).unwrap();
        let legacy_object_board: String = connection
            .query_row(
                "SELECT board_id FROM objects WHERE id = 'legacy-card'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let legacy_snapshot_board: String = connection
            .query_row(
                "SELECT board_id FROM mind_snapshots WHERE id = 'legacy-snapshot'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy_object_board, "board-1");
        assert_eq!(legacy_snapshot_board, "board-1");

        let first_board = get_all_boards(&connection).unwrap();
        assert_eq!(first_board.len(), 1);
        assert_eq!(first_board[0].id, "board-1");
        create_board(&mut connection, "Disposable", 2, "board-2").unwrap();
        save_semantic_snapshot(
            &mut connection,
            "board-2".to_string(),
            "Board content".to_string(),
            None,
            None,
            None,
            vec![test_card("disposable-card", "Disposable")],
            vec![],
        )
        .unwrap();
        delete_board(&mut connection, "board-1").unwrap();
        assert_eq!(get_all_boards(&connection).unwrap().len(), 1);
        let migrated_data: i64 = connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM objects WHERE board_id = 'board-1') +
                    (SELECT COUNT(*) FROM mind_snapshots WHERE board_id = 'board-1')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(migrated_data, 0);
        delete_board(&mut connection, "board-2").unwrap();
        assert!(get_all_boards(&connection).unwrap().is_empty());
        let remaining_data: i64 = connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM objects WHERE board_id = 'board-2') +
                    (SELECT COUNT(*) FROM mind_snapshots WHERE board_id = 'board-2')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining_data, 0);
    }
}
