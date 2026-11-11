use super::*;
use crate::mind::canvas_db::AppState;
use tauri::State;

#[tauri::command]
pub fn model_get_workspace(
    state: State<AppState>,
    project_id: String,
) -> Result<ModelWorkspaceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::workspace(&conn, &project_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn model_create(
    state: State<AppState>,
    request: CreateModelReq,
) -> Result<ModelDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_model(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn model_create_version(
    state: State<AppState>,
    request: CreateModelVersionReq,
) -> Result<ModelVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_model_version(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn model_create_evidence(
    state: State<AppState>,
    request: CreateModelEvidenceReq,
) -> Result<ModelEvidenceDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_evidence(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn model_create_evaluation(
    state: State<AppState>,
    request: CreateModelEvaluationReq,
) -> Result<ModelEvaluationDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_evaluation(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn model_create_validation(
    state: State<AppState>,
    request: CreateModelValidationReq,
) -> Result<ModelValidationDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_validation(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn model_finalize_validation(
    state: State<AppState>,
    request: FinalizeModelValidationReq,
) -> Result<ModelValidationDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::finalize_validation(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn model_create_replication(
    state: State<AppState>,
    request: CreateModelReplicationReq,
) -> Result<ModelReplicationDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_replication(&mut conn, &request).map_err(|error| error.to_string())
}
