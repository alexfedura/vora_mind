export type TaskFlowType = "task" | "ai";

export interface TaskFlowItem {
  id: string;
  title: string;
  description: string;
  taskType: TaskFlowType;
  completed: boolean;
  position: number;
  createdAt: number;
  updatedAt: number;
}

export type TraceNodeKind = "state" | "decision" | "experiment" | "rejected";
export type TraceNodeStatus = "accepted" | "rejected" | "pending" | "current" | "abandoned";
export type TraceEdgeKind = "main" | "branch" | "rejected";

export interface TraceAlternative {
  title: string;
  status: TraceNodeStatus;
  reason?: string | null;
}

export interface TraceNodeData {
  id: string;
  kind: TraceNodeKind;
  title: string;
  status: TraceNodeStatus;
  whyAmIHere?: string | null;
  objective?: string | null;
  reason?: string | null;
  createdAt?: string | null;
  parentId?: string | null;
  gitCommit?: string | null;
  linkedCommits?: string[];
  alternatives?: TraceAlternative[];
  evidence?: string[];
  next?: string[];
}

export interface TraceEdgeData {
  id: string;
  source: string;
  target: string;
  kind: TraceEdgeKind;
  label?: string;
}

export interface TraceGraph {
  schemaVersion: string;
  project: string;
  nodes: TraceNodeData[];
  edges: TraceEdgeData[];
}

export interface TraceProjectInfo {
  id: string;
  path: string;
  role: string;
  status: string;
  description: string | null;
  exists: boolean;
  git_branch: string | null;
  git_head: string | null;
  git_dirty: boolean | null;
}

export interface TraceGitLogEntry {
  hash: string;
  short: string;
  date: string;
  message: string;
}

export interface JournalEntry {
  date: string;
  content: string;
  createdAt: number;
  updatedAt: number;
}

export interface QuestionItem {
  id: string;
  title: string;
  description: string;
  status: "parked" | "triaged" | "archived" | "promoted";
  impact: number | null;
  feasibility: number | null;
  urgency: number | null;
  priorityScore: number | null;
  triageNotes: string | null;
  triagedAt: number | null;
  isSelectedForPromotion: boolean;
  promotedToHypothesisId: string | null;
  createdAt: number;
  updatedAt: number;
}

export type HypothesisVerdict =
  | "supported"
  | "partially_supported"
  | "rejected"
  | "inconclusive";

export interface LinkedDefinition {
  libraryGroupId: string;
  libraryVersionId: string;
  definitionName: string;
  versionString: string;
}

export interface LibraryDefinitionSelection {
  groupId: string;
  versionId: string;
  name: string;
  versionString: string;
}

export interface Hypothesis {
  id: string;
  inquiryId: string;
  statementH1: string;
  statementH0: string;
  independentVar: string;
  dependentVar: string;
  metric: string;
  successThreshold: string;
  status: string;
  createdAt: number;
  updatedAt: number;
  evidenceNotes?: string;
  verdict?: HypothesisVerdict | null;
  evaluatedAt?: number | null;
  promotedToLibraryId?: string | null;
  parentHypothesisId?: string | null;
  linkedDefinitions: LinkedDefinition[];
  linkedSnapshots: string[];
  protocolSteps?: string | null;
  protocolLockedAt?: number | null;
  rawData?: string | null;
  dataAnalysis?: string | null;
  executedAt?: number | null;
  boundaryConditions?: string | null;
  refinedFromId?: string | null;
  checklistSchema?: string | null;
  protocolType: "text" | "flowchart";
  protocolFlowchartJson?: string | null;
  protocolCompletedSteps?: string | null;
  sourceCardId: string | null;
  sourceHighlightId: string | null;
}

export type ChecklistFieldType = "select" | "number" | "text" | "boolean";

export type ProtocolNodeKind = "step" | "decision" | "start-end";

export interface ChecklistField {
  label: string;
  type: ChecklistFieldType;
  options?: string[];
}

export interface DataEntry {
  id: string;
  hypothesisId: string;
  dataJson: string;
  createdAt: number;
}

export interface HypothesisInsight {
  id: string;
  hypothesisId: string;
  content: string;
  createdAt: number;
}

export interface HypothesisPromotionInput {
  inquiryId: string;
  statementH1: string;
  statementH0: string;
  independentVar: string;
  dependentVar: string;
  metric: string;
  successThreshold: string;
  linkedDefinitions?: LinkedDefinition[];
}

export interface SourceHypothesisInput {
  sourceCardId: string;
  sourceHighlightId?: string | null;
  sourceTitle: string;
  sourceContent: string;
  statementH1: string;
  statementH0: string;
  independentVar: string;
  dependentVar: string;
  metric: string;
  successThreshold: string;
}

export interface ChildHypothesisInput {
  parentId: string;
  statementH1: string;
  statementH0: string;
  independentVar: string;
  dependentVar: string;
  metric: string;
  successThreshold: string;
  linkedDefinitions?: LinkedDefinition[];
}

export interface CustomFontSetting {
  id: string;
  name: string;
  dataUrl: string;
}

export type SourceCaptureKind = "edition" | "retrieval";
export type SourceLocationKind =
  | "page"
  | "paragraph"
  | "line"
  | "timestamp"
  | "time_range"
  | "text_anchor"
  | "unknown";
