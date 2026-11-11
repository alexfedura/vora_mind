use super::{
    repository::{self, DatasetSealMaterial, ExperimentError, PreparedDatasetAsset, Result},
    ExperimentDatasetAssetDto, ExperimentDatasetVersionDto, RegisterDatasetAssetReq,
};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

fn material_checksum(material: &DatasetSealMaterial, assets_root: &Path) -> Result<String> {
    repository::verify_dataset_assets(assets_root, &material.assets)?;
    repository::compute_dataset_checksum(material)
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
    let actual_checksum = material_checksum(&material, assets_root)?;
    if material.lifecycle_status == "sealed"
        && material.content_sha256.as_deref() != Some(actual_checksum.as_str())
    {
        return Err(ExperimentError::from(std::io::Error::other(
            "Dataset version manifest checksum verification failed.",
        )));
    }
    if version.lifecycle_status == "sealed" && version.content_sha256.as_deref().is_none() {
        return Err(ExperimentError::from(std::io::Error::other(
            "Sealed dataset version is missing its manifest checksum.",
        )));
    }
    Ok(version)
}

