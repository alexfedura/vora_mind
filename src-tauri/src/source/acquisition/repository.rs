use rusqlite::{params, Connection, OptionalExtension, Row};

use super::*;

type RepoResult<T> = Result<T, rusqlite::Error>;

fn record_event(
    conn: &Connection,
    target_kind: &str,
    target_id: &str,
    event_kind: &str,
    actor_id: Option<&str>,
    occurred_at_us: i64,
    reason: &str,
    before_json: Option<&str>,
    after_json: Option<&str>,
) -> RepoResult<()> {
    let columns = match target_kind {
        "source" => ("source_id", "source_id"),
        "source_version" => ("source_version_id", "source_version_id"),
        "source_location" => ("source_segment_id", "source_segment_id"),
        "extraction" => ("extraction_id", "extraction_id"),
        "source_claim" => ("source_claim_id", "source_claim_id"),
        "concept" => ("concept_id", "concept_id"),
        "principle" => ("principle_id", "principle_id"),
        "procedure" => ("procedure_id", "procedure_id"),
        "framework" => ("framework_id", "framework_id"),
        "framework_version" => ("framework_version_id", "framework_version_id"),
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    conn.execute(
        &format!(
            "INSERT INTO acquisition_provenance_events
                (id, {}, event_kind, actor_id, occurred_at_us, reason, before_json, after_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            columns.0
        ),
        params![
            Uuid::now_v7().to_string(),
            target_id,
            event_kind,
            actor_id,
            occurred_at_us,
            reason,
            before_json,
            after_json
        ],
    )?;
    Ok(())
}

fn map_source_version(row: &Row<'_>) -> RepoResult<SourceVersionDto> {
    let capture_kind: String = row.get(2)?;
    Ok(SourceVersionDto {
        id: row.get(0)?,
        source_id: row.get(1)?,
        capture_kind: SourceCaptureKind::parse(&capture_kind)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        edition_label: row.get(3)?,
        canonical_uri: row.get(4)?,
        captured_at_us: row.get(5)?,
        content_sha256: row.get(6)?,
        external_asset_ref: row.get(7)?,
        metadata_json: row.get(8)?,
        created_by: row.get(9)?,
        created_at_us: row.get(10)?,
    })
}

const SOURCE_VERSION_COLUMNS: &str =
    "id, source_id, capture_kind, edition_label, canonical_uri, captured_at_us,
     content_sha256, external_asset_ref, metadata_json, created_by, created_at_us";

pub fn insert_source_version(conn: &Connection, dto: &SourceVersionDto) -> RepoResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO source_versions
            (id, source_id, capture_kind, edition_label, canonical_uri, captured_at_us,
             content_sha256, external_asset_ref, metadata_json, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            dto.id,
            dto.source_id,
            dto.capture_kind.as_str(),
            dto.edition_label,
            dto.canonical_uri,
            dto.captured_at_us,
            dto.content_sha256,
            dto.external_asset_ref,
            dto.metadata_json,
            dto.created_by,
            dto.created_at_us
        ],
    )?;
    let after = serde_json::to_string(dto).map_err(|_| rusqlite::Error::InvalidQuery)?;
    record_event(
        &tx,
        "source_version",
        &dto.id,
        "created",
        Some(&dto.created_by),
        dto.created_at_us,
        "source version captured",
        None,
        Some(&after),
    )?;
    tx.commit()
}

pub fn list_source_versions(
    conn: &Connection,
    source_id: &str,
) -> RepoResult<Vec<SourceVersionDto>> {
    let mut statement = conn.prepare(&format!(
        "SELECT {SOURCE_VERSION_COLUMNS} FROM source_versions
         WHERE source_id = ?1 ORDER BY captured_at_us DESC, id"
    ))?;
    let rows = statement
        .query_map([source_id], map_source_version)?
        .collect();
    rows
}

fn map_source_location(row: &Row<'_>) -> RepoResult<SourceLocationDto> {
    let kind: String = row.get(5)?;
    Ok(SourceLocationDto {
        id: row.get(0)?,
        source_id: row.get(1)?,
        source_version_id: row.get(2)?,
        legacy_segment_id: row.get(3)?,
        label: row.get(4)?,
        location_kind: SourceLocationKind::parse(&kind).map_err(|_| rusqlite::Error::InvalidQuery)?,
        page_start: row.get(6)?,
        page_end: row.get(7)?,
        paragraph_start: row.get(8)?,
        paragraph_end: row.get(9)?,
        line_start: row.get(10)?,
        line_end: row.get(11)?,
        time_start_ms: row.get(12)?,
        time_end_ms: row.get(13)?,
        stable_selector: row.get(14)?,
        locator_text: row.get(15)?,
        notes: row.get(16)?,
        parent_segment_id: row.get(17)?,
        order_index: row.get(18)?,
        created_at_us: row.get(19)?,
    })
}

