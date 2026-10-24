use rusqlite::Connection;

use super::model::*;
use super::revision;

// ── Schema bootstrap ─────────────────────────────────────────────

/// Idempotently apply the source-module schema on the given connection.
/// Safe to call multiple times; all statements are IF NOT EXISTS.
pub fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(include_str!("../../schema_source.sql"))?;
    migrate_segments_tree(conn)?;
    Ok(())
}

/// Idempotent: add hierarchy columns to segments if missing.
/// Safe to call on every startup.
pub fn migrate_segments_tree(conn: &Connection) -> Result<(), rusqlite::Error> {
    let existing: Vec<String> = conn
        .prepare("PRAGMA table_info(segments)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .filter_map(|x| x.ok())
        .collect();

    if !existing.iter().any(|c| c == "parent_segment_id") {
        conn.execute_batch(
            "ALTER TABLE segments
               ADD COLUMN parent_segment_id TEXT
                 REFERENCES segments(id);",
        )?;
    }
    if !existing.iter().any(|c| c == "order_index") {
        conn.execute_batch(
            "ALTER TABLE segments
               ADD COLUMN order_index INTEGER NOT NULL DEFAULT 0;",
        )?;
    }

    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_segments_parent
            ON segments(parent_segment_id);
         CREATE INDEX IF NOT EXISTS idx_segments_parent_order
            ON segments(parent_segment_id, order_index);",
    )?;
    Ok(())
}

// ── Sequential ID generator ─────────────────────────────────────

/// Returns next zero-padded sequential id for the given 3-letter prefix,
/// e.g. next_id(conn, "SRC") -> "SRC-0007".
pub fn next_id(conn: &Connection, prefix: &str) -> Result<String, Box<dyn std::error::Error>> {
    let table = match prefix {
        "SRC" => "sources",
        "SEG" => "segments",
        "EXT" => "extracts",
        "LNK" => "links",
        "REV" => "source_revisions",
        _ => return Err(format!("unknown prefix: {}", prefix).into()),
    };
    let max_id: Option<String> = conn
        .query_row(&format!("SELECT MAX(id) FROM {}", table), [], |r| r.get(0))
        .ok()
        .flatten();
    let next_n: u32 = match max_id {
        Some(id) if id.starts_with(prefix) => id[4..].parse::<u32>().unwrap_or(0) + 1,
        _ => 1,
    };
    Ok(format!("{}-{:04}", prefix, next_n))
}

// ── SOURCES ─────────────────────────────────────────────────────

pub fn create_source(
    conn: &Connection,
    mut s: Source,
) -> Result<String, Box<dyn std::error::Error>> {
    s.id = next_id(conn, "SRC")?;
    let now = now_ts();
    s.created_at = now;
    s.updated_at = now;
    if s.status.is_empty() {
        s.status = "active".into();
    }
    if s.metadata_json.is_empty() {
        s.metadata_json = "{}".into();
    }

    conn.execute(
        "INSERT INTO sources
           (id, kind, path, title, author, language,
            duration_sec, page_count, metadata_json,
            notes, status, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        rusqlite::params![
            s.id,
            s.kind.as_str(),
            s.path,
            s.title,
            s.author,
            s.language,
            s.duration_sec,
            s.page_count,
            s.metadata_json,
            s.notes,
            s.status,
            s.created_at,
            s.updated_at,
        ],
    )?;
    let after = serde_json::to_string(&s)?;
    revision::record(
        conn,
        EntityKind::Source,
        &s.id,
        RevisionType::Create,
        None,
        &after,
        "create source",
        "user",
    )?;
    Ok(s.id)
}

pub fn get_source(conn: &Connection, id: &str) -> Result<Source, Box<dyn std::error::Error>> {
    let s = conn.query_row(
        "SELECT id, kind, path, title, author, language,
                duration_sec, page_count, metadata_json,
                notes, status, created_at, updated_at
         FROM sources WHERE id = ?1",
        [id],
        |r| {
            Ok(Source {
                id: r.get(0)?,
                kind: SourceKind::from_str(&r.get::<_, String>(1)?).unwrap_or(SourceKind::Doc),
                path: r.get(2)?,
                title: r.get(3)?,
                author: r.get(4)?,
                language: r.get(5)?,
                duration_sec: r.get(6)?,
                page_count: r.get(7)?,
                metadata_json: r.get(8)?,
                notes: r.get(9)?,
                status: r.get(10)?,
                created_at: r.get(11)?,
                updated_at: r.get(12)?,
            })
        },
    )?;
    Ok(s)
}

pub fn list_sources(
    conn: &Connection,
    kind_filter: Option<&str>,
) -> Result<Vec<Source>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    let ids: Vec<String> = if let Some(k) = kind_filter {
        let mut stmt = conn.prepare("SELECT id FROM sources WHERE kind = ?1 ORDER BY id")?;
        let v: Vec<String> = stmt
            .query_map([k], |r| r.get(0))?
            .filter_map(|x| x.ok())
            .collect();
        v
    } else {
        let mut stmt = conn.prepare("SELECT id FROM sources ORDER BY id")?;
        let v: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .filter_map(|x| x.ok())
            .collect();
        v
    };
    for id in ids {
        out.push(get_source(conn, &id)?);
    }
    Ok(out)
}

