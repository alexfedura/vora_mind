import { invoke } from "@tauri-apps/api/core";
import type {
  AnalysisWorkspaceDto,
  CreateFindingVersionReq,
  CreateFindingRelationReq,
  CreateEvidenceReq,
  EvidenceDto,
  FindingRelationDto,
  TransitionEvidenceReq,
  TransitionFindingReq,
  FindingVersionDto,
} from "@/types";

export const analysisService = {
  getWorkspace: (projectId: string) =>
    invoke<AnalysisWorkspaceDto>("analysis_get_workspace", { projectId }),
  createEvidence: (request: CreateEvidenceReq) =>
    invoke<EvidenceDto>("analysis_create_evidence", { request }),
  transitionEvidence: (request: TransitionEvidenceReq) =>
    invoke<EvidenceDto>("analysis_transition_evidence", { request }),
  createFindingVersion: (request: CreateFindingVersionReq) =>
    invoke<FindingVersionDto>("analysis_create_finding_version", { request }),
  createFindingRelation: (request: CreateFindingRelationReq) =>
    invoke<FindingRelationDto>("analysis_create_finding_relation", { request }),
  transitionFinding: (request: TransitionFindingReq) =>
    invoke<FindingVersionDto>("analysis_transition_finding", { request }),
};
