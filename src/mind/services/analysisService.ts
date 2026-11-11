import { invoke } from "@tauri-apps/api/core";
import type {
  AnalysisWorkspaceDto,
  CreateEvidenceReq,
  EvidenceDto,
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
  transitionFinding: (request: TransitionFindingReq) =>
    invoke<FindingVersionDto>("analysis_transition_finding", { request }),
};
