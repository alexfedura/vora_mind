import React, { useState, useEffect } from "react";
import {
  Sparkles,
  X,
  Play,
  FileCode2,
  GitGraph,
  Workflow,
  Terminal,
  FolderOpen,
  CheckCircle2,
  Cpu,
  Eye,
  AlertTriangle,
  FolderTree,
  Settings,
  BookOpen,
  ShieldCheck,
  Flame,
  Network,
  Search,
  Bot,
  Check,
  Loader2,
  XCircle,
  History,
  Clock,
  RotateCcw,
  ArchiveRestore,
  Layers,
  ArrowRight,
} from "lucide-react";
import { useAppStore } from "@/shared/store/useAppStore";
import {
  forgeService,
  ExecutionPlan,
  ImpactAnalysisReport,
  AstSymbol,
  AstRelation,
  CodeChurnMetric,
  ImpactedTarget,
  PlanStep,
  normalizeId,
  CanonicalSymbolRecord,
} from "@/forge/services/forgeService";
import { forgeLLMService } from "@/forge/services/llmService";
import {
  ForgeAgentOrchestrator,
  PatchProposal,
  AutonomousTaskResult,
} from "@/forge/services/agentOrchestrator";
import { PatchDiffModal } from "../modals/PatchDiffModal";
import { LLMSettingsModal } from "../modals/LLMSettingsModal";
import { CanonicalEntityInspector } from "./CanonicalEntityInspector";

interface ForgeAgentDrawerProps {
  onInjectGraphToCanvas: (payload: {
    nodes: any[];
    edges: any[];
  }) => void;
}

const AUTONOMOUS_STEPS = [
  { id: "intent", label: "Intent Decomposed" },
  { id: "target", label: "Target Discovered & Disambiguated" },
  { id: "context", label: "Context Assembled" },
  { id: "plan", label: "Plan & Blast Radius (N-Tier)" },
  { id: "patch", label: "AST Patch & Import Rewiring" },
  { id: "preflight", label: "Overlay Sandbox Validated" },
  { id: "tests", label: "Candidate Tests Green" },
  { id: "tx", label: "Atomic Transaction Committed" },
  { id: "git", label: "Semantic Git SSOT Recorded" },
  { id: "verify", label: "Post-Commit Verification Gate" },
];

