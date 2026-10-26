import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { TraceGitLogEntry, TraceGraph, TraceNodeData, TraceProjectInfo } from "@/types";

export async function loadGraph(): Promise<TraceGraph> {
  return await invoke<TraceGraph>("trace_load_graph");
}

export async function saveGraph(graph: TraceGraph): Promise<void> {
  await invoke("trace_save_graph", { graph });
}

export async function saveDecisionMd(node: TraceNodeData): Promise<string> {
  return await invoke<string>("trace_save_decision_md", { node });
}

export async function listDecisions(): Promise<string[]> {
  return await invoke<string[]>("trace_list_decisions");
}

export async function scanEcosystem(): Promise<TraceProjectInfo[]> {
  return await invoke<TraceProjectInfo[]>("trace_scan_ecosystem");
}

export async function gitLogForProject(
  projectPath: string,
  limit = 10,
): Promise<TraceGitLogEntry[]> {
  return await invoke<TraceGitLogEntry[]>("trace_git_log_for_project", {
    projectPath,
    limit,
  });
}

export async function readAllDecisions(): Promise<TraceNodeData[]> {
  return await invoke<TraceNodeData[]>("trace_read_all_decisions");
}

export async function findCommitsForFile(
  projectPath: string,
  filePattern: string,
  limit = 10,
): Promise<TraceGitLogEntry[]> {
  return await invoke<TraceGitLogEntry[]>("trace_find_commits_for_file", {
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
  return await invoke<string>("trace_export_graph_json", { destPath });
}

export async function importGraphJson(srcPath: string): Promise<TraceGraph> {
  return await invoke<TraceGraph>("trace_import_graph_json", { srcPath });
}

export async function appVersion(): Promise<string> {
  return await invoke<string>("trace_app_version");
}

export async function recentDecisions(limit = 10): Promise<TraceNodeData[]> {
  return await invoke<TraceNodeData[]>("trace_recent_decisions", { limit });
}

export async function chooseGraphExportPath(): Promise<string | null> {
  const path = await save({
    title: "Export Trace Graph",
    defaultPath: "vora-trace-graph.json",
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  return path;
}

export async function chooseGraphImportPath(): Promise<string | null> {
  const path = await open({
    title: "Import Trace Graph",
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  return typeof path === "string" ? path : null;
}
