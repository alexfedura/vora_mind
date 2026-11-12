export type TaskFlowType = "task" | "ai";

export interface UpdaterFailureMarkerDto {
  version: string;
  failedAt: number;
}

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

export type AnalysisApplicability<T> =
  | { state: "applicable"; details: T }
  | { state: "not_applicable"; reason: string }
  | { state: "not_assessed"; rationale: string | null };

export interface StatisticalIntegrityPlanDto {
  overfitting: AnalysisApplicability<{
    trainingTuningSeparation: string;
    parameterSearchSpace: string;
    modelSelectionProcess: string;
    complexityReporting: string;
    sensitivityReporting: string;
  }>;
  dataLeakage: AnalysisApplicability<{
    dataCutoffs: string;
    transformationOrder: string;
    featureSelectionProvenance: string;
    evaluationDataIsolation: string;
  }>;
  selectionBias: AnalysisApplicability<{
    samplingFrame: string;
    inclusionRules: string;
    exclusionRules: string;
    filteredAndMissingCases: string;
    survivorPopulation: string;
  }>;
  lookAhead: AnalysisApplicability<{
    featureAvailabilityPolicy: string;
    predictionTimestampPolicy: string;
    decisionTimestampPolicy: string;
  }>;
  multipleTesting: AnalysisApplicability<{
    testFamily: string;
    correctionMethod: string | null;
    noCorrectionJustification: string | null;
  }>;
  datasetSeparation: AnalysisApplicability<{
    requireDisjointSplits: boolean;
    selectionRationale: string;
  }>;
  outOfSample: AnalysisApplicability<{
    evaluationPeriod: string;
    evaluationPopulation: string;
    modelSelectionUsage: "not_used" | "used_for_selection" | "partially_used";
  }>;
  walkForward: AnalysisApplicability<{
    windows: Array<{
      label: string;
      trainingStartAtUs: number;
      trainingEndAtUs: number;
      validationStartAtUs: number;
      validationEndAtUs: number;
      testStartAtUs: number;
      testEndAtUs: number;
    }>;
    refitCadence: string;
    aggregationMethod: string;
  }>;
  forwardTesting: AnalysisApplicability<{
    executionKind: "prospective" | "paper" | "live";
    actualExecutionConditions: string;
    costsAndFees: string;
  }>;
  reproducibility: AnalysisApplicability<{
    protocolVersion: string;
    rawInputChecksumsRequired: boolean;
    transformationLineageRequired: boolean;
    codeAndEnvironmentRequired: boolean;
    parametersAndSeedsRequired: boolean;
    timestampsAndDeviationsRequired: boolean;
  }>;
}

export interface AnalysisDefinitionSpecificationDto {
  protocolVersion: string;
  requiredInputRoles: DatasetRole[];
  parametersSchema: Record<string, unknown>;
  integrity: StatisticalIntegrityPlanDto;
}

export type AnalysisVerificationStatus = "passed" | "failed";
export type AnalysisDefinitionStatus = "draft" | "review" | "locked" | "retired";
export type AnalysisRunStatus =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled"
  | "inconclusive";
export type AnalysisLogSeverity = "info" | "warning" | "error";
export type AnalysisIntegritySeverity = "warning" | "blocking";
export type EvidencePolarity = "supports" | "contradicts" | "qualifies" | "neutral";
export type EvidenceStatus =
  | "recorded"
  | "assessed"
  | "accepted"
  | "contested"
  | "rejected"
  | "superseded"
  | "retracted";
export type FindingConclusionKind = "result" | "null_result" | "contradictory" | "inconclusive";
export type FindingStatus =
  | "draft"
  | "review"
  | "accepted"
  | "contested"
  | "unsupported"
  | "inconclusive"
  | "superseded"
  | "retracted";
export type FindingRelationKind = "contests" | "contradicts" | "qualifies" | "supersedes";

export interface AnalysisMethodVersionDto {
  id: string;
  methodId: string;
  projectId: string;
  name: string;
  versionNo: number;
  versionLabel: string;
  codeUri: string;
  codeRevision: string;
  dependencyLockSha256: string | null;
  environment: Record<string, unknown>;
  assumptions: string[];
  testSuiteUri: string;
  testSuiteSha256: string;
  testsTotal: number;
  testsPassed: number;
  verificationStatus: AnalysisVerificationStatus;
  contentSha256: string;
  createdBy: string | null;
  createdAtUs: number;
}

export interface MetricDefinitionVersionDto {
  id: string;
  metricDefinitionId: string;
  projectId: string;
  key: string;
  name: string;
  versionNo: number;
  versionLabel: string;
  formula: string;
  unit: string;
  testSuiteUri: string;
  testSuiteSha256: string;
  testsTotal: number;
  testsPassed: number;
  verificationStatus: AnalysisVerificationStatus;
  contentSha256: string;
  createdBy: string | null;
  createdAtUs: number;
}