export const ForgeAgentDrawer: React.FC<ForgeAgentDrawerProps> = ({
  onInjectGraphToCanvas,
}) => {
  const store = useAppStore();
  const {
    isForgeAgentOpen,
    toggleForgeAgent,
    addCard,
    fetchCanonicalEntities,
    canonicalEntities,
    selectCanonicalEntity,
    executedTasks,
    recordExecutedTask,
    clearExecutedTasks,
  } = store;
  const [activeTab, setActiveTab] = useState<"intent" | "graph" | "history">("intent");

  const [filePath, setFilePath] = useState("src/forge/ast_engine.rs");
  const [workspaceDir, setWorkspaceDir] = useState("src/forge");
  const [sourceCode, setSourceCode] = useState(`pub struct MemoryManager {
    capacity: usize,
}

pub fn allocate_block(size: usize) -> bool {
    let _mgr: MemoryManager;
    true
}

pub fn deallocate_block(id: usize) {
    allocate_block(0);
}`);

  // فیلدهای بخش Intent Runner
  const [prompt, setPrompt] = useState(
    "Modify allocate_block to check against capacity and return false if exceeds"
  );
  const [targetSymbol, setTargetSymbol] = useState("allocate_block");
  const [searchQuery, setSearchQuery] = useState("");
  const [newSnippet, setNewSnippet] = useState(`pub fn allocate_block(size: usize) -> bool {
    if size > 1024 {
        return false;
    }
    true
}`);
  const [currentPatch, setCurrentPatch] = useState<PatchProposal | null>(null);
  const [executionPlan, setExecutionPlan] = useState<ExecutionPlan | null>(null);
  const [impactReport, setImpactReport] = useState<ImpactAnalysisReport | null>(null);
  const [isDiffModalOpen, setIsDiffModalOpen] = useState(false);
  const [isSettingsModalOpen, setIsSettingsModalOpen] = useState(false);

  // وضعیت‌های پیشرفت موتور تمام‌خودکار (P10.5 & P12 Stepper State)
  const [currentAutoStepIndex, setCurrentAutoStepIndex] = useState<number>(-1);
  const [autoTaskSuccess, setAutoTaskSuccess] = useState<boolean | null>(null);

  const [isProcessing, setIsProcessing] = useState(false);
  const [isAutonomousRunning, setIsAutonomousRunning] = useState(false);
  const [isLoadingFile, setIsLoadingFile] = useState(false);
  const [isScanningWorkspace, setIsScanningWorkspace] = useState(false);
  const [isCalculatingHeatmap, setIsCalculatingHeatmap] = useState(false);
  const [agentLogs, setAgentLogs] = useState<string[]>([
    "Forge AST Engine Protocol v0.48.0 ready.",
    "FTP (Forge Tool Protocol) & Semantic Core v1.0-Frozen initialized.",
    "Autonomous Multi-File Orchestrator (P12 Deep Observability) armed.",
  ]);

  useEffect(() => {
    fetchCanonicalEntities();
  }, [fetchCanonicalEntities]);

  useEffect(() => {
    const handleInspectAstNode = (e: Event) => {
      const customEvent = e as CustomEvent<{
        name: string;
        filePath?: string;
        signature?: string;
        canonicalEntityId?: string;
      }>;
      const { name, filePath: nodeFile, signature, canonicalEntityId } = customEvent.detail;
      if (!isForgeAgentOpen) toggleForgeAgent();
      setActiveTab("intent");
      if (nodeFile) setFilePath(nodeFile);
      setTargetSymbol(name);
      setPrompt(`Refactor symbol '${name}' to validate bounds and handle errors`);
      if (signature) setNewSnippet(signature);

      if (canonicalEntityId) {
        const found = canonicalEntities.find(
          (c) => normalizeId(c.entity_id) === canonicalEntityId || c.name === name
        );
        if (found) {
          selectCanonicalEntity(found);
        }
      }

      setAgentLogs((prev) => [
        ...prev,
        `[Canvas Focus 🎯] Focused on AST Node: '${name}'`,
      ]);
    };

    window.addEventListener("vora:inspect-ast-node", handleInspectAstNode);
    return () => window.removeEventListener("vora:inspect-ast-node", handleInspectAstNode);
  }, [isForgeAgentOpen, toggleForgeAgent, canonicalEntities, selectCanonicalEntity]);

  if (!isForgeAgentOpen) return null;

  const handleSearchSelect = (entity: CanonicalSymbolRecord) => {
    selectCanonicalEntity(entity);
    setTargetSymbol(entity.name);
    setNewSnippet(entity.canonical_signature);
    setPrompt(`Refactor canonical entity '${entity.name}'`);
    setAgentLogs((prev) => [
      ...prev,
      `[Canonical Inspector 🔍] Selected Entity: ${entity.name} (${normalizeId(entity.entity_id)})`,
    ]);
  };

  const filteredEntities = searchQuery.trim()
    ? canonicalEntities.filter(
        (e) =>
          e.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          e.module_namespace.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : [];

  const handleLoadProjectFile = async () => {
    if (!filePath.trim()) return;
    setIsLoadingFile(true);
    setAgentLogs((prev) => [...prev, `[FileSystem] Reading file: ${filePath}...`]);

    try {
      const content = await forgeService.readProjectFile(filePath);
      setSourceCode(content);
      setAgentLogs((prev) => [
        ...prev,
        `[Success] File loaded (${content.length} bytes).`,
      ]);
    } catch (err: any) {
      setAgentLogs((prev) => [
        ...prev,
        `[Error] Read failed: ${err?.toString() || err}`,
      ]);
    } finally {
      setIsLoadingFile(false);
    }
  };

  const handleRunAstAnalysis = async () => {
    if (!sourceCode.trim()) return;
    setIsProcessing(true);
    setAgentLogs((prev) => [...prev, `[Agent] Parsing: ${filePath}...`]);

    try {
      const result = await forgeService.parseCodeAst(filePath, sourceCode);

      const CARD_WIDTH = 300;
      const CARD_HEIGHT = 160;
      const GAP_X = 70;
      const GAP_Y = 60;

      const startX = 350;
      const startY = 150;
      const columns = 2;

      const generatedNodes: any[] = [];
      const generatedEdges: any[] = [];

      result.symbols.forEach((sym: AstSymbol, index: number) => {
        const col = index % columns;
        const row = Math.floor(index / columns);
        const nodeId = `ast-${sym.name}`;

        generatedNodes.push({
          id: nodeId,
          type: "forgeAst",
          position: {
            x: startX + col * (CARD_WIDTH + GAP_X),
            y: startY + row * (CARD_HEIGHT + GAP_Y),
          },
          data: {
            name: sym.name,
            kind: sym.kind,
            signature: sym.signature,
            filePath: sym.file_path,
            canonicalEntityId: normalizeId(sym.canonical_entity_id),
            status: "clean",
          },
        });
      });

      if (result.relations && result.relations.length > 0) {
        result.relations.forEach((rel: AstRelation, rIdx: number) => {
          const sourceNode = generatedNodes.find((n) => rel.source_id.endsWith(`:${n.data.name}`));
          const targetNode = generatedNodes.find((n) => rel.target_id.endsWith(`:${n.data.name}`));

          if (sourceNode && targetNode) {
            generatedEdges.push({
              id: `edge-${sourceNode.id}-${targetNode.id}-${rIdx}`,
              source: sourceNode.id,
              target: targetNode.id,
              type: "smoothstep",
              label: rel.relation_type === "references_type" ? "uses type" : "calls",
              labelStyle: { fill: "#c084fc", fontSize: 10, fontFamily: "monospace" },
              style: {
                stroke: rel.relation_type === "references_type" ? "#a855f7" : "#06b6d4",
                strokeWidth: 2,
              },
              animated: true,
            });
          }
        });
      }

      onInjectGraphToCanvas({
        nodes: generatedNodes,
        edges: generatedEdges,
      });

      setAgentLogs((prev) => [
        ...prev,
        `[Success] Rendered ${generatedNodes.length} Nodes & ${generatedEdges.length} Semantic Edges.`,
      ]);
    } catch (err: any) {
      setAgentLogs((prev) => [
        ...prev,
        `[Error] AST Analysis failed: ${err?.toString() || err}`,
      ]);
    } finally {
      setIsProcessing(false);
    }
  };

  const handleScanEntireWorkspace = async () => {
    if (!workspaceDir.trim()) return;
    setIsScanningWorkspace(true);
    setAgentLogs((prev) => [
      ...prev,
      `[Workspace Indexer] Scanning directory recursively: ${workspaceDir}...`,
    ]);

    try {
      const payload = await forgeService.scanWorkspace(workspaceDir);
      await fetchCanonicalEntities();

      const CARD_WIDTH = 300;
      const CARD_HEIGHT = 160;
      const GAP_X = 80;
      const GAP_Y = 70;

      const startX = 350;
      const startY = 150;
      const columns = 3;

      const generatedNodes: any[] = [];
      const generatedEdges: any[] = [];

      payload.symbols.forEach((sym: AstSymbol, index: number) => {
        const col = index % columns;
        const row = Math.floor(index / columns);
        const nodeId = `ast-ws-${sym.name}-${index}`;

        generatedNodes.push({
          id: nodeId,
          type: "forgeAst",
          position: {
            x: startX + col * (CARD_WIDTH + GAP_X),
            y: startY + row * (CARD_HEIGHT + GAP_Y),
          },
          data: {
            name: sym.name,
            kind: sym.kind,
            signature: sym.signature,
            filePath: sym.file_path,
            canonicalEntityId: normalizeId(sym.canonical_entity_id),
            status: "clean",
          },
        });
      });

      onInjectGraphToCanvas({
        nodes: generatedNodes,
        edges: generatedEdges,
      });

      setAgentLogs((prev) => [
        ...prev,
        `[Workspace Indexed 🌐] Rendered ${generatedNodes.length} symbols across '${workspaceDir}'.`,
      ]);
    } catch (err: any) {
      setAgentLogs((prev) => [
        ...prev,
        `[Error ❌] Workspace scan failed: ${err?.toString() || err}`,
      ]);
    } finally {
      setIsScanningWorkspace(false);
    }
  };

  const handleCalculateCodeChurnHeatmap = async () => {
    setIsCalculatingHeatmap(true);
    setAgentLogs((prev) => [
      ...prev,
      `[Git Churn Engine 🔥] Analyzing commit frequency and line mutations across repository history...`,
    ]);

    try {
      const churnMetrics = await forgeService.getCodeChurnMetrics(40);

      window.dispatchEvent(
        new CustomEvent("vora:update-canvas-churn-heatmap", {
          detail: { metrics: churnMetrics },
        })
      );

      const hotspotCount = churnMetrics.filter((m: CodeChurnMetric) => m.churn_score >= 0.6).length;

      setAgentLogs((prev) => [
        ...prev,
        `[Heatmap Applied 🔥] Analyzed ${churnMetrics.length} files. Identified ${hotspotCount} high-churn hotspots on Canvas.`,
      ]);
    } catch (err: any) {
      setAgentLogs((prev) => [
        ...prev,
        `[Error ❌] Failed to calculate Churn Heatmap: ${err?.toString() || err}`,
      ]);
    } finally {
      setIsCalculatingHeatmap(false);
    }
  };

  const handleSimulateSemanticBlastRadius = async () => {
    if (!targetSymbol) return;
    setIsProcessing(true);
    setAgentLogs((prev) => [
      ...prev,
      `[Semantic Simulation 🔬] Simulating Blast Radius and Execution Plan for '${targetSymbol}'...`,
    ]);

    try {
      const isSignatureMutated = newSnippet.includes("(") && !newSnippet.includes("let _mgr");
      const semanticChange: any = isSignatureMutated
        ? {
            SignatureChanged: {
              old_symbol_id: `${filePath}:${targetSymbol}`,
              new_symbol_id: `${filePath}:${targetSymbol}`,
              lineage_id: `lin_${targetSymbol}`,
              old_signature: "pub fn allocate_block(size: usize) -> bool",
              new_signature: newSnippet,
            },
          }
        : {
            BodyChanged: {
              symbol_id: `${filePath}:${targetSymbol}`,
              lineage_id: `lin_${targetSymbol}`,
              old_body_hash: "old_hash",
              new_body_hash: "new_hash",
            },
          };

      const [report, plan] = await Promise.all([
        forgeService.simulateSemanticChange(semanticChange),
        forgeService.createSemanticPlan(".", semanticChange).catch(() => null),
      ]);

      setImpactReport(report);
      setExecutionPlan(plan);

      if (report && report.direct_dependents) {
        const dependentNames = [
          ...report.direct_dependents.map((d: ImpactedTarget) => d.symbol_name),
          ...report.indirect_dependents.map((d: ImpactedTarget) => d.symbol_name),
        ];
        window.dispatchEvent(
          new CustomEvent("vora:highlight-impact-nodes", {
            detail: { dependents: dependentNames },
          })
        );
      }

      setAgentLogs((prev) => [
        ...prev,
        `[Simulation Complete 📊] Impacted Symbols: ${report.total_impacted_count}, Affected Tests: ${report.affected_test_files.length}`,
      ]);
    } catch (err: any) {
      setAgentLogs((prev) => [
        ...prev,
        `[Simulation Error ❌] ${err?.toString() || err}`,
      ]);
    } finally {
      setIsProcessing(false);
    }
  };

  /**
   * P10.5 & P11 & P12: اجرای گردش‌کار تمام‌خودکار چندفایلی فقط با یک پرامپت خام متنی (One-Click Autonomous Pipeline)
   */
  const handleRunFullAutonomousTask = async () => {
    if (!prompt.trim()) return;
    setIsAutonomousRunning(true);
    setCurrentAutoStepIndex(0);
    setAutoTaskSuccess(null);
    setCurrentPatch(null);

    const orchestrator = new ForgeAgentOrchestrator((log) => {
      switch (log.type) {
        case "intent_parsed":
          setCurrentAutoStepIndex(1);
          break;
        case "target_resolved":
          setCurrentAutoStepIndex(2);
          break;
        case "context_assembled":
          setCurrentAutoStepIndex(3);
          break;
        case "plan_generated":
        case "plan_repaired":
          setCurrentAutoStepIndex(4);
          break;
        case "patch_generated":
          setCurrentAutoStepIndex(5);
          break;
        case "simulation_started":
        case "candidate_testing":
          setCurrentAutoStepIndex(6);
          break;
        case "tests_passed":
          setCurrentAutoStepIndex(7);
          break;
        case "transaction_committed":
          setCurrentAutoStepIndex(8);
          break;
        case "git_committed":
          setCurrentAutoStepIndex(9);
          break;
        case "post_commit_verified":
        case "task_completed":
          setCurrentAutoStepIndex(10);
          break;
      }

      const prefix =
        log.type === "think"
          ? "[Think 🧠]"
          : log.type === "intent_parsed"
          ? "[Intent 🎯]"
          : log.type === "target_resolved"
          ? "[Target 🔍]"
          : log.type === "context_assembled"
          ? "[Context 🧩]"
          : log.type === "plan_generated"
          ? "[Planner 📋]"
          : log.type === "plan_repaired"
          ? "[Plan Repair 🔄]"
          : log.type === "patch_generated"
          ? "[Synthesizer ⚡]"
          : log.type === "simulation_started"
          ? "[Preflight 🛡️]"
          : log.type === "candidate_testing"
          ? "[Test Gate 🧪]"
          : log.type === "tests_passed"
          ? "[Tests Passed ✅]"
          : log.type === "self_healing_attempt"
          ? "[Self-Healing 🩹]"
          : log.type === "transaction_committed"
          ? "[Atomic Commit 🔒]"
          : log.type === "git_committed"
          ? "[Git SSOT 🚀]"
          : log.type === "post_commit_verified"
          ? "[Verified 🛡️]"
          : log.type === "task_completed"
          ? "[Done 🎉]"
          : "[Error ❌]";

      setAgentLogs((prev) => [...prev, `${prefix} ${log.message}`]);
    });

    try {
      const res: AutonomousTaskResult = await orchestrator.runAutonomousTask(prompt);
      setAutoTaskSuccess(res.success);

      // ثبت تسک در سابقه اجرایی با فیلدهای جامع Audit Trail (A14)
      recordExecutedTask({
        taskId: `auto_task_${Date.now()}`,
        intent: prompt,
        targetSymbol: res.targetSymbol,
        targetFile: res.targetFile,
        canonicalEntityId: res.canonicalEntityId,
        snapshotId: res.snapshotId,
        gitCommitHash: res.gitCommitHash,
        success: res.success,
        patchedFilesCount: res.executionPlan?.steps?.length || 1,
        healingAttempts: res.healingAttempts || 0,
      });

      if (res.success) {
        setTargetSymbol(res.targetSymbol);
        setFilePath(res.targetFile);
        setNewSnippet(res.patchedSnippet);
        setExecutionPlan(res.executionPlan);
        setImpactReport(res.impactReport);
        await fetchCanonicalEntities();

        window.dispatchEvent(
          new CustomEvent("vora:update-canvas-ast-node", {
            detail: {
              symbolName: res.targetSymbol,
              signature: res.patchedSnippet,
              status: "committed",
            },
          })
        );
      }
    } finally {
      setIsAutonomousRunning(false);
    }
  };

  const handleRunIntentOrchestrator = async () => {
    setIsProcessing(true);
    setCurrentPatch(null);

    const orchestrator = new ForgeAgentOrchestrator((log) => {
      const prefix =
        log.type === "think"
          ? "[Think 🧠]"
          : log.type === "tool_call"
          ? "[Tool Call ⚡]"
          : log.type === "tool_result"
          ? "[Tool Result 📥]"
          : log.type === "patch_ready"
          ? "[Patch Ready 🎯]"
          : "[Error ❌]";

      setAgentLogs((prev) => [...prev, `${prefix} ${log.message}`]);
    });

    const proposal = await orchestrator.runIntent(
      filePath,
      prompt,
      targetSymbol,
      newSnippet
    );

    setCurrentPatch(proposal);

    if (proposal && proposal.dependents) {
      window.dispatchEvent(
        new CustomEvent("vora:highlight-impact-nodes", {
          detail: { dependents: proposal.dependents },
        })
      );
    }

    setIsProcessing(false);
  };

  const handleApplyPatchToSource = async () => {
    if (!currentPatch) return;
    setIsProcessing(true);

    setAgentLogs((prev) => [
      ...prev,
      `[Safety Guard 🛡️] Executing project unit tests before applying patch...`,
    ]);

    try {
      const testRes = await forgeService.runProjectTests();
      if (!testRes.success) {
        setAgentLogs((prev) => [
          ...prev,
          `[Test Failed ❌] Aborting commit! Unit tests failed:\n${testRes.output.slice(0, 300)}...`,
        ]);
        setIsProcessing(false);
        return;
      }

      setAgentLogs((prev) => [
        ...prev,
        `[Tests Passed ✅] All tests green. Proceeding with atomic transaction execution...`,
      ]);

      const txResult = await forgeService.executePatchTransaction({
        baseCommit: "HEAD",
        taskId: `task_${Date.now()}`,
        filePath: currentPatch.filePath,
        baseContent: sourceCode,
        symbolId: currentPatch.targetSymbol,
        newSymbolCode: newSnippet,
        intent: prompt,
      });

      if (!txResult.is_committed) {
        throw new Error("Transaction commit failed");
      }

      setSourceCode(currentPatch.patchedContent);
      setIsDiffModalOpen(false);

      recordExecutedTask({
        taskId: `task_${Date.now()}`,
        intent: prompt,
        targetSymbol: currentPatch.targetSymbol,
        targetFile: currentPatch.filePath,
        canonicalEntityId: currentPatch.canonicalEntityId,
        snapshotId: txResult.snapshot_id,
        gitCommitHash: "manual_commit",
        success: true,
        patchedFilesCount: 1,
        healingAttempts: 0,
      });

      setAgentLogs((prev) => [
        ...prev,
        `[Atomic Transaction Committed 🚀] Snapshot ID: ${txResult.snapshot_id || "latest"}`,
      ]);

      await fetchCanonicalEntities();

      window.dispatchEvent(
        new CustomEvent("vora:update-canvas-ast-node", {
          detail: {
            symbolName: currentPatch.targetSymbol,
            signature: newSnippet,
            status: "committed",
          },
        })
      );

      window.dispatchEvent(
        new CustomEvent("vora:highlight-impact-nodes", {
          detail: { dependents: [] },
        })
      );
    } catch (err: any) {
      setAgentLogs((prev) => [
        ...prev,
        `[Error ❌] Operation failed: ${err?.toString() || err}`,
      ]);
    } finally {
      setIsProcessing(false);
    }
  };

  const handleGenerateDocCard = async () => {
    if (!targetSymbol) return;
    setIsProcessing(true);
    setAgentLogs((prev) => [
      ...prev,
      `[Mind Bridge 🧠] Generating documentation card for '${targetSymbol}'...`,
    ]);

    try {
      const doc = await forgeLLMService.generateSymbolDocumentation(
        targetSymbol,
        newSnippet || prompt,
        filePath
      );

      const cardId = addCard();
      const anyStore = store as any;
      if (typeof anyStore.updateCardContent === "function") {
        anyStore.updateCardContent(cardId, doc.title, doc.content);
      }

      const cardNodeId = `node-${cardId}-${Date.now()}`;
      const docNode = {
        id: cardNodeId,
        type: "voraCard",
        position: { x: 750, y: 200 },
        data: {
          label: doc.title,
          title: doc.title,
          content: doc.content,
          cardId,
          version: "v0.1.0",
        },
      };

      const docEdge = {
        id: `edge-doc-${targetSymbol}-${cardId}`,
        source: `ast-${targetSymbol}`,
        target: cardNodeId,
        type: "smoothstep",
        label: "documents",
        labelStyle: { fill: "#38bdf8", fontSize: 10, fontFamily: "monospace" },
        style: { stroke: "#38bdf8", strokeWidth: 1.5, strokeDasharray: "5 5" },
        animated: true,
      };

      onInjectGraphToCanvas({
        nodes: [docNode],
        edges: [docEdge],
      });

      setAgentLogs((prev) => [
        ...prev,
        `[Doc Created 📄] Linked '${targetSymbol}' to Mind Knowledge Card: "${doc.title}".`,
      ]);
    } catch (err: any) {
      setAgentLogs((prev) => [
        ...prev,
        `[Error ❌] Failed to create doc card: ${err?.toString() || err}`,
      ]);
    } finally {
      setIsProcessing(false);
    }
  };

  const handleTriggerTimeTravel = (snapshotId: string) => {
    window.dispatchEvent(
      new CustomEvent("vora:time-travel-snapshot", {
        detail: { snapshotId },
      })
    );
    setAgentLogs((prev) => [
      ...prev,
      `[Time Travel ⏳] Canvas state navigated to Snapshot: ${snapshotId}`,
    ]);
  };

  const handleRollbackDiskToSnapshot = async (snapshotId: string) => {
    setIsProcessing(true);
    setAgentLogs((prev) => [
      ...prev,
      `[Disk Rollback 🔄] Restoring files from Snapshot '${snapshotId}'...`,
    ]);

    try {
      const restored = await forgeService.rollbackToSnapshot(snapshotId);
      setAgentLogs((prev) => [
        ...prev,
        `[Rollback Success ✅] Restored ${restored.length} file(s) on disk to snapshot state.`,
      ]);

      handleTriggerTimeTravel(snapshotId);
      await fetchCanonicalEntities();
    } catch (err: any) {
      setAgentLogs((prev) => [
        ...prev,
        `[Rollback Error ❌] ${err?.toString() || err}`,
      ]);
    } finally {
      setIsProcessing(false);
    }
  };

  return (
    <>
      <aside className="w-[30rem] flex-shrink-0 bg-[#0e1015] flex flex-col z-30 shadow-2xl transition-all border-l border-[#1f232e] select-none text-gray-200">
        <div className="h-14 px-5 flex items-center justify-between border-b border-[#1b1f2b] bg-[#0b0c10]">
          <div className="flex items-center gap-2.5">
            <Sparkles size={16} className="text-violet-400" />
            <span className="text-sm font-semibold tracking-wide text-zinc-100">
              Forge AI Agent
            </span>
            <span className="text-[10px] font-mono tracking-wider bg-violet-500/10 text-violet-300 border border-violet-500/20 px-2 py-0.5 rounded-full font-medium shadow-sm">
              P12 Orchestrator
            </span>
          </div>
          <div className="flex items-center gap-1">
            <button
              onClick={() => setIsSettingsModalOpen(true)}
              className="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-[#181b24] transition"
              title="LLM Provider Settings"
            >
              <Settings size={15} />
            </button>
            <button
              onClick={toggleForgeAgent}
              className="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-[#181b24] transition"
              title="Close Agent"
            >
              <X size={16} />
            </button>
          </div>
        </div>

        <div className="flex border-b border-[#1b1f2b] bg-[#0d0f14] p-1.5 gap-1.5">
          <button
            onClick={() => setActiveTab("intent")}
            className={`flex-1 py-1.5 text-xs font-medium rounded-lg transition flex items-center justify-center gap-1.5 ${
              activeTab === "intent"
                ? "bg-violet-600/15 text-violet-300 border border-violet-500/30 shadow-sm"
                : "text-zinc-400 hover:text-zinc-200 hover:bg-[#161822]"
            }`}
          >
            <Cpu size={13.5} />
            <span>Autonomous</span>
          </button>
          <button
            onClick={() => setActiveTab("graph")}
            className={`flex-1 py-1.5 text-xs font-medium rounded-lg transition flex items-center justify-center gap-1.5 ${
              activeTab === "graph"
                ? "bg-violet-600/15 text-violet-300 border border-violet-500/30 shadow-sm"
                : "text-zinc-400 hover:text-zinc-200 hover:bg-[#161822]"
            }`}
          >
            <Workflow size={13.5} />
            <span>Graph</span>
          </button>
          <button
            onClick={() => setActiveTab("history")}
            className={`flex-1 py-1.5 text-xs font-medium rounded-lg transition flex items-center justify-center gap-1.5 ${
              activeTab === "history"
                ? "bg-violet-600/15 text-violet-300 border border-violet-500/30 shadow-sm"
                : "text-zinc-400 hover:text-zinc-200 hover:bg-[#161822]"
            }`}
          >
            <History size={13.5} />
            <span>History ({executedTasks.length})</span>
          </button>
        </div>

        <div className="flex-1 overflow-y-auto no-scrollbar p-4 flex flex-col gap-3.5">
          {activeTab === "intent" ? (
            <>
              {/* پرامپت خام و اجرای تمام‌خودکار (P10.5 & P12 Raw Autonomous Runner) */}
              <div className="p-3 bg-gradient-to-b from-[#151322] to-[#0f1118] border border-violet-500/30 rounded-xl flex flex-col gap-2.5 shadow-lg shadow-violet-950/20">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-violet-200 flex items-center gap-1.5">
                    <Bot size={15} className="text-violet-400" />
                    Autonomous Multi-File Engine
                  </span>
                  <span className="text-[9px] font-mono bg-violet-950/80 text-violet-300 border border-violet-500/40 px-1.5 py-0.2 rounded">
                    N-Tier Cascade
                  </span>
                </div>
                <textarea
                  rows={3}
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                  placeholder="e.g. Rename calculate_tax to compute_vat and update all callers and imports across project..."
                  className="w-full bg-[#0a0c12] border border-[#262c3e] focus:border-violet-500/60 rounded-lg p-2.5 text-xs text-zinc-100 focus:outline-none resize-none leading-relaxed"
                />

                <button
                  onClick={handleRunFullAutonomousTask}
                  disabled={isAutonomousRunning || isProcessing || !prompt.trim()}
                  className="w-full py-2 bg-gradient-to-r from-violet-600 via-indigo-600 to-purple-600 hover:from-violet-500 hover:to-purple-500 active:opacity-90 disabled:opacity-50 text-white rounded-lg text-xs font-bold transition flex items-center justify-center gap-1.5 shadow-md shadow-violet-950/50"
                >
                  {isAutonomousRunning ? (
                    <Loader2 size={14} className="animate-spin text-white" />
                  ) : (
                    <Sparkles size={14} className="text-violet-200 animate-pulse" />
                  )}
                  <span>
                    {isAutonomousRunning
                      ? "Autonomous Engine Executing..."
                      : "Run Autonomous Task (Plan ➔ Cascade ➔ Test ➔ Commit)"}
                  </span>
                </button>

                {/* نمایشگر زنده مراحل ۱۰ گانه استپر خودکار (P12 Stepper & Telemetry) */}
                {currentAutoStepIndex >= 0 && (
                  <div className="pt-2 border-t border-[#232838] flex flex-col gap-1.5 animate-in fade-in">
                    <div className="flex items-center justify-between text-[10px] font-mono text-zinc-400">
                      <span>Pipeline & Trust Gate Progress</span>
                      <div className="flex items-center gap-1.5">
                        {autoTaskSuccess === true && (
                          <span className="text-emerald-400 flex items-center gap-0.5 text-[10px]">
                            <Check size={11} /> Success
                          </span>
                        )}
                        {autoTaskSuccess === false && (
                          <span className="text-rose-400 flex items-center gap-0.5 text-[10px]">
                            <XCircle size={11} /> Failed
                          </span>
                        )}
                        <span>
                          {Math.min(currentAutoStepIndex, AUTONOMOUS_STEPS.length)} / {AUTONOMOUS_STEPS.length}
                        </span>
                      </div>
                    </div>
                    <div className="flex flex-col gap-1">
                      {AUTONOMOUS_STEPS.map((step, sIdx) => {
                        const isDone = sIdx < currentAutoStepIndex;
                        const isCurrent = sIdx === currentAutoStepIndex && isAutonomousRunning;
                        return (
                          <div
                            key={step.id}
                            className={`flex items-center gap-2 text-[10px] font-mono px-2 py-1 rounded transition ${
                              isDone
                                ? "bg-emerald-950/40 text-emerald-300 border border-emerald-500/20"
                                : isCurrent
                                ? "bg-violet-950/60 text-violet-300 border border-violet-500/40 animate-pulse"
                                : "text-zinc-600 bg-[#0d0f14]"
                            }`}
                          >
                            {isDone ? (
                              <Check size={11} className="text-emerald-400" />
                            ) : isCurrent ? (
                              <Loader2 size={11} className="animate-spin text-violet-400" />
                            ) : (
                              <div className="w-2.5 h-2.5 rounded-full border border-zinc-700" />
                            )}
                            <span className="truncate">{step.label}</span>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}
              </div>

              {/* جستجوی سریع موجودیت‌های کانونیکال (P10.3) */}
              <div className="flex flex-col gap-1.5">
                <label className="text-xs font-medium text-zinc-300 flex items-center justify-between">
                  <div className="flex items-center gap-1.5">
                    <Search size={13.5} className="text-purple-400" />
                    <span>Search Canonical Entity Registry</span>
                  </div>
                  <span className="text-[10px] font-mono text-zinc-500">
                    {canonicalEntities.length} Registered
                  </span>
                </label>
                <div className="relative">
                  <input
                    type="text"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    placeholder="Search by symbol or namespace..."
                    className="w-full bg-[#12141c] border border-[#202433] focus:border-purple-500/50 rounded-lg px-3 py-1.5 text-xs font-mono text-zinc-200 focus:outline-none transition"
                  />
                  {filteredEntities.length > 0 && (
                    <div className="absolute top-full mt-1 left-0 right-0 max-h-40 overflow-y-auto bg-[#0e1017] border border-[#282e42] rounded-lg shadow-2xl z-50 flex flex-col">
                      {filteredEntities.map((ent) => (
                        <div
                          key={normalizeId(ent.entity_id)}
                          onClick={() => {
                            handleSearchSelect(ent);
                            setSearchQuery("");
                          }}
                          className="px-3 py-1.5 hover:bg-[#1b2030] cursor-pointer flex items-center justify-between text-xs border-b border-[#1a1e2b] last:border-none transition"
                        >
                          <span className="font-mono text-zinc-200 font-medium">
                            {ent.name}
                          </span>
                          <span className="text-[10px] text-zinc-500 font-mono">
                            {ent.module_namespace}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>

              {/* اینسپکتور موجودیت کانونیکال فعال (P10.4 & P10.5) */}
              <CanonicalEntityInspector />

              <div className="flex flex-col gap-1.5 pt-1 border-t border-[#1b1f2b]">
                <label className="text-xs font-medium text-zinc-300 flex items-center justify-between">
                  <div className="flex items-center gap-1.5">
                    <FileCode2 size={13.5} className="text-violet-400" />
                    <span>Target File Path</span>
                  </div>
                  <button
                    onClick={handleLoadProjectFile}
                    disabled={isLoadingFile}
                    className="text-[11px] text-violet-300 hover:text-violet-200 flex items-center gap-1 bg-[#161922] hover:bg-[#1f2330] px-2 py-0.5 rounded-md border border-[#252a3a] transition disabled:opacity-50"
                  >
                    <FolderOpen size={11} />
                    <span>{isLoadingFile ? "Loading..." : "Load File"}</span>
                  </button>
                </label>
                <input
                  type="text"
                  value={filePath}
                  onChange={(e) => setFilePath(e.target.value)}
                  className="w-full bg-[#12141c] border border-[#202433] focus:border-violet-500/50 rounded-lg px-3 py-1.5 text-xs font-mono text-zinc-200 focus:outline-none transition"
                />
              </div>

              <div className="flex flex-col gap-1.5">
                <label className="text-xs font-medium text-zinc-300">
                  Target AST Symbol Name
                </label>
                <input
                  type="text"
                  value={targetSymbol}
                  onChange={(e) => setTargetSymbol(e.target.value)}
                  placeholder="e.g. allocate_block"
                  className="w-full bg-[#12141c] border border-[#202433] focus:border-violet-500/50 rounded-lg px-3 py-1.5 text-xs font-mono text-zinc-200 focus:outline-none transition"
                />
              </div>

              <div className="flex flex-col gap-1.5">
                <label className="text-xs font-medium text-zinc-300">
                  New Symbol AST Replacement Snippet
                </label>
                <textarea
                  rows={4}
                  value={newSnippet}
                  onChange={(e) => setNewSnippet(e.target.value)}
                  placeholder="Leave empty to auto-generate using LLM..."
                  className="w-full bg-[#12141c] border border-[#202433] focus:border-violet-500/50 rounded-lg p-2.5 text-xs font-mono text-zinc-200 focus:outline-none resize-none leading-relaxed"
                />
              </div>

              <div className="flex gap-2 pt-1">
                <button
                  onClick={handleRunIntentOrchestrator}
                  disabled={isProcessing || isAutonomousRunning}
                  className="flex-1 py-2 bg-[#1b1f2e] hover:bg-[#23293d] border border-violet-500/30 text-violet-300 rounded-lg text-xs font-medium transition flex items-center justify-center gap-1.5 disabled:opacity-50"
                  title="Run Step-by-Step Tool Loop"
                >
                  {isProcessing ? (
                    <GitGraph size={14} className="animate-spin" />
                  ) : (
                    <Play size={14} />
                  )}
                  <span>Run Tool Loop (ReAct)</span>
                </button>

                <button
                  onClick={handleSimulateSemanticBlastRadius}
                  disabled={isProcessing || isAutonomousRunning || !targetSymbol}
                  className="px-3 py-2 bg-[#161a26] hover:bg-[#1e2333] border border-[#262c3e] text-amber-300 rounded-lg text-xs font-medium transition flex items-center justify-center gap-1.5 disabled:opacity-50 shadow-sm"
                  title="Simulate Semantic Change & Blast Radius"
                >
                  <Network size={14} />
                  <span>Simulate</span>
                </button>

                <button
                  onClick={handleGenerateDocCard}
                  disabled={isProcessing || isAutonomousRunning || !targetSymbol}
                  className="px-3 py-2 bg-[#161a26] hover:bg-[#1e2333] border border-[#262c3e] text-cyan-300 rounded-lg text-xs font-medium transition flex items-center justify-center gap-1.5 disabled:opacity-50 shadow-sm"
                  title="Generate Documentation Card in Mind"
                >
                  <BookOpen size={14} />
                  <span>Doc to Card</span>
                </button>
              </div>

              {/* P12 Deep Observability: نمایش جامع درخت وابستگی آبشاری چندفایلی */}
              {impactReport && (
                <div className="p-3 bg-[#131620] border border-amber-500/30 rounded-xl flex flex-col gap-2.5 animate-in fade-in">
                  <div className="flex items-center justify-between text-xs font-medium text-amber-300">
                    <div className="flex items-center gap-1.5">
                      <Network size={14} className="text-amber-400" />
                      <span>Cascading Blast Radius Report</span>
                    </div>
                    <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-amber-950/60 text-amber-200 border border-amber-500/30">
                      Total Impact: {impactReport.total_impacted_count}
                    </span>
                  </div>
                  <div className="text-[11px] font-mono text-gray-400 flex flex-col gap-1">
                    <div>Direct Dependents (Tier 1): <span className="text-zinc-200">{impactReport.direct_dependents.length}</span></div>
                    <div>Indirect Cascade (Tier 2+): <span className="text-zinc-200">{impactReport.indirect_dependents.length}</span></div>
                    <div>Affected Test Files: <span className="text-zinc-200">{impactReport.affected_test_files.length}</span></div>
                  </div>

                  {/* بازنمایی بصری سلسله‌مراتبی گام‌های پلن N-Tier */}
                  {executionPlan && (
                    <div className="pt-2 border-t border-[#232838] flex flex-col gap-1.5">
                      <div className="text-[11px] font-medium text-violet-300 flex items-center justify-between">
                        <div className="flex items-center gap-1">
                          <Layers size={13} className="text-violet-400" />
                          <span>Multi-Tier Cascade Plan ({executionPlan.total_steps} steps)</span>
                        </div>
                        <span className="text-[9px] font-mono bg-violet-950/80 text-violet-300 px-1.5 py-0.5 rounded border border-violet-500/20">
                          Root: {executionPlan.root_symbol}
                        </span>
                      </div>

                      <div className="max-h-36 overflow-y-auto no-scrollbar flex flex-col gap-1.5">
                        {executionPlan.steps.map((step: PlanStep) => {
                          const depth = step.cascade_depth ?? 0;
                          const tierBadge =
                            depth === 0
                              ? "Tier 0 (Root)"
                              : depth === 1
                              ? "Tier 1 (Direct)"
                              : depth === 2
                              ? "Tier 2 (Interface)"
                              : `Tier ${depth} (Deep)`;

                          const tierColor =
                            depth === 0
                              ? "border-violet-500/50 bg-violet-950/40 text-violet-200"
                              : depth === 1
                              ? "border-cyan-500/40 bg-cyan-950/30 text-cyan-200"
                              : depth === 2
                              ? "border-amber-500/40 bg-amber-950/30 text-amber-200"
                              : "border-purple-500/40 bg-purple-950/30 text-purple-200";

                          return (
                            <div
                              key={step.step_order}
                              className={`text-[10px] font-mono p-2 rounded-lg border flex flex-col gap-1 ${tierColor}`}
                            >
                              <div className="flex items-center justify-between">
                                <div className="flex items-center gap-1.5 font-bold">
                                  <span>#{step.step_order}</span>
                                  <ArrowRight size={10} />
                                  <span>{step.symbol_name}</span>
                                </div>
                                <span className="text-[9px] px-1.5 py-0.2 rounded font-sans uppercase opacity-80 border border-current">
                                  {tierBadge}
                                </span>
                              </div>
                              <div className="text-[9.5px] opacity-75 truncate flex items-center justify-between">
                                <span>{step.file_path}</span>
                                <span className="italic">{step.reason}</span>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  )}
                </div>
              )}

              {currentPatch && (
                <div className="p-3 bg-[#131620] border border-violet-500/30 rounded-xl flex flex-col gap-2.5 animate-in fade-in">
                  <div className="flex items-center justify-between text-xs font-medium text-violet-300">
                    <div className="flex items-center gap-1.5">
                      <CheckCircle2 size={14} className="text-emerald-400" />
                      <span>Patch Ready ({currentPatch.originalLines})</span>
                    </div>
                    {currentPatch.dependentsCount > 0 ? (
                      <span className="flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded bg-amber-950/50 text-amber-300 border border-amber-500/30">
                        <AlertTriangle size={11} />
                        <span>Impact: {currentPatch.dependentsCount} dependents</span>
                      </span>
                    ) : (
                      <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-950/50 text-emerald-300 border border-emerald-500/30">
                        Impact: Isolated
                      </span>
                    )}
                  </div>

                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => setIsDiffModalOpen(true)}
                      className="flex-1 flex items-center justify-center gap-1.5 bg-[#1a1e2b] hover:bg-[#23283a] text-violet-300 border border-violet-500/20 text-xs font-medium py-1.5 rounded-lg transition"
                    >
                      <Eye size={13.5} />
                      <span>Diff Preview</span>
                    </button>
                    <button
                      onClick={handleApplyPatchToSource}
                      disabled={isProcessing || isAutonomousRunning}
                      className="flex-1 flex items-center justify-center gap-1.5 bg-violet-600 hover:bg-violet-500 disabled:opacity-50 text-white text-xs font-medium py-1.5 rounded-lg transition shadow-md shadow-violet-950/40"
                      title="Run unit tests and commit"
                    >
                      <ShieldCheck size={13.5} />
                      <span>Verify & Commit</span>
                    </button>
                  </div>
                </div>
              )}
            </>
          ) : activeTab === "graph" ? (
            <>
              <div className="p-3 bg-[#131620] border border-[#202534] rounded-xl flex flex-col gap-2">
                <label className="text-xs font-medium text-zinc-300 flex items-center gap-1.5">
                  <FolderTree size={13.5} className="text-indigo-400" />
                  <span>Workspace Root Directory</span>
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={workspaceDir}
                    onChange={(e) => setWorkspaceDir(e.target.value)}
                    placeholder="e.g. src-tauri/src"
                    className="flex-1 bg-[#0f1118] border border-[#202433] focus:border-indigo-500/50 rounded-lg px-3 py-1 text-xs font-mono text-zinc-200 focus:outline-none transition"
                  />
                  <button
                    onClick={handleScanEntireWorkspace}
                    disabled={isScanningWorkspace}
                    className="px-3 py-1 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded-lg text-xs font-medium transition flex items-center gap-1.5 shadow-sm"
                  >
                    <FolderTree size={12} />
                    <span>{isScanningWorkspace ? "Scanning..." : "Scan All"}</span>
                  </button>
                </div>
              </div>

              <div className="flex items-center gap-2">
                <button
                  onClick={handleCalculateCodeChurnHeatmap}
                  disabled={isCalculatingHeatmap}
                  className="flex-1 py-1.5 bg-gradient-to-r from-rose-950/30 to-amber-950/30 hover:from-rose-900/40 hover:to-amber-900/40 border border-rose-500/20 text-rose-300 rounded-lg text-xs font-medium transition flex items-center justify-center gap-1.5 disabled:opacity-50"
                  title="Compute Git churn metrics and paint hotspot heatmap on canvas"
                >
                  <Flame size={13.5} className="text-rose-400" />
                  <span>{isCalculatingHeatmap ? "Analyzing Churn..." : "Calculate Heatmap"}</span>
                </button>
              </div>

              <div className="flex flex-col gap-1.5 flex-1">
                <label className="text-xs font-medium text-zinc-300 flex items-center gap-1.5">
                  <Workflow size={13.5} className="text-cyan-400" />
                  <span>Single File Source Code</span>
                </label>
                <textarea
                  rows={7}
                  value={sourceCode}
                  onChange={(e) => setSourceCode(e.target.value)}
                  placeholder="Paste code snippet here..."
                  className="w-full bg-[#12141c] border border-[#202433] focus:border-violet-500/50 rounded-lg p-2.5 text-xs font-mono text-zinc-200 focus:outline-none resize-none leading-relaxed"
                />
              </div>

              <button
                onClick={handleRunAstAnalysis}
                disabled={isProcessing}
                className="w-full py-2 bg-gradient-to-r from-violet-600 to-indigo-600 hover:from-violet-500 hover:to-indigo-500 active:from-violet-700 disabled:opacity-50 text-white rounded-lg text-xs font-medium transition flex items-center justify-center gap-1.5 shadow-md shadow-violet-950/40"
              >
                {isProcessing ? (
                  <GitGraph size={14} className="animate-spin" />
                ) : (
                  <Play size={14} />
                )}
                <span>{isProcessing ? "Analyzing AST..." : "Parse File & Draw Graph"}</span>
              </button>
            </>
          ) : (
            /* تب تاریخچه و تایم‌لاین وظایف اجراشده با جزئیات کامل Provenance (P12 & A14) */
            <div className="flex flex-col gap-3">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-zinc-200 flex items-center gap-1.5">
                  <History size={14} className="text-violet-400" />
                  Autonomous Execution History
                </span>
                {executedTasks.length > 0 && (
                  <button
                    onClick={clearExecutedTasks}
                    className="text-[10px] text-zinc-500 hover:text-zinc-300 transition"
                  >
                    Clear History
                  </button>
                )}
              </div>

              {executedTasks.length === 0 ? (
                <div className="p-8 text-center border border-[#1b1f2b] rounded-xl bg-[#0b0c10] text-zinc-500 text-xs flex flex-col items-center gap-2">
                  <Clock size={24} className="text-zinc-600" />
                  <span>No autonomous tasks recorded yet.</span>
                </div>
              ) : (
                <div className="flex flex-col gap-2 max-h-[28rem] overflow-y-auto no-scrollbar">
                  {executedTasks.map((t) => (
                    <div
                      key={t.id}
                      className="p-3 bg-[#11131c] border border-[#1e2332] rounded-xl flex flex-col gap-2 shadow-sm"
                    >
                      <div className="flex items-center justify-between">
                        <span className="text-xs font-medium text-zinc-200 truncate max-w-[16rem]">
                          {t.intent}
                        </span>
                        {t.success ? (
                          <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-950/60 text-emerald-300 border border-emerald-500/20 flex items-center gap-1">
                            <Check size={10} /> Done
                          </span>
                        ) : (
                          <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-rose-950/60 text-rose-300 border border-rose-500/20 flex items-center gap-1">
                            <XCircle size={10} /> Failed
                          </span>
                        )}
                      </div>

                      <div className="text-[10px] font-mono text-zinc-400 flex items-center justify-between">
                        <span>Target: {t.targetSymbol}</span>
                        <span>{new Date(t.timestamp).toLocaleTimeString()}</span>
                      </div>

                      <div className="text-[10px] font-mono text-zinc-500 truncate flex items-center justify-between">
                        <span className="truncate max-w-[15rem]">File: {t.targetFile}</span>
                        {t.canonicalEntityId && (
                          <span className="text-purple-400/80 text-[9px] font-mono">
                            {t.canonicalEntityId.slice(0, 8)}...
                          </span>
                        )}
                      </div>

                      {t.snapshotId && (
                        <div className="pt-2 border-t border-[#1b1f2b] flex items-center justify-between gap-1.5">
                          <span className="text-[10px] font-mono text-violet-300 truncate max-w-[7rem]">
                            Snap: {t.snapshotId.slice(0, 10)}...
                          </span>
                          <div className="flex items-center gap-1">
                            <button
                              onClick={() => handleTriggerTimeTravel(t.snapshotId!)}
                              className="px-2 py-0.5 bg-violet-600/20 hover:bg-violet-600/30 text-violet-300 border border-violet-500/30 rounded text-[10px] font-medium flex items-center gap-1 transition"
                              title="Travel back to this snapshot state on canvas"
                            >
                              <RotateCcw size={10} />
                              <span>View</span>
                            </button>
                            <button
                              onClick={() => handleRollbackDiskToSnapshot(t.snapshotId!)}
                              disabled={isProcessing}
                              className="px-2 py-0.5 bg-rose-950/40 hover:bg-rose-900/50 text-rose-300 border border-rose-500/30 rounded text-[10px] font-medium flex items-center gap-1 transition disabled:opacity-50"
                              title="Rollback source files on disk to this snapshot"
                            >
                              <ArchiveRestore size={10} />
                              <span>Rollback Code</span>
                            </button>
                          </div>
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          <div className="flex flex-col gap-1 mt-1">
            <div className="text-[11px] font-medium text-zinc-400 flex items-center gap-1.5">
              <Terminal size={12} className="text-zinc-400" />
              <span>Agent Orchestrator Log</span>
            </div>
            <div className="h-32 bg-[#090b0e] border border-[#181b24] rounded-lg p-2.5 overflow-y-auto no-scrollbar font-mono text-[10.5px] flex flex-col gap-1 text-zinc-400">
              {agentLogs.map((log: string, idx: number) => (
                <div
                  key={idx}
                  className={
                    log.includes("[Success]") || log.includes("[Done") || log.includes("[Git SSOT") || log.includes("[Tests Passed") || log.includes("[Atomic Commit") || log.includes("[Workspace Indexed") || log.includes("[Doc Created") || log.includes("[Heatmap Applied") || log.includes("[Rollback Success") || log.includes("[Verified")
                      ? "text-emerald-400"
                      : log.includes("[Error") || log.includes("[Test Failed") || log.includes("[Tests Failed") || log.includes("[Simulation Error") || log.includes("[Rollback Error")
                      ? "text-rose-400"
                      : log.includes("[Tool Call") || log.includes("[Safety Guard") || log.includes("[Git Churn Engine") || log.includes("[Preflight") || log.includes("[Test Gate") || log.includes("[Self-Healing") || log.includes("[Time Travel") || log.includes("[Disk Rollback")
                      ? "text-amber-300"
                      : log.includes("[Tool Result") || log.includes("[Target") || log.includes("[Planner")
                      ? "text-cyan-300"
                      : log.includes("[Think") || log.includes("[Intent") || log.includes("[Context") || log.includes("[Synthesizer") || log.includes("[Canvas Focus") || log.includes("[Mind Bridge") || log.includes("[Plan Repair") || log.includes("[ImportRewiring")
                      ? "text-violet-300 font-medium"
                      : "text-zinc-400"
                  }
                >
                  {log}
                </div>
              ))}
            </div>
          </div>
        </div>
      </aside>

      {isDiffModalOpen && currentPatch && (
        <PatchDiffModal
          patch={currentPatch}
          originalSnippet={sourceCode}
          newSnippet={newSnippet}
          isApplying={isProcessing}
          onClose={() => setIsDiffModalOpen(false)}
          onApply={handleApplyPatchToSource}
        />
      )}

      <LLMSettingsModal
        isOpen={isSettingsModalOpen}
        onClose={() => setIsSettingsModalOpen(false)}
      />
    </>
  );
};

export default ForgeAgentDrawer;