const SOURCE_LOCATION_COLUMNS: &str =
    "id, source_id, source_version_id, legacy_segment_id, label, location_kind,
     page_start, page_end, paragraph_start, paragraph_end, line_start, line_end,
     time_start_ms, time_end_ms, stable_selector, locator_text, notes,
     parent_segment_id, order_index, created_at_us";

pub fn insert_source_location(conn: &Connection, dto: &SourceLocationDto) -> RepoResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO source_version_segments
            (id, source_id, source_version_id, legacy_segment_id, label, location_kind,
             page_start, page_end, paragraph_start, paragraph_end, line_start, line_end,
             time_start_ms, time_end_ms, stable_selector, locator_text, notes,
             parent_segment_id, order_index, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                 ?15, ?16, ?17, ?18, ?19, ?20)",
        params![
            dto.id,
            dto.source_id,
            dto.source_version_id,
            dto.legacy_segment_id,
            dto.label,
            dto.location_kind.as_str(),
            dto.page_start,
            dto.page_end,
            dto.paragraph_start,
            dto.paragraph_end,
            dto.line_start,
            dto.line_end,
            dto.time_start_ms,
            dto.time_end_ms,
            dto.stable_selector,
            dto.locator_text,
            dto.notes,
            dto.parent_segment_id,
            dto.order_index,
            dto.created_at_us
        ],
    )?;
    let after = serde_json::to_string(dto).map_err(|_| rusqlite::Error::InvalidQuery)?;
    record_event(
        &tx,
        "source_location",
        &dto.id,
        "created",
        None,
        dto.created_at_us,
        "version-specific source location created",
        None,
        Some(&after),
    )?;
    tx.commit()
}

pub fn list_source_locations(
    conn: &Connection,
    source_version_id: &str,
) -> RepoResult<Vec<SourceLocationDto>> {
    let mut statement = conn.prepare(&format!(
        "SELECT {SOURCE_LOCATION_COLUMNS} FROM source_version_segments
         WHERE source_version_id = ?1 ORDER BY order_index, id"
    ))?;
    let rows = statement
        .query_map([source_version_id], map_source_location)?
        .collect();
    rows
}

fn map_extraction(row: &Row<'_>) -> RepoResult<KnowledgeExtractionDto> {
    let kind: String = row.get(4)?;
    let review: String = row.get(13)?;
    Ok(KnowledgeExtractionDto {
        id: row.get(0)?,
        source_id: row.get(1)?,
        source_version_id: row.get(2)?,
        source_segment_id: row.get(3)?,
        kind: ExtractionKind::parse(&kind).map_err(|_| rusqlite::Error::InvalidQuery)?,
        content: row.get(5)?,
        language: row.get(6)?,
        parent_extraction_id: row.get(7)?,
        tool_identity: row.get(8)?,
        tool_version: row.get(9)?,
        prompt_config_identity: row.get(10)?,
        confidence_decimal: row.get(11)?,
        uncertainty: row.get(12)?,
        review_status: ExtractionReviewStatus::parse(&review)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        superseded_by_id: row.get(14)?,
        created_by: row.get(15)?,
        created_at_us: row.get(16)?,
    })
}

const EXTRACTION_SELECT: &str = "
    SELECT e.id, e.source_id, e.source_version_id, e.source_segment_id, e.kind, e.content,
           e.language, e.parent_extraction_id, e.tool_identity, e.tool_version,
           e.prompt_config_identity, e.confidence_decimal, e.uncertainty, e.review_status,
           (SELECT s.replacement_extraction_id FROM extraction_supersessions s
             WHERE s.prior_extraction_id = e.id),
           e.created_by, e.created_at_us
    FROM knowledge_extractions e";

pub fn insert_extraction(conn: &Connection, dto: &KnowledgeExtractionDto) -> RepoResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO knowledge_extractions
            (id, source_id, source_version_id, source_segment_id, kind, content, language,
             parent_extraction_id, tool_identity, tool_version, prompt_config_identity,
             confidence_decimal, uncertainty, review_status, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'unreviewed', ?14, ?15)",
        params![
            dto.id,
            dto.source_id,
            dto.source_version_id,
            dto.source_segment_id,
            dto.kind.as_str(),
            dto.content,
            dto.language,
            dto.parent_extraction_id,
            dto.tool_identity,
            dto.tool_version,
            dto.prompt_config_identity,
            dto.confidence_decimal,
            dto.uncertainty,
            dto.created_by,
            dto.created_at_us
        ],
    )?;
    let after = serde_json::to_string(dto).map_err(|_| rusqlite::Error::InvalidQuery)?;
    record_event(
        &tx,
        "extraction",
        &dto.id,
        "created",
        Some(&dto.created_by),
        dto.created_at_us,
        "source extraction created; review pending",
        None,
        Some(&after),
    )?;
    tx.commit()
}

