use super::*;
use crate::mind::canvas_db::AppState;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn experiment_get_workspace(
    state: State<AppState>,
    project_id: String,
) -> Result<ExperimentWorkspaceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::workspace(&conn, &project_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_create_design_version(
    state: State<AppState>,
    request: CreateExperimentDesignVersionReq,
) -> Result<ExperimentDesignVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_design_version(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_preregister_design(
    state: State<AppState>,
    request: PreregisterExperimentDesignReq,
) -> Result<ExperimentDesignVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::preregister_design(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_create_dataset(
    state: State<AppState>,
    request: CreateDatasetReq,
) -> Result<ExperimentDatasetDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_dataset(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_create_dataset_version(
    state: State<AppState>,
    request: CreateDatasetVersionReq,
) -> Result<ExperimentDatasetVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_dataset_version(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_add_dataset_samples(
    state: State<AppState>,
    request: AddDatasetSamplesReq,
) -> Result<i64, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::add_dataset_samples(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_register_dataset_asset(
    app: AppHandle,
    state: State<AppState>,
    request: RegisterDatasetAssetReq,
) -> Result<ExperimentDatasetAssetDto, String> {
    let assets_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    service::register_dataset_asset(&state.db, &request, &assets_root)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_add_dataset_split_group(
    state: State<AppState>,
    request: AddDatasetSplitGroupReq,
) -> Result<ExperimentDatasetSplitGroupDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::add_dataset_split_group(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_create_dataset_transformation(
    state: State<AppState>,
    request: CreateDatasetTransformationReq,
) -> Result<String, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::create_dataset_transformation(&mut conn, &request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_seal_dataset_version(
    app: AppHandle,
    state: State<AppState>,
    dataset_version_id: String,
) -> Result<ExperimentDatasetVersionDto, String> {
    let assets_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    service::seal_dataset_version(&state.db, &dataset_version_id, &assets_root)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_verify_dataset_version(
    app: AppHandle,
    state: State<AppState>,
    dataset_version_id: String,
) -> Result<ExperimentDatasetVersionDto, String> {
    let assets_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    service::verify_dataset_version(&state.db, &dataset_version_id, &assets_root)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_get_dataset_samples(
    state: State<AppState>,
    dataset_version_id: String,
    query: Option<String>,
    offset: i64,
    limit: i64,
) -> Result<DatasetSamplePageDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::get_dataset_samples(&conn, &dataset_version_id, query.as_deref(), offset, limit)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_create_run(
    app: AppHandle,
    state: State<AppState>,
    request: CreateExperimentRunReq,
) -> Result<ExperimentRunDto, String> {
    let assets_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    service::create_run(&state.db, &request, &assets_root).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_transition_run(
    app: AppHandle,
    state: State<AppState>,
    request: TransitionExperimentRunReq,
) -> Result<ExperimentRunDto, String> {
    let assets_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    service::transition_run(&state.db, &request, &assets_root).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_add_run_deviation(
    state: State<AppState>,
    request: AddExperimentRunDeviationReq,
) -> Result<ExperimentRunDeviationDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::add_run_deviation(&mut conn, &request).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn experiment_add_run_log(
    state: State<AppState>,
    request: AddExperimentRunLogReq,
) -> Result<ExperimentRunLogDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    repository::add_run_log(&mut conn, &request).map_err(|error| error.to_string())
}
