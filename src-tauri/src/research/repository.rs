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
                params![
                    RESEARCH_SCHEMA_VERSION,
                    checksum,
                    now_ts_us().map_err(|_| rusqlite::Error::InvalidQuery)?
                ],
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
    insert_event(
        &tx,
        "project",
        id,
        "created",
        req.created_by.as_deref(),
        "",
        now,
    )?;
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

fn validate_project_domain(
    conn: &Connection,
    project_id: &str,
    domain_id: Option<&str>,
) -> Result<()> {
    let Some(domain_id) = domain_id else {
        return Ok(());
    };
    let assigned: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM research_project_domains
         WHERE project_id = ?1 AND domain_id = ?2)",
        params![project_id, domain_id],
        |row| row.get(0),
    )?;
    if !assigned {
        return Err(rusqlite::Error::InvalidQuery);
    }
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
                lifecycle_status: parse_domain_status(&row.get::<_, String>(4)?)?,
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
        params![
            id,
            req.project_id,
            req.title.trim(),
            req.search_scope,
            req.created_by,
            now
        ],
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
                lifecycle_status: parse_review_status(&row.get::<_, String>(4)?)?,
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
                lifecycle_status: parse_review_status(&row.get::<_, String>(5)?)?,
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
    if let Some(domain_id) = req.domain_id.as_deref() {
        validate_project_domain(conn, &req.project_id, Some(domain_id))?;
    }
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
    id: &str,
    now: i64,
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO research_gap_review_exceptions
            (id, gap_id, rationale, reviewer_id, review_decision, reviewed_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            req.gap_id,
            req.rationale.trim(),
            req.reviewer_id,
            req.review_decision.as_str(),
            now
        ],
    )?;
    insert_event(
        &tx,
        "gap",
        &req.gap_id,
        if req.review_decision == GapReviewDecision::Accepted {
            "exception_accepted"
        } else {
            "exception_rejected"
        },
        req.reviewer_id.as_deref(),
        &req.rationale,
        now,
    )?;
    tx.commit()
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
        let rows = statement
            .query_map([id], |row| row.get(0))?
            .collect::<Result<Vec<_>>>()?;
        rows
    };
    gap.prior_assessment_ids = {
        let mut statement = conn.prepare(
            "SELECT assessment_id FROM research_gap_prior_assessments
             WHERE gap_id = ?1 ORDER BY assessment_id",
        )?;
        let rows = statement
            .query_map([id], |row| row.get(0))?
            .collect::<Result<Vec<_>>>()?;
        rows
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
    let mut project_id = req.project_id.clone();
    let mut domain_id = req.domain_id.clone();
    if let Some(gap_id) = req.research_gap_id.as_deref() {
        let gap = tx.query_row(
            "SELECT project_id, domain_id, lifecycle_status
             FROM research_gaps WHERE id = ?1",
            [gap_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        if gap.2 != "accepted" || project_id.as_deref().is_some_and(|id| id != gap.0) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        project_id = Some(gap.0);
        if domain_id.is_none() {
            domain_id = gap.1;
        }
    }
    if let Some(project_id) = project_id.as_deref() {
        if let Some(domain_id) = domain_id.as_deref() {
            validate_project_domain(&tx, project_id, Some(domain_id))?;
        }
    } else if req.research_gap_id.is_some() {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let legacy_ms = now / 1_000;
    tx.execute(
        "INSERT INTO questions (id, title, description, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'parked', ?4, ?4)",
        params![id, req.title.trim(), req.description, legacy_ms],
    )?;
    let ownership = if project_id.is_some() {
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
            project_id,
            req.research_gap_id,
            domain_id,
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
        params![derived_id, parent.0, parent.1, parent.2, parent.3, rationale, actor_id, now],
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
    insert_event(
        &tx, "question", derived_id, "draft", actor_id, rationale, now,
    )?;
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
    if let Some(domain_id) = req.domain_id.as_deref() {
        validate_project_domain(conn, &req.project_id, Some(domain_id))?;
    }
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
                lifecycle_status: parse_phenomenon_status(&row.get::<_, String>(8)?)?,
                created_by: row.get(9)?,
                created_at_us: row.get(10)?,
            })
        },
    )
}

pub fn create_observation_plan(
    conn: &mut Connection,
    req: &CreateObservationPlanReq,
    plan_id: &str,
    version_id: &str,
    now: i64,
) -> Result<ObservationPlanDto> {
    let tx = conn.transaction()?;
    let phenomenon_matches: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM research_phenomena
         WHERE id = ?1 AND project_id = ?2)",
        params![req.phenomenon_id, req.project_id],
        |row| row.get(0),
    )?;
    if !phenomenon_matches {
        return Err(rusqlite::Error::InvalidQuery);
    }
    tx.execute(
        "INSERT INTO observation_plans
            (id, project_id, phenomenon_id, title, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            plan_id,
            req.project_id,
            req.phenomenon_id,
            req.title.trim(),
            req.created_by,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO observation_plan_versions
            (id, plan_id, version_no, measurement_method, sample_frame,
             conditions_template, inclusion_rules, lifecycle_status, created_by, created_at_us)
         VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, 'draft', ?7, ?8)",
        params![
            version_id,
            plan_id,
            req.measurement_method,
            req.sample_frame,
            req.conditions_template,
            req.inclusion_rules,
            req.created_by,
            now
        ],
    )?;
    insert_event(
        &tx,
        "observation_plan",
        plan_id,
        "draft_version_created",
        req.created_by.as_deref(),
        "",
        now,
    )?;
    tx.commit()?;
    query_observation_plan(conn, plan_id)
}

pub fn review_observation_plan(
    conn: &Connection,
    version_id: &str,
    preregister: bool,
    actor_id: Option<&str>,
    now: i64,
) -> Result<ObservationPlanVersionDto> {
    let next_status = if preregister {
        "preregistered"
    } else {
        "reviewed"
    };
    let tx = conn.unchecked_transaction()?;
    let changed = tx.execute(
        "UPDATE observation_plan_versions SET lifecycle_status = ?1,
            preregistered_at_us = ?2
         WHERE id = ?3 AND lifecycle_status = 'draft'",
        params![
            next_status,
            if preregister { Some(now) } else { None },
            version_id
        ],
    )?;
    if changed != 1 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let plan_id: String = tx.query_row(
        "SELECT plan_id FROM observation_plan_versions WHERE id = ?1",
        [version_id],
        |row| row.get(0),
    )?;
    insert_event(
        &tx,
        "observation_plan",
        &plan_id,
        next_status,
        actor_id,
        "",
        now,
    )?;
    tx.commit()?;
    query_observation_plan_version(conn, version_id)
}

fn query_observation_plan(conn: &Connection, plan_id: &str) -> Result<ObservationPlanDto> {
    let mut plan = conn.query_row(
        "SELECT id, project_id, phenomenon_id, title, created_by, created_at_us
         FROM observation_plans WHERE id = ?1",
        [plan_id],
        |row| {
            Ok(ObservationPlanDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                phenomenon_id: row.get(2)?,
                title: row.get(3)?,
                created_by: row.get(4)?,
                created_at_us: row.get(5)?,
                versions: Vec::new(),
            })
        },
    )?;
    let mut statement = conn.prepare(
        "SELECT id FROM observation_plan_versions WHERE plan_id = ?1 ORDER BY version_no",
    )?;
    let version_ids = statement
        .query_map([plan_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>>>()?;
    for version_id in version_ids {
        plan.versions
            .push(query_observation_plan_version(conn, &version_id)?);
    }
    Ok(plan)
}

fn query_observation_plan_version(
    conn: &Connection,
    version_id: &str,
) -> Result<ObservationPlanVersionDto> {
    conn.query_row(
        "SELECT id, plan_id, version_no, measurement_method, sample_frame,
                conditions_template, inclusion_rules, lifecycle_status, created_by,
                created_at_us, preregistered_at_us
         FROM observation_plan_versions WHERE id = ?1",
        [version_id],
        |row| {
            Ok(ObservationPlanVersionDto {
                id: row.get(0)?,
                plan_id: row.get(1)?,
                version_no: row.get(2)?,
                measurement_method: row.get(3)?,
                sample_frame: row.get(4)?,
                conditions_template: row.get(5)?,
                inclusion_rules: row.get(6)?,
                lifecycle_status: parse_plan_status(&row.get::<_, String>(7)?)?,
                created_by: row.get(8)?,
                created_at_us: row.get(9)?,
                preregistered_at_us: row.get(10)?,
            })
        },
    )
}

pub fn create_dataset_version_reference(
    conn: &Connection,
    req: &CreateDatasetVersionReferenceReq,
    id: &str,
    now: i64,
) -> Result<DatasetVersionReferenceDto> {
    conn.execute(
        "INSERT INTO research_dataset_version_references
            (id, project_id, title, version_label, external_uri, content_sha256,
             source_version_id, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            req.project_id,
            req.title.trim(),
            req.version_label.trim(),
            req.external_uri,
            req.content_sha256,
            req.source_version_id,
            req.created_by,
            now
        ],
    )?;
    query_dataset_version(conn, id)
}

fn query_dataset_version(conn: &Connection, id: &str) -> Result<DatasetVersionReferenceDto> {
    conn.query_row(
        "SELECT id, project_id, title, version_label, external_uri, content_sha256,
                source_version_id, created_by, created_at_us
         FROM research_dataset_version_references WHERE id = ?1",
        [id],
        |row| {
            Ok(DatasetVersionReferenceDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                version_label: row.get(3)?,
                external_uri: row.get(4)?,
                content_sha256: row.get(5)?,
                source_version_id: row.get(6)?,
                created_by: row.get(7)?,
                created_at_us: row.get(8)?,
            })
        },
    )
}

