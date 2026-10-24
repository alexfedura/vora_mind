use rusqlite::Connection;

use super::db;
use super::model::*;
use super::revision;
use super::tree;

/// Helper struct for split parts.
#[derive(Debug, Clone)]
pub struct SegmentPart {
    pub label: String,
    pub start_sec: Option<i64>,
    pub end_sec: Option<i64>,
    pub page_start: Option<i64>,
    pub page_end: Option<i64>,
    pub line_start: Option<i64>,
    pub line_end: Option<i64>,
    pub notes: String,
}

/// SPLIT: create N children under `parent_id`.
/// Each child inherits source_id from parent.
/// Children do NOT copy time/page ranges automatically —
/// the caller must supply label + optional range for each.
pub fn split_segment(
    conn: &Connection,
    parent_id: &str,
    parts: Vec<SegmentPart>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    if parts.is_empty() {
        return Err("split requires at least one part".into());
    }
    let parent = db::get_segment(conn, parent_id)
        .map_err(|e| format!("parent segment '{}' not found: {}", parent_id, e))?;

    let mut new_ids = Vec::new();
    for (i, part) in parts.into_iter().enumerate() {
        let id = db::create_segment(
            conn,
            Segment {
                id: String::new(),
                source_id: parent.source_id.clone(),
                label: part.label,
                start_sec: part.start_sec,
                end_sec: part.end_sec,
                page_start: part.page_start,
                page_end: part.page_end,
                line_start: part.line_start,
                line_end: part.line_end,
                notes: part.notes,
                status: String::new(),
                created_at: 0,
                updated_at: 0,
                parent_segment_id: Some(parent_id.to_string()),
                order_index: i as i64,
            },
        )?;
        new_ids.push(id);
    }

    // Record a revision on the PARENT indicating it was split.
    let after = format!(
        r#"{{"action":"split","children":[{}]}}"#,
        new_ids
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<_>>()
            .join(",")
    );
    revision::record(
        conn,
        EntityKind::Segment,
        parent_id,
        RevisionType::Update,
        None,
        &after,
        "split into children",
        "user",
    )?;

    Ok(new_ids)
}

/// MERGE: create a parent that spans all children.
/// The new parent's range = union of children ranges.
/// Children are re-parented to the new parent.
pub fn merge_segments(
    conn: &Connection,
    child_ids: &[String],
    parent_label: &str,
    notes: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    if child_ids.len() < 2 {
        return Err("merge requires at least two segments".into());
    }

    let children: Vec<Segment> = child_ids
        .iter()
        .map(|id| db::get_segment(conn, id))
        .collect::<Result<Vec<_>, _>>()?;

    let source_id = children[0].source_id.clone();
    for c in &children {
        if c.source_id != source_id {
            return Err("cannot merge segments from different sources".into());
        }
    }

    // Compute union range.
    let start_sec = children.iter().filter_map(|c| c.start_sec).min();
    let end_sec = children.iter().filter_map(|c| c.end_sec).max();
    let page_start = children.iter().filter_map(|c| c.page_start).min();
    let page_end = children.iter().filter_map(|c| c.page_end).max();
    let line_start = children.iter().filter_map(|c| c.line_start).min();
    let line_end = children.iter().filter_map(|c| c.line_end).max();

    // Create the new parent.
    let parent_id = db::create_segment(
        conn,
        Segment {
            id: String::new(),
            source_id: source_id.clone(),
            label: parent_label.to_string(),
            start_sec,
            end_sec,
            page_start,
            page_end,
            line_start,
            line_end,
            notes: notes.unwrap_or_default(),
            status: String::new(),
            created_at: 0,
            updated_at: 0,
            parent_segment_id: None,
            order_index: 0,
        },
    )?;

    // Re-parent each child, preserving order.
    let now = now_ts();
    for (i, child) in children.iter().enumerate() {
        let before = serde_json::to_string(child)?;
        conn.execute(
            "UPDATE segments
                SET parent_segment_id = ?1,
                    order_index = ?2,
                    updated_at = ?3
              WHERE id = ?4",
            rusqlite::params![parent_id, i as i64, now, child.id],
        )?;
        let after_seg = db::get_segment(conn, &child.id)?;
        let after = serde_json::to_string(&after_seg)?;
        revision::record(
            conn,
            EntityKind::Segment,
            &child.id,
            RevisionType::Update,
            Some(&before),
            &after,
            &format!("merged under {}", parent_id),
            "user",
        )?;
    }

    Ok(parent_id)
}