export type ExtractionKind =
  | "quote"
  | "paraphrase"
  | "summary"
  | "learner_interpretation"
  | "source_claim";
export type ExtractionReviewStatus =
  | "unreviewed"
  | "accepted"
  | "rejected"
  | "needs_correction";
export type KnowledgeItemKind = "source_claim" | "concept" | "principle" | "procedure";
export type ClaimRelationKind =
  | "supports"
  | "contradicts"
  | "qualifies"
  | "defines"
  | "describes";
export type FrameworkChangeClass =
  | "refinement"
  | "extension"
  | "revision"
  | "replacement";
export type FrameworkLifecycle = "draft" | "review" | "published" | "withdrawn" | "retired";
export type FrameworkVersionStatus = "draft" | "review" | "published" | "withdrawn";
export type FrameworkReviewStatus = "approved" | "changes_requested" | "rejected";
export type AcquisitionProvenanceTargetKind =
  | "source"
  | "source_version"
  | "source_location"
  | "extraction"
  | "source_claim"
  | "concept"
  | "principle"
  | "procedure"
  | "framework"
  | "framework_version";
export type LegacySourceEntityKind = "source" | "segment" | "extract" | "link";
export type LegacySourceRevisionType = "create" | "update" | "correct" | "retract" | "review";
export type FrameworkMembershipRole =
  | "core"
  | "supporting"
  | "counterpoint"
  | "method"
  | "example"
  | "context";

export interface SourceVersionDto {
  id: string;
  sourceId: string;
  captureKind: SourceCaptureKind;
  editionLabel: string | null;
  canonicalUri: string | null;
  capturedAtUs: number;
  contentSha256: string | null;
  externalAssetRef: string | null;
  metadataJson: string;
  createdBy: string;
  createdAtUs: number;
}

export interface CreateSourceVersionReq {
  sourceId: string;
  captureKind: SourceCaptureKind;
  editionLabel: string | null;
  canonicalUri: string | null;
  capturedAtUs: number | null;
  contentSha256: string | null;
  externalAssetRef: string | null;
  metadataJson: string | null;
}

export interface SourceLocationDto {
  id: string;
  sourceId: string;
  sourceVersionId: string;
  legacySegmentId: string | null;
  label: string;
  locationKind: SourceLocationKind;
  pageStart: number | null;
  pageEnd: number | null;
  paragraphStart: number | null;
  paragraphEnd: number | null;
  lineStart: number | null;
  lineEnd: number | null;
  timeStartMs: number | null;
  timeEndMs: number | null;
  stableSelector: string | null;
  locatorText: string | null;
  notes: string;
  parentSegmentId: string | null;
  orderIndex: number;
  createdAtUs: number;
}

export interface CreateSourceLocationReq {
  sourceId: string;
  sourceVersionId: string;
  legacySegmentId: string | null;
  label: string;
  locationKind: SourceLocationKind;
  pageStart: number | null;
  pageEnd: number | null;
  paragraphStart: number | null;
  paragraphEnd: number | null;
  lineStart: number | null;
  lineEnd: number | null;
  timeStartMs: number | null;
  timeEndMs: number | null;
  stableSelector: string | null;
  locatorText: string | null;
  notes: string | null;
  parentSegmentId: string | null;
  orderIndex: number | null;
}

export interface KnowledgeExtractionDto {
  id: string;
  sourceId: string;
  sourceVersionId: string;
  sourceSegmentId: string;
  kind: ExtractionKind;
  content: string;
  language: string | null;
  parentExtractionId: string | null;
  toolIdentity: string | null;
  toolVersion: string | null;
  promptConfigIdentity: string | null;
  confidenceDecimal: string | null;
  uncertainty: string | null;
  reviewStatus: ExtractionReviewStatus;
  supersededById: string | null;
  createdBy: string;
  createdAtUs: number;
}

export interface CreateKnowledgeExtractionReq {
  sourceId: string;
  sourceVersionId: string;
  sourceSegmentId: string;
  kind: ExtractionKind;
  content: string;
  language: string | null;
  parentExtractionId: string | null;
  toolIdentity: string | null;
  toolVersion: string | null;
  promptConfigIdentity: string | null;
  confidenceDecimal: string | null;
  uncertainty: string | null;
  createdBy: string | null;
}

export interface ReviewExtractionReq {
  extractionId: string;
  reviewStatus: Exclude<ExtractionReviewStatus, "unreviewed">;
  reviewerId: string | null;
  rationale: string | null;
}

export interface CorrectExtractionReq {
  extractionId: string;
  content: string;
  reason: string;
  actorId: string | null;
}

export interface SourceClaimDto {
  id: string;
  sourceId: string;
  sourceVersionId: string;
  extractionId: string;
  statement: string;
  assertedBy: string | null;
  scope: string | null;
  createdAtUs: number;
}

export interface CreateSourceClaimReq {
  extractionId: string;
  statement: string;
  assertedBy: string | null;
  scope: string | null;
}

export interface KnowledgeItemDto {
  id: string;
  kind: KnowledgeItemKind;
  title: string;
  content: string;
  lifecycle: string;
  sourceClaimId: string | null;
  relationKind: ClaimRelationKind | null;
  createdAtUs: number;
}