pub fn create_observation(
    conn: &Connection,
    req: &CreateObservationReq,
    id: &str,
    now: i64,
) -> Result<ObservationDto> {
    let plan_matches: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM observation_plans p
            JOIN research_phenomena f ON f.id = p.phenomenon_id
            JOIN observation_plan_versions v ON v.plan_id = p.id AND v.id = ?3
            WHERE p.id = ?1 AND p.project_id = ?2 AND f.id = ?4
              AND f.project_id = ?2 AND v.lifecycle_status IN ('reviewed', 'preregistered')
         )",
        params![
            req.plan_id,
            req.project_id,
            req.plan_version_id,
            req.phenomenon_id
        ],
        |row| row.get(0),
    )?;
    if !plan_matches {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if let Some(dataset_ref) = req.dataset_version_reference_id.as_deref() {
        let matches_project: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM research_dataset_version_references
             WHERE id = ?1 AND project_id = ?2)",
            params![dataset_ref, req.project_id],
            |row| row.get(0),
        )?;
        if !matches_project {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }
    conn.execute(
        "INSERT INTO research_observations
            (id, project_id, phenomenon_id, plan_id, plan_version_id,
             dataset_version_reference_id, measured_at_us, value_decimal, value_state,
             unit, measurement_method, conditions, sample_identifier, quality_status,
             created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'unreviewed', ?14, ?15)",
        params![
            id,
            req.project_id,
            req.phenomenon_id,
            req.plan_id,
            req.plan_version_id,
            req.dataset_version_reference_id,
            req.measured_at_us,
            req.value_decimal,
            req.value_state.as_str(),
            req.unit,
            req.measurement_method,
            req.conditions,
            req.sample_identifier,
            req.created_by,
            now
        ],
    )?;
    query_observation(conn, id)
}

fn query_observation(conn: &Connection, id: &str) -> Result<ObservationDto> {
    conn.query_row(
        "SELECT id, project_id, phenomenon_id, plan_id, plan_version_id,
                dataset_version_reference_id, measured_at_us, value_decimal, value_state,
                unit, measurement_method, conditions, sample_identifier, quality_status,
                supersedes_observation_id, created_by, created_at_us
         FROM research_observations WHERE id = ?1",
        [id],
        |row| {
            Ok(ObservationDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                phenomenon_id: row.get(2)?,
                plan_id: row.get(3)?,
                plan_version_id: row.get(4)?,
                dataset_version_reference_id: row.get(5)?,
                measured_at_us: row.get(6)?,
                value_decimal: row.get(7)?,
                value_state: parse_observation_value_state(&row.get::<_, String>(8)?)?,
                unit: row.get(9)?,
                measurement_method: row.get(10)?,
                conditions: row.get(11)?,
                sample_identifier: row.get(12)?,
                quality_status: parse_observation_quality_status(&row.get::<_, String>(13)?)?,
                supersedes_observation_id: row.get(14)?,
                created_by: row.get(15)?,
                created_at_us: row.get(16)?,
            })
        },
    )
}

pub fn create_pattern(
    conn: &mut Connection,
    req: &CreatePatternReq,
    id: &str,
    now: i64,
) -> Result<PatternDto> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO research_patterns
            (id, project_id, title, statement, analysis_method, uncertainty,
             lifecycle_status, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'candidate', ?7, ?8)",
        params![
            id,
            req.project_id,
            req.title.trim(),
            req.statement,
            req.analysis_method,
            req.uncertainty,
            req.created_by,
            now
        ],
    )?;
    for observation_id in &req.observation_ids {
        let matches_project: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM research_observations
             WHERE id = ?1 AND project_id = ?2)",
            params![observation_id, req.project_id],
            |row| row.get(0),
        )?;
        if !matches_project {
            return Err(rusqlite::Error::InvalidQuery);
        }
        tx.execute(
            "INSERT INTO research_pattern_observations (pattern_id, observation_id)
             VALUES (?1, ?2)",
            params![id, observation_id],
        )?;
    }
    for dataset_id in &req.dataset_version_reference_ids {
        let matches_project: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM research_dataset_version_references
             WHERE id = ?1 AND project_id = ?2)",
            params![dataset_id, req.project_id],
            |row| row.get(0),
        )?;
        if !matches_project {
            return Err(rusqlite::Error::InvalidQuery);
        }
        tx.execute(
            "INSERT INTO research_pattern_dataset_versions
                (pattern_id, dataset_version_reference_id) VALUES (?1, ?2)",
            params![id, dataset_id],
        )?;
    }
    insert_event(
        &tx,
        "pattern",
        id,
        "candidate",
        req.created_by.as_deref(),
        "",
        now,
    )?;
    tx.commit()?;
    query_pattern(conn, id)
}

fn query_pattern(conn: &Connection, id: &str) -> Result<PatternDto> {
    let mut pattern = conn.query_row(
        "SELECT id, project_id, title, statement, analysis_method, uncertainty,
                lifecycle_status, created_by, created_at_us
         FROM research_patterns WHERE id = ?1",
        [id],
        |row| {
            Ok(PatternDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                statement: row.get(3)?,
                analysis_method: row.get(4)?,
                uncertainty: row.get(5)?,
                lifecycle_status: parse_pattern_status(&row.get::<_, String>(6)?)?,
                observation_ids: Vec::new(),
                dataset_version_reference_ids: Vec::new(),
                created_by: row.get(7)?,
                created_at_us: row.get(8)?,
            })
        },
    )?;
    let mut observations = conn.prepare(
        "SELECT observation_id FROM research_pattern_observations
         WHERE pattern_id = ?1 ORDER BY observation_id",
    )?;
    pattern.observation_ids = observations
        .query_map([id], |row| row.get(0))?
        .collect::<Result<Vec<_>>>()?;
    let mut datasets = conn.prepare(
        "SELECT dataset_version_reference_id FROM research_pattern_dataset_versions
         WHERE pattern_id = ?1 ORDER BY dataset_version_reference_id",
    )?;
    pattern.dataset_version_reference_ids = datasets
        .query_map([id], |row| row.get(0))?
        .collect::<Result<Vec<_>>>()?;
    Ok(pattern)
}

pub fn create_idea(conn: &Connection, req: &CreateIdeaReq, id: &str, now: i64) -> Result<IdeaDto> {
    let origin = [
        req.phenomenon_id.as_deref(),
        req.observation_id.as_deref(),
        req.pattern_id.as_deref(),
        req.question_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    .next()
    .ok_or(rusqlite::Error::InvalidQuery)?;
    let origin_matches = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM research_phenomena WHERE id = ?1 AND project_id = ?2
            UNION ALL
            SELECT 1 FROM research_observations WHERE id = ?1 AND project_id = ?2
            UNION ALL
            SELECT 1 FROM research_patterns WHERE id = ?1 AND project_id = ?2
            UNION ALL
            SELECT 1 FROM research_questions WHERE id = ?1 AND project_id = ?2
         )",
        params![origin, req.project_id],
        |row| row.get::<_, bool>(0),
    )?;
    if !origin_matches {
        return Err(rusqlite::Error::InvalidQuery);
    }
    conn.execute(
        "INSERT INTO research_ideas
            (id, project_id, title, content, lifecycle_status, phenomenon_id,
             observation_id, pattern_id, question_id, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, 'captured', ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            id,
            req.project_id,
            req.title.trim(),
            req.content,
            req.phenomenon_id,
            req.observation_id,
            req.pattern_id,
            req.question_id,
            req.created_by,
            now
        ],
    )?;
    query_idea(conn, id)
}

