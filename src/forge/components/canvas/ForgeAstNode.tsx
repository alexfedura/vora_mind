import React, { memo } from "react";
import { Handle, Position, NodeProps } from "@xyflow/react";
import { Code2, Box, GitCommit, Sparkles, AlertTriangle, Flame, Layers, HelpCircle, Fingerprint } from "lucide-react";

export const ForgeAstNode = memo(({ id, data, selected }: NodeProps) => {
  const kind = (data?.kind as string) || "function";
  const name = (data?.name as string) || "anonymous";
  const signature = (data?.signature as string) || "";
  const filePath = (data?.filePath as string) || "";
  const status = (data?.status as string) || "clean";
  const isImpacted = Boolean(data?.isImpacted);
  const isModified = Boolean(data?.isModified);
  const isAmbiguous = Boolean(data?.isAmbiguous);
  const churnScore = Number(data?.churnScore) || 0;
  const scopePath = (data?.scopePath as string) || "";
  const canonicalEntityId = (data?.canonicalEntityId as string) || "";
  const lineageId = (data?.lineageId as string) || "";

  const isHotspot = churnScore >= 0.6;
  const isModerateChurn = churnScore >= 0.3 && churnScore < 0.6;

  const handleInspect = (e: React.MouseEvent) => {
    e.stopPropagation();
    window.dispatchEvent(
      new CustomEvent("vora:inspect-ast-node", {
        detail: {
          nodeId: id,
          name,
          kind,
          filePath,
          signature,
          scopePath,
          canonicalEntityId,
          lineageId,
          isImpacted,
          isModified,
        },
      })
    );
  };

  const isCommitted = status === "committed";

  // استایل داینامیک گرادیان و رنگ لبه نود بر اساس وضعیت‌های معنایی و Churn
  const getHeatmapBorderClass = () => {
    if (selected) return "border-purple-500 shadow-purple-500/20 ring-1 ring-purple-500/50";
    if (isImpacted) return "border-amber-500/80 shadow-amber-950/40 ring-2 ring-amber-500/40 animate-pulse";
    if (isModified) return "border-cyan-500/80 shadow-cyan-950/40 ring-2 ring-cyan-500/40";
    if (isAmbiguous) return "border-yellow-500/70 shadow-yellow-950/30 ring-1 ring-yellow-500/30";
    if (isCommitted) return "border-emerald-500/60 shadow-emerald-950/30";
    if (isHotspot) return "border-rose-500/80 shadow-rose-950/50 ring-1 ring-rose-500/40";
    if (isModerateChurn) return "border-amber-500/50 shadow-amber-950/30";
    return "border-[#252a38] hover:border-purple-500/50 hover:bg-[#171a25]";
  };

  return (
    <div
      onClick={handleInspect}
      className={`w-72 rounded-xl bg-[#141720] border p-3.5 flex flex-col gap-2.5 shadow-2xl transition-all cursor-pointer ${getHeatmapBorderClass()}`}
    >
      <Handle
        type="target"
        position={Position.Left}
        className="!w-2.5 !h-2.5 !bg-purple-500 !border-2 !border-[#141720] !-left-1"
      />
      <Handle
        type="source"
        position={Position.Right}
        className="!w-2.5 !h-2.5 !bg-purple-500 !border-2 !border-[#141720] !-right-1"
      />

      <div className="flex items-center justify-between border-b border-[#212634] pb-2">
        <div className="flex items-center gap-2 overflow-hidden">
          {kind === "function" ? (
            <Code2 size={15} className="text-cyan-400 flex-shrink-0" />
          ) : (
            <Box size={15} className="text-purple-400 flex-shrink-0" />
          )}
          <span className="text-xs font-mono font-bold text-gray-200 truncate">
            {name}
          </span>
        </div>
        <div className="flex items-center gap-1.5">
          {canonicalEntityId && (
            <span
              className="flex items-center gap-0.5 text-[10px] font-mono bg-purple-950/70 text-purple-300 border border-purple-500/40 px-1.5 py-0.5 rounded"
              title={`Canonical Entity: ${canonicalEntityId}`}
            >
              <Fingerprint size={10} className="text-purple-400" />
              <span>Entity</span>
            </span>
          )}
          {isHotspot && !isImpacted && !isModified && (
            <span
              className="flex items-center gap-0.5 text-[10px] font-mono bg-rose-950/70 text-rose-300 border border-rose-500/40 px-1.5 py-0.5 rounded"
              title={`High Git Churn Hotspot: ${(churnScore * 100).toFixed(0)}%`}
            >
              <Flame size={10} className="text-rose-400 animate-pulse" />
              <span>Hotspot</span>
            </span>
          )}
          {isModified && (
            <span className="flex items-center gap-1 text-[10px] font-mono bg-cyan-950/70 text-cyan-300 border border-cyan-500/40 px-1.5 py-0.5 rounded">
              <Layers size={10} />
              <span>Overlay</span>
            </span>
          )}
          {isImpacted && (
            <span className="flex items-center gap-1 text-[10px] font-mono bg-amber-950/70 text-amber-300 border border-amber-500/40 px-1.5 py-0.5 rounded animate-bounce">
              <AlertTriangle size={10} />
              <span>Impacted</span>
            </span>
          )}
          {isAmbiguous && (
            <span className="flex items-center gap-1 text-[10px] font-mono bg-yellow-950/60 text-yellow-300 border border-yellow-500/40 px-1.5 py-0.5 rounded">
              <HelpCircle size={10} />
              <span>Ambiguous</span>
            </span>
          )}
          {isCommitted && !isImpacted && !isModified && (
            <span className="flex items-center gap-1 text-[10px] font-mono bg-emerald-950/60 text-emerald-300 border border-emerald-500/40 px-1.5 py-0.5 rounded animate-pulse">
              <GitCommit size={10} />
              <span>Committed</span>
            </span>
          )}
          <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-[#1e2330] text-gray-400 border border-[#2b3244]">
            {kind}
          </span>
        </div>
      </div>

      {signature && (
        <div className="bg-[#0c0e12] p-2 rounded-lg border border-[#1b1f2b] font-mono text-[11px] text-gray-300 overflow-x-auto no-scrollbar">
          {signature}
        </div>
      )}

      <div className="flex items-center justify-between text-[10px] text-gray-500 font-mono pt-1">
        <span className="truncate max-w-[170px]" title={filePath}>
          {filePath}
        </span>
        <div className="flex items-center gap-1 text-purple-400/80 hover:text-purple-300">
          <Sparkles size={11} />
          <span>Refactor</span>
        </div>
      </div>
    </div>
  );
});

ForgeAstNode.displayName = "ForgeAstNode";
export default ForgeAstNode;