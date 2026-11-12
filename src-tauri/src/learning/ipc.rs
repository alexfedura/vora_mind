use super::*;
use crate::mind::canvas_db::AppState;
use tauri::State;

#[tauri::command]
pub fn learning_get_workspace(state: State<AppState>) -> Result<LearningWorkspaceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::workspace(&conn).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_create_learner(
    state: State<AppState>,
    request: CreateLearnerReq,
) -> Result<LearningLearnerDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_learner(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_create_project(
    state: State<AppState>,
    request: CreateLearningProjectReq,
) -> Result<LearningProjectDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_project(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_create_objective(
    state: State<AppState>,
    request: CreateLearningObjectiveReq,
) -> Result<LearningObjectiveVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_objective_version(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_create_assessment_version(
    state: State<AppState>,
    request: CreateAssessmentVersionReq,
) -> Result<AssessmentVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_assessment_version(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_review_assessment(
    state: State<AppState>,
    request: ReviewAssessmentReq,
) -> Result<AssessmentVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::review_assessment(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_transition_assessment_version(
    state: State<AppState>,
    request: TransitionAssessmentVersionReq,
) -> Result<AssessmentVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::transition_assessment_version(&mut conn, &request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_create_attempt(
    state: State<AppState>,
    request: CreateAssessmentAttemptReq,
) -> Result<AssessmentAttemptDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_attempt(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_save_attempt_response(
    state: State<AppState>,
    request: SaveAttemptResponseReq,
) -> Result<AttemptResponseDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::save_attempt_response(&conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_submit_attempt(
    state: State<AppState>,
    attempt_id: String,
) -> Result<AssessmentAttemptDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::submit_attempt(&mut conn, &attempt_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_evaluate_attempt(
    state: State<AppState>,
    request: EvaluateAssessmentAttemptReq,
) -> Result<AttemptEvaluationDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::evaluate_attempt(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_create_competency(
    state: State<AppState>,
    request: CreateCompetencyReq,
) -> Result<CompetencyDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_competency(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_create_mastery_rule(
    state: State<AppState>,
    request: CreateMasteryRuleVersionReq,
) -> Result<MasteryRuleDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_mastery_rule_version(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_evaluate_competency(
    state: State<AppState>,
    request: EvaluateCompetencyReq,
) -> Result<CompetencyRecordDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::evaluate_competency(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_create_reproduction_task(
    state: State<AppState>,
    request: CreateReproductionTaskReq,
) -> Result<ReproductionTaskDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_reproduction_task(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_submit_reproduction(
    state: State<AppState>,
    request: SubmitReproductionReq,
) -> Result<ReproductionSubmissionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::submit_reproduction(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn learning_evaluate_reproduction(
    state: State<AppState>,
    request: EvaluateReproductionReq,
) -> Result<ReproductionEvaluationDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::evaluate_reproduction(&mut conn, &request).map_err(|error| error.to_string())
}
