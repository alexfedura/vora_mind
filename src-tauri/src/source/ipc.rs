//! Tauri IPC layer for the source module.
//!
//! All functions here are thin wrappers over db.rs / ops.rs / tree.rs.
//! Business logic lives in those modules; this file only maps
//! types and converts errors to String at the IPC boundary.
//!
//! The 9 commands:
//!   1. src_list_sources
//!   2. src_get_source_bundle
//!   3. src_list_extracts
//!   4. src_get_extract
//!   5. src_create_source
//!   6. src_create_segment
//!   7. src_create_extract
//!   8. src_create_link
//!   9. src_mark_extract_reviewed

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::mind::canvas_db::AppState;
use crate::source::{db, model::*, tree};

// ══════════════════════════════════════════════════════════════════
// DTOs — request / response shapes
// ══════════════════════════════════════════════════════════════════

#[derive(Serialize)]
pub struct SourceSummary {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub author: Option<String>,
    pub duration_sec: Option<i64>,
    pub status: String,
}

#[derive(Serialize)]
pub struct SegmentTreeNode {
    pub id: String,
    pub label: String,
    pub start_sec: Option<i64>,
    pub end_sec: Option<i64>,
    pub page_start: Option<i64>,
    pub page_end: Option<i64>,
    pub order_index: i64,
    pub children: Vec<SegmentTreeNode>,
}

#[derive(Serialize)]
pub struct SourceBundle {
    pub source: Source,
    pub segment_tree: Vec<SegmentTreeNode>,
    pub segment_count: usize,
    pub extract_count: usize,
}

#[derive(Serialize)]
pub struct ExtractSummary {
    pub id: String,
    pub segment_id: String,
    pub transform_kind: String,
    pub content_preview: String, // first 200 chars
    pub language: Option<String>,
    pub confidence: String,
    pub created_by: String,
    pub reviewed_by_user: bool,
    pub superseded_by: Option<String>,
    pub created_at: i64,
}

#[derive(Serialize)]
pub struct ExtractDetail {
    pub extract: Extract,
    pub segment: Segment,
    pub source: Source,
    pub links: Vec<Link>,
}