// ── SEGMENTS ────────────────────────────────────────────────────

/// Row mapper shared by get_segment, list_segments, list_children.
fn map_segment(r: &rusqlite::Row<'_>) -> rusqlite::Result<Segment> {
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
}

const SEG_SELECT: &str = "SELECT id, source_id, label, start_sec, end_sec,
            page_start, page_end, line_start, line_end,
            notes, status, created_at, updated_at,
            parent_segment_id, order_index";

pub fn create_segment(
    conn: &Connection,
    mut s: Segment,
) -> Result<String, Box<dyn std::error::Error>> {
    s.id = next_id(conn, "SEG")?;
    let now = now_ts();
    s.created_at = now;
    s.updated_at = now;
    if s.status.is_empty() {
        s.status = "active".into();
    }

    conn.execute(
        "INSERT INTO segments
           (id, source_id, label, start_sec, end_sec,
            page_start, page_end, line_start, line_end,
            notes, status, created_at, updated_at,
            parent_segment_id, order_index)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        rusqlite::params![
            s.id,
            s.source_id,
            s.label,
            s.start_sec,
            s.end_sec,
            s.page_start,
            s.page_end,
            s.line_start,
            s.line_end,
            s.notes,
            s.status,
            s.created_at,
            s.updated_at,
            s.parent_segment_id,
            s.order_index,
        ],
    )?;
    let after = serde_json::to_string(&s)?;
    revision::record(
        conn,
        EntityKind::Segment,
        &s.id,
        RevisionType::Create,
        None,
        &after,
        "create segment",
        "user",
    )?;
    Ok(s.id)
}

pub fn get_segment(conn: &Connection, id: &str) -> Result<Segment, Box<dyn std::error::Error>> {
    let sql = format!("{} FROM segments WHERE id = ?1", SEG_SELECT);
    let s = conn.query_row(&sql, [id], map_segment)?;
    Ok(s)
}