export interface AnalysisDefinitionVersionDto {
  id: string;
  definitionId: string;
  projectId: string;
  versionNo: number;
  title: string;
  versionLabel: string;
  parentVersionId: string | null;
  changeRationale: string | null;
  methodVersionId: string;
  metricDefinitionVersionIds: string[];
  specification: AnalysisDefinitionSpecificationDto;
  specificationSha256: string;
  lifecycleStatus: AnalysisDefinitionStatus;
  createdBy: string | null;
  createdAtUs: number;
  lockedAtUs: number | null;
}

export interface AnalysisInputSampleDto {
  sampleId: string;
  splitKey: string | null;
  featureAvailableAtUs: number | null;
  predictionAtUs: number | null;
  decisionAtUs: number | null;
  outcomeAvailableAtUs: number | null;
}

export interface AnalysisInputDto {
  id: string;
  datasetVersionId: string;
  datasetChecksum: string;
  role: DatasetRole;
  splitGroupId: string | null;
  sampleCount: number;
  transformationLineage: unknown;
  samples: AnalysisInputSampleDto[];
}

export interface AnalysisIntegrityFindingDto {
  id: string;
  category: string;
  code: string;
  severity: AnalysisIntegritySeverity;
  message: string;
}

export interface AnalysisIntegrityDeclarationDto {
  category: string;
  applicability: "applicable" | "not_applicable" | "not_assessed";
  rationale: string;
  details: unknown;
}

export interface AnalysisOutputDto {
  id: string;
  outputKey: string;
  outputKind: string;
  title: string;
  payload: unknown;
  payloadSha256: string;
  artifactUri: string | null;
  artifactSha256: string | null;
  mediaType: string | null;
  byteLength: number | null;
  createdAtUs: number;
}

export type UncertaintyReportDto =
  | { kind: "interval"; interval: { lower: string; upper: string; level: string } }
  | { kind: "not_meaningful"; reason: string };

export interface MissingDataReportDto {
  policy: string;
  missingCount: number;
  rationale: string;
}

export interface AnalysisMetricDto {
  id: string;
  outputId: string;
  metricDefinitionVersionId: string;
  methodVersionId: string;
  estimand: string;
  sampleSize: number;
  effectEstimate: string;
  unit: string;
  uncertainty: UncertaintyReportDto;
  assumptions: string[];
  exclusions: string[];
  missingData: MissingDataReportDto;
  multiplicityTreatment: string;
  pValue: string | null;
}

export interface AnalysisRunLogDto {
  id: string;
  severity: AnalysisLogSeverity;
  message: string;
  details: unknown;
  recordedBy: string | null;
  recordedAtUs: number;
}

export interface AnalysisErrorDto {
  id: string;
  errorKind: string;
  message: string;
  details: unknown;
  recordedAtUs: number;
}

export interface AnalysisDeviationDto {
  id: string;
  description: string;
  rationale: string;
  impactAssessment: string;
  recordedBy: string | null;
  recordedAtUs: number;
}

export interface AnalysisRunDto {
  id: string;
  projectId: string;
  definitionVersionId: string;
  lifecycleStatus: AnalysisRunStatus;
  parameters: Record<string, unknown>;
  environment: Record<string, unknown>;
  randomSeeds: string[];
  protocolVersion: string;
  createdBy: string | null;
  createdAtUs: number;
  startedAtUs: number | null;
  endedAtUs: number | null;
  completionSummary: string | null;
  failureSummary: string | null;
  inputs: AnalysisInputDto[];
  integrityDeclarations: AnalysisIntegrityDeclarationDto[];
  integrityFindings: AnalysisIntegrityFindingDto[];
  outputs: AnalysisOutputDto[];
  metrics: AnalysisMetricDto[];
  logs: AnalysisRunLogDto[];
  errors: AnalysisErrorDto[];
  deviations: AnalysisDeviationDto[];
}

export type PropositionRefDto =
  | { kind: "source_claim"; id: string }
  | { kind: "hypothesis_version"; id: string }
  | { kind: "finding_version"; id: string };

export type EvidenceArtifactRefDto =
  | { kind: "analysis_output"; id: string }
  | { kind: "experiment_run"; id: string }
  | { kind: "dataset_version"; id: string }
  | { kind: "research_observation"; id: string }
  | { kind: "source_version_segment"; id: string }
  | { kind: "source_claim"; id: string };

export interface EvidenceEventDto {
  id: string;
  status: EvidenceStatus;
  rationale: string;
  actorId: string | null;
  occurredAtUs: number;
}

export interface EvidenceDto {
  id: string;
  projectId: string;
  proposition: PropositionRefDto;
  artifact: EvidenceArtifactRefDto;
  polarity: EvidencePolarity;
  scope: string;
  qualityRationale: string;
  limitations: string;
  provenance: Record<string, unknown>;
  createdBy: string | null;
  createdAtUs: number;
  status: EvidenceStatus;
  events: EvidenceEventDto[];
}

export interface FindingEvidenceLinkDto {
  evidenceId: string;
  synthesisPolarity: EvidencePolarity;
  rationale: string;
}

export interface FindingEventDto {
  id: string;
  status: FindingStatus;
  rationale: string;
  actorId: string | null;
  occurredAtUs: number;
}