pub fn list_extractions(
    conn: &Connection,
    source_version_id: &str,
) -> RepoResult<Vec<KnowledgeExtractionDto>> {
    let mut statement = conn.prepare(&format!(
        "{EXTRACTION_SELECT} WHERE e.source_version_id = ?1 ORDER BY e.created_at_us, e.id"
    ))?;
    let rows = statement
        .query_map([source_version_id], map_extraction)?
        .collect();
    rows
}

pub fn review_extraction(
    conn: &Connection,
    extraction_id: &str,
    review_status: &str,
    reviewer_id: Option<&str>,
    rationale: &str,
    now: i64,
) -> RepoResult<()> {
    let tx = conn.unchecked_transaction()?;
    let before = get_extraction(&tx, extraction_id)?;
    tx.execute(
        "INSERT INTO extraction_reviews
            (id, extraction_id, review_status, reviewer_id, reviewed_at_us, rationale)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![Uuid::now_v7().to_string(), extraction_id, review_status, reviewer_id, now, rationale],
    )?;
    let after = get_extraction(&tx, extraction_id)?;
    let before_json = serde_json::to_string(&before).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let after_json = serde_json::to_string(&after).map_err(|_| rusqlite::Error::InvalidQuery)?;
    record_event(
        &tx,
        "extraction",
        extraction_id,
        "reviewed",
        reviewer_id,
        now,
        rationale,
        Some(&before_json),
        Some(&after_json),
    )?;
    tx.commit()
}

fn get_extraction(conn: &Connection, id: &str) -> RepoResult<KnowledgeExtractionDto> {
    conn.query_row(
        &format!("{EXTRACTION_SELECT} WHERE e.id = ?1"),
        [id],
        map_extraction,
    )
}

pub fn correct_extraction(
    conn: &Connection,
    extraction_id: &str,
    content: &str,
    reason: &str,
    actor_id: Option<&str>,
    now: i64,
    replacement_id: &str,
) -> RepoResult<KnowledgeExtractionDto> {
    let tx = conn.unchecked_transaction()?;
    let prior = get_extraction(&tx, extraction_id)?;
    let replacement = KnowledgeExtractionDto {
        id: replacement_id.to_string(),
        source_id: prior.source_id.clone(),
        source_version_id: prior.source_version_id.clone(),
        source_segment_id: prior.source_segment_id.clone(),
        kind: prior.kind.clone(),
        content: content.to_string(),
        language: prior.language.clone(),
        parent_extraction_id: Some(prior.id.clone()),
        tool_identity: prior.tool_identity.clone(),
        tool_version: prior.tool_version.clone(),
        prompt_config_identity: prior.prompt_config_identity.clone(),
        confidence_decimal: prior.confidence_decimal.clone(),
        uncertainty: prior.uncertainty.clone(),
        review_status: ExtractionReviewStatus::Unreviewed,
        superseded_by_id: None,
        created_by: actor_id.unwrap_or("unknown").to_string(),
        created_at_us: now,
    };
    tx.execute(
        "INSERT INTO knowledge_extractions
            (id, source_id, source_version_id, source_segment_id, kind, content, language,
             parent_extraction_id, tool_identity, tool_version, prompt_config_identity,
             confidence_decimal, uncertainty, review_status, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'unreviewed', ?14, ?15)",
        params![
            replacement.id,
            replacement.source_id,
            replacement.source_version_id,
            replacement.source_segment_id,
            replacement.kind.as_str(),
            replacement.content,
            replacement.language,
            replacement.parent_extraction_id,
            replacement.tool_identity,
            replacement.tool_version,
            replacement.prompt_config_identity,
            replacement.confidence_decimal,
            replacement.uncertainty,
            replacement.created_by,
            replacement.created_at_us
        ],
    )?;
    tx.execute(
        "INSERT INTO extraction_supersessions
            (id, prior_extraction_id, replacement_extraction_id, reason, actor_id, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            Uuid::now_v7().to_string(),
            prior.id,
            replacement.id,
            reason,
            actor_id,
            now
        ],
    )?;
    let before_json = serde_json::to_string(&prior).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let after_json =
        serde_json::to_string(&replacement).map_err(|_| rusqlite::Error::InvalidQuery)?;
    record_event(
        &tx,
        "extraction",
        extraction_id,
        "superseded",
        actor_id,
        now,
        reason,
        Some(&before_json),
        Some(&after_json),
    )?;
    record_event(
        &tx,
        "extraction",
        &replacement.id,
        "corrected",
        actor_id,
        now,
        reason,
        Some(&before_json),
        Some(&after_json),
    )?;
    tx.commit()?;
    Ok(replacement)
}

