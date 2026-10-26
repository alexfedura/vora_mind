import { invoke } from "@tauri-apps/api/core";
import type { TraceGraph, TraceNodeData } from "../types/trace";

export interface ProjectInfo {
  id: string;
  path: string;
  role: string;
  status: string;
  description?: string;
  exists: boolean;
  git_branch?: string;
  git_head?: string;
  git_dirty?: boolean;
}

export interface GitLogEntry {
  hash: string;
  short: string;
  date: string;
  message: string;
}

export async function loadGraph(): Promise<TraceGraph> {
  return await invoke<TraceGraph>("load_graph");
}

export async function saveGraph(graph: TraceGraph): Promise<void> {
  await invoke("save_graph", { graph });
}

export async function saveDecisionMd(node: TraceNodeData): Promise<string> {
  return await invoke<string>("save_decision_md", { node });
}

export async function listDecisions(): Promise<string[]> {
  return await invoke<string[]>("list_decisions");
}

export async function scanEcosystem(): Promise<ProjectInfo[]> {
  return await invoke<ProjectInfo[]>("scan_ecosystem");
}

export async function gitLogForProject(
  projectPath: string,
  limit = 10
): Promise<GitLogEntry[]> {
  return await invoke<GitLogEntry[]>("git_log_for_project", {
    projectPath,
    limit,
  });
}

export async function readAllDecisions(): Promise<TraceNodeData[]> {
  return await invoke<TraceNodeData[]>("read_all_decisions");
}

export async function findCommitsForFile(
  projectPath: string,
  filePattern: string,
  limit = 10
): Promise<GitLogEntry[]> {
  return await invoke<GitLogEntry[]>("find_commits_for_file", {
    projectPath,
    filePattern,
    limit,
  });
}

export function isTauri(): boolean {
  return typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);
}

export async function exportGraphJson(destPath: string): Promise<string> {
  return await invoke<string>("export_graph_json", { destPath });
}

export async function importGraphJson(srcPath: string): Promise<TraceGraph> {
  return await invoke<TraceGraph>("import_graph_json", { srcPath });
}

export async function appVersion(): Promise<string> {
  return await invoke<string>("app_version");
}

export async function recentDecisions(limit = 10): Promise<TraceNodeData[]> {
  return await invoke<TraceNodeData[]>("recent_decisions", { limit });
}
