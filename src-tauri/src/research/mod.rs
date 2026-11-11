use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub mod ipc;
pub mod repository;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchPathwayKey {
    ExistingKnowledgeAcquisition,
    ExistingKnowledgeInvestigation,
    FirstPrinciplesDiscovery,
}

impl ResearchPathwayKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ExistingKnowledgeAcquisition => "existing_knowledge_acquisition",
            Self::ExistingKnowledgeInvestigation => "existing_knowledge_investigation",
            Self::FirstPrinciplesDiscovery => "first_principles_discovery",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "existing_knowledge_acquisition" => Ok(Self::ExistingKnowledgeAcquisition),
            "existing_knowledge_investigation" => Ok(Self::ExistingKnowledgeInvestigation),
            "first_principles_discovery" => Ok(Self::FirstPrinciplesDiscovery),
            _ => Err(format!("invalid research pathway: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchProjectStatus {
    Proposed,
    Active,
    Paused,
    Completed,
    Archived,
}

impl ResearchProjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchGapStatus {
    Candidate,
    Assessed,
    Accepted,
    Rejected,
    Addressed,
    Superseded,
}

impl ResearchGapStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Assessed => "assessed",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Addressed => "addressed",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchQuestionStatus {
    LegacyUnassessed,
    Draft,
    Review,
    Accepted,
    Rejected,
    Deferred,
    Refined,
    Superseded,
}

impl ResearchQuestionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LegacyUnassessed => "legacy_unassessed",
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Deferred => "deferred",
            Self::Refined => "refined",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchQuestionRelation {
    DerivedFrom,
    RefinedFrom,
    Supersedes,
}

impl ResearchQuestionRelation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DerivedFrom => "derived_from",
            Self::RefinedFrom => "refined_from",
            Self::Supersedes => "supersedes",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchHypothesisStatus {
    LegacyUntyped,
    Draft,
    Review,
    Preregistered,
    Closed,
}

impl ResearchHypothesisStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LegacyUntyped => "legacy_untyped",
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Preregistered => "preregistered",
            Self::Closed => "closed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisDefinitionStatus {
    Draft,
    Review,
    Preregistered,
}

impl HypothesisDefinitionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Preregistered => "preregistered",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisVariableRole {
    Independent,
    Dependent,
    Covariate,
}

impl HypothesisVariableRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Independent => "independent",
            Self::Dependent => "dependent",
            Self::Covariate => "covariate",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisValueKind {
    Continuous,
    Discrete,
    Categorical,
    Qualitative,
    Other,
}

impl HypothesisValueKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Continuous => "continuous",
            Self::Discrete => "discrete",
            Self::Categorical => "categorical",
            Self::Qualitative => "qualitative",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchPolarity {
    Supports,
    Contradicts,
    Qualifies,
}

