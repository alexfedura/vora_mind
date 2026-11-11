use serde::{Deserialize, Serialize};

pub mod ipc;
pub mod repository;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    Proposed,
    Active,
    Deprecated,
    Replaced,
    Retired,
}

impl ModelStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Active => "active",
            Self::Deprecated => "deprecated",
            Self::Replaced => "replaced",
            Self::Retired => "retired",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelVersionLifecycleStatus {
    Draft,
    Review,
    Released,
    Superseded,
    Withdrawn,
}

impl ModelVersionLifecycleStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Released => "released",
            Self::Superseded => "superseded",
            Self::Withdrawn => "withdrawn",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelChangeClass {
    Refinement,
    Extension,
    Revision,
    Replacement,
}

impl ModelChangeClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Refinement => "refinement",
            Self::Extension => "extension",
            Self::Revision => "revision",
            Self::Replacement => "replacement",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "refinement" => Ok(Self::Refinement),
            "extension" => Ok(Self::Extension),
            "revision" => Ok(Self::Revision),
            "replacement" => Ok(Self::Replacement),
            _ => Err(format!("invalid model change class: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelRelationKind {
    ContributesTo,
    Contradicts,
    Qualifies,
    TestedIn,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelContributionKind {
    ContributesTo,
    Contradicts,
    Qualifies,
    TestedIn,
    DatasetUse,
    EvidencedBy,
}

impl ModelContributionKind {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "contributes_to" => Ok(Self::ContributesTo),
            "contradicts" => Ok(Self::Contradicts),
            "qualifies" => Ok(Self::Qualifies),
            "tested_in" => Ok(Self::TestedIn),
            "dataset_use" => Ok(Self::DatasetUse),
            "evidenced_by" => Ok(Self::EvidencedBy),
            _ => Err(format!("invalid model contribution kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum ModelContributionTarget {
    HypothesisVersion {
        id: String,
    },
    FindingVersion {
        id: String,
    },
    Experiment {
        design_version_id: String,
        experiment_run_id: Option<String>,
    },
    Dataset {
        dataset_version_id: String,
        role: String,
        split_group_id: Option<String>,
    },
    Evidence {
        id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelContributionDto {
    pub id: String,
    pub relation_kind: ModelContributionKind,
    pub target: ModelContributionTarget,
    pub rationale: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

impl ModelRelationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ContributesTo => "contributes_to",
            Self::Contradicts => "contradicts",
            Self::Qualifies => "qualifies",
            Self::TestedIn => "tested_in",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelEvidencePolarity {
    Supports,
    Contradicts,
    Qualifies,
    Neutral,
}

impl ModelEvidencePolarity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Supports => "supports",
            Self::Contradicts => "contradicts",
            Self::Qualifies => "qualifies",
            Self::Neutral => "neutral",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum ModelArtifactRef {
    AnalysisOutput(String),
    ExperimentRun(String),
    DatasetVersion(String),
    ResearchObservation(String),
    SourceVersionSegment(String),
    SourceClaim(String),
    FindingVersion(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum ModelValidationTarget {
    ModelVersion(String),
    FindingVersion(String),
    SourceClaim(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateModelReq {
    pub display_name: String,
    pub description: String,
    pub origin_project_id: Option<String>,
    pub domain_id: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransitionModelReq {
    pub model_id: String,
    pub next_status: ModelStatus,
    pub rationale: String,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelLifecycleEventDto {
    pub status: String,
    pub rationale: String,
    pub actor_id: Option<String>,
    pub occurred_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransitionModelVersionReq {
    pub version_id: String,
    pub next_status: ModelVersionLifecycleStatus,
    pub rationale: String,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelDto {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub origin_project_id: Option<String>,
    pub domain_id: Option<String>,
    pub status: String,
    pub events: Vec<ModelLifecycleEventDto>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelSpecificationDto {
    pub human_readable: String,
    pub machine_readable: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelParameterReq {
    pub name: String,
    pub value: String,
    pub value_kind: String,
    pub unit: Option<String>,
    pub lower_bound: Option<String>,
    pub upper_bound: Option<String>,
    pub default_value: Option<String>,
    pub role: String,
    pub tuneable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelDependencyReq {
    pub name: String,
    pub version: String,
    pub source_uri: Option<String>,
    pub checksum_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelArtifactReq {
    pub artifact_kind: String,
    pub title: String,
    pub artifact_uri: String,
    pub checksum_sha256: String,
    pub media_type: Option<String>,
    pub byte_length: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelImpactKind {
    Added,
    Changed,
    Removed,
    Affected,
}

impl ModelImpactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Changed => "changed",
            Self::Removed => "removed",
            Self::Affected => "affected",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ModelImpactTarget {
    SourceClaim { claim_id: String },
    HypothesisVersion { hypothesis_version_id: String },
    FindingVersion { finding_version_id: String },
    Assumption { assumption_key: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelImpactReq {
    pub target: ModelImpactTarget,
    pub impact_kind: ModelImpactKind,
    pub previous_statement: Option<String>,
    pub current_statement: Option<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelAssumptionReq {
    pub key: String,
    pub statement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelParentReq {
    pub version_id: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRelationReq {
    pub target_id: String,
    pub relation_kind: ModelRelationKind,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelDatasetUseReq {
    pub dataset_version_id: String,
    pub role: String,
    pub split_group_id: Option<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelExperimentRelationReq {
    pub design_version_id: String,
    pub experiment_run_id: Option<String>,
    pub relation_kind: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateModelVersionReq {
    pub project_id: String,
    pub model_id: String,
    pub change_class: ModelChangeClass,
    pub change_rationale: String,
    pub expected_implications: String,
    pub specification: ModelSpecificationDto,
    pub assumptions: Vec<ModelAssumptionReq>,
    pub scope: String,
    pub dependencies: Vec<ModelDependencyReq>,
    pub environment: serde_json::Value,
    pub parents: Vec<ModelParentReq>,
    pub parameters: Vec<ModelParameterReq>,
    pub artifacts: Vec<ModelArtifactReq>,
    pub impacts: Vec<ModelImpactReq>,
    pub hypotheses: Vec<ModelRelationReq>,
    pub findings: Vec<ModelRelationReq>,
    pub experiments: Vec<ModelExperimentRelationReq>,
    pub dataset_uses: Vec<ModelDatasetUseReq>,
    pub evidence_ids: Vec<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelParameterDto {
    pub name: String,
    pub value: String,
    pub value_kind: String,
    pub unit: Option<String>,
    pub lower_bound: Option<String>,
    pub upper_bound: Option<String>,
    pub default_value: Option<String>,
    pub role: String,
    pub tuneable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelArtifactDto {
    pub id: String,
    pub artifact_kind: String,
    pub title: String,
    pub artifact_uri: String,
    pub checksum_sha256: String,
    pub media_type: Option<String>,
    pub byte_length: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelVersionDto {
    pub id: String,
    pub model_id: String,
    pub version_no: i64,
    pub created_in_project_id: String,
    pub change_class: ModelChangeClass,
    pub change_rationale: String,
    pub expected_implications: String,
    pub specification: ModelSpecificationDto,
    pub assumptions: Vec<ModelAssumptionReq>,
    pub scope: String,
    pub dependencies: Vec<ModelDependencyReq>,
    pub environment: serde_json::Value,
    pub parents: Vec<ModelParentReq>,
    pub parameters: Vec<ModelParameterDto>,
    pub artifacts: Vec<ModelArtifactDto>,
    pub impacts: Vec<ModelImpactReq>,
    pub contributions: Vec<ModelContributionDto>,
    pub hypotheses: Vec<ModelRelationReq>,
    pub findings: Vec<ModelRelationReq>,
    pub experiments: Vec<ModelExperimentRelationReq>,
    pub dataset_uses: Vec<ModelDatasetUseReq>,
    pub evidence_ids: Vec<String>,
    pub content_sha256: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub lifecycle_status: ModelVersionLifecycleStatus,
    pub lifecycle_events: Vec<ModelLifecycleEventDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateModelEvidenceReq {
    pub model_version_id: String,
    pub artifact: ModelArtifactRef,
    pub polarity: ModelEvidencePolarity,
    pub scope: String,
    pub quality_rationale: String,
    pub limitations: String,
    pub provenance: serde_json::Value,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelEvidenceDto {
    pub id: String,
    pub model_version_id: String,
    pub artifact: ModelArtifactRef,
    pub polarity: ModelEvidencePolarity,
    pub scope: String,
    pub quality_rationale: String,
    pub limitations: String,
    pub provenance: serde_json::Value,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateModelEvaluationReq {
    pub project_id: String,
    pub model_version_id: String,
    pub analysis_run_id: String,
    pub analysis_run_input_id: String,
    pub dataset_version_id: String,
    pub analysis_metric_id: String,
    pub role: String,
    pub split_group_id: Option<String>,
    pub limitations: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelEvaluationDto {
    pub id: String,
    pub project_id: String,
    pub model_version_id: String,
    pub analysis_run_id: String,
    pub analysis_run_input_id: String,
    pub dataset_version_id: String,
    pub role: String,
    pub split_group_id: Option<String>,
    pub analysis_metric_id: String,
    pub metric_definition_version_id: String,
    pub estimand: String,
    pub sample_size: i64,
    pub effect_estimate: String,
    pub unit: String,
    pub uncertainty_kind: String,
    pub uncertainty_lower: Option<String>,
    pub uncertainty_upper: Option<String>,
    pub uncertainty_level: Option<String>,
    pub uncertainty_reason: Option<String>,
    pub limitations: String,
    pub created_by: Option<String>,
    pub evaluated_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateModelValidationReq {
    pub project_id: String,
    pub target: ModelValidationTarget,
    pub criteria: Vec<serde_json::Value>,
    pub scope: String,
    pub population_scope: String,
    pub evaluation_method: String,
    pub evaluator_identity: String,
    pub independence_state: String,
    pub independence_rationale: String,
    pub conflict_of_interest: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelValidationOutcome {
    Pass,
    Fail,
    Inconclusive,
}

impl ModelValidationOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeModelValidationReq {
    pub validation_id: String,
    pub outcome: ModelValidationOutcome,
    pub rationale: String,
    pub deviations: Vec<String>,
    pub analysis_evidence_ids: Vec<String>,
    pub model_evidence_ids: Vec<String>,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelValidationDto {
    pub id: String,
    pub project_id: String,
    pub target: ModelValidationTarget,
    pub criteria: Vec<serde_json::Value>,
    pub scope: String,
    pub population_scope: String,
    pub evaluation_method: String,
    pub evaluator_identity: String,
    pub independence_state: String,
    pub independence_rationale: String,
    pub conflict_of_interest: String,
    pub outcome: Option<ModelValidationOutcome>,
    pub result_rationale: Option<String>,
    pub deviations: Vec<String>,
    pub analysis_evidence_ids: Vec<String>,
    pub model_evidence_ids: Vec<String>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub finalized_at_us: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelReplicationDatasetReq {
    pub dataset_version_id: String,
    pub role: String,
    pub split_group_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateModelReplicationReq {
    pub project_id: String,
    pub model_version_id: String,
    pub original_experiment_run_id: String,
    pub replication_kind: String,
    pub independent_team_operator: String,
    pub protocol_context: String,
    pub dataset_context: serde_json::Value,
    pub datasets: Vec<ModelReplicationDatasetReq>,
    pub outcome: String,
    pub result_summary: String,
    pub deviations: Vec<String>,
    pub provenance: serde_json::Value,
    pub analysis_evidence_ids: Vec<String>,
    pub model_evidence_ids: Vec<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelReplicationDto {
    pub id: String,
    pub project_id: String,
    pub model_version_id: String,
    pub original_experiment_run_id: String,
    pub original_design_version_id: String,
    pub replication_kind: String,
    pub independent_team_operator: String,
    pub protocol_context: String,
    pub dataset_context: serde_json::Value,
    pub datasets: Vec<ModelReplicationDatasetReq>,
    pub outcome: String,
    pub result_summary: String,
    pub deviations: Vec<String>,
    pub provenance: serde_json::Value,
    pub analysis_evidence_ids: Vec<String>,
    pub model_evidence_ids: Vec<String>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelWorkspaceDto {
    pub models: Vec<ModelDto>,
    pub versions: Vec<ModelVersionDto>,
    pub evidence: Vec<ModelEvidenceDto>,
    pub evaluations: Vec<ModelEvaluationDto>,
    pub validations: Vec<ModelValidationDto>,
    pub replications: Vec<ModelReplicationDto>,
}