export interface FindingRelationDto {
  id: string;
  targetVersionId: string;
  relationKind: FindingRelationKind;
  rationale: string;
  createdBy: string | null;
  createdAtUs: number;
}

export interface FindingVersionDto {
  id: string;
  findingId: string;
  projectId: string;
  versionNo: number;
  parentVersionId: string | null;
  changeRationale: string | null;
  title: string;
  statement: string;
  proposition: PropositionRefDto;
  scope: string;
  applicability: string;
  uncertainty: string;
  limitations: string;
  conclusionKind: FindingConclusionKind;
  analysisRunIds: string[];
  evidence: FindingEvidenceLinkDto[];
  status: FindingStatus;
  events: FindingEventDto[];
  relations: FindingRelationDto[];
  contentSha256: string;
  createdBy: string | null;
  createdAtUs: number;
}

export interface AnalysisWorkspaceDto {
  methods: AnalysisMethodVersionDto[];
  metrics: MetricDefinitionVersionDto[];
  definitions: AnalysisDefinitionVersionDto[];
  runs: AnalysisRunDto[];
  evidence: EvidenceDto[];
  findings: FindingVersionDto[];
}

export interface CreateEvidenceReq {
  projectId: string;
  proposition: PropositionRefDto;
  artifact: EvidenceArtifactRefDto;
  polarity: EvidencePolarity;
  scope: string;
  qualityRationale: string;
  limitations: string;
  provenance: Record<string, unknown>;
  createdBy: string | null;
}

export interface TransitionEvidenceReq {
  evidenceId: string;
  nextStatus: EvidenceStatus;
  rationale: string;
  actorId: string | null;
}

export interface TransitionFindingReq {
  versionId: string;
  nextStatus: FindingStatus;
  rationale: string;
  actorId: string | null;
}

export interface FindingEvidenceLinkReq {
  evidenceId: string;
  synthesisPolarity: EvidencePolarity;
  rationale: string;
}

export interface CreateFindingVersionReq {
  projectId: string;
  findingId: string | null;
  parentVersionId: string | null;
  title: string;
  statement: string;
  proposition: PropositionRefDto;
  scope: string;
  applicability: string;
  uncertainty: string;
  limitations: string;
  conclusionKind: FindingConclusionKind;
  analysisRunIds: string[];
  evidence: FindingEvidenceLinkReq[];
  changeRationale: string;
  createdBy: string | null;
}

export interface CreateFindingRelationReq {
  sourceVersionId: string;
  targetVersionId: string;
  relationKind: FindingRelationKind;
  rationale: string;
  createdBy: string | null;
}

export type ModelChangeClass = "refinement" | "extension" | "revision" | "replacement";
export type ModelRelationKind = "contributes_to" | "contradicts" | "qualifies" | "tested_in";
export type ModelContributionKind =
  | ModelRelationKind
  | "dataset_use"
  | "evidenced_by";
export type ModelEvidencePolarity = "supports" | "contradicts" | "qualifies" | "neutral";
export type ModelArtifactRef =
  | { kind: "analysis_output"; id: string }
  | { kind: "experiment_run"; id: string }
  | { kind: "dataset_version"; id: string }
  | { kind: "research_observation"; id: string }
  | { kind: "source_version_segment"; id: string }
  | { kind: "source_claim"; id: string }
  | { kind: "finding_version"; id: string };
export type ModelValidationTarget =
  | { kind: "model_version"; id: string }
  | { kind: "finding_version"; id: string }
  | { kind: "source_claim"; id: string };
export type ModelValidationOutcome = "pass" | "fail" | "inconclusive";
export type ModelStatus = "proposed" | "active" | "deprecated" | "replaced" | "retired";
export type ModelVersionLifecycleStatus = "draft" | "review" | "released" | "superseded" | "withdrawn";

export interface ModelLifecycleEventDto {
  status: ModelStatus | ModelVersionLifecycleStatus;
  rationale: string;
  actorId: string | null;
  occurredAtUs: number;
}

export interface TransitionModelReq {
  modelId: string;
  nextStatus: ModelStatus;
  rationale: string;
  actorId: string | null;
}

export interface TransitionModelVersionReq {
  versionId: string;
  nextStatus: ModelVersionLifecycleStatus;
  rationale: string;
  actorId: string | null;
}

export interface CreateModelReq {
  displayName: string;
  description: string;
  originProjectId: string | null;
  domainId: string | null;
  createdBy: string | null;
}

export interface ModelDto {
  id: string;
  displayName: string;
  description: string;
  originProjectId: string | null;
  domainId: string | null;
  status: ModelStatus;
  events: ModelLifecycleEventDto[];
  createdBy: string | null;
  createdAtUs: number;
}

export interface ModelSpecificationDto {
  humanReadable: string;
  machineReadable: Record<string, unknown>;
}

export interface ModelParameterReq {
  name: string;
  value: string;
  valueKind: "decimal" | "integer" | "boolean" | "string" | "json";
  unit: string | null;
  lowerBound: string | null;
  upperBound: string | null;
  defaultValue: string | null;
  role: string;
  tuneable: boolean;
}

