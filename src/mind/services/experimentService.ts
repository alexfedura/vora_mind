import { invoke } from "@tauri-apps/api/core";
import type {
  AddDatasetSamplesReq,
  AddDatasetSplitGroupReq,
  AddExperimentRunDeviationReq,
  AddExperimentRunLogReq,
  AddDatasetSamplesReq,
  CreateDatasetReq,
  CreateDatasetTransformationReq,
  CreateDatasetVersionReq,
  CreateExperimentDesignVersionReq,
  CreateExperimentRunReq,
  DatasetSamplePageDto,
  ExperimentDatasetAssetDto,
  ExperimentDatasetDto,
  ExperimentDatasetTransformationDto,
  ExperimentDatasetVersionDto,
  ExperimentDesignVersionDto,
  ExperimentRunDto,
  ExperimentRunDeviationDto,
  ExperimentRunLogDto,
  ExperimentWorkspaceDto,
  PreregisterExperimentDesignReq,
  RegisterDatasetAssetReq,
  TransitionExperimentRunReq,
} from "@/types";

export const experimentService = {
  getWorkspace: (projectId: string) =>
    invoke<ExperimentWorkspaceDto>("experiment_get_workspace", { projectId }),
  createDesignVersion: (request: CreateExperimentDesignVersionReq) =>
    invoke<ExperimentDesignVersionDto>("experiment_create_design_version", { request }),
  preregisterDesign: (request: PreregisterExperimentDesignReq) =>
    invoke<ExperimentDesignVersionDto>("experiment_preregister_design", { request }),
  createRun: (request: CreateExperimentRunReq) =>
    invoke<ExperimentRunDto>("experiment_create_run", { request }),
  transitionRun: (request: TransitionExperimentRunReq) =>
    invoke<ExperimentRunDto>("experiment_transition_run", { request }),
  addRunDeviation: (request: AddExperimentRunDeviationReq) =>
    invoke<ExperimentRunDeviationDto>("experiment_add_run_deviation", { request }),
  addRunLog: (request: AddExperimentRunLogReq) =>
    invoke<ExperimentRunLogDto>("experiment_add_run_log", { request }),
  createDataset: (request: CreateDatasetReq) =>
    invoke<ExperimentDatasetDto>("experiment_create_dataset", { request }),
  createDatasetVersion: (request: CreateDatasetVersionReq) =>
    invoke<ExperimentDatasetVersionDto>("experiment_create_dataset_version", { request }),
  addDatasetSamples: (request: AddDatasetSamplesReq) =>
    invoke<number>("experiment_add_dataset_samples", { request }),
  registerDatasetAsset: (request: RegisterDatasetAssetReq) =>
    invoke<ExperimentDatasetAssetDto>("experiment_register_dataset_asset", { request }),
  addDatasetSplitGroup: (request: AddDatasetSplitGroupReq) =>
    invoke<import("@/types").ExperimentDatasetSplitGroupDto>(
      "experiment_add_dataset_split_group",
      { request },
    ),
  createDatasetTransformation: (request: CreateDatasetTransformationReq) =>
    invoke<string>("experiment_create_dataset_transformation", { request }),
  sealDatasetVersion: (datasetVersionId: string) =>
    invoke<ExperimentDatasetVersionDto>("experiment_seal_dataset_version", {
      datasetVersionId,
    }),
  verifyDatasetVersion: (datasetVersionId: string) =>
    invoke<ExperimentDatasetVersionDto>("experiment_verify_dataset_version", {
      datasetVersionId,
    }),
  getDatasetSamples: (
    datasetVersionId: string,
    query: string | null,
    offset: number,
    limit: number,
  ) =>
    invoke<DatasetSamplePageDto>("experiment_get_dataset_samples", {
      datasetVersionId,
      query,
      offset,
      limit,
    }),
};
