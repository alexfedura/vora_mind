use serde::{Deserialize, Serialize};

pub mod ipc;
pub mod repository;
pub mod service;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferArtifactKind {
    FindingVersion,
    ConceptVersion,
    PrincipleVersion,
    FrameworkVersion,
    ModelVersion,
}

impl TransferArtifactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FindingVersion => "finding_version",
            Self::ConceptVersion => "concept_version",
            Self::PrincipleVersion => "principle_version",
            Self::FrameworkVersion => "framework_version",
            Self::ModelVersion => "model_version",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "finding_version" => Ok(Self::FindingVersion),
            "concept_version" => Ok(Self::ConceptVersion),
            "principle_version" => Ok(Self::PrincipleVersion),
            "framework_version" => Ok(Self::FrameworkVersion),
            "model_version" => Ok(Self::ModelVersion),
            _ => Err(format!("Unsupported transfer artifact kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum TransferArtifactRef {
    FindingVersion {
        finding_id: String,
        version_id: String,
    },
    ConceptVersion {
        concept_id: String,
        version_id: String,
    },
    PrincipleVersion {
        principle_id: String,
        version_id: String,
    },
    FrameworkVersion {
        framework_id: String,
        version_id: String,
    },
    ModelVersion {
        model_id: String,
        version_id: String,
    },
}

impl TransferArtifactRef {
    pub fn kind(&self) -> TransferArtifactKind {
        match self {
            Self::FindingVersion { .. } => TransferArtifactKind::FindingVersion,
            Self::ConceptVersion { .. } => TransferArtifactKind::ConceptVersion,
            Self::PrincipleVersion { .. } => TransferArtifactKind::PrincipleVersion,
            Self::FrameworkVersion { .. } => TransferArtifactKind::FrameworkVersion,
            Self::ModelVersion { .. } => TransferArtifactKind::ModelVersion,
        }
    }

    pub fn version_id(&self) -> &str {
        match self {
            Self::FindingVersion { version_id, .. }
            | Self::ConceptVersion { version_id, .. }
            | Self::PrincipleVersion { version_id, .. }
            | Self::FrameworkVersion { version_id, .. }
            | Self::ModelVersion { version_id, .. } => version_id,
        }
    }

    pub fn entity_id(&self) -> &str {
        match self {
            Self::FindingVersion { finding_id, .. } => finding_id,
            Self::ConceptVersion { concept_id, .. } => concept_id,
            Self::PrincipleVersion { principle_id, .. } => principle_id,
            Self::FrameworkVersion { framework_id, .. } => framework_id,
            Self::ModelVersion { model_id, .. } => model_id,
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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferAccessScope {
    Workspace,
    SelectedProjects,
}

impl TransferAccessScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::SelectedProjects => "selected_projects",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferEligibilityState {
    Eligible,
    Restricted,
    Withdrawn,
}

impl TransferEligibilityState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Eligible => "eligible",
            Self::Restricted => "restricted",
            Self::Withdrawn => "withdrawn",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeTransferState {
    Accepted,
    Rejected,
    Withdrawn,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeVersionKind {
    Concept,
    Principle,
}

impl KnowledgeVersionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Concept => "concept",
            Self::Principle => "principle",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeVersionStatus {
    Draft,
    Reviewed,
    Deprecated,
}

impl KnowledgeVersionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Reviewed => "reviewed",
            Self::Deprecated => "deprecated",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferLineageRelation {
    Adopted,
    DerivedFrom,
    BasedOn,
}

impl TransferLineageRelation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Adopted => "adopted",
            Self::DerivedFrom => "derived_from",
            Self::BasedOn => "based_on",
        }
    }
}

impl KnowledgeTransferState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Withdrawn => "withdrawn",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchProjectDto {
    pub id: String,
    pub title: String,
    pub lifecycle_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferArtifactCatalogDto {
    pub artifact: TransferArtifactRef,
    pub title: String,
    pub version_no: i64,
    pub owner_project_id: Option<String>,
    pub lifecycle_status: String,
    pub origin_id: Option<String>,
    pub eligibility_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeVersionDto {
    pub kind: KnowledgeVersionKind,
    pub entity_id: String,
    pub version_id: String,
    pub version_no: i64,
    pub project_id: String,
    pub title: String,
    pub content: String,
    pub rationale: String,
    pub status: String,
    pub parent_version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeTransferOriginDto {
    pub id: String,
    pub artifact: TransferArtifactRef,
    pub origin_project_id: String,
    pub origin_project_title: String,
    pub rationale: String,
    pub recorded_by: Option<String>,
    pub recorded_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferEligibilityDto {
    pub id: String,
    pub origin: KnowledgeTransferOriginDto,
    pub license_identifier: String,
    pub license_uri: Option<String>,
    pub access_scope: TransferAccessScope,
    pub permitted_purposes: Vec<TransferPurpose>,
    pub allowed_project_ids: Vec<String>,
    pub attribution_text: String,
    pub permission_basis: String,
    pub reviewed_by: String,
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
    pub origin: KnowledgeTransferOriginDto,
    pub receiving_project_id: String,
    pub receiving_project_title: String,
    pub intended_purpose: TransferPurpose,
    pub attribution_text: String,
    pub context: String,
    pub requested_by: Option<String>,
    pub requested_at_us: i64,
    pub state: KnowledgeTransferState,
    pub events: Vec<KnowledgeTransferEventDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferAdaptationDto {
    pub id: String,
    pub transfer_id: String,
    pub origin: KnowledgeTransferOriginDto,
    pub receiving_project_id: String,
    pub artifact: TransferArtifactRef,
    pub title: String,
    pub rationale: String,
    pub adapted_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum DownstreamUseRef {
    FindingVersion {
        finding_id: String,
        version_id: String,
    },
    ConceptVersion {
        concept_id: String,
        version_id: String,
    },
    PrincipleVersion {
        principle_id: String,
        version_id: String,
    },
    FrameworkVersion {
        framework_id: String,
        version_id: String,
    },
    ModelVersion {
        model_id: String,
        version_id: String,
    },
    ModelEvaluation {
        id: String,
    },
    AnalysisRun {
        id: String,
    },
    ExperimentRun {
        id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownstreamUsageDto {
    pub id: String,
    pub transfer_id: String,
    pub project_id: String,
    pub reference: DownstreamUseRef,
    pub use_context: String,
    pub recorded_by: Option<String>,
    pub recorded_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchLineageDto {
    pub id: String,
    pub transfer_id: String,
    pub origin: TransferArtifactRef,
    pub origin_project_id: String,
    pub receiving_project_id: String,
    pub relation_kind: TransferLineageRelation,
    pub adaptation: Option<TransferArtifactRef>,
    pub rationale: String,
    pub occurred_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferProvenanceRecordDto {
    pub record_kind: String,
    pub record_id: String,
    pub label: String,
    pub relationship: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferProvenancePreviewDto {
    pub origin: KnowledgeTransferOriginDto,
    pub title: String,
    pub version_no: i64,
    pub lifecycle_status: String,
    pub scope: String,
    pub applicability: String,
    pub uncertainty: String,
    pub limitations: String,
    pub validation_scope: Vec<String>,
    pub provenance: Vec<TransferProvenanceRecordDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferWorkspaceDto {
    pub projects: Vec<ResearchProjectDto>,
    pub artifacts: Vec<TransferArtifactCatalogDto>,
    pub knowledge_versions: Vec<KnowledgeVersionDto>,
    pub origins: Vec<KnowledgeTransferOriginDto>,
    pub eligibilities: Vec<TransferEligibilityDto>,
    pub transfers: Vec<KnowledgeTransferDto>,
    pub adaptations: Vec<TransferAdaptationDto>,
    pub downstream_usage: Vec<DownstreamUsageDto>,
    pub lineage: Vec<ResearchLineageDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateConceptVersionReq {
    pub concept_id: Option<String>,
    pub project_id: String,
    pub parent_version_id: Option<String>,
    pub title: String,
    pub definition: String,
    pub rationale: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePrincipleVersionReq {
    pub principle_id: Option<String>,
    pub project_id: String,
    pub parent_version_id: Option<String>,
    pub title: String,
    pub statement: String,
    pub rationale: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewKnowledgeVersionReq {
    pub version_id: String,
    pub kind: KnowledgeVersionKind,
    pub status: KnowledgeVersionStatus,
    pub rationale: String,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTransferOriginReq {
    pub artifact: TransferArtifactRef,
    pub origin_project_id: String,
    pub rationale: String,
    pub recorded_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTransferEligibilityReq {
    pub origin_id: String,
    pub license_identifier: String,
    pub license_uri: Option<String>,
    pub access_scope: TransferAccessScope,
    pub permitted_purposes: Vec<TransferPurpose>,
    pub allowed_project_ids: Vec<String>,
    pub attribution_text: String,
    pub permission_basis: String,
    pub permission_granted: bool,
    pub reviewed_by: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeTransferEligibilityReq {
    pub eligibility_id: String,
    pub state: TransferEligibilityState,
    pub rationale: String,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptKnowledgeReq {
    pub eligibility_id: String,
    pub receiving_project_id: String,
    pub intended_purpose: TransferPurpose,
    pub attribution_text: String,
    pub context: String,
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTransferAdaptationReq {
    pub transfer_id: String,
    pub artifact: TransferArtifactRef,
    pub relation_kind: TransferLineageRelation,
    pub rationale: String,
    pub adapted_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordDownstreamUsageReq {
    pub transfer_id: String,
    pub reference: DownstreamUseRef,
    pub use_context: String,
    pub recorded_by: Option<String>,
}
