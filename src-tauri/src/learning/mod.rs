use serde::{Deserialize, Serialize};

pub mod ipc;
pub mod repository;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LearningVersionStatus {
    Draft,
    Review,
    Published,
    Deprecated,
}

impl LearningVersionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Published => "published",
            Self::Deprecated => "deprecated",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "draft" => Ok(Self::Draft),
            "review" => Ok(Self::Review),
            "published" => Ok(Self::Published),
            "deprecated" => Ok(Self::Deprecated),
            _ => Err(format!("invalid learning version status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentReviewStatus {
    Approved,
    ChangesRequested,
    Rejected,
}

impl AssessmentReviewStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::ChangesRequested => "changes_requested",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentOutcome {
    Pass,
    Fail,
    Inconclusive,
}

impl AssessmentOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MasteryOutcome {
    Mastered,
    NotMastered,
}

impl MasteryOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mastered => "mastered",
            Self::NotMastered => "not_mastered",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LearningLearnerDto {
    pub id: String,
    pub display_name: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LearningProjectDto {
    pub id: String,
    pub title: String,
    pub learner_id: String,
    pub learner_name: String,
    pub framework_id: String,
    pub framework_version_id: String,
    pub framework_title: String,
    pub framework_version_no: i64,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LearningFrameworkVersionDto {
    pub framework_id: String,
    pub framework_version_id: String,
    pub title: String,
    pub version_no: i64,
    pub published_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LearningObjectiveVersionDto {
    pub id: String,
    pub objective_id: String,
    pub version_no: i64,
    pub learning_project_id: String,
    pub framework_id: String,
    pub framework_version_id: String,
    pub parent_version_id: Option<String>,
    pub title: String,
    pub observable_outcome: String,
    pub criteria_text: String,
    pub level: String,
    pub prerequisite_version_ids: Vec<String>,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentQuestionDto {
    pub id: String,
    pub objective_version_id: String,
    pub question_order: i64,
    pub prompt: String,
    pub response_kind: String,
    pub max_points: f64,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentRubricCriterionDto {
    pub id: String,
    pub objective_version_id: String,
    pub criterion_key: String,
    pub description: String,
    pub max_points: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentReviewDto {
    pub id: String,
    pub review_status: AssessmentReviewStatus,
    pub reviewer_id: Option<String>,
    pub rationale: String,
    pub reviewed_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentVersionDto {
    pub id: String,
    pub assessment_id: String,
    pub title: String,
    pub version_no: i64,
    pub learning_project_id: String,
    pub framework_id: String,
    pub framework_version_id: String,
    pub rubric_version_id: String,
    pub rubric_pass_threshold: f64,
    pub status: LearningVersionStatus,
    pub change_rationale: String,
    pub created_by: Option<String>,
    pub created_at_us: i64,
    pub published_at_us: Option<i64>,
    pub questions: Vec<AssessmentQuestionDto>,
    pub rubric_criteria: Vec<AssessmentRubricCriterionDto>,
    pub objective_version_ids: Vec<String>,
    pub reviews: Vec<AssessmentReviewDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AttemptResponseDto {
    pub id: String,
    pub assessment_question_id: String,
    pub response: serde_json::Value,
    pub updated_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationCriterionScoreDto {
    pub rubric_criterion_id: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AttemptEvaluationDto {
    pub id: String,
    pub attempt_id: String,
    pub rubric_version_id: String,
    pub revision_no: i64,
    pub correction_of_id: Option<String>,
    pub evaluator_id: String,
    pub outcome: AssessmentOutcome,
    pub score: f64,
    pub rationale: String,
    pub criterion_scores: Vec<EvaluationCriterionScoreDto>,
    pub feedback: Vec<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentAttemptDto {
    pub id: String,
    pub learning_project_id: String,
    pub learner_id: String,
    pub assessment_version_id: String,
    pub framework_version_id: String,
    pub status: String,
    pub started_at_us: i64,
    pub submitted_at_us: Option<i64>,
    pub responses: Vec<AttemptResponseDto>,
    pub evaluations: Vec<AttemptEvaluationDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompetencyCriterionDto {
    pub id: String,
    pub objective_version_id: String,
    pub criterion: String,
    pub minimum_score: f64,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompetencyDto {
    pub id: String,
    pub learning_project_id: String,
    pub framework_id: String,
    pub framework_version_id: String,
    pub title: String,
    pub description: String,
    pub criteria: Vec<CompetencyCriterionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MasteryRuleVersionDto {
    pub id: String,
    pub mastery_rule_id: String,
    pub competency_id: String,
    pub version_no: i64,
    pub minimum_average_score: f64,
    pub minimum_evaluated_attempts: i64,
    pub all_required_criteria: bool,
    pub reproduction_required: bool,
    pub required_reproduction_outcome: AssessmentOutcome,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MasteryRuleDto {
    pub id: String,
    pub competency_id: String,
    pub title: String,
    pub versions: Vec<MasteryRuleVersionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompetencyRecordCriterionDto {
    pub competency_criterion_id: String,
    pub satisfied: bool,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompetencyRecordDto {
    pub id: String,
    pub competency_id: String,
    pub learning_project_id: String,
    pub learner_id: String,
    pub framework_version_id: String,
    pub mastery_rule_version_id: String,
    pub outcome: MasteryOutcome,
    pub explanation: String,
    pub attempt_evaluation_ids: Vec<String>,
    pub reproduction_evaluation_ids: Vec<String>,
    pub objective_version_ids: Vec<String>,
    pub criteria: Vec<CompetencyRecordCriterionDto>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReproductionTaskDto {
    pub id: String,
    pub learning_project_id: String,
    pub framework_id: String,
    pub framework_version_id: String,
    pub objective_version_id: String,
    pub rubric_version_id: String,
    pub title: String,
    pub task_description: String,
    pub conditions: String,
    pub expected_result: serde_json::Value,
    pub created_by: Option<String>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReproductionSubmissionDto {
    pub id: String,
    pub task_id: String,
    pub learner_id: String,
    pub artifact_uri: Option<String>,
    pub artifact_sha256: Option<String>,
    pub result: Option<serde_json::Value>,
    pub deviations: String,
    pub submitted_at_us: i64,
    pub evaluations: Vec<ReproductionEvaluationDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReproductionEvaluationDto {
    pub id: String,
    pub submission_id: String,
    pub rubric_version_id: String,
    pub evaluator_id: String,
    pub outcome: AssessmentOutcome,
    pub score: f64,
    pub rationale: String,
    pub evidence_description: String,
    pub criterion_scores: Vec<EvaluationCriterionScoreDto>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LearnerProgressDto {
    pub learner_id: String,
    pub learning_project_id: String,
    pub objective_version_id: String,
    pub status: String,
    pub explanation: String,
    pub latest_evaluation_id: Option<String>,
    pub competency_record_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LearningWorkspaceDto {
    pub learners: Vec<LearningLearnerDto>,
    pub projects: Vec<LearningProjectDto>,
    pub framework_versions: Vec<LearningFrameworkVersionDto>,
    pub objectives: Vec<LearningObjectiveVersionDto>,
    pub assessments: Vec<AssessmentVersionDto>,
    pub attempts: Vec<AssessmentAttemptDto>,
    pub competencies: Vec<CompetencyDto>,
    pub mastery_rules: Vec<MasteryRuleDto>,
    pub competency_records: Vec<CompetencyRecordDto>,
    pub reproduction_tasks: Vec<ReproductionTaskDto>,
    pub reproduction_submissions: Vec<ReproductionSubmissionDto>,
    pub progress: Vec<LearnerProgressDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLearnerReq {
    pub display_name: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLearningProjectReq {
    pub title: String,
    pub learner_id: String,
    pub framework_id: String,
    pub framework_version_id: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLearningObjectiveReq {
    pub objective_id: Option<String>,
    pub learning_project_id: String,
    pub title: String,
    pub observable_outcome: String,
    pub criteria_text: String,
    pub level: String,
    pub parent_version_id: Option<String>,
    pub prerequisite_version_ids: Vec<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAssessmentQuestionReq {
    pub objective_version_id: String,
    pub question_order: i64,
    pub prompt: String,
    pub response_kind: String,
    pub max_points: f64,
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRubricCriterionReq {
    pub objective_version_id: String,
    pub criterion_key: String,
    pub description: String,
    pub max_points: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAssessmentVersionReq {
    pub assessment_id: Option<String>,
    pub learning_project_id: String,
    pub title: String,
    pub change_rationale: String,
    pub rubric_title: String,
    pub rubric_pass_threshold: f64,
    pub questions: Vec<CreateAssessmentQuestionReq>,
    pub rubric_criteria: Vec<CreateRubricCriterionReq>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewAssessmentReq {
    pub assessment_version_id: String,
    pub review_status: AssessmentReviewStatus,
    pub reviewer_id: Option<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionAssessmentVersionReq {
    pub assessment_version_id: String,
    pub next_status: LearningVersionStatus,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAssessmentAttemptReq {
    pub learning_project_id: String,
    pub learner_id: String,
    pub assessment_version_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAttemptResponseReq {
    pub attempt_id: String,
    pub assessment_question_id: String,
    pub response: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateAssessmentAttemptReq {
    pub attempt_id: String,
    pub correction_of_id: Option<String>,
    pub evaluator_id: String,
    pub outcome: AssessmentOutcome,
    pub criterion_scores: Vec<EvaluationCriterionScoreDto>,
    pub rationale: String,
    pub feedback: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCompetencyCriterionReq {
    pub objective_version_id: String,
    pub criterion: String,
    pub minimum_score: f64,
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCompetencyReq {
    pub learning_project_id: String,
    pub title: String,
    pub description: String,
    pub criteria: Vec<CreateCompetencyCriterionReq>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMasteryRuleVersionReq {
    pub mastery_rule_id: Option<String>,
    pub competency_id: String,
    pub title: String,
    pub minimum_average_score: f64,
    pub minimum_evaluated_attempts: i64,
    pub all_required_criteria: bool,
    pub reproduction_required: bool,
    pub required_reproduction_outcome: AssessmentOutcome,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateCompetencyReq {
    pub competency_id: String,
    pub learner_id: String,
    pub mastery_rule_version_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReproductionTaskReq {
    pub learning_project_id: String,
    pub objective_version_id: String,
    pub rubric_version_id: String,
    pub title: String,
    pub task_description: String,
    pub conditions: String,
    pub expected_result: serde_json::Value,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitReproductionReq {
    pub task_id: String,
    pub learner_id: String,
    pub artifact_uri: Option<String>,
    pub artifact_sha256: Option<String>,
    pub result: Option<serde_json::Value>,
    pub deviations: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateReproductionReq {
    pub submission_id: String,
    pub evaluator_id: String,
    pub outcome: AssessmentOutcome,
    pub criterion_scores: Vec<EvaluationCriterionScoreDto>,
    pub rationale: String,
    pub evidence_description: String,
}
