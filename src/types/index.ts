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

export type HypothesisVerdict = "supported" | "rejected" | "inconclusive";

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
}

export interface HypothesisPromotionInput {
  inquiryId: string;
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
}

export interface CustomFontSetting {
  id: string;
  name: string;
  dataUrl: string;
}
