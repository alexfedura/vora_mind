import React, { useEffect, useState } from "react";
import { GitBranch, X } from "lucide-react";
import type { BoardVersionIncrement } from "@/mind/services/dbService";

interface CommitBoardModalProps {
  isOpen: boolean;
  boardTitle: string;
  increment: BoardVersionIncrement;
  onClose: () => void;
  onCommit: (reason: string) => Promise<void>;
}

export const CommitBoardModal: React.FC<CommitBoardModalProps> = ({
  isOpen,
  boardTitle,
  increment,
  onClose,
  onCommit,
}) => {
  const [reason, setReason] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    setReason("");
    setError(null);
  }, [isOpen]);

  if (!isOpen) return null;

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const trimmedReason = reason.trim();
    if (!trimmedReason) {
      setError("Enter a reason for this Board version.");
      return;
    }
    setIsSubmitting(true);
    setError(null);
    try {
      await onCommit(trimmedReason);
      onClose();
    } catch (cause) {
      console.error("Could not create Board version:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[70] flex items-center justify-center bg-black/70 p-4 backdrop-blur-sm">
      <form
        role="dialog"
        aria-modal="true"
        aria-labelledby="commit-board-title"
        onSubmit={(event) => void handleSubmit(event)}
        className="flex w-full max-w-md flex-col gap-5 rounded-2xl border border-[#303543] bg-[#171a22] p-6 text-zinc-200 shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-[#2a2f3d] pb-3">
          <div className="flex items-center gap-2 text-violet-300">
            <GitBranch size={18} />
            <h2 id="commit-board-title" className="text-sm font-semibold">Create {increment} version</h2>
          </div>
          <button type="button" onClick={onClose} aria-label="Close" className="rounded p-1 text-zinc-400 hover:bg-[#252936] hover:text-white">
            <X size={16} />
          </button>
        </div>
        <p className="text-xs text-zinc-400">Save a version of <span className="text-zinc-200">{boardTitle}</span>.</p>
        <label className="flex flex-col gap-2 text-xs text-zinc-300">
          <span>Reason for change</span>
          <textarea
            autoFocus
            required
            dir="auto"
            rows={4}
            value={reason}
            onChange={(event) => setReason(event.currentTarget.value)}
            placeholder="Describe what changed and why..."
            className="resize-y rounded-lg border border-[#303644] bg-[#111216] px-3 py-2 text-sm text-white outline-none focus:border-violet-500"
          />
        </label>
        {error && <p role="alert" className="text-xs text-rose-300">{error}</p>}
        <div className="flex justify-end gap-2 border-t border-[#2a2f3d] pt-3">
          <button type="button" onClick={onClose} className="rounded-lg bg-[#252936] px-4 py-2 text-xs text-zinc-300 hover:bg-[#303543]">
            Cancel
          </button>
          <button type="submit" disabled={!reason.trim() || isSubmitting} className="rounded-lg bg-violet-600 px-4 py-2 text-xs font-semibold text-white hover:bg-violet-500 disabled:opacity-50">
            {isSubmitting ? "Saving..." : "Save version"}
          </button>
        </div>
      </form>
    </div>
  );
};