export interface CreateKnowledgeItemReq {
  kind: Exclude<KnowledgeItemKind, "source_claim">;
  title: string;
  content: string;
  sourceClaimId: string;
}

export interface ClaimRelationDto {
  targetKind: KnowledgeItemKind;
  targetId: string;
  relationKind: ClaimRelationKind;
  rationale: string;
}

export interface CreateClaimRelationReq {
  sourceClaimId: string;
  targetKind: KnowledgeItemKind;
  targetId: string;
  relationKind: ClaimRelationKind;
  rationale: string | null;
  createdBy: string | null;
}

export interface FrameworkDto {
  id: string;
  title: string;
  description: string;
  lifecycle: FrameworkLifecycle;
  currentVersionId: string | null;
  createdBy: string | null;
  createdAtUs: number;
}

export interface FrameworkVersionItemDto {
  id: string;
  frameworkVersionId: string;
  itemKind: KnowledgeItemKind;
  itemId: string;
  membershipRole: FrameworkMembershipRole;
  itemOrder: number;
  rationale: string;
  itemTitle: string;
}

export interface FrameworkVersionDto {
  id: string;
  frameworkId: string;
  versionNo: number;
  parentVersionId: string | null;
  title: string;
  description: string;
  status: "draft" | "review" | "published" | "withdrawn";
  changeClass: FrameworkChangeClass | null;
  changeRationale: string | null;
  createdBy: string | null;
  createdAtUs: number;
  publishedAtUs: number | null;
  items: FrameworkVersionItemDto[];
}

export interface FrameworkReviewDto {
  id: string;
  frameworkVersionId: string;
  reviewStatus: FrameworkReviewStatus;
  reviewerId: string | null;
  rationale: string;
  reviewedAtUs: number;
}

export interface CreateFrameworkReq {
  title: string;
  description: string | null;
  createdBy: string | null;
}

export interface CreateFrameworkVersionReq {
  frameworkId: string;
  changeClass: FrameworkChangeClass;
  changeRationale: string;
  affectedClaimIds: string[];
  assumptionClaimIds: string[];
  createdBy: string | null;
}

export interface AddFrameworkItemReq {
  frameworkVersionId: string;
  itemKind: KnowledgeItemKind;
  itemId: string;
  membershipRole: FrameworkMembershipRole;
  itemOrder: number;
  rationale: string | null;
}

export interface ReorderFrameworkItemsReq {
  frameworkVersionId: string;
  orderedItemIds: string[];
}

export interface UpdateFrameworkVersionReq {
  frameworkVersionId: string;
  title: string;
  description: string;
}

export interface ReviewFrameworkReq {
  frameworkVersionId: string;
  reviewStatus: FrameworkReviewStatus;
  reviewerId: string | null;
  rationale: string | null;
}

export interface ProvenanceEventDto {
  id: string;
  targetKind: AcquisitionProvenanceTargetKind;
  targetId: string;
  eventKind: string;
  actorId: string | null;
  occurredAtUs: number;
  reason: string;
  beforeJson: string | null;
  afterJson: string | null;
}

export interface SourceRevisionDto {
  id: string;
  entityKind: LegacySourceEntityKind;
  entityId: string;
  revisionType: LegacySourceRevisionType;
  beforeJson: string | null;
  afterJson: string;
  reason: string;
  createdBy: string;
  createdAt: number;
}

export type ResearchPathwayKey =
  | "existing_knowledge_acquisition"
  | "existing_knowledge_investigation"
  | "first_principles_discovery";
export type ResearchProjectStatus = "proposed" | "active" | "paused" | "completed" | "archived";
export type ResearchDomainStatus = "proposed" | "active" | "deprecated";
export type ReviewLifecycleStatus = "draft" | "reviewed";
export type ResearchGapStatus = "candidate" | "assessed" | "accepted" | "rejected" | "addressed" | "superseded";
export type ResearchQuestionStatus =
  | "legacy_unassessed" | "draft" | "review" | "accepted" | "rejected"
  | "deferred" | "refined" | "superseded";
export type ResearchQuestionRelation = "derived_from" | "refined_from" | "supersedes";
export type ResearchHypothesisStatus =
  | "legacy_untyped" | "draft" | "review" | "preregistered" | "closed";
export type HypothesisDefinitionStatus = "draft" | "review" | "preregistered";
export type HypothesisVariableRole = "independent" | "dependent" | "covariate";
export type HypothesisValueKind = "continuous" | "discrete" | "categorical" | "qualitative" | "other";
export type ResearchPolarity = "supports" | "contradicts" | "qualifies";
export type PhenomenonStatus = "identified" | "characterized" | "retired";
export type ObservationPlanStatus = "draft" | "reviewed" | "preregistered" | "withdrawn";
export type ObservationValueState =
  | "observed" | "not_measured" | "not_recorded" | "unknown"
  | "not_applicable" | "withheld" | "invalid";
export type ObservationQualityStatus = "unreviewed" | "accepted" | "excluded" | "retracted";
export type PatternStatus = "candidate" | "examined" | "supported" | "unsupported" | "retired";
export type IdeaStatus = "captured" | "explored" | "promoted" | "abandoned";
export type HypothesisExpectedDirection = "positive" | "negative" | "non_directional" | "equivalence" | "other";
export type ResearchDesignStatus = "draft" | "review" | "preregistered" | "withdrawn";
export type GapReviewDecision = "accepted" | "rejected";
export type HypothesisDesignRelation = "tests" | "competing_design";

