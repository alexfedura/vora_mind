import {
  forgeService,
  ToolCallResponse,
  ImpactAnalysisReport,
  ExecutionPlan,
  TransactionPatchItem,
  normalizeId,
} from "./forgeService";
import { forgeLLMService } from "./llmService";

export type AgentLogType =
  | "intent_parsed"
  | "target_resolved"
  | "context_assembled"
  | "plan_generated"
  | "plan_repaired"
  | "simulation_started"
  | "patch_generated"
  | "candidate_testing"
  | "tests_passed"
  | "tests_failed"
  | "self_healing_attempt"
  | "transaction_committed"
  | "git_committed"
  | "post_commit_verified"
  | "task_completed"
  | "patch_ready"
  | "tool_call"
  | "tool_result"
  | "error"
  | "think";

export type FailureCategory =
  | "SyntaxFailure"
  | "CompileFailure"
  | "TestFailure"
  | "SemanticIdentityViolation"
  | "UnexpectedBlastRadius"
  | "SandboxTimeout"
  | "StaleWorkspace"
  | "AmbiguousTarget"
  | "PolicyFailure";

export interface AgentStepLog {
  type: AgentLogType;
  message: string;
  payload?: any;
}

export type AgentLog = AgentStepLog;

export interface TaskIntent {
  goal: string;
  symbolHint: string;
  scope?: string;
  constraints?: string[];
  requestedBehavior?: string;
  acceptanceCriteria?: string[];
  riskPolicy?: string;
}

export interface PatchProposal {
  filePath: string;
  targetSymbol: string;
  originalLines: string;
  patchedContent: string;
  dependentsCount: number;
  dependents: string[];
  canonicalEntityId?: string;
}

export interface AutonomousTaskResult {
  success: boolean;
  intent: string;
  targetSymbol: string;
  targetFile: string;
  canonicalEntityId?: string;
  executionPlan: ExecutionPlan | null;
  impactReport: ImpactAnalysisReport | null;
  patchedSnippet: string;
  originalSnippet: string;
  snapshotId?: string | null;
  gitCommitHash?: string | null;
  healingAttempts: number;
  error?: string;
}

export class ForgeAgentOrchestrator {
  private onLog: (log: AgentStepLog) => void;

  constructor(onLog: (log: AgentStepLog) => void) {
    this.onLog = onLog;
  }

  /**
   * طبقه‌بندی هوشمند شکست‌ها برای هدایت خودترمیمی و بازسازی پلن (A8 Healing State Machine)
   */
  private classifyFailure(errorLog: string): FailureCategory {
    const lower = errorLog.toLowerCase();

    if (
      lower.includes("sandbox timeout") ||
      lower.includes("execution exceeded") ||
      lower.includes("process killed safely")
    ) {
      return "SandboxTimeout";
    }
    if (
      lower.includes("stale head conflict") ||
      lower.includes("stale_workspace") ||
      lower.includes("branch moved from")
    ) {
      return "StaleWorkspace";
    }
    if (
      lower.includes("ambiguity hard gate") ||
      lower.includes("ambiguous target") ||
      lower.includes("target resolution gate")
    ) {
      return "AmbiguousTarget";
    }
    if (
      lower.includes("syntax error") ||
      lower.includes("unparseable") ||
      lower.includes("missing closing") ||
      lower.includes("unexpected token") ||
      lower.includes("tree-sitter ast validation failed")
    ) {
      return "SyntaxFailure";
    }
    if (
      lower.includes("cannot find function") ||
      lower.includes("cannot find symbol") ||
      lower.includes("no method named") ||
      lower.includes("unresolved import") ||
      lower.includes("undefined variable")
    ) {
      return "UnexpectedBlastRadius";
    }
    if (
      lower.includes("error[e") ||
      lower.includes("cannot find") ||
      lower.includes("mismatched types") ||
      lower.includes("borrowed while") ||
      lower.includes("could not compile") ||
      lower.includes("type error")
    ) {
      return "CompileFailure";
    }
    if (
      lower.includes("canonical entity") ||
      lower.includes("lineage invariant broken") ||
      lower.includes("spine integrity")
    ) {
      return "SemanticIdentityViolation";
    }
    if (
      lower.includes("panicked at") ||
      lower.includes("assertion `left == right` failed") ||
      lower.includes("test failed") ||
      lower.includes("failures:")
    ) {
      return "TestFailure";
    }
    return "PolicyFailure";
  }

