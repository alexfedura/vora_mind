use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod ipc;
pub mod repository;
pub mod service;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatasetRole {
    Training,
    Validation,
    Test,
    Exploratory,
    Prospective,
    Forward,
}

impl DatasetRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Training => "training",
            Self::Validation => "validation",
            Self::Test => "test",
            Self::Exploratory => "exploratory",
            Self::Prospective => "prospective",
            Self::Forward => "forward",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentRunStatus {
    Created,
    Ready,
    Running,
    Completed,
    Failed,
    Cancelled,
    Inconclusive,
}

impl ExperimentRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Inconclusive => "inconclusive",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::Inconclusive
        )
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "created" => Ok(Self::Created),
            "ready" => Ok(Self::Ready),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "inconclusive" => Ok(Self::Inconclusive),
            _ => Err(format!("invalid experiment run status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StudyType {
    RandomizedControlled,
    QuasiExperimental,
    Observational,
    MarketBacktest,
    MarketForward,
    CaseStudy,
    Other,
}

impl StudyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RandomizedControlled => "randomized_controlled",
            Self::QuasiExperimental => "quasi_experimental",
            Self::Observational => "observational",
            Self::MarketBacktest => "market_backtest",
            Self::MarketForward => "market_forward",
            Self::CaseStudy => "case_study",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatasetSampleValueKind {
    Decimal,
    Integer,
    Text,
    Category,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissingValueState {
    NotObserved,
    NotApplicable,
    Censored,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DatasetSampleValue {
    Decimal {
        value: String,
        unit: String,
        uncertainty: Option<String>,
    },
    Integer {
        value: i64,
        unit: Option<String>,
    },
    Text {
        value: String,
    },
    Category {
        value: String,
    },
    Boolean {
        value: bool,
    },
    Missing {
        state: MissingValueState,
        unit: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatasetSampleKind {
    DataSample,
    TradeSample,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SampleProvenanceRef {
    pub reference_kind: String,
    pub reference_id: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataSampleDto {
    pub sample_id: String,
    pub dataset_version_id: String,
    pub occurred_at_us: Option<i64>,
    pub entity_ref: Option<String>,
    pub values: std::collections::BTreeMap<String, DatasetSampleValue>,
    pub provenance: Vec<SampleProvenanceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DecimalMeasurement {
    pub value: String,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarketOrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarketOrderType {
    Market,
    Limit,
    Stop,
    StopLimit,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TradeSampleDto {
    #[serde(flatten)]
    pub sample: DataSampleDto,
    pub instrument: String,
    pub order_side: Option<MarketOrderSide>,
    pub order_type: Option<MarketOrderType>,
    pub order_quantity: Option<DecimalMeasurement>,
    pub execution_price: Option<DecimalMeasurement>,
    pub fees: Option<DecimalMeasurement>,
    pub slippage: Option<DecimalMeasurement>,
    pub venue: Option<String>,
    pub market_session: Option<String>,
    pub session_assumptions: Vec<String>,
    pub decision_at_us: Option<i64>,
    pub execution_at_us: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "sampleKind", content = "sample", rename_all = "snake_case")]
pub enum DatasetSampleDto {
    DataSample(DataSampleDto),
    TradeSample(TradeSampleDto),
}

impl DatasetSampleDto {
    pub fn data_sample(&self) -> &DataSampleDto {
        match self {
            Self::DataSample(sample) => sample,
            Self::TradeSample(sample) => &sample.sample,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::DataSample(_) => "data",
            Self::TradeSample(_) => "trade",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatasetFieldDefinitionDto {
    pub key: String,
    pub value_kind: DatasetSampleValueKind,
    pub required: bool,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatasetSampleSchemaDto {
    pub schema_id: String,
    pub schema_version: i64,
    pub fields: Vec<DatasetFieldDefinitionDto>,
    pub allow_additional_fields: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatasetRolePinDto {
    pub dataset_version_id: String,
    pub role: DatasetRole,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationalDefinitionDto {
    pub name: String,
    pub definition: String,
    pub unit: Option<String>,
    pub measurement_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonGroupDto {
    pub name: String,
    pub description: String,
    pub intervention: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DataCollectionScheduleEntryDto {
    pub stage: String,
    pub schedule: String,
    pub instrument_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeEnvironmentIdentityDto {
    pub code_uri: String,
    pub code_revision: Option<String>,
    pub environment: String,
    pub dependency_lock_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MarketDesignContextDto {
    pub instrument: String,
    pub order_execution_context: String,
    pub fee_assumptions: String,
    pub slippage_assumptions: String,
    pub market_session_assumptions: Vec<String>,
    pub decision_timestamp_policy: String,
    pub execution_timestamp_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentDesignSpecificationDto {
    pub target_research_question_id: Option<String>,
    pub target_hypothesis_version_id: Option<String>,
    pub target_model_version_id: Option<String>,
    pub estimand: String,
    pub study_type: StudyType,
    pub intervention: Option<String>,
    pub control: Option<String>,
    pub comparison_groups: Vec<ComparisonGroupDto>,
    pub factors: Vec<OperationalDefinitionDto>,
    pub outcomes: Vec<OperationalDefinitionDto>,
    pub covariates: Vec<OperationalDefinitionDto>,
    pub unit_of_analysis: String,
    pub sampling_frame: String,
    pub eligibility_rules: Vec<String>,
    pub exclusion_rules: Vec<String>,
    pub planned_sample_size: Option<i64>,
    pub sample_size_rationale: String,
    pub stopping_rule: String,
    pub randomization: Option<String>,
    pub blinding: Option<String>,
    pub instruments: Vec<String>,
    pub operational_definitions: Vec<OperationalDefinitionDto>,
    pub data_collection_schedule: Vec<DataCollectionScheduleEntryDto>,
    pub primary_outcomes: Vec<String>,
    pub secondary_outcomes: Vec<String>,
    pub preregistered_analysis_plan: String,
    pub multiplicity_strategy: String,
    pub deviations_policy: String,
    pub datasets: Vec<DatasetRolePinDto>,
    pub dataset_plan_rationale: String,
    pub code_environment: CodeEnvironmentIdentityDto,
    pub random_seeds: Vec<String>,
    pub reproducibility_instructions: String,
    pub market_context: Option<MarketDesignContextDto>,
}

impl ExperimentDesignSpecificationDto {
    pub fn validate_for_preregistration(&self) -> Result<(), String> {
        if self.target_research_question_id.is_none()
            && self.target_hypothesis_version_id.is_none()
            && self.target_model_version_id.is_none()
        {
            return Err(
                "A preregistered design must target a research question, hypothesis version, or model version."
                    .into(),
            );
        }
        for (field, value) in [
            ("estimand", self.estimand.as_str()),
            ("unit of analysis", self.unit_of_analysis.as_str()),
            ("sampling frame", self.sampling_frame.as_str()),
            ("sample-size rationale", self.sample_size_rationale.as_str()),
            ("stopping rule", self.stopping_rule.as_str()),
            (
                "preregistered analysis plan",
                self.preregistered_analysis_plan.as_str(),
            ),
            ("multiplicity strategy", self.multiplicity_strategy.as_str()),
            ("deviations policy", self.deviations_policy.as_str()),
            ("code identity", self.code_environment.code_uri.as_str()),
            (
                "environment identity",
                self.code_environment.environment.as_str(),
            ),
            (
                "reproducibility instructions",
                self.reproducibility_instructions.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(format!("A preregistered design requires {field}."));
            }
        }
        if !self
            .primary_outcomes
            .iter()
            .any(|outcome| !outcome.trim().is_empty())
            || self.outcomes.is_empty()
        {
            return Err(
                "A preregistered design requires at least one defined primary outcome.".into(),
            );
        }
        if self.planned_sample_size.is_none_or(|size| size <= 0) {
            return Err("A preregistered design requires a positive planned sample size.".into());
        }
        if self.datasets.is_empty() && self.dataset_plan_rationale.trim().is_empty() {
            return Err(
                "A preregistered design must pin exact dataset versions or explain the prospective data plan."
                    .into(),
            );
        }
        for definition in self
            .factors
            .iter()
            .chain(&self.outcomes)
            .chain(&self.covariates)
            .chain(&self.operational_definitions)
        {
            if definition.name.trim().is_empty() || definition.definition.trim().is_empty() {
                return Err("Operational definitions require a name and definition.".into());
            }
        }
        if self.study_type == StudyType::MarketBacktest
            || self.study_type == StudyType::MarketForward
        {
            let market = self
                .market_context
                .as_ref()
                .ok_or("Market studies require explicit market and execution assumptions.")?;
            if [
                market.instrument.as_str(),
                market.order_execution_context.as_str(),
                market.fee_assumptions.as_str(),
                market.slippage_assumptions.as_str(),
                market.decision_timestamp_policy.as_str(),
                market.execution_timestamp_policy.as_str(),
            ]
            .iter()
            .any(|value| value.trim().is_empty())
            {
                return Err("Market study context fields cannot be blank.".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentPreregistrationDto {
    pub id: String,
    pub design_version_id: String,
    pub specification_sha256: String,
    pub reviewer_id: Option<String>,
    pub rationale: String,
    pub preregistered_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentDesignVersionDto {
    pub id: String,
    pub design_id: String,
    pub project_id: String,
    pub title: String,
    pub version_no: i64,
    pub description: String,
    pub protocol: String,
    pub lifecycle_status: crate::research::ResearchDesignStatus,
    pub specification: Option<ExperimentDesignSpecificationDto>,
    pub specification_sha256: Option<String>,
    pub parent_version_id: Option<String>,
    pub change_rationale: Option<String>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub preregistration: Option<ExperimentPreregistrationDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentDesignDatasetPinDto {
    pub dataset_version_id: String,
    pub dataset_id: String,
    pub dataset_title: String,
    pub version_no: i64,
    pub role: DatasetRole,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentRunDatasetPinDto {
    pub dataset_version_id: String,
    pub dataset_id: String,
    pub dataset_title: String,
    pub version_no: i64,
    pub role: DatasetRole,
    pub usage_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentRunEventDto {
    pub id: String,
    pub event_kind: String,
    pub from_status: Option<ExperimentRunStatus>,
    pub to_status: Option<ExperimentRunStatus>,
    pub actor_id: Option<String>,
    pub details: String,
    pub occurred_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentRunDeviationDto {
    pub id: String,
    pub description: String,
    pub rationale: String,
    pub impact_assessment: String,
    pub recorded_by: Option<String>,
    pub recorded_at_us: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunLogSeverity {
    Debug,
    Info,
    Warning,
    Error,
}

impl RunLogSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentRunLogDto {
    pub id: String,
    pub severity: RunLogSeverity,
    pub message: String,
    pub details: serde_json::Value,
    pub recorded_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentRunDto {
    pub id: String,
    pub project_id: String,
    pub design_version_id: String,
    pub lifecycle_status: ExperimentRunStatus,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub started_at_us: Option<i64>,
    pub ended_at_us: Option<i64>,
    pub completion_summary: Option<String>,
    pub failure_details: Option<String>,
    pub dataset_pins: Vec<ExperimentRunDatasetPinDto>,
    pub events: Vec<ExperimentRunEventDto>,
    pub deviations: Vec<ExperimentRunDeviationDto>,
    pub logs: Vec<ExperimentRunLogDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentDatasetDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub versions: Vec<ExperimentDatasetVersionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentDatasetVersionDto {
    pub id: String,
    pub dataset_id: String,
    pub version_no: i64,
    pub schema: DatasetSampleSchemaDto,
    pub expected_sample_count: i64,
    pub actual_sample_count: i64,
    pub lifecycle_status: String,
    pub content_sha256: Option<String>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub sealed_at_us: Option<i64>,
    pub assets: Vec<ExperimentDatasetAssetDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentDatasetAssetDto {
    pub id: String,
    pub dataset_version_id: String,
    pub storage_key: String,
    pub original_name: String,
    pub media_type: String,
    pub byte_length: i64,
    pub content_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentDatasetSplitGroupDto {
    pub id: String,
    pub dataset_version_id: String,
    pub split_group_key: String,
    pub require_disjoint: bool,
    pub assignments: Vec<ExperimentSampleSplitAssignmentDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentSampleSplitAssignmentDto {
    pub id: String,
    pub sample_id: String,
    pub role: DatasetRole,
    pub split_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentWorkspaceDto {
    pub design_versions: Vec<ExperimentDesignVersionDto>,
    pub runs: Vec<ExperimentRunDto>,
    pub datasets: Vec<ExperimentDatasetDto>,
    pub split_groups: Vec<ExperimentDatasetSplitGroupDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatasetSamplePageDto {
    pub dataset_version_id: String,
    pub samples: Vec<DatasetSampleDto>,
    pub total_count: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateExperimentDesignVersionReq {
    pub project_id: String,
    pub design_id: Option<String>,
    pub parent_version_id: Option<String>,
    pub title: String,
    pub description: String,
    pub protocol: String,
    pub change_rationale: String,
    pub specification: ExperimentDesignSpecificationDto,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreregisterExperimentDesignReq {
    pub design_version_id: String,
    pub reviewer_id: Option<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateExperimentRunReq {
    pub design_version_id: String,
    pub dataset_pins: Vec<DatasetRolePinDto>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransitionExperimentRunReq {
    pub run_id: String,
    pub next_status: ExperimentRunStatus,
    pub actor_id: Option<String>,
    pub details: String,
    pub completion_summary: Option<String>,
    pub failure_details: Option<String>,
    pub output_dataset_pins: Vec<DatasetRolePinDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddExperimentRunDeviationReq {
    pub run_id: String,
    pub description: String,
    pub rationale: String,
    pub impact_assessment: String,
    pub recorded_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddExperimentRunLogReq {
    pub run_id: String,
    pub severity: RunLogSeverity,
    pub message: String,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDatasetReq {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDatasetVersionReq {
    pub dataset_id: String,
    pub schema: DatasetSampleSchemaDto,
    pub expected_sample_count: i64,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddDatasetSamplesReq {
    pub dataset_version_id: String,
    pub samples: Vec<DatasetSampleDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDatasetAssetReq {
    pub dataset_version_id: String,
    pub source_path: String,
    pub original_name: String,
    pub media_type: String,
    pub expected_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddDatasetSplitGroupReq {
    pub dataset_version_id: String,
    pub split_group_key: String,
    pub require_disjoint: bool,
    pub assignments: Vec<DatasetSplitAssignmentReq>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatasetSplitAssignmentReq {
    pub sample_id: String,
    pub role: DatasetRole,
    pub split_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDatasetTransformationReq {
    pub output_dataset_version_id: String,
    pub method: String,
    pub parameters: serde_json::Value,
    pub provenance: serde_json::Value,
    pub inputs: Vec<DatasetTransformationInputReq>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatasetTransformationInputReq {
    pub dataset_version_id: String,
    pub role: DatasetRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SealDatasetVersionReq {
    pub dataset_version_id: String,
}

pub fn now_ts_us() -> Result<i64, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_micros();
    i64::try_from(timestamp).map_err(|error| error.to_string())
}

pub fn validate_decimal(value: &str) -> Result<(), String> {
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    if unsigned.is_empty() {
        return Err("decimal value cannot be empty".into());
    }
    let mut parts = unsigned.split('.');
    let integer_part = parts.next().unwrap_or_default();
    let fraction_part = parts.next();
    let valid_integer = !integer_part.is_empty()
        && integer_part.bytes().all(|byte| byte.is_ascii_digit())
        && (integer_part == "0" || !integer_part.starts_with('0'));
    let valid_fraction = fraction_part.is_none_or(|part| {
        !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
    });
    if !valid_integer || !valid_fraction || parts.next().is_some() {
        return Err(format!("invalid canonical decimal: {value}"));
    }
    Ok(())
}

pub fn validate_sample_schema(schema: &DatasetSampleSchemaDto) -> Result<(), String> {
    if schema.schema_id.trim().is_empty() || schema.schema_version != 1 {
        return Err("dataset sample schema requires an ID and supported version 1".into());
    }
    let mut keys = std::collections::HashSet::new();
    for field in &schema.fields {
        if field.key.trim().is_empty() || !keys.insert(field.key.as_str()) {
            return Err("dataset sample schema field keys must be non-empty and unique".into());
        }
        if field.value_kind == DatasetSampleValueKind::Decimal
            && field.unit.as_deref().is_none_or(str::is_empty)
        {
            return Err(format!(
                "decimal sample field '{}' requires an explicit unit",
                field.key
            ));
        }
    }
    Ok(())
}

pub fn validate_sample(
    schema: &DatasetSampleSchemaDto,
    sample: &DatasetSampleDto,
) -> Result<(), String> {
    let base = sample.data_sample();
    if base.sample_id.trim().is_empty() || base.dataset_version_id.trim().is_empty() {
        return Err("dataset samples require a stable sample ID and exact dataset version".into());
    }
    for field in &schema.fields {
        let value = base.values.get(&field.key);
        if value.is_none() && field.required {
            return Err(format!(
                "required dataset sample field '{}' is missing",
                field.key
            ));
        }
        if let Some(value) = value {
            validate_sample_value(field, value)?;
        }
    }
    if !schema.allow_additional_fields {
        for key in base.values.keys() {
            if !schema.fields.iter().any(|field| field.key == *key) {
                return Err(format!("undeclared dataset sample field '{key}'"));
            }
        }
    }
    if let DatasetSampleDto::TradeSample(trade) = sample {
        if trade.instrument.trim().is_empty() {
            return Err("trade samples require an instrument".into());
        }
        if let (Some(decision), Some(execution)) = (trade.decision_at_us, trade.execution_at_us) {
            if execution < decision {
                return Err("trade execution time cannot precede decision time".into());
            }
        }
        for value in [
            trade.order_quantity.as_ref(),
            trade.execution_price.as_ref(),
            trade.fees.as_ref(),
            trade.slippage.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            validate_decimal(&value.value)?;
            if value.unit.trim().is_empty() {
                return Err("trade market values require explicit units".into());
            }
        }
    }
    Ok(())
}

fn validate_sample_value(
    field: &DatasetFieldDefinitionDto,
    value: &DatasetSampleValue,
) -> Result<(), String> {
    let valid_kind = matches!(
        (value, &field.value_kind),
        (
            DatasetSampleValue::Decimal { .. },
            DatasetSampleValueKind::Decimal
        ) | (
            DatasetSampleValue::Integer { .. },
            DatasetSampleValueKind::Integer
        ) | (
            DatasetSampleValue::Text { .. },
            DatasetSampleValueKind::Text
        ) | (
            DatasetSampleValue::Category { .. },
            DatasetSampleValueKind::Category
        ) | (
            DatasetSampleValue::Boolean { .. },
            DatasetSampleValueKind::Boolean
        ) | (DatasetSampleValue::Missing { .. }, _)
    );
    if !valid_kind {
        return Err(format!(
            "dataset sample field '{}' does not match its declared type",
            field.key
        ));
    }
    match value {
        DatasetSampleValue::Decimal {
            value,
            unit,
            uncertainty,
        } => {
            validate_decimal(value)?;
            if let Some(uncertainty) = uncertainty {
                validate_decimal(uncertainty)?;
            }
            if unit.trim().is_empty()
                || field
                    .unit
                    .as_deref()
                    .is_some_and(|expected| expected != unit)
            {
                return Err(format!(
                    "dataset sample field '{}' has an invalid or mismatched unit",
                    field.key
                ));
            }
        }
        DatasetSampleValue::Integer { unit, .. } => {
            if field
                .unit
                .as_deref()
                .is_some_and(|expected| unit.as_deref() != Some(expected))
            {
                return Err(format!(
                    "dataset sample field '{}' has a mismatched unit",
                    field.key
                ));
            }
        }
        DatasetSampleValue::Missing { unit, .. } => {
            if field
                .unit
                .as_deref()
                .is_some_and(|expected| unit.as_deref() != Some(expected))
            {
                return Err(format!(
                    "missing dataset sample field '{}' has a mismatched unit",
                    field.key
                ));
            }
        }
        DatasetSampleValue::Text { .. }
        | DatasetSampleValue::Category { .. }
        | DatasetSampleValue::Boolean { .. } => {}
    }
    Ok(())
}
