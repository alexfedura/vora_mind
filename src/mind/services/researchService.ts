import { invoke } from "@tauri-apps/api/core";
import type {
  AddGapEvidenceReq,
  AddHypothesisEvidenceReq,
  AddHypothesisModelReferenceReq,
  AddLiteratureSourceReq,
  CreateDatasetVersionReferenceReq,
  CreateIdeaReq,
  CreateLiteratureReviewReq,
  CreateObservationPlanReq,
  CreateObservationReq,
  CreatePatternReq,
  CreatePhenomenonReq,
  CreatePriorKnowledgeAssessmentReq,
  CreateResearchDesignReq,
  CreateLinkedResearchDesignReq,
  CreateResearchDomainReq,
  CreateResearchGapReq,
  CreateResearchProjectReq,
  CreateResearchQuestionReq,
  CreateReviewedLiteratureReviewReq,
  CreateTypedHypothesisReq,
  DatasetVersionReferenceDto,
  HypothesisModelReferenceDto,
  IdeaDto,
  LinkCompetingHypothesesReq,
  LinkHypothesisDesignReq,
  LiteratureReviewDto,
  LiteratureReviewSourceDto,
  ObservationDto,
  ObservationPlanDto,
  ObservationPlanVersionDto,
  PatternDto,
  PhenomenonDto,
  PriorKnowledgeAssessmentDto,
  ProjectKnowledgeAdoptionDto,
  ResearchDomainDto,
  ResearchDesignStatus,
  ResearchExperimentDesignVersionDto,
  ResearchGapDto,
  ResearchGapStatus,
  ResearchHypothesisDto,
  ResearchHypothesisStatus,
  ResearchProjectDto,
  ResearchQuestionDto,
  ResearchQuestionRelation,
  ResearchQuestionStatus,
  ResearchWorkspaceDto,
  ReviewGapExceptionReq,
} from "@/types";