export interface ModelDependencyReq {
  name: string;
  version: string;
  sourceUri: string | null;
  checksumSha256: string | null;
}

export interface ModelArtifactReq {
  artifactKind: string;
  title: string;
  artifactUri: string;
  checksumSha256: string;
  mediaType: string | null;
  byteLength: number | null;
}

export type ModelImpactKind = "added" | "changed" | "removed" | "affected";
export type ModelImpactTarget =
  | { kind: "source_claim"; claimId: string }
  | { kind: "hypothesis_version"; hypothesisVersionId: string }
  | { kind: "finding_version"; findingVersionId: string }
  | { kind: "assumption"; assumptionKey: string };

export interface ModelImpactReq {
  target: ModelImpactTarget;
  impactKind: ModelImpactKind;
  previousStatement: string | null;
  currentStatement: string | null;
  rationale: string;
}

export interface ModelAssumptionReq {
  key: string;
  statement: string;
}

export interface ModelParentReq {
  versionId: string;
  rationale: string;
}

export interface ModelRelationReq {
  targetId: string;
  relationKind: ModelRelationKind;
  rationale: string;
}

export interface ModelDatasetUseReq {
  datasetVersionId: string;
  role: DatasetRole;
  splitGroupId: string | null;
  rationale: string;
}

export interface ModelExperimentRelationReq {
  designVersionId: string;
  experimentRunId: string | null;
  relationKind: "contributes_to" | "tested_in";
  rationale: string;
}

export type ModelContributionTarget =
  | { kind: "hypothesis_version"; id: string }
  | { kind: "finding_version"; id: string }
  | { kind: "experiment"; designVersionId: string; experimentRunId: string | null }
  | { kind: "dataset"; datasetVersionId: string; role: DatasetRole; splitGroupId: string | null }
  | { kind: "evidence"; id: string };

export interface ModelContributionDto {
  id: string;
  relationKind: ModelContributionKind;
  target: ModelContributionTarget;
  rationale: string;
  createdBy: string | null;
  createdAtUs: number;
}

export interface CreateModelVersionReq {
  projectId: string;
  modelId: string;
  changeClass: ModelChangeClass;
  changeRationale: string;
  expectedImplications: string;
  specification: ModelSpecificationDto;
  assumptions: ModelAssumptionReq[];
  scope: string;
  dependencies: ModelDependencyReq[];
  environment: Record<string, unknown>;
  parents: ModelParentReq[];
  parameters: ModelParameterReq[];
  artifacts: ModelArtifactReq[];
  impacts: ModelImpactReq[];
  hypotheses: ModelRelationReq[];
  findings: ModelRelationReq[];
  experiments: ModelExperimentRelationReq[];
  datasetUses: ModelDatasetUseReq[];
  evidenceIds: string[];
  createdBy: string | null;
}

export interface ModelParameterDto extends ModelParameterReq {}
export interface ModelArtifactDto extends ModelArtifactReq {
  id: string;
}

export interface ModelVersionDto {
  id: string;
  modelId: string;
  versionNo: number;
  createdInProjectId: string;
  changeClass: ModelChangeClass;
  changeRationale: string;
  expectedImplications: string;
  specification: ModelSpecificationDto;
  assumptions: ModelAssumptionReq[];
  scope: string;
  dependencies: ModelDependencyReq[];
  environment: Record<string, unknown>;
  parents: ModelParentReq[];
  parameters: ModelParameterDto[];
  artifacts: ModelArtifactDto[];
  impacts: ModelImpactReq[];
  contributions: ModelContributionDto[];
  hypotheses: ModelRelationReq[];
  findings: ModelRelationReq[];
  experiments: ModelExperimentRelationReq[];
  datasetUses: ModelDatasetUseReq[];
  evidenceIds: string[];
  contentSha256: string;
  createdBy: string | null;
  createdAtUs: number;
  lifecycleStatus: ModelVersionLifecycleStatus;
  lifecycleEvents: ModelLifecycleEventDto[];
}

export interface CreateModelEvidenceReq {
  projectId: string;
  modelVersionId: string;
  artifact: ModelArtifactRef;
  polarity: ModelEvidencePolarity;
  scope: string;
  qualityRationale: string;
  limitations: string;
  provenance: Record<string, unknown>;
  createdBy: string | null;
}

export interface ModelEvidenceDto extends CreateModelEvidenceReq {
  id: string;
  createdAtUs: number;
}

export interface CreateModelEvaluationReq {
  projectId: string;
  modelVersionId: string;
  analysisRunId: string;
  analysisRunInputId: string;
  datasetVersionId: string;
  analysisMetricId: string;
  role: DatasetRole;
  splitGroupId: string | null;
  limitations: string;
  createdBy: string | null;
}

