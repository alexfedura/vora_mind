use rusqlite::Connection;

use super::db::next_id;
use super::model::{now_ts, EntityKind, Revision, RevisionType};

/// Append a revision. This is the ONLY way mutations are recorded.
/// The DB triggers prevent UPDATE/DELETE on the table.
pub fn record(
    conn: &Connection,
    entity_kind: EntityKind,
    entity_id: &str,
    revision_type: RevisionType,
    before_json: Option<&str>,
    after_json: &str,
    reason: &str,
    created_by: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let id = next_id(conn, "REV")?;
    let now = now_ts();
    conn.execute(
        "INSERT INTO source_revisions
           (id, entity_kind, entity_id, revision_type,
            before_json, after_json, reason, created_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            id,
            kind_str(&entity_kind),
            entity_id,
            rev_str(&revision_type),
            before_json,
            after_json,
            reason,
            created_by,
            now,
        ],
    )?;
    Ok(id)
}

pub fn list_for_entity(
    conn: &Connection,
    entity_kind: EntityKind,
    entity_id: &str,
) -> Result<Vec<Revision>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT id, entity_kind, entity_id, revision_type,
                before_json, after_json, reason, created_by, created_at
         FROM source_revisions
         WHERE entity_kind = ?1 AND entity_id = ?2
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = stmt.query_map(rusqlite::params![kind_str(&entity_kind), entity_id], |r| {
        Ok(Revision {
            id: r.get(0)?,
            entity_kind: parse_kind(&r.get::<_, String>(1)?),
            entity_id: r.get(2)?,
            revision_type: parse_rev_type(&r.get::<_, String>(3)?),
            before_json: r.get(4)?,
            after_json: r.get(5)?,
            reason: r.get(6)?,
            created_by: r.get(7)?,
            created_at: r.get(8)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

// ── string <-> enum helpers (private) ──

fn kind_str(k: &EntityKind) -> &'static str {
    match k {
        EntityKind::Source => "source",
        EntityKind::Segment => "segment",
        EntityKind::Extract => "extract",
        EntityKind::Link => "link",
    }
}
fn parse_kind(s: &str) -> EntityKind {
    match s {
        "source" => EntityKind::Source,
        "segment" => EntityKind::Segment,
        "extract" => EntityKind::Extract,
        _ => EntityKind::Link,
    }
}
fn rev_str(r: &RevisionType) -> &'static str {
    match r {
        RevisionType::Create => "create",
        RevisionType::Update => "update",
        RevisionType::Correct => "correct",
        RevisionType::Retract => "retract",
        RevisionType::Review => "review",
    }
}
fn parse_rev_type(s: &str) -> RevisionType {
    match s {
        "create" => RevisionType::Create,
        "update" => RevisionType::Update,
        "correct" => RevisionType::Correct,
        "retract" => RevisionType::Retract,
        _ => RevisionType::Review,
    }
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

    #[test]
    fn revision_record_and_list() {
        let conn = mem_conn();
        let id = record(
            &conn,
            EntityKind::Source,
            "SRC-0001",
            RevisionType::Create,
            None,
            r#"{"title":"x"}"#,
            "initial",
            "user",
        )
        .unwrap();
        assert!(id.starts_with("REV-"));

        let list = list_for_entity(&conn, EntityKind::Source, "SRC-0001").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].revision_type, RevisionType::Create);
    }

    #[test]
    fn revisions_cannot_be_deleted() {
        let conn = mem_conn();
        record(
            &conn,
            EntityKind::Source,
            "SRC-0001",
            RevisionType::Create,
            None,
            "{}",
            "",
            "user",
        )
        .unwrap();
        let res = conn.execute("DELETE FROM source_revisions", []);
        assert!(res.is_err());
    }

    #[test]
    fn revisions_cannot_be_updated() {
        let conn = mem_conn();
        record(
            &conn,
            EntityKind::Source,
            "SRC-0001",
            RevisionType::Create,
            None,
            "{}",
            "",
            "user",
        )
        .unwrap();
        let res = conn.execute("UPDATE source_revisions SET reason = 'x'", []);
        assert!(res.is_err());
    }
}
