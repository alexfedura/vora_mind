import React, { useEffect, useMemo, useState } from "react";
import { diffLines } from "diff";
import { GitCommit, History, RotateCcw, X } from "lucide-react";
import { useDocumentStore } from "@/features/file-store/useDocumentStore";
import { dbService } from "@/mind/services/dbService";
import type { CardVersion, CardVersionContent } from "@/mind/types";
import { useAppStore } from "@/shared/store/useAppStore";

interface CardHistoryModalProps {
  isOpen: boolean;
  cardId: string | null;
  onClose: () => void;
  onOpenCommit: () => void;
}

interface DiffLine {
  key: string;
  prefix: string;
  text: string;
  kind: "added" | "removed" | "unchanged";
}

function getWorkingContent(cardId: string): string {
  const document = useDocumentStore.getState();
  if (document.currentDocId === cardId) return document.content;
  return useAppStore.getState().cards[cardId]?.content ?? "";
}

function getWorkingTitle(cardId: string): string {
  const document = useDocumentStore.getState();
  if (document.currentDocId === cardId) return document.currentTitle;
  return useAppStore.getState().cards[cardId]?.title ?? "";
}

export const CardHistoryModal: React.FC<CardHistoryModalProps> = ({
  isOpen,
  cardId,
  onClose,
  onOpenCommit,
}) => {
  const [versions, setVersions] = useState<CardVersion[]>([]);
  const [selectedVersionId, setSelectedVersionId] = useState("");
  const [compareVersionId, setCompareVersionId] = useState("");
  const [selectedContent, setSelectedContent] = useState<CardVersionContent | null>(null);
  const [compareContent, setCompareContent] = useState("");
  const [compareTitle, setCompareTitle] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [restoringVersionId, setRestoringVersionId] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    if (!isOpen || !cardId) return () => { cancelled = true; };

    setIsLoading(true);
    setError(null);
    void dbService.getCardVersions(cardId).then((loadedVersions) => {
      if (cancelled) return;
      setVersions(loadedVersions);
      setSelectedVersionId(loadedVersions[0]?.id ?? "");
      setCompareVersionId("");
    }).catch((cause: unknown) => {
      console.error("Could not load card version history:", cause);
      if (!cancelled) setError(cause instanceof Error ? cause.message : String(cause));
    }).finally(() => {
      if (!cancelled) setIsLoading(false);
    });

    return () => { cancelled = true; };
  }, [cardId, isOpen]);

  useEffect(() => {
    let cancelled = false;
    if (!isOpen || !cardId || !selectedVersionId) {
      setSelectedContent(null);
      setCompareContent("");
      setCompareTitle("");
      return () => { cancelled = true; };
    }

    const loadDiffContents = async (): Promise<void> => {
      try {
        const [selected, comparison] = await Promise.all([
          dbService.getCardVersionContent(selectedVersionId),
          compareVersionId
            ? dbService.getCardVersionContent(compareVersionId)
            : Promise.resolve(null),
        ]);
        if (cancelled) return;
        setSelectedContent(selected);
        setCompareContent(comparison?.content ?? getWorkingContent(cardId));
        setCompareTitle(comparison?.title ?? getWorkingTitle(cardId));
        setError(null);
      } catch (cause) {
        console.error("Could not load card version content:", cause);
        if (!cancelled) setError(cause instanceof Error ? cause.message : String(cause));
      }
    };
    void loadDiffContents();
    return () => { cancelled = true; };
  }, [cardId, compareVersionId, isOpen, selectedVersionId]);

  const diffLinesResult = useMemo<DiffLine[]>(() => {
    if (!selectedContent) return [];
    let key = 0;
    return diffLines(compareContent, selectedContent.content).flatMap((part) => {
      const kind = part.added ? "added" : part.removed ? "removed" : "unchanged";
      const prefix = kind === "added" ? "+" : kind === "removed" ? "-" : " ";
      const lines = part.value.length === 0 ? [] : part.value.split("\n");
      if (lines[lines.length - 1] === "") lines.pop();
      return lines.map((text) => ({
        key: `line-${key++}`,
        prefix,
        text,
        kind,
      }));
    });
  }, [compareContent, selectedContent]);
  const hasChanges = diffLinesResult.some((line) => line.kind !== "unchanged");
  const hasTitleChange = selectedContent !== null && compareTitle !== selectedContent.title;

  if (!isOpen || !cardId) return null;

  const handleRestore = async (version: CardVersion): Promise<void> => {
    setRestoringVersionId(version.id);
    setError(null);
    try {
      const document = useDocumentStore.getState();
      if (document.currentDocId === cardId) await document.saveDoc();
      const restored = await useAppStore.getState().restoreCardVersion(cardId, version.id);
      if (useDocumentStore.getState().currentDocId === cardId) {
        useDocumentStore.setState({
          currentTitle: restored.title,
          currentVersion: restored.version,
          content: restored.content,
          pendingTextChangeSets: [],
          isDirty: false,
          lastError: null,
        });
      }
      const refreshedVersions = await dbService.getCardVersions(cardId);
      setVersions(refreshedVersions);
      setSelectedVersionId(refreshedVersions[0]?.id ?? "");
      setCompareVersionId("");
    } catch (cause) {
      console.error("Could not restore card version:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setRestoringVersionId(null);
    }
  };

  return (
    <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/75 p-4">
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="card-history-title"
        dir="auto"
        className="flex max-h-[90vh] w-full max-w-5xl flex-col overflow-hidden rounded-2xl border border-[#303543] bg-[#171a22] text-zinc-200 shadow-2xl"
      >
        <header className="flex items-center justify-between border-b border-[#282d38] px-5 py-4">
          <div className="flex min-w-0 items-center gap-3">
            <h2 id="card-history-title" className="flex items-center gap-2 text-sm font-semibold text-amber-100">
              <History size={17} />
              Card version history
            </h2>
            <button
              type="button"
              onClick={onOpenCommit}
              className="flex shrink-0 items-center gap-1.5 rounded-lg bg-amber-500/20 px-3 py-2 text-xs font-semibold text-amber-100 hover:bg-amber-500/30"
            >
              <GitCommit size={14} />
              + Commit New Version
            </button>
          </div>
          <button type="button" onClick={onClose} aria-label="Close" className="rounded-lg p-1 text-zinc-400 hover:bg-[#282d38] hover:text-white">
            <X size={16} />
          </button>
        </header>

        <div className="grid min-h-0 flex-1 grid-cols-1 md:grid-cols-[minmax(240px,0.8fr)_minmax(0,1.6fr)]">
          <aside className="min-h-0 overflow-y-auto border-b border-[#282d38] p-4 md:border-b-0 md:border-r">
            <h3 className="mb-3 text-xs font-semibold uppercase tracking-wide text-zinc-500">Versions</h3>
            {isLoading && <p className="text-sm text-zinc-500">Loading history...</p>}
            {!isLoading && versions.length === 0 && (
              <p className="text-sm text-zinc-500">No committed card versions yet.</p>
            )}
            <ul className="space-y-2">
              {versions.map((version) => (
                <li key={version.id}>
                  <div className={`rounded-lg border p-3 ${
                    selectedVersionId === version.id
                      ? "border-amber-500/40 bg-amber-500/5"
                      : "border-[#292e39] bg-[#111318]"
                  }`}>
                    <button
                      type="button"
                      aria-pressed={selectedVersionId === version.id}
                      onClick={() => setSelectedVersionId(version.id)}
                      className="w-full text-start"
                    >
                      <span className="font-mono text-xs font-semibold text-amber-200">{version.version}</span>
                      <time className="mt-1 block text-[10px] text-zinc-500" dateTime={new Date(version.created_at).toISOString()}>
                        {new Date(version.created_at).toLocaleString()}
                      </time>
                      <span
                        dir="auto"
                        style={{ unicodeBidi: "plaintext", textAlign: "start" }}
                        className="mt-2 block whitespace-pre-wrap text-xs text-zinc-300"
                      >
                        {version.title || "Untitled"}
                      </span>
                      <span
                        dir="auto"
                        style={{ unicodeBidi: "plaintext", textAlign: "start" }}
                        className="mt-1 block whitespace-pre-wrap text-[11px] text-zinc-400"
                      >
                        {version.reason || "No reason recorded"}
                      </span>
                    </button>
                    <button
                      type="button"
                      disabled={restoringVersionId !== null}
                      onClick={() => void handleRestore(version)}
                      className="mt-3 flex items-center gap-1.5 rounded-md px-2 py-1 text-[11px] text-zinc-300 hover:bg-[#292e39] hover:text-white disabled:cursor-wait disabled:opacity-50"
                    >
                      <RotateCcw size={12} />
                      {restoringVersionId === version.id ? "Restoring..." : "Restore"}
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          </aside>

          <main className="flex min-h-0 flex-col p-4">
            <div className="mb-3 grid gap-3 sm:grid-cols-2">
              <label className="text-[11px] text-zinc-400">
                Version to inspect
                <select
                  value={selectedVersionId}
                  onChange={(event) => setSelectedVersionId(event.target.value)}
                  className="mt-1 block w-full rounded-md border border-[#363b47] bg-[#101217] px-2 py-1.5 font-mono text-xs text-zinc-200"
                >
                  {versions.map((version) => (
                    <option key={version.id} value={version.id}>{version.version}</option>
                  ))}
                </select>
              </label>
              <label className="text-[11px] text-zinc-400">
                Compare against
                <select
                  value={compareVersionId}
                  onChange={(event) => setCompareVersionId(event.target.value)}
                  className="mt-1 block w-full rounded-md border border-[#363b47] bg-[#101217] px-2 py-1.5 text-xs text-zinc-200"
                >
                  <option value="">Current working copy</option>
                  {versions.filter((version) => version.id !== selectedVersionId).map((version) => (
                    <option key={version.id} value={version.id}>{version.version}</option>
                  ))}
                </select>
              </label>
            </div>
            {error && <p role="alert" className="mb-3 text-sm text-rose-300">{error}</p>}
            <div className="min-h-0 flex-1 overflow-auto rounded-lg border border-[#292e39] bg-[#0e1014]">
              {selectedContent ? (
                <>
                  {hasTitleChange && (
                    <p
                      dir="auto"
                      style={{ unicodeBidi: "plaintext", textAlign: "start" }}
                      className="border-b border-[#292e39] bg-amber-500/5 px-3 py-2 text-xs text-amber-100"
                    >
                      Title changed from <bdi>{compareTitle || "Untitled"}</bdi> to{" "}
                      <bdi>{selectedContent.title || "Untitled"}</bdi>
                    </p>
                  )}
                  {hasChanges ? (
                  <pre
                    dir="auto"
                    style={{ unicodeBidi: "plaintext", textAlign: "start" }}
                    className="min-w-max p-3 font-mono text-xs leading-5"
                  >
                    {diffLinesResult.map((line) => (
                      <span
                        key={line.key}
                        className={`block whitespace-pre ${
                          line.kind === "added"
                            ? "bg-emerald-500/10 text-emerald-300"
                            : line.kind === "removed"
                              ? "bg-rose-500/10 text-rose-300"
                              : "text-zinc-400"
                        }`}
                      >
                        {line.prefix} {line.text || " "}
                      </span>
                    ))}
                  </pre>
                  ) : (
                    <p className="p-5 text-sm text-zinc-500">
                      {hasTitleChange ? "No content differences." : "No content or title differences."}
                    </p>
                  )}
                </>
              ) : (
                <p className="p-5 text-sm text-zinc-500">
                  {versions.length ? "Select a version to view its content diff." : "Commit a card version to start its history."}
                </p>
              )}
            </div>
          </main>
        </div>
      </section>
    </div>
  );
};
