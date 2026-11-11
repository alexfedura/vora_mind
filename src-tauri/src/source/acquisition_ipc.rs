use tauri::State;

use crate::mind::canvas_db::AppState;

use super::acquisition::{
    self, AcquisitionProvenanceTargetKind, AddFrameworkItemReq, CorrectExtractionReq,
    CreateFrameworkReq, CreateFrameworkVersionReq, CreateKnowledgeExtractionReq,
    CreateKnowledgeItemReq, CreateSourceClaimReq, CreateSourceLocationReq, CreateSourceVersionReq,
    FrameworkReviewDto, KnowledgeExtractionDto, KnowledgeItemDto, ProvenanceEventDto,
    PublishFrameworkVersionReq, ReorderFrameworkItemsReq, ReviewExtractionReq, ReviewFrameworkReq,
    SourceClaimDto, SourceLocationDto, SourceVersionDto, UpdateFrameworkVersionReq,
};
#[tauri::command]
pub fn src_list_source_versions(
    source_id: String,
    state: State<AppState>,
) -> Result<Vec<SourceVersionDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::list_source_versions(&conn, &source_id)
}

#[tauri::command]
pub fn src_create_source_version(
    req: CreateSourceVersionReq,
    state: State<AppState>,
) -> Result<SourceVersionDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::create_source_version(&conn, req, Some("user"))
}

#[tauri::command]
pub fn src_list_source_locations(
    source_version_id: String,
    state: State<AppState>,
) -> Result<Vec<SourceLocationDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::list_source_locations(&conn, &source_version_id)
}

#[tauri::command]
pub fn src_create_source_location(
    req: CreateSourceLocationReq,
    state: State<AppState>,
) -> Result<SourceLocationDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::create_source_location(&conn, req)
}

#[tauri::command]
pub fn src_list_knowledge_extractions(
    source_version_id: String,
    state: State<AppState>,
) -> Result<Vec<KnowledgeExtractionDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::list_extractions(&conn, &source_version_id)
}

#[tauri::command]
pub fn src_create_knowledge_extraction(
    req: CreateKnowledgeExtractionReq,
    state: State<AppState>,
) -> Result<KnowledgeExtractionDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::create_extraction(&conn, req)
}

#[tauri::command]
pub fn src_review_knowledge_extraction(
    req: ReviewExtractionReq,
    state: State<AppState>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::review_extraction(&conn, req)
}

#[tauri::command]
pub fn src_correct_knowledge_extraction(
    req: CorrectExtractionReq,
    state: State<AppState>,
) -> Result<KnowledgeExtractionDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::correct_extraction(&conn, req)
}

#[tauri::command]
pub fn src_list_source_claims(
    source_id: String,
    state: State<AppState>,
) -> Result<Vec<SourceClaimDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::list_source_claims(&conn, &source_id)
}

#[tauri::command]
pub fn src_create_source_claim(
    req: CreateSourceClaimReq,
    state: State<AppState>,
) -> Result<SourceClaimDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::create_source_claim(&conn, req)
}

#[tauri::command]
pub fn src_list_knowledge_items(state: State<AppState>) -> Result<Vec<KnowledgeItemDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::list_knowledge_items(&conn)
}

#[tauri::command]
pub fn src_create_knowledge_item(
    req: CreateKnowledgeItemReq,
    state: State<AppState>,
) -> Result<KnowledgeItemDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::create_knowledge_item(&conn, req, Some("user"))
}

#[tauri::command]
pub fn src_list_claim_relations(
    source_claim_id: String,
    state: State<AppState>,
) -> Result<Vec<acquisition::ClaimRelationDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::list_claim_relations(&conn, &source_claim_id)
}

#[tauri::command]
pub fn src_create_claim_relation(
    req: acquisition::CreateClaimRelationReq,
    state: State<AppState>,
) -> Result<String, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::create_claim_relation(&conn, req)
}

#[tauri::command]
pub fn src_list_frameworks(
    state: State<AppState>,
) -> Result<Vec<acquisition::FrameworkDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::list_frameworks(&conn)
}

#[tauri::command]
pub fn src_create_framework(
    req: CreateFrameworkReq,
    state: State<AppState>,
) -> Result<acquisition::FrameworkDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::create_framework(&conn, req)
}

#[tauri::command]
pub fn src_list_framework_versions(
    framework_id: String,
    state: State<AppState>,
) -> Result<Vec<acquisition::FrameworkVersionDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::list_framework_versions(&conn, &framework_id)
}

#[tauri::command]
pub fn src_list_framework_reviews(
    framework_version_id: String,
    state: State<AppState>,
) -> Result<Vec<FrameworkReviewDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::list_framework_reviews(&conn, &framework_version_id)
}

#[tauri::command]
pub fn src_create_framework_version(
    req: CreateFrameworkVersionReq,
    state: State<AppState>,
) -> Result<acquisition::FrameworkVersionDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::create_framework_version(&conn, req)
}

#[tauri::command]
pub fn src_add_framework_item(
    req: AddFrameworkItemReq,
    state: State<AppState>,
) -> Result<String, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::add_framework_item(&conn, req)
}

#[tauri::command]
pub fn src_remove_framework_item(item_id: String, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::remove_framework_item(&conn, &item_id)
}

#[tauri::command]
pub fn src_reorder_framework_items(
    req: ReorderFrameworkItemsReq,
    state: State<AppState>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::reorder_framework_items(&conn, req)
}

#[tauri::command]
pub fn src_update_framework_version(
    req: UpdateFrameworkVersionReq,
    state: State<AppState>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::update_framework_version(&conn, req)
}

#[tauri::command]
pub fn src_review_framework_version(
    req: ReviewFrameworkReq,
    state: State<AppState>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::review_framework_version(&conn, req)
}

#[tauri::command]
pub fn src_publish_framework_version(
    req: PublishFrameworkVersionReq,
    state: State<AppState>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::publish_framework_version(&conn, &req.framework_version_id)
}

#[tauri::command]
pub fn src_list_provenance(
    target_kind: AcquisitionProvenanceTargetKind,
    target_id: String,
    state: State<AppState>,
) -> Result<Vec<ProvenanceEventDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    acquisition::list_provenance(&conn, target_kind, &target_id)
}