export interface ModelEvaluationDto extends CreateModelEvaluationReq {
  id: string;
  metricDefinitionVersionId: string;
  estimand: string;
  sampleSize: number;
  effectEstimate: string;
  unit: string;
  uncertaintyKind: "interval" | "not_meaningful";
  uncertaintyLower: string | null;
  uncertaintyUpper: string | null;
  uncertaintyLevel: string | null;
  uncertaintyReason: string | null;
  evaluatedAtUs: number;
}

export interface CreateModelValidationReq {
  projectId: string;
  target: ModelValidationTarget;
  criteria: Array<string | { criterion: string }>;
  scope: string;
  populationScope: string;
  evaluationMethod: string;
  evaluatorIdentity: string;
  independenceState: "independent" | "not_independent" | "unknown";
  independenceRationale: string;
  conflictOfInterest: string;
  createdBy: string | null;
}

export interface FinalizeModelValidationReq {
  validationId: string;
  outcome: ModelValidationOutcome;
  rationale: string;
  deviations: string[];
  analysisEvidenceIds: string[];
  modelEvidenceIds: string[];
  actorId: string | null;
}

export interface ModelValidationDto extends CreateModelValidationReq {
  id: string;
  outcome: ModelValidationOutcome | null;
  resultRationale: string | null;
  deviations: string[];
  analysisEvidenceIds: string[];
  modelEvidenceIds: string[];
  createdAtUs: number;
  finalizedAtUs: number | null;
}

export interface ModelReplicationDatasetReq {
  datasetVersionId: string;
  role: DatasetRole;
  splitGroupId: string | null;
}

export interface CreateModelReplicationReq {
  projectId: string;
  modelVersionId: string;
  originalExperimentRunId: string;
  replicationKind: "direct" | "conceptual";
  independentTeamOperator: string;
  protocolContext: string;
  datasetContext: Record<string, unknown>;
  datasets: ModelReplicationDatasetReq[];
  outcome: "supports" | "contradicts" | "mixed" | "inconclusive";
  resultSummary: string;
  deviations: string[];
  provenance: Record<string, unknown>;
  analysisEvidenceIds: string[];
  modelEvidenceIds: string[];
  createdBy: string | null;
}

export interface ModelReplicationDto extends CreateModelReplicationReq {
  id: string;
  originalDesignVersionId: string;
  createdAtUs: number;
}

export interface ModelWorkspaceDto {
  models: ModelDto[];
  versions: ModelVersionDto[];
  evidence: ModelEvidenceDto[];
  evaluations: ModelEvaluationDto[];
  validations: ModelValidationDto[];
  replications: ModelReplicationDto[];
}

export type LearningVersionStatus = "draft" | "review" | "published" | "deprecated";
export type LearningReviewStatus = "approved" | "changes_requested" | "rejected";
export type LearningAssessmentOutcome = "pass" | "fail" | "inconclusive";
export type LearningMasteryOutcome = "mastered" | "not_mastered";

export interface LearningLearnerDto {
  id: string;
  displayName: string;
  createdBy: string | null;
  createdAtUs: number;
}

export interface LearningProjectDto {
  id: string;
  title: string;
  learnerId: string;
  learnerName: string;
  frameworkId: string;
  frameworkVersionId: string;
  frameworkTitle: string;
  frameworkVersionNo: number;
  createdBy: string | null;
  createdAtUs: number;
}

export interface LearningFrameworkVersionDto {
  frameworkId: string;
  frameworkVersionId: string;
  title: string;
  versionNo: number;
  publishedAtUs: number;
}

export interface LearningObjectiveVersionDto {
  id: string;
  objectiveId: string;
  versionNo: number;
  learningProjectId: string;
  frameworkId: string;
  frameworkVersionId: string;
  parentVersionId: string | null;
  title: string;
  observableOutcome: string;
  criteriaText: string;
  level: string;
  prerequisiteVersionIds: string[];
  createdBy: string | null;
  createdAtUs: number;
}

export interface AssessmentQuestionDto {
  id: string;
  objectiveVersionId: string;
  questionOrder: number;
  prompt: string;
  responseKind: string;
  maxPoints: number;
  required: boolean;
}

export interface AssessmentRubricCriterionDto {
  id: string;
  objectiveVersionId: string;
  criterionKey: string;
  description: string;
  maxPoints: number;
}

export interface AssessmentReviewDto {
  id: string;
  reviewStatus: LearningReviewStatus;
  reviewerId: string | null;
  rationale: string;
  reviewedAtUs: number;
}

export interface AssessmentVersionDto {
  id: string;
  assessmentId: string;
  title: string;
  versionNo: number;
  learningProjectId: string;
  frameworkId: string;
  frameworkVersionId: string;
  rubricVersionId: string;
  rubricPassThreshold: number;
  status: LearningVersionStatus;
  changeRationale: string;
  createdBy: string | null;
  createdAtUs: number;
  publishedAtUs: number | null;
  questions: AssessmentQuestionDto[];
  rubricCriteria: AssessmentRubricCriterionDto[];
  objectiveVersionIds: string[];
  reviews: AssessmentReviewDto[];
}

export interface AttemptResponseDto {
  id: string;
  assessmentQuestionId: string;
  response: unknown;
  updatedAtUs: number;
}

