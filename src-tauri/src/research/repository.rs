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

pub fn create_literature_review(
    conn: &Connection,
    req: &CreateLiteratureReviewReq,
    id: &str,
    now: i64,
) -> Result<LiteratureReviewDto> {
    conn.execute(
        "INSERT INTO literature_reviews
            (id, project_id, title, search_scope, lifecycle_status, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, 'draft', ?5, ?6)",
        params![id, req.project_id, req.title.trim(), req.search_scope, req.created_by, now],
    )?;
    query_literature_review(conn, id)
}

pub fn add_literature_source(
    conn: &Connection,
    req: &AddLiteratureSourceReq,
    id: &str,
) -> Result<LiteratureReviewSourceDto> {
    let status: String = conn.query_row(
        "SELECT lifecycle_status FROM literature_reviews WHERE id = ?1",
        [&req.literature_review_id],
        |row| row.get(0),
    )?;
    if status != "draft" {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if let Some(claim_id) = req.source_claim_id.as_deref() {
        let matches_version: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM source_claims
             WHERE id = ?1 AND source_id = ?2 AND source_version_id = ?3)",
            params![claim_id, req.source_id, req.source_version_id],
            |row| row.get(0),
        )?;
        if !matches_version {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }
    conn.execute(
        "INSERT INTO literature_review_sources
            (id, literature_review_id, source_id, source_version_id,
             source_location_id, source_claim_id, citation_note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            req.literature_review_id,
            req.source_id,
            req.source_version_id,
            req.source_location_id,
            req.source_claim_id,
            req.citation_note
        ],
    )?;
    Ok(LiteratureReviewSourceDto {
        id: id.into(),
        source_id: req.source_id.clone(),
        source_version_id: req.source_version_id.clone(),
        source_location_id: req.source_location_id.clone(),
        source_claim_id: req.source_claim_id.clone(),
        citation_note: req.citation_note.clone(),
    })
}

pub fn review_literature_review(
    conn: &Connection,
    review_id: &str,
    reviewer_id: Option<&str>,
    now: i64,
) -> Result<LiteratureReviewDto> {
    let tx = conn.unchecked_transaction()?;
    let changed = tx.execute(
        "UPDATE literature_reviews SET lifecycle_status = 'reviewed',
            reviewer_id = ?1, reviewed_at_us = ?2
         WHERE id = ?3 AND lifecycle_status = 'draft'
           AND EXISTS (
               SELECT 1 FROM literature_review_sources s WHERE s.literature_review_id = ?3
           )",
        params![reviewer_id, now, review_id],
    )?;
    if changed != 1 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    insert_event(
        &tx,
        "literature_review",
        review_id,
        "reviewed",
        reviewer_id,
        "",
        now,
    )?;
    tx.commit()?;
    query_literature_review(conn, review_id)
}

