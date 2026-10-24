import { invoke } from "@tauri-apps/api/core";

export interface AstSymbol {
  id: string;
  name: string;
  kind: string;
  file_path: string;
  start_line: number;
  end_line: number;
  signature?: string;
  canonical_entity_id?: string;
}

export interface AstRelation {
  source_id: string;
  target_id: string;
  relation_type: string;
}

export interface AstGraphPayload {
  file_path?: string;
  symbols: AstSymbol[];
  relations: AstRelation[];
}

export interface ToolDefinition {
  name: string;
  description: string;
  parameters: Record<string, any>;
}

export interface ToolCallResponse {
  success: boolean;
  result?: any;
  data?: any;
  error?: string;
}

export interface GitCommitItem {
  hash: string;
  short_hash: string;
  message: string;
  author: string;
  timestamp: string;
}

export interface TestResult {
  success: boolean;
  output: string;
  exit_code?: number | null;
}

export interface CodeChurnMetric {
  file_path: string;
  commit_count?: number;
  commits_count?: number;
  lines_added: number;
  lines_deleted: number;
  churn_score: number;
}

export interface ImpactedTarget {
  symbol_id: string;
  file_path: string;
  symbol_name: string;
  relation_kind: string;
  cascade_depth: number;
  is_test: boolean;
}

export interface ImpactAnalysisReport {
  target_symbol_id: string;
  total_impacted_count: number;
  direct_dependents: ImpactedTarget[];
  indirect_dependents: ImpactedTarget[];
  affected_test_files: string[];
}

// --- قراردادهای معنایی Semantic Core Contract v1.0 & Canonical Entities ---

export type SemanticEdgeStatus =
  | { Resolved: { 0: string } | string }
  | { Ambiguous: Array<{ 0: string } | string> }
  | "Unresolved"
  | "DynamicFallback";

export interface ResolvedSemanticEdge {
  source_symbol: { 0: string } | string;
  target_symbol: { 0: string } | string | null;
  candidate_symbols?: Array<{ 0: string } | string>;
  relation: string;
  status: SemanticEdgeStatus;
  call_line: number;
}

export type SemanticChange =
  | {
      BodyChanged: {
        symbol_id: { 0: string } | string;
        lineage_id: { 0: string } | string;
        old_body_hash: string;
        new_body_hash: string;
      };
    }
  | {
      SignatureChanged: {
        old_symbol_id: { 0: string } | string;
        new_symbol_id: { 0: string } | string;
        lineage_id: { 0: string } | string;
        old_signature: string;
        new_signature: string;
      };
    }
  | {
      TypeDefinitionChanged: {
        old_symbol_id: { 0: string } | string;
        new_symbol_id: { 0: string } | string;
        lineage_id: { 0: string } | string;
      };
    }
  | {
      SymbolRenamed: {
        old_symbol_id: { 0: string } | string;
        new_symbol_id: { 0: string } | string;
        lineage_id: { 0: string } | string;
        old_name: string;
        new_name: string;
      };
    }
  | {
      SymbolAdded: {
        symbol_id: { 0: string } | string;
        lineage_id: { 0: string } | string;
        signature: string;
      };
    }
  | {
      SymbolRemoved: {
        symbol_id: { 0: string } | string;
        lineage_id: { 0: string } | string;
      };
    }
  | {
      VisibilityChanged: {
        symbol_id: { 0: string } | string;
        lineage_id: { 0: string } | string;
        old_vis: string;
        new_vis: string;
      };
    };

export interface CodeSlice {
  target_symbol: string;
  target_file: string;
  target_code: string;
  related_definitions: string[];
  global_skeleton: string;
}

export interface PlanStep {
  step_order: number;
  symbol_name: string;
  file_path: string;
  reason: string;
  cascade_depth?: number;
  code_slice?: CodeSlice | null;
}

export interface ExecutionPlan {
  root_symbol: string;
  total_steps: number;
  steps: PlanStep[];
}

export interface PatchResult {
  file_path: string;
  symbol_id: string;
  original_byte_range: [number, number];
  patched_byte_range: [number, number];
  is_syntax_valid: boolean;
  applied_to_disk: boolean;
}

export interface SymbolSearchResult {
  id: string;
  name: string;
  kind: string;
  file_path: string;
  signature: string;
  start_line: number;
  end_line: number;
}

export interface TransactionExecutionResult {
  is_committed: boolean;
  commit_message: string;
  patched_files: string[];
  changes_count: number;
  file_moves_count: number;
  snapshot_id?: string | null;
}

export interface TransactionPatchItem {
  file_path: string;
  symbol_id: string;
  new_symbol_code: string;
}

export interface LineageHistoryEntry {
  snapshot_id: string;
  symbol_id: string;
  confidence: number | null;
}

export interface CanonicalSymbolRecord {
  entity_id: { 0: string } | string;
  symbol_id: { 0: string } | string;
  lineage_id: { 0: string } | string;
  file_id: { 0: string } | string;
  module_namespace: string;
  name: string;
  kind: string;
  canonical_signature: string;
  scope_path: string;
  qualified_scope: string;
}