export interface EvaluationCriterionScoreDto {
  rubricCriterionId: string;
  score: number;
}

export interface AttemptEvaluationDto {
  id: string;
  attemptId: string;
  rubricVersionId: string;
  revisionNo: number;
  correctionOfId: string | null;
  evaluatorId: string;
  outcome: LearningAssessmentOutcome;
  score: number;
  rationale: string;
  criterionScores: EvaluationCriterionScoreDto[];
  feedback: string[];
  createdAtUs: number;
}

export interface AssessmentAttemptDto {
  id: string;
  learningProjectId: string;
  learnerId: string;
  assessmentVersionId: string;
  frameworkVersionId: string;
  status: string;
  startedAtUs: number;
  submittedAtUs: number | null;
  responses: AttemptResponseDto[];
  evaluations: AttemptEvaluationDto[];
}

export interface CompetencyCriterionDto {
  id: string;
  objectiveVersionId: string;
  criterion: string;
  minimumScore: number;
  required: boolean;
}

export interface CompetencyDto {
  id: string;
  learningProjectId: string;
  frameworkId: string;
  frameworkVersionId: string;
  title: string;
  description: string;
  criteria: CompetencyCriterionDto[];
}

export interface MasteryRuleVersionDto {
  id: string;
  masteryRuleId: string;
  competencyId: string;
  versionNo: number;
  minimumAverageScore: number;
  minimumEvaluatedAttempts: number;
  allRequiredCriteria: boolean;
  reproductionRequired: boolean;
  requiredReproductionOutcome: LearningAssessmentOutcome;
  createdBy: string | null;
  createdAtUs: number;
}

export interface MasteryRuleDto {
  id: string;
  competencyId: string;
  title: string;
  versions: MasteryRuleVersionDto[];
}

export interface CompetencyRecordCriterionDto {
  competencyCriterionId: string;
  satisfied: boolean;
  rationale: string;
}

export interface CompetencyRecordDto {
  id: string;
  competencyId: string;
  learningProjectId: string;
  learnerId: string;
  frameworkVersionId: string;
  masteryRuleVersionId: string;
  outcome: LearningMasteryOutcome;
  explanation: string;
  attemptEvaluationIds: string[];
  reproductionEvaluationIds: string[];
  objectiveVersionIds: string[];
  criteria: CompetencyRecordCriterionDto[];
  createdAtUs: number;
}

export interface ReproductionTaskDto {
  id: string;
  learningProjectId: string;
  frameworkId: string;
  frameworkVersionId: string;
  objectiveVersionId: string;
  rubricVersionId: string;
  title: string;
  taskDescription: string;
  conditions: string;
  expectedResult: unknown;
  createdBy: string | null;
  createdAtUs: number;
}

export interface ReproductionSubmissionDto {
  id: string;
  taskId: string;
  learnerId: string;
  artifactUri: string | null;
  artifactSha256: string | null;
  result: unknown;
  deviations: string;
  submittedAtUs: number;
  evaluations: ReproductionEvaluationDto[];
}

export interface ReproductionEvaluationDto {
  id: string;
  submissionId: string;
  rubricVersionId: string;
  evaluatorId: string;
  outcome: LearningAssessmentOutcome;
  score: number;
  rationale: string;
  evidenceDescription: string;
  criterionScores: EvaluationCriterionScoreDto[];
  createdAtUs: number;
}

export interface LearnerProgressDto {
  learnerId: string;
  learningProjectId: string;
  objectiveVersionId: string;
  status: string;
  explanation: string;
  latestEvaluationId: string | null;
  competencyRecordId: string | null;
}

export interface LearningWorkspaceDto {
  learners: LearningLearnerDto[];
  projects: LearningProjectDto[];
  frameworkVersions: LearningFrameworkVersionDto[];
  objectives: LearningObjectiveVersionDto[];
  assessments: AssessmentVersionDto[];
  attempts: AssessmentAttemptDto[];
  competencies: CompetencyDto[];
  masteryRules: MasteryRuleDto[];
  competencyRecords: CompetencyRecordDto[];
  reproductionTasks: ReproductionTaskDto[];
  reproductionSubmissions: ReproductionSubmissionDto[];
  progress: LearnerProgressDto[];
}

export interface CreateLearnerReq {
  displayName: string;
  createdBy: string | null;
}

export interface CreateLearningProjectReq {
  title: string;
  learnerId: string;
  frameworkId: string;
  frameworkVersionId: string;
  createdBy: string | null;
}

export interface CreateLearningObjectiveReq {
  objectiveId: string | null;
  learningProjectId: string;
  title: string;
  observableOutcome: string;
  criteriaText: string;
  level: string;
  parentVersionId: string | null;
  prerequisiteVersionIds: string[];
  createdBy: string | null;
}

export interface CreateAssessmentQuestionReq {
  objectiveVersionId: string;
  questionOrder: number;
  prompt: string;
  responseKind: string;
  maxPoints: number;
  required: boolean;
}

