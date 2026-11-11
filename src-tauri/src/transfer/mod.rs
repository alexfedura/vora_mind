use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub mod ipc;
pub mod repository;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferArtifactKind {
    ConceptVersion,
    PrincipleVersion,
    FrameworkVersion,
    FindingVersion,
    ModelVersion,
}

impl TransferArtifactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ConceptVersion => "concept_version",
            Self::PrincipleVersion => "principle_version",
            Self::FrameworkVersion => "framework_version",
            Self::FindingVersion => "finding_version",
            Self::ModelVersion => "model_version",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "concept_version" => Ok(Self::ConceptVersion),
            "principle_version" => Ok(Self::PrincipleVersion),
            "framework_version" => Ok(Self::FrameworkVersion),
            "finding_version" => Ok(Self::FindingVersion),
            "model_version" => Ok(Self::ModelVersion),
            _ => Err(format!("unsupported transfer artifact kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferPurpose {
    Research,
    Learning,
    Commercial,
    PublicDistribution,
}

impl TransferPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Research => "research",
            Self::Learning => "learning",
            Self::Commercial => "commercial",
            Self::PublicDistribution => "public_distribution",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "research" => Ok(Self::Research),
            "learning" => Ok(Self::Learning),
            "commercial" => Ok(Self::Commercial),
            "public_distribution" => Ok(Self::PublicDistribution),
            _ => Err(format!("unsupported transfer purpose: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferAccessScope {
    Workspace,
    SelectedProjects,
}

impl TransferAccessScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::SelectedProjects => "selected_projects",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferEligibilityState {
    Eligible,
    Restricted,
    Withdrawn,
}

impl TransferEligibilityState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Eligible => "eligible",
            Self::Restricted => "restricted",
            Self::Withdrawn => "withdrawn",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeTransferState {
    Proposed,
    Accepted,
    Rejected,
    Withdrawn,
}

impl KnowledgeTransferState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Withdrawn => "withdrawn",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferArtifactDto {
    pub artifact_kind: TransferArtifactKind,
    pub artifact_id: String,
    pub artifact_version_id: String,
    pub title: String,
    pub version_no: i64,
    pub owner_project_id: Option<String>,
    pub origin_project_id: Option<String>,
    pub origin_rationale: Option<String>,
    pub scientific_status: String,
    pub eligibility_id: Option<String>,
    pub eligibility_status: Option<TransferEligibilityState>,
    pub license_identifier: Option<String>,
    pub license_uri: Option<String>,
    pub access_scope: Option<TransferAccessScope>,
    pub allowed_purposes: Vec<TransferPurpose>,
    pub attribution_text: Option<String>,
    pub permission_basis: Option<String>,
    pub block_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferEligibilityDto {
    pub id: String,
    pub artifact_kind: TransferArtifactKind,
    pub artifact_id: String,
    pub artifact_version_id: String,
    pub artifact_title: String,
    pub origin_id: String,
    pub origin_project_id: String,
    pub license_identifier: String,
    pub license_uri: Option<String>,
    pub access_scope: TransferAccessScope,
    pub allowed_purposes: Vec<TransferPurpose>,
    pub allowed_project_ids: Vec<String>,
    pub attribution_text: String,
    pub permission_basis: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at_us: i64,
    pub state: TransferEligibilityState,
    pub state_rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeTransferEventDto {
    pub id: String,
    pub state: KnowledgeTransferState,
    pub rationale: String,
    pub actor_id: Option<String>,
    pub occurred_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeTransferDto {
    pub id: String,
    pub eligibility_id: String,
    pub artifact_kind: TransferArtifactKind,
    pub artifact_id: String,
    pub artifact_version_id: String,
    pub artifact_title: String,
    pub origin_project_id: String,
    pub receiving_project_id: String,
    pub intended_purpose: TransferPurpose,
    pub use_context: String,
    pub attribution_text: String,
    pub requested_by: Option<String>,
    pub requested_at_us: i64,
    pub reasserts_transfer_id: Option<String>,
    pub state: KnowledgeTransferState,
    pub events: Vec<KnowledgeTransferEventDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferAdaptationDto {
    pub id: String,
    pub transfer_id: String,
    pub receiving_project_id: String,
    pub artifact_kind: TransferArtifactKind,
    pub artifact_version_id: String,
    pub artifact_title: String,
    pub rationale: String,
    pub adapted_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferDownstreamUsageDto {
    pub id: String,
    pub transfer_id: String,
    pub project_id: String,
    pub usage_kind: String,
    pub artifact_id: String,
    pub artifact_title: String,
    pub use_context: String,
    pub recorded_by: Option<String>,
    pub recorded_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferWorkspaceDto {
    pub artifacts: Vec<TransferArtifactDto>,
    pub eligibility_records: Vec<TransferEligibilityDto>,
    pub transfers: Vec<KnowledgeTransferDto>,
    pub adaptations: Vec<TransferAdaptationDto>,
    pub downstream_usage: Vec<TransferDownstreamUsageDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateArtifactVersionReq {
    pub artifact_kind: TransferArtifactKind,
    pub artifact_id: String,
    pub project_id: String,
    pub title: String,
    pub content: String,
    pub rationale: String,
    pub parent_version_id: Option<String>,
    pub reviewed_by: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordTransferOriginReq {
    pub artifact_kind: TransferArtifactKind,
    pub artifact_version_id: String,
    pub origin_project_id: String,
    pub rationale: String,
    pub recorded_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateTransferEligibilityReq {
    pub origin_id: String,
    pub license_identifier: String,
    pub license_uri: Option<String>,
    pub access_scope: TransferAccessScope,
    pub allowed_purposes: Vec<TransferPurpose>,
    pub allowed_project_ids: Vec<String>,
    pub attribution_text: String,
    pub permission_basis: String,
    pub reviewed_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeTransferEligibilityReq {
    pub eligibility_id: String,
    pub next_state: TransferEligibilityState,
    pub rationale: String,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeTransferReq {
    pub eligibility_id: String,
    pub receiving_project_id: String,
    pub intended_purpose: TransferPurpose,
    pub use_context: String,
    pub attribution_text: String,
    pub requested_by: Option<String>,
    pub reasserts_transfer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransitionKnowledgeTransferReq {
    pub transfer_id: String,
    pub next_state: KnowledgeTransferState,
    pub rationale: String,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateTransferAdaptationReq {
    pub transfer_id: String,
    pub artifact_kind: TransferArtifactKind,
    pub artifact_version_id: String,
    pub rationale: String,
    pub adapted_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateTransferUsageReq {
    pub transfer_id: String,
    pub project_id: String,
    pub usage_kind: String,
    pub artifact_id: String,
    pub use_context: String,
    pub recorded_by: Option<String>,
}

fn now_us() -> Result<i64, String> {
    let value = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_micros();
    i64::try_from(value).map_err(|error| error.to_string())
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::now_v7())
}

fn required(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    Ok(())
}

pub fn initialize_schema(conn: &Connection) -> Result<(), String> {
    repository::initialize_schema(conn).map_err(|error| error.to_string())
}

pub fn workspace(conn: &Connection) -> Result<TransferWorkspaceDto, String> {
    repository::workspace(conn).map_err(|error| error.to_string())
}

pub fn create_artifact_version(
    conn: &mut Connection,
    request: CreateArtifactVersionReq,
) -> Result<TransferArtifactDto, String> {
    if !matches!(
        request.artifact_kind,
        TransferArtifactKind::ConceptVersion | TransferArtifactKind::PrincipleVersion
    ) {
        return Err("only concept and principle versions are created by this workflow".into());
    }
    required(&request.artifact_id, "artifact ID")?;
    required(&request.project_id, "project ID")?;
    required(&request.title, "artifact title")?;
    required(&request.content, "artifact content")?;
    required(&request.rationale, "version rationale")?;
    repository::create_artifact_version(
        conn,
        &request,
        &new_id("transfer_artifact_version"),
        now_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn record_origin(
    conn: &mut Connection,
    request: RecordTransferOriginReq,
) -> Result<String, String> {
    required(&request.artifact_version_id, "artifact version ID")?;
    required(&request.origin_project_id, "origin project ID")?;
    required(&request.rationale, "origin rationale")?;
    repository::record_origin(
        conn,
        &request,
        &new_id("transfer_origin"),
        now_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn create_eligibility(
    conn: &mut Connection,
    request: CreateTransferEligibilityReq,
) -> Result<TransferEligibilityDto, String> {
    required(&request.origin_id, "origin assertion ID")?;
    required(&request.license_identifier, "license identifier")?;
    if request.license_identifier.trim().eq_ignore_ascii_case("unknown") {
        return Err("unknown license metadata cannot authorize transfer".into());
    }
    required(&request.attribution_text, "attribution")?;
    required(&request.permission_basis, "permission basis")?;
    if request.allowed_purposes.is_empty() {
        return Err("at least one permitted use purpose is required".into());
    }
    match request.access_scope {
        TransferAccessScope::Workspace if !request.allowed_project_ids.is_empty() => {
            return Err("workspace-scoped eligibility must not list individual projects".into());
        }
        TransferAccessScope::SelectedProjects if request.allowed_project_ids.is_empty() => {
            return Err("selected-project eligibility must list at least one recipient".into());
        }
        _ => {}
    }
    repository::create_eligibility(
        conn,
        &request,
        &new_id("transfer_eligibility"),
        &new_id("transfer_eligibility_event"),
        now_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn change_eligibility(
    conn: &Connection,
    request: ChangeTransferEligibilityReq,
) -> Result<TransferEligibilityDto, String> {
    required(&request.rationale, "eligibility change rationale")?;
    repository::change_eligibility(&request, &new_id("transfer_eligibility_event"), now_us()?)
        .map_err(|error| error.to_string())
}

pub fn create_transfer(
    conn: &mut Connection,
    request: CreateKnowledgeTransferReq,
) -> Result<KnowledgeTransferDto, String> {
    required(&request.eligibility_id, "eligibility ID")?;
    required(&request.receiving_project_id, "receiving project ID")?;
    required(&request.use_context, "transfer context")?;
    required(&request.attribution_text, "transfer attribution")?;
    repository::create_transfer(
        conn,
        &request,
        &new_id("knowledge_transfer"),
        &new_id("knowledge_transfer_event"),
        now_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn transition_transfer(
    conn: &Connection,
    request: TransitionKnowledgeTransferReq,
) -> Result<KnowledgeTransferDto, String> {
    required(&request.rationale, "transfer transition rationale")?;
    repository::transition_transfer(
        &request,
        &new_id("knowledge_transfer_event"),
        now_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn create_adaptation(
    conn: &Connection,
    request: CreateTransferAdaptationReq,
) -> Result<TransferAdaptationDto, String> {
    required(&request.transfer_id, "transfer ID")?;
    required(&request.artifact_version_id, "adapted artifact version ID")?;
    required(&request.rationale, "adaptation rationale")?;
    repository::create_adaptation(&request, &new_id("transfer_adaptation"), now_us()?)
        .map_err(|error| error.to_string())
}

pub fn create_downstream_usage(
    conn: &Connection,
    request: CreateTransferUsageReq,
) -> Result<TransferDownstreamUsageDto, String> {
    required(&request.transfer_id, "transfer ID")?;
    required(&request.project_id, "project ID")?;
    required(&request.artifact_id, "downstream artifact ID")?;
    required(&request.use_context, "downstream use context")?;
    repository::create_downstream_usage(&request, &new_id("transfer_usage"), now_us()?)
        .map_err(|error| error.to_string())
}