fn query_idea(conn: &Connection, id: &str) -> Result<IdeaDto> {
    conn.query_row(
        "SELECT id, project_id, title, content, lifecycle_status, phenomenon_id,
                observation_id, pattern_id, question_id, created_by, created_at_us
         FROM research_ideas WHERE id = ?1",
        [id],
        |row| {
            Ok(IdeaDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                lifecycle_status: parse_idea_status(&row.get::<_, String>(4)?)?,
                phenomenon_id: row.get(5)?,
                observation_id: row.get(6)?,
                pattern_id: row.get(7)?,
                question_id: row.get(8)?,
                created_by: row.get(9)?,
                created_at_us: row.get(10)?,
            })
        },
    )
}

pub fn create_typed_hypothesis(
    conn: &mut Connection,
    req: &CreateTypedHypothesisReq,
    hypothesis_id: &str,
    legacy_hypothesis_id: &str,
    version_id: &str,
    now: i64,
) -> Result<ResearchHypothesisDto> {
    let tx = conn.transaction()?;
    let question_matches: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM research_questions
         WHERE id = ?1 AND project_id = ?2 AND lifecycle_status = 'accepted')",
        params![req.research_question_id, req.project_id],
        |row| row.get(0),
    )?;
    if !question_matches {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if let Some(parent_id) = req.parent_hypothesis_id.as_deref() {
        let parent_matches: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM research_hypotheses
             WHERE id = ?1 AND project_id = ?2)",
            params![parent_id, req.project_id],
            |row| row.get(0),
        )?;
        if !parent_matches || parent_id == hypothesis_id {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }
    let legacy_ms = now / 1_000;
    let independent = req
        .variables
        .iter()
        .filter(|variable| variable.role == HypothesisVariableRole::Independent)
        .collect::<Vec<_>>();
    let dependent = req
        .variables
        .iter()
        .filter(|variable| variable.role == HypothesisVariableRole::Dependent)
        .collect::<Vec<_>>();
    tx.execute(
        "INSERT INTO hypotheses
            (id, inquiry_id, statement_h1, statement_h0, independent_var, dependent_var,
             metric, success_threshold, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', '', 'draft', ?7, ?7)",
        params![
            legacy_hypothesis_id,
            req.research_question_id,
            req.statement_h1,
            req.statement_h0,
            if independent.len() == 1 {
                independent[0].name.as_str()
            } else {
                ""
            },
            if dependent.len() == 1 {
                dependent[0].name.as_str()
            } else {
                ""
            },
            legacy_ms
        ],
    )?;
    tx.execute(
        "INSERT INTO research_hypotheses
            (id, project_id, research_question_id, legacy_hypothesis_id,
             parent_hypothesis_id, ownership_state, lifecycle_status, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, 'project', 'draft', ?6, ?7)",
        params![
            hypothesis_id,
            req.project_id,
            req.research_question_id,
            legacy_hypothesis_id,
            req.parent_hypothesis_id,
            req.created_by,
            now
        ],
    )?;
    if let Some(parent_id) = req.parent_hypothesis_id.as_deref() {
        tx.execute(
            "INSERT INTO research_hypothesis_lineage
                (id, parent_hypothesis_id, derived_hypothesis_id, relation_kind,
                 rationale, created_by, created_at_us)
             VALUES (?1, ?2, ?3, 'derived_from', ?4, ?5, ?6)",
            params![
                Uuid::now_v7().to_string(),
                parent_id,
                hypothesis_id,
                req.lineage_rationale.as_deref().unwrap_or_default(),
                req.created_by,
                now
            ],
        )?;
    }
    tx.execute(
        "INSERT INTO hypothesis_definition_versions
            (id, hypothesis_id, version_no, statement_h0, statement_h1,
             population_scope, research_scope, expected_direction, expected_effect_decimal,
             expected_effect_unit, falsification_criteria, assumptions, lifecycle_status,
             created_by, created_at_us)
         VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'draft', ?12, ?13)",
        params![
            version_id,
            hypothesis_id,
            req.statement_h0,
            req.statement_h1,
            req.population_scope,
            req.research_scope,
            req.expected_direction.as_str(),
            req.expected_effect_decimal,
            req.expected_effect_unit,
            req.falsification_criteria,
            req.assumptions,
            req.created_by,
            now
        ],
    )?;
    for variable in &req.variables {
        tx.execute(
            "INSERT INTO hypothesis_variables
                (id, hypothesis_version_id, role, name, operational_definition,
                 value_kind, unit, measurement_method)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                Uuid::now_v7().to_string(),
                version_id,
                variable.role.as_str(),
                variable.name.trim(),
                variable.operational_definition,
                variable.value_kind.as_str(),
                variable.unit,
                variable.measurement_method
            ],
        )?;
    }
    insert_event(
        &tx,
        "hypothesis",
        hypothesis_id,
        "draft_definition_created",
        req.created_by.as_deref(),
        "",
        now,
    )?;
    insert_event(
        &tx,
        "hypothesis_version",
        version_id,
        "draft",
        req.created_by.as_deref(),
        "",
        now,
    )?;
    tx.commit()?;
    query_hypothesis(conn, hypothesis_id)
}

pub fn transition_hypothesis(
    conn: &Connection,
    hypothesis_id: &str,
    next_status: ResearchHypothesisStatus,
    actor_id: Option<&str>,
    now: i64,
) -> Result<ResearchHypothesisDto> {
    let tx = conn.unchecked_transaction()?;
    let current: String = tx.query_row(
        "SELECT lifecycle_status FROM research_hypotheses WHERE id = ?1",
        [hypothesis_id],
        |row| row.get(0),
    )?;
    let next = next_status.as_str();
    let allowed = matches!(
        (current.as_str(), next),
        ("draft", "review" | "closed")
            | ("review", "draft" | "preregistered" | "closed")
            | ("preregistered", "closed")
    );
    if !allowed {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if next == "preregistered" {
        let version_id: String = tx.query_row(
            "SELECT id FROM hypothesis_definition_versions
             WHERE hypothesis_id = ?1 ORDER BY version_no DESC LIMIT 1",
            [hypothesis_id],
            |row| row.get(0),
        )?;
        let variable_counts = tx.query_row(
            "SELECT
                sum(CASE WHEN role = 'independent' THEN 1 ELSE 0 END),
                sum(CASE WHEN role = 'dependent' THEN 1 ELSE 0 END)
             FROM hypothesis_variables WHERE hypothesis_version_id = ?1",
            [&version_id],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?.unwrap_or(0),
                    row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                ))
            },
        )?;
        if variable_counts.0 == 0 || variable_counts.1 == 0 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        tx.execute(
            "UPDATE hypothesis_definition_versions SET lifecycle_status = 'preregistered',
                preregistered_at_us = ?1 WHERE id = ?2 AND lifecycle_status IN ('draft', 'review')",
            params![now, version_id],
        )?;
    }
    tx.execute(
        "UPDATE research_hypotheses SET lifecycle_status = ?1 WHERE id = ?2",
        params![next, hypothesis_id],
    )?;
    insert_event(&tx, "hypothesis", hypothesis_id, next, actor_id, "", now)?;
    tx.commit()?;
    query_hypothesis(conn, hypothesis_id)
}