pub fn create_source_claim(
    conn: &Connection,
    id: &str,
    extraction_id: &str,
    statement: &str,
    asserted_by: Option<&str>,
    scope: Option<&str>,
    now: i64,
) -> RepoResult<SourceClaimDto> {
    let tx = conn.unchecked_transaction()?;
    let extraction = get_extraction(&tx, extraction_id)?;
    tx.execute(
        "INSERT INTO source_claims
            (id, source_id, source_version_id, extraction_id, statement, asserted_by, scope, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            extraction.source_id,
            extraction.source_version_id,
            extraction.id,
            statement,
            asserted_by,
            scope,
            now
        ],
    )?;
    let dto = SourceClaimDto {
        id: id.to_string(),
        source_id: extraction.source_id,
        source_version_id: extraction.source_version_id,
        extraction_id: extraction.id,
        statement: statement.to_string(),
        asserted_by: asserted_by.map(str::to_string),
        scope: scope.map(str::to_string),
        created_at_us: now,
    };
    let after = serde_json::to_string(&dto).map_err(|_| rusqlite::Error::InvalidQuery)?;
    record_event(
        &tx,
        "source_claim",
        &dto.id,
        "source_claim_created",
        asserted_by,
        now,
        "source-attributed assertion; extraction review is not truth validation",
        None,
        Some(&after),
    )?;
    tx.commit()?;
    Ok(dto)
}

pub fn create_knowledge_item(
    conn: &Connection,
    id: &str,
    req: &CreateKnowledgeItemReq,
    actor_id: Option<&str>,
    now: i64,
) -> RepoResult<KnowledgeItemDto> {
    let tx = conn.unchecked_transaction()?;
    let (table, relation_table, content_column, relation_kind) = match req.kind {
        KnowledgeItemKind::Concept => (
            "knowledge_concepts",
            "source_claim_concept_relations",
            "definition",
            ClaimRelationKind::Defines,
        ),
        KnowledgeItemKind::Principle => (
            "knowledge_principles",
            "source_claim_principle_relations",
            "statement",
            ClaimRelationKind::Supports,
        ),
        KnowledgeItemKind::Procedure => (
            "knowledge_procedures",
            "source_claim_procedure_relations",
            "description",
            ClaimRelationKind::Describes,
        ),
        KnowledgeItemKind::SourceClaim => return Err(rusqlite::Error::InvalidQuery),
    };
    tx.execute(
        &format!("INSERT INTO {table} (id, title, {content_column}, created_at_us) VALUES (?1, ?2, ?3, ?4)"),
        params![id, req.title, req.content, now],
    )?;
    tx.execute(
        &format!(
            "INSERT INTO {relation_table}
                (id, source_claim_id, {}, relation_kind, rationale, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, '', ?5, ?6)",
            match req.kind {
                KnowledgeItemKind::Concept => "concept_id",
                KnowledgeItemKind::Principle => "principle_id",
                KnowledgeItemKind::Procedure => "procedure_id",
                KnowledgeItemKind::SourceClaim => return Err(rusqlite::Error::InvalidQuery),
            }
        ),
        params![
            Uuid::now_v7().to_string(),
            req.source_claim_id,
            id,
            relation_kind.as_str(),
            actor_id,
            now
        ],
    )?;
    let dto = KnowledgeItemDto {
        id: id.to_string(),
        kind: req.kind.clone(),
        title: req.title.clone(),
        content: req.content.clone(),
        lifecycle: "proposed".to_string(),
        source_claim_id: req.source_claim_id.clone(),
        relation_kind,
        created_at_us: now,
    };
    let after = serde_json::to_string(&dto).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let target_kind = match req.kind {
        KnowledgeItemKind::Concept => "concept",
        KnowledgeItemKind::Principle => "principle",
        KnowledgeItemKind::Procedure => "procedure",
        KnowledgeItemKind::SourceClaim => return Err(rusqlite::Error::InvalidQuery),
    };
    record_event(
        &tx,
        target_kind,
        id,
        "created",
        actor_id,
        now,
        "knowledge item created with source-claim provenance",
        None,
        Some(&after),
    )?;
    tx.commit()?;
    Ok(dto)
}