export interface ResearchPathwayDto {
  id: string;
  pathwayKey: ResearchPathwayKey;
  title: string;
  description: string;
}

export interface ResearchProjectDto {
  id: string;
  title: string;
  aim: string;
  lifecycleStatus: ResearchProjectStatus;
  pathwayKeys: ResearchPathwayKey[];
  domainIds: string[];
  createdBy: string | null;
  createdAtUs: number;
  updatedAtUs: number;
}

export interface ResearchDomainDto {
  id: string;
  name: string;
  description: string;
  parentDomainId: string | null;
  lifecycleStatus: ResearchDomainStatus;
  createdBy: string | null;
  createdAtUs: number;
}

export interface FrameworkVersionReferenceDto {
  frameworkId: string;
  frameworkVersionId: string;
  title: string;
  versionNo: number;
}

export interface SourceVersionReferenceDto {
  sourceId: string;
  sourceVersionId: string;
  sourceTitle: string;
  editionLabel: string | null;
  sourceLocationId: string | null;
  sourceLocationLabel: string | null;
  sourceClaimId: string | null;
  capturedAtUs: number;
}

export interface ProjectKnowledgeAdoptionDto {
  id: string;
  projectId: string;
  frameworkId: string;
  frameworkVersionId: string;
  title: string;
  versionNo: number;
  useContext: string;
  adoptedBy: string | null;
  adoptedAtUs: number;
}

export interface LiteratureReviewSourceDto {
  id: string;
  sourceId: string;
  sourceVersionId: string;
  sourceLocationId: string | null;
  sourceClaimId: string | null;
  citationNote: string;
}

export interface LiteratureReviewDto {
  id: string;
  projectId: string;
  title: string;
  searchScope: string;
  lifecycleStatus: ReviewLifecycleStatus;
  reviewerId: string | null;
  reviewedAtUs: number | null;
  sources: LiteratureReviewSourceDto[];
  createdBy: string | null;
  createdAtUs: number;
}

export interface PriorKnowledgeAssessmentDto {
  id: string;
  projectId: string;
  title: string;
  assessment: string;
  frameworkVersionId: string | null;
  lifecycleStatus: ReviewLifecycleStatus;
  reviewerId: string | null;
  reviewedAtUs: number | null;
  createdBy: string | null;
  createdAtUs: number;
}

export interface ResearchGapDto {
  id: string;
  projectId: string;
  domainId: string | null;
  title: string;
  statement: string;
  lifecycleStatus: ResearchGapStatus;
  literatureReviewIds: string[];
  priorAssessmentIds: string[];
  hasReviewedException: boolean;
  createdBy: string | null;
  createdAtUs: number;
  updatedAtUs: number;
}

export interface ResearchQuestionDto {
  id: string;
  title: string;
  description: string;
  projectId: string | null;
  researchGapId: string | null;
  domainId: string | null;
  lifecycleStatus: ResearchQuestionStatus;
  ownershipState: "project" | "unassigned" | "unknown";
  migrationNote: string | null;
  rationale: string;
  legacyStatus: QuestionItem["status"];
  createdAt: number;
  updatedAt: number;
}

export interface ResearchQuestionLineageDto {
  id: string;
  parentQuestionId: string;
  derivedQuestionId: string;
  relationKind: ResearchQuestionRelation;
  rationale: string;
  createdBy: string | null;
  createdAtUs: number;
}

export interface PhenomenonDto {
  id: string;
  projectId: string;
  domainId: string | null;
  title: string;
  description: string;
  populationScope: string;
  context: string;
  temporalBoundary: string;
  lifecycleStatus: PhenomenonStatus;
  createdBy: string | null;
  createdAtUs: number;
}

export interface ObservationPlanVersionDto {
  id: string;
  planId: string;
  versionNo: number;
  measurementMethod: string;
  sampleFrame: string;
  conditionsTemplate: string;
  inclusionRules: string;
  lifecycleStatus: ObservationPlanStatus;
  createdBy: string | null;
  createdAtUs: number;
  preregisteredAtUs: number | null;
}

export interface ObservationPlanDto {
  id: string;
  projectId: string;
  phenomenonId: string;
  title: string;
  createdBy: string | null;
  createdAtUs: number;
  versions: ObservationPlanVersionDto[];
}

export interface DatasetVersionReferenceDto {
  id: string;
  projectId: string;
  title: string;
  versionLabel: string;
  externalUri: string | null;
  contentSha256: string | null;
  sourceVersionId: string | null;
  createdBy: string | null;
  createdAtUs: number;
}

export interface ObservationDto {
  id: string;
  projectId: string;
  phenomenonId: string;
  planId: string;
  planVersionId: string;
  datasetVersionReferenceId: string | null;
  measuredAtUs: number;
  valueDecimal: string | null;
  valueState: ObservationValueState;
  unit: string | null;
  measurementMethod: string;
  conditions: string;
  sampleIdentifier: string | null;
  qualityStatus: ObservationQualityStatus;
  supersedesObservationId: string | null;
  createdBy: string | null;
  createdAtUs: number;
}

