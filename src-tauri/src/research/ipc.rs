use super::*;
use crate::mind::canvas_db::AppState;
use tauri::State;

#[tauri::command]
pub fn research_get_workspace(state: State<AppState>) -> Result<ResearchWorkspaceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    workspace(&conn)
}

#[tauri::command]
pub fn research_create_project(
    state: State<AppState>,
    request: CreateResearchProjectReq,
) -> Result<ResearchProjectDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_project(&mut conn, request)
}

#[tauri::command]
pub fn research_create_domain(
    state: State<AppState>,
    request: CreateResearchDomainReq,
) -> Result<ResearchDomainDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_domain(&mut conn, request)
}

#[tauri::command]
pub fn research_assign_domain(
    state: State<AppState>,
    project_id: String,
    domain_id: String,
    rationale: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    assign_domain(&conn, &project_id, &domain_id, &rationale)
}

#[tauri::command]
pub fn research_adopt_framework_version(
    state: State<AppState>,
    project_id: String,
    framework_id: String,
    framework_version_id: String,
    use_context: String,
    actor_id: Option<String>,
) -> Result<ProjectKnowledgeAdoptionDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    adopt_framework_version(
        &conn,
        &project_id,
        &framework_id,
        &framework_version_id,
        &use_context,
        actor_id.as_deref(),
    )
}

#[tauri::command]
pub fn research_create_literature_review(
    state: State<AppState>,
    request: CreateLiteratureReviewReq,
) -> Result<LiteratureReviewDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_literature_review(&conn, request)
}

#[tauri::command]
pub fn research_add_literature_source(
    state: State<AppState>,
    request: AddLiteratureSourceReq,
) -> Result<LiteratureReviewSourceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    add_literature_source(&conn, request)
}

#[tauri::command]
pub fn research_review_literature_review(
    state: State<AppState>,
    review_id: String,
    reviewer_id: Option<String>,
) -> Result<LiteratureReviewDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    review_literature_review(&conn, &review_id, reviewer_id.as_deref())
}

#[tauri::command]
pub fn research_create_prior_assessment(
    state: State<AppState>,
    request: CreatePriorKnowledgeAssessmentReq,
) -> Result<PriorKnowledgeAssessmentDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_prior_assessment(&conn, request)
}

#[tauri::command]
pub fn research_review_prior_assessment(
    state: State<AppState>,
    assessment_id: String,
    reviewer_id: Option<String>,
) -> Result<PriorKnowledgeAssessmentDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    review_prior_assessment(&conn, &assessment_id, reviewer_id.as_deref())
}

#[tauri::command]
pub fn research_create_gap(
    state: State<AppState>,
    request: CreateResearchGapReq,
) -> Result<ResearchGapDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_gap(&conn, request)
}

#[tauri::command]
pub fn research_add_gap_evidence(
    state: State<AppState>,
    request: AddGapEvidenceReq,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    add_gap_evidence(&conn, request)
}

#[tauri::command]
pub fn research_review_gap_exception(
    state: State<AppState>,
    request: ReviewGapExceptionReq,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    review_gap_exception(&conn, request)
}

#[tauri::command]
pub fn research_transition_gap(
    state: State<AppState>,
    gap_id: String,
    next_status: ResearchGapStatus,
    actor_id: Option<String>,
) -> Result<ResearchGapDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    transition_gap(&conn, &gap_id, next_status, actor_id.as_deref())
}

#[tauri::command]
pub fn research_create_question(
    state: State<AppState>,
    request: CreateResearchQuestionReq,
) -> Result<ResearchQuestionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_question(&mut conn, request)
}

#[tauri::command]
pub fn research_transition_question(
    state: State<AppState>,
    question_id: String,
    next_status: ResearchQuestionStatus,
    actor_id: Option<String>,
    rationale: String,
) -> Result<ResearchQuestionDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    transition_question(
        &conn,
        &question_id,
        next_status,
        actor_id.as_deref(),
        &rationale,
    )
}

