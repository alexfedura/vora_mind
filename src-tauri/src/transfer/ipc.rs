use super::*;
use crate::mind::canvas_db::AppState;
use tauri::State;

#[tauri::command]
pub fn transfer_get_workspace(state: State<AppState>) -> Result<TransferWorkspaceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    workspace(&conn)
}

#[tauri::command]
pub fn transfer_create_artifact_version(
    state: State<AppState>,
    request: CreateArtifactVersionReq,
) -> Result<TransferArtifactDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_artifact_version(&mut conn, request)
}

#[tauri::command]
pub fn transfer_record_origin(
    state: State<AppState>,
    request: RecordTransferOriginReq,
) -> Result<String, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    record_origin(&mut conn, request)
}

#[tauri::command]
pub fn transfer_create_eligibility(
    state: State<AppState>,
    request: CreateTransferEligibilityReq,
) -> Result<TransferEligibilityDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_eligibility(&mut conn, request)
}

#[tauri::command]
pub fn transfer_change_eligibility(
    state: State<AppState>,
    request: ChangeTransferEligibilityReq,
) -> Result<TransferEligibilityDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    change_eligibility(&conn, request)
}

#[tauri::command]
pub fn transfer_create_transfer(
    state: State<AppState>,
    request: CreateKnowledgeTransferReq,
) -> Result<KnowledgeTransferDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_transfer(&mut conn, request)
}

#[tauri::command]
pub fn transfer_transition_transfer(
    state: State<AppState>,
    request: TransitionKnowledgeTransferReq,
) -> Result<KnowledgeTransferDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    transition_transfer(&conn, request)
}

#[tauri::command]
pub fn transfer_create_adaptation(
    state: State<AppState>,
    request: CreateTransferAdaptationReq,
) -> Result<TransferAdaptationDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_adaptation(&conn, request)
}

#[tauri::command]
pub fn transfer_create_downstream_usage(
    state: State<AppState>,
    request: CreateTransferUsageReq,
) -> Result<TransferDownstreamUsageDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_downstream_usage(&conn, request)
}