pub fn create_claim_relation(
    conn: &Connection,
    req: &CreateClaimRelationReq,
    id: &str,
    now: i64,
) -> RepoResult<String> {
    let table_and_column = match (&req.target_kind, &req.relation_kind) {
        (
            KnowledgeItemKind::SourceClaim,
            ClaimRelationKind::Supports | ClaimRelationKind::Contradicts | ClaimRelationKind::Qualifies,
        ) => ("source_claim_relations", "subject_claim_id", "object_claim_id"),
        (KnowledgeItemKind::Concept, ClaimRelationKind::Defines | ClaimRelationKind::Supports | ClaimRelationKind::Contradicts | ClaimRelationKind::Qualifies) =>
            ("source_claim_concept_relations", "source_claim_id", "concept_id"),
        (KnowledgeItemKind::Principle, ClaimRelationKind::Supports | ClaimRelationKind::Contradicts | ClaimRelationKind::Qualifies) =>
            ("source_claim_principle_relations", "source_claim_id", "principle_id"),
        (KnowledgeItemKind::Procedure, ClaimRelationKind::Describes | ClaimRelationKind::Supports | ClaimRelationKind::Contradicts | ClaimRelationKind::Qualifies) =>
            ("source_claim_procedure_relations", "source_claim_id", "procedure_id"),
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        &format!(
            "INSERT INTO {} (id, {}, {}, relation_kind, rationale, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            table_and_column.0, table_and_column.1, table_and_column.2
        ),
        params![
            id,
            req.source_claim_id,
            req.target_id,
            req.relation_kind.as_str(),
            req.rationale.as_deref().unwrap_or_default(),
            req.created_by,
            now
        ],
    )?;
    tx.commit()?;
    Ok(id.to_string())
}

pub fn list_claim_relations(
    conn: &Connection,
    source_claim_id: &str,
) -> RepoResult<Vec<serde_json::Value>> {
    let mut statement = conn.prepare(
        "SELECT 'source_claim' AS target_kind, object_claim_id AS target_id, relation_kind, rationale
           FROM source_claim_relations WHERE subject_claim_id = ?1
         UNION ALL
         SELECT 'concept', concept_id, relation_kind, rationale
           FROM source_claim_concept_relations WHERE source_claim_id = ?1
         UNION ALL
         SELECT 'principle', principle_id, relation_kind, rationale
           FROM source_claim_principle_relations WHERE source_claim_id = ?1
         UNION ALL
         SELECT 'procedure', procedure_id, relation_kind, rationale
           FROM source_claim_procedure_relations WHERE source_claim_id = ?1
         ORDER BY target_kind, target_id, relation_kind",
    )?;
    let rows = statement
        .query_map([source_claim_id], |row| {
            Ok(serde_json::json!({
                "targetKind": row.get::<_, String>(0)?,
                "targetId": row.get::<_, String>(1)?,
                "relationKind": row.get::<_, String>(2)?,
                "rationale": row.get::<_, String>(3)?
            }))
        })?
        .collect();
    rows
}

pub fn create_framework(
    conn: &Connection,
    framework_id: &str,
    version_id: &str,
    req: &CreateFrameworkReq,
    now: i64,
) -> RepoResult<FrameworkDto> {
    let tx = conn.unchecked_transaction()?;
    let created_by = req.created_by.as_deref();
    tx.execute(
        "INSERT INTO knowledge_frameworks
            (id, title, description, lifecycle, current_version_id, created_by, created_at_us)
         VALUES (?1, ?2, ?3, 'draft', NULL, ?4, ?5)",
        params![
            framework_id,
            req.title,
            req.description.as_deref().unwrap_or_default(),
            created_by,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO knowledge_framework_versions
            (id, framework_id, version_no, parent_version_id, title, description, status,
             change_class, change_rationale, created_by, created_at_us, published_at_us)
         VALUES (?1, ?2, 1, NULL, ?3, ?4, 'draft', NULL, NULL, ?5, ?6, NULL)",
        params![
            version_id,
            framework_id,
            req.title,
            req.description.as_deref().unwrap_or_default(),
            created_by,
            now
        ],
    )?;
    record_event(
        &tx,
        "framework",
        framework_id,
        "created",
        created_by,
        now,
        "framework identity created",
        None,
        Some(version_id),
    )?;
    record_event(
        &tx,
        "framework_version",
        version_id,
        "draft_created",
        created_by,
        now,
        "initial draft framework version",
        None,
        None,
    )?;
    let dto = FrameworkDto {
        id: framework_id.to_string(),
        title: req.title.clone(),
        description: req.description.clone().unwrap_or_default(),
        lifecycle: "draft".to_string(),
        current_version_id: None,
        created_by: created_by.map(str::to_string),
        created_at_us: now,
    };
    tx.commit()?;
    Ok(dto)
}

pub fn list_frameworks(conn: &Connection) -> RepoResult<Vec<FrameworkDto>> {
    let mut statement = conn.prepare(
        "SELECT id, title, description, lifecycle, current_version_id, created_by, created_at_us
         FROM knowledge_frameworks ORDER BY created_at_us DESC, id",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(FrameworkDto {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                lifecycle: row.get(3)?,
                current_version_id: row.get(4)?,
                created_by: row.get(5)?,
                created_at_us: row.get(6)?,
            })
        })?
        .collect();
    rows
}