export interface PatternDto {
  id: string;
  projectId: string;
  title: string;
  statement: string;
  analysisMethod: string;
  uncertainty: string;
  lifecycleStatus: PatternStatus;
  observationIds: string[];
  datasetVersionReferenceIds: string[];
  createdBy: string | null;
  createdAtUs: number;
}

export interface IdeaDto {
  id: string;
  projectId: string;
  title: string;
  content: string;
  lifecycleStatus: IdeaStatus;
  phenomenonId: string | null;
  observationId: string | null;
  patternId: string | null;
  questionId: string | null;
  createdBy: string | null;
  createdAtUs: number;
}

export interface HypothesisVariableDto {
  id: string;
  hypothesisVersionId: string;
  role: HypothesisVariableRole;
  name: string;
  operationalDefinition: string;
  valueKind: HypothesisValueKind;
  unit: string | null;
  measurementMethod: string;
}

export interface HypothesisDefinitionVersionDto {
  id: string;
  hypothesisId: string;
  versionNo: number;
  parentVersionId: string | null;
  statementH0: string;
  statementH1: string;
  populationScope: string;
  researchScope: string;
  expectedDirection: HypothesisExpectedDirection;
  expectedEffectDecimal: string | null;
  expectedEffectUnit: string | null;
  falsificationCriteria: string;
  assumptions: string;
  lifecycleStatus: HypothesisDefinitionStatus;
  createdBy: string | null;
  createdAtUs: number;
  preregisteredAtUs: number | null;
  variables: HypothesisVariableDto[];
}

export interface HypothesisModelReferenceDto {
  id: string;
  hypothesisId: string;
  modelId: string;
  modelVersionId: string;
  label: string;
  rationale: string;
  resolutionStatus: "unresolved";
}

export interface HypothesisEvidenceDto {
  targetKind: "source_claim" | "observation" | "pattern";
  targetId: string;
  polarity: ResearchPolarity;
  rationale: string;
  sourceId: string | null;
  sourceVersionId: string | null;
  sourceLocationId: string | null;
}

export interface ResearchHypothesisDto {
  id: string;
  projectId: string | null;
  researchQuestionId: string | null;
  legacyHypothesisId: string | null;
  parentHypothesisId: string | null;
  ownershipState: "project" | "unassigned" | "unknown";
  lifecycleStatus: ResearchHypothesisStatus;
  definitions: HypothesisDefinitionVersionDto[];
  competingHypothesisIds: string[];
  modelReferences: HypothesisModelReferenceDto[];
  evidence: HypothesisEvidenceDto[];
  createdBy: string | null;
  createdAtUs: number;
}

export interface ResearchExperimentDesignVersionDto {
  id: string;
  designId: string;
  projectId: string;
  title: string;
  versionNo: number;
  description: string;
  protocol: string;
  lifecycleStatus: ResearchDesignStatus;
  createdBy: string | null;
  createdAtUs: number;
  preregisteredAtUs: number | null;
}

export interface ResearchWorkspaceDto {
  pathways: ResearchPathwayDto[];
  projects: ResearchProjectDto[];
  domains: ResearchDomainDto[];
  frameworkVersions: FrameworkVersionReferenceDto[];
  sourceVersions: SourceVersionReferenceDto[];
  adoptions: ProjectKnowledgeAdoptionDto[];
  literatureReviews: LiteratureReviewDto[];
  priorAssessments: PriorKnowledgeAssessmentDto[];
  gaps: ResearchGapDto[];
  questions: ResearchQuestionDto[];
  questionLineage: ResearchQuestionLineageDto[];
  phenomena: PhenomenonDto[];
  observationPlans: ObservationPlanDto[];
  datasetVersions: DatasetVersionReferenceDto[];
  observations: ObservationDto[];
  patterns: PatternDto[];
  ideas: IdeaDto[];
  hypotheses: ResearchHypothesisDto[];
  designVersions: ResearchExperimentDesignVersionDto[];
}

export interface CreateResearchProjectReq {
  title: string;
  aim: string;
  pathways: ResearchPathwayKey[];
  createdBy: string | null;
}

export interface CreateResearchDomainReq {
  name: string;
  description: string;
  parentDomainId: string | null;
  createdBy: string | null;
}

export interface CreateResearchGapReq {
  projectId: string;
  domainId: string | null;
  title: string;
  statement: string;
  createdBy: string | null;
}

export interface CreateResearchQuestionReq {
  projectId: string | null;
  researchGapId: string | null;
  domainId: string | null;
  title: string;
  description: string;
  rationale: string;
  createdBy: string | null;
}

export interface CreateLiteratureReviewReq {
  projectId: string;
  title: string;
  searchScope: string;
  createdBy: string | null;
}

export interface CreateReviewedLiteratureReviewReq {
  projectId: string;
  title: string;
  searchScope: string;
  sourceId: string;
  sourceVersionId: string;
  sourceLocationId: string | null;
  sourceClaimId: string | null;
  citationNote: string;
  createdBy: string | null;
  reviewerId: string | null;
}

export interface AddLiteratureSourceReq {
  literatureReviewId: string;
  sourceId: string;
  sourceVersionId: string;
  sourceLocationId: string | null;
  sourceClaimId: string | null;
  citationNote: string;
}