#[derive(Deserialize)]
pub struct CreateSourceReq {
    pub kind: String,
    pub title: String,
    pub path: Option<String>,
    pub author: Option<String>,
    pub language: Option<String>,
    pub duration_sec: Option<i64>,
    pub page_count: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateSegmentReq {
    pub source_id: String,
    pub label: String,
    pub start_sec: Option<i64>,
    pub end_sec: Option<i64>,
    pub page_start: Option<i64>,
    pub page_end: Option<i64>,
    pub line_start: Option<i64>,
    pub line_end: Option<i64>,
    pub parent_segment_id: Option<String>,
    pub order_index: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateExtractReq {
    pub segment_id: String,
    pub transform_kind: String,
    pub content: String,
    pub language: Option<String>,
    pub confidence: Option<String>,
    pub parent_extract_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateLinkReq {
    pub extract_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub link_kind: Option<String>,
    pub note: Option<String>,
}

// ══════════════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════════════

fn e<E: std::fmt::Display>(err: E) -> String {
    format!("{}", err)
}

fn to_tree_node(n: &tree::SegmentNode) -> SegmentTreeNode {
    SegmentTreeNode {
        id: n.segment.id.clone(),
        label: n.segment.label.clone(),
        start_sec: n.segment.start_sec,
        end_sec: n.segment.end_sec,
        page_start: n.segment.page_start,
        page_end: n.segment.page_end,
        order_index: n.segment.order_index,
        children: n.children.iter().map(to_tree_node).collect(),
    }
}

fn preview(s: &str) -> String {
    if s.chars().count() <= 200 {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(197).collect();
        out.push_str("...");
        out
    }
}

fn count_nodes(ns: &[tree::SegmentNode]) -> usize {
    ns.iter().map(|n| 1 + count_nodes(&n.children)).sum()
}

fn walk_nodes(n: &tree::SegmentNode) -> Vec<&tree::SegmentNode> {
    let mut out = vec![n];
    for c in &n.children {
        out.extend(walk_nodes(c));
    }
    out
}

// ══════════════════════════════════════════════════════════════════
// _impl functions — testable without IPC overhead
// ══════════════════════════════════════════════════════════════════

pub fn list_sources_impl(
    conn: &rusqlite::Connection,
    kind: Option<&str>,
) -> Result<Vec<SourceSummary>, String> {
    let rows = db::list_sources(conn, kind).map_err(e)?;
    Ok(rows
        .into_iter()
        .map(|s| SourceSummary {
            id: s.id,
            kind: s.kind.as_str().to_string(),
            title: s.title,
            author: s.author,
            duration_sec: s.duration_sec,
            status: s.status,
        })
        .collect())
}

pub fn get_source_bundle_impl(
    conn: &rusqlite::Connection,
    id: &str,
) -> Result<SourceBundle, String> {
    let source = db::get_source(conn, id).map_err(e)?;
    let roots = tree::build_tree(conn, id).map_err(e)?;
    let segment_count = count_nodes(&roots);
    let extract_count = roots
        .iter()
        .flat_map(|r| walk_nodes(r))
        .map(|n| {
            db::list_extracts(conn, &n.segment.id)
                .map(|v| v.len())
                .unwrap_or(0)
        })
        .sum();
    Ok(SourceBundle {
        source,
        segment_tree: roots.iter().map(to_tree_node).collect(),
        segment_count,
        extract_count,
    })
}

pub fn list_extracts_impl(
    conn: &rusqlite::Connection,
    segment_id: &str,
) -> Result<Vec<ExtractSummary>, String> {
    let rows = db::list_extracts(conn, segment_id).map_err(e)?;
    Ok(rows
        .into_iter()
        .map(|ex| ExtractSummary {
            id: ex.id,
            segment_id: ex.segment_id,
            transform_kind: ex.transform_kind.as_str().to_string(),
            content_preview: preview(&ex.content),
            language: ex.language,
            confidence: ex.confidence.as_str().to_string(),
            created_by: ex.created_by.as_str().to_string(),
            reviewed_by_user: ex.reviewed_by_user,
            superseded_by: ex.superseded_by,
            created_at: ex.created_at,
        })
        .collect())
}

pub fn get_extract_impl(conn: &rusqlite::Connection, id: &str) -> Result<ExtractDetail, String> {
    let extract = db::get_extract(conn, id).map_err(e)?;
    let segment = db::get_segment(conn, &extract.segment_id).map_err(e)?;
    let source = db::get_source(conn, &segment.source_id).map_err(e)?;
    let links = db::list_links_for_extract(conn, id).map_err(e)?;
    Ok(ExtractDetail {
        extract,
        segment,
        source,
        links,
    })
}

pub fn create_source_impl(
    conn: &rusqlite::Connection,
    req: CreateSourceReq,
) -> Result<String, String> {
    let kind =
        SourceKind::from_str(&req.kind).ok_or_else(|| format!("invalid kind: {}", req.kind))?;
    db::create_source(
        conn,
        Source {
            id: String::new(),
            kind,
            path: req.path,
            title: req.title,
            author: req.author,
            language: req.language,
            duration_sec: req.duration_sec,
            page_count: req.page_count,
            metadata_json: "{}".into(),
            notes: req.notes.unwrap_or_default(),
            status: String::new(),
            created_at: 0,
            updated_at: 0,
        },
    )
    .map_err(e)
}

pub fn create_segment_impl(
    conn: &rusqlite::Connection,
    req: CreateSegmentReq,
) -> Result<String, String> {
    if let Some(pid) = &req.parent_segment_id {
        db::get_segment(conn, pid).map_err(e)?;
    }
    db::create_segment(
        conn,
        Segment {
            id: String::new(),
            source_id: req.source_id,
            label: req.label,
            start_sec: req.start_sec,
            end_sec: req.end_sec,
            page_start: req.page_start,
            page_end: req.page_end,
            line_start: req.line_start,
            line_end: req.line_end,
            notes: req.notes.unwrap_or_default(),
            status: String::new(),
            created_at: 0,
            updated_at: 0,
            parent_segment_id: req.parent_segment_id,
            order_index: req.order_index.unwrap_or(0),
        },
    )
    .map_err(e)
}

pub fn create_extract_impl(
    conn: &rusqlite::Connection,
    req: CreateExtractReq,
) -> Result<String, String> {
    let tk = TransformKind::from_str(&req.transform_kind)
        .ok_or_else(|| format!("invalid transform_kind: {}", req.transform_kind))?;
    let cf = match req.confidence.as_deref() {
        None | Some("certain") => Confidence::Certain,
        Some("probable") => Confidence::Probable,
        Some("uncertain") => Confidence::Uncertain,
        Some(other) => return Err(format!("invalid confidence: {}", other)),
    };
    if let Some(pid) = &req.parent_extract_id {
        db::get_extract(conn, pid).map_err(e)?;
    }
    db::create_extract(
        conn,
        Extract {
            id: String::new(),
            segment_id: req.segment_id,
            transform_kind: tk,
            content: req.content,
            language: req.language,
            confidence: cf,
            created_by: CreatedBy::User,
            reviewed_by_user: true,
            parent_extract_id: req.parent_extract_id,
            superseded_by: None,
            notes: req.notes.unwrap_or_default(),
            status: String::new(),
            created_at: 0,
            updated_at: 0,
        },
    )
    .map_err(e)
}

pub fn create_link_impl(conn: &rusqlite::Connection, req: CreateLinkReq) -> Result<String, String> {
    let tk = match req.target_kind.as_str() {
        "mind_node" => LinkTargetKind::MindNode,
        "chart_annotation" => LinkTargetKind::ChartAnnotation,
        "source" => LinkTargetKind::Source,
        "external" => LinkTargetKind::External,
        other => return Err(format!("invalid target_kind: {}", other)),
    };
    let lk = match req.link_kind.as_deref() {
        None | Some("reference") => LinkKind::Reference,
        Some("evidence") => LinkKind::Evidence,
        Some("inspiration") => LinkKind::Inspiration,
        Some("contradicts") => LinkKind::Contradicts,
        Some("supports") => LinkKind::Supports,
        Some(other) => return Err(format!("invalid link_kind: {}", other)),
    };
    db::create_link(
        conn,
        Link {
            id: String::new(),
            extract_id: req.extract_id,
            target_kind: tk,
            target_id: req.target_id,
            link_kind: lk,
            note: req.note.unwrap_or_default(),
            status: String::new(),
            created_at: 0,
            updated_at: 0,
        },
    )
    .map_err(e)
}

pub fn mark_extract_reviewed_impl(conn: &rusqlite::Connection, id: &str) -> Result<(), String> {
    db::mark_extract_reviewed(conn, id).map_err(e)
}

// ══════════════════════════════════════════════════════════════════
// Tauri commands — lock the shared connection and delegate to _impl
// ══════════════════════════════════════════════════════════════════

#[tauri::command]
pub fn src_list_sources(
    kind: Option<String>,
    state: State<AppState>,
) -> Result<Vec<SourceSummary>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    list_sources_impl(&conn, kind.as_deref())
}

#[tauri::command]
pub fn src_get_source_bundle(id: String, state: State<AppState>) -> Result<SourceBundle, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    get_source_bundle_impl(&conn, &id)
}

#[tauri::command]
pub fn src_list_extracts(
    segment_id: String,
    state: State<AppState>,
) -> Result<Vec<ExtractSummary>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    list_extracts_impl(&conn, &segment_id)
}

