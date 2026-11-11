use super::*;
use rusqlite::{params, Connection, OptionalExtension, Result, Transaction};
use sha2::{Digest, Sha256};

const RESEARCH_SCHEMA_VERSION: i64 = 1;
const RESEARCH_SCHEMA_SQL: &str = include_str!("schema.sql");

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    let checksum = format!("{:x}", Sha256::digest(RESEARCH_SCHEMA_SQL.as_bytes()));
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_schema_migrations (
            version INTEGER PRIMARY KEY,
            migration_id TEXT NOT NULL UNIQUE,
            checksum TEXT NOT NULL,
            applied_at_us INTEGER NOT NULL
        );",
    )?;
    let tx = conn.unchecked_transaction()?;
    let existing = tx
        .query_row(
            "SELECT checksum FROM research_schema_migrations WHERE version = ?1",
            [RESEARCH_SCHEMA_VERSION],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    match existing {
        Some(applied_checksum) if applied_checksum != checksum => {
            return Err(rusqlite::Error::InvalidQuery);
        }
        Some(_) => {}
        None => {
            tx.execute_batch(RESEARCH_SCHEMA_SQL)?;
            tx.execute_batch(
                "INSERT INTO research_pathways (id, pathway_key, title, description) VALUES
                    ('pathway_existing_knowledge_acquisition', 'existing_knowledge_acquisition',
                     'Existing Knowledge Acquisition', 'Acquire and reconstruct existing knowledge.'),
                    ('pathway_existing_knowledge_investigation', 'existing_knowledge_investigation',
                     'Existing Knowledge Investigation', 'Investigate and develop existing knowledge.'),
                    ('pathway_first_principles_discovery', 'first_principles_discovery',
                     'First-Principles Discovery', 'Investigate phenomena without assuming an adequate framework.');",
            )?;
            tx.execute(
                "INSERT INTO research_schema_migrations (version, migration_id, checksum, applied_at_us)
                 VALUES (?1, 'research-domain-v1', ?2, ?3)",
                params![RESEARCH_SCHEMA_VERSION, checksum, now_ts_us()?],
            )?;
        }
    }
    tx.commit()
}

pub fn synchronize_legacy_questions(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO research_questions (
            id, project_id, research_gap_id, domain_id, lifecycle_status,
            ownership_state, migration_note, rationale, created_by, mapped_at_us
         )
         SELECT q.id, NULL, NULL, NULL, 'legacy_unassessed', 'unassigned',
                'Legacy question state and project ownership have not been semantically reviewed.',
                '', NULL, ?1
         FROM questions q",
        [now_ts_us().map_err(|_| rusqlite::Error::InvalidQuery)?],
    )?;
    Ok(())
}

pub fn synchronize_legacy_hypotheses(conn: &Connection, now: i64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO research_hypotheses (
            id, project_id, research_question_id, legacy_hypothesis_id,
            parent_hypothesis_id, ownership_state, lifecycle_status, created_by, created_at_us
         )
         SELECT h.id, NULL,
                CASE WHEN EXISTS (
                    SELECT 1 FROM research_questions q WHERE q.id = h.inquiry_id
                ) THEN h.inquiry_id ELSE NULL END,
                h.id,
                CASE WHEN EXISTS (
                    SELECT 1 FROM hypotheses parent WHERE parent.id = h.parent_hypothesis_id
                ) THEN h.parent_hypothesis_id
                WHEN EXISTS (
                    SELECT 1 FROM hypotheses parent WHERE parent.id = h.refined_from_id
                ) THEN h.refined_from_id ELSE NULL END,
                'unassigned', 'legacy_untyped', NULL, ?1
         FROM hypotheses h",
        [now],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO research_hypothesis_lineage (
            id, parent_hypothesis_id, derived_hypothesis_id, relation_kind,
            rationale, created_by, created_at_us
         )
         SELECT 'legacy-parent-' || h.id, h.parent_hypothesis_id, h.id,
                'derived_from', 'Preserved legacy parent_hypothesis_id without inferring typed definitions.',
                NULL, ?1
         FROM hypotheses h
         JOIN research_hypotheses child ON child.id = h.id
         JOIN research_hypotheses parent ON parent.id = h.parent_hypothesis_id
         WHERE h.parent_hypothesis_id IS NOT NULL AND h.parent_hypothesis_id <> h.id",
        [now],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO research_hypothesis_lineage (
            id, parent_hypothesis_id, derived_hypothesis_id, relation_kind,
            rationale, created_by, created_at_us
         )
         SELECT 'legacy-refined-' || h.id, h.refined_from_id, h.id,
                'refined_from', 'Preserved legacy refined_from_id without inferring typed definitions.',
                NULL, ?1
         FROM hypotheses h
         JOIN research_hypotheses child ON child.id = h.id
         JOIN research_hypotheses parent ON parent.id = h.refined_from_id
         WHERE h.refined_from_id IS NOT NULL AND h.refined_from_id <> h.id",
        [now],
    )?;
    Ok(())
}

