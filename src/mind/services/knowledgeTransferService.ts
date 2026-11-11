import { invoke } from "@tauri-apps/api/core";
import type {
  AdoptKnowledgeReq,
  ChangeTransferEligibilityReq,
  CreateConceptVersionReq,
  CreatePrincipleVersionReq,
  CreateTransferAdaptationReq,
  CreateTransferEligibilityReq,
  CreateTransferOriginReq,
  DownstreamUsageDto,
  KnowledgeTransferDto,
  KnowledgeTransferOriginDto,
  RecordDownstreamUsageReq,
  ReviewKnowledgeVersionReq,
  TransferAdaptationDto,
  TransferEligibilityDto,
  TransferProvenancePreviewDto,
  TransferWorkspaceDto,
} from "@/types";

export const knowledgeTransferService = {
  getWorkspace: () =>
    invoke<TransferWorkspaceDto>("knowledge_transfer_get_workspace"),
  getProvenancePreview: (originId: string) =>
    invoke<TransferProvenancePreviewDto>("knowledge_transfer_get_provenance_preview", {
      originId,
    }),
  createConceptVersion: (request: CreateConceptVersionReq) =>
    invoke<string>("knowledge_transfer_create_concept_version", { request }),
  createPrincipleVersion: (request: CreatePrincipleVersionReq) =>
    invoke<string>("knowledge_transfer_create_principle_version", { request }),
  reviewKnowledgeVersion: (request: ReviewKnowledgeVersionReq) =>
    invoke<void>("knowledge_transfer_review_knowledge_version", { request }),
  createOrigin: (request: CreateTransferOriginReq) =>
    invoke<KnowledgeTransferOriginDto>("knowledge_transfer_create_origin", { request }),
  createEligibility: (request: CreateTransferEligibilityReq) =>
    invoke<TransferEligibilityDto>("knowledge_transfer_create_eligibility", { request }),
  changeEligibility: (request: ChangeTransferEligibilityReq) =>
    invoke<TransferEligibilityDto>("knowledge_transfer_change_eligibility", { request }),
  adopt: (request: AdoptKnowledgeReq) =>
    invoke<KnowledgeTransferDto>("knowledge_transfer_adopt", { request }),
  createAdaptation: (request: CreateTransferAdaptationReq) =>
    invoke<TransferAdaptationDto>("knowledge_transfer_create_adaptation", { request }),
  recordUsage: (request: RecordDownstreamUsageReq) =>
    invoke<DownstreamUsageDto>("knowledge_transfer_record_usage", { request }),
};