export interface CreateRubricCriterionReq {
  objectiveVersionId: string;
  criterionKey: string;
  description: string;
  maxPoints: number;
}

export interface CreateAssessmentVersionReq {
  assessmentId: string | null;
  learningProjectId: string;
  title: string;
  changeRationale: string;
  rubricTitle: string;
  rubricPassThreshold: number;
  questions: CreateAssessmentQuestionReq[];
  rubricCriteria: CreateRubricCriterionReq[];
  createdBy: string | null;
}

export interface ReviewAssessmentReq {
  assessmentVersionId: string;
  reviewStatus: LearningReviewStatus;
  reviewerId: string | null;
  rationale: string;
}

export interface TransitionAssessmentVersionReq {
  assessmentVersionId: string;
  nextStatus: LearningVersionStatus;
  actorId: string | null;
}

export interface CreateAssessmentAttemptReq {
  learningProjectId: string;
  learnerId: string;
  assessmentVersionId: string;
}

export interface SaveAttemptResponseReq {
  attemptId: string;
  assessmentQuestionId: string;
  response: unknown;
}

export interface EvaluateAssessmentAttemptReq {
  attemptId: string;
  correctionOfId: string | null;
  evaluatorId: string;
  outcome: LearningAssessmentOutcome;
  criterionScores: EvaluationCriterionScoreDto[];
  rationale: string;
  feedback: string;
}

export interface CreateCompetencyCriterionReq {
  objectiveVersionId: string;
  criterion: string;
  minimumScore: number;
  required: boolean;
}

export interface CreateCompetencyReq {
  learningProjectId: string;
  title: string;
  description: string;
  criteria: CreateCompetencyCriterionReq[];
  createdBy: string | null;
}

export interface CreateMasteryRuleVersionReq {
  masteryRuleId: string | null;
  competencyId: string;
  title: string;
  minimumAverageScore: number;
  minimumEvaluatedAttempts: number;
  allRequiredCriteria: boolean;
  reproductionRequired: boolean;
  requiredReproductionOutcome: LearningAssessmentOutcome;
  createdBy: string | null;
}

export interface EvaluateCompetencyReq {
  competencyId: string;
  learnerId: string;
  masteryRuleVersionId: string;
}

export interface CreateReproductionTaskReq {
  learningProjectId: string;
  objectiveVersionId: string;
  rubricVersionId: string;
  title: string;
  taskDescription: string;
  conditions: string;
  expectedResult: unknown;
  createdBy: string | null;
}

export interface SubmitReproductionReq {
  taskId: string;
  learnerId: string;
  artifactUri: string | null;
  artifactSha256: string | null;
  result: unknown;
  deviations: string;
}

export interface EvaluateReproductionReq {
  submissionId: string;
  evaluatorId: string;
  outcome: LearningAssessmentOutcome;
  criterionScores: EvaluationCriterionScoreDto[];
  rationale: string;
  evidenceDescription: string;
}

export type TransferArtifactKind =
  | "finding_version"
  | "concept_version"
  | "principle_version"
  | "framework_version"
  | "model_version";

export type TransferArtifactRef =
  | { kind: "finding_version"; findingId: string; versionId: string }
  | { kind: "concept_version"; conceptId: string; versionId: string }
  | { kind: "principle_version"; principleId: string; versionId: string }
  | { kind: "framework_version"; frameworkId: string; versionId: string }
  | { kind: "model_version"; modelId: string; versionId: string };

export type TransferPurpose = "research" | "learning" | "commercial" | "public_distribution";
export type TransferAccessScope = "workspace" | "selected_projects";
export type TransferEligibilityState = "eligible" | "restricted" | "withdrawn";
export type KnowledgeTransferState = "accepted" | "rejected" | "withdrawn";
export type KnowledgeVersionKind = "concept" | "principle";
export type KnowledgeVersionStatus = "draft" | "reviewed" | "deprecated";
export type TransferLineageRelation = "adopted" | "derived_from" | "based_on";

export interface ResearchTransferProjectDto {
  id: string;
  title: string;
  lifecycleStatus: string;
}

export interface TransferArtifactCatalogDto {
  artifact: TransferArtifactRef;
  title: string;
  versionNo: number;
  ownerProjectId: string | null;
  lifecycleStatus: string;
  originId: string | null;
  eligibilityIds: string[];
}

export interface KnowledgeVersionDto {
  kind: KnowledgeVersionKind;
  entityId: string;
  versionId: string;
  versionNo: number;
  projectId: string;
  title: string;
  content: string;
  rationale: string;
  status: string;
  parentVersionId: string | null;
}

export interface KnowledgeTransferOriginDto {
  id: string;
  artifact: TransferArtifactRef;
  originProjectId: string;
  originProjectTitle: string;
  rationale: string;
  recordedBy: string | null;
  recordedAtUs: number;
}

