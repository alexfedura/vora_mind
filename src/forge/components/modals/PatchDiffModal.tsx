import React from "react";
import { X, Check, FileDiff, ShieldAlert, GitCommit, Network, Fingerprint, Layers, ShieldCheck, Cpu } from "lucide-react";
import { PatchProposal } from "@/forge/services/agentOrchestrator";

interface PatchDiffModalProps {
  patch: PatchProposal | null;
  originalSnippet: string;
  newSnippet: string;
  onClose: () => void;
  onApply: () => void;
  isApplying?: boolean;
}

export const PatchDiffModal: React.FC<PatchDiffModalProps> = ({
  patch,
  originalSnippet,
  newSnippet,
  onClose,
  onApply,
  isApplying = false,
}) => {
  if (!patch) return null;

  // تشخیص نوع تغییر معنایی جهت نمایش دسته‌بندی دقیق به کاربر
  const isSignatureChanged = Boolean(
    newSnippet.trim() &&
      originalSnippet.trim() &&
      !originalSnippet.split("{")[0]?.trim().includes(newSnippet.split("{")[0]?.trim())
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm animate-in fade-in duration-150 select-none">
      <div className="w-[54rem] max-h-[90vh] bg-[#12141a] border border-[#262b37] rounded-2xl shadow-2xl flex flex-col overflow-hidden text-gray-200">
        {/* Header */}
        <div className="h-14 px-5 flex items-center justify-between border-b border-[#20242f] bg-[#0e1014]">
          <div className="flex items-center gap-2.5">
            <FileDiff size={18} className="text-purple-400" />
            <span className="text-sm font-bold text-white">Semantic Patch Diff Preview</span>
            <span className="text-[11px] font-mono bg-[#1c202b] text-purple-300 border border-purple-500/30 px-2 py-0.5 rounded">
              {patch.targetSymbol} ({patch.originalLines})
            </span>
            <span className="flex items-center gap-1 text-[10px] font-mono bg-indigo-950/60 text-indigo-300 border border-indigo-500/30 px-2 py-0.5 rounded">
              <Cpu size={11} />
              <span>{isSignatureChanged ? "SignatureChanged" : "BodyChanged"}</span>
            </span>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-gray-400 hover:text-white hover:bg-[#1c202a] transition"
            title="Close"
          >
            <X size={17} />
          </button>
        </div>

        {/* Content Comparison */}
        <div className="flex-1 p-5 overflow-y-auto flex flex-col gap-4 font-mono text-xs">
          {/* Target & Semantic Blast Radius Summary */}
          <div className="flex flex-wrap items-center justify-between gap-2 p-3 bg-[#171a23] border border-[#252b3b] rounded-xl">
            <div className="text-[11px] text-gray-400 font-sans flex items-center gap-2">
              <span>Target File:</span>
              <span className="text-gray-100 font-mono bg-[#0d0f15] px-2 py-0.5 rounded border border-[#222737]">
                {patch.filePath}
              </span>
            </div>
            <div className="flex items-center gap-3 text-[11px] font-sans">
              <span className="flex items-center gap-1 text-amber-400 font-semibold">
                <Network size={14} />
                Dependents: {patch.dependentsCount}
              </span>
              {patch.dependentsCount > 0 ? (
                <span className="flex items-center gap-1 text-purple-300 bg-purple-950/50 border border-purple-500/30 px-2 py-0.5 rounded">
                  <ShieldAlert size={12} />
                  Impact Radius Active
                </span>
              ) : (
                <span className="flex items-center gap-1 text-emerald-400 bg-emerald-950/50 border border-emerald-500/30 px-2 py-0.5 rounded">
                  <ShieldCheck size={12} />
                  Zero Impact Isolation
                </span>
              )}
            </div>
          </div>

          {/* Lineage & Invariant Guarantee Badge */}
          <div className="flex items-center justify-between p-2.5 bg-[#10131b] border border-purple-500/20 rounded-xl text-[11px] font-sans text-zinc-300">
            <div className="flex items-center gap-1.5 font-mono">
              <Fingerprint size={13} className="text-purple-400" />
              <span className="text-zinc-400">Canonical Identity:</span>
              <span className="text-purple-300">Preserved across transaction</span>
            </div>
            <div className="flex items-center gap-1 text-emerald-400 font-mono text-[10px]">
              <Layers size={11} />
              <span>Surgical Overlay Sandbox Ready</span>
            </div>
          </div>

          {/* Dependents Tags */}
          {patch.dependents.length > 0 && (
            <div className="p-2.5 bg-[#141720] border border-[#202533] rounded-xl flex flex-col gap-1.5">
              <span className="text-[10px] font-sans text-gray-400 font-semibold uppercase tracking-wider flex items-center gap-1">
                <Network size={12} className="text-amber-400" />
                Downstream Impacted Symbols:
              </span>
              <div className="flex flex-wrap gap-1.5 max-h-20 overflow-y-auto">
                {patch.dependents.map((dep, idx) => (
                  <span
                    key={idx}
                    className="text-[10px] bg-[#1d2230] text-purple-200 border border-purple-500/20 px-2 py-0.5 rounded font-mono"
                  >
                    {dep}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Side-by-side Code Comparison */}
          <div className="grid grid-cols-2 gap-3 flex-1 min-h-[260px] select-text">
            {/* Original Block */}
            <div className="flex flex-col rounded-xl border border-rose-500/30 bg-[#160e12] overflow-hidden">
              <div className="px-3 py-1.5 bg-rose-950/40 border-b border-rose-500/20 text-rose-300 font-sans font-bold flex items-center justify-between text-[11px]">
                <span>Original Code Snapshot</span>
                <span className="font-mono text-[10px] text-rose-400/80">({patch.originalLines})</span>
              </div>
              <pre className="p-3.5 overflow-x-auto text-rose-200/90 whitespace-pre-wrap leading-relaxed">
                {originalSnippet || "// Original code block"}
              </pre>
            </div>

            {/* New Patched Block */}
            <div className="flex flex-col rounded-xl border border-emerald-500/30 bg-[#0e1612] overflow-hidden">
              <div className="px-3 py-1.5 bg-emerald-950/40 border-b border-emerald-500/20 text-emerald-300 font-sans font-bold flex items-center justify-between text-[11px]">
                <span>Patched AST Node (Syntax Verified)</span>
                <span className="font-mono text-[10px] text-emerald-400/80">Tree-sitter Ready</span>
              </div>
              <pre className="p-3.5 overflow-x-auto text-emerald-200/90 whitespace-pre-wrap leading-relaxed">
                {newSnippet}
              </pre>
            </div>
          </div>
        </div>

        {/* Footer Actions */}
        <div className="h-14 px-5 bg-[#0e1014] border-t border-[#20242f] flex items-center justify-between">
          <button
            onClick={onClose}
            className="px-4 py-2 rounded-xl text-xs font-semibold text-gray-400 hover:text-white hover:bg-[#1a1d26] transition"
          >
            Cancel
          </button>
          <button
            onClick={onApply}
            disabled={isApplying}
            className="px-4 py-2 bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-500 hover:to-indigo-500 disabled:opacity-50 text-white rounded-xl text-xs font-bold transition flex items-center gap-1.5 shadow-lg shadow-purple-600/30"
          >
            {isApplying ? <GitCommit size={14} className="animate-spin" /> : <Check size={14} />}
            <span>{isApplying ? "Committing Transaction..." : "Confirm & Apply Atomic Patch"}</span>
          </button>
        </div>
      </div>
    </div>
  );
};

export default PatchDiffModal;