  /**
   * استخراج نمادهای ناشناخته از لاگ خطای کامپایل یا تست جهت بازسازی پلن (Blast Radius Discovery)
   */
  private extractDiscoveredSymbolsFromError(errorLog: string): string[] {
    const discovered = new Set<string>();
    const funcRegex = /cannot find function `([^`]+)`/g;
    let match;
    while ((match = funcRegex.exec(errorLog)) !== null) {
      if (match[1]) discovered.add(match[1]);
    }
    const methodRegex = /no (?:method|function) named `([^`]+)`/g;
    while ((match = methodRegex.exec(errorLog)) !== null) {
      if (match[1]) discovered.add(match[1]);
    }
    return Array.from(discovered);
  }

  /**
   * اجرای چرخه تصمیم‌گیری برای ویرایش یا ریفکتور نماد در کد منبع با موتور معنایی و سندباکس اورلی
   */
  async runIntent(
    filePath: string,
    prompt: string,
    targetSymbolName: string,
    newSymbolSnippet: string
  ): Promise<PatchProposal | null> {
    try {
      this.onLog({
        type: "think",
        message: `Analyzing user intent: "${prompt}" on file [${filePath}]...`,
      });

      this.onLog({
        type: "tool_call",
        message: `Executing tool 'find_symbol' for symbol '${targetSymbolName}'`,
        payload: { query: targetSymbolName },
      });

      const findResult: ToolCallResponse = await forgeService.executeForgeTool(
        "find_symbol",
        { query: targetSymbolName }
      );

      let targetSymbolMeta = null;
      if (findResult.success && Array.isArray(findResult.data) && findResult.data.length > 0) {
        targetSymbolMeta =
          findResult.data.find(
            (s: any) => s.name === targetSymbolName || s.file_path?.endsWith(filePath)
          ) || findResult.data[0];
      }

      const symbolId = targetSymbolMeta ? targetSymbolMeta.id : targetSymbolName;
      const canonicalEntityId = targetSymbolMeta?.canonical_entity_id
        ? normalizeId(targetSymbolMeta.canonical_entity_id)
        : undefined;

      const lineRange = targetSymbolMeta
        ? `Lines ${targetSymbolMeta.start_line}-${targetSymbolMeta.end_line}`
        : "Lines ~";

      this.onLog({
        type: "tool_result",
        message: `Symbol '${targetSymbolName}' identified. (ID: ${symbolId}, Scope: ${lineRange})`,
        payload: targetSymbolMeta || { symbolId },
      });

      this.onLog({
        type: "tool_call",
        message: `Calculating blast radius with tool 'get_impact_radius'`,
        payload: { symbol_id: symbolId },
      });

      const impactResult: ToolCallResponse = await forgeService.executeForgeTool(
        "get_impact_radius",
        { symbol_id: symbolId }
      );

      let dependents: string[] = [];
      if (impactResult.success && impactResult.data) {
        const report = impactResult.data as ImpactAnalysisReport;
        const direct = Array.isArray(report.direct_dependents)
          ? report.direct_dependents.map((d: any) => d.symbol_name || d.symbol_id)
          : [];
        const indirect = Array.isArray(report.indirect_dependents)
          ? report.indirect_dependents.map((d: any) => d.symbol_name || d.symbol_id)
          : [];
        dependents = [...direct, ...indirect];
      }

      const dependentsCount = dependents.length;

      this.onLog({
        type: "tool_result",
        message: `Impact Analysis: ${dependentsCount} downstream dependent(s) reference '${targetSymbolName}'`,
        payload: { dependents, dependentsCount },
      });

      const originalFileContent = await forgeService.readProjectFile(filePath);

      let codeToApply = newSymbolSnippet.trim();
      if (!codeToApply) {
        const activeModel = forgeLLMService.getConfig().model;
        this.onLog({
          type: "think",
          message: `Synthesizing surgical AST replacement using LLM (${activeModel})...`,
        });

        codeToApply = await forgeLLMService.generateRefactoredSymbol(
          targetSymbolName,
          targetSymbolMeta?.signature || "",
          prompt,
          filePath
        );

        this.onLog({
          type: "tool_result",
          message: `LLM generated ${codeToApply.length} bytes of refactored code.`,
          payload: { generated_snippet: codeToApply },
        });
      }

      this.onLog({
        type: "tool_call",
        message: `Validating patch syntax in Overlay Sandbox with 'apply_surgical_patch' (dry_run: true)`,
        payload: { file_path: filePath, symbol_id: symbolId, dry_run: true },
      });

      const patchResult: ToolCallResponse = await forgeService.executeForgeTool(
        "apply_surgical_patch",
        {
          file_path: filePath,
          symbol_id: symbolId,
          new_symbol_code: codeToApply,
          dry_run: true,
        }
      );

      if (!patchResult.success) {
        throw new Error(patchResult.error || "Surgical patch syntax validation failed");
      }

      this.onLog({
        type: "tool_result",
        message: `AST syntax validation passed on dry-run! Byte range: ${
          patchResult.data?.original_byte_range?.[0] ?? 0
        }..${patchResult.data?.original_byte_range?.[1] ?? 0}`,
        payload: patchResult.data,
      });

      const lines = originalFileContent.split("\n");
      const startLine = targetSymbolMeta ? Math.max(0, targetSymbolMeta.start_line - 1) : 0;
      const endLine = targetSymbolMeta ? Math.min(lines.length, targetSymbolMeta.end_line) : lines.length;

      const before = lines.slice(0, startLine).join("\n");
      const after = lines.slice(endLine).join("\n");
      const patchedFullContent = `${before}\n${codeToApply}\n${after}`.trim();

      const proposal: PatchProposal = {
        filePath,
        targetSymbol: targetSymbolName,
        originalLines: lineRange,
        patchedContent: patchedFullContent,
        dependentsCount,
        dependents,
        canonicalEntityId,
      };

      this.onLog({
        type: "patch_ready",
        message: `AST Patch ready for lines ${proposal.originalLines}. Downstream dependents: ${dependentsCount}. Ready for review.`,
        payload: proposal,
      });

      return proposal;
    } catch (err: any) {
      this.onLog({
        type: "error",
        message: `Agent execution aborted: ${err?.message || err}`,
      });
      return null;
    }
  }

  /**
   * P10.5 & P11 & P13: موتور اجرای تمام‌خودکار بر پایه هدف معین، بازیابی خطا، ممیزی ردپا، AST Diff و سیم‌کشی ایمپورت‌ها در هسته Rust
   */
  async runAutonomousTask(rawPrompt: string): Promise<AutonomousTaskResult> {
    const taskId = `task_${Date.now()}`;
    let healAttempt = 0;
    let targetSymbolName = "";
    let targetFilePath = "";
    let baseGitHead = "HEAD";

    try {
      // A6 Runtime Stale-HEAD Gate: ثبت و ذخیره قطعی هش HEAD فعلی در آغاز تسک
      try {
        baseGitHead = await forgeService.getCurrentGitHead();
      } catch {
        baseGitHead = "HEAD";
      }

      // ۱. تحلیل نیت کاربر و استخراج ساختاریافته هدف با استفاده از LLM (TaskIntent Contract)
      this.onLog({
        type: "think",
        message: `Parsing raw intent into structured TaskIntent: "${rawPrompt}"...`,
      });

      const parsedIntent = await this.extractIntentAndTarget(rawPrompt);
      this.onLog({
        type: "intent_parsed",
        message: `Goal: "${parsedIntent.goal}". Identified target symbol hint: '${parsedIntent.symbolHint}'`,
        payload: parsedIntent,
      });

      // ۲. A1 Target Resolver Hard Gate: جستجو و اعمال قطعی گیت رفع ابهام
      const findResult: ToolCallResponse = await forgeService.executeForgeTool(
        "find_symbol",
        { query: parsedIntent.symbolHint }
      );

      if (
        !findResult.success ||
        !Array.isArray(findResult.data) ||
        findResult.data.length === 0
      ) {
        throw new Error(
          `Target Resolution Gate: Zero symbols matching '${parsedIntent.symbolHint}' in workspace index.`
        );
      }

      const exactMatches = findResult.data.filter(
        (s: any) => s.name.toLowerCase() === parsedIntent.symbolHint.toLowerCase()
      );

      let targetMeta: any = null;

      if (exactMatches.length === 1) {
        targetMeta = exactMatches[0];
      } else if (exactMatches.length > 1) {
        this.onLog({
          type: "error",
          message: `Ambiguity Hard Gate: Found ${exactMatches.length} matching symbols for '${parsedIntent.symbolHint}'. Execution halted to protect codebase invariants.`,
          payload: exactMatches,
        });
        throw new Error(
          `Ambiguous target '${parsedIntent.symbolHint}' exists in multiple files (${exactMatches
            .map((e: any) => e.file_path)
            .join(", ")}). Please refine your prompt with the exact file path.`
        );
      } else {
        this.onLog({
          type: "error",
          message: `Target Resolution Gate: No exact match for '${parsedIntent.symbolHint}'. Execution halted to avoid guessing targets.`,
          payload: findResult.data,
        });
        throw new Error(
          `No exact symbol matched for '${parsedIntent.symbolHint}'. Candidates: [${findResult.data
            .map((d: any) => d.name)
            .slice(0, 5)
            .join(", ")}]. Refine prompt target.`
        );
      }

      targetSymbolName = targetMeta.name;
      targetFilePath = targetMeta.file_path;
      const symbolId = targetMeta.id;
      const canonicalEntityId = targetMeta.canonical_entity_id
        ? normalizeId(targetMeta.canonical_entity_id)
        : undefined;

      this.onLog({
        type: "target_resolved",
        message: `Target disambiguated: '${targetSymbolName}' in ${targetFilePath} (Symbol ID: ${symbolId})`,
        payload: { targetSymbolName, targetFilePath, symbolId, canonicalEntityId },
      });

      // A7: ثبت شروع لاگ ردپا بدون سایلنت کردن خطاها
      await forgeService.recordAuditEntry({
        taskId,
        intent: parsedIntent.goal,
        targetSymbol: targetSymbolName,
        targetFile: targetFilePath,
        healingAttempts: 0,
        status: "IN_PROGRESS",
      });

      // ۳. جمع‌آوری کانتکست ایزوله نماد (Context Assembly)
      this.onLog({
        type: "context_assembled",
        message: `Extracting AST slice & context for '${targetSymbolName}'...`,
      });

      const originalFileContent = await forgeService.readProjectFile(targetFilePath);

      // ۴. سنتز اولیه کد کاندید با هوش مصنوعی (Candidate Code Generation)
      this.onLog({
        type: "patch_generated",
        message: `Synthesizing primary surgical AST replacement using LLM...`,
      });

      let newSymbolCode = await forgeLLMService.generateRefactoredSymbol(
        targetSymbolName,
        targetMeta.signature || "",
        rawPrompt,
        targetFilePath
      );

      // ۵. P10.5-H2: محاسبه تفاوت معنایی واقعی با Tree-sitter در هسته Rust (Real AST Semantic Diff)
      let actualSemanticChange = await forgeService.computeAstSemanticDiff(
        targetFilePath,
        targetSymbolName,
        originalFileContent,
        newSymbolCode
      );

      let impactReport = await forgeService.getImpactRadius(symbolId);
      let executionPlan = await forgeService
        .createSemanticPlan(".", actualSemanticChange)
        .catch(() => null);

      this.onLog({
        type: "plan_generated",
        message: `Execution Plan synthesized (${executionPlan?.total_steps || 1} steps). Downstream blast radius: ${impactReport.total_impacted_count} dependent(s).`,
        payload: { impactReport, executionPlan, actualSemanticChange },
      });

      // ۶. گردآوری پچ‌های چندفایلی مرتبط (Multi-file Patch Assembly & Import Rewiring)
      const multiPatches: TransactionPatchItem[] = [
        {
          file_path: targetFilePath,
          symbol_id: symbolId,
          new_symbol_code: newSymbolCode,
        },
      ];

      const baseFilesMap = new Map<string, string>();
      baseFilesMap.set(targetFilePath, originalFileContent);

      const assembleDependentPatches = async () => {
        if (executionPlan && executionPlan.steps.length > 1) {
          // P11: استخراج نام جدید نماد در صورت وجود جهش تغییر نام (Rename Rewiring)
          const renameInfo =
            typeof actualSemanticChange === "object" && "SymbolRenamed" in actualSemanticChange
              ? actualSemanticChange.SymbolRenamed
              : null;

          for (const step of executionPlan.steps) {
            if (step.symbol_name === targetSymbolName && step.file_path === targetFilePath) {
              continue;
            }

            try {
              let stepBaseContent = baseFilesMap.get(step.file_path);
              if (!stepBaseContent) {
                stepBaseContent = await forgeService.readProjectFile(step.file_path);
                baseFilesMap.set(step.file_path, stepBaseContent);
              }

              // P11: سیم‌کشی مجدد خودکار ایمپورت‌ها در صورت وجود تغییر نام نماد
              if (renameInfo && renameInfo.old_name && renameInfo.new_name) {
                try {
                  const rewireRes = await forgeService.rewireImports(
                    step.file_path,
                    renameInfo.old_name,
                    renameInfo.new_name,
                    false
                  );
                  if (rewireRes.rewiredContent && rewireRes.rewiredContent !== stepBaseContent) {
                    stepBaseContent = rewireRes.rewiredContent;
                    baseFilesMap.set(step.file_path, stepBaseContent);
                    this.onLog({
                      type: "patch_generated",
                      message: `[ImportRewiring] Auto-rewired imports for '${renameInfo.old_name}' -> '${renameInfo.new_name}' in ${step.file_path}`,
                    });
                  }
                } catch {
                  // در صورت عدم نیاز به تغییر ایمپورت، ادامه می‌دهیم
                }
              }

              this.onLog({
                type: "patch_generated",
                message: `Synthesizing dependent patch for step #${step.step_order} [${step.symbol_name}] in ${step.file_path} (Depth: ${step.cascade_depth ?? 1})...`,
              });

              const dependentCode = await forgeLLMService.generateRefactoredSymbol(
                step.symbol_name,
                step.code_slice?.target_code || "",
                `Adapt caller to updated '${targetSymbolName}': ${rawPrompt}`,
                step.file_path
              );

              const existingIdx = multiPatches.findIndex(
                (p) => p.file_path === step.file_path && p.symbol_id === step.symbol_name
              );
              if (existingIdx >= 0) {
                multiPatches[existingIdx].new_symbol_code = dependentCode;
              } else {
                multiPatches.push({
                  file_path: step.file_path,
                  symbol_id: step.symbol_name,
                  new_symbol_code: dependentCode,
                });
              }
            } catch (e: any) {
              const isRequiredStep = (step.cascade_depth ?? 1) <= 2;
              if (isRequiredStep) {
                throw new Error(`Required dependent step #${step.step_order} [${step.symbol_name}] in ${step.file_path} failed to synthesize: ${e?.message || e}`);
              }
              this.onLog({
                type: "think",
                message: `Optional dependent step [${step.symbol_name}] skipped: ${e?.message || e}`,
              });
            }
          }
        }
      };

      await assembleDependentPatches();

      // ۷. اجرای آزمون‌های سندباکس چندفایلی به همراه بازسازی پلن و خودترمیمی پویا (A8 Healing State Machine)
      const MAX_HEAL_ATTEMPTS = 3;
      let testPassed = false;

      while (healAttempt <= MAX_HEAL_ATTEMPTS && !testPassed) {
        this.onLog({
          type: "simulation_started",
          message: `Validating ${multiPatches.length} patch(es) in Overlay Sandbox (Attempt ${healAttempt + 1}/${MAX_HEAL_ATTEMPTS + 1})...`,
        });

        const preflight = await forgeService.executeForgeTool("apply_surgical_patch", {
          file_path: targetFilePath,
          symbol_id: symbolId,
          new_symbol_code: newSymbolCode,
          dry_run: true,
        });

        if (!preflight.success) {
          if (healAttempt < MAX_HEAL_ATTEMPTS) {
            this.onLog({
              type: "self_healing_attempt",
              message: `[SyntaxFailure] AST syntax invalid. Activating self-healing loop...`,
            });
            newSymbolCode = await forgeLLMService.healRefactoredSymbol(
              targetSymbolName,
              newSymbolCode,
              preflight.error || "AST syntax parse failure",
              rawPrompt,
              targetFilePath
            );
            multiPatches[0].new_symbol_code = newSymbolCode;
            healAttempt++;
            continue;
          } else {
            throw new Error(preflight.error || "AST Syntax validation failed after self-healing attempts.");
          }
        }

        this.onLog({
          type: "candidate_testing",
          message: `Executing project regression test suite in isolated workspace sandbox...`,
        });

        const testResult = await forgeService.testMultiCandidatePatches(multiPatches);

        if (testResult.success) {
          testPassed = true;
          this.onLog({
            type: "tests_passed",
            message: `All candidate workspace regression tests passed cleanly!`,
          });
        } else {
          const category = this.classifyFailure(testResult.output);

          // A8: گیت توقف فوری روی خطاهای غیرقابل بازیابی
          if (
            category === "SandboxTimeout" ||
            category === "StaleWorkspace" ||
            category === "AmbiguousTarget" ||
            category === "SemanticIdentityViolation"
          ) {
            this.onLog({
              type: "error",
              message: `[Fatal Failure: ${category}] Execution aborted immediately to prevent unsafe mutations.`,
              payload: testResult.output,
            });
            throw new Error(`Execution halted due to unrecoverable invariant failure: ${category}`);
          }

          if (healAttempt < MAX_HEAL_ATTEMPTS) {
            healAttempt++;

            // P10.5-H4 Plan Repair: بازسازی و گسترش پلن در هسته Rust هنگام کشف نمادهای وابسته ناشناخته
            if (category === "UnexpectedBlastRadius" && executionPlan) {
              const discoveredSymbols = this.extractDiscoveredSymbolsFromError(testResult.output);

              this.onLog({
                type: "plan_repaired",
                message: `[PlanRepair] Discovered ${discoveredSymbols.length} unexpected dependent callsite(s). Expanding execution plan in Rust engine...`,
                payload: { discoveredSymbols },
              });

              executionPlan = await forgeService.repairAndExpandPlan(
                ".",
                executionPlan,
                discoveredSymbols
              );

              await assembleDependentPatches();
            }

            this.onLog({
              type: "self_healing_attempt",
              message: `[${category}] Candidate tests failed. Policy-driven healing attempt ${healAttempt}/${MAX_HEAL_ATTEMPTS}...`,
              payload: { category, output: testResult.output },
            });

            await forgeService.recordAuditEntry({
              taskId,
              intent: parsedIntent.goal,
              targetSymbol: targetSymbolName,
              targetFile: targetFilePath,
              healingAttempts: healAttempt,
              status: "HEALING",
              errorPayload: testResult.output.slice(0, 500),
            });

            newSymbolCode = await forgeLLMService.healRefactoredSymbol(
              targetSymbolName,
              newSymbolCode,
              testResult.output,
              rawPrompt,
              targetFilePath
            );
            multiPatches[0].new_symbol_code = newSymbolCode;
          } else {
            this.onLog({
              type: "tests_failed",
              message: `Candidate tests failed after ${MAX_HEAL_ATTEMPTS} healing attempts. Aborting commit!`,
              payload: testResult.output,
            });
            throw new Error(`Tests failed on candidate code:\n${testResult.output.slice(0, 300)}`);
          }
        }
      }

      // A6: راستی‌آزمایی Stale HEAD پیش از تعهد تراکنش
      try {
        const latestHead = await forgeService.getCurrentGitHead();
        if (baseGitHead !== "HEAD" && latestHead !== baseGitHead) {
          throw new Error(`Stale HEAD Conflict: Branch moved from ${baseGitHead} to ${latestHead} during task execution.`);
        }
      } catch (e: any) {
        if (e.message?.includes("Stale HEAD Conflict")) throw e;
      }

      // ۸. کامیت تراکنش اتمیک همه‌یاهیچ (Atomic Multi-File / Single-File Transaction Commit)
      let txSnapshotId: string | null = null;
      let finalCommitMessage = "";

      if (multiPatches.length > 1) {
        this.onLog({
          type: "transaction_committed",
          message: `Committing multi-file atomic transaction (${multiPatches.length} patches)...`,
        });

        const baseFilesArray: Array<[string, string]> = Array.from(baseFilesMap.entries());
        const multiTxRes = await forgeService.executeMultiPatchTransaction({
          baseCommit: baseGitHead,
          taskId,
          baseFiles: baseFilesArray,
          patches: multiPatches,
          intent: rawPrompt,
        });

        if (!multiTxRes.is_committed) {
          throw new Error("Multi-file atomic transaction commit failed in SQLite engine.");
        }
        txSnapshotId = multiTxRes.snapshot_id || null;
        finalCommitMessage = multiTxRes.commit_message;
      } else {
        const txResult = await forgeService.executePatchTransaction({
          baseCommit: baseGitHead,
          taskId,
          filePath: targetFilePath,
          baseContent: originalFileContent,
          symbolId: symbolId,
          newSymbolCode: newSymbolCode,
          intent: rawPrompt,
        });

        if (!txResult.is_committed) {
          throw new Error("Atomic transaction commit failed in SQLite engine.");
        }
        txSnapshotId = txResult.snapshot_id || null;
        finalCommitMessage = txResult.commit_message;
      }

      this.onLog({
        type: "transaction_committed",
        message: `Atomic snapshot recorded: ${txSnapshotId || "latest"}`,
      });

      // ۹. A5: ثبت Native Semantic Git SSOT بر اساس پیام یکپارچه تولیدشده از SemanticCommitPayload
      const gitMsg = await forgeService.createGitCommit(
        targetFilePath,
        targetSymbolName,
        finalCommitMessage || `refactor(${targetSymbolName}): ${parsedIntent.goal}\n\nTask ID: ${taskId}`
      );

      this.onLog({
        type: "git_committed",
        message: `Native Semantic Git SSOT commit recorded.`,
        payload: gitMsg,
      });

      // ۱۰. گیت اعتبارسنجی قطعی پس از کامیت (Post-Commit Hard Verification Gate)
      this.onLog({
        type: "post_commit_verified",
        message: `Post-commit verification: Checking canonical identity spine integrity...`,
      });

      const updatedEntities = await forgeService.getCanonicalEntities();
      const entityPreserved = updatedEntities.some(
        (e) => e.name === targetSymbolName || normalizeId(e.entity_id) === canonicalEntityId
      );

      if (!entityPreserved) {
        throw new Error(
          `Post-Commit Verification Gate Failed: Canonical entity '${targetSymbolName}' could not be verified in post-commit index spine.`
        );
      }

      // ثبت موفقیت کامل در لاگ ردپا
      await forgeService.recordAuditEntry({
        taskId,
        intent: parsedIntent.goal,
        targetSymbol: targetSymbolName,
        targetFile: targetFilePath,
        committedSnapshotId: txSnapshotId || undefined,
        healingAttempts: healAttempt,
        status: "COMPLETED",
      });

      this.onLog({
        type: "task_completed",
        message: `Autonomous task completed successfully! Target '${targetSymbolName}' safely refactored and verified.`,
      });

      const lines = originalFileContent.split("\n");
      const startLine = Math.max(0, targetMeta.start_line - 1);
      const endLine = Math.min(lines.length, targetMeta.end_line);

      return {
        success: true,
        intent: parsedIntent.goal,
        targetSymbol: targetSymbolName,
        targetFile: targetFilePath,
        canonicalEntityId,
        executionPlan,
        impactReport,
        patchedSnippet: newSymbolCode,
        originalSnippet: originalFileContent.slice(startLine, endLine),
        snapshotId: txSnapshotId,
        gitCommitHash: gitMsg,
        healingAttempts: healAttempt,
      };
    } catch (error: any) {
      const errorMsg = error?.message || String(error);

      // ثبت خطای قطعی در صورت توقف تسک
      await forgeService.recordAuditEntry({
        taskId,
        intent: rawPrompt,
        targetSymbol: targetSymbolName || "unknown",
        targetFile: targetFilePath || "unknown",
        healingAttempts: healAttempt,
        status: "FAILED",
        errorPayload: errorMsg,
      }).catch(() => 0);

      this.onLog({
        type: "error",
        message: `Autonomous execution aborted: ${errorMsg}`,
      });
      return {
        success: false,
        intent: rawPrompt,
        targetSymbol: "",
        targetFile: "",
        executionPlan: null,
        impactReport: null,
        patchedSnippet: "",
        originalSnippet: "",
        healingAttempts: healAttempt,
        error: errorMsg,
      };
    }
  }

  /**
   * استخراج نیت ساختاریافته از پرامپت خام با استفاده از LLM همراه با گیت اعتبارسنجی قطعی
   */
  private async extractIntentAndTarget(rawPrompt: string): Promise<TaskIntent> {
    const res = await forgeLLMService.decomposeTaskIntent(rawPrompt);
    if (!res || !res.goal || !res.symbolHint) {
      throw new Error(
        `Failed to parse task intent from raw prompt. Ambiguous or missing goal/target symbol in: "${rawPrompt}"`
      );
    }
    const rawRes = res as any;
    return {
      goal: res.goal.trim(),
      symbolHint: res.symbolHint.trim(),
      scope: res.scope?.trim(),
      constraints: rawRes.constraints,
      requestedBehavior: rawRes.requestedBehavior,
      acceptanceCriteria: rawRes.acceptanceCriteria,
      riskPolicy: rawRes.riskPolicy,
    };
  }
}