fn query_literature_review(conn: &Connection, id: &str) -> Result<LiteratureReviewDto> {
    let mut review = conn.query_row(
        "SELECT id, project_id, title, search_scope, lifecycle_status,
                reviewer_id, reviewed_at_us, created_by, created_at_us
         FROM literature_reviews WHERE id = ?1",
        [id],
        |row| {
            Ok(LiteratureReviewDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                search_scope: row.get(3)?,
                lifecycle_status: row.get(4)?,
                reviewer_id: row.get(5)?,
                reviewed_at_us: row.get(6)?,
                sources: Vec::new(),
                created_by: row.get(7)?,
                created_at_us: row.get(8)?,
            })
        },
    )?;
    let mut statement = conn.prepare(
        "SELECT id, source_id, source_version_id, source_location_id, source_claim_id, citation_note
         FROM literature_review_sources WHERE literature_review_id = ?1 ORDER BY id",
    )?;
    review.sources = statement
        .query_map([id], |row| {
            Ok(LiteratureReviewSourceDto {
                id: row.get(0)?,
                source_id: row.get(1)?,
                source_version_id: row.get(2)?,
                source_location_id: row.get(3)?,
                source_claim_id: row.get(4)?,
                citation_note: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(review)
}

pub fn create_prior_assessment(
    conn: &Connection,
    req: &CreatePriorKnowledgeAssessmentReq,
    id: &str,
    now: i64,
) -> Result<PriorKnowledgeAssessmentDto> {
    conn.execute(
        "INSERT INTO prior_knowledge_assessments
            (id, project_id, title, assessment, framework_version_id, lifecycle_status,
             created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, 'draft', ?6, ?7)",
        params![
            id,
            req.project_id,
            req.title.trim(),
            req.assessment,
            req.framework_version_id,
            req.created_by,
            now
        ],
    )?;
    query_prior_assessment(conn, id)
}

pub fn review_prior_assessment(
    conn: &Connection,
    id: &str,
    reviewer_id: Option<&str>,
    now: i64,
) -> Result<PriorKnowledgeAssessmentDto> {
    let tx = conn.unchecked_transaction()?;
    let changed = tx.execute(
        "UPDATE prior_knowledge_assessments SET lifecycle_status = 'reviewed',
            reviewer_id = ?1, reviewed_at_us = ?2
         WHERE id = ?3 AND lifecycle_status = 'draft'
           AND length(trim(assessment)) > 0",
        params![reviewer_id, now, id],
    )?;
    if changed != 1 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    insert_event(
        &tx,
        "prior_knowledge_assessment",
        id,
        "reviewed",
        reviewer_id,
        "",
        now,
    )?;
    tx.commit()?;
    query_prior_assessment(conn, id)
}

fn query_prior_assessment(conn: &Connection, id: &str) -> Result<PriorKnowledgeAssessmentDto> {
    conn.query_row(
        "SELECT id, project_id, title, assessment, framework_version_id, lifecycle_status,
                reviewer_id, reviewed_at_us, created_by, created_at_us
         FROM prior_knowledge_assessments WHERE id = ?1",
        [id],
        |row| {
            Ok(PriorKnowledgeAssessmentDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                assessment: row.get(3)?,
                framework_version_id: row.get(4)?,
                lifecycle_status: row.get(5)?,
                reviewer_id: row.get(6)?,
                reviewed_at_us: row.get(7)?,
                created_by: row.get(8)?,
                created_at_us: row.get(9)?,
            })
        },
    )
}

pub fn create_gap(
    conn: &Connection,
    req: &CreateResearchGapReq,
    id: &str,
    now: i64,
) -> Result<ResearchGapDto> {
    conn.execute(
        "INSERT INTO research_gaps
            (id, project_id, domain_id, title, statement, lifecycle_status,
             created_by, created_at_us, updated_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, 'candidate', ?6, ?7, ?7)",
        params![
            id,
            req.project_id,
            req.domain_id,
            req.title.trim(),
            req.statement,
            req.created_by,
            now
        ],
    )?;
    query_gap(conn, id)
}

pub fn add_gap_evidence(conn: &Connection, req: &AddGapEvidenceReq) -> Result<()> {
    let gap_project: String = conn.query_row(
        "SELECT project_id FROM research_gaps WHERE id = ?1",
        [&req.gap_id],
        |row| row.get(0),
    )?;
    if let Some(review_id) = req.literature_review_id.as_deref() {
        let matches_project: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM literature_reviews WHERE id = ?1 AND project_id = ?2)",
            params![review_id, gap_project],
            |row| row.get(0),
        )?;
        if !matches_project {
            return Err(rusqlite::Error::InvalidQuery);
        }
        conn.execute(
            "INSERT INTO research_gap_literature_reviews (gap_id, literature_review_id)
             VALUES (?1, ?2)",
            params![req.gap_id, review_id],
        )?;
    } else if let Some(assessment_id) = req.prior_assessment_id.as_deref() {
        let matches_project: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM prior_knowledge_assessments
             WHERE id = ?1 AND project_id = ?2)",
            params![assessment_id, gap_project],
            |row| row.get(0),
        )?;
        if !matches_project {
            return Err(rusqlite::Error::InvalidQuery);
        }
        conn.execute(
            "INSERT INTO research_gap_prior_assessments (gap_id, assessment_id)
             VALUES (?1, ?2)",
            params![req.gap_id, assessment_id],
        )?;
    }
    Ok(())
}

pub fn review_gap_exception(
    conn: &Connection,
    req: &ReviewGapExceptionReq,
    now: i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO research_gap_review_exceptions
            (gap_id, rationale, reviewer_id, review_decision, reviewed_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(gap_id) DO UPDATE SET
            rationale = excluded.rationale, reviewer_id = excluded.reviewer_id,
            review_decision = excluded.review_decision, reviewed_at_us = excluded.reviewed_at_us",
        params![
            req.gap_id,
            req.rationale.trim(),
            req.reviewer_id,
            req.review_decision,
            now
        ],
    )?;
    Ok(())
}

