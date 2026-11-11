use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::db::now_ts_us;

pub mod repository;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceCaptureKind {
    Edition,
    Retrieval,
}

impl SourceCaptureKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Edition => "edition",
            Self::Retrieval => "retrieval",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "source_claim" => Ok(Self::SourceClaim),
            "edition" => Ok(Self::Edition),
            "retrieval" => Ok(Self::Retrieval),
            _ => Err(format!("invalid source capture kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceLocationKind {
    Page,
    Paragraph,
    Line,
    Timestamp,
    TimeRange,
    TextAnchor,
    Unknown,
}

impl SourceLocationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Paragraph => "paragraph",
            Self::Line => "line",
            Self::Timestamp => "timestamp",
            Self::TimeRange => "time_range",
            Self::TextAnchor => "text_anchor",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "page" => Ok(Self::Page),
            "paragraph" => Ok(Self::Paragraph),
            "line" => Ok(Self::Line),
            "timestamp" => Ok(Self::Timestamp),
            "time_range" => Ok(Self::TimeRange),
            "text_anchor" => Ok(Self::TextAnchor),
            "unknown" => Ok(Self::Unknown),
            _ => Err(format!("invalid source location kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionKind {
    Quote,
    Paraphrase,
    Summary,
    LearnerInterpretation,
    SourceClaim,
}

impl ExtractionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Quote => "quote",
            Self::Paraphrase => "paraphrase",
            Self::Summary => "summary",
            Self::LearnerInterpretation => "learner_interpretation",
            Self::SourceClaim => "source_claim",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "quote" => Ok(Self::Quote),
            "paraphrase" => Ok(Self::Paraphrase),
            "summary" => Ok(Self::Summary),
            "learner_interpretation" => Ok(Self::LearnerInterpretation),
            "source_claim" => Ok(Self::SourceClaim),
            _ => Err(format!("invalid extraction kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionReviewStatus {
    Unreviewed,
    Accepted,
    Rejected,
    NeedsCorrection,
}

impl ExtractionReviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unreviewed => "unreviewed",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::NeedsCorrection => "needs_correction",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "unreviewed" => Ok(Self::Unreviewed),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            "needs_correction" => Ok(Self::NeedsCorrection),
            _ => Err(format!("invalid extraction review status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeItemKind {
    SourceClaim,
    Concept,
    Principle,
    Procedure,
}

impl KnowledgeItemKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SourceClaim => "source_claim",
            Self::Concept => "concept",
            Self::Principle => "principle",
            Self::Procedure => "procedure",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "concept" => Ok(Self::Concept),
            "principle" => Ok(Self::Principle),
            "procedure" => Ok(Self::Procedure),
            _ => Err(format!("invalid knowledge item kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimRelationKind {
    Supports,
    Contradicts,
    Qualifies,
    Defines,
    Describes,
}

impl ClaimRelationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Supports => "supports",
            Self::Contradicts => "contradicts",
            Self::Qualifies => "qualifies",
            Self::Defines => "defines",
            Self::Describes => "describes",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceVersionDto {
    pub id: String,
    pub source_id: String,
    pub capture_kind: SourceCaptureKind,
    pub edition_label: Option<String>,
    pub canonical_uri: Option<String>,
    pub captured_at_us: i64,
    pub content_sha256: Option<String>,
    pub external_asset_ref: Option<String>,
    pub metadata_json: String,
    pub created_by: String,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSourceVersionReq {
    pub source_id: String,
    pub capture_kind: SourceCaptureKind,
    pub edition_label: Option<String>,
    pub canonical_uri: Option<String>,
    pub captured_at_us: Option<i64>,
    pub content_sha256: Option<String>,
    pub external_asset_ref: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLocationDto {
    pub id: String,
    pub source_id: String,
    pub source_version_id: String,
    pub legacy_segment_id: Option<String>,
    pub label: String,
    pub location_kind: SourceLocationKind,
    pub page_start: Option<i64>,
    pub page_end: Option<i64>,
    pub paragraph_start: Option<i64>,
    pub paragraph_end: Option<i64>,
    pub line_start: Option<i64>,
    pub line_end: Option<i64>,
    pub time_start_ms: Option<i64>,
    pub time_end_ms: Option<i64>,
    pub stable_selector: Option<String>,
    pub locator_text: Option<String>,
    pub notes: String,
    pub parent_segment_id: Option<String>,
    pub order_index: i64,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSourceLocationReq {
    pub source_id: String,
    pub source_version_id: String,
    pub legacy_segment_id: Option<String>,
    pub label: String,
    pub location_kind: SourceLocationKind,
    pub page_start: Option<i64>,
    pub page_end: Option<i64>,
    pub paragraph_start: Option<i64>,
    pub paragraph_end: Option<i64>,
    pub line_start: Option<i64>,
    pub line_end: Option<i64>,
    pub time_start_ms: Option<i64>,
    pub time_end_ms: Option<i64>,
    pub stable_selector: Option<String>,
    pub locator_text: Option<String>,
    pub notes: Option<String>,
    pub parent_segment_id: Option<String>,
    pub order_index: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeExtractionDto {
    pub id: String,
    pub source_id: String,
    pub source_version_id: String,
    pub source_segment_id: String,
    pub kind: ExtractionKind,
    pub content: String,
    pub language: Option<String>,
    pub parent_extraction_id: Option<String>,
    pub tool_identity: Option<String>,
    pub tool_version: Option<String>,
    pub prompt_config_identity: Option<String>,
    pub confidence_decimal: Option<String>,
    pub uncertainty: Option<String>,
    pub review_status: ExtractionReviewStatus,
    pub superseded_by_id: Option<String>,
    pub created_by: String,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeExtractionReq {
    pub source_id: String,
    pub source_version_id: String,
    pub source_segment_id: String,
    pub kind: ExtractionKind,
    pub content: String,
    pub language: Option<String>,
    pub parent_extraction_id: Option<String>,
    pub tool_identity: Option<String>,
    pub tool_version: Option<String>,
    pub prompt_config_identity: Option<String>,
    pub confidence_decimal: Option<String>,
    pub uncertainty: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewExtractionReq {
    pub extraction_id: String,
    pub review_status: ExtractionReviewStatus,
    pub reviewer_id: Option<String>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectExtractionReq {
    pub extraction_id: String,
    pub content: String,
    pub reason: String,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceClaimDto {
    pub id: String,
    pub source_id: String,
    pub source_version_id: String,
    pub extraction_id: String,
    pub statement: String,
    pub asserted_by: Option<String>,
    pub scope: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSourceClaimReq {
    pub extraction_id: String,
    pub statement: String,
    pub asserted_by: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeItemDto {
    pub id: String,
    pub kind: KnowledgeItemKind,
    pub title: String,
    pub content: String,
    pub lifecycle: String,
    pub source_claim_id: String,
    pub relation_kind: ClaimRelationKind,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeItemReq {
    pub kind: KnowledgeItemKind,
    pub title: String,
    pub content: String,
    pub source_claim_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateClaimRelationReq {
    pub source_claim_id: String,
    pub target_kind: KnowledgeItemKind,
    pub target_id: String,
    pub relation_kind: ClaimRelationKind,
    pub rationale: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkDto {
    pub id: String,
    pub title: String,
    pub description: String,
    pub lifecycle: String,
    pub current_version_id: Option<String>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkVersionItemDto {
    pub id: String,
    pub framework_version_id: String,
    pub item_kind: KnowledgeItemKind,
    pub item_id: String,
    pub membership_role: String,
    pub item_order: i64,
    pub rationale: String,
    pub item_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkVersionDto {
    pub id: String,
    pub framework_id: String,
    pub version_no: i64,
    pub parent_version_id: Option<String>,
    pub title: String,
    pub description: String,
    pub status: String,
    pub change_class: Option<String>,
    pub change_rationale: Option<String>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub published_at_us: Option<i64>,
    pub items: Vec<FrameworkVersionItemDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFrameworkReq {
    pub title: String,
    pub description: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFrameworkVersionReq {
    pub framework_id: String,
    pub change_class: String,
    pub change_rationale: String,
    pub affected_claim_ids: Vec<String>,
    pub assumption_claim_ids: Vec<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddFrameworkItemReq {
    pub framework_version_id: String,
    pub item_kind: KnowledgeItemKind,
    pub item_id: String,
    pub membership_role: String,
    pub item_order: i64,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFrameworkVersionReq {
    pub framework_version_id: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveFrameworkItemReq {
    pub item_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFrameworkReq {
    pub framework_version_id: String,
    pub review_status: String,
    pub reviewer_id: Option<String>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishFrameworkVersionReq {
    pub framework_version_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceEventDto {
    pub id: String,
    pub target_kind: String,
    pub target_id: String,
    pub event_kind: String,
    pub actor_id: Option<String>,
    pub occurred_at_us: i64,
    pub reason: String,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
}

fn new_id() -> String {
    Uuid::now_v7().to_string()
}

fn validate_required(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    Ok(())
}

fn validate_confidence(value: Option<&str>) -> Result<(), String> {
    if let Some(value) = value {
        let parsed = value
            .parse::<f64>()
            .map_err(|_| "confidence must be a decimal between 0 and 1".to_string())?;
        if !parsed.is_finite() || !(0.0..=1.0).contains(&parsed) {
            return Err("confidence must be a decimal between 0 and 1".to_string());
        }
        if value.trim().is_empty()
            || value
                .bytes()
                .any(|byte| !byte.is_ascii_digit() && byte != b'.')
            || value.bytes().filter(|byte| *byte == b'.').count() > 1
        {
            return Err("confidence must be a non-negative decimal between 0 and 1".to_string());
        }
    }
    Ok(())
}

pub fn create_source_version(
    conn: &Connection,
    req: CreateSourceVersionReq,
    actor_id: Option<&str>,
) -> Result<SourceVersionDto, String> {
    if let Some(checksum) = req.content_sha256.as_deref() {
        if checksum.len() != 64
            || !checksum
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("contentSha256 must be 64 lowercase hexadecimal characters".into());
        }
    }
    let metadata_json = req.metadata_json.unwrap_or_else(|| "{}".into());
    let metadata: serde_json::Value =
        serde_json::from_str(&metadata_json).map_err(|error| error.to_string())?;
    if !metadata.is_object() {
        return Err("metadataJson must be a JSON object".into());
    }
    let now = now_ts_us();
    let dto = SourceVersionDto {
        id: new_id(),
        source_id: req.source_id,
        capture_kind: req.capture_kind,
        edition_label: req.edition_label,
        canonical_uri: req.canonical_uri,
        captured_at_us: req.captured_at_us.unwrap_or(now),
        content_sha256: req.content_sha256,
        external_asset_ref: req.external_asset_ref,
        metadata_json,
        created_by: actor_id.unwrap_or("unknown").to_string(),
        created_at_us: now,
    };
    repository::insert_source_version(conn, &dto).map_err(|error| error.to_string())?;
    Ok(dto)
}

pub fn list_source_versions(
    conn: &Connection,
    source_id: &str,
) -> Result<Vec<SourceVersionDto>, String> {
    repository::list_source_versions(conn, source_id).map_err(|error| error.to_string())
}

pub fn create_source_location(
    conn: &Connection,
    req: CreateSourceLocationReq,
) -> Result<SourceLocationDto, String> {
    validate_required(&req.label, "label")?;
    match req.location_kind {
        SourceLocationKind::Page if req.page_start.is_none() => {
            return Err("page locations require pageStart".into())
        }
        SourceLocationKind::Paragraph if req.paragraph_start.is_none() => {
            return Err("paragraph locations require paragraphStart".into())
        }
        SourceLocationKind::Line if req.line_start.is_none() => {
            return Err("line locations require lineStart".into())
        }
        SourceLocationKind::Timestamp if req.time_start_ms.is_none() => {
            return Err("timestamp locations require timeStartMs".into())
        }
        SourceLocationKind::TimeRange
            if req.time_start_ms.is_none() || req.time_end_ms.is_none() =>
        {
            return Err("time ranges require timeStartMs and timeEndMs".into())
        }
        SourceLocationKind::TextAnchor
            if req
                .stable_selector
                .as_deref()
                .map_or(true, str::is_empty) =>
        {
            return Err("text anchors require a stableSelector".into())
        }
        _ => {}
    }
    let dto = SourceLocationDto {
        id: new_id(),
        source_id: req.source_id,
        source_version_id: req.source_version_id,
        legacy_segment_id: req.legacy_segment_id,
        label: req.label.trim().to_string(),
        location_kind: req.location_kind,
        page_start: req.page_start,
        page_end: req.page_end,
        paragraph_start: req.paragraph_start,
        paragraph_end: req.paragraph_end,
        line_start: req.line_start,
        line_end: req.line_end,
        time_start_ms: req.time_start_ms,
        time_end_ms: req.time_end_ms,
        stable_selector: req.stable_selector,
        locator_text: req.locator_text,
        notes: req.notes.unwrap_or_default(),
        parent_segment_id: req.parent_segment_id,
        order_index: req.order_index.unwrap_or(0),
        created_at_us: now_ts_us(),
    };
    repository::insert_source_location(conn, &dto).map_err(|error| error.to_string())?;
    Ok(dto)
}

pub fn list_source_locations(
    conn: &Connection,
    source_version_id: &str,
) -> Result<Vec<SourceLocationDto>, String> {
    repository::list_source_locations(conn, source_version_id).map_err(|error| error.to_string())
}

pub fn create_extraction(
    conn: &Connection,
    req: CreateKnowledgeExtractionReq,
) -> Result<KnowledgeExtractionDto, String> {
    validate_required(&req.content, "content")?;
    validate_confidence(req.confidence_decimal.as_deref())?;
    let dto = KnowledgeExtractionDto {
        id: new_id(),
        source_id: req.source_id,
        source_version_id: req.source_version_id,
        source_segment_id: req.source_segment_id,
        kind: req.kind,
        content: req.content,
        language: req.language,
        parent_extraction_id: req.parent_extraction_id,
        tool_identity: req.tool_identity,
        tool_version: req.tool_version,
        prompt_config_identity: req.prompt_config_identity,
        confidence_decimal: req.confidence_decimal,
        uncertainty: req.uncertainty,
        review_status: ExtractionReviewStatus::Unreviewed,
        superseded_by_id: None,
        created_by: req.created_by.unwrap_or_else(|| "user".to_string()),
        created_at_us: now_ts_us(),
    };
    repository::insert_extraction(conn, &dto).map_err(|error| error.to_string())?;
    Ok(dto)
}

pub fn list_extractions(
    conn: &Connection,
    source_version_id: &str,
) -> Result<Vec<KnowledgeExtractionDto>, String> {
    repository::list_extractions(conn, source_version_id).map_err(|error| error.to_string())
}

pub fn review_extraction(
    conn: &Connection,
    req: ReviewExtractionReq,
) -> Result<(), String> {
    if req.review_status == ExtractionReviewStatus::Unreviewed {
        return Err("reviewStatus must be a completed review outcome".into());
    }
    repository::review_extraction(
        conn,
        &req.extraction_id,
        req.review_status.as_str(),
        req.reviewer_id.as_deref(),
        req.rationale.as_deref().unwrap_or_default(),
        now_ts_us(),
    )
    .map_err(|error| error.to_string())
}

pub fn correct_extraction(
    conn: &Connection,
    req: CorrectExtractionReq,
) -> Result<KnowledgeExtractionDto, String> {
    validate_required(&req.content, "content")?;
    validate_required(&req.reason, "reason")?;
    let replacement = repository::correct_extraction(
        conn,
        &req.extraction_id,
        &req.content,
        &req.reason,
        req.actor_id.as_deref(),
        now_ts_us(),
        &new_id(),
    )
    .map_err(|error| error.to_string())?;
    Ok(replacement)
}

pub fn create_source_claim(
    conn: &Connection,
    req: CreateSourceClaimReq,
) -> Result<SourceClaimDto, String> {
    validate_required(&req.statement, "statement")?;
    let id = new_id();
    repository::create_source_claim(
        conn,
        &id,
        &req.extraction_id,
        &req.statement,
        req.asserted_by.as_deref(),
        req.scope.as_deref(),
        now_ts_us(),
    )
    .map_err(|error| error.to_string())
}

pub fn create_knowledge_item(
    conn: &Connection,
    req: CreateKnowledgeItemReq,
    actor_id: Option<&str>,
) -> Result<KnowledgeItemDto, String> {
    validate_required(&req.title, "title")?;
    validate_required(&req.content, "content")?;
    if req.kind == KnowledgeItemKind::SourceClaim {
        return Err("source claims must be created from an accepted source-claim extraction".into());
    }
    let id = new_id();
    repository::create_knowledge_item(
        conn,
        &id,
        &req,
        actor_id,
        now_ts_us(),
    )
    .map_err(|error| error.to_string())
}

pub fn create_claim_relation(
    conn: &Connection,
    req: CreateClaimRelationReq,
) -> Result<String, String> {
    if !matches!(
        req.relation_kind,
        ClaimRelationKind::Supports
            | ClaimRelationKind::Contradicts
            | ClaimRelationKind::Qualifies
            | ClaimRelationKind::Defines
            | ClaimRelationKind::Describes
    ) {
        return Err("unsupported source claim relation".into());
    }
    repository::create_claim_relation(conn, &req, &new_id(), now_ts_us())
        .map_err(|error| error.to_string())
}

pub fn list_claim_relations(
    conn: &Connection,
    source_claim_id: &str,
) -> Result<Vec<serde_json::Value>, String> {
    repository::list_claim_relations(conn, source_claim_id).map_err(|error| error.to_string())
}

pub fn create_framework(
    conn: &Connection,
    req: CreateFrameworkReq,
) -> Result<FrameworkDto, String> {
    validate_required(&req.title, "title")?;
    let framework_id = new_id();
    let version_id = new_id();
    repository::create_framework(
        conn,
        &framework_id,
        &version_id,
        &req,
        now_ts_us(),
    )
    .map_err(|error| error.to_string())
}

pub fn list_frameworks(conn: &Connection) -> Result<Vec<FrameworkDto>, String> {
    repository::list_frameworks(conn).map_err(|error| error.to_string())
}

pub fn list_framework_versions(
    conn: &Connection,
    framework_id: &str,
) -> Result<Vec<FrameworkVersionDto>, String> {
    repository::list_framework_versions(conn, framework_id).map_err(|error| error.to_string())
}

pub fn create_framework_version(
    conn: &Connection,
    req: CreateFrameworkVersionReq,
) -> Result<FrameworkVersionDto, String> {
    validate_required(&req.change_class, "changeClass")?;
    validate_required(&req.change_rationale, "changeRationale")?;
    if req.affected_claim_ids.is_empty() && req.assumption_claim_ids.is_empty() {
        return Err("a framework change must link at least one affected claim or assumption".into());
    }
    repository::create_framework_version(conn, &req, &new_id(), now_ts_us())
        .map_err(|error| error.to_string())
}

pub fn add_framework_item(conn: &Connection, req: AddFrameworkItemReq) -> Result<String, String> {
    if req.item_order < 0 {
        return Err("itemOrder must be non-negative".into());
    }
    if !matches!(
        req.membership_role.as_str(),
        "core" | "supporting" | "counterpoint" | "method" | "example" | "context"
    ) {
        return Err(format!("invalid framework membership role: {}", req.membership_role));
    }
    repository::add_framework_item(conn, &req, &new_id(), now_ts_us())
        .map_err(|error| error.to_string())
}

pub fn remove_framework_item(conn: &Connection, item_id: &str) -> Result<(), String> {
    repository::remove_framework_item(conn, item_id).map_err(|error| error.to_string())
}

pub fn update_framework_version(
    conn: &Connection,
    req: UpdateFrameworkVersionReq,
) -> Result<(), String> {
    validate_required(&req.title, "title")?;
    repository::update_framework_version(conn, &req, now_ts_us())
        .map_err(|error| error.to_string())
}

pub fn review_framework_version(
    conn: &Connection,
    req: ReviewFrameworkReq,
) -> Result<(), String> {
    if !matches!(
        req.review_status.as_str(),
        "approved" | "changes_requested" | "rejected"
    ) {
        return Err(format!("invalid framework review status: {}", req.review_status));
    }
    repository::review_framework_version(
        conn,
        &req,
        &new_id(),
        now_ts_us(),
    )
    .map_err(|error| error.to_string())
}

pub fn publish_framework_version(
    conn: &Connection,
    version_id: &str,
) -> Result<(), String> {
    repository::publish_framework_version(conn, version_id, now_ts_us())
        .map_err(|error| error.to_string())
}

pub fn list_provenance(
    conn: &Connection,
    target_kind: &str,
    target_id: &str,
) -> Result<Vec<ProvenanceEventDto>, String> {
    repository::list_provenance(conn, target_kind, target_id).map_err(|error| error.to_string())
}