impl ResearchPolarity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Supports => "supports",
            Self::Contradicts => "contradicts",
            Self::Qualifies => "qualifies",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchPathwayDto {
    pub id: String,
    pub pathway_key: ResearchPathwayKey,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchProjectDto {
    pub id: String,
    pub title: String,
    pub aim: String,
    pub lifecycle_status: ResearchProjectStatus,
    pub pathway_keys: Vec<ResearchPathwayKey>,
    pub domain_ids: Vec<String>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub updated_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchDomainDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub parent_domain_id: Option<String>,
    pub lifecycle_status: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkVersionReferenceDto {
    pub framework_id: String,
    pub framework_version_id: String,
    pub title: String,
    pub version_no: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceVersionReferenceDto {
    pub source_id: String,
    pub source_version_id: String,
    pub source_title: String,
    pub edition_label: Option<String>,
    pub source_location_id: Option<String>,
    pub source_location_label: Option<String>,
    pub source_claim_id: Option<String>,
    pub captured_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectKnowledgeAdoptionDto {
    pub id: String,
    pub project_id: String,
    pub framework_id: String,
    pub framework_version_id: String,
    pub title: String,
    pub version_no: i64,
    pub use_context: String,
    pub adopted_by: Option<String>,
    pub adopted_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LiteratureReviewSourceDto {
    pub id: String,
    pub source_id: String,
    pub source_version_id: String,
    pub source_location_id: Option<String>,
    pub source_claim_id: Option<String>,
    pub citation_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LiteratureReviewDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub search_scope: String,
    pub lifecycle_status: String,
    pub reviewer_id: Option<String>,
    pub reviewed_at_us: Option<i64>,
    pub sources: Vec<LiteratureReviewSourceDto>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PriorKnowledgeAssessmentDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub assessment: String,
    pub framework_version_id: Option<String>,
    pub lifecycle_status: String,
    pub reviewer_id: Option<String>,
    pub reviewed_at_us: Option<i64>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchGapDto {
    pub id: String,
    pub project_id: String,
    pub domain_id: Option<String>,
    pub title: String,
    pub statement: String,
    pub lifecycle_status: ResearchGapStatus,
    pub literature_review_ids: Vec<String>,
    pub prior_assessment_ids: Vec<String>,
    pub has_reviewed_exception: bool,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub updated_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchQuestionDto {
    pub id: String,
    pub title: String,
    pub description: String,
    pub project_id: Option<String>,
    pub research_gap_id: Option<String>,
    pub domain_id: Option<String>,
    pub lifecycle_status: ResearchQuestionStatus,
    pub ownership_state: String,
    pub migration_note: Option<String>,
    pub rationale: String,
    pub legacy_status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchQuestionLineageDto {
    pub id: String,
    pub parent_question_id: String,
    pub derived_question_id: String,
    pub relation_kind: ResearchQuestionRelation,
    pub rationale: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PhenomenonDto {
    pub id: String,
    pub project_id: String,
    pub domain_id: Option<String>,
    pub title: String,
    pub description: String,
    pub population_scope: String,
    pub context: String,
    pub temporal_boundary: String,
    pub lifecycle_status: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObservationPlanDto {
    pub id: String,
    pub project_id: String,
    pub phenomenon_id: String,
    pub title: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub versions: Vec<ObservationPlanVersionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObservationPlanVersionDto {
    pub id: String,
    pub plan_id: String,
    pub version_no: i64,
    pub measurement_method: String,
    pub sample_frame: String,
    pub conditions_template: String,
    pub inclusion_rules: String,
    pub lifecycle_status: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub preregistered_at_us: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatasetVersionReferenceDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub version_label: String,
    pub external_uri: Option<String>,
    pub content_sha256: Option<String>,
    pub source_version_id: Option<String>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObservationDto {
    pub id: String,
    pub project_id: String,
    pub phenomenon_id: String,
    pub plan_id: String,
    pub plan_version_id: String,
    pub dataset_version_reference_id: Option<String>,
    pub measured_at_us: i64,
    pub value_decimal: Option<String>,
    pub value_state: String,
    pub unit: Option<String>,
    pub measurement_method: String,
    pub conditions: String,
    pub sample_identifier: Option<String>,
    pub quality_status: String,
    pub supersedes_observation_id: Option<String>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PatternDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub statement: String,
    pub analysis_method: String,
    pub uncertainty: String,
    pub lifecycle_status: String,
    pub observation_ids: Vec<String>,
    pub dataset_version_reference_ids: Vec<String>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdeaDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub content: String,
    pub lifecycle_status: String,
    pub phenomenon_id: Option<String>,
    pub observation_id: Option<String>,
    pub pattern_id: Option<String>,
    pub question_id: Option<String>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HypothesisVariableDto {
    pub id: String,
    pub hypothesis_version_id: String,
    pub role: HypothesisVariableRole,
    pub name: String,
    pub operational_definition: String,
    pub value_kind: HypothesisValueKind,
    pub unit: Option<String>,
    pub measurement_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HypothesisDefinitionVersionDto {
    pub id: String,
    pub hypothesis_id: String,
    pub version_no: i64,
    pub parent_version_id: Option<String>,
    pub statement_h0: String,
    pub statement_h1: String,
    pub population_scope: String,
    pub research_scope: String,
    pub expected_direction: String,
    pub expected_effect_decimal: Option<String>,
    pub expected_effect_unit: Option<String>,
    pub falsification_criteria: String,
    pub assumptions: String,
    pub lifecycle_status: HypothesisDefinitionStatus,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub preregistered_at_us: Option<i64>,
    pub variables: Vec<HypothesisVariableDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HypothesisModelReferenceDto {
    pub id: String,
    pub hypothesis_id: String,
    pub model_id: String,
    pub model_version_id: String,
    pub label: String,
    pub rationale: String,
    pub resolution_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HypothesisEvidenceDto {
    pub target_kind: String,
    pub target_id: String,
    pub polarity: ResearchPolarity,
    pub rationale: String,
    pub source_id: Option<String>,
    pub source_version_id: Option<String>,
    pub source_location_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchHypothesisDto {
    pub id: String,
    pub project_id: Option<String>,
    pub research_question_id: Option<String>,
    pub legacy_hypothesis_id: Option<String>,
    pub parent_hypothesis_id: Option<String>,
    pub ownership_state: String,
    pub lifecycle_status: ResearchHypothesisStatus,
    pub definitions: Vec<HypothesisDefinitionVersionDto>,
    pub competing_hypothesis_ids: Vec<String>,
    pub model_references: Vec<HypothesisModelReferenceDto>,
    pub evidence: Vec<HypothesisEvidenceDto>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchExperimentDesignVersionDto {
    pub id: String,
    pub design_id: String,
    pub project_id: String,
    pub title: String,
    pub version_no: i64,
    pub description: String,
    pub protocol: String,
    pub lifecycle_status: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub preregistered_at_us: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchWorkspaceDto {
    pub pathways: Vec<ResearchPathwayDto>,
    pub projects: Vec<ResearchProjectDto>,
    pub domains: Vec<ResearchDomainDto>,
    pub framework_versions: Vec<FrameworkVersionReferenceDto>,
    pub source_versions: Vec<SourceVersionReferenceDto>,
    pub adoptions: Vec<ProjectKnowledgeAdoptionDto>,
    pub literature_reviews: Vec<LiteratureReviewDto>,
    pub prior_assessments: Vec<PriorKnowledgeAssessmentDto>,
    pub gaps: Vec<ResearchGapDto>,
    pub questions: Vec<ResearchQuestionDto>,
    pub question_lineage: Vec<ResearchQuestionLineageDto>,
    pub phenomena: Vec<PhenomenonDto>,
    pub observation_plans: Vec<ObservationPlanDto>,
    pub dataset_versions: Vec<DatasetVersionReferenceDto>,
    pub observations: Vec<ObservationDto>,
    pub patterns: Vec<PatternDto>,
    pub ideas: Vec<IdeaDto>,
    pub hypotheses: Vec<ResearchHypothesisDto>,
    pub design_versions: Vec<ResearchExperimentDesignVersionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateResearchProjectReq {
    pub title: String,
    pub aim: String,
    pub pathways: Vec<ResearchPathwayKey>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateResearchDomainReq {
    pub name: String,
    pub description: String,
    pub parent_domain_id: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateResearchGapReq {
    pub project_id: String,
    pub domain_id: Option<String>,
    pub title: String,
    pub statement: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateResearchQuestionReq {
    pub project_id: Option<String>,
    pub research_gap_id: Option<String>,
    pub domain_id: Option<String>,
    pub title: String,
    pub description: String,
    pub rationale: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateLiteratureReviewReq {
    pub project_id: String,
    pub title: String,
    pub search_scope: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddLiteratureSourceReq {
    pub literature_review_id: String,
    pub source_id: String,
    pub source_version_id: String,
    pub source_location_id: Option<String>,
    pub source_claim_id: Option<String>,
    pub citation_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreatePriorKnowledgeAssessmentReq {
    pub project_id: String,
    pub title: String,
    pub assessment: String,
    pub framework_version_id: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddGapEvidenceReq {
    pub gap_id: String,
    pub literature_review_id: Option<String>,
    pub prior_assessment_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReviewGapExceptionReq {
    pub gap_id: String,
    pub rationale: String,
    pub reviewer_id: Option<String>,
    pub review_decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreatePhenomenonReq {
    pub project_id: String,
    pub domain_id: Option<String>,
    pub title: String,
    pub description: String,
    pub population_scope: String,
    pub context: String,
    pub temporal_boundary: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateObservationPlanReq {
    pub project_id: String,
    pub phenomenon_id: String,
    pub title: String,
    pub measurement_method: String,
    pub sample_frame: String,
    pub conditions_template: String,
    pub inclusion_rules: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDatasetVersionReferenceReq {
    pub project_id: String,
    pub title: String,
    pub version_label: String,
    pub external_uri: Option<String>,
    pub content_sha256: Option<String>,
    pub source_version_id: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateObservationReq {
    pub project_id: String,
    pub phenomenon_id: String,
    pub plan_id: String,
    pub plan_version_id: String,
    pub dataset_version_reference_id: Option<String>,
    pub measured_at_us: i64,
    pub value_decimal: Option<String>,
    pub value_state: String,
    pub unit: Option<String>,
    pub measurement_method: String,
    pub conditions: String,
    pub sample_identifier: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreatePatternReq {
    pub project_id: String,
    pub title: String,
    pub statement: String,
    pub analysis_method: String,
    pub uncertainty: String,
    pub observation_ids: Vec<String>,
    pub dataset_version_reference_ids: Vec<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateIdeaReq {
    pub project_id: String,
    pub title: String,
    pub content: String,
    pub phenomenon_id: Option<String>,
    pub observation_id: Option<String>,
    pub pattern_id: Option<String>,
    pub question_id: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HypothesisVariableInput {
    pub role: HypothesisVariableRole,
    pub name: String,
    pub operational_definition: String,
    pub value_kind: HypothesisValueKind,
    pub unit: Option<String>,
    pub measurement_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateTypedHypothesisReq {
    pub project_id: String,
    pub research_question_id: String,
    pub parent_hypothesis_id: Option<String>,
    pub lineage_rationale: Option<String>,
    pub statement_h0: String,
    pub statement_h1: String,
    pub population_scope: String,
    pub research_scope: String,
    pub expected_direction: String,
    pub expected_effect_decimal: Option<String>,
    pub expected_effect_unit: Option<String>,
    pub falsification_criteria: String,
    pub assumptions: String,
    pub variables: Vec<HypothesisVariableInput>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinkCompetingHypothesesReq {
    pub hypothesis_id: String,
    pub competing_hypothesis_id: String,
    pub rationale: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddHypothesisModelReferenceReq {
    pub hypothesis_id: String,
    pub model_id: String,
    pub model_version_id: String,
    pub label: String,
    pub rationale: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddHypothesisEvidenceReq {
    pub hypothesis_version_id: String,
    pub source_claim_id: Option<String>,
    pub observation_id: Option<String>,
    pub pattern_id: Option<String>,
    pub polarity: ResearchPolarity,
    pub rationale: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateResearchDesignReq {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub protocol: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinkHypothesisDesignReq {
    pub hypothesis_version_id: String,
    pub design_version_id: String,
    pub relationship_kind: String,
    pub rationale: String,
    pub created_by: Option<String>,
}

fn now_ts_us() -> Result<i64, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_micros();
    i64::try_from(timestamp).map_err(|error| error.to_string())
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::now_v7())
}

fn validate_required(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    Ok(())
}

pub fn init_schema(conn: &Connection) -> Result<(), String> {
    repository::initialize_schema(conn).map_err(|error| error.to_string())?;
    synchronize_legacy_records(conn)
}

pub fn synchronize_legacy_records(conn: &Connection) -> Result<(), String> {
    repository::synchronize_legacy_questions(conn).map_err(|error| error.to_string())?;
    repository::synchronize_legacy_hypotheses(conn, now_ts_us()?).map_err(|error| error.to_string())
}

pub fn workspace(conn: &Connection) -> Result<ResearchWorkspaceDto, String> {
    synchronize_legacy_records(conn)?;
    repository::workspace(conn).map_err(|error| error.to_string())
}

pub fn create_project(
    conn: &mut Connection,
    req: CreateResearchProjectReq,
) -> Result<ResearchProjectDto, String> {
    validate_required(&req.title, "project title")?;
    if req.pathways.is_empty() {
        return Err("a research project must select at least one pathway".into());
    }
    let mut keys = req
        .pathways
        .iter()
        .map(ResearchPathwayKey::as_str)
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys.dedup();
    if keys.len() != req.pathways.len() {
        return Err("research pathways must not be duplicated".into());
    }
    repository::create_project(conn, &req, &new_id("research_project"), now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn create_domain(
    conn: &mut Connection,
    req: CreateResearchDomainReq,
) -> Result<ResearchDomainDto, String> {
    validate_required(&req.name, "domain name")?;
    if let Some(parent_id) = req.parent_domain_id.as_deref() {
        repository::validate_domain_parent(conn, parent_id).map_err(|error| error.to_string())?;
    }
    repository::create_domain(conn, &req, &new_id("research_domain"), now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn assign_domain(
    conn: &Connection,
    project_id: &str,
    domain_id: &str,
    rationale: &str,
) -> Result<(), String> {
    repository::assign_domain(conn, project_id, domain_id, rationale, now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn adopt_framework_version(
    conn: &Connection,
    project_id: &str,
    framework_id: &str,
    framework_version_id: &str,
    use_context: &str,
    actor_id: Option<&str>,
) -> Result<ProjectKnowledgeAdoptionDto, String> {
    validate_required(use_context, "knowledge adoption context")?;
    repository::adopt_framework_version(
        conn,
        &new_id("knowledge_adoption"),
        project_id,
        framework_id,
        framework_version_id,
        use_context,
        actor_id,
        now_ts_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn create_literature_review(
    conn: &Connection,
    req: CreateLiteratureReviewReq,
) -> Result<LiteratureReviewDto, String> {
    validate_required(&req.title, "literature review title")?;
    validate_required(&req.search_scope, "literature search scope")?;
    repository::create_literature_review(conn, &req, &new_id("literature_review"), now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn add_literature_source(
    conn: &Connection,
    req: AddLiteratureSourceReq,
) -> Result<LiteratureReviewSourceDto, String> {
    repository::add_literature_source(conn, &req, &new_id("literature_source"))
        .map_err(|error| error.to_string())
}

pub fn review_literature_review(
    conn: &Connection,
    review_id: &str,
    reviewer_id: Option<&str>,
) -> Result<LiteratureReviewDto, String> {
    repository::review_literature_review(conn, review_id, reviewer_id, now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn create_prior_assessment(
    conn: &Connection,
    req: CreatePriorKnowledgeAssessmentReq,
) -> Result<PriorKnowledgeAssessmentDto, String> {
    validate_required(&req.title, "prior-knowledge assessment title")?;
    validate_required(&req.assessment, "prior-knowledge assessment")?;
    repository::create_prior_assessment(conn, &req, &new_id("prior_knowledge"), now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn review_prior_assessment(
    conn: &Connection,
    assessment_id: &str,
    reviewer_id: Option<&str>,
) -> Result<PriorKnowledgeAssessmentDto, String> {
    repository::review_prior_assessment(conn, assessment_id, reviewer_id, now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn create_gap(conn: &Connection, req: CreateResearchGapReq) -> Result<ResearchGapDto, String> {
    validate_required(&req.title, "research gap title")?;
    validate_required(&req.statement, "research gap statement")?;
    repository::create_gap(conn, &req, &new_id("research_gap"), now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn add_gap_evidence(conn: &Connection, req: AddGapEvidenceReq) -> Result<(), String> {
    if req.literature_review_id.is_none() && req.prior_assessment_id.is_none() {
        return Err("a gap must link a literature review or prior-knowledge assessment".into());
    }
    if req.literature_review_id.is_some() && req.prior_assessment_id.is_some() {
        return Err("add one typed gap-evidence relation per request".into());
    }
    repository::add_gap_evidence(conn, &req).map_err(|error| error.to_string())
}

pub fn review_gap_exception(conn: &Connection, req: ReviewGapExceptionReq) -> Result<(), String> {
    validate_required(&req.rationale, "review exception rationale")?;
    if !matches!(req.review_decision.as_str(), "accepted" | "rejected") {
        return Err("review decision must be accepted or rejected".into());
    }
    repository::review_gap_exception(conn, &req, now_ts_us()?).map_err(|error| error.to_string())
}

pub fn transition_gap(
    conn: &Connection,
    gap_id: &str,
    next_status: ResearchGapStatus,
    actor_id: Option<&str>,
) -> Result<ResearchGapDto, String> {
    repository::transition_gap(conn, gap_id, next_status, actor_id, now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn create_question(
    conn: &mut Connection,
    req: CreateResearchQuestionReq,
) -> Result<ResearchQuestionDto, String> {
    validate_required(&req.title, "question title")?;
    repository::create_question(conn, &req, &new_id("question"), now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn transition_question(
    conn: &Connection,
    question_id: &str,
    next_status: ResearchQuestionStatus,
    actor_id: Option<&str>,
    rationale: &str,
) -> Result<ResearchQuestionDto, String> {
    repository::transition_question(
        conn,
        question_id,
        next_status,
        actor_id,
        rationale,
        now_ts_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn refine_question(
    conn: &mut Connection,
    parent_question_id: &str,
    title: &str,
    description: &str,
    rationale: &str,
    actor_id: Option<&str>,
) -> Result<ResearchQuestionDto, String> {
    validate_required(title, "refined question title")?;
    validate_required(rationale, "question refinement rationale")?;
    repository::refine_question(
        conn,
        parent_question_id,
        title,
        description,
        rationale,
        actor_id,
        &new_id("question"),
        now_ts_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn link_question_lineage(
    conn: &Connection,
    parent_question_id: &str,
    derived_question_id: &str,
    relation_kind: ResearchQuestionRelation,
    rationale: &str,
    actor_id: Option<&str>,
) -> Result<ResearchQuestionLineageDto, String> {
    validate_required(rationale, "question lineage rationale")?;
    repository::link_question_lineage(
        conn,
        parent_question_id,
        derived_question_id,
        relation_kind,
        rationale,
        actor_id,
        &new_id("question_lineage"),
        now_ts_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn create_phenomenon(
    conn: &Connection,
    req: CreatePhenomenonReq,
) -> Result<PhenomenonDto, String> {
    validate_required(&req.title, "phenomenon title")?;
    validate_required(&req.description, "phenomenon description")?;
    validate_required(&req.population_scope, "phenomenon population scope")?;
    repository::create_phenomenon(conn, &req, &new_id("phenomenon"), now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn create_observation_plan(
    conn: &mut Connection,
    req: CreateObservationPlanReq,
) -> Result<ObservationPlanDto, String> {
    validate_required(&req.title, "observation-plan title")?;
    validate_required(&req.measurement_method, "measurement method")?;
    validate_required(&req.sample_frame, "observation sample frame")?;
    repository::create_observation_plan(
        conn,
        &req,
        &new_id("observation_plan"),
        &new_id("observation_plan_version"),
        now_ts_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn review_observation_plan(
    conn: &Connection,
    plan_version_id: &str,
    preregister: bool,
    actor_id: Option<&str>,
) -> Result<ObservationPlanVersionDto, String> {
    repository::review_observation_plan(conn, plan_version_id, preregister, actor_id, now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn create_dataset_version_reference(
    conn: &Connection,
    req: CreateDatasetVersionReferenceReq,
) -> Result<DatasetVersionReferenceDto, String> {
    validate_required(&req.title, "dataset title")?;
    validate_required(&req.version_label, "dataset version label")?;
    repository::create_dataset_version_reference(
        conn,
        &req,
        &new_id("dataset_version_reference"),
        now_ts_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn create_observation(
    conn: &Connection,
    req: CreateObservationReq,
) -> Result<ObservationDto, String> {
    validate_required(&req.measurement_method, "observation measurement method")?;
    validate_required(&req.conditions, "observation conditions")?;
    match (
        req.value_state.as_str(),
        req.value_decimal.as_deref(),
        req.unit.as_deref(),
    ) {
        ("observed", Some(value), Some(unit)) => {
            validate_decimal(value, "observation value")?;
            validate_required(unit, "observation unit")?;
        }
        ("observed", _, _) => return Err("observed values require a decimal value and unit".into()),
        (state, None, _)
            if matches!(
                state,
                "not_measured"
                    | "not_recorded"
                    | "unknown"
                    | "not_applicable"
                    | "withheld"
                    | "invalid"
            ) => {}
        _ => return Err("missing observation values require an explicit missingness state".into()),
    }
    repository::create_observation(conn, &req, &new_id("observation"), now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn create_pattern(conn: &mut Connection, req: CreatePatternReq) -> Result<PatternDto, String> {
    validate_required(&req.title, "pattern title")?;
    validate_required(&req.statement, "pattern statement")?;
    validate_required(&req.analysis_method, "pattern analysis method")?;
    if req.observation_ids.is_empty() && req.dataset_version_reference_ids.is_empty() {
        return Err(
            "a pattern must declare at least one observation or dataset-version input".into(),
        );
    }
    repository::create_pattern(conn, &req, &new_id("pattern"), now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn create_idea(conn: &Connection, req: CreateIdeaReq) -> Result<IdeaDto, String> {
    validate_required(&req.title, "idea title")?;
    validate_required(&req.content, "idea content")?;
    let origin_count = [
        req.phenomenon_id.is_some(),
        req.observation_id.is_some(),
        req.pattern_id.is_some(),
        req.question_id.is_some(),
    ]
    .into_iter()
    .filter(|value| *value)
    .count();
    if origin_count != 1 {
        return Err("an idea must name exactly one typed origin".into());
    }
    repository::create_idea(conn, &req, &new_id("idea"), now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn create_typed_hypothesis(
    conn: &mut Connection,
    req: CreateTypedHypothesisReq,
) -> Result<ResearchHypothesisDto, String> {
    if req.parent_hypothesis_id.is_some() {
        validate_required(
            req.lineage_rationale.as_deref().unwrap_or_default(),
            "hypothesis lineage rationale",
        )?;
    } else if req.lineage_rationale.is_some() {
        return Err("hypothesis lineage rationale requires a parent hypothesis".into());
    }
    validate_hypothesis_definition(
        &req.statement_h0,
        &req.statement_h1,
        &req.population_scope,
        &req.research_scope,
        &req.expected_direction,
        &req.falsification_criteria,
        req.expected_effect_decimal.as_deref(),
        req.expected_effect_unit.as_deref(),
        &req.variables,
    )?;
    repository::create_typed_hypothesis(
        conn,
        &req,
        &new_id("hypothesis"),
        &new_id("legacy_hypothesis"),
        &new_id("hypothesis_definition"),
        now_ts_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn transition_hypothesis(
    conn: &Connection,
    hypothesis_id: &str,
    next_status: ResearchHypothesisStatus,
    actor_id: Option<&str>,
) -> Result<ResearchHypothesisDto, String> {
    repository::transition_hypothesis(conn, hypothesis_id, next_status, actor_id, now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn link_competing_hypotheses(
    conn: &Connection,
    req: LinkCompetingHypothesesReq,
) -> Result<(), String> {
    validate_required(&req.rationale, "competition rationale")?;
    repository::link_competing_hypotheses(conn, &req, now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn add_hypothesis_model_reference(
    conn: &Connection,
    req: AddHypothesisModelReferenceReq,
) -> Result<HypothesisModelReferenceDto, String> {
    validate_required(&req.model_id, "model id")?;
    validate_required(&req.model_version_id, "model version id")?;
    validate_required(&req.label, "model reference label")?;
    validate_required(&req.rationale, "model reference rationale")?;
    repository::add_hypothesis_model_reference(
        conn,
        &req,
        &new_id("hypothesis_model_reference"),
        now_ts_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn add_hypothesis_evidence(
    conn: &Connection,
    req: AddHypothesisEvidenceReq,
) -> Result<(), String> {
    let evidence_count = [
        req.source_claim_id.is_some(),
        req.observation_id.is_some(),
        req.pattern_id.is_some(),
    ]
    .into_iter()
    .filter(|value| *value)
    .count();
    if evidence_count != 1 {
        return Err(
            "hypothesis evidence must name exactly one typed source claim, observation, or pattern"
                .into(),
        );
    }
    repository::add_hypothesis_evidence(conn, &req, &new_id("hypothesis_evidence"), now_ts_us()?)
        .map_err(|error| error.to_string())
}

pub fn create_research_design(
    conn: &mut Connection,
    req: CreateResearchDesignReq,
) -> Result<ResearchExperimentDesignVersionDto, String> {
    validate_required(&req.title, "experiment-design title")?;
    validate_required(&req.description, "experiment-design description")?;
    validate_required(&req.protocol, "experiment-design protocol")?;
    repository::create_research_design(
        conn,
        &req,
        &new_id("experiment_design"),
        &new_id("experiment_design_version"),
        now_ts_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn transition_research_design(
    conn: &Connection,
    design_version_id: &str,
    next_status: &str,
    actor_id: Option<&str>,
) -> Result<ResearchExperimentDesignVersionDto, String> {
    if !matches!(next_status, "review" | "preregistered" | "withdrawn") {
        return Err(format!(
            "unsupported design transition target: {next_status}"
        ));
    }
    repository::transition_research_design(
        conn,
        design_version_id,
        next_status,
        actor_id,
        now_ts_us()?,
    )
    .map_err(|error| error.to_string())
}

pub fn link_hypothesis_design(
    conn: &Connection,
    req: LinkHypothesisDesignReq,
) -> Result<(), String> {
    validate_required(&req.rationale, "hypothesis-design rationale")?;
    if !matches!(req.relationship_kind.as_str(), "tests" | "competing_design") {
        return Err(format!(
            "unsupported hypothesis-design relation: {}",
            req.relationship_kind
        ));
    }
    repository::link_hypothesis_design(conn, &req, now_ts_us()?).map_err(|error| error.to_string())
}

fn validate_decimal(value: &str, name: &str) -> Result<(), String> {
    let trimmed = value.trim();
    let bytes = trimmed.as_bytes();
    if bytes.is_empty()
        || bytes
            .iter()
            .any(|byte| !byte.is_ascii_digit() && *byte != b'.' && *byte != b'-')
        || bytes.iter().filter(|byte| **byte == b'.').count() > 1
        || bytes.iter().filter(|byte| **byte == b'-').count() > 1
        || bytes.iter().skip(1).any(|byte| *byte == b'-')
    {
        return Err(format!("{name} must be a decimal string"));
    }
    trimmed
        .parse::<f64>()
        .map_err(|_| format!("{name} must be a decimal string"))?;
    Ok(())
}

fn validate_hypothesis_definition(
    h0: &str,
    h1: &str,
    population: &str,
    scope: &str,
    direction: &str,
    falsification: &str,
    effect: Option<&str>,
    effect_unit: Option<&str>,
    variables: &[HypothesisVariableInput],
) -> Result<(), String> {
    for (value, label) in [
        (h0, "null hypothesis"),
        (h1, "alternative hypothesis"),
        (population, "hypothesis population/scope"),
        (scope, "hypothesis research scope"),
        (falsification, "falsification criteria"),
    ] {
        validate_required(value, label)?;
    }
    if !matches!(
        direction,
        "positive" | "negative" | "non_directional" | "equivalence" | "other"
    ) {
        return Err("expected direction must be a supported typed value".into());
    }
    if effect.is_some() != effect_unit.is_some() {
        return Err("an expected effect requires both a decimal and its unit".into());
    }
    if let Some(effect) = effect {
        validate_decimal(effect, "expected effect")?;
    }
    let independent_count = variables
        .iter()
        .filter(|variable| variable.role == HypothesisVariableRole::Independent)
        .count();
    let dependent_count = variables
        .iter()
        .filter(|variable| variable.role == HypothesisVariableRole::Dependent)
        .count();
    if independent_count == 0 || dependent_count == 0 {
        return Err(
            "typed hypotheses require at least one independent and one dependent variable".into(),
        );
    }
    for variable in variables {
        validate_required(&variable.name, "variable name")?;
        validate_required(
            &variable.operational_definition,
            "variable operational definition",
        )?;
        validate_required(&variable.measurement_method, "variable measurement method")?;
        if variable.value_kind == HypothesisValueKind::Continuous
            || variable.value_kind == HypothesisValueKind::Discrete
        {
            validate_required(
                variable.unit.as_deref().unwrap_or_default(),
                "variable unit",
            )?;
        }
    }
    Ok(())
}