pub fn transition_gap(
    conn: &Connection,
    gap_id: &str,
    next_status: ResearchGapStatus,
    actor_id: Option<&str>,
    now: i64,
) -> Result<ResearchGapDto> {
    let tx = conn.unchecked_transaction()?;
    let current: String = tx.query_row(
        "SELECT lifecycle_status FROM research_gaps WHERE id = ?1",
        [gap_id],
        |row| row.get(0),
    )?;
    let next = next_status.as_str();
    let allowed = matches!(
        (current.as_str(), next),
        ("candidate", "assessed" | "rejected")
            | ("assessed", "accepted" | "rejected")
            | ("accepted", "addressed" | "superseded")
    );
    if !allowed {
        return Err(rusqlite::Error::InvalidQuery);
    }
    tx.execute(
        "UPDATE research_gaps SET lifecycle_status = ?1, updated_at_us = ?2 WHERE id = ?3",
        params![next, now, gap_id],
    )?;
    insert_event(&tx, "gap", gap_id, next, actor_id, "", now)?;
    tx.commit()?;
    query_gap(conn, gap_id)
}

fn query_gap(conn: &Connection, id: &str) -> Result<ResearchGapDto> {
    let mut gap = conn.query_row(
        "SELECT id, project_id, domain_id, title, statement, lifecycle_status,
                created_by, created_at_us, updated_at_us
         FROM research_gaps WHERE id = ?1",
        [id],
        |row| {
            let status = row.get::<_, String>(5)?;
            Ok(ResearchGapDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                domain_id: row.get(2)?,
                title: row.get(3)?,
                statement: row.get(4)?,
                lifecycle_status: match status.as_str() {
                    "candidate" => ResearchGapStatus::Candidate,
                    "assessed" => ResearchGapStatus::Assessed,
                    "accepted" => ResearchGapStatus::Accepted,
                    "rejected" => ResearchGapStatus::Rejected,
                    "addressed" => ResearchGapStatus::Addressed,
                    _ => ResearchGapStatus::Superseded,
                },
                literature_review_ids: Vec::new(),
                prior_assessment_ids: Vec::new(),
                has_reviewed_exception: false,
                created_by: row.get(6)?,
                created_at_us: row.get(7)?,
                updated_at_us: row.get(8)?,
            })
        },
    )?;
    gap.literature_review_ids = {
        let mut statement = conn.prepare(
            "SELECT literature_review_id FROM research_gap_literature_reviews
             WHERE gap_id = ?1 ORDER BY literature_review_id",
        )?;
        statement
            .query_map([id], |row| row.get(0))?
            .collect::<Result<Vec<_>>>()?
    };
    gap.prior_assessment_ids = {
        let mut statement = conn.prepare(
            "SELECT assessment_id FROM research_gap_prior_assessments
             WHERE gap_id = ?1 ORDER BY assessment_id",
        )?;
        statement
            .query_map([id], |row| row.get(0))?
            .collect::<Result<Vec<_>>>()?
    };
    gap.has_reviewed_exception = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM research_gap_review_exceptions
         WHERE gap_id = ?1 AND review_decision = 'accepted')",
        [id],
        |row| row.get(0),
    )?;
    Ok(gap)
}

pub fn create_question(
    conn: &mut Connection,
    req: &CreateResearchQuestionReq,
    id: &str,
    now: i64,
) -> Result<ResearchQuestionDto> {
    let tx = conn.transaction()?;
    let legacy_ms = now / 1_000;
    tx.execute(
        "INSERT INTO questions (id, title, description, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'parked', ?4, ?4)",
        params![id, req.title.trim(), req.description, legacy_ms],
    )?;
    let ownership = if req.project_id.is_some() {
        "project"
    } else {
        "unassigned"
    };
    tx.execute(
        "INSERT INTO research_questions
            (id, project_id, research_gap_id, domain_id, lifecycle_status,
             ownership_state, migration_note, rationale, created_by, mapped_at_us)
         VALUES (?1, ?2, ?3, ?4, 'draft', ?5, NULL, ?6, ?7, ?8)",
        params![
            id,
            req.project_id,
            req.research_gap_id,
            req.domain_id,
            ownership,
            req.rationale,
            req.created_by,
            now
        ],
    )?;
    insert_event(
        &tx,
        "question",
        id,
        "draft",
        req.created_by.as_deref(),
        &req.rationale,
        now,
    )?;
    tx.commit()?;
    query_question(conn, id)
}

