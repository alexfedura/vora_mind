use super::*;
use crate::mind::canvas_db::AppState;
use std::collections::HashSet;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn analysis_get_workspace(
    state: State<AppState>,
    project_id: String,
) -> Result<AnalysisWorkspaceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::workspace(&conn, &project_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_create_method_version(
    state: State<AppState>,
    request: CreateAnalysisMethodVersionReq,
) -> Result<AnalysisMethodVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_method_version(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_create_metric_version(
    state: State<AppState>,
    request: CreateMetricDefinitionVersionReq,
) -> Result<MetricDefinitionVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_metric_definition_version(&mut conn, &request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_create_definition_version(
    state: State<AppState>,
    request: CreateAnalysisDefinitionVersionReq,
) -> Result<AnalysisDefinitionVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_definition_version(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_transition_definition(
    state: State<AppState>,
    request: TransitionAnalysisDefinitionReq,
) -> Result<AnalysisDefinitionVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::transition_definition(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_create_run(
    app: AppHandle,
    state: State<AppState>,
    request: CreateAnalysisRunReq,
) -> Result<AnalysisRunDto, String> {
    let assets_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let dataset_versions = request
        .inputs
        .iter()
        .map(|input| input.dataset_version_id.as_str())
        .collect::<HashSet<_>>();
    for dataset_version_id in dataset_versions {
        crate::experiment::service::verify_dataset_version(
            &state.db,
            dataset_version_id,
            &assets_root,
        )
        .map_err(|error| error.to_string())?;
    }
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_run(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_start_run(
    state: State<AppState>,
    run_id: String,
    actor_id: Option<String>,
) -> Result<AnalysisRunDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::start_run(&mut conn, &run_id, actor_id.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_add_run_log(
    state: State<AppState>,
    request: AddAnalysisRunLogReq,
) -> Result<AnalysisRunLogDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::add_run_log(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_add_run_deviation(
    state: State<AppState>,
    run_id: String,
    request: AnalysisDeviationReq,
) -> Result<AnalysisDeviationDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::add_run_deviation(&mut conn, &run_id, &request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_finalize_run(
    state: State<AppState>,
    request: FinalizeAnalysisRunReq,
) -> Result<AnalysisRunDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::finalize_run(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_get_run(
    state: State<AppState>,
    run_id: String,
) -> Result<AnalysisRunDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::query_run(&conn, &run_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_create_evidence(
    state: State<AppState>,
    request: CreateEvidenceReq,
) -> Result<EvidenceDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_evidence(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_transition_evidence(
    state: State<AppState>,
    request: TransitionEvidenceReq,
) -> Result<EvidenceDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::transition_evidence(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_create_finding_version(
    state: State<AppState>,
    request: CreateFindingVersionReq,
) -> Result<FindingVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_finding_version(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_transition_finding(
    state: State<AppState>,
    request: TransitionFindingReq,
) -> Result<FindingVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::transition_finding(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn analysis_create_finding_relation(
    state: State<AppState>,
    request: CreateFindingRelationReq,
) -> Result<FindingRelationDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_finding_relation(&mut conn, &request).map_err(|error| error.to_string())
}