pub fn link_competing_hypotheses(
    conn: &Connection,
    req: &LinkCompetingHypothesesReq,
    now: i64,
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    let same_project = tx.query_row(
        "SELECT (a.project_id IS b.project_id)
         FROM research_hypotheses a, research_hypotheses b
         WHERE a.id = ?1 AND b.id = ?2",
        params![req.hypothesis_id, req.competing_hypothesis_id],
        |row| row.get::<_, bool>(0),
    )?;
    if !same_project || req.hypothesis_id == req.competing_hypothesis_id {
        return Err(rusqlite::Error::InvalidQuery);
    }
    tx.execute(
        "INSERT INTO hypothesis_competitions
            (hypothesis_id, competing_hypothesis_id, rationale, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            req.hypothesis_id,
            req.competing_hypothesis_id,
            req.rationale,
            req.created_by,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO hypothesis_competitions
            (hypothesis_id, competing_hypothesis_id, rationale, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            req.competing_hypothesis_id,
            req.hypothesis_id,
            req.rationale,
            req.created_by,
            now
        ],
    )?;
    tx.commit()
}

pub fn add_hypothesis_model_reference(
    conn: &Connection,
    req: &AddHypothesisModelReferenceReq,
    id: &str,
    now: i64,
) -> Result<HypothesisModelReferenceDto> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO hypothesis_model_references
            (id, hypothesis_id, model_id, model_version_id, label, rationale,
             resolution_status, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'unresolved', ?7, ?8)",
        params![
            id,
            req.hypothesis_id,
            req.model_id.trim(),
            req.model_version_id.trim(),
            req.label.trim(),
            req.rationale,
            req.created_by,
            now
        ],
    )?;
    insert_event(
        &tx,
        "hypothesis",
        &req.hypothesis_id,
        "model_reference_added",
        req.created_by.as_deref(),
        &req.rationale,
        now,
    )?;
    tx.commit()?;
    Ok(HypothesisModelReferenceDto {
        id: id.into(),
        hypothesis_id: req.hypothesis_id.clone(),
        model_id: req.model_id.trim().into(),
        model_version_id: req.model_version_id.trim().into(),
        label: req.label.trim().into(),
        rationale: req.rationale.clone(),
        resolution_status: "unresolved".into(),
    })
}