pub fn transition_question(
    conn: &Connection,
    question_id: &str,
    next_status: ResearchQuestionStatus,
    actor_id: Option<&str>,
    rationale: &str,
    now: i64,
) -> Result<ResearchQuestionDto> {
    let tx = conn.unchecked_transaction()?;
    let current: String = tx.query_row(
        "SELECT lifecycle_status FROM research_questions WHERE id = ?1",
        [question_id],
        |row| row.get(0),
    )?;
    let next = next_status.as_str();
    let allowed = matches!(
        (current.as_str(), next),
        ("draft", "review" | "rejected" | "deferred")
            | ("review", "accepted" | "rejected" | "deferred" | "draft")
            | ("accepted", "deferred" | "refined" | "superseded")
            | ("deferred", "draft" | "review" | "accepted" | "rejected")
    );
    if !allowed {
        return Err(rusqlite::Error::InvalidQuery);
    }
    tx.execute(
        "UPDATE research_questions SET lifecycle_status = ?1, rationale = ?2 WHERE id = ?3",
        params![next, rationale, question_id],
    )?;
    insert_event(&tx, "question", question_id, next, actor_id, rationale, now)?;
    tx.commit()?;
    query_question(conn, question_id)
}

pub fn refine_question(
    conn: &mut Connection,
    parent_question_id: &str,
    title: &str,
    description: &str,
    rationale: &str,
    actor_id: Option<&str>,
    derived_id: &str,
    now: i64,
) -> Result<ResearchQuestionDto> {
    let tx = conn.transaction()?;
    let parent = tx.query_row(
        "SELECT q.project_id, q.research_gap_id, q.domain_id, q.ownership_state
         FROM research_questions q WHERE q.id = ?1 AND q.lifecycle_status = 'accepted'",
        [parent_question_id],
        |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    )?;
    let legacy_ms = now / 1_000;
    tx.execute(
        "INSERT INTO questions (id, title, description, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'parked', ?4, ?4)",
        params![derived_id, title.trim(), description, legacy_ms],
    )?;
    tx.execute(
        "INSERT INTO research_questions
            (id, project_id, research_gap_id, domain_id, lifecycle_status,
             ownership_state, migration_note, rationale, created_by, mapped_at_us)
         VALUES (?1, ?2, ?3, ?4, 'draft', ?5, NULL, ?6, ?7, ?8)",
        params![
            derived_id,
            parent.0,
            parent.1,
            parent.2,
            parent.3,
            rationale,
            actor_id,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO research_question_lineage
            (id, parent_question_id, derived_question_id, relation_kind, rationale, created_by, created_at_us)
         VALUES (?1, ?2, ?3, 'refined_from', ?4, ?5, ?6)",
        params![
            Uuid::now_v7().to_string(),
            parent_question_id,
            derived_id,
            rationale,
            actor_id,
            now
        ],
    )?;
    tx.execute(
        "UPDATE research_questions SET lifecycle_status = 'refined', rationale = ?1
         WHERE id = ?2",
        params![rationale, parent_question_id],
    )?;
    insert_event(
        &tx,
        "question",
        parent_question_id,
        "refined",
        actor_id,
        rationale,
        now,
    )?;
    insert_event(&tx, "question", derived_id, "draft", actor_id, rationale, now)?;
    tx.commit()?;
    query_question(conn, derived_id)
}

pub fn link_question_lineage(
    conn: &Connection,
    parent_question_id: &str,
    derived_question_id: &str,
    relation_kind: ResearchQuestionRelation,
    rationale: &str,
    actor_id: Option<&str>,
    id: &str,
    now: i64,
) -> Result<ResearchQuestionLineageDto> {
    let tx = conn.unchecked_transaction()?;
    let same_project = tx.query_row(
        "SELECT (parent.project_id IS derived.project_id)
         FROM research_questions parent, research_questions derived
         WHERE parent.id = ?1 AND derived.id = ?2",
        params![parent_question_id, derived_question_id],
        |row| row.get::<_, bool>(0),
    )?;
    if !same_project || parent_question_id == derived_question_id {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let creates_cycle: bool = tx.query_row(
        "WITH RECURSIVE descendants(id) AS (
            SELECT derived_question_id FROM research_question_lineage WHERE parent_question_id = ?1
            UNION
            SELECT l.derived_question_id FROM research_question_lineage l
            JOIN descendants d ON l.parent_question_id = d.id
         )
         SELECT EXISTS(SELECT 1 FROM descendants WHERE id = ?2)",
        params![derived_question_id, parent_question_id],
        |row| row.get(0),
    )?;
    if creates_cycle {
        return Err(rusqlite::Error::InvalidQuery);
    }
    tx.execute(
        "INSERT INTO research_question_lineage
            (id, parent_question_id, derived_question_id, relation_kind, rationale, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            parent_question_id,
            derived_question_id,
            relation_kind.as_str(),
            rationale,
            actor_id,
            now
        ],
    )?;
    insert_event(
        &tx,
        "question",
        derived_question_id,
        relation_kind.as_str(),
        actor_id,
        rationale,
        now,
    )?;
    tx.commit()?;
    Ok(ResearchQuestionLineageDto {
        id: id.into(),
        parent_question_id: parent_question_id.into(),
        derived_question_id: derived_question_id.into(),
        relation_kind,
        rationale: rationale.into(),
        created_by: actor_id.map(str::to_string),
        created_at_us: now,
    })
}

fn query_question(conn: &Connection, id: &str) -> Result<ResearchQuestionDto> {
    conn.query_row(
        "SELECT r.id, q.title, q.description, r.project_id, r.research_gap_id, r.domain_id,
                r.lifecycle_status, r.ownership_state, r.migration_note, r.rationale,
                q.status, q.created_at, q.updated_at
         FROM research_questions r JOIN questions q ON q.id = r.id WHERE r.id = ?1",
        [id],
        |row| {
            let status = row.get::<_, String>(6)?;
            Ok(ResearchQuestionDto {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                project_id: row.get(3)?,
                research_gap_id: row.get(4)?,
                domain_id: row.get(5)?,
                lifecycle_status: match status.as_str() {
                    "draft" => ResearchQuestionStatus::Draft,
                    "review" => ResearchQuestionStatus::Review,
                    "accepted" => ResearchQuestionStatus::Accepted,
                    "rejected" => ResearchQuestionStatus::Rejected,
                    "deferred" => ResearchQuestionStatus::Deferred,
                    "refined" => ResearchQuestionStatus::Refined,
                    "superseded" => ResearchQuestionStatus::Superseded,
                    _ => ResearchQuestionStatus::LegacyUnassessed,
                },
                ownership_state: row.get(7)?,
                migration_note: row.get(8)?,
                rationale: row.get(9)?,
                legacy_status: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        },
    )
}

pub fn create_phenomenon(
    conn: &Connection,
    req: &CreatePhenomenonReq,
    id: &str,
    now: i64,
) -> Result<PhenomenonDto> {
    conn.execute(
        "INSERT INTO research_phenomena
            (id, project_id, domain_id, title, description, population_scope,
             context, temporal_boundary, lifecycle_status, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'identified', ?9, ?10)",
        params![
            id,
            req.project_id,
            req.domain_id,
            req.title.trim(),
            req.description,
            req.population_scope,
            req.context,
            req.temporal_boundary,
            req.created_by,
            now
        ],
    )?;
    query_phenomenon(conn, id)
}

fn query_phenomenon(conn: &Connection, id: &str) -> Result<PhenomenonDto> {
    conn.query_row(
        "SELECT id, project_id, domain_id, title, description, population_scope,
                context, temporal_boundary, lifecycle_status, created_by, created_at_us
         FROM research_phenomena WHERE id = ?1",
        [id],
        |row| {
            Ok(PhenomenonDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                domain_id: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                population_scope: row.get(5)?,
                context: row.get(6)?,
                temporal_boundary: row.get(7)?,
                lifecycle_status: row.get(8)?,
                created_by: row.get(9)?,
                created_at_us: row.get(10)?,
            })
        },
    )
}

