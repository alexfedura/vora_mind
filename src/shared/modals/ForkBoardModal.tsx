import React, { useEffect, useState } from "react";
import { GitBranch, X } from "lucide-react";
import type { BoardVersionSummary } from "@/mind/services/dbService";

interface ForkBoardModalProps {
  isOpen: boolean;
  boardTitle: string;
  version: BoardVersionSummary | null;
  onClose: () => void;
  onFork: (newBoardTitle: string) => Promise<void>;
}

export const ForkBoardModal: React.FC<ForkBoardModalProps> = ({
  isOpen,
  boardTitle,
  version,
  onClose,
  onFork,
}) => {
  const [branchName, setBranchName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    setBranchName(`${boardTitle} Branch`);
    setError(null);
  }, [boardTitle, isOpen]);

  if (!isOpen || !version) return null;

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const title = branchName.trim();
    if (!title) {
      setError("Enter a name for this Board branch.");
      return;
    }
    setIsSubmitting(true);
    setError(null);
    try {
      await onFork(title);
      onClose();
    } catch (cause) {
      console.error("Could not create Board branch:", cause);
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
        aria-labelledby="fork-board-title"
        onSubmit={(event) => void handleSubmit(event)}
        className="flex w-full max-w-md flex-col gap-5 rounded-2xl border border-[#303543] bg-[#171a22] p-6 text-zinc-200 shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-[#2a2f3d] pb-3">
          <div className="flex items-center gap-2 text-violet-300">
            <GitBranch size={18} />
            <h2 id="fork-board-title" className="text-sm font-semibold">Create Board branch</h2>
          </div>
          <button type="button" onClick={onClose} aria-label="Close" className="rounded p-1 text-zinc-400 hover:bg-[#252936] hover:text-white">
            <X size={16} />
          </button>
        </div>
        <p className="text-xs text-zinc-400">
          Create an independent copy of <span className="text-zinc-200">{boardTitle}</span> at version{" "}
          <span className="text-zinc-200">{version.version}</span>.
        </p>
        <label className="flex flex-col gap-2 text-xs text-zinc-300">
          <span>Enter a name for the new branch:</span>
          <input
            autoFocus
            required
            maxLength={80}
            dir="auto"
            value={branchName}
            onChange={(event) => setBranchName(event.currentTarget.value)}
            placeholder="Name this branch..."
            aria-label="Branch Name"
            className="rounded-lg border border-[#303644] bg-[#111216] px-3 py-2 text-sm text-white outline-none focus:border-violet-500"
          />
        </label>
        {error && <p role="alert" className="text-xs text-rose-300">{error}</p>}
        <div className="flex justify-end gap-2 border-t border-[#2a2f3d] pt-3">
          <button type="button" onClick={onClose} className="rounded-lg bg-[#252936] px-4 py-2 text-xs text-zinc-300 hover:bg-[#303543]">
            Cancel
          </button>
          <button type="submit" disabled={!branchName.trim() || isSubmitting} className="rounded-lg bg-violet-600 px-4 py-2 text-xs font-semibold text-white hover:bg-violet-500 disabled:opacity-50">
            {isSubmitting ? "Creating..." : "Create Branch"}
          </button>
        </div>
      </form>
    </div>
  );
};
