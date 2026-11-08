import React, { useEffect, useState } from "react";
import { GitCommit, X } from "lucide-react";
import { nextPatchVersion } from "@/mind/utils/cardVersion";
import { useAppStore } from "@/shared/store/useAppStore";
import { useDocumentStore } from "@/features/file-store/useDocumentStore";

interface CommitCardModalProps {
  isOpen: boolean;
  cardId: string | null;
  isFirstVersion: boolean;
  onClose: () => void;
}

export const CommitCardModal: React.FC<CommitCardModalProps> = ({
  isOpen,
  cardId,
  isFirstVersion,
  onClose,
}) => {
  const [version, setVersion] = useState("v0.1.1");
  const [reason, setReason] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (!isOpen || !cardId) return;
    const card = useAppStore.getState().cards[cardId];
    setVersion(isFirstVersion ? "v0.1.0" : nextPatchVersion(card?.version ?? "v0.1.0"));
    setReason(isFirstVersion ? "Initial Version (Baseline)" : "");
    setError(null);
  }, [cardId, isFirstVersion, isOpen]);

  if (!isOpen || !cardId) return null;

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const versionTag = version.trim();
    const changeReason = isFirstVersion ? "Initial Version (Baseline)" : reason.trim();
    if (!changeReason) {
      setError("Enter a reason for this card version.");
      return;
    }

    setIsSubmitting(true);
    setError(null);
    try {
      const documentStore = useDocumentStore.getState();
      if (documentStore.currentDocId === cardId) await documentStore.saveDoc();

      const card = useAppStore.getState().cards[cardId];
      if (!card) throw new Error("The card is no longer available.");
      const currentDocument = useDocumentStore.getState();
      const isCurrentDocument = currentDocument.currentDocId === cardId;
      const title = isCurrentDocument ? currentDocument.currentTitle : card.title;
      const content = isCurrentDocument ? currentDocument.content : card.content;
      await useAppStore.getState().commitCardVersion(
        cardId,
        title,
        content,
        versionTag,
        changeReason,
      );
      window.dispatchEvent(new CustomEvent<string>("vora:card-version-saved", { detail: cardId }));

      if (isCurrentDocument) {
        useDocumentStore.setState({
          currentTitle: title,
          currentVersion: versionTag,
          content,
          pendingTextChangeSets: [],
          isDirty: false,
          lastError: null,
        });
      }
      onClose();
    } catch (cause) {
      console.error("Could not commit card version:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/70 p-4">
      <form
        role="dialog"
        aria-modal="true"
        aria-labelledby="commit-card-version-title"
        onSubmit={(event) => void handleSubmit(event)}
        className="w-full max-w-md rounded-2xl border border-[#303543] bg-[#171a22] p-6 text-zinc-200 shadow-2xl"
      >
        <div className="flex items-center justify-between">
          <h2 id="commit-card-version-title" className="flex items-center gap-2 text-sm font-semibold text-amber-200">
            <GitCommit size={17} />
            {isFirstVersion ? "Create Initial Version" : "Commit New Version"}
          </h2>
          <button type="button" onClick={onClose} aria-label="Close" className="rounded-lg p-1 text-zinc-400 hover:bg-[#282d38] hover:text-white">
            <X size={16} />
          </button>
        </div>

        <label htmlFor="card-version-tag" className="mt-5 block text-xs font-medium text-zinc-300">
          New version tag
        </label>
        <input
          id="card-version-tag"
          autoFocus
          required
          value={version}
          onChange={(event) => setVersion(event.target.value)}
          placeholder="v0.1.1"
          pattern="v?(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
          title="Use a three-part numeric version, such as v0.1.1"
          className="mt-2 w-full rounded-lg border border-[#363b47] bg-[#101217] px-3 py-2 font-mono text-sm text-white outline-none focus:border-amber-500/60 disabled:cursor-not-allowed disabled:opacity-70"
        />
        <label htmlFor="card-version-reason" className="mt-4 block text-xs font-medium text-zinc-300">
          Reason for change
        </label>
        <textarea
          id="card-version-reason"
          required
          value={reason}
          onChange={(event) => setReason(event.target.value)}
          readOnly={isFirstVersion}
          rows={3}
          placeholder="Why are you saving this version?"
          dir="auto"
          style={{ textAlign: "start", unicodeBidi: "plaintext" }}
          className="mt-2 w-full resize-y rounded-lg border border-[#363b47] bg-[#101217] px-3 py-2 text-sm text-white outline-none focus:border-amber-500/60"
        />
        {error && <p role="alert" className="mt-3 text-sm text-rose-300">{error}</p>}

        <div className="mt-5 flex justify-end gap-2 border-t border-[#282d38] pt-4">
          <button type="button" onClick={onClose} className="rounded-lg px-3 py-2 text-xs text-zinc-300 hover:bg-[#282d38]">
            Cancel
          </button>
          <button
            type="submit"
            disabled={isSubmitting}
            className="rounded-lg bg-amber-500/20 px-3 py-2 text-xs font-semibold text-amber-100 hover:bg-amber-500/30 disabled:cursor-wait disabled:opacity-50"
          >
            {isSubmitting
              ? "Committing..."
              : isFirstVersion ? "Save Initial Version" : "Commit version"}
          </button>
        </div>
      </form>
    </div>
  );
};