fn insert_event(
    tx: &Transaction<'_>,
    target_kind: &str,
    target_id: &str,
    event_kind: &str,
    actor_id: Option<&str>,
    rationale: &str,
    now: i64,
) -> Result<()> {
    tx.execute(
        "INSERT INTO research_provenance_events
            (id, target_kind, target_id, event_kind, actor_id, rationale, occurred_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            Uuid::now_v7().to_string(),
            target_kind,
            target_id,
            event_kind,
            actor_id,
            rationale,
            now
        ],
    )?;
    Ok(())
}

fn query_project(conn: &Connection, id: &str) -> Result<ResearchProjectDto> {
    let mut project = conn.query_row(
        "SELECT id, title, aim, lifecycle_status, created_by, created_at_us, updated_at_us
         FROM research_projects WHERE id = ?1",
        [id],
        |row| {
            Ok(ResearchProjectDto {
                id: row.get(0)?,
                title: row.get(1)?,
                aim: row.get(2)?,
                lifecycle_status: match row.get::<_, String>(3)?.as_str() {
                    "proposed" => ResearchProjectStatus::Proposed,
                    "active" => ResearchProjectStatus::Active,
                    "paused" => ResearchProjectStatus::Paused,
                    "completed" => ResearchProjectStatus::Completed,
                    _ => ResearchProjectStatus::Archived,
                },
                pathway_keys: Vec::new(),
                domain_ids: Vec::new(),
                created_by: row.get(4)?,
                created_at_us: row.get(5)?,
                updated_at_us: row.get(6)?,
            })
        },
    )?;
    project.pathway_keys = list_project_pathways(conn, id)?;
    project.domain_ids = list_project_domain_ids(conn, id)?;
    Ok(project)
}

fn list_project_pathways(conn: &Connection, project_id: &str) -> Result<Vec<ResearchPathwayKey>> {
    let mut statement = conn.prepare(
        "SELECT p.pathway_key FROM research_pathways p
         JOIN research_project_pathways pp ON pp.pathway_id = p.id
         WHERE pp.project_id = ?1 ORDER BY p.pathway_key",
    )?;
    let rows = statement.query_map([project_id], |row| {
        let value: String = row.get(0)?;
        ResearchPathwayKey::parse(&value).map_err(|_| rusqlite::Error::InvalidQuery)
    })?;
    rows.collect()
}

fn list_project_domain_ids(conn: &Connection, project_id: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT domain_id FROM research_project_domains WHERE project_id = ?1 ORDER BY domain_id",
    )?;
    let rows = statement.query_map([project_id], |row| row.get(0))?;
    rows.collect()
}

pub fn create_project(
    conn: &mut Connection,
    req: &CreateResearchProjectReq,
    id: &str,
    now: i64,
) -> Result<ResearchProjectDto> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO research_projects
            (id, title, aim, lifecycle_status, created_by, created_at_us, updated_at_us)
         VALUES (?1, ?2, ?3, 'proposed', ?4, ?5, ?5)",
        params![id, req.title.trim(), req.aim, req.created_by, now],
    )?;
    for pathway in &req.pathways {
        tx.execute(
            "INSERT INTO research_project_pathways (project_id, pathway_id, selected_at_us)
             SELECT ?1, id, ?3 FROM research_pathways WHERE pathway_key = ?2",
            params![id, pathway.as_str(), now],
        )?;
    }
    insert_event(&tx, "project", id, "created", req.created_by.as_deref(), "", now)?;
    tx.commit()?;
    query_project(conn, id)
}

