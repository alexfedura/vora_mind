use super::*;
use crate::mind::canvas_db::AppState;
use tauri::State;

#[tauri::command]
pub fn knowledge_transfer_get_workspace(
    state: State<AppState>,
) -> Result<TransferWorkspaceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::workspace(&conn).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn knowledge_transfer_get_provenance_preview(
    state: State<AppState>,
    origin_id: String,
) -> Result<TransferProvenancePreviewDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::provenance_preview(&conn, &origin_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn knowledge_transfer_create_concept_version(
    state: State<AppState>,
    request: CreateConceptVersionReq,
) -> Result<String, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_concept_version(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn knowledge_transfer_create_principle_version(
    state: State<AppState>,
    request: CreatePrincipleVersionReq,
) -> Result<String, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_principle_version(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn knowledge_transfer_review_knowledge_version(
    state: State<AppState>,
    request: ReviewKnowledgeVersionReq,
) -> Result<(), String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::review_knowledge_version(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn knowledge_transfer_create_origin(
    state: State<AppState>,
    request: CreateTransferOriginReq,
) -> Result<KnowledgeTransferOriginDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_origin(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn knowledge_transfer_create_eligibility(
    state: State<AppState>,
    request: CreateTransferEligibilityReq,
) -> Result<TransferEligibilityDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_eligibility(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn knowledge_transfer_change_eligibility(
    state: State<AppState>,
    request: ChangeTransferEligibilityReq,
) -> Result<TransferEligibilityDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::change_eligibility(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn knowledge_transfer_adopt(
    state: State<AppState>,
    request: AdoptKnowledgeReq,
) -> Result<KnowledgeTransferDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::adopt_knowledge(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn knowledge_transfer_create_adaptation(
    state: State<AppState>,
    request: CreateTransferAdaptationReq,
) -> Result<TransferAdaptationDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_adaptation(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn knowledge_transfer_record_usage(
    state: State<AppState>,
    request: RecordDownstreamUsageReq,
) -> Result<DownstreamUsageDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::record_downstream_usage(&mut conn, &request).map_err(|error| error.to_string())
}
