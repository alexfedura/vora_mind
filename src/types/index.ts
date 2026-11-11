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
export type FrameworkReviewStatus = "approved" | "changes_requested" | "rejected";
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
  lifecycle: string;
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
  targetKind: string;
  targetId: string;
  eventKind: string;
  actorId: string | null;
  occurredAtUs: number;
  reason: string;
  beforeJson: string | null;
  afterJson: string | null;
}