pub fn list_segments(
    conn: &Connection,
    source_id: &str,
) -> Result<Vec<Segment>, Box<dyn std::error::Error>> {
    let sql = format!(
        "{} FROM segments WHERE source_id = ?1 ORDER BY order_index ASC, id ASC",
        SEG_SELECT
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([source_id], map_segment)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn list_children(
    conn: &Connection,
    parent_id: &str,
) -> Result<Vec<Segment>, Box<dyn std::error::Error>> {
    let sql = format!(
        "{} FROM segments WHERE parent_segment_id = ?1 ORDER BY order_index ASC, id ASC",
        SEG_SELECT
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([parent_id], map_segment)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

// ── EXTRACTS ────────────────────────────────────────────────────

pub fn create_extract(
    conn: &Connection,
    mut e: Extract,
) -> Result<String, Box<dyn std::error::Error>> {
    e.id = next_id(conn, "EXT")?;
    let now = now_ts();
    e.created_at = now;
    e.updated_at = now;
    if e.status.is_empty() {
        e.status = "active".into();
    }

    conn.execute(
        "INSERT INTO extracts
           (id, segment_id, transform_kind, content, language,
            confidence, created_by, reviewed_by_user,
            parent_extract_id, superseded_by, notes, status,
            created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        rusqlite::params![
            e.id,
            e.segment_id,
            e.transform_kind.as_str(),
            e.content,
            e.language,
            e.confidence.as_str(),
            e.created_by.as_str(),
            if e.reviewed_by_user { 1i64 } else { 0i64 },
            e.parent_extract_id,
            e.superseded_by,
            e.notes,
            e.status,
            e.created_at,
            e.updated_at,
        ],
    )?;
    let after = serde_json::to_string(&e)?;
    let created_by_str = e.created_by.as_str().to_string();
    revision::record(
        conn,
        EntityKind::Extract,
        &e.id,
        RevisionType::Create,
        None,
        &after,
        "create extract",
        &created_by_str,
    )?;
    Ok(e.id)
}

pub fn get_extract(conn: &Connection, id: &str) -> Result<Extract, Box<dyn std::error::Error>> {
    let e = conn.query_row(
        "SELECT id, segment_id, transform_kind, content, language,
                confidence, created_by, reviewed_by_user,
                parent_extract_id, superseded_by, notes, status,
                created_at, updated_at
         FROM extracts WHERE id = ?1",
        [id],
        |r| {
            Ok(Extract {
                id: r.get(0)?,
                segment_id: r.get(1)?,
                transform_kind: TransformKind::from_str(&r.get::<_, String>(2)?)
                    .unwrap_or(TransformKind::Transcribed),
                content: r.get(3)?,
                language: r.get(4)?,
                confidence: Confidence::from_str(&r.get::<_, String>(5)?)
                    .unwrap_or(Confidence::Certain),
                created_by: CreatedBy::from_str(&r.get::<_, String>(6)?).unwrap_or(CreatedBy::User),
                reviewed_by_user: r.get::<_, i64>(7)? != 0,
                parent_extract_id: r.get(8)?,
                superseded_by: r.get(9)?,
                notes: r.get(10)?,
                status: r.get(11)?,
                created_at: r.get(12)?,
                updated_at: r.get(13)?,
            })
        },
    )?;
    Ok(e)
}

pub fn list_extracts(
    conn: &Connection,
    segment_id: &str,
) -> Result<Vec<Extract>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare("SELECT id FROM extracts WHERE segment_id = ?1 ORDER BY id")?;
    let ids: Vec<String> = stmt
        .query_map([segment_id], |r| r.get(0))?
        .filter_map(|x| x.ok())
        .collect();
    let mut out = Vec::new();
    for id in ids {
        out.push(get_extract(conn, &id)?);
    }
    Ok(out)
}

/// Mark an extract as reviewed by the user (appends a Review revision).
pub fn mark_extract_reviewed(
    conn: &Connection,
    id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let before = get_extract(conn, id)?;
    if before.reviewed_by_user {
        return Ok(());
    }
    let now = now_ts();
    conn.execute(
        "UPDATE extracts SET reviewed_by_user = 1, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, id],
    )?;
    let after = get_extract(conn, id)?;
    revision::record(
        conn,
        EntityKind::Extract,
        id,
        RevisionType::Review,
        Some(&serde_json::to_string(&before)?),
        &serde_json::to_string(&after)?,
        "user reviewed auto-generated extract",
        "user",
    )?;
    Ok(())
}

// ── LINKS ───────────────────────────────────────────────────────

pub fn create_link(conn: &Connection, mut l: Link) -> Result<String, Box<dyn std::error::Error>> {
    l.id = next_id(conn, "LNK")?;
    let now = now_ts();
    l.created_at = now;
    l.updated_at = now;
    if l.status.is_empty() {
        l.status = "active".into();
    }

    conn.execute(
        "INSERT INTO links
           (id, extract_id, target_kind, target_id, link_kind,
            note, status, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        rusqlite::params![
            l.id,
            l.extract_id,
            link_target_str(&l.target_kind),
            l.target_id,
            link_kind_str(&l.link_kind),
            l.note,
            l.status,
            l.created_at,
            l.updated_at,
        ],
    )?;
    let after = serde_json::to_string(&l)?;
    revision::record(
        conn,
        EntityKind::Link,
        &l.id,
        RevisionType::Create,
        None,
        &after,
        "create link",
        "user",
    )?;
    Ok(l.id)
}

pub fn list_links_for_extract(
    conn: &Connection,
    extract_id: &str,
) -> Result<Vec<Link>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT id, extract_id, target_kind, target_id, link_kind,
                note, status, created_at, updated_at
         FROM links WHERE extract_id = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map([extract_id], |r| {
        Ok(Link {
            id: r.get(0)?,
            extract_id: r.get(1)?,
            target_kind: parse_link_target(&r.get::<_, String>(2)?),
            target_id: r.get(3)?,
            link_kind: parse_link_kind(&r.get::<_, String>(4)?),
            note: r.get(5)?,
            status: r.get(6)?,
            created_at: r.get(7)?,
            updated_at: r.get(8)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

// ── helpers ──

pub fn link_target_str(k: &LinkTargetKind) -> &'static str {
    match k {
        LinkTargetKind::MindNode => "mind_node",
        LinkTargetKind::ChartAnnotation => "chart_annotation",
        LinkTargetKind::Source => "source",
        LinkTargetKind::External => "external",
    }
}
pub fn parse_link_target(s: &str) -> LinkTargetKind {
    match s {
        "mind_node" => LinkTargetKind::MindNode,
        "chart_annotation" => LinkTargetKind::ChartAnnotation,
        "source" => LinkTargetKind::Source,
        _ => LinkTargetKind::External,
    }
}
pub fn link_kind_str(k: &LinkKind) -> &'static str {
    match k {
        LinkKind::Evidence => "evidence",
        LinkKind::Reference => "reference",
        LinkKind::Inspiration => "inspiration",
        LinkKind::Contradicts => "contradicts",
        LinkKind::Supports => "supports",
    }
}
pub fn parse_link_kind(s: &str) -> LinkKind {
    match s {
        "evidence" => LinkKind::Evidence,
        "reference" => LinkKind::Reference,
        "inspiration" => LinkKind::Inspiration,
        "contradicts" => LinkKind::Contradicts,
        _ => LinkKind::Supports,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    fn make_source(conn: &Connection) -> String {
        create_source(
            conn,
            Source {
                id: String::new(),
                kind: SourceKind::Video,
                path: None,
                title: "x".into(),
                author: None,
                language: None,
                duration_sec: None,
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

    #[test]
    fn id_sequence_starts_at_1() {
        let conn = mem_conn();
        assert_eq!(next_id(&conn, "SRC").unwrap(), "SRC-0001");
        assert_eq!(next_id(&conn, "SEG").unwrap(), "SEG-0001");
        assert_eq!(next_id(&conn, "EXT").unwrap(), "EXT-0001");
    }

    #[test]
    fn id_sequence_increments() {
        let conn = mem_conn();
        let s = Source {
            id: String::new(),
            kind: SourceKind::Video,
            path: None,
            title: "x".into(),
            author: None,
            language: None,
            duration_sec: None,
            page_count: None,
            metadata_json: "{}".into(),
            notes: String::new(),
            status: String::new(),
            created_at: 0,
            updated_at: 0,
        };
        let id1 = create_source(&conn, s.clone()).unwrap();
        let id2 = create_source(&conn, s).unwrap();
        assert_eq!(id1, "SRC-0001");
        assert_eq!(id2, "SRC-0002");
    }

    #[test]
    fn source_roundtrip() {
        let conn = mem_conn();
        let id = create_source(
            &conn,
            Source {
                id: String::new(),
                kind: SourceKind::Video,
                path: Some("D:/video.mp4".into()),
                title: "Fractal Mind Ep 5".into(),
                author: Some("X".into()),
                language: Some("en".into()),
                duration_sec: Some(5025),
                page_count: None,
                metadata_json: "{}".into(),
                notes: String::new(),
                status: String::new(),
                created_at: 0,
                updated_at: 0,
            },
        )
        .unwrap();
        let got = get_source(&conn, &id).unwrap();
        assert_eq!(got.title, "Fractal Mind Ep 5");
        assert_eq!(got.duration_sec, Some(5025));
    }

    #[test]
    fn segment_hierarchy_roundtrip() {
        let conn = mem_conn();
        let src = make_source(&conn);
        let parent = create_segment(
            &conn,
            Segment {
                id: String::new(),
                source_id: src.clone(),
                label: "parent".into(),
                start_sec: Some(0),
                end_sec: Some(3600),
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
        .unwrap();
        let child = create_segment(
            &conn,
            Segment {
                id: String::new(),
                source_id: src.clone(),
                label: "child".into(),
                start_sec: Some(0),
                end_sec: Some(1800),
                page_start: None,
                page_end: None,
                line_start: None,
                line_end: None,
                notes: String::new(),
                status: String::new(),
                created_at: 0,
                updated_at: 0,
                parent_segment_id: Some(parent.clone()),
                order_index: 0,
            },
        )
        .unwrap();

        let got = get_segment(&conn, &child).unwrap();
        assert_eq!(got.parent_segment_id.as_deref(), Some(parent.as_str()));
        assert_eq!(got.order_index, 0);

        let kids = list_children(&conn, &parent).unwrap();
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].id, child);
    }

    #[test]
    fn migrate_idempotent() {
        let conn = mem_conn();
        // Calling twice must not error.
        migrate_segments_tree(&conn).unwrap();
        migrate_segments_tree(&conn).unwrap();
    }
}