export interface CreatePriorKnowledgeAssessmentReq {
  projectId: string;
  title: string;
  assessment: string;
  frameworkVersionId: string | null;
  createdBy: string | null;
}

export interface AddGapEvidenceReq {
  gapId: string;
  literatureReviewId: string | null;
  priorAssessmentId: string | null;
}

export interface ReviewGapExceptionReq {
  gapId: string;
  rationale: string;
  reviewerId: string | null;
  reviewDecision: GapReviewDecision;
}

export interface CreatePhenomenonReq {
  projectId: string;
  domainId: string | null;
  title: string;
  description: string;
  populationScope: string;
  context: string;
  temporalBoundary: string;
  createdBy: string | null;
}

export interface CreateObservationPlanReq {
  projectId: string;
  phenomenonId: string;
  title: string;
  measurementMethod: string;
  sampleFrame: string;
  conditionsTemplate: string;
  inclusionRules: string;
  createdBy: string | null;
}

export interface CreateDatasetVersionReferenceReq {
  projectId: string;
  title: string;
  versionLabel: string;
  externalUri: string | null;
  contentSha256: string | null;
  sourceVersionId: string | null;
  createdBy: string | null;
}

export interface CreateObservationReq {
  projectId: string;
  phenomenonId: string;
  planId: string;
  planVersionId: string;
  datasetVersionReferenceId: string | null;
  measuredAtUs: number;
  valueDecimal: string | null;
  valueState: ObservationValueState;
  unit: string | null;
  measurementMethod: string;
  conditions: string;
  sampleIdentifier: string | null;
  createdBy: string | null;
}

export interface CreatePatternReq {
  projectId: string;
  title: string;
  statement: string;
  analysisMethod: string;
  uncertainty: string;
  observationIds: string[];
  datasetVersionReferenceIds: string[];
  createdBy: string | null;
}

export interface CreateIdeaReq {
  projectId: string;
  title: string;
  content: string;
  phenomenonId: string | null;
  observationId: string | null;
  patternId: string | null;
  questionId: string | null;
  createdBy: string | null;
}

export interface HypothesisVariableInput {
  role: HypothesisVariableRole;
  name: string;
  operationalDefinition: string;
  valueKind: HypothesisValueKind;
  unit: string | null;
  measurementMethod: string;
}

export interface CreateTypedHypothesisReq {
  projectId: string;
  researchQuestionId: string;
  parentHypothesisId: string | null;
  lineageRationale: string | null;
  statementH0: string;
  statementH1: string;
  populationScope: string;
  researchScope: string;
  expectedDirection: HypothesisExpectedDirection;
  expectedEffectDecimal: string | null;
  expectedEffectUnit: string | null;
  falsificationCriteria: string;
  assumptions: string;
  variables: HypothesisVariableInput[];
  createdBy: string | null;
}

export interface LinkCompetingHypothesesReq {
  hypothesisId: string;
  competingHypothesisId: string;
  rationale: string;
  createdBy: string | null;
}

export interface AddHypothesisModelReferenceReq {
  hypothesisId: string;
  modelId: string;
  modelVersionId: string;
  label: string;
  rationale: string;
  createdBy: string | null;
}

export interface AddHypothesisEvidenceReq {
  hypothesisVersionId: string;
  sourceClaimId: string | null;
  observationId: string | null;
  patternId: string | null;
  polarity: ResearchPolarity;
  rationale: string;
  createdBy: string | null;
}

export interface CreateResearchDesignReq {
  projectId: string;
  title: string;
  description: string;
  protocol: string;
  createdBy: string | null;
}

export interface CreateLinkedResearchDesignReq {
  projectId: string;
  title: string;
  description: string;
  protocol: string;
  hypothesisVersionId: string;
  relationshipKind: HypothesisDesignRelation;
  rationale: string;
  createdBy: string | null;
}

export interface LinkHypothesisDesignReq {
  hypothesisVersionId: string;
  designVersionId: string;
  relationshipKind: HypothesisDesignRelation;
  rationale: string;
  createdBy: string | null;
}

export type DatasetRole =
  | "training" | "validation" | "test" | "exploratory" | "prospective" | "forward";
export type ExperimentRunStatus =
  | "created" | "ready" | "running" | "completed" | "failed" | "cancelled" | "inconclusive";
export type ExperimentStudyType =
  | "randomized_controlled" | "quasi_experimental" | "observational"
  | "market_backtest" | "market_forward" | "case_study" | "other";
export type DatasetSampleValueKind = "decimal" | "integer" | "text" | "category" | "boolean";
export type MissingValueState = "not_observed" | "not_applicable" | "censored" | "unknown";
export type MarketOrderSide = "buy" | "sell";
export type MarketOrderType = "market" | "limit" | "stop" | "stop_limit" | "other";
export type ExperimentRunLogSeverity = "debug" | "info" | "warning" | "error";

export type DatasetSampleValue =
  | { kind: "decimal"; value: string; unit: string; uncertainty: string | null }
  | { kind: "integer"; value: number; unit: string | null }
  | { kind: "text"; value: string }
  | { kind: "category"; value: string }
  | { kind: "boolean"; value: boolean }
  | { kind: "missing"; state: MissingValueState; unit: string | null };