#[tauri::command]
pub fn research_refine_question(
    state: State<AppState>,
    parent_question_id: String,
    title: String,
    description: String,
    rationale: String,
    actor_id: Option<String>,
) -> Result<ResearchQuestionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    refine_question(
        &mut conn,
        &parent_question_id,
        &title,
        &description,
        &rationale,
        actor_id.as_deref(),
    )
}

#[tauri::command]
pub fn research_link_question_lineage(
    state: State<AppState>,
    parent_question_id: String,
    derived_question_id: String,
    relation_kind: ResearchQuestionRelation,
    rationale: String,
    actor_id: Option<String>,
) -> Result<ResearchQuestionLineageDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    link_question_lineage(
        &conn,
        &parent_question_id,
        &derived_question_id,
        relation_kind,
        &rationale,
        actor_id.as_deref(),
    )
}

#[tauri::command]
pub fn research_create_phenomenon(
    state: State<AppState>,
    request: CreatePhenomenonReq,
) -> Result<PhenomenonDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_phenomenon(&conn, request)
}

#[tauri::command]
pub fn research_create_observation_plan(
    state: State<AppState>,
    request: CreateObservationPlanReq,
) -> Result<ObservationPlanDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_observation_plan(&mut conn, request)
}

#[tauri::command]
pub fn research_review_observation_plan(
    state: State<AppState>,
    plan_version_id: String,
    preregister: bool,
    actor_id: Option<String>,
) -> Result<ObservationPlanVersionDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    review_observation_plan(&conn, &plan_version_id, preregister, actor_id.as_deref())
}

#[tauri::command]
pub fn research_create_dataset_version_reference(
    state: State<AppState>,
    request: CreateDatasetVersionReferenceReq,
) -> Result<DatasetVersionReferenceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_dataset_version_reference(&conn, request)
}

#[tauri::command]
pub fn research_create_observation(
    state: State<AppState>,
    request: CreateObservationReq,
) -> Result<ObservationDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_observation(&conn, request)
}

#[tauri::command]
pub fn research_create_pattern(
    state: State<AppState>,
    request: CreatePatternReq,
) -> Result<PatternDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_pattern(&mut conn, request)
}

#[tauri::command]
pub fn research_create_idea(
    state: State<AppState>,
    request: CreateIdeaReq,
) -> Result<IdeaDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_idea(&conn, request)
}

#[tauri::command]
pub fn research_create_typed_hypothesis(
    state: State<AppState>,
    request: CreateTypedHypothesisReq,
) -> Result<ResearchHypothesisDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_typed_hypothesis(&mut conn, request)
}

#[tauri::command]
pub fn research_transition_hypothesis(
    state: State<AppState>,
    hypothesis_id: String,
    next_status: ResearchHypothesisStatus,
    actor_id: Option<String>,
) -> Result<ResearchHypothesisDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    transition_hypothesis(&conn, &hypothesis_id, next_status, actor_id.as_deref())
}

#[tauri::command]
pub fn research_link_competing_hypotheses(
    state: State<AppState>,
    request: LinkCompetingHypothesesReq,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    link_competing_hypotheses(&conn, request)
}

#[tauri::command]
pub fn research_add_hypothesis_model_reference(
    state: State<AppState>,
    request: AddHypothesisModelReferenceReq,
) -> Result<HypothesisModelReferenceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    add_hypothesis_model_reference(&conn, request)
}

#[tauri::command]
pub fn research_add_hypothesis_evidence(
    state: State<AppState>,
    request: AddHypothesisEvidenceReq,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    add_hypothesis_evidence(&conn, request)
}

#[tauri::command]
pub fn research_create_design(
    state: State<AppState>,
    request: CreateResearchDesignReq,
) -> Result<ResearchExperimentDesignVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_research_design(&mut conn, request)
}

#[tauri::command]
pub fn research_transition_design(
    state: State<AppState>,
    design_version_id: String,
    next_status: ResearchDesignStatus,
    actor_id: Option<String>,
) -> Result<ResearchExperimentDesignVersionDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    transition_research_design(&conn, &design_version_id, &next_status, actor_id.as_deref())
}

#[tauri::command]
pub fn research_link_hypothesis_design(
    state: State<AppState>,
    request: LinkHypothesisDesignReq,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    link_hypothesis_design(&conn, request)
}