pub fn add_hypothesis_evidence(
    conn: &Connection,
    req: &AddHypothesisEvidenceReq,
    id: &str,
    now: i64,
) -> Result<()> {
    let (hypothesis_project, evidence_project): (Option<String>, Option<String>) = conn.query_row(
        "SELECT h.project_id,
                CASE
                    WHEN ?2 IS NOT NULL THEN (
                        SELECT h.project_id FROM hypothesis_definition_versions v
                        JOIN research_hypotheses h ON h.id = v.hypothesis_id WHERE v.id = ?1
                    )
                    WHEN ?3 IS NOT NULL THEN (
                        SELECT project_id FROM research_observations WHERE id = ?3
                    )
                    WHEN ?4 IS NOT NULL THEN (
                        SELECT project_id FROM research_patterns WHERE id = ?4
                    )
                    ELSE NULL
                END
             FROM hypothesis_definition_versions v
             JOIN research_hypotheses h ON h.id = v.hypothesis_id
             WHERE v.id = ?1",
        params![
            req.hypothesis_version_id,
            req.source_claim_id,
            req.observation_id,
            req.pattern_id
        ],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if req.source_claim_id.is_none() && hypothesis_project != evidence_project {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if let Some(claim_id) = req.source_claim_id.as_deref() {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM source_claims WHERE id = ?1)",
            [claim_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }
    let polarity = req.polarity.as_str();
    if let Some(claim_id) = req.source_claim_id.as_deref() {
        conn.execute(
            "INSERT INTO hypothesis_source_claim_evidence
                (id, hypothesis_version_id, source_claim_id, polarity, rationale, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, req.hypothesis_version_id, claim_id, polarity, req.rationale, req.created_by, now],
        )?;
    } else if let Some(observation_id) = req.observation_id.as_deref() {
        conn.execute(
            "INSERT INTO hypothesis_observation_evidence
                (id, hypothesis_version_id, observation_id, polarity, rationale, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, req.hypothesis_version_id, observation_id, polarity, req.rationale, req.created_by, now],
        )?;
    } else if let Some(pattern_id) = req.pattern_id.as_deref() {
        conn.execute(
            "INSERT INTO hypothesis_pattern_evidence
                (id, hypothesis_version_id, pattern_id, polarity, rationale, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, req.hypothesis_version_id, pattern_id, polarity, req.rationale, req.created_by, now],
        )?;
    }
    Ok(())
}

pub fn create_research_design(
    conn: &mut Connection,
    req: &CreateResearchDesignReq,
    design_id: &str,
    version_id: &str,
    now: i64,
) -> Result<ResearchExperimentDesignVersionDto> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO research_experiment_designs
            (id, project_id, title, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            design_id,
            req.project_id,
            req.title.trim(),
            req.created_by,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO research_experiment_design_versions
            (id, design_id, version_no, description, protocol, lifecycle_status, created_by, created_at_us)
         VALUES (?1, ?2, 1, ?3, ?4, 'draft', ?5, ?6)",
        params![
            version_id,
            design_id,
            req.description,
            req.protocol,
            req.created_by,
            now
        ],
    )?;
    insert_event(
        &tx,
        "experiment_design",
        design_id,
        "draft_version_created",
        req.created_by.as_deref(),
        "",
        now,
    )?;
    tx.commit()?;
    query_design_version(conn, version_id)
}

pub fn transition_research_design(
    conn: &Connection,
    version_id: &str,
    next_status: &str,
    actor_id: Option<&str>,
    now: i64,
) -> Result<ResearchExperimentDesignVersionDto> {
    let tx = conn.unchecked_transaction()?;
    let current: String = tx.query_row(
        "SELECT lifecycle_status FROM research_experiment_design_versions WHERE id = ?1",
        [version_id],
        |row| row.get(0),
    )?;
    let allowed = matches!(
        (current.as_str(), next_status),
        ("draft", "review" | "withdrawn")
            | ("review", "preregistered" | "withdrawn")
            | ("preregistered", "withdrawn")
    );
    if !allowed {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let preregistered_at = if next_status == "preregistered" {
        Some(now)
    } else {
        None
    };
    tx.execute(
        "UPDATE research_experiment_design_versions
         SET lifecycle_status = ?1, preregistered_at_us = ?2 WHERE id = ?3",
        params![next_status, preregistered_at, version_id],
    )?;
    let design_id: String = tx.query_row(
        "SELECT design_id FROM research_experiment_design_versions WHERE id = ?1",
        [version_id],
        |row| row.get(0),
    )?;
    insert_event(
        &tx,
        "experiment_design",
        &design_id,
        next_status,
        actor_id,
        "",
        now,
    )?;
    tx.commit()?;
    query_design_version(conn, version_id)
}

pub fn link_hypothesis_design(
    conn: &Connection,
    req: &LinkHypothesisDesignReq,
    now: i64,
) -> Result<()> {
    let same_project = conn.query_row(
        "SELECT (h.project_id IS d.project_id)
         FROM hypothesis_definition_versions v
         JOIN research_hypotheses h ON h.id = v.hypothesis_id,
         research_experiment_design_versions dv
         JOIN research_experiment_designs d ON d.id = dv.design_id
         WHERE v.id = ?1 AND dv.id = ?2",
        params![req.hypothesis_version_id, req.design_version_id],
        |row| row.get::<_, bool>(0),
    )?;
    if !same_project {
        return Err(rusqlite::Error::InvalidQuery);
    }
    conn.execute(
        "INSERT INTO hypothesis_design_links
            (hypothesis_version_id, design_version_id, relationship_kind,
             rationale, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            req.hypothesis_version_id,
            req.design_version_id,
            req.relationship_kind.as_str(),
            req.rationale,
            req.created_by,
            now
        ],
    )?;
    Ok(())
}

fn query_design_version(
    conn: &Connection,
    version_id: &str,
) -> Result<ResearchExperimentDesignVersionDto> {
    conn.query_row(
        "SELECT v.id, v.design_id, d.project_id, d.title, v.version_no, v.description,
                v.protocol, v.lifecycle_status, v.created_by, v.created_at_us, v.preregistered_at_us
         FROM research_experiment_design_versions v
         JOIN research_experiment_designs d ON d.id = v.design_id
         WHERE v.id = ?1",
        [version_id],
        |row| {
            Ok(ResearchExperimentDesignVersionDto {
                id: row.get(0)?,
                design_id: row.get(1)?,
                project_id: row.get(2)?,
                title: row.get(3)?,
                version_no: row.get(4)?,
                description: row.get(5)?,
                protocol: row.get(6)?,
                lifecycle_status: parse_design_status(&row.get::<_, String>(7)?)?,
                created_by: row.get(8)?,
                created_at_us: row.get(9)?,
                preregistered_at_us: row.get(10)?,
            })
        },
    )
}

fn query_hypothesis(conn: &Connection, hypothesis_id: &str) -> Result<ResearchHypothesisDto> {
    let mut hypothesis = conn.query_row(
        "SELECT id, project_id, research_question_id, legacy_hypothesis_id,
                parent_hypothesis_id, ownership_state, lifecycle_status, created_by, created_at_us
         FROM research_hypotheses WHERE id = ?1",
        [hypothesis_id],
        |row| {
            let status = row.get::<_, String>(6)?;
            Ok(ResearchHypothesisDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                research_question_id: row.get(2)?,
                legacy_hypothesis_id: row.get(3)?,
                parent_hypothesis_id: row.get(4)?,
                ownership_state: row.get(5)?,
                lifecycle_status: match status.as_str() {
                    "draft" => ResearchHypothesisStatus::Draft,
                    "review" => ResearchHypothesisStatus::Review,
                    "preregistered" => ResearchHypothesisStatus::Preregistered,
                    "closed" => ResearchHypothesisStatus::Closed,
                    _ => ResearchHypothesisStatus::LegacyUntyped,
                },
                definitions: Vec::new(),
                competing_hypothesis_ids: Vec::new(),
                model_references: Vec::new(),
                evidence: Vec::new(),
                created_by: row.get(7)?,
                created_at_us: row.get(8)?,
            })
        },
    )?;
    let mut versions = conn.prepare(
        "SELECT id FROM hypothesis_definition_versions
         WHERE hypothesis_id = ?1 ORDER BY version_no",
    )?;
    let version_ids = versions
        .query_map([hypothesis_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>>>()?;
    for version_id in version_ids {
        hypothesis
            .definitions
            .push(query_hypothesis_definition(conn, &version_id)?);
    }
    let mut competitors = conn.prepare(
        "SELECT competing_hypothesis_id FROM hypothesis_competitions
         WHERE hypothesis_id = ?1 ORDER BY competing_hypothesis_id",
    )?;
    hypothesis.competing_hypothesis_ids = competitors
        .query_map([hypothesis_id], |row| row.get(0))?
        .collect::<Result<Vec<_>>>()?;
    let mut model_references = conn.prepare(
        "SELECT id, hypothesis_id, model_id, model_version_id, label, rationale, resolution_status
         FROM hypothesis_model_references WHERE hypothesis_id = ?1 ORDER BY created_at_us, id",
    )?;
    hypothesis.model_references = model_references
        .query_map([hypothesis_id], |row| {
            Ok(HypothesisModelReferenceDto {
                id: row.get(0)?,
                hypothesis_id: row.get(1)?,
                model_id: row.get(2)?,
                model_version_id: row.get(3)?,
                label: row.get(4)?,
                rationale: row.get(5)?,
                resolution_status: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    let mut source_evidence = conn.prepare(
        "SELECT e.source_claim_id, e.polarity, e.rationale, c.source_id,
                c.source_version_id, x.source_segment_id
         FROM hypothesis_source_claim_evidence e
         JOIN source_claims c ON c.id = e.source_claim_id
         JOIN knowledge_extractions x ON x.id = c.extraction_id
         JOIN hypothesis_definition_versions v ON v.id = e.hypothesis_version_id
         WHERE v.hypothesis_id = ?1 ORDER BY e.created_at_us, e.id",
    )?;
    hypothesis.evidence.extend(
        source_evidence
            .query_map([hypothesis_id], |row| {
                let polarity = row.get::<_, String>(1)?;
                Ok(HypothesisEvidenceDto {
                    target_kind: "source_claim".into(),
                    target_id: row.get(0)?,
                    polarity: parse_polarity(&polarity)?,
                    rationale: row.get(2)?,
                    source_id: row.get(3)?,
                    source_version_id: row.get(4)?,
                    source_location_id: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?,
    );
    for (table, target_kind, id_column) in [
        (
            "hypothesis_observation_evidence",
            "observation",
            "observation_id",
        ),
        ("hypothesis_pattern_evidence", "pattern", "pattern_id"),
    ] {
        let sql = format!(
            "SELECT e.{id_column}, e.polarity, e.rationale
             FROM {table} e JOIN hypothesis_definition_versions v
               ON v.id = e.hypothesis_version_id
             WHERE v.hypothesis_id = ?1 ORDER BY e.created_at_us, e.id"
        );
        let mut statement = conn.prepare(&sql)?;
        hypothesis.evidence.extend(
            statement
                .query_map([hypothesis_id], |row| {
                    let polarity = row.get::<_, String>(1)?;
                    Ok(HypothesisEvidenceDto {
                        target_kind: target_kind.into(),
                        target_id: row.get(0)?,
                        polarity: parse_polarity(&polarity)?,
                        rationale: row.get(2)?,
                        source_id: None,
                        source_version_id: None,
                        source_location_id: None,
                    })
                })?
                .collect::<Result<Vec<_>>>()?,
        );
    }
    Ok(hypothesis)
}

fn parse_polarity(value: &str) -> rusqlite::Result<ResearchPolarity> {
    match value {
        "supports" => Ok(ResearchPolarity::Supports),
        "contradicts" => Ok(ResearchPolarity::Contradicts),
        "qualifies" => Ok(ResearchPolarity::Qualifies),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_domain_status(value: &str) -> rusqlite::Result<ResearchDomainStatus> {
    match value {
        "proposed" => Ok(ResearchDomainStatus::Proposed),
        "active" => Ok(ResearchDomainStatus::Active),
        "deprecated" => Ok(ResearchDomainStatus::Deprecated),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_review_status(value: &str) -> rusqlite::Result<ReviewLifecycleStatus> {
    match value {
        "draft" => Ok(ReviewLifecycleStatus::Draft),
        "reviewed" => Ok(ReviewLifecycleStatus::Reviewed),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_phenomenon_status(value: &str) -> rusqlite::Result<PhenomenonStatus> {
    match value {
        "identified" => Ok(PhenomenonStatus::Identified),
        "characterized" => Ok(PhenomenonStatus::Characterized),
        "retired" => Ok(PhenomenonStatus::Retired),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_plan_status(value: &str) -> rusqlite::Result<ObservationPlanStatus> {
    match value {
        "draft" => Ok(ObservationPlanStatus::Draft),
        "reviewed" => Ok(ObservationPlanStatus::Reviewed),
        "preregistered" => Ok(ObservationPlanStatus::Preregistered),
        "withdrawn" => Ok(ObservationPlanStatus::Withdrawn),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_observation_value_state(value: &str) -> rusqlite::Result<ObservationValueState> {
    match value {
        "observed" => Ok(ObservationValueState::Observed),
        "not_measured" => Ok(ObservationValueState::NotMeasured),
        "not_recorded" => Ok(ObservationValueState::NotRecorded),
        "unknown" => Ok(ObservationValueState::Unknown),
        "not_applicable" => Ok(ObservationValueState::NotApplicable),
        "withheld" => Ok(ObservationValueState::Withheld),
        "invalid" => Ok(ObservationValueState::Invalid),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_observation_quality_status(value: &str) -> rusqlite::Result<ObservationQualityStatus> {
    match value {
        "unreviewed" => Ok(ObservationQualityStatus::Unreviewed),
        "accepted" => Ok(ObservationQualityStatus::Accepted),
        "excluded" => Ok(ObservationQualityStatus::Excluded),
        "retracted" => Ok(ObservationQualityStatus::Retracted),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_pattern_status(value: &str) -> rusqlite::Result<PatternStatus> {
    match value {
        "candidate" => Ok(PatternStatus::Candidate),
        "examined" => Ok(PatternStatus::Examined),
        "supported" => Ok(PatternStatus::Supported),
        "unsupported" => Ok(PatternStatus::Unsupported),
        "retired" => Ok(PatternStatus::Retired),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_idea_status(value: &str) -> rusqlite::Result<IdeaStatus> {
    match value {
        "captured" => Ok(IdeaStatus::Captured),
        "explored" => Ok(IdeaStatus::Explored),
        "promoted" => Ok(IdeaStatus::Promoted),
        "abandoned" => Ok(IdeaStatus::Abandoned),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_expected_direction(value: &str) -> rusqlite::Result<HypothesisExpectedDirection> {
    match value {
        "positive" => Ok(HypothesisExpectedDirection::Positive),
        "negative" => Ok(HypothesisExpectedDirection::Negative),
        "non_directional" => Ok(HypothesisExpectedDirection::NonDirectional),
        "equivalence" => Ok(HypothesisExpectedDirection::Equivalence),
        "other" => Ok(HypothesisExpectedDirection::Other),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn parse_design_status(value: &str) -> rusqlite::Result<ResearchDesignStatus> {
    match value {
        "draft" => Ok(ResearchDesignStatus::Draft),
        "review" => Ok(ResearchDesignStatus::Review),
        "preregistered" => Ok(ResearchDesignStatus::Preregistered),
        "withdrawn" => Ok(ResearchDesignStatus::Withdrawn),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn query_hypothesis_definition(
    conn: &Connection,
    version_id: &str,
) -> Result<HypothesisDefinitionVersionDto> {
    let mut version = conn.query_row(
        "SELECT id, hypothesis_id, version_no, parent_version_id, statement_h0, statement_h1,
                population_scope, research_scope, expected_direction, expected_effect_decimal,
                expected_effect_unit, falsification_criteria, assumptions, lifecycle_status,
                created_by, created_at_us, preregistered_at_us
         FROM hypothesis_definition_versions WHERE id = ?1",
        [version_id],
        |row| {
            let status = row.get::<_, String>(13)?;
            Ok(HypothesisDefinitionVersionDto {
                id: row.get(0)?,
                hypothesis_id: row.get(1)?,
                version_no: row.get(2)?,
                parent_version_id: row.get(3)?,
                statement_h0: row.get(4)?,
                statement_h1: row.get(5)?,
                population_scope: row.get(6)?,
                research_scope: row.get(7)?,
                expected_direction: parse_expected_direction(&row.get::<_, String>(8)?)?,
                expected_effect_decimal: row.get(9)?,
                expected_effect_unit: row.get(10)?,
                falsification_criteria: row.get(11)?,
                assumptions: row.get(12)?,
                lifecycle_status: match status.as_str() {
                    "review" => HypothesisDefinitionStatus::Review,
                    "preregistered" => HypothesisDefinitionStatus::Preregistered,
                    _ => HypothesisDefinitionStatus::Draft,
                },
                created_by: row.get(14)?,
                created_at_us: row.get(15)?,
                preregistered_at_us: row.get(16)?,
                variables: Vec::new(),
            })
        },
    )?;
    let mut statement = conn.prepare(
        "SELECT id, hypothesis_version_id, role, name, operational_definition,
                value_kind, unit, measurement_method
         FROM hypothesis_variables WHERE hypothesis_version_id = ?1 ORDER BY role, name",
    )?;
    version.variables = statement
        .query_map([version_id], |row| {
            let role = row.get::<_, String>(2)?;
            let value_kind = row.get::<_, String>(5)?;
            Ok(HypothesisVariableDto {
                id: row.get(0)?,
                hypothesis_version_id: row.get(1)?,
                role: match role.as_str() {
                    "dependent" => HypothesisVariableRole::Dependent,
                    "covariate" => HypothesisVariableRole::Covariate,
                    _ => HypothesisVariableRole::Independent,
                },
                name: row.get(3)?,
                operational_definition: row.get(4)?,
                value_kind: match value_kind.as_str() {
                    "discrete" => HypothesisValueKind::Discrete,
                    "categorical" => HypothesisValueKind::Categorical,
                    "qualitative" => HypothesisValueKind::Qualitative,
                    "other" => HypothesisValueKind::Other,
                    _ => HypothesisValueKind::Continuous,
                },
                unit: row.get(6)?,
                measurement_method: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(version)
}

pub fn workspace(conn: &Connection) -> Result<ResearchWorkspaceDto> {
    let mut pathways_statement = conn
        .prepare("SELECT id, pathway_key, title, description FROM research_pathways ORDER BY id")?;
    let pathways = pathways_statement
        .query_map([], |row| {
            let key = row.get::<_, String>(1)?;
            Ok(ResearchPathwayDto {
                id: row.get(0)?,
                pathway_key: ResearchPathwayKey::parse(&key)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                title: row.get(2)?,
                description: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    let project_ids = query_ids(
        conn,
        "SELECT id FROM research_projects ORDER BY updated_at_us DESC, id",
    )?;
    let projects = project_ids
        .iter()
        .map(|id| query_project(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let mut domains_statement = conn.prepare(
        "SELECT id, name, description, parent_domain_id, lifecycle_status, created_by, created_at_us
         FROM research_domains ORDER BY name, id",
    )?;
    let domains = domains_statement
        .query_map([], |row| {
            Ok(ResearchDomainDto {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                parent_domain_id: row.get(3)?,
                lifecycle_status: parse_domain_status(&row.get::<_, String>(4)?)?,
                created_by: row.get(5)?,
                created_at_us: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    let mut framework_statement = conn.prepare(
        "SELECT f.id, v.id, v.title, v.version_no
         FROM knowledge_framework_versions v
         JOIN knowledge_frameworks f ON f.id = v.framework_id
         WHERE v.status = 'published'
         ORDER BY v.title, v.version_no DESC",
    )?;
    let framework_versions = framework_statement
        .query_map([], |row| {
            Ok(FrameworkVersionReferenceDto {
                framework_id: row.get(0)?,
                framework_version_id: row.get(1)?,
                title: row.get(2)?,
                version_no: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    let mut source_statement = conn.prepare(
        "SELECT s.id, v.id, s.title, v.edition_label, seg.id, seg.label,
                (SELECT c.id FROM source_claims c
                 WHERE c.source_id = s.id AND c.source_version_id = v.id
                 ORDER BY c.created_at_us LIMIT 1),
                v.captured_at_us
         FROM source_versions v
         JOIN sources s ON s.id = v.source_id
         LEFT JOIN source_version_segments seg ON seg.source_id = v.source_id
             AND seg.source_version_id = v.id
         ORDER BY s.title, v.captured_at_us DESC, seg.order_index, seg.id",
    )?;
    let source_versions = source_statement
        .query_map([], |row| {
            Ok(SourceVersionReferenceDto {
                source_id: row.get(0)?,
                source_version_id: row.get(1)?,
                source_title: row.get(2)?,
                edition_label: row.get(3)?,
                source_location_id: row.get(4)?,
                source_location_label: row.get(5)?,
                source_claim_id: row.get(6)?,
                captured_at_us: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    let mut adoption_statement = conn.prepare(
        "SELECT a.id, a.project_id, a.framework_id, a.framework_version_id,
                v.title, v.version_no, a.use_context, a.adopted_by, a.adopted_at_us
         FROM research_project_knowledge_adoptions a
         JOIN knowledge_framework_versions v ON v.id = a.framework_version_id
         ORDER BY a.adopted_at_us DESC",
    )?;
    let adoptions = adoption_statement
        .query_map([], |row| {
            Ok(ProjectKnowledgeAdoptionDto {
                id: row.get(0)?,
                project_id: row.get(1)?,
                framework_id: row.get(2)?,
                framework_version_id: row.get(3)?,
                title: row.get(4)?,
                version_no: row.get(5)?,
                use_context: row.get(6)?,
                adopted_by: row.get(7)?,
                adopted_at_us: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    let review_ids = query_ids(
        conn,
        "SELECT id FROM literature_reviews ORDER BY created_at_us DESC",
    )?;
    let literature_reviews = review_ids
        .iter()
        .map(|id| query_literature_review(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let assessment_ids = query_ids(
        conn,
        "SELECT id FROM prior_knowledge_assessments ORDER BY created_at_us DESC",
    )?;
    let prior_assessments = assessment_ids
        .iter()
        .map(|id| query_prior_assessment(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let gap_ids = query_ids(
        conn,
        "SELECT id FROM research_gaps ORDER BY updated_at_us DESC",
    )?;
    let gaps = gap_ids
        .iter()
        .map(|id| query_gap(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let question_ids = query_ids(
        conn,
        "SELECT id FROM research_questions ORDER BY mapped_at_us DESC",
    )?;
    let questions = question_ids
        .iter()
        .map(|id| query_question(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let question_lineage = {
        let mut statement = conn.prepare(
            "SELECT id, parent_question_id, derived_question_id, relation_kind,
                    rationale, created_by, created_at_us
             FROM research_question_lineage ORDER BY created_at_us, id",
        )?;
        let rows = statement
            .query_map([], |row| {
                let relation = row.get::<_, String>(3)?;
                Ok(ResearchQuestionLineageDto {
                    id: row.get(0)?,
                    parent_question_id: row.get(1)?,
                    derived_question_id: row.get(2)?,
                    relation_kind: match relation.as_str() {
                        "refined_from" => ResearchQuestionRelation::RefinedFrom,
                        "supersedes" => ResearchQuestionRelation::Supersedes,
                        _ => ResearchQuestionRelation::DerivedFrom,
                    },
                    rationale: row.get(4)?,
                    created_by: row.get(5)?,
                    created_at_us: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        rows
    };
    let phenomenon_ids = query_ids(
        conn,
        "SELECT id FROM research_phenomena ORDER BY created_at_us DESC",
    )?;
    let phenomena = phenomenon_ids
        .iter()
        .map(|id| query_phenomenon(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let plan_ids = query_ids(
        conn,
        "SELECT id FROM observation_plans ORDER BY created_at_us DESC",
    )?;
    let observation_plans = plan_ids
        .iter()
        .map(|id| query_observation_plan(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let dataset_ids = query_ids(
        conn,
        "SELECT id FROM research_dataset_version_references ORDER BY created_at_us DESC",
    )?;
    let dataset_versions = dataset_ids
        .iter()
        .map(|id| query_dataset_version(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let observation_ids = query_ids(
        conn,
        "SELECT id FROM research_observations ORDER BY measured_at_us DESC",
    )?;
    let observations = observation_ids
        .iter()
        .map(|id| query_observation(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let pattern_ids = query_ids(
        conn,
        "SELECT id FROM research_patterns ORDER BY created_at_us DESC",
    )?;
    let patterns = pattern_ids
        .iter()
        .map(|id| query_pattern(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let idea_ids = query_ids(
        conn,
        "SELECT id FROM research_ideas ORDER BY created_at_us DESC",
    )?;
    let ideas = idea_ids
        .iter()
        .map(|id| query_idea(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let hypothesis_ids = query_ids(
        conn,
        "SELECT id FROM research_hypotheses ORDER BY created_at_us DESC",
    )?;
    let hypotheses = hypothesis_ids
        .iter()
        .map(|id| query_hypothesis(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let design_ids = query_ids(
        conn,
        "SELECT id FROM research_experiment_design_versions ORDER BY created_at_us DESC",
    )?;
    let design_versions = design_ids
        .iter()
        .map(|id| query_design_version(conn, id))
        .collect::<Result<Vec<_>>>()?;

    Ok(ResearchWorkspaceDto {
        pathways,
        projects,
        domains,
        framework_versions,
        source_versions,
        adoptions,
        literature_reviews,
        prior_assessments,
        gaps,
        questions,
        question_lineage,
        phenomena,
        observation_plans,
        dataset_versions,
        observations,
        patterns,
        ideas,
        hypotheses,
        design_versions,
    })
}

fn query_ids(conn: &Connection, sql: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map([], |row| row.get(0))?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initialized_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        crate::mind::portable_db::initialize_workspace_schema(&conn)
            .expect("initialize complete workspace schema");
        conn
    }

    fn project(conn: &mut Connection, title: &str) -> ResearchProjectDto {
        create_project(
            conn,
            &CreateResearchProjectReq {
                title: title.into(),
                aim: "Test research aim".into(),
                pathways: vec![ResearchPathwayKey::FirstPrinciplesDiscovery],
                created_by: None,
            },
            &Uuid::now_v7().to_string(),
            now_ts_us().expect("timestamp"),
        )
        .expect("create project")
    }

    fn accepted_question(conn: &mut Connection, project_id: &str) -> ResearchQuestionDto {
        let question = create_question(
            conn,
            &CreateResearchQuestionReq {
                project_id: Some(project_id.into()),
                research_gap_id: None,
                domain_id: None,
                title: "Does treatment change outcome?".into(),
                description: "Test question".into(),
                rationale: "It is answerable".into(),
                created_by: None,
            },
            &Uuid::now_v7().to_string(),
            now_ts_us().expect("timestamp"),
        )
        .expect("create question");
        transition_question(
            conn,
            &question.id,
            ResearchQuestionStatus::Review,
            None,
            "Ready for review",
            now_ts_us().expect("timestamp"),
        )
        .expect("submit question");
        transition_question(
            conn,
            &question.id,
            ResearchQuestionStatus::Accepted,
            None,
            "Accepted",
            now_ts_us().expect("timestamp"),
        )
        .expect("accept question")
    }

    fn typed_hypothesis(
        conn: &mut Connection,
        project_id: &str,
        question_id: &str,
    ) -> ResearchHypothesisDto {
        create_typed_hypothesis(
            conn,
            &CreateTypedHypothesisReq {
                project_id: project_id.into(),
                research_question_id: question_id.into(),
                parent_hypothesis_id: None,
                lineage_rationale: None,
                statement_h0: "The treatment has no effect.".into(),
                statement_h1: "The treatment changes the outcome.".into(),
                population_scope: "Adults".into(),
                research_scope: "This study".into(),
                expected_direction: "non_directional".into(),
                expected_effect_decimal: None,
                expected_effect_unit: None,
                falsification_criteria: "Reject if observed effect exceeds stated threshold."
                    .into(),
                assumptions: "Measurements are comparable.".into(),
                variables: vec![
                    HypothesisVariableInput {
                        role: HypothesisVariableRole::Independent,
                        name: "Treatment dose".into(),
                        operational_definition: "Dose in milligrams".into(),
                        value_kind: HypothesisValueKind::Continuous,
                        unit: Some("mg".into()),
                        measurement_method: "Dose record".into(),
                    },
                    HypothesisVariableInput {
                        role: HypothesisVariableRole::Dependent,
                        name: "Outcome".into(),
                        operational_definition: "Score at follow-up".into(),
                        value_kind: HypothesisValueKind::Continuous,
                        unit: Some("points".into()),
                        measurement_method: "Validated scale".into(),
                    },
                ],
                created_by: None,
            },
            &Uuid::now_v7().to_string(),
            &Uuid::now_v7().to_string(),
            &Uuid::now_v7().to_string(),
            now_ts_us().expect("timestamp"),
        )
        .expect("create typed hypothesis")
    }

    #[test]
    fn migration_preserves_legacy_questions_and_hypotheses_as_unassessed() {
        let conn = initialized_connection();
        conn.execute(
            "INSERT INTO questions (id, title, description, created_at, updated_at)
             VALUES ('legacy-question', 'Legacy question', 'Original detail', 10, 20)",
            [],
        )
        .expect("insert legacy question");
        conn.execute(
            "INSERT INTO hypotheses
                (id, inquiry_id, statement_h1, statement_h0, independent_var, dependent_var,
                 metric, success_threshold, status, created_at, updated_at)
             VALUES ('legacy-hypothesis', 'legacy-question', 'Legacy H1', 'Legacy H0',
                     'x', 'y', 'metric', 'threshold', 'testing', 10, 20)",
            [],
        )
        .expect("insert legacy hypothesis");

        let workspace = super::super::workspace(&conn).expect("load Research workspace");
        let question = workspace
            .questions
            .iter()
            .find(|question| question.id == "legacy-question")
            .expect("mapped legacy question");
        assert_eq!(
            question.lifecycle_status,
            ResearchQuestionStatus::LegacyUnassessed
        );
        assert_eq!(question.ownership_state, "unassigned");
        assert_eq!(question.project_id, None);
        assert_eq!(question.title, "Legacy question");

        let hypothesis = workspace
            .hypotheses
            .iter()
            .find(|hypothesis| {
                hypothesis.legacy_hypothesis_id.as_deref() == Some("legacy-hypothesis")
            })
            .expect("mapped legacy hypothesis");
        assert_eq!(
            hypothesis.lifecycle_status,
            ResearchHypothesisStatus::LegacyUntyped
        );
        assert!(hypothesis.definitions.is_empty());
        assert_eq!(hypothesis.project_id, None);
        assert_eq!(
            hypothesis.research_question_id.as_deref(),
            Some("legacy-question")
        );
    }

    #[test]
    fn accepted_gaps_require_reviewed_prior_knowledge_evidence() {
        let mut conn = initialized_connection();
        let project = project(&mut conn, "Evidence gated project");
        let gap = create_gap(
            &conn,
            &CreateResearchGapReq {
                project_id: project.id.clone(),
                domain_id: None,
                title: "Unanswered question".into(),
                statement: "Prior work does not resolve this outcome.".into(),
                created_by: None,
            },
            &Uuid::now_v7().to_string(),
            now_ts_us().expect("timestamp"),
        )
        .expect("create gap");
        transition_gap(
            &conn,
            &gap.id,
            ResearchGapStatus::Assessed,
            None,
            now_ts_us().expect("timestamp"),
        )
        .expect("assess gap");
        assert!(transition_gap(
            &conn,
            &gap.id,
            ResearchGapStatus::Accepted,
            None,
            now_ts_us().expect("timestamp"),
        )
        .is_err());

        let assessment = create_prior_assessment(
            &conn,
            &CreatePriorKnowledgeAssessmentReq {
                project_id: project.id.clone(),
                title: "Prior-knowledge assessment".into(),
                assessment: "The known literature leaves the stated outcome unresolved.".into(),
                framework_version_id: None,
                created_by: None,
            },
            &Uuid::now_v7().to_string(),
            now_ts_us().expect("timestamp"),
        )
        .expect("create assessment");
        review_prior_assessment(
            &conn,
            &assessment.id,
            Some("reviewer"),
            now_ts_us().expect("timestamp"),
        )
        .expect("review assessment");
        add_gap_evidence(
            &conn,
            &AddGapEvidenceReq {
                gap_id: gap.id.clone(),
                literature_review_id: None,
                prior_assessment_id: Some(assessment.id),
            },
        )
        .expect("attach prior assessment");
        let accepted = transition_gap(
            &conn,
            &gap.id,
            ResearchGapStatus::Accepted,
            None,
            now_ts_us().expect("timestamp"),
        )
        .expect("accept evidence-backed gap");
        assert_eq!(accepted.lifecycle_status, ResearchGapStatus::Accepted);
        assert_eq!(accepted.prior_assessment_ids.len(), 1);
    }

    #[test]
    fn questions_keep_lineage_and_competing_hypotheses_keep_typed_definitions() {
        let mut conn = initialized_connection();
        let project = project(&mut conn, "Lineage project");
        let parent = accepted_question(&mut conn, &project.id);
        let child = refine_question(
            &mut conn,
            &parent.id,
            "Does the dose change the outcome in subgroup A?",
            "A refined question.",
            "Narrowed to the target subgroup.",
            None,
            &Uuid::now_v7().to_string(),
            now_ts_us().expect("timestamp"),
        )
        .expect("refine question");
        assert_eq!(child.lifecycle_status, ResearchQuestionStatus::Draft);
        let lineage_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM research_question_lineage
                 WHERE parent_question_id = ?1 AND derived_question_id = ?2
                   AND relation_kind = 'refined_from'",
                params![parent.id, child.id],
                |row| row.get(0),
            )
            .expect("read question lineage");
        assert_eq!(lineage_count, 1);
        transition_question(
            &conn,
            &child.id,
            ResearchQuestionStatus::Review,
            None,
            "Ready for review",
            now_ts_us().expect("timestamp"),
        )
        .expect("submit refined question");
        transition_question(
            &conn,
            &child.id,
            ResearchQuestionStatus::Accepted,
            None,
            "Accepted",
            now_ts_us().expect("timestamp"),
        )
        .expect("accept refined question");

        let parent_hypothesis = typed_hypothesis(&mut conn, &project.id, &child.id);
        let competing = typed_hypothesis(&mut conn, &project.id, &child.id);
        link_competing_hypotheses(
            &conn,
            &LinkCompetingHypothesesReq {
                hypothesis_id: parent_hypothesis.id.clone(),
                competing_hypothesis_id: competing.id.clone(),
                rationale: "Different predicted mechanism.".into(),
                created_by: None,
            },
            now_ts_us().expect("timestamp"),
        )
        .expect("link competing hypotheses");
        assert_eq!(parent_hypothesis.definitions[0].variables.len(), 2);
        assert_eq!(
            query_hypothesis(&conn, &parent_hypothesis.id)
                .expect("read hypothesis")
                .competing_hypothesis_ids,
            vec![competing.id.clone()]
        );

        for _ in 0..2 {
            let design = create_research_design(
                &mut conn,
                &CreateResearchDesignReq {
                    project_id: project.id.clone(),
                    title: "Design version".into(),
                    description: "Design only".into(),
                    protocol: "Protocol outline".into(),
                    created_by: None,
                },
                &Uuid::now_v7().to_string(),
                &Uuid::now_v7().to_string(),
                now_ts_us().expect("timestamp"),
            )
            .expect("create design");
            link_hypothesis_design(
                &conn,
                &LinkHypothesisDesignReq {
                    hypothesis_version_id: parent_hypothesis.definitions[0].id.clone(),
                    design_version_id: design.id,
                    relationship_kind: "tests".into(),
                    rationale: "Design targets this hypothesis.".into(),
                    created_by: None,
                },
                now_ts_us().expect("timestamp"),
            )
            .expect("link design");
        }
        let design_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM hypothesis_design_links WHERE hypothesis_version_id = ?1",
                [&parent_hypothesis.definitions[0].id],
                |row| row.get(0),
            )
            .expect("count linked designs");
        assert_eq!(design_count, 2);
    }

    #[test]
    fn observation_plans_and_records_are_distinct_from_patterns() {
        let mut conn = initialized_connection();
        let project = project(&mut conn, "Observation project");
        let phenomenon = create_phenomenon(
            &conn,
            &CreatePhenomenonReq {
                project_id: project.id.clone(),
                domain_id: None,
                title: "Measured phenomenon".into(),
                description: "A measurable event.".into(),
                population_scope: "Samples".into(),
                context: "Controlled context".into(),
                temporal_boundary: "One session".into(),
                created_by: None,
            },
            &Uuid::now_v7().to_string(),
            now_ts_us().expect("timestamp"),
        )
        .expect("create phenomenon");
        let plan = create_observation_plan(
            &mut conn,
            &CreateObservationPlanReq {
                project_id: project.id.clone(),
                phenomenon_id: phenomenon.id.clone(),
                title: "Measurement plan".into(),
                measurement_method: "Sensor".into(),
                sample_frame: "Ten samples".into(),
                conditions_template: "Room temperature".into(),
                inclusion_rules: "Valid readings only".into(),
                created_by: None,
            },
            &Uuid::now_v7().to_string(),
            &Uuid::now_v7().to_string(),
            now_ts_us().expect("timestamp"),
        )
        .expect("create plan");
        let reviewed_plan = review_observation_plan(
            &conn,
            &plan.versions[0].id,
            false,
            None,
            now_ts_us().expect("timestamp"),
        )
        .expect("review plan");
        let observation = create_observation(
            &conn,
            &CreateObservationReq {
                project_id: project.id.clone(),
                phenomenon_id: phenomenon.id.clone(),
                plan_id: plan.id.clone(),
                plan_version_id: reviewed_plan.id,
                dataset_version_reference_id: None,
                measured_at_us: now_ts_us().expect("timestamp"),
                value_decimal: Some("4.2".into()),
                value_state: "observed".into(),
                unit: Some("units".into()),
                measurement_method: "Sensor".into(),
                conditions: "Room temperature".into(),
                sample_identifier: Some("sample-1".into()),
                created_by: None,
            },
            &Uuid::now_v7().to_string(),
            now_ts_us().expect("timestamp"),
        )
        .expect("create observation");
        let pattern = create_pattern(
            &mut conn,
            &CreatePatternReq {
                project_id: project.id,
                title: "Increasing trend".into(),
                statement: "Values increased across samples.".into(),
                analysis_method: "Ordered comparison".into(),
                uncertainty: "Only one sample in this fixture.".into(),
                observation_ids: vec![observation.id.clone()],
                dataset_version_reference_ids: Vec::new(),
                created_by: None,
            },
            &Uuid::now_v7().to_string(),
            now_ts_us().expect("timestamp"),
        )
        .expect("create pattern");
        assert_eq!(pattern.observation_ids, vec![observation.id]);
        assert_ne!(pattern.id, phenomenon.id);
    }

    #[test]
    fn research_records_survive_database_reopen() {
        let path = std::env::temp_dir().join(format!("vora-research-{}.db", Uuid::now_v7()));
        {
            let conn = Connection::open(&path).expect("open file-backed database");
            crate::mind::portable_db::initialize_workspace_schema(&conn)
                .expect("initialize file-backed schema");
            let mut conn = conn;
            let project = project(&mut conn, "Persistent project");
            assert!(!project.id.is_empty());
        }
        {
            let conn = Connection::open(&path).expect("reopen file-backed database");
            crate::mind::portable_db::initialize_workspace_schema(&conn).expect("reopen schema");
            let workspace = super::super::workspace(&conn).expect("reload workspace");
            assert!(workspace
                .projects
                .iter()
                .any(|item| item.title == "Persistent project"));
        }
        std::fs::remove_file(path).expect("remove temporary database");
    }
}