export interface SampleProvenanceRef {
  referenceKind: string;
  referenceId: string;
  rationale: string;
}

export interface DataSampleDto {
  sampleId: string;
  datasetVersionId: string;
  occurredAtUs: number | null;
  entityRef: string | null;
  values: Record<string, DatasetSampleValue>;
  provenance: SampleProvenanceRef[];
}

export interface DecimalMeasurement {
  value: string;
  unit: string;
}

export interface TradeSampleDto extends DataSampleDto {
  instrument: string;
  orderSide: MarketOrderSide | null;
  orderType: MarketOrderType | null;
  orderQuantity: DecimalMeasurement | null;
  executionPrice: DecimalMeasurement | null;
  fees: DecimalMeasurement | null;
  slippage: DecimalMeasurement | null;
  venue: string | null;
  marketSession: string | null;
  sessionAssumptions: string[];
  decisionAtUs: number | null;
  executionAtUs: number | null;
}

export type DatasetSampleDto =
  | { sampleKind: "data_sample"; sample: DataSampleDto }
  | { sampleKind: "trade_sample"; sample: TradeSampleDto };

export interface DatasetFieldDefinitionDto {
  key: string;
  valueKind: DatasetSampleValueKind;
  required: boolean;
  unit: string | null;
}

export interface DatasetSampleSchemaDto {
  schemaId: string;
  schemaVersion: number;
  fields: DatasetFieldDefinitionDto[];
  allowAdditionalFields: boolean;
}

export interface DatasetRolePinDto {
  datasetVersionId: string;
  role: DatasetRole;
  rationale: string;
}

export interface OperationalDefinitionDto {
  name: string;
  definition: string;
  unit: string | null;
  measurementMethod: string | null;
}

export interface ComparisonGroupDto {
  name: string;
  description: string;
  intervention: boolean;
}

export interface DataCollectionScheduleEntryDto {
  stage: string;
  schedule: string;
  instrumentIds: string[];
}

export interface CodeEnvironmentIdentityDto {
  codeUri: string;
  codeRevision: string | null;
  environment: string;
  dependencyLockSha256: string | null;
}

export interface MarketDesignContextDto {
  instrument: string;
  orderExecutionContext: string;
  feeAssumptions: string;
  slippageAssumptions: string;
  marketSessionAssumptions: string[];
  decisionTimestampPolicy: string;
  executionTimestampPolicy: string;
}

export interface ExperimentDesignSpecificationDto {
  targetResearchQuestionId: string | null;
  targetHypothesisVersionId: string | null;
  targetModelVersionId: string | null;
  estimand: string;
  studyType: ExperimentStudyType;
  intervention: string | null;
  control: string | null;
  comparisonGroups: ComparisonGroupDto[];
  factors: OperationalDefinitionDto[];
  outcomes: OperationalDefinitionDto[];
  covariates: OperationalDefinitionDto[];
  unitOfAnalysis: string;
  samplingFrame: string;
  eligibilityRules: string[];
  exclusionRules: string[];
  plannedSampleSize: number | null;
  sampleSizeRationale: string;
  stoppingRule: string;
  randomization: string | null;
  blinding: string | null;
  instruments: string[];
  operationalDefinitions: OperationalDefinitionDto[];
  dataCollectionSchedule: DataCollectionScheduleEntryDto[];
  primaryOutcomes: string[];
  secondaryOutcomes: string[];
  preregisteredAnalysisPlan: string;
  multiplicityStrategy: string;
  deviationsPolicy: string;
  datasets: DatasetRolePinDto[];
  datasetPlanRationale: string;
  codeEnvironment: CodeEnvironmentIdentityDto;
  randomSeeds: string[];
  reproducibilityInstructions: string;
  marketContext: MarketDesignContextDto | null;
}

export interface ExperimentPreregistrationDto {
  id: string;
  designVersionId: string;
  specificationSha256: string;
  reviewerId: string | null;
  rationale: string;
  preregisteredAtUs: number;
}

export interface ExperimentDesignVersionDto {
  id: string;
  designId: string;
  projectId: string;
  title: string;
  versionNo: number;
  description: string;
  protocol: string;
  lifecycleStatus: ResearchDesignStatus;
  specification: ExperimentDesignSpecificationDto | null;
  specificationSha256: string | null;
  parentVersionId: string | null;
  changeRationale: string | null;
  createdBy: string | null;
  createdAtUs: number;
  preregistration: ExperimentPreregistrationDto | null;
}

export interface ExperimentDesignDatasetPinDto {
  datasetVersionId: string;
  datasetId: string;
  datasetTitle: string;
  versionNo: number;
  role: DatasetRole;
  rationale: string;
}

export interface ExperimentRunDatasetPinDto {
  datasetVersionId: string;
  datasetId: string;
  datasetTitle: string;
  versionNo: number;
  role: DatasetRole;
  usageKind: "input" | "output";
}

export interface ExperimentRunEventDto {
  id: string;
  eventKind: string;
  fromStatus: ExperimentRunStatus | null;
  toStatus: ExperimentRunStatus | null;
  actorId: string | null;
  details: string;
  occurredAtUs: number;
}

export interface ExperimentRunDeviationDto {
  id: string;
  description: string;
  rationale: string;
  impactAssessment: string;
  recordedBy: string | null;
  recordedAtUs: number;
}