fn load_framework_items(
    conn: &Connection,
    version_id: &str,
) -> RepoResult<Vec<FrameworkVersionItemDto>> {
    let mut statement = conn.prepare(
        "SELECT i.id, i.framework_version_id, i.item_kind, i.source_claim_id, i.concept_id,
                i.principle_id, i.procedure_id, i.membership_role, i.item_order, i.rationale,
                COALESCE(c.statement, co.title, p.title, pr.title)
         FROM framework_version_items i
         LEFT JOIN source_claims c ON c.id = i.source_claim_id
         LEFT JOIN knowledge_concepts co ON co.id = i.concept_id
         LEFT JOIN knowledge_principles p ON p.id = i.principle_id
         LEFT JOIN knowledge_procedures pr ON pr.id = i.procedure_id
         WHERE i.framework_version_id = ?1
         ORDER BY i.item_order, i.id",
    )?;
    let rows = statement.query_map([version_id], |row| {
        let kind: String = row.get(2)?;
        let id = row
            .get::<_, Option<String>>(3)?
            .or(row.get::<_, Option<String>>(4)?)
            .or(row.get::<_, Option<String>>(5)?)
            .or(row.get::<_, Option<String>>(6)?)
            .ok_or(rusqlite::Error::InvalidQuery)?;
        Ok(FrameworkVersionItemDto {
            id: row.get(0)?,
            framework_version_id: row.get(1)?,
            item_kind: KnowledgeItemKind::parse(&kind)
                .map_err(|_| rusqlite::Error::InvalidQuery)?,
            item_id: id,
            membership_role: row.get(7)?,
            item_order: row.get(8)?,
            rationale: row.get(9)?,
            item_title: row.get(10)?,
        })
    })?;
    rows.collect()
}

fn map_framework_version(conn: &Connection, row: &Row<'_>) -> RepoResult<FrameworkVersionDto> {
    let id: String = row.get(0)?;
    Ok(FrameworkVersionDto {
        items: load_framework_items(conn, &id)?,
        id,
        framework_id: row.get(1)?,
        version_no: row.get(2)?,
        parent_version_id: row.get(3)?,
        title: row.get(4)?,
        description: row.get(5)?,
        status: row.get(6)?,
        change_class: row.get(7)?,
        change_rationale: row.get(8)?,
        created_by: row.get(9)?,
        created_at_us: row.get(10)?,
        published_at_us: row.get(11)?,
    })
}

const FRAMEWORK_VERSION_COLUMNS: &str =
    "id, framework_id, version_no, parent_version_id, title, description, status,
     change_class, change_rationale, created_by, created_at_us, published_at_us";

pub fn list_framework_versions(
    conn: &Connection,
    framework_id: &str,
) -> RepoResult<Vec<FrameworkVersionDto>> {
    let mut statement = conn.prepare(&format!(
        "SELECT {FRAMEWORK_VERSION_COLUMNS} FROM knowledge_framework_versions
         WHERE framework_id = ?1 ORDER BY version_no DESC"
    ))?;
    let rows = statement.query_map([framework_id], |row| {
        map_framework_version(conn, row)
    })?;
    let versions = rows.collect();
    versions
}

