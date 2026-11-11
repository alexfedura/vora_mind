use crate::experiment::DatasetRole;
use serde::{Deserialize, Serialize};

pub mod ipc;
pub mod repository;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Applicability<T> {
    Applicable { details: T },
    NotApplicable { reason: String },
    NotAssessed { rationale: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OverfittingSafeguardDto {
    pub training_tuning_separation: String,
    pub parameter_search_space: String,
    pub model_selection_process: String,
    pub complexity_reporting: String,
    pub sensitivity_reporting: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LeakageSafeguardDto {
    pub data_cutoffs: String,
    pub transformation_order: String,
    pub feature_selection_provenance: String,
    pub evaluation_data_isolation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectionBiasSafeguardDto {
    pub sampling_frame: String,
    pub inclusion_rules: String,
    pub exclusion_rules: String,
    pub filtered_and_missing_cases: String,
    pub survivor_population: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LookAheadSafeguardDto {
    pub feature_availability_policy: String,
    pub prediction_timestamp_policy: String,
    pub decision_timestamp_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MultiplicitySafeguardDto {
    pub test_family: String,
    pub correction_method: Option<String>,
    pub no_correction_justification: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatasetSeparationSafeguardDto {
    pub require_disjoint_splits: bool,
    pub selection_rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelSelectionUsage {
    NotUsed,
    UsedForSelection,
    PartiallyUsed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OutOfSampleSafeguardDto {
    pub evaluation_period: String,
    pub evaluation_population: String,
    pub model_selection_usage: ModelSelectionUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalkForwardWindowDto {
    pub label: String,
    pub training_start_at_us: i64,
    pub training_end_at_us: i64,
    pub validation_start_at_us: i64,
    pub validation_end_at_us: i64,
    pub test_start_at_us: i64,
    pub test_end_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WalkForwardSafeguardDto {
    pub windows: Vec<WalkForwardWindowDto>,
    pub refit_cadence: String,
    pub aggregation_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ForwardExecutionKind {
    Prospective,
    Paper,
    Live,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ForwardTestingSafeguardDto {
    pub execution_kind: ForwardExecutionKind,
    pub actual_execution_conditions: String,
    pub costs_and_fees: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReproducibilitySafeguardDto {
    pub protocol_version: String,
    pub raw_input_checksums_required: bool,
    pub transformation_lineage_required: bool,
    pub code_and_environment_required: bool,
    pub parameters_and_seeds_required: bool,
    pub timestamps_and_deviations_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatisticalIntegrityPlanDto {
    pub overfitting: Applicability<OverfittingSafeguardDto>,
    pub data_leakage: Applicability<LeakageSafeguardDto>,
    pub selection_bias: Applicability<SelectionBiasSafeguardDto>,
    pub look_ahead: Applicability<LookAheadSafeguardDto>,
    pub multiple_testing: Applicability<MultiplicitySafeguardDto>,
    pub dataset_separation: Applicability<DatasetSeparationSafeguardDto>,
    pub out_of_sample: Applicability<OutOfSampleSafeguardDto>,
    pub walk_forward: Applicability<WalkForwardSafeguardDto>,
    pub forward_testing: Applicability<ForwardTestingSafeguardDto>,
    pub reproducibility: Applicability<ReproducibilitySafeguardDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisDefinitionSpecificationDto {
    pub protocol_version: String,
    pub required_input_roles: Vec<DatasetRole>,
    pub parameters_schema: serde_json::Value,
    pub integrity: StatisticalIntegrityPlanDto,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisVerificationStatus {
    Passed,
    Failed,
}

impl AnalysisVerificationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "passed" => Ok(Self::Passed),
            "failed" => Ok(Self::Failed),
            _ => Err(format!("invalid analysis verification status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateAnalysisMethodVersionReq {
    pub project_id: String,
    pub method_id: Option<String>,
    pub name: String,
    pub version_label: String,
    pub code_uri: String,
    pub code_revision: String,
    pub dependency_lock_sha256: Option<String>,
    pub environment: serde_json::Value,
    pub assumptions: Vec<String>,
    pub test_suite_uri: String,
    pub test_suite_sha256: String,
    pub tests_total: i64,
    pub tests_passed: i64,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisMethodVersionDto {
    pub id: String,
    pub method_id: String,
    pub project_id: String,
    pub name: String,
    pub version_no: i64,
    pub version_label: String,
    pub code_uri: String,
    pub code_revision: String,
    pub dependency_lock_sha256: Option<String>,
    pub environment: serde_json::Value,
    pub assumptions: Vec<String>,
    pub test_suite_uri: String,
    pub test_suite_sha256: String,
    pub tests_total: i64,
    pub tests_passed: i64,
    pub verification_status: AnalysisVerificationStatus,
    pub content_sha256: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateMetricDefinitionVersionReq {
    pub project_id: String,
    pub metric_definition_id: Option<String>,
    pub key: String,
    pub name: String,
    pub version_label: String,
    pub formula: String,
    pub unit: String,
    pub test_suite_uri: String,
    pub test_suite_sha256: String,
    pub tests_total: i64,
    pub tests_passed: i64,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MetricDefinitionVersionDto {
    pub id: String,
    pub metric_definition_id: String,
    pub project_id: String,
    pub key: String,
    pub name: String,
    pub version_no: i64,
    pub version_label: String,
    pub formula: String,
    pub unit: String,
    pub test_suite_uri: String,
    pub test_suite_sha256: String,
    pub tests_total: i64,
    pub tests_passed: i64,
    pub verification_status: AnalysisVerificationStatus,
    pub content_sha256: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisDefinitionStatus {
    Draft,
    Review,
    Locked,
    Retired,
}

impl AnalysisDefinitionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Locked => "locked",
            Self::Retired => "retired",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "draft" => Ok(Self::Draft),
            "review" => Ok(Self::Review),
            "locked" => Ok(Self::Locked),
            "retired" => Ok(Self::Retired),
            _ => Err(format!("invalid analysis definition status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateAnalysisDefinitionVersionReq {
    pub project_id: String,
    pub definition_id: Option<String>,
    pub parent_version_id: Option<String>,
    pub title: String,
    pub version_label: String,
    pub change_rationale: String,
    pub method_version_id: String,
    pub metric_definition_version_ids: Vec<String>,
    pub specification: AnalysisDefinitionSpecificationDto,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransitionAnalysisDefinitionReq {
    pub version_id: String,
    pub next_status: AnalysisDefinitionStatus,
    pub rationale: String,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisDefinitionVersionDto {
    pub id: String,
    pub definition_id: String,
    pub project_id: String,
    pub version_no: i64,
    pub title: String,
    pub version_label: String,
    pub parent_version_id: Option<String>,
    pub change_rationale: Option<String>,
    pub method_version_id: String,
    pub metric_definition_version_ids: Vec<String>,
    pub specification: AnalysisDefinitionSpecificationDto,
    pub specification_sha256: String,
    pub lifecycle_status: AnalysisDefinitionStatus,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub locked_at_us: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisInputSampleReq {
    pub sample_id: String,
    pub feature_available_at_us: Option<i64>,
    pub prediction_at_us: Option<i64>,
    pub decision_at_us: Option<i64>,
    pub outcome_available_at_us: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisInputReq {
    pub dataset_version_id: String,
    pub role: DatasetRole,
    pub split_group_id: Option<String>,
    pub samples: Vec<AnalysisInputSampleReq>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateAnalysisRunReq {
    pub definition_version_id: String,
    pub parameters: serde_json::Value,
    pub environment: serde_json::Value,
    pub random_seeds: Vec<String>,
    pub inputs: Vec<AnalysisInputReq>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisRunStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Inconclusive,
}

impl AnalysisRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Inconclusive => "inconclusive",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "inconclusive" => Ok(Self::Inconclusive),
            _ => Err(format!("invalid analysis run status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisLogSeverity {
    Info,
    Warning,
    Error,
}

impl AnalysisLogSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            _ => Err(format!("invalid analysis log severity: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddAnalysisRunLogReq {
    pub run_id: String,
    pub severity: AnalysisLogSeverity,
    pub message: String,
    pub details: serde_json::Value,
    pub recorded_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisRunLogDto {
    pub id: String,
    pub severity: AnalysisLogSeverity,
    pub message: String,
    pub details: serde_json::Value,
    pub recorded_by: Option<String>,
    pub recorded_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisErrorReq {
    pub error_kind: String,
    pub message: String,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisErrorDto {
    pub id: String,
    pub error_kind: String,
    pub message: String,
    pub details: serde_json::Value,
    pub recorded_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisDeviationReq {
    pub description: String,
    pub rationale: String,
    pub impact_assessment: String,
    pub recorded_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisDeviationDto {
    pub id: String,
    pub description: String,
    pub rationale: String,
    pub impact_assessment: String,
    pub recorded_by: Option<String>,
    pub recorded_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisOutputReq {
    pub output_key: String,
    pub output_kind: String,
    pub title: String,
    pub payload: serde_json::Value,
    pub artifact_uri: Option<String>,
    pub artifact_sha256: Option<String>,
    pub media_type: Option<String>,
    pub byte_length: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisUncertaintyIntervalDto {
    pub lower: String,
    pub upper: String,
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UncertaintyReportDto {
    Interval {
        interval: AnalysisUncertaintyIntervalDto,
    },
    NotMeaningful {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MissingDataReportDto {
    pub policy: String,
    pub missing_count: i64,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisMetricReportDto {
    pub output_key: String,
    pub metric_definition_version_id: String,
    pub estimand: String,
    pub sample_size: i64,
    pub effect_estimate: String,
    pub unit: String,
    pub uncertainty: UncertaintyReportDto,
    pub assumptions: Vec<String>,
    pub exclusions: Vec<String>,
    pub missing_data: MissingDataReportDto,
    pub multiplicity_treatment: String,
    pub p_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisIntegritySeverity {
    Warning,
    Blocking,
}

impl AnalysisIntegritySeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Blocking => "blocking",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisIntegrityFindingDto {
    pub id: String,
    pub category: String,
    pub code: String,
    pub severity: AnalysisIntegritySeverity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeAnalysisRunReq {
    pub run_id: String,
    pub next_status: AnalysisRunStatus,
    pub summary: String,
    pub outputs: Vec<AnalysisOutputReq>,
    pub metrics: Vec<AnalysisMetricReportDto>,
    pub errors: Vec<AnalysisErrorReq>,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisInputSampleDto {
    pub sample_id: String,
    pub split_key: Option<String>,
    pub feature_available_at_us: Option<i64>,
    pub prediction_at_us: Option<i64>,
    pub decision_at_us: Option<i64>,
    pub outcome_available_at_us: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisInputDto {
    pub id: String,
    pub dataset_version_id: String,
    pub dataset_checksum: String,
    pub role: DatasetRole,
    pub split_group_id: Option<String>,
    pub sample_count: i64,
    pub transformation_lineage: serde_json::Value,
    pub samples: Vec<AnalysisInputSampleDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisOutputDto {
    pub id: String,
    pub output_kind: String,
    pub title: String,
    pub payload: serde_json::Value,
    pub payload_sha256: String,
    pub artifact_uri: Option<String>,
    pub artifact_sha256: Option<String>,
    pub media_type: Option<String>,
    pub byte_length: Option<i64>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisMetricDto {
    pub id: String,
    pub output_id: String,
    pub metric_definition_version_id: String,
    pub method_version_id: String,
    pub estimand: String,
    pub sample_size: i64,
    pub effect_estimate: String,
    pub unit: String,
    pub uncertainty: UncertaintyReportDto,
    pub assumptions: Vec<String>,
    pub exclusions: Vec<String>,
    pub missing_data: MissingDataReportDto,
    pub multiplicity_treatment: String,
    pub p_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisRunDto {
    pub id: String,
    pub project_id: String,
    pub definition_version_id: String,
    pub lifecycle_status: AnalysisRunStatus,
    pub parameters: serde_json::Value,
    pub environment: serde_json::Value,
    pub random_seeds: Vec<String>,
    pub protocol_version: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub started_at_us: Option<i64>,
    pub ended_at_us: Option<i64>,
    pub completion_summary: Option<String>,
    pub failure_summary: Option<String>,
    pub inputs: Vec<AnalysisInputDto>,
    pub integrity_findings: Vec<AnalysisIntegrityFindingDto>,
    pub outputs: Vec<AnalysisOutputDto>,
    pub metrics: Vec<AnalysisMetricDto>,
    pub logs: Vec<AnalysisRunLogDto>,
    pub errors: Vec<AnalysisErrorDto>,
    pub deviations: Vec<AnalysisDeviationDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum PropositionRefDto {
    SourceClaim(String),
    HypothesisVersion(String),
    FindingVersion(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum EvidenceArtifactRefDto {
    AnalysisOutput(String),
    ExperimentRun(String),
    DatasetVersion(String),
    ResearchObservation(String),
    SourceVersionSegment(String),
    SourceClaim(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidencePolarity {
    Supports,
    Contradicts,
    Qualifies,
    Neutral,
}

impl EvidencePolarity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Supports => "supports",
            Self::Contradicts => "contradicts",
            Self::Qualifies => "qualifies",
            Self::Neutral => "neutral",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "supports" => Ok(Self::Supports),
            "contradicts" => Ok(Self::Contradicts),
            "qualifies" => Ok(Self::Qualifies),
            "neutral" => Ok(Self::Neutral),
            _ => Err(format!("invalid evidence polarity: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateEvidenceReq {
    pub project_id: String,
    pub proposition: PropositionRefDto,
    pub artifact: EvidenceArtifactRefDto,
    pub polarity: EvidencePolarity,
    pub scope: String,
    pub quality_rationale: String,
    pub limitations: String,
    pub provenance: serde_json::Value,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Recorded,
    Assessed,
    Accepted,
    Contested,
    Rejected,
    Superseded,
    Retracted,
}

impl EvidenceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Recorded => "recorded",
            Self::Assessed => "assessed",
            Self::Accepted => "accepted",
            Self::Contested => "contested",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
            Self::Retracted => "retracted",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "recorded" => Ok(Self::Recorded),
            "assessed" => Ok(Self::Assessed),
            "accepted" => Ok(Self::Accepted),
            "contested" => Ok(Self::Contested),
            "rejected" => Ok(Self::Rejected),
            "superseded" => Ok(Self::Superseded),
            "retracted" => Ok(Self::Retracted),
            _ => Err(format!("invalid evidence status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceEventDto {
    pub id: String,
    pub status: EvidenceStatus,
    pub rationale: String,
    pub actor_id: Option<String>,
    pub occurred_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransitionEvidenceReq {
    pub evidence_id: String,
    pub next_status: EvidenceStatus,
    pub rationale: String,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceDto {
    pub id: String,
    pub project_id: String,
    pub proposition: PropositionRefDto,
    pub artifact: EvidenceArtifactRefDto,
    pub polarity: EvidencePolarity,
    pub scope: String,
    pub quality_rationale: String,
    pub limitations: String,
    pub provenance: serde_json::Value,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub status: EvidenceStatus,
    pub events: Vec<EvidenceEventDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingConclusionKind {
    Result,
    NullResult,
    Contradictory,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Draft,
    Review,
    Accepted,
    Contested,
    Unsupported,
    Inconclusive,
    Superseded,
    Retracted,
}

impl FindingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Accepted => "accepted",
            Self::Contested => "contested",
            Self::Unsupported => "unsupported",
            Self::Inconclusive => "inconclusive",
            Self::Superseded => "superseded",
            Self::Retracted => "retracted",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "draft" => Ok(Self::Draft),
            "review" => Ok(Self::Review),
            "accepted" => Ok(Self::Accepted),
            "contested" => Ok(Self::Contested),
            "unsupported" => Ok(Self::Unsupported),
            "inconclusive" => Ok(Self::Inconclusive),
            "superseded" => Ok(Self::Superseded),
            "retracted" => Ok(Self::Retracted),
            _ => Err(format!("invalid finding status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FindingEvidenceLinkReq {
    pub evidence_id: String,
    pub synthesis_polarity: EvidencePolarity,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateFindingVersionReq {
    pub project_id: String,
    pub finding_id: Option<String>,
    pub parent_version_id: Option<String>,
    pub title: String,
    pub statement: String,
    pub proposition: PropositionRefDto,
    pub scope: String,
    pub applicability: String,
    pub uncertainty: String,
    pub limitations: String,
    pub conclusion_kind: FindingConclusionKind,
    pub analysis_run_ids: Vec<String>,
    pub evidence: Vec<FindingEvidenceLinkReq>,
    pub change_rationale: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransitionFindingReq {
    pub version_id: String,
    pub next_status: FindingStatus,
    pub rationale: String,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingRelationKind {
    Contests,
    Contradicts,
    Qualifies,
    Supersedes,
}

impl FindingRelationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Contests => "contests",
            Self::Contradicts => "contradicts",
            Self::Qualifies => "qualifies",
            Self::Supersedes => "supersedes",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateFindingRelationReq {
    pub source_version_id: String,
    pub target_version_id: String,
    pub relation_kind: FindingRelationKind,
    pub rationale: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FindingEvidenceLinkDto {
    pub evidence_id: String,
    pub synthesis_polarity: EvidencePolarity,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FindingEventDto {
    pub id: String,
    pub status: FindingStatus,
    pub rationale: String,
    pub actor_id: Option<String>,
    pub occurred_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FindingRelationDto {
    pub id: String,
    pub target_version_id: String,
    pub relation_kind: FindingRelationKind,
    pub rationale: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FindingVersionDto {
    pub id: String,
    pub finding_id: String,
    pub project_id: String,
    pub version_no: i64,
    pub parent_version_id: Option<String>,
    pub change_rationale: Option<String>,
    pub title: String,
    pub statement: String,
    pub proposition: PropositionRefDto,
    pub scope: String,
    pub applicability: String,
    pub uncertainty: String,
    pub limitations: String,
    pub conclusion_kind: FindingConclusionKind,
    pub analysis_run_ids: Vec<String>,
    pub evidence: Vec<FindingEvidenceLinkDto>,
    pub status: FindingStatus,
    pub events: Vec<FindingEventDto>,
    pub relations: Vec<FindingRelationDto>,
    pub content_sha256: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisWorkspaceDto {
    pub methods: Vec<AnalysisMethodVersionDto>,
    pub metrics: Vec<MetricDefinitionVersionDto>,
    pub definitions: Vec<AnalysisDefinitionVersionDto>,
    pub runs: Vec<AnalysisRunDto>,
    pub evidence: Vec<EvidenceDto>,
    pub findings: Vec<FindingVersionDto>,
}