#[tauri::command]
pub fn src_get_extract(id: String, state: State<AppState>) -> Result<ExtractDetail, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    get_extract_impl(&conn, &id)
}

#[tauri::command]
pub fn src_create_source(req: CreateSourceReq, state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    create_source_impl(&conn, req)
}

#[tauri::command]
pub fn src_create_segment(req: CreateSegmentReq, state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    create_segment_impl(&conn, req)
}

#[tauri::command]
pub fn src_create_extract(req: CreateExtractReq, state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    create_extract_impl(&conn, req)
}

#[tauri::command]
pub fn src_create_link(req: CreateLinkReq, state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    create_link_impl(&conn, req)
}

#[tauri::command]
pub fn src_mark_extract_reviewed(id: String, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    mark_extract_reviewed_impl(&conn, &id)
}

// ══════════════════════════════════════════════════════════════════
// Tests — call _impl variants directly with an in-memory DB
// ══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::db::init_schema;
    use rusqlite::Connection;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    fn seed_source(conn: &Connection) -> String {
        create_source_impl(
            conn,
            CreateSourceReq {
                kind: "video".into(),
                title: "Test Video".into(),
                path: Some("/test.mp4".into()),
                author: None,
                language: Some("en".into()),
                duration_sec: Some(3600),
                page_count: None,
                notes: None,
            },
        )
        .unwrap()
    }

    fn seed_segment(conn: &Connection, source_id: &str) -> String {
        create_segment_impl(
            conn,
            CreateSegmentReq {
                source_id: source_id.into(),
                label: "Full".into(),
                start_sec: Some(0),
                end_sec: Some(3600),
                page_start: None,
                page_end: None,
                line_start: None,
                line_end: None,
                parent_segment_id: None,
                order_index: Some(0),
                notes: None,
            },
        )
        .unwrap()
    }

    fn seed_extract(conn: &Connection, segment_id: &str) -> String {
        create_extract_impl(
            conn,
            CreateExtractReq {
                segment_id: segment_id.into(),
                transform_kind: "transcribed".into(),
                content: "Sample transcription content.".into(),
                language: Some("en".into()),
                confidence: None,
                parent_extract_id: None,
                notes: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn create_source_then_list() {
        let conn = mem_conn();
        let id = seed_source(&conn);
        assert!(id.starts_with("SRC-"));

        let list = list_sources_impl(&conn, None).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id);
        assert_eq!(list[0].kind, "video");
    }

    #[test]
    fn create_source_then_bundle() {
        let conn = mem_conn();
        let src_id = seed_source(&conn);
        let seg_id = seed_segment(&conn, &src_id);

        let bundle = get_source_bundle_impl(&conn, &src_id).unwrap();
        assert_eq!(bundle.source.id, src_id);
        assert_eq!(bundle.segment_count, 1);
        assert_eq!(bundle.segment_tree.len(), 1);
        assert_eq!(bundle.segment_tree[0].id, seg_id);

        // Add an extract and verify count
        seed_extract(&conn, &seg_id);
        let bundle2 = get_source_bundle_impl(&conn, &src_id).unwrap();
        assert_eq!(bundle2.extract_count, 1);
    }

    #[test]
    fn create_extract_then_get() {
        let conn = mem_conn();
        let src_id = seed_source(&conn);
        let seg_id = seed_segment(&conn, &src_id);
        let ext_id = seed_extract(&conn, &seg_id);

        let detail = get_extract_impl(&conn, &ext_id).unwrap();
        assert_eq!(detail.extract.id, ext_id);
        assert_eq!(detail.segment.id, seg_id);
        assert_eq!(detail.source.id, src_id);
        assert!(detail.links.is_empty());
    }

    #[test]
    fn create_link_then_get_extract() {
        let conn = mem_conn();
        let src_id = seed_source(&conn);
        let seg_id = seed_segment(&conn, &src_id);
        let ext_id = seed_extract(&conn, &seg_id);

        let lnk_id = create_link_impl(
            &conn,
            CreateLinkReq {
                extract_id: ext_id.clone(),
                target_kind: "mind_node".into(),
                target_id: "card-001".into(),
                link_kind: Some("evidence".into()),
                note: Some("key evidence".into()),
            },
        )
        .unwrap();
        assert!(lnk_id.starts_with("LNK-"));

        let detail = get_extract_impl(&conn, &ext_id).unwrap();
        assert_eq!(detail.links.len(), 1);
    }

    #[test]
    fn mark_reviewed_works() {
        let conn = mem_conn();
        let src_id = seed_source(&conn);
        let seg_id = seed_segment(&conn, &src_id);

        // Create unreviewed extract
        let ext_id = create_extract_impl(
            &conn,
            CreateExtractReq {
                segment_id: seg_id,
                transform_kind: "summarized".into(),
                content: "auto summary".into(),
                language: None,
                confidence: Some("probable".into()),
                parent_extract_id: None,
                notes: None,
            },
        )
        .unwrap();

        // Force reviewed_by_user = 0 to test the mark function
        conn.execute(
            "UPDATE extracts SET reviewed_by_user = 0 WHERE id = ?1",
            [&ext_id],
        )
        .unwrap();

        let before = db::get_extract(&conn, &ext_id).unwrap();
        assert!(!before.reviewed_by_user);

        mark_extract_reviewed_impl(&conn, &ext_id).unwrap();

        let after = db::get_extract(&conn, &ext_id).unwrap();
        assert!(after.reviewed_by_user);
    }

    #[test]
    fn invalid_kind_returns_error() {
        let conn = mem_conn();
        let err = create_source_impl(
            &conn,
            CreateSourceReq {
                kind: "nonexistent".into(),
                title: "x".into(),
                path: None,
                author: None,
                language: None,
                duration_sec: None,
                page_count: None,
                notes: None,
            },
        );
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("invalid kind"));
    }

    #[test]
    fn list_extracts_preview_truncated() {
        let conn = mem_conn();
        let src_id = seed_source(&conn);
        let seg_id = seed_segment(&conn, &src_id);
        // 250-char content
        let long_content = "a".repeat(250);
        create_extract_impl(
            &conn,
            CreateExtractReq {
                segment_id: seg_id.clone(),
                transform_kind: "quoted".into(),
                content: long_content.clone(),
                language: None,
                confidence: None,
                parent_extract_id: None,
                notes: None,
            },
        )
        .unwrap();

        let list = list_extracts_impl(&conn, &seg_id).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].content_preview.len() <= 200);
        assert!(list[0].content_preview.ends_with("..."));
    }

    #[test]
    fn segment_with_parent_guard() {
        let conn = mem_conn();
        let src_id = seed_source(&conn);
        // Providing a nonexistent parent_segment_id must fail
        let result = create_segment_impl(
            &conn,
            CreateSegmentReq {
                source_id: src_id,
                label: "orphan".into(),
                start_sec: None,
                end_sec: None,
                page_start: None,
                page_end: None,
                line_start: None,
                line_end: None,
                parent_segment_id: Some("SEG-9999".into()),
                order_index: None,
                notes: None,
            },
        );
        assert!(result.is_err());
    }
}
