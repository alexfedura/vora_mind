use rusqlite::Connection;

use super::db;
use super::model::Segment;

/// A node in the segment tree (with computed children).
#[derive(Debug, Clone)]
pub struct SegmentNode {
    pub segment: Segment,
    pub children: Vec<SegmentNode>,
    pub depth: usize,
}

/// Return ALL segments of a source ordered by (parent, order_index, id).
pub fn list_all_segments(
    conn: &Connection,
    source_id: &str,
) -> Result<Vec<Segment>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT id, source_id, label, start_sec, end_sec,
                page_start, page_end, line_start, line_end,
                notes, status, created_at, updated_at,
                parent_segment_id, order_index
         FROM segments
         WHERE source_id = ?1
         ORDER BY
           COALESCE(parent_segment_id, ''),
           order_index ASC,
           id ASC",
    )?;
    let rows = stmt.query_map([source_id], |r| {
        Ok(Segment {
            id: r.get(0)?,
            source_id: r.get(1)?,
            label: r.get(2)?,
            start_sec: r.get(3)?,
            end_sec: r.get(4)?,
            page_start: r.get(5)?,
            page_end: r.get(6)?,
            line_start: r.get(7)?,
            line_end: r.get(8)?,
            notes: r.get(9)?,
            status: r.get(10)?,
            created_at: r.get(11)?,
            updated_at: r.get(12)?,
            parent_segment_id: r.get(13)?,
            order_index: r.get(14)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Build the full tree for a source. Returns the roots.
pub fn build_tree(
    conn: &Connection,
    source_id: &str,
) -> Result<Vec<SegmentNode>, Box<dyn std::error::Error>> {
    let all = list_all_segments(conn, source_id)?;
    build_from_list(all)
}

/// Pure in-memory tree build (testable without DB).
pub fn build_from_list(all: Vec<Segment>) -> Result<Vec<SegmentNode>, Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    let mut by_parent: HashMap<Option<String>, Vec<Segment>> = HashMap::new();
    for s in all {
        by_parent
            .entry(s.parent_segment_id.clone())
            .or_default()
            .push(s);
    }
    // Sort each bucket by order_index then id.
    for v in by_parent.values_mut() {
        v.sort_by(|a, b| a.order_index.cmp(&b.order_index).then(a.id.cmp(&b.id)));
    }
    let roots = by_parent.remove(&None).unwrap_or_default();
    Ok(roots
        .into_iter()
        .map(|s| build_node(s, &mut by_parent, 0))
        .collect())
}

fn build_node(
    s: Segment,
    by_parent: &mut std::collections::HashMap<Option<String>, Vec<Segment>>,
    depth: usize,
) -> SegmentNode {
    let kids = by_parent.remove(&Some(s.id.clone())).unwrap_or_default();
    let children = kids
        .into_iter()
        .map(|c| build_node(c, by_parent, depth + 1))
        .collect();
    SegmentNode {
        segment: s,
        children,
        depth,
    }
}

/// Walk up from a segment to the root, returning the chain
/// [self, parent, grandparent, ..., root].
pub fn ancestors(conn: &Connection, id: &str) -> Result<Vec<Segment>, Box<dyn std::error::Error>> {
    let mut chain = Vec::new();
    let mut cur = Some(id.to_string());
    let mut guard = 0usize;
    while let Some(this) = cur {
        guard += 1;
        if guard > 10_000 {
            return Err(format!("ancestor chain too deep (likely cycle) at {}", this).into());
        }
        let seg = db::get_segment(conn, &this)?;
        cur = seg.parent_segment_id.clone();
        chain.push(seg);
    }
    Ok(chain)
}

/// All descendants of `id` (recursive), excluding self.
pub fn descendants(
    conn: &Connection,
    id: &str,
) -> Result<Vec<Segment>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    let mut stack = vec![id.to_string()];
    let mut guard = 0usize;
    while let Some(this) = stack.pop() {
        guard += 1;
        if guard > 10_000 {
            return Err(format!("descendant traversal too deep at {}", this).into());
        }
        let kids = db::list_children(conn, &this)?;
        for k in kids {
            stack.push(k.id.clone());
            out.push(k);
        }
    }
    Ok(out)
}

/// Check whether `maybe_ancestor_id` is an ancestor of `id` (or equal).
/// Used to prevent cycles on move.
pub fn is_ancestor_or_self(
    conn: &Connection,
    maybe_ancestor_id: &str,
    id: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    if maybe_ancestor_id == id {
        return Ok(true);
    }
    let chain = ancestors(conn, id)?;
    Ok(chain.iter().any(|s| s.id == maybe_ancestor_id))
}