pub fn create_framework_version(
    conn: &Connection,
    req: &CreateFrameworkVersionReq,
    version_id: &str,
    now: i64,
) -> RepoResult<FrameworkVersionDto> {
    if !matches!(
        req.change_class.as_str(),
        "refinement" | "extension" | "revision" | "replacement"
    ) {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let tx = conn.unchecked_transaction()?;
    let parent = tx
        .query_row(
            "SELECT id, version_no, title, description, status
             FROM knowledge_framework_versions
             WHERE framework_id = ?1 ORDER BY version_no DESC LIMIT 1",
            [&req.framework_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?
        .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    if parent.4 != "published" {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let version_no = parent.1 + 1;
    tx.execute(
        "INSERT INTO knowledge_framework_versions
            (id, framework_id, version_no, parent_version_id, title, description, status,
             change_class, change_rationale, created_by, created_at_us, published_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'draft', ?7, ?8, ?9, ?10, NULL)",
        params![
            version_id,
            req.framework_id,
            version_no,
            parent.0,
            parent.2,
            parent.3,
            req.change_class,
            req.change_rationale,
            req.created_by,
            now
        ],
    )?;
    let parent_items = {
        let mut statement = tx.prepare(
            "SELECT item_kind, source_claim_id, concept_id, principle_id, procedure_id,
                    membership_role, item_order, rationale
             FROM framework_version_items WHERE framework_version_id = ?1 ORDER BY item_order",
        )?;
        let rows = statement
            .query_map([&parent.0], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    for (kind, claim, concept, principle, procedure, role, order, rationale) in parent_items {
        tx.execute(
            "INSERT INTO framework_version_items
                (id, framework_version_id, item_kind, source_claim_id, concept_id,
                 principle_id, procedure_id, membership_role, item_order, rationale, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                Uuid::now_v7().to_string(),
                version_id,
                kind,
                claim,
                concept,
                principle,
                procedure,
                role,
                order,
                rationale,
                now
            ],
        )?;
    }
    for (claim_id, link_kind) in req
        .affected_claim_ids
        .iter()
        .map(|id| (id, "affected_claim"))
        .chain(req.assumption_claim_ids.iter().map(|id| (id, "assumption")))
    {
        tx.execute(
            "INSERT INTO framework_version_change_links
                (id, framework_version_id, source_claim_id, link_kind, rationale)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                Uuid::now_v7().to_string(),
                version_id,
                claim_id,
                link_kind,
                req.change_rationale
            ],
        )?;
    }
    record_event(
        &tx,
        "framework_version",
        version_id,
        "version_created",
        req.created_by.as_deref(),
        now,
        &req.change_rationale,
        Some(&parent.0),
        None,
    )?;
    let items = load_framework_items(&tx, version_id)?;
    let dto = FrameworkVersionDto {
        id: version_id.to_string(),
        framework_id: req.framework_id.clone(),
        version_no,
        parent_version_id: Some(parent.0),
        title: parent.2,
        description: parent.3,
        status: "draft".to_string(),
        change_class: Some(req.change_class.clone()),
        change_rationale: Some(req.change_rationale.clone()),
        created_by: req.created_by.clone(),
        created_at_us: now,
        published_at_us: None,
        items,
    };
    tx.commit()?;
    Ok(dto)
}

pub fn add_framework_item(
    conn: &Connection,
    req: &AddFrameworkItemReq,
    item_id: &str,
    now: i64,
) -> RepoResult<String> {
    let tx = conn.unchecked_transaction()?;
    let status: String = tx.query_row(
        "SELECT status FROM knowledge_framework_versions WHERE id = ?1",
        [&req.framework_version_id],
        |row| row.get(0),
    )?;
    if status != "draft" {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let fk_columns = match req.item_kind {
        KnowledgeItemKind::Concept => ("concept_id", "concept"),
        KnowledgeItemKind::Principle => ("principle_id", "principle"),
        KnowledgeItemKind::Procedure => ("procedure_id", "procedure"),
        KnowledgeItemKind::SourceClaim => ("source_claim_id", "source_claim"),
    };
    tx.execute(
        &format!(
            "INSERT INTO framework_version_items
                (id, framework_version_id, item_kind, {}, membership_role, item_order, rationale, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            fk_columns.0
        ),
        params![
            item_id,
            req.framework_version_id,
            fk_columns.1,
            req.item_id,
            req.membership_role,
            req.item_order,
            req.rationale.as_deref().unwrap_or_default(),
            now
        ],
    )?;
    record_event(
        &tx,
        "framework_version",
        &req.framework_version_id,
        "item_added",
        None,
        now,
        req.rationale.as_deref().unwrap_or_default(),
        None,
        Some(item_id),
    )?;
    tx.commit()?;
    Ok(item_id.to_string())
}

pub fn remove_framework_item(conn: &Connection, item_id: &str) -> RepoResult<()> {
    let tx = conn.unchecked_transaction()?;
    let version_id: String = tx.query_row(
        "SELECT framework_version_id FROM framework_version_items WHERE id = ?1",
        [item_id],
        |row| row.get(0),
    )?;
    let status: String = tx.query_row(
        "SELECT status FROM knowledge_framework_versions WHERE id = ?1",
        [&version_id],
        |row| row.get(0),
    )?;
    if status != "draft" {
        return Err(rusqlite::Error::InvalidQuery);
    }
    tx.execute("DELETE FROM framework_version_items WHERE id = ?1", [item_id])?;
    record_event(
        &tx,
        "framework_version",
        &version_id,
        "item_removed",
        None,
        super::super::db::now_ts_us(),
        "draft framework item removed",
        Some(item_id),
        None,
    )?;
    tx.commit()
}

pub fn update_framework_version(
    conn: &Connection,
    req: &UpdateFrameworkVersionReq,
    now: i64,
) -> RepoResult<()> {
    let tx = conn.unchecked_transaction()?;
    let before: (String, String, String) = tx.query_row(
        "SELECT framework_id, title, description FROM knowledge_framework_versions
         WHERE id = ?1 AND status = 'draft'",
        [&req.framework_version_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    tx.execute(
        "UPDATE knowledge_framework_versions SET title = ?1, description = ?2
         WHERE id = ?3 AND status = 'draft'",
        params![req.title, req.description, req.framework_version_id],
    )?;
    let before_json = serde_json::to_string(&serde_json::json!({
        "title": before.1,
        "description": before.2
    }))
    .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let after_json = serde_json::to_string(&serde_json::json!({
        "title": req.title,
        "description": req.description
    }))
    .map_err(|_| rusqlite::Error::InvalidQuery)?;
    record_event(
        &tx,
        "framework_version",
        &req.framework_version_id,
        "draft_updated",
        None,
        now,
        "framework draft edited",
        Some(&before_json),
        Some(&after_json),
    )?;
    tx.commit()
}

pub fn review_framework_version(
    conn: &Connection,
    req: &ReviewFrameworkReq,
    review_id: &str,
    now: i64,
) -> RepoResult<()> {
    let tx = conn.unchecked_transaction()?;
    let status: String = tx.query_row(
        "SELECT status FROM knowledge_framework_versions WHERE id = ?1",
        [&req.framework_version_id],
        |row| row.get(0),
    )?;
    if !matches!(status.as_str(), "draft" | "review") {
        return Err(rusqlite::Error::InvalidQuery);
    }
    tx.execute(
        "INSERT INTO framework_version_reviews
            (id, framework_version_id, review_status, reviewer_id, rationale, reviewed_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            review_id,
            req.framework_version_id,
            req.review_status,
            req.reviewer_id,
            req.rationale.as_deref().unwrap_or_default(),
            now
        ],
    )?;
    record_event(
        &tx,
        "framework_version",
        &req.framework_version_id,
        "reviewed",
        req.reviewer_id.as_deref(),
        now,
        req.rationale.as_deref().unwrap_or_default(),
        Some(&status),
        Some(&req.review_status),
    )?;
    tx.commit()
}

pub fn publish_framework_version(
    conn: &Connection,
    version_id: &str,
    now: i64,
) -> RepoResult<()> {
    let tx = conn.unchecked_transaction()?;
    let version: (String, i64, String) = tx.query_row(
        "SELECT framework_id, version_no, status
         FROM knowledge_framework_versions WHERE id = ?1",
        [version_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    if !matches!(version.2.as_str(), "draft" | "review") {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let latest_review: Option<String> = tx
        .query_row(
            "SELECT review_status FROM framework_version_reviews
             WHERE framework_version_id = ?1 ORDER BY reviewed_at_us DESC, id DESC LIMIT 1",
            [version_id],
            |row| row.get(0),
        )
        .optional()?;
    if latest_review.as_deref() != Some("approved") {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let item_count: i64 = tx.query_row(
        "SELECT COUNT(*) FROM framework_version_items WHERE framework_version_id = ?1",
        [version_id],
        |row| row.get(0),
    )?;
    if item_count == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    tx.execute(
        "UPDATE knowledge_framework_versions
         SET status = 'published', published_at_us = ?1 WHERE id = ?2",
        params![now, version_id],
    )?;
    record_event(
        &tx,
        "framework_version",
        version_id,
        "published",
        None,
        now,
        "approved immutable framework snapshot published",
        Some(&version.2),
        Some("published"),
    )?;
    tx.commit()
}

pub fn list_provenance(
    conn: &Connection,
    target_kind: &str,
    target_id: &str,
) -> RepoResult<Vec<ProvenanceEventDto>> {
    let (column, sql_kind) = match target_kind {
        "source" => ("source_id", "source"),
        "source_version" => ("source_version_id", "source_version"),
        "source_location" => ("source_segment_id", "source_location"),
        "extraction" => ("extraction_id", "extraction"),
        "source_claim" => ("source_claim_id", "source_claim"),
        "concept" => ("concept_id", "concept"),
        "principle" => ("principle_id", "principle"),
        "procedure" => ("procedure_id", "procedure"),
        "framework" => ("framework_id", "framework"),
        "framework_version" => ("framework_version_id", "framework_version"),
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    let mut statement = conn.prepare(&format!(
        "SELECT id, '{sql_kind}', {column}, event_kind, actor_id, occurred_at_us,
                reason, before_json, after_json
         FROM acquisition_provenance_events
         WHERE {column} = ?1 ORDER BY occurred_at_us, id"
    ))?;
    let rows = statement
        .query_map([target_id], |row| {
            Ok(ProvenanceEventDto {
                id: row.get(0)?,
                target_kind: row.get(1)?,
                target_id: row.get(2)?,
                event_kind: row.get(3)?,
                actor_id: row.get(4)?,
                occurred_at_us: row.get(5)?,
                reason: row.get(6)?,
                before_json: row.get(7)?,
                after_json: row.get(8)?,
            })
        })?
        .collect();
    rows
}
