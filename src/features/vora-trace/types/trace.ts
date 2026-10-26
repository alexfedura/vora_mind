export type NodeKind = "state" | "decision" | "experiment" | "rejected";

export type NodeStatus =
  | "accepted"
  | "rejected"
  | "pending"
  | "current"
  | "abandoned";

export interface TraceNodeData {
  id: string;
  kind: NodeKind;
  title: string;
  status: NodeStatus;
  whyAmIHere?: string;
  objective?: string;
  reason?: string;
  createdAt?: string;
  parentId?: string | null;
  gitCommit?: string;
  linkedCommits?: string[];
  alternatives?: { title: string; status: NodeStatus; reason?: string }[];
  evidence?: string[];
  next?: string[];
}

export interface TraceEdgeData {
  id: string;
  source: string;
  target: string;
  kind: "main" | "branch" | "rejected";
  label?: string;
}

export interface TraceGraph {
  schemaVersion: string;
  project: string;
  nodes: TraceNodeData[];
  edges: TraceEdgeData[];
}
