import React, { useEffect, useState } from "react";
import {
  Fingerprint,
  GitBranch,
  Network,
  Code2,
  FileCode,
  Layers,
  AlertTriangle,
  History,
  ShieldCheck,
  Zap,
  ListOrdered,
} from "lucide-react";
import { useAppStore } from "@/shared/store/useAppStore";
import {
  normalizeId,
  forgeService,
  AuditTrailRecord,
} from "@/forge/services/forgeService";

export const CanonicalEntityInspector: React.FC = () => {
  const {
    activeCanonicalEntity,
    activeLineageHistory,
    activeImpactReport,
    inspectImpactForSymbol,
  } = useAppStore();

  const [activeTab, setActiveTab] = useState<"inspector" | "audit">("inspector");
  const [auditLogs, setAuditLogs] = useState<AuditTrailRecord[]>([]);
  const [loadingAudit, setLoadingAudit] = useState<boolean>(false);

  const entityIdStr = normalizeId(activeCanonicalEntity?.entity_id);
  const lineageIdStr = normalizeId(activeCanonicalEntity?.lineage_id);
  const symbolIdStr = normalizeId(activeCanonicalEntity?.symbol_id);
  const fileIdStr = normalizeId(activeCanonicalEntity?.file_id);

  const loadAuditHistory = async () => {
    if (!symbolIdStr && !activeCanonicalEntity?.name) return;
    setLoadingAudit(true);
    try {
      const records = await forgeService.getTaskAuditHistory(
        activeCanonicalEntity?.name || symbolIdStr
      );
      setAuditLogs(records);
    } catch (err) {
      console.error("Failed to load audit history:", err);
    } finally {
      setLoadingAudit(false);
    }
  };

  useEffect(() => {
    if (activeTab === "audit" && activeCanonicalEntity) {
      loadAuditHistory();
    }
  }, [activeTab, activeCanonicalEntity]);

  if (!activeCanonicalEntity) {
    return (
      <div className="p-4 bg-[#11131a] border border-[#1f2433] rounded-xl flex flex-col items-center justify-center text-center gap-2 py-8 text-zinc-500">
        <Fingerprint size={28} className="text-zinc-600 animate-pulse" />
        <span className="text-xs font-medium text-zinc-400">
          No Canonical Entity Selected
        </span>
        <span className="text-[11px] text-zinc-500 max-w-[220px]">
          Click any AST node on the canvas or search a symbol to inspect its persistent lineage and blast radius.
        </span>
      </div>
    );
  }

  return (
    <div className="bg-[#11131a] border border-purple-500/30 rounded-xl p-3.5 flex flex-col gap-3 shadow-xl animate-in fade-in select-text">
      {/* سوییچر تب‌های Inspector و Audit Trail */}
      <div className="flex items-center justify-between border-b border-[#212638] pb-2">
        <div className="flex items-center gap-1.5 bg-[#0c0d12] p-0.5 rounded-lg border border-[#1d212f]">
          <button
            onClick={() => setActiveTab("inspector")}
            className={`px-2 py-0.5 rounded text-[10px] font-mono transition-colors ${
              activeTab === "inspector"
                ? "bg-purple-950/80 text-purple-300 font-bold border border-purple-500/30"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            Entity & Lineage
          </button>
          <button
            onClick={() => setActiveTab("audit")}
            className={`px-2 py-0.5 rounded text-[10px] font-mono transition-colors ${
              activeTab === "audit"
                ? "bg-amber-950/80 text-amber-300 font-bold border border-amber-500/30"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            Audit Trail
          </button>
        </div>

        <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-purple-950/70 text-purple-300 border border-purple-500/30">
          {activeCanonicalEntity.kind}
        </span>
      </div>

      {activeTab === "inspector" ? (
        <>
          {/* هدر هویت پایدار کانونیکال */}
          <div className="flex items-start justify-between">
            <div className="flex flex-col gap-0.5 overflow-hidden">
              <div className="flex items-center gap-1.5">
                <Fingerprint size={14} className="text-purple-400 flex-shrink-0" />
                <span className="text-xs font-mono font-bold text-zinc-100 truncate">
                  {activeCanonicalEntity.name}
                </span>
              </div>
              <span className="text-[10px] font-mono text-purple-300/80 truncate">
                {activeCanonicalEntity.module_namespace}
              </span>
            </div>
          </div>

          {/* شناسه‌ها و متادیتا */}
          <div className="grid grid-cols-2 gap-2 text-[10px] font-mono bg-[#0c0d12] p-2 rounded-lg border border-[#1d212f]">
            <div className="flex flex-col gap-0.5">
              <span className="text-zinc-500 flex items-center gap-1">
                <Fingerprint size={10} /> Entity ID
              </span>
              <span className="text-zinc-300 truncate" title={entityIdStr}>
                {entityIdStr.slice(0, 14)}...
              </span>
            </div>
            <div className="flex flex-col gap-0.5">
              <span className="text-zinc-500 flex items-center gap-1">
                <GitBranch size={10} /> Lineage ID
              </span>
              <span className="text-zinc-300 truncate" title={lineageIdStr}>
                {lineageIdStr.slice(0, 14)}...
              </span>
            </div>
            <div className="flex flex-col gap-0.5">
              <span className="text-zinc-500 flex items-center gap-1">
                <Code2 size={10} /> Symbol ID
              </span>
              <span className="text-zinc-300 truncate" title={symbolIdStr}>
                {symbolIdStr.slice(0, 14)}...
              </span>
            </div>
            <div className="flex flex-col gap-0.5">
              <span className="text-zinc-500 flex items-center gap-1">
                <FileCode size={10} /> File ID
              </span>
              <span className="text-zinc-300 truncate" title={fileIdStr}>
                {fileIdStr.slice(0, 14)}...
              </span>
            </div>
          </div>

          {/* امضای کانونیکال ساختاریافته */}
          <div className="flex flex-col gap-1">
            <span className="text-[11px] font-medium text-zinc-400 flex items-center gap-1">
              <Layers size={11} className="text-cyan-400" />
              Canonical Signature
            </span>
            <div className="bg-[#090a0e] p-2 rounded-lg border border-[#1b1f2b] font-mono text-[11px] text-cyan-200 overflow-x-auto no-scrollbar">
              {activeCanonicalEntity.canonical_signature}
            </div>
          </div>

          {/* خط زمانی تبار (Lineage Timeline) */}
          <div className="flex flex-col gap-1.5 pt-1 border-t border-[#1f2434]">
            <div className="flex items-center justify-between text-[11px] font-medium text-zinc-400">
              <div className="flex items-center gap-1">
                <History size={12} className="text-violet-400" />
                <span>Evolution Timeline ({activeLineageHistory.length})</span>
              </div>
              <span className="text-[10px] font-mono text-emerald-400 flex items-center gap-0.5">
                <ShieldCheck size={10} /> Invariant Preserved
              </span>
            </div>

            {activeLineageHistory.length > 0 ? (
              <div className="max-h-24 overflow-y-auto no-scrollbar flex flex-col gap-1">
                {activeLineageHistory.map((item, idx) => (
                  <div
                    key={idx}
                    className="text-[10px] font-mono bg-[#0c0d12] p-1.5 rounded border border-[#1c202d] flex items-center justify-between text-zinc-300"
                  >
                    <div className="flex items-center gap-1.5 truncate">
                      <span className="text-violet-400">#{idx + 1}</span>
                      <span className="text-zinc-400 truncate">
                        Snap: {item.snapshot_id.slice(0, 8)}...
                      </span>
                    </div>
                    {item.confidence !== null && (
                      <span className="text-[9px] px-1 py-0.2 bg-emerald-950/60 text-emerald-300 border border-emerald-500/30 rounded">
                        {(item.confidence * 100).toFixed(0)}% Match
                      </span>
                    )}
                  </div>
                ))}
              </div>
            ) : (
              <span className="text-[10px] font-mono text-zinc-500 italic">
                Single Snapshot (Root Generation)
              </span>
            )}
          </div>

          {/* گزارش شعاع اثر (Blast Radius) */}
          <div className="flex flex-col gap-1.5 pt-1 border-t border-[#1f2434]">
            <div className="flex items-center justify-between text-[11px] font-medium text-zinc-400">
              <div className="flex items-center gap-1">
                <Network size={12} className="text-amber-400" />
                <span>Blast Radius & Dependents</span>
              </div>
              <button
                onClick={() => inspectImpactForSymbol(symbolIdStr)}
                className="text-[10px] text-amber-300 hover:text-amber-200 bg-amber-950/40 hover:bg-amber-900/40 border border-amber-500/30 px-1.5 py-0.5 rounded transition flex items-center gap-0.5"
              >
                <Zap size={9} />
                <span>Refresh</span>
              </button>
            </div>

            {activeImpactReport ? (
              <div className="grid grid-cols-3 gap-1 text-[10px] font-mono text-center">
                <div className="bg-[#0c0d12] p-1.5 rounded border border-[#1b1f2b]">
                  <div className="text-amber-300 font-bold">
                    {activeImpactReport.direct_dependents.length}
                  </div>
                  <div className="text-zinc-500 text-[9px]">Direct</div>
                </div>
                <div className="bg-[#0c0d12] p-1.5 rounded border border-[#1b1f2b]">
                  <div className="text-amber-300 font-bold">
                    {activeImpactReport.indirect_dependents.length}
                  </div>
                  <div className="text-zinc-500 text-[9px]">Indirect</div>
                </div>
                <div className="bg-[#0c0d12] p-1.5 rounded border border-[#1b1f2b]">
                  <div className="text-amber-300 font-bold">
                    {activeImpactReport.affected_test_files.length}
                  </div>
                  <div className="text-zinc-500 text-[9px]">Tests</div>
                </div>
              </div>
            ) : (
              <div className="flex items-center gap-1 text-[10px] font-mono text-amber-400/80 bg-amber-950/20 p-1.5 rounded border border-amber-500/20">
                <AlertTriangle size={11} />
                <span>Click Refresh to calculate live dependency graph</span>
              </div>
            )}
          </div>
        </>
      ) : (
        /* تب نمایش لاگ‌های ردیابی تسک (Audit Trail) */
        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between text-[11px] font-medium text-zinc-400">
            <span className="flex items-center gap-1">
              <ListOrdered size={12} className="text-amber-400" />
              Execution Logs ({auditLogs.length})
            </span>
            <button
              onClick={loadAuditHistory}
              className="text-[10px] text-amber-300 hover:text-amber-200 bg-amber-950/40 hover:bg-amber-900/40 border border-amber-500/30 px-1.5 py-0.5 rounded transition flex items-center gap-0.5"
            >
              <Zap size={9} />
              <span>Sync</span>
            </button>
          </div>

          {loadingAudit ? (
            <div className="text-center py-4 text-zinc-500 text-[10px] font-mono">
              Loading Audit Trail...
            </div>
          ) : auditLogs.length === 0 ? (
            <div className="text-center py-4 text-zinc-500 text-[10px] font-mono italic">
              No audit logs recorded for this entity.
            </div>
          ) : (
            <div className="max-h-60 overflow-y-auto no-scrollbar flex flex-col gap-1.5">
              {auditLogs.map((log) => (
                <div
                  key={log.id}
                  className="bg-[#0c0d12] p-2 rounded-lg border border-[#1d212f] flex flex-col gap-1 font-mono text-[10px]"
                >
                  <div className="flex items-center justify-between">
                    <span className="text-amber-400 font-bold">[{log.status}]</span>
                    <span className="text-zinc-500 text-[9px]">
                      {new Date(log.created_at * 1000).toLocaleTimeString()}
                    </span>
                  </div>
                  <div className="text-zinc-300 truncate">{log.intent}</div>
                  <div className="text-zinc-500 text-[9px] flex gap-2">
                    <span>Attempts: {log.healingAttempts}</span>
                    {log.committed_snapshot_id && (
                      <span>Snap: {log.committed_snapshot_id.slice(0, 8)}...</span>
                    )}
                  </div>
                  {log.error_payload && (
                    <div className="text-red-400 bg-red-950/30 p-1 rounded text-[9px] break-words mt-0.5">
                      {log.error_payload}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};