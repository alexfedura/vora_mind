use super::{
    repository::{self, DatasetSealMaterial, ExperimentError, PreparedDatasetAsset, Result},
    ExperimentDatasetAssetDto, ExperimentDatasetVersionDto, RegisterDatasetAssetReq,
};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

fn material_checksum(material: &DatasetSealMaterial, assets_root: &Path) -> Result<String> {
    if material.lifecycle_status == "sealed"
        && material.content_sha256.as_deref().is_none()
    {
        return Err(ExperimentError::message(
            "Sealed dataset version is missing its manifest checksum.",
        ));
    }
    repository::verify_dataset_assets(assets_root, &material.assets)?;
    repository::compute_dataset_checksum(material)
}

fn verify_sealed_material(material: &DatasetSealMaterial, assets_root: &Path) -> Result<()> {
    if material.lifecycle_status != "sealed" {
        return Err(ExperimentError::message(
            "Experiment runs can only pin sealed dataset versions.",
        ));
    }
    let actual_checksum = material_checksum(material, assets_root)?;
    if material.content_sha256.as_deref() != Some(actual_checksum.as_str()) {
        return Err(ExperimentError::message(
            "Dataset version manifest checksum verification failed.",
        ));
    }
    Ok(())
}

fn verify_dataset_ids(
    db: &Mutex<Connection>,
    dataset_version_ids: &[String],
    assets_root: &Path,
) -> Result<()> {
    let materials = {
        let conn = db.lock().map_err(|error| {
            ExperimentError::message(format!("Workspace database lock failed: {error}"))
        })?;
        dataset_version_ids
            .iter()
            .map(|id| repository::dataset_seal_material(&conn, id))
            .collect::<Result<Vec<_>>>()?
    };
    for material in &materials {
        verify_sealed_material(material, assets_root)?;
    }
    Ok(())
}

pub fn register_dataset_asset(
    db: &Mutex<Connection>,
    request: &RegisterDatasetAssetReq,
    assets_root: &Path,
) -> Result<ExperimentDatasetAssetDto> {
    let prepared = repository::prepare_dataset_asset(request, assets_root)?;
    let attach_result = db
        .lock()
        .map_err(|error| ExperimentError::from(std::io::Error::other(error.to_string())))
        .and_then(|conn| repository::attach_prepared_dataset_asset(&conn, &prepared));
    if attach_result.is_err() {
        discard_prepared_asset(prepared);
    }
    attach_result
}

fn discard_prepared_asset(prepared: PreparedDatasetAsset) {
    prepared.discard();
}

pub fn seal_dataset_version(
    db: &Mutex<Connection>,
    dataset_version_id: &str,
    assets_root: &Path,
) -> Result<ExperimentDatasetVersionDto> {
    let material = {
        let conn = db
            .lock()
            .map_err(|error| ExperimentError::from(std::io::Error::other(error.to_string())))?;
        repository::dataset_seal_material(&conn, dataset_version_id)?
    };
    if material.lifecycle_status != "staged" {
        return Err(ExperimentError::from(std::io::Error::other(
            "Only staged dataset versions can be sealed.",
        )));
    }
    let checksum = material_checksum(&material, assets_root)?;
    let mut conn = db
        .lock()
        .map_err(|error| ExperimentError::from(std::io::Error::other(error.to_string())))?;
    repository::seal_dataset_version(
        &mut conn,
        dataset_version_id,
        material.staged_revision,
        &checksum,
    )
}

pub fn verify_dataset_version(
    db: &Mutex<Connection>,
    dataset_version_id: &str,
    assets_root: &Path,
) -> Result<ExperimentDatasetVersionDto> {
    let (material, version) = {
        let conn = db
            .lock()
            .map_err(|error| ExperimentError::from(std::io::Error::other(error.to_string())))?;
        (
            repository::dataset_seal_material(&conn, dataset_version_id)?,
            repository::get_dataset_version(&conn, dataset_version_id)?,
        )
    };
    if material.lifecycle_status == "sealed" {
        verify_sealed_material(&material, assets_root)?;
    } else {
        material_checksum(&material, assets_root)?;
    }
    Ok(version)
}

pub fn create_run(
    db: &Mutex<Connection>,
    request: &super::CreateExperimentRunReq,
    assets_root: &Path,
) -> Result<super::ExperimentRunDto> {
    let dataset_versions = request
        .dataset_pins
        .iter()
        .map(|pin| pin.dataset_version_id.clone())
        .collect::<Vec<_>>();
    verify_dataset_ids(db, &dataset_versions, assets_root)?;
    let mut conn = db.lock().map_err(|error| {
        ExperimentError::message(format!("Workspace database lock failed: {error}"))
    })?;
    repository::create_run(&mut conn, request)
}

pub fn transition_run(
    db: &Mutex<Connection>,
    request: &super::TransitionExperimentRunReq,
    assets_root: &Path,
) -> Result<super::ExperimentRunDto> {
    let mut dataset_versions = {
        let conn = db.lock().map_err(|error| {
            ExperimentError::message(format!("Workspace database lock failed: {error}"))
        })?;
        repository::run_dataset_version_ids(&conn, &request.run_id)?
    };
    dataset_versions.extend(
        request
            .output_dataset_pins
            .iter()
            .map(|pin| pin.dataset_version_id.clone()),
    );
    if matches!(
        request.next_status,
        super::ExperimentRunStatus::Ready | super::ExperimentRunStatus::Running
    ) || !request.output_dataset_pins.is_empty()
    {
        verify_dataset_ids(db, &dataset_versions, assets_root)?;
    }
    let mut conn = db.lock().map_err(|error| {
        ExperimentError::message(format!("Workspace database lock failed: {error}"))
    })?;
    repository::transition_run(&mut conn, request)
}