export interface TransferEligibilityDto {
  id: string;
  origin: KnowledgeTransferOriginDto;
  licenseIdentifier: string;
  licenseUri: string | null;
  accessScope: TransferAccessScope;
  permittedPurposes: TransferPurpose[];
  allowedProjectIds: string[];
  attributionText: string;
  permissionBasis: string;
  reviewedBy: string;
  reviewedAtUs: number;
  state: TransferEligibilityState;
  stateRationale: string;
}

export interface KnowledgeTransferEventDto {
  id: string;
  state: KnowledgeTransferState;
  rationale: string;
  actorId: string | null;
  occurredAtUs: number;
}

export interface KnowledgeTransferDto {
  id: string;
  eligibilityId: string;
  origin: KnowledgeTransferOriginDto;
  receivingProjectId: string;
  receivingProjectTitle: string;
  intendedPurpose: TransferPurpose;
  attributionText: string;
  context: string;
  requestedBy: string | null;
  requestedAtUs: number;
  state: KnowledgeTransferState;
  events: KnowledgeTransferEventDto[];
}

export interface TransferAdaptationDto {
  id: string;
  transferId: string;
  origin: KnowledgeTransferOriginDto;
  receivingProjectId: string;
  artifact: TransferArtifactRef;
  title: string;
  rationale: string;
  adaptedBy: string | null;
  createdAtUs: number;
}

export type DownstreamUseRef =
  | { kind: "finding_version"; findingId: string; versionId: string }
  | { kind: "concept_version"; conceptId: string; versionId: string }
  | { kind: "principle_version"; principleId: string; versionId: string }
  | { kind: "framework_version"; frameworkId: string; versionId: string }
  | { kind: "model_version"; modelId: string; versionId: string }
  | { kind: "model_evaluation"; id: string }
  | { kind: "analysis_run"; id: string }
  | { kind: "experiment_run"; id: string };

export interface DownstreamUsageDto {
  id: string;
  transferId: string;
  projectId: string;
  reference: DownstreamUseRef;
  useContext: string;
  recordedBy: string | null;
  recordedAtUs: number;
}

export interface ResearchLineageDto {
  id: string;
  transferId: string;
  origin: TransferArtifactRef;
  originProjectId: string;
  receivingProjectId: string;
  relationKind: TransferLineageRelation;
  adaptation: TransferArtifactRef | null;
  rationale: string;
  occurredAtUs: number;
}

export interface TransferProvenanceRecordDto {
  recordKind: string;
  recordId: string;
  label: string;
  relationship: string;
  details: string;
}

export interface TransferProvenancePreviewDto {
  origin: KnowledgeTransferOriginDto;
  title: string;
  versionNo: number;
  lifecycleStatus: string;
  scope: string;
  applicability: string;
  uncertainty: string;
  limitations: string;
  validationScope: string[];
  provenance: TransferProvenanceRecordDto[];
}

export interface TransferWorkspaceDto {
  projects: ResearchTransferProjectDto[];
  artifacts: TransferArtifactCatalogDto[];
  knowledgeVersions: KnowledgeVersionDto[];
  origins: KnowledgeTransferOriginDto[];
  eligibilities: TransferEligibilityDto[];
  transfers: KnowledgeTransferDto[];
  adaptations: TransferAdaptationDto[];
  downstreamUsage: DownstreamUsageDto[];
  lineage: ResearchLineageDto[];
}

export interface CreateConceptVersionReq {
  conceptId: string | null;
  projectId: string;
  parentVersionId: string | null;
  title: string;
  definition: string;
  rationale: string;
  createdBy: string | null;
}

export interface CreatePrincipleVersionReq {
  principleId: string | null;
  projectId: string;
  parentVersionId: string | null;
  title: string;
  statement: string;
  rationale: string;
  createdBy: string | null;
}

export interface ReviewKnowledgeVersionReq {
  versionId: string;
  kind: KnowledgeVersionKind;
  status: KnowledgeVersionStatus;
  rationale: string;
  actorId: string | null;
}

export interface CreateTransferOriginReq {
  artifact: TransferArtifactRef;
  originProjectId: string;
  rationale: string;
  recordedBy: string | null;
}

export interface CreateTransferEligibilityReq {
  originId: string;
  licenseIdentifier: string;
  licenseUri: string | null;
  accessScope: TransferAccessScope;
  permittedPurposes: TransferPurpose[];
  allowedProjectIds: string[];
  attributionText: string;
  permissionBasis: string;
  permissionGranted: boolean;
  reviewedBy: string;
}

export interface ChangeTransferEligibilityReq {
  eligibilityId: string;
  state: TransferEligibilityState;
  rationale: string;
  actorId: string | null;
}

export interface AdoptKnowledgeReq {
  eligibilityId: string;
  receivingProjectId: string;
  intendedPurpose: TransferPurpose;
  attributionText: string;
  context: string;
  requestedBy: string | null;
}

export interface CreateTransferAdaptationReq {
  transferId: string;
  artifact: TransferArtifactRef;
  relationKind: TransferLineageRelation;
  rationale: string;
  adaptedBy: string | null;
}

export interface RecordDownstreamUsageReq {
  transferId: string;
  reference: DownstreamUseRef;
  useContext: string;
  recordedBy: string | null;
}