export interface ExperimentRunLogDto {
  id: string;
  severity: ExperimentRunLogSeverity;
  message: string;
  details: unknown;
  recordedAtUs: number;
}

export interface ExperimentRunDto {
  id: string;
  projectId: string;
  designVersionId: string;
  lifecycleStatus: ExperimentRunStatus;
  createdBy: string | null;
  createdAtUs: number;
  startedAtUs: number | null;
  endedAtUs: number | null;
  completionSummary: string | null;
  failureDetails: string | null;
  datasetPins: ExperimentRunDatasetPinDto[];
  events: ExperimentRunEventDto[];
  deviations: ExperimentRunDeviationDto[];
  logs: ExperimentRunLogDto[];
}

export interface ExperimentDatasetAssetDto {
  id: string;
  datasetVersionId: string;
  storageKey: string;
  originalName: string;
  mediaType: string;
  byteLength: number;
  contentSha256: string;
}

export interface ExperimentDatasetVersionDto {
  id: string;
  datasetId: string;
  versionNo: number;
  schema: DatasetSampleSchemaDto;
  expectedSampleCount: number;
  actualSampleCount: number;
  lifecycleStatus: "staged" | "sealed";
  contentSha256: string | null;
  createdBy: string | null;
  createdAtUs: number;
  sealedAtUs: number | null;
  assets: ExperimentDatasetAssetDto[];
}

export interface ExperimentDatasetDto {
  id: string;
  projectId: string;
  title: string;
  description: string;
  createdBy: string | null;
  createdAtUs: number;
  versions: ExperimentDatasetVersionDto[];
}

export interface ExperimentDatasetSplitGroupDto {
  id: string;
  datasetVersionId: string;
  splitGroupKey: string;
  requireDisjoint: boolean;
  assignments: ExperimentSampleSplitAssignmentDto[];
}

export interface ExperimentSampleSplitAssignmentDto {
  id: string;
  sampleId: string;
  role: DatasetRole;
  splitKey: string;
}

export interface DatasetTransformationInputDto {
  datasetVersionId: string;
  role: DatasetRole;
}

export interface ExperimentDatasetTransformationDto {
  id: string;
  outputDatasetVersionId: string;
  method: string;
  parameters: unknown;
  provenance: unknown;
  inputs: DatasetTransformationInputDto[];
  createdBy: string | null;
  createdAtUs: number;
}

export interface ExperimentWorkspaceDto {
  designVersions: ExperimentDesignVersionDto[];
  runs: ExperimentRunDto[];
  datasets: ExperimentDatasetDto[];
  splitGroups: ExperimentDatasetSplitGroupDto[];
  transformations: ExperimentDatasetTransformationDto[];
}

export interface DatasetSamplePageDto {
  datasetVersionId: string;
  samples: DatasetSampleDto[];
  totalCount: number;
  offset: number;
  limit: number;
}

export interface CreateExperimentDesignVersionReq {
  projectId: string;
  designId: string | null;
  parentVersionId: string | null;
  title: string;
  description: string;
  protocol: string;
  changeRationale: string;
  specification: ExperimentDesignSpecificationDto;
  createdBy: string | null;
}

export interface PreregisterExperimentDesignReq {
  designVersionId: string;
  reviewerId: string | null;
  rationale: string;
}

export interface CreateExperimentRunReq {
  designVersionId: string;
  datasetPins: DatasetRolePinDto[];
  createdBy: string | null;
}

export interface TransitionExperimentRunReq {
  runId: string;
  nextStatus: ExperimentRunStatus;
  actorId: string | null;
  details: string;
  completionSummary: string | null;
  failureDetails: string | null;
  outputDatasetPins: DatasetRolePinDto[];
}

export interface AddExperimentRunDeviationReq {
  runId: string;
  description: string;
  rationale: string;
  impactAssessment: string;
  recordedBy: string | null;
}

export interface AddExperimentRunLogReq {
  runId: string;
  severity: ExperimentRunLogSeverity;
  message: string;
  details: unknown;
}

export interface CreateDatasetReq {
  projectId: string;
  title: string;
  description: string;
  createdBy: string | null;
}

export interface CreateDatasetVersionReq {
  datasetId: string;
  schema: DatasetSampleSchemaDto;
  expectedSampleCount: number;
  createdBy: string | null;
}

export interface AddDatasetSamplesReq {
  datasetVersionId: string;
  samples: DatasetSampleDto[];
}

export interface RegisterDatasetAssetReq {
  datasetVersionId: string;
  sourcePath: string;
  originalName: string;
  mediaType: string;
  expectedSha256: string | null;
}

export interface DatasetSplitAssignmentReq {
  sampleId: string;
  role: DatasetRole;
  splitKey: string;
}

export interface AddDatasetSplitGroupReq {
  datasetVersionId: string;
  splitGroupKey: string;
  requireDisjoint: boolean;
  assignments: DatasetSplitAssignmentReq[];
}

export interface DatasetTransformationInputReq {
  datasetVersionId: string;
  role: DatasetRole;
}

export interface CreateDatasetTransformationReq {
  outputDatasetVersionId: string;
  method: string;
  parameters: unknown;
  provenance: unknown;
  inputs: DatasetTransformationInputReq[];
  createdBy: string | null;
}
