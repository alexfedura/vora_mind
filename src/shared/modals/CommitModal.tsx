import React, { useState } from "react";
import { GitCommit, X, Sparkles } from "lucide-react";

interface CommitModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCommit: (name: string, reason: string, summary: string) => Promise<void>;
  currentSnapshotDepth: number;
}

export const CommitModal: React.FC<CommitModalProps> = ({
  isOpen,
  onClose,
  onCommit,
  currentSnapshotDepth,
}) => {
  const [name, setName] = useState(`Snapshot #${currentSnapshotDepth + 1}`);
  const [reason, setReason] = useState("");
  const [summary, setSummary] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    setIsSubmitting(true);
    try {
      await onCommit(name, reason, summary);
      onClose();
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
      <div className="w-full max-w-md bg-[#16181f] border border-[#2b303c] rounded-2xl shadow-2xl p-6 flex flex-col gap-5 text-gray-200">
        <div className="flex items-center justify-between border-b border-[#252936] pb-3">
          <div className="flex items-center gap-2 text-purple-400 font-semibold text-sm">
            <GitCommit size={18} />
            <span>Create Semantic Snapshot</span>
          </div>
          <button
            onClick={onClose}
            className="p-1 hover:bg-[#252936] text-gray-400 hover:text-white rounded-lg transition"
          >
            <X size={16} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          <div className="flex flex-col gap-1.5">
            <label className="text-xs text-gray-400 font-medium">Snapshot Name</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Learn Board Refactor"
              className="bg-[#111216] border border-[#262b37] focus:border-purple-500/60 rounded-lg px-3 py-2 text-xs text-white focus:outline-none transition"
              required
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-xs text-gray-400 font-medium">Why did you make this change? (Reason)</label>
            <textarea
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              placeholder="دلیل و فلسفه تغییر این چیدمان یا متن..."
              rows={2}
              className="bg-[#111216] border border-[#262b37] focus:border-purple-500/60 rounded-lg px-3 py-2 text-xs text-white placeholder-gray-600 focus:outline-none resize-none transition"
              dir="auto"
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-xs text-gray-400 font-medium">Summary of Changes</label>
            <textarea
              value={summary}
              onChange={(e) => setSummary(e.target.value)}
              placeholder="خلاصه تغییرات: اصلاح کارت اول، افزودن دیاگرام..."
              rows={2}
              className="bg-[#111216] border border-[#262b37] focus:border-purple-500/60 rounded-lg px-3 py-2 text-xs text-white placeholder-gray-600 focus:outline-none resize-none transition"
              dir="auto"
            />
          </div>

          <div className="flex items-center justify-end gap-2.5 pt-2 border-t border-[#252936]">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-lg bg-[#202430] hover:bg-[#282e3d] text-xs text-gray-300 transition"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="flex items-center gap-1.5 px-4 py-2 rounded-lg bg-purple-600 hover:bg-purple-500 text-xs font-semibold text-white shadow-lg shadow-purple-600/30 transition disabled:opacity-50"
            >
              <Sparkles size={13} />
              <span>{isSubmitting ? "Committing..." : "Commit Snapshot"}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
