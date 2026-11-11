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
            "source_claim" => Ok(Self::SourceClaim),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionProvenanceTargetKind {
    Source,
    SourceVersion,
    SourceLocation,
    Extraction,
    SourceClaim,
    Concept,
    Principle,
    Procedure,
    Framework,
    FrameworkVersion,
}

impl AcquisitionProvenanceTargetKind {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "source" => Ok(Self::Source),
            "source_version" => Ok(Self::SourceVersion),
            "source_location" => Ok(Self::SourceLocation),
            "extraction" => Ok(Self::Extraction),
            "source_claim" => Ok(Self::SourceClaim),
            "concept" => Ok(Self::Concept),
            "principle" => Ok(Self::Principle),
            "procedure" => Ok(Self::Procedure),
            "framework" => Ok(Self::Framework),
            "framework_version" => Ok(Self::FrameworkVersion),
            _ => Err(format!("invalid acquisition provenance target: {value}")),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::SourceVersion => "source_version",
            Self::SourceLocation => "source_location",
            Self::Extraction => "extraction",
            Self::SourceClaim => "source_claim",
            Self::Concept => "concept",
            Self::Principle => "principle",
            Self::Procedure => "procedure",
            Self::Framework => "framework",
            Self::FrameworkVersion => "framework_version",
        }
    }
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

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "supports" => Ok(Self::Supports),
            "contradicts" => Ok(Self::Contradicts),
            "qualifies" => Ok(Self::Qualifies),
            "defines" => Ok(Self::Defines),
            "describes" => Ok(Self::Describes),
            _ => Err(format!("invalid claim relation kind: {value}")),
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
    pub source_claim_id: Option<String>,
    pub relation_kind: Option<ClaimRelationKind>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimRelationDto {
    pub target_kind: KnowledgeItemKind,
    pub target_id: String,
    pub relation_kind: ClaimRelationKind,
    pub rationale: String,
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
pub struct ReorderFrameworkItemsReq {
    pub framework_version_id: String,
    pub ordered_item_ids: Vec<String>,
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
    pub target_kind: AcquisitionProvenanceTargetKind,
    pub target_id: String,
    pub event_kind: String,
    pub actor_id: Option<String>,
    pub occurred_at_us: i64,
    pub reason: String,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkReviewDto {
    pub id: String,
    pub framework_version_id: String,
    pub review_status: String,
    pub reviewer_id: Option<String>,
    pub rationale: String,
    pub reviewed_at_us: i64,
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
) -> Result<Vec<ClaimRelationDto>, String> {
    repository::list_claim_relations(conn, source_claim_id).map_err(|error| error.to_string())
}

pub fn list_source_claims(
    conn: &Connection,
    source_id: &str,
) -> Result<Vec<SourceClaimDto>, String> {
    repository::list_source_claims(conn, source_id).map_err(|error| error.to_string())
}

pub fn list_knowledge_items(conn: &Connection) -> Result<Vec<KnowledgeItemDto>, String> {
    repository::list_knowledge_items(conn).map_err(|error| error.to_string())
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

pub fn reorder_framework_items(
    conn: &Connection,
    req: ReorderFrameworkItemsReq,
) -> Result<(), String> {
    if req.ordered_item_ids.is_empty() {
        return Err("a framework version must retain at least one ordered member".into());
    }
    let unique_ids: std::collections::HashSet<&str> =
        req.ordered_item_ids.iter().map(String::as_str).collect();
    if unique_ids.len() != req.ordered_item_ids.len() {
        return Err("framework item order contains duplicate item IDs".into());
    }
    repository::reorder_framework_items(
        conn,
        &req.framework_version_id,
        &req.ordered_item_ids,
        now_ts_us(),
    )
    .map_err(|error| error.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{
        db::{self, init_schema},
        model::{Source, SourceKind},
    };
    use rusqlite::Connection;

    fn connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&conn).expect("initialize source schema");
        conn
    }

    fn source(conn: &Connection) -> String {
        db::create_source(
            conn,
            Source {
                id: String::new(),
                kind: SourceKind::Doc,
                path: Some("https://example.test/paper".into()),
                title: "Test paper".into(),
                author: Some("A. Author".into()),
                language: Some("en".into()),
                duration_sec: None,
                page_count: Some(12),
                metadata_json: "{}".into(),
                notes: String::new(),
                status: String::new(),
                created_at: 0,
                updated_at: 0,
            },
        )
        .expect("create legacy source")
    }

    fn version(conn: &Connection, source_id: &str, label: &str) -> SourceVersionDto {
        create_source_version(
            conn,
            CreateSourceVersionReq {
                source_id: source_id.into(),
                capture_kind: SourceCaptureKind::Edition,
                edition_label: Some(label.into()),
                canonical_uri: Some("https://example.test/paper".into()),
                captured_at_us: None,
                content_sha256: None,
                external_asset_ref: None,
                metadata_json: None,
            },
            Some("test"),
        )
        .expect("create source version")
    }

    fn location(
        conn: &Connection,
        source_id: &str,
        source_version_id: &str,
    ) -> SourceLocationDto {
        create_source_location(
            conn,
            CreateSourceLocationReq {
                source_id: source_id.into(),
                source_version_id: source_version_id.into(),
                legacy_segment_id: None,
                label: "Page 3, paragraph 2".into(),
                location_kind: SourceLocationKind::Paragraph,
                page_start: Some(3),
                page_end: Some(3),
                paragraph_start: Some(2),
                paragraph_end: Some(2),
                line_start: None,
                line_end: None,
                time_start_ms: None,
                time_end_ms: None,
                stable_selector: Some("page=3&paragraph=2".into()),
                locator_text: Some("An exact cited passage.".into()),
                notes: None,
                parent_segment_id: None,
                order_index: Some(0),
            },
        )
        .expect("create version-specific location")
    }

    fn extraction(
        conn: &Connection,
        source_id: &str,
        source_version_id: &str,
        source_segment_id: &str,
        kind: ExtractionKind,
        content: &str,
    ) -> KnowledgeExtractionDto {
        create_extraction(
            conn,
            CreateKnowledgeExtractionReq {
                source_id: source_id.into(),
                source_version_id: source_version_id.into(),
                source_segment_id: source_segment_id.into(),
                kind,
                content: content.into(),
                language: Some("en".into()),
                parent_extraction_id: None,
                tool_identity: Some("test-tool".into()),
                tool_version: Some("1.0".into()),
                prompt_config_identity: Some("test-prompt".into()),
                confidence_decimal: Some("0.8".into()),
                uncertainty: Some("Manual verification required".into()),
                created_by: Some("machine:test".into()),
            },
        )
        .expect("create extraction")
    }

    fn accept_extraction(extraction_id: &str) -> ReviewExtractionReq {
        ReviewExtractionReq {
            extraction_id: extraction_id.into(),
            review_status: ExtractionReviewStatus::Accepted,
            reviewer_id: Some("reviewer:test".into()),
            rationale: Some("The extraction matches the cited passage.".into()),
        }
    }

    #[test]
    fn source_versions_and_citations_pin_the_exact_version() {
        let conn = connection();
        let source_id = source(&conn);
        let first = version(&conn, &source_id, "edition one");
        let second = version(&conn, &source_id, "edition two");
        let first_location = location(&conn, &source_id, &first.id);
        let second_location = location(&conn, &source_id, &second.id);

        assert_ne!(first.id, second.id);
        assert_eq!(list_source_versions(&conn, &source_id).unwrap().len(), 2);
        assert_eq!(
            list_source_locations(&conn, &first.id).unwrap()[0].id,
            first_location.id
        );
        assert_eq!(
            list_source_locations(&conn, &second.id).unwrap()[0].source_version_id,
            second.id
        );
        assert_eq!(second_location.source_version_id, second.id);

        let extract = extraction(
            &conn,
            &source_id,
            &first.id,
            &first_location.id,
            ExtractionKind::Quote,
            "Quoted text",
        );
        assert_eq!(extract.source_version_id, first.id);
        assert_eq!(extract.source_segment_id, first_location.id);
        assert_eq!(extract.review_status, ExtractionReviewStatus::Unreviewed);
    }

    #[test]
    fn extraction_review_and_correction_preserve_original_and_gate_source_claims() {
        let conn = connection();
        let source_id = source(&conn);
        let source_version = version(&conn, &source_id, "v1");
        let location = location(&conn, &source_id, &source_version.id);
        let original = extraction(
            &conn,
            &source_id,
            &source_version.id,
            &location.id,
            ExtractionKind::SourceClaim,
            "Original source proposition.",
        );

        assert!(create_source_claim(
            &conn,
            CreateSourceClaimReq {
                extraction_id: original.id.clone(),
                statement: original.content.clone(),
                asserted_by: Some("A. Author".into()),
                scope: None,
            },
        )
        .is_err());

        let replacement = correct_extraction(
            &conn,
            CorrectExtractionReq {
                extraction_id: original.id.clone(),
                content: "Corrected source proposition.".into(),
                reason: "Corrected transcription.".into(),
                actor_id: Some("reviewer:test".into()),
            },
        )
        .unwrap();
        assert_eq!(
            replacement.parent_extraction_id.as_deref(),
            Some(original.id.as_str())
        );
        assert_eq!(replacement.review_status, ExtractionReviewStatus::Unreviewed);

        let history = list_extractions(&conn, &source_version.id).unwrap();
        let retained_original = history.iter().find(|item| item.id == original.id).unwrap();
        assert_eq!(retained_original.content, "Original source proposition.");
        assert_eq!(
            retained_original.superseded_by_id.as_deref(),
            Some(replacement.id.as_str())
        );
        assert_eq!(history.len(), 2);

        review_extraction(&conn, accept_extraction(&replacement.id)).unwrap();
        let claim = create_source_claim(
            &conn,
            CreateSourceClaimReq {
                extraction_id: replacement.id.clone(),
                statement: replacement.content.clone(),
                asserted_by: Some("A. Author".into()),
                scope: Some("Test scope".into()),
            },
        )
        .unwrap();
        assert_eq!(claim.source_version_id, source_version.id);
        assert_eq!(claim.extraction_id, replacement.id);
        assert!(list_provenance(&conn, "source_claim", &claim.id)
            .unwrap()
            .iter()
            .any(|event| event.event_kind == "source_claim_created"));
    }

    #[test]
    fn contradictory_source_claims_coexist_and_framework_publication_is_immutable() {
        let conn = connection();
        let source_id = source(&conn);
        let source_version = version(&conn, &source_id, "v1");
        let location = location(&conn, &source_id, &source_version.id);

        let first = extraction(
            &conn,
            &source_id,
            &source_version.id,
            &location.id,
            ExtractionKind::SourceClaim,
            "The method increases the measured outcome.",
        );
        let second = extraction(
            &conn,
            &source_id,
            &source_version.id,
            &location.id,
            ExtractionKind::SourceClaim,
            "The method does not increase the measured outcome.",
        );
        review_extraction(&conn, accept_extraction(&first.id)).unwrap();
        review_extraction(&conn, accept_extraction(&second.id)).unwrap();
        let first_claim = create_source_claim(
            &conn,
            CreateSourceClaimReq {
                extraction_id: first.id,
                statement: "The method increases the measured outcome.".into(),
                asserted_by: None,
                scope: None,
            },
        )
        .unwrap();
        let second_claim = create_source_claim(
            &conn,
            CreateSourceClaimReq {
                extraction_id: second.id,
                statement: "The method does not increase the measured outcome.".into(),
                asserted_by: None,
                scope: None,
            },
        )
        .unwrap();
        create_claim_relation(
            &conn,
            CreateClaimRelationReq {
                source_claim_id: first_claim.id.clone(),
                target_kind: KnowledgeItemKind::SourceClaim,
                target_id: second_claim.id.clone(),
                relation_kind: ClaimRelationKind::Contradicts,
                rationale: Some("The source propositions disagree.".into()),
                created_by: Some("reviewer:test".into()),
            },
        )
        .unwrap();
        assert_eq!(list_source_claims(&conn, &source_id).unwrap().len(), 2);
        assert!(list_claim_relations(&conn, &first_claim.id)
            .unwrap()
            .iter()
            .any(|relation| relation.target_id == second_claim.id
                && relation.relation_kind == ClaimRelationKind::Contradicts));

        let framework = create_framework(
            &conn,
            CreateFrameworkReq {
                title: "Test framework".into(),
                description: None,
                created_by: Some("user:test".into()),
            },
        )
        .unwrap();
        let initial_version = list_framework_versions(&conn, &framework.id).unwrap()[0].clone();
        add_framework_item(
            &conn,
            AddFrameworkItemReq {
                framework_version_id: initial_version.id.clone(),
                item_kind: KnowledgeItemKind::SourceClaim,
                item_id: first_claim.id.clone(),
                membership_role: "core".into(),
                item_order: 0,
                rationale: None,
            },
        )
        .unwrap();
        let second_item_id = add_framework_item(
            &conn,
            AddFrameworkItemReq {
                framework_version_id: initial_version.id.clone(),
                item_kind: KnowledgeItemKind::SourceClaim,
                item_id: second_claim.id.clone(),
                membership_role: "counterpoint".into(),
                item_order: 1,
                rationale: None,
            },
        )
        .unwrap();
        let first_item_id = list_framework_versions(&conn, &framework.id).unwrap()[0].items[0]
            .id
            .clone();
        reorder_framework_items(
            &conn,
            ReorderFrameworkItemsReq {
                framework_version_id: initial_version.id.clone(),
                ordered_item_ids: vec![second_item_id, first_item_id],
            },
        )
        .unwrap();
        let ordered = list_framework_versions(&conn, &framework.id).unwrap()[0].items.clone();
        assert_eq!(ordered[0].item_id, second_claim.id);
        assert_eq!(ordered[0].item_order, 0);

        review_framework_version(
            &conn,
            ReviewFrameworkReq {
                framework_version_id: initial_version.id.clone(),
                review_status: "approved".into(),
                reviewer_id: Some("reviewer:test".into()),
                rationale: None,
            },
        )
        .unwrap();
        publish_framework_version(&conn, &initial_version.id).unwrap();
        assert!(update_framework_version(
            &conn,
            UpdateFrameworkVersionReq {
                framework_version_id: initial_version.id.clone(),
                title: "Must not change".into(),
                description: String::new(),
            },
        )
        .is_err());
        assert!(add_framework_item(
            &conn,
            AddFrameworkItemReq {
                framework_version_id: initial_version.id.clone(),
                item_kind: KnowledgeItemKind::SourceClaim,
                item_id: first_claim.id.clone(),
                membership_role: "example".into(),
                item_order: 2,
                rationale: None,
            },
        )
        .is_err());

        let next = create_framework_version(
            &conn,
            CreateFrameworkVersionReq {
                framework_id: framework.id.clone(),
                change_class: "revision".into(),
                change_rationale: "Retain the conflicting source claim in a new draft.".into(),
                affected_claim_ids: vec![first_claim.id.clone()],
                assumption_claim_ids: vec![second_claim.id.clone()],
                created_by: Some("user:test".into()),
            },
        )
        .unwrap();
        assert_eq!(
            next.parent_version_id.as_deref(),
            Some(initial_version.id.as_str())
        );
        assert_eq!(next.items.len(), 2);
        assert_eq!(
            list_framework_versions(&conn, &framework.id).unwrap().len(),
            2
        );
    }

    #[test]
    fn additive_migration_preserves_legacy_source_rows_without_guessing_versions() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        conn.execute_batch(include_str!("../../schema_source.sql"))
            .expect("create legacy source schema");
        conn.execute(
            "INSERT INTO sources (id, kind, title, created_at, updated_at)
             VALUES ('SRC-legacy', 'doc', 'Legacy source', 1, 2)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO segments (id, source_id, label, created_at, updated_at)
             VALUES ('SEG-legacy', 'SRC-legacy', 'Legacy page', 1, 2)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO extracts (id, segment_id, transform_kind, content, created_at, updated_at)
             VALUES ('EXT-legacy', 'SEG-legacy', 'quoted', 'Legacy quote', 1, 2)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO source_revisions
                (id, entity_kind, entity_id, revision_type, after_json, created_at)
             VALUES ('REV-legacy', 'source', 'SRC-legacy', 'create', '{}', 1)",
            [],
        )
        .unwrap();

        init_schema(&conn).expect("apply additive acquisition migration");
        let source_title: String = conn
            .query_row("SELECT title FROM sources WHERE id = 'SRC-legacy'", [], |row| {
                row.get(0)
            })
            .unwrap();
        let extract_content: String = conn
            .query_row(
                "SELECT content FROM extracts WHERE id = 'EXT-legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let version_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM source_versions", [], |row| row.get(0))
            .unwrap();
        let new_extract_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM knowledge_extractions",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(source_title, "Legacy source");
        assert_eq!(extract_content, "Legacy quote");
        assert_eq!(version_count, 0);
        assert_eq!(new_extract_count, 0);
    }

    #[test]
    fn provenance_relations_survive_database_reopen() {
        let path = std::env::temp_dir().join(format!(
            "vora-acquisition-{}.sqlite",
            Uuid::now_v7()
        ));
        let (source_id, version_id, extraction_id, claim_id) = {
            let conn = Connection::open(&path).expect("create persistent test database");
            init_schema(&conn).expect("initialize persistent test database");
            let source_id = source(&conn);
            let source_version = version(&conn, &source_id, "v1");
            let location = location(&conn, &source_id, &source_version.id);
            let extraction = extraction(
                &conn,
                &source_id,
                &source_version.id,
                &location.id,
                ExtractionKind::SourceClaim,
                "A persisted source assertion.",
            );
            review_extraction(&conn, accept_extraction(&extraction.id)).unwrap();
            let claim = create_source_claim(
                &conn,
                CreateSourceClaimReq {
                    extraction_id: extraction.id.clone(),
                    statement: extraction.content.clone(),
                    asserted_by: Some("A. Author".into()),
                    scope: None,
                },
            )
            .unwrap();
            (
                source_id,
                source_version.id,
                extraction.id,
                claim.id,
            )
        };

        {
            let conn = Connection::open(&path).expect("reopen persistent test database");
            init_schema(&conn).expect("verify migration on database reopen");
            assert_eq!(list_source_versions(&conn, &source_id).unwrap().len(), 1);
            assert_eq!(list_extractions(&conn, &version_id).unwrap()[0].id, extraction_id);
            assert_eq!(list_source_claims(&conn, &source_id).unwrap()[0].id, claim_id);
            assert!(!list_provenance(&conn, "source_claim", &claim_id)
                .unwrap()
                .is_empty());
        }

        std::fs::remove_file(path).expect("remove persistent test database");
    }
}
