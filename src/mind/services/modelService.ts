import { invoke } from "@tauri-apps/api/core";
import type {
  CreateModelEvidenceReq,
  CreateModelEvaluationReq,
  CreateModelReplicationReq,
  CreateModelReq,
  CreateModelValidationReq,
  CreateModelVersionReq,
  FinalizeModelValidationReq,
  ModelDto,
  ModelEvidenceDto,
  ModelEvaluationDto,
  ModelReplicationDto,
  ModelValidationDto,
  ModelVersionDto,
  ModelWorkspaceDto,
  TransitionModelReq,
  TransitionModelVersionReq,
} from "@/types";

export const modelService = {
  getWorkspace: (projectId: string) =>
    invoke<ModelWorkspaceDto>("model_get_workspace", { projectId }),
  create: (request: CreateModelReq) =>
    invoke<ModelDto>("model_create", { request }),
  transition: (request: TransitionModelReq) =>
    invoke<ModelDto>("model_transition", { request }),
  createVersion: (request: CreateModelVersionReq) =>
    invoke<ModelVersionDto>("model_create_version", { request }),
  transitionVersion: (request: TransitionModelVersionReq) =>
    invoke<ModelVersionDto>("model_transition_version", { request }),
  createEvidence: (request: CreateModelEvidenceReq) =>
    invoke<ModelEvidenceDto>("model_create_evidence", { request }),
  createEvaluation: (request: CreateModelEvaluationReq) =>
    invoke<ModelEvaluationDto>("model_create_evaluation", { request }),
  createValidation: (request: CreateModelValidationReq) =>
    invoke<ModelValidationDto>("model_create_validation", { request }),
  finalizeValidation: (request: FinalizeModelValidationReq) =>
    invoke<ModelValidationDto>("model_finalize_validation", { request }),
  createReplication: (request: CreateModelReplicationReq) =>
    invoke<ModelReplicationDto>("model_create_replication", { request }),
};