/// MOVE: change a segment's parent.
/// Cycle check: the new parent must NOT be a descendant of the
/// moving segment (and must not be itself).
pub fn move_segment(
    conn: &Connection,
    id: &str,
    new_parent_id: Option<&str>,
    new_order: Option<i64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let seg = db::get_segment(conn, id)?;

    if let Some(np) = new_parent_id {
        if np == id {
            return Err(format!("cannot move segment '{}' under itself", id).into());
        }
        let np_seg = db::get_segment(conn, np)
            .map_err(|e| format!("new parent '{}' not found: {}", np, e))?;
        if np_seg.source_id != seg.source_id {
            return Err("cannot move segment across sources".into());
        }
        if tree::is_ancestor_or_self(conn, id, np)? {
            return Err(format!(
                "move would create a cycle: '{}' is an ancestor of '{}'",
                id, np
            )
            .into());
        }
    }

    let before = serde_json::to_string(&seg)?;
    let now = now_ts();
    let order = new_order.unwrap_or(seg.order_index);
    conn.execute(
        "UPDATE segments
            SET parent_segment_id = ?1,
                order_index = ?2,
                updated_at = ?3
          WHERE id = ?4",
        rusqlite::params![new_parent_id, order, now, id],
    )?;
    let after_seg = db::get_segment(conn, id)?;
    let after = serde_json::to_string(&after_seg)?;
    revision::record(
        conn,
        EntityKind::Segment,
        id,
        RevisionType::Update,
        Some(&before),
        &after,
        "move segment",
        "user",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::db::init_schema;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    fn seed_source(conn: &Connection) -> String {
        db::create_source(
            conn,
            Source {
                id: String::new(),
                kind: SourceKind::Video,
                path: None,
                title: "t".into(),
                author: None,
                language: None,
                duration_sec: Some(3600),
                page_count: None,
                metadata_json: "{}".into(),
                notes: String::new(),
                status: String::new(),
                created_at: 0,
                updated_at: 0,
            },
        )
        .unwrap()
    }

    fn make_seg(
        conn: &Connection,
        src: &str,
        label: &str,
        start: Option<i64>,
        end: Option<i64>,
    ) -> String {
        db::create_segment(
            conn,
            Segment {
                id: String::new(),
                source_id: src.into(),
                label: label.into(),
                start_sec: start,
                end_sec: end,
                page_start: None,
                page_end: None,
                line_start: None,
                line_end: None,
                notes: String::new(),
                status: String::new(),
                created_at: 0,
                updated_at: 0,
                parent_segment_id: None,
                order_index: 0,
            },
        )
        .unwrap()
    }

    fn part(label: &str, start: Option<i64>, end: Option<i64>) -> SegmentPart {
        SegmentPart {
            label: label.into(),
            start_sec: start,
            end_sec: end,
            page_start: None,
            page_end: None,
            line_start: None,
            line_end: None,
            notes: String::new(),
        }
    }

    #[test]
    fn split_creates_children() {
        let conn = mem_conn();
        let src = seed_source(&conn);
        let parent = make_seg(&conn, &src, "full", Some(0), Some(3600));

        let kids = split_segment(
            &conn,
            &parent,
            vec![
                part("part A", Some(0), Some(1800)),
                part("part B", Some(1800), Some(3600)),
            ],
        )
        .unwrap();
        assert_eq!(kids.len(), 2);

        let children = db::list_children(&conn, &parent).unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].order_index, 0);
        assert_eq!(children[1].order_index, 1);
    }

    #[test]
    fn merge_creates_parent() {
        let conn = mem_conn();
        let src = seed_source(&conn);
        let a = make_seg(&conn, &src, "a", Some(0), Some(100));
        let b = make_seg(&conn, &src, "b", Some(100), Some(200));

        let p = merge_segments(&conn, &[a.clone(), b.clone()], "merged", None).unwrap();
        let parent = db::get_segment(&conn, &p).unwrap();
        assert_eq!(parent.start_sec, Some(0));
        assert_eq!(parent.end_sec, Some(200));

        let ca = db::get_segment(&conn, &a).unwrap();
        let cb = db::get_segment(&conn, &b).unwrap();
        assert_eq!(ca.parent_segment_id.as_deref(), Some(p.as_str()));
        assert_eq!(cb.parent_segment_id.as_deref(), Some(p.as_str()));
    }

    #[test]
    fn move_rejects_cycle() {
        let conn = mem_conn();
        let src = seed_source(&conn);
        let a = make_seg(&conn, &src, "a", Some(0), Some(100));
        let b = make_seg(&conn, &src, "b", Some(100), Some(200));
        move_segment(&conn, &b, Some(&a), Some(0)).unwrap();

        // Try to move `a` under its child `b` -> cycle.
        let r = move_segment(&conn, &a, Some(&b), Some(0));
        assert!(r.is_err());
        let msg = r.unwrap_err().to_string();
        assert!(msg.contains("cycle"), "expected cycle error, got: {}", msg);
    }

    #[test]
    fn move_self_rejected() {
        let conn = mem_conn();
        let src = seed_source(&conn);
        let a = make_seg(&conn, &src, "a", Some(0), Some(100));
        let r = move_segment(&conn, &a, Some(&a), None);
        assert!(r.is_err());
    }

    #[test]
    fn ancestors_walk_up() {
        let conn = mem_conn();
        let src = seed_source(&conn);
        let a = make_seg(&conn, &src, "a", Some(0), Some(300));
        let kids = split_segment(&conn, &a, vec![part("k", Some(0), Some(300))]).unwrap();
        let chain = tree::ancestors(&conn, &kids[0]).unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].id, kids[0]);
        assert_eq!(chain[1].id, a);
    }

    #[test]
    fn descendants_finds_all() {
        let conn = mem_conn();
        let src = seed_source(&conn);
        let a = make_seg(&conn, &src, "a", Some(0), Some(300));
        let kids = split_segment(
            &conn,
            &a,
            vec![part("k1", None, None), part("k2", None, None)],
        )
        .unwrap();
        // Split k1 into one grandchild.
        split_segment(&conn, &kids[0], vec![part("g", None, None)]).unwrap();

        let all = tree::descendants(&conn, &a).unwrap();
        assert_eq!(all.len(), 3); // k1, k2, g
    }

    #[test]
    fn merge_requires_two() {
        let conn = mem_conn();
        let src = seed_source(&conn);
        let a = make_seg(&conn, &src, "a", Some(0), Some(100));
        assert!(merge_segments(&conn, &[a], "x", None).is_err());
    }

    #[test]
    fn merge_rejects_mixed_sources() {
        let conn = mem_conn();
        let s1 = seed_source(&conn);
        let s2 = seed_source(&conn);
        let a = make_seg(&conn, &s1, "a", Some(0), Some(100));
        let b = make_seg(&conn, &s2, "b", Some(0), Some(100));
        assert!(merge_segments(&conn, &[a, b], "x", None).is_err());
    }

    #[test]
    fn split_records_revision_on_parent() {
        let conn = mem_conn();
        let src = seed_source(&conn);
        let a = make_seg(&conn, &src, "a", Some(0), Some(100));
        split_segment(&conn, &a, vec![part("x", None, None)]).unwrap();
        let revs = revision::list_for_entity(&conn, EntityKind::Segment, &a).unwrap();
        // create + update(split) = 2
        assert_eq!(revs.len(), 2);
        assert_eq!(revs[1].revision_type, RevisionType::Update);
    }

    #[test]
    fn build_tree_from_list_orders_children() {
        let conn = mem_conn();
        let src = seed_source(&conn);
        let a = make_seg(&conn, &src, "a", Some(0), Some(300));
        let kids = split_segment(
            &conn,
            &a,
            vec![part("z", None, None), part("y", None, None)],
        )
        .unwrap();
        let t = tree::build_tree(&conn, &src).unwrap();
        assert_eq!(t.len(), 1); // root
        assert_eq!(t[0].segment.id, a);
        assert_eq!(t[0].children.len(), 2);
        assert_eq!(t[0].children[0].segment.id, kids[0]);
        assert_eq!(t[0].children[1].segment.id, kids[1]);
    }

    #[test]
    fn move_to_root_clears_parent() {
        let conn = mem_conn();
        let src = seed_source(&conn);
        let a = make_seg(&conn, &src, "a", Some(0), Some(100));
        let b = make_seg(&conn, &src, "b", Some(100), Some(200));
        // Make b a child of a.
        move_segment(&conn, &b, Some(&a), None).unwrap();
        // Now move b back to root.
        move_segment(&conn, &b, None, None).unwrap();
        let got = db::get_segment(&conn, &b).unwrap();
        assert!(got.parent_segment_id.is_none());
    }
}