pub fn create_domain(
    conn: &Connection,
    req: &CreateResearchDomainReq,
    id: &str,
    now: i64,
) -> Result<ResearchDomainDto> {
    conn.execute(
        "INSERT INTO research_domains
            (id, name, description, parent_domain_id, lifecycle_status, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?6)",
        params![
            id,
            req.name.trim(),
            req.description,
            req.parent_domain_id,
            req.created_by,
            now
        ],
    )?;
    query_domain(conn, id)
}

pub fn domain_is_descendant(
    conn: &Connection,
    domain_id: &str,
    candidate_parent_id: &str,
) -> Result<bool> {
    conn.query_row(
        "WITH RECURSIVE ancestors(id, parent_domain_id) AS (
            SELECT id, parent_domain_id FROM research_domains WHERE id = ?1
            UNION ALL
            SELECT d.id, d.parent_domain_id
            FROM research_domains d JOIN ancestors a ON d.id = a.parent_domain_id
         )
         SELECT EXISTS(SELECT 1 FROM ancestors WHERE id = ?2)",
        params![domain_id, candidate_parent_id],
        |row| row.get(0),
    )
}

pub fn validate_domain_parent(conn: &Connection, parent_id: &str) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM research_domains WHERE id = ?1)",
        [parent_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    let has_cycle = conn.query_row(
        "WITH RECURSIVE ancestors(id, parent_domain_id, path, cycle) AS (
            SELECT id, parent_domain_id, ',' || id || ',', 0
            FROM research_domains WHERE id = ?1
            UNION ALL
            SELECT d.id, d.parent_domain_id, a.path || d.id || ',',
                   instr(a.path, ',' || d.id || ',') > 0
            FROM research_domains d JOIN ancestors a ON d.id = a.parent_domain_id
            WHERE a.cycle = 0
         )
         SELECT EXISTS(SELECT 1 FROM ancestors WHERE cycle = 1)",
        [parent_id],
        |row| row.get::<_, bool>(0),
    )?;
    if has_cycle {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

pub fn assign_domain(
    conn: &Connection,
    project_id: &str,
    domain_id: &str,
    rationale: &str,
    now: i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO research_project_domains (project_id, domain_id, rationale, assigned_at_us)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(project_id, domain_id) DO UPDATE SET
            rationale = excluded.rationale, assigned_at_us = excluded.assigned_at_us",
        params![project_id, domain_id, rationale, now],
    )?;
    Ok(())
}

pub fn adopt_framework_version(
    conn: &Connection,
    id: &str,
    project_id: &str,
    framework_id: &str,
    framework_version_id: &str,
    use_context: &str,
    actor_id: Option<&str>,
    now: i64,
) -> Result<ProjectKnowledgeAdoptionDto> {
    let version = conn.query_row(
        "SELECT v.title, v.version_no FROM knowledge_framework_versions v
         WHERE v.id = ?1 AND v.framework_id = ?2 AND v.status = 'published'",
        params![framework_version_id, framework_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    )?;
    conn.execute(
        "INSERT INTO research_project_knowledge_adoptions
            (id, project_id, framework_id, framework_version_id, use_context, adopted_by, adopted_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, project_id, framework_id, framework_version_id, use_context, actor_id, now],
    )?;
    Ok(ProjectKnowledgeAdoptionDto {
        id: id.into(),
        project_id: project_id.into(),
        framework_id: framework_id.into(),
        framework_version_id: framework_version_id.into(),
        title: version.0,
        version_no: version.1,
        use_context: use_context.into(),
        adopted_by: actor_id.map(str::to_string),
        adopted_at_us: now,
    })
}

fn query_domain(conn: &Connection, id: &str) -> Result<ResearchDomainDto> {
    conn.query_row(
        "SELECT id, name, description, parent_domain_id, lifecycle_status, created_by, created_at_us
         FROM research_domains WHERE id = ?1",
        [id],
        |row| {
            Ok(ResearchDomainDto {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                parent_domain_id: row.get(3)?,
                lifecycle_status: row.get(4)?,
                created_by: row.get(5)?,
                created_at_us: row.get(6)?,
            })
        },
    )
}