export interface AuditTrailRecord {
  id: number;
  task_id: string;
  workspace_id: string;
  intent: string;
  target_symbol: string;
  target_file: string;
  base_snapshot_id?: string | null;
  committed_snapshot_id?: string | null;
  healingAttempts: number;
  status: string;
  error_payload?: string | null;
  created_at: number;
}

/** هلپر نرمال‌سازی رشته‌های wrap شده در تاپل‌های ساختاری Rust */
export function normalizeId(idVal: { 0: string } | string | null | undefined): string {
  if (!idVal) return "";
  if (typeof idVal === "string") return idVal;
  if (typeof idVal === "object" && "0" in idVal) return idVal["0"];
  return String(idVal);
}

export const forgeService = {
  async parseCodeAst(filePath: string, content: string): Promise<AstGraphPayload> {
    return await invoke<AstGraphPayload>("parse_code_ast", {
      filePath,
      content,
    });
  },

  async readProjectFile(filePath: string): Promise<string> {
    return await invoke<string>("read_project_file", {
      filePath,
    });
  },

  async getForgeTools(): Promise<ToolDefinition[]> {
    return await invoke<ToolDefinition[]>("get_forge_tools");
  },

  async executeForgeTool(tool: string, argumentsObj: Record<string, any>): Promise<ToolCallResponse> {
    const res = await invoke<any>("execute_forge_tool", {
      tool,
      arguments: argumentsObj,
    });
    return {
      success: res.success,
      data: res.result !== undefined ? res.result : res.data,
      result: res.result !== undefined ? res.result : res.data,
      error: res.error,
    };
  },

  async applyPatchAndValidate(filePath: string, content: string): Promise<boolean> {
    return await invoke<boolean>("apply_patch_and_validate", {
      filePath,
      content,
    });
  },

  async createGitCommit(filePath: string, symbolName: string, reason: string): Promise<string> {
    return await invoke<string>("create_git_commit", {
      filePath,
      symbolName,
      reason,
    });
  },

  async getCurrentGitHead(): Promise<string> {
    const tools = await this.executeForgeTool("get_current_head", {});
    if (tools.success && typeof tools.data === "string") {
      return tools.data;
    }
    const history = await this.getGitCommitHistory(1);
    return history[0]?.hash || "HEAD";
  },

  async scanWorkspace(directoryPath: string): Promise<AstGraphPayload> {
    return await invoke<AstGraphPayload>("scan_workspace", {
      directoryPath,
    });
  },

  async getGitCommitHistory(maxCount = 20): Promise<GitCommitItem[]> {
    return await invoke<GitCommitItem[]>("get_git_commit_history", {
      maxCount,
    });
  },

  async runProjectTests(testCommand?: string): Promise<TestResult> {
    return await invoke<TestResult>("run_project_tests", {
      testCommand,
    });
  },

  async testCandidatePatch(
    filePath: string,
    symbolId: string,
    newSymbolCode: string,
    testCommand?: string
  ): Promise<TestResult> {
    return await invoke<TestResult>("forge_test_candidate_patch", {
      filePath,
      symbolId,
      newSymbolCode,
      testCommand,
    });
  },

  async testMultiCandidatePatches(
    patches: TransactionPatchItem[],
    testCommand?: string
  ): Promise<TestResult> {
    return await invoke<TestResult>("forge_test_multi_candidate_patches", {
      patches,
      testCommand,
    });
  },

  async rollbackToSnapshot(snapshotId: string): Promise<string[]> {
    return await invoke<string[]>("forge_rollback_to_snapshot", {
      snapshotId,
    });
  },

  async getCodeChurnMetrics(maxCommits = 40): Promise<CodeChurnMetric[]> {
    const commits = await this.getGitCommitHistory(maxCommits);
    const fileStats = new Map<string, { commits: number; added: number; deleted: number }>();

    commits.forEach((c) => {
      const msg = c.message || "";
      const match = msg.match(/\bin\s+([^\s:]+)/);
      const filePath = match ? match[1] : "src/lib.rs";

      const current = fileStats.get(filePath) || { commits: 0, added: 0, deleted: 0 };
      current.commits += 1;
      current.added += 15;
      current.deleted += 5;
      fileStats.set(filePath, current);
    });

    const metrics: CodeChurnMetric[] = [];
    fileStats.forEach((stat, file_path) => {
      const churn_score = Math.min(1.0, (stat.commits * 0.4 + (stat.added + stat.deleted) * 0.01) / 5.0);
      metrics.push({
        file_path,
        commit_count: stat.commits,
        commits_count: stat.commits,
        lines_added: stat.added,
        lines_deleted: stat.deleted,
        churn_score: Number(churn_score.toFixed(2)),
      });
    });

    return metrics;
  },

  // --- متدهای اختصاصی ارتباط با هسته معنایی (Semantic Core IPC Calls) ---

  async resolveStrictEdges(): Promise<ResolvedSemanticEdge[]> {
    return await invoke<ResolvedSemanticEdge[]>("resolve_strict_edges");
  },

  async simulateSemanticChange(change: SemanticChange): Promise<ImpactAnalysisReport> {
    return await invoke<ImpactAnalysisReport>("simulate_semantic_change", {
      change,
    });
  },

  async createSemanticPlan(repoRoot: string, change: SemanticChange): Promise<ExecutionPlan> {
    return await invoke<ExecutionPlan>("create_semantic_plan", {
      repoRoot,
      change,
    });
  },

  async computeAstSemanticDiff(
    filePath: string,
    targetSymbolName: string,
    oldCode: string,
    newCode: string
  ): Promise<SemanticChange> {
    return await invoke<SemanticChange>("forge_compute_ast_semantic_diff", {
      filePath,
      targetSymbolName,
      oldCode,
      newCode,
    });
  },

  async repairAndExpandPlan(
    repoRoot: string,
    currentPlan: ExecutionPlan,
    discoveredSymbolNames: string[]
  ): Promise<ExecutionPlan> {
    return await invoke<ExecutionPlan>("forge_repair_and_expand_plan", {
      repoRoot,
      currentPlan,
      discoveredSymbolNames,
    });
  },

  async rewireImports(
    filePath: string,
    oldSymbolName: string,
    newSymbolName: string,
    dryRun = false
  ): Promise<{ filePath: string; rewiredContent: string }> {
    const res = await this.executeForgeTool("rewire_imports", {
      file_path: filePath,
      old_symbol_name: oldSymbolName,
      new_symbol_name: newSymbolName,
      dry_run: dryRun,
    });
    if (!res.success) {
      throw new Error(res.error || `Failed to rewire imports in '${filePath}'`);
    }
    return {
      filePath: res.data?.file_path || filePath,
      rewiredContent: res.data?.rewired_content || "",
    };
  },

  async applyPatchToOverlaySandbox(
    filePath: string,
    symbolId: string,
    newSymbolCode: string
  ): Promise<PatchResult> {
    const res = await this.executeForgeTool("apply_surgical_patch", {
      file_path: filePath,
      symbol_id: symbolId,
      new_symbol_code: newSymbolCode,
      dry_run: true,
    });
    if (!res.success) {
      throw new Error(res.error || "Sandbox patch validation failed");
    }
    return res.data as PatchResult;
  },

  async findSymbol(query: string): Promise<SymbolSearchResult[]> {
    return await invoke<SymbolSearchResult[]>("forge_find_symbol", {
      query,
    });
  },

  async getImpactRadius(symbolId: string): Promise<ImpactAnalysisReport> {
    return await invoke<ImpactAnalysisReport>("forge_get_impact_radius", {
      symbolId,
    });
  },

  async executePatchTransaction(params: {
    baseCommit: string;
    taskId: string;
    filePath: string;
    baseContent: string;
    symbolId: string;
    newSymbolCode: string;
    intent: string;
  }): Promise<TransactionExecutionResult> {
    return await invoke<TransactionExecutionResult>("forge_execute_patch_transaction", {
      baseCommit: params.baseCommit,
      taskId: params.taskId,
      filePath: params.filePath,
      baseContent: params.baseContent,
      symbolId: params.symbolId,
      newSymbolCode: params.newSymbolCode,
      intent: params.intent,
    });
  },

  async executeMultiPatchTransaction(params: {
    baseCommit: string;
    taskId: string;
    baseFiles: Array<[string, string]>;
    patches: TransactionPatchItem[];
    intent: string;
  }): Promise<TransactionExecutionResult> {
    return await invoke<TransactionExecutionResult>("forge_execute_multi_patch_transaction", {
      baseCommit: params.baseCommit,
      taskId: params.taskId,
      baseFiles: params.baseFiles,
      patches: params.patches,
      intent: params.intent,
    });
  },

  async getLineageHistory(lineageId: string): Promise<LineageHistoryEntry[]> {
    return await invoke<LineageHistoryEntry[]>("forge_get_lineage_history", {
      lineageId,
    });
  },

  async getCanonicalEntities(): Promise<CanonicalSymbolRecord[]> {
    return await invoke<CanonicalSymbolRecord[]>("forge_get_canonical_entities");
  },

  async recordAuditEntry(params: {
    taskId: string;
    intent: string;
    targetSymbol: string;
    targetFile: string;
    baseSnapshotId?: string;
    committedSnapshotId?: string;
    healingAttempts: number;
    status: string;
    errorPayload?: string;
  }): Promise<number> {
    return await invoke<number>("forge_record_audit_entry", {
      taskId: params.taskId,
      intent: params.intent,
      targetSymbol: params.targetSymbol,
      targetFile: params.targetFile,
      baseSnapshotId: params.baseSnapshotId,
      committedSnapshotId: params.committedSnapshotId,
      healingAttempts: params.healingAttempts,
      status: params.status,
      errorPayload: params.errorPayload,
    });
  },

  async getTaskAuditHistory(taskId: string): Promise<AuditTrailRecord[]> {
    return await invoke<AuditTrailRecord[]>("forge_get_task_audit_history", {
      taskId,
    });
  },
};