export const researchService = {
  getWorkspace: () => invoke<ResearchWorkspaceDto>("research_get_workspace"),
  createProject: (request: CreateResearchProjectReq) =>
    invoke<ResearchProjectDto>("research_create_project", { request }),
  createDomain: (request: CreateResearchDomainReq) =>
    invoke<ResearchDomainDto>("research_create_domain", { request }),
  assignDomain: (projectId: string, domainId: string, rationale: string) =>
    invoke<void>("research_assign_domain", { projectId, domainId, rationale }),
  adoptFrameworkVersion: (
    projectId: string,
    frameworkId: string,
    frameworkVersionId: string,
    useContext: string,
    actorId: string | null,
  ) => invoke<ProjectKnowledgeAdoptionDto>("research_adopt_framework_version", {
    projectId, frameworkId, frameworkVersionId, useContext, actorId,
  }),
  createLiteratureReview: (request: CreateLiteratureReviewReq) =>
    invoke<LiteratureReviewDto>("research_create_literature_review", { request }),
  createReviewedLiteratureReview: (request: CreateReviewedLiteratureReviewReq) =>
    invoke<LiteratureReviewDto>("research_create_reviewed_literature_review", { request }),
  addLiteratureSource: (request: AddLiteratureSourceReq) =>
    invoke<LiteratureReviewSourceDto>("research_add_literature_source", { request }),
  reviewLiteratureReview: (reviewId: string, reviewerId: string | null) =>
    invoke<LiteratureReviewDto>("research_review_literature_review", { reviewId, reviewerId }),
  createPriorAssessment: (request: CreatePriorKnowledgeAssessmentReq) =>
    invoke<PriorKnowledgeAssessmentDto>("research_create_prior_assessment", { request }),
  reviewPriorAssessment: (assessmentId: string, reviewerId: string | null) =>
    invoke<PriorKnowledgeAssessmentDto>("research_review_prior_assessment", { assessmentId, reviewerId }),
  createGap: (request: CreateResearchGapReq) =>
    invoke<ResearchGapDto>("research_create_gap", { request }),
  addGapEvidence: (request: AddGapEvidenceReq) =>
    invoke<void>("research_add_gap_evidence", { request }),
  reviewGapException: (request: ReviewGapExceptionReq) =>
    invoke<void>("research_review_gap_exception", { request }),
  transitionGap: (gapId: string, nextStatus: ResearchGapStatus, actorId: string | null) =>
    invoke<ResearchGapDto>("research_transition_gap", { gapId, nextStatus, actorId }),
  createQuestion: (request: CreateResearchQuestionReq) =>
    invoke<ResearchQuestionDto>("research_create_question", { request }),
  transitionQuestion: (
    questionId: string,
    nextStatus: ResearchQuestionStatus,
    actorId: string | null,
    rationale: string,
  ) => invoke<ResearchQuestionDto>("research_transition_question", {
    questionId, nextStatus, actorId, rationale,
  }),
  refineQuestion: (
    parentQuestionId: string,
    title: string,
    description: string,
    rationale: string,
    actorId: string | null,
  ) => invoke<ResearchQuestionDto>("research_refine_question", {
    parentQuestionId, title, description, rationale, actorId,
  }),
  linkQuestionLineage: (
    parentQuestionId: string,
    derivedQuestionId: string,
    relationKind: ResearchQuestionRelation,
    rationale: string,
    actorId: string | null,
  ) => invoke("research_link_question_lineage", {
    parentQuestionId, derivedQuestionId, relationKind, rationale, actorId,
  }),
  createPhenomenon: (request: CreatePhenomenonReq) =>
    invoke<PhenomenonDto>("research_create_phenomenon", { request }),
  createObservationPlan: (request: CreateObservationPlanReq) =>
    invoke<ObservationPlanDto>("research_create_observation_plan", { request }),
  reviewObservationPlan: (planVersionId: string, preregister: boolean, actorId: string | null) =>
    invoke<ObservationPlanVersionDto>("research_review_observation_plan", {
      planVersionId, preregister, actorId,
    }),
  createDatasetVersionReference: (request: CreateDatasetVersionReferenceReq) =>
    invoke<DatasetVersionReferenceDto>("research_create_dataset_version_reference", { request }),
  createObservation: (request: CreateObservationReq) =>
    invoke<ObservationDto>("research_create_observation", { request }),
  createPattern: (request: CreatePatternReq) =>
    invoke<PatternDto>("research_create_pattern", { request }),
  createIdea: (request: CreateIdeaReq) =>
    invoke<IdeaDto>("research_create_idea", { request }),
  createTypedHypothesis: (request: CreateTypedHypothesisReq) =>
    invoke<ResearchHypothesisDto>("research_create_typed_hypothesis", { request }),
  transitionHypothesis: (
    hypothesisId: string,
    nextStatus: ResearchHypothesisStatus,
    actorId: string | null,
  ) => invoke<ResearchHypothesisDto>("research_transition_hypothesis", {
    hypothesisId, nextStatus, actorId,
  }),
  linkCompetingHypotheses: (request: LinkCompetingHypothesesReq) =>
    invoke<void>("research_link_competing_hypotheses", { request }),
  addHypothesisModelReference: (request: AddHypothesisModelReferenceReq) =>
    invoke<HypothesisModelReferenceDto>("research_add_hypothesis_model_reference", { request }),
  addHypothesisEvidence: (request: AddHypothesisEvidenceReq) =>
    invoke<void>("research_add_hypothesis_evidence", { request }),
  createDesign: (request: CreateResearchDesignReq) =>
    invoke<ResearchExperimentDesignVersionDto>("research_create_design", { request }),
  createLinkedDesign: (request: CreateLinkedResearchDesignReq) =>
    invoke<ResearchExperimentDesignVersionDto>("research_create_linked_design", { request }),
  transitionDesign: (
    designVersionId: string,
    nextStatus: ResearchDesignStatus,
    actorId: string | null,
  ) => invoke<ResearchExperimentDesignVersionDto>("research_transition_design", {
    designVersionId, nextStatus, actorId,
  }),
  linkHypothesisDesign: (request: LinkHypothesisDesignReq) =>
    invoke<void>("research_link_hypothesis_design", { request }),
};
