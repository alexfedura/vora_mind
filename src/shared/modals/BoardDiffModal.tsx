import React, { useEffect, useMemo, useState } from "react";
import { diffLines } from "diff";
import { GitCompare, X } from "lucide-react";
import type {
  BoardVersionSnapshot,
  BoardVersionSummary,
} from "@/mind/services/dbService";
import { diffBoardSnapshots, type BoardDiffResult, type BoardCardState } from "@/shared/canvas/boardDiffUtils";

interface BoardDiffModalProps {
  isOpen: boolean;
  boardTitle: string;
  versions: BoardVersionSummary[];
  onClose: () => void;
  onGetSnapshot: (versionId: string) => Promise<BoardVersionSnapshot>;
}

interface LoadedDiff {
  oldVersionId: string;
  newVersionId: string;
  result: BoardDiffResult;
}

export const BoardDiffModal: React.FC<BoardDiffModalProps> = ({
  isOpen,
  boardTitle,
  versions,
  onClose,
  onGetSnapshot,
}) => {
  const [selectedVersionIds, setSelectedVersionIds] = useState<string[]>([]);
  const [loadedDiff, setLoadedDiff] = useState<LoadedDiff | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) {
      setSelectedVersionIds([]);
      setLoadedDiff(null);
      setError(null);
    }
  }, [isOpen]);

  useEffect(() => {
    let cancelled = false;
    setLoadedDiff(null);
    setError(null);
    if (!isOpen || selectedVersionIds.length !== 2) {
      setIsLoading(false);
      return () => { cancelled = true; };
    }

    const [oldVersionId, newVersionId] = selectedVersionIds;
    setIsLoading(true);
    const loadSnapshots = async (): Promise<void> => {
      try {
        const [oldSnapshot, newSnapshot] = await Promise.all([
          onGetSnapshot(oldVersionId),
          onGetSnapshot(newVersionId),
        ]);
        if (cancelled) return;
        setLoadedDiff({
          oldVersionId,
          newVersionId,
          result: diffBoardSnapshots(oldSnapshot, newSnapshot),
        });
      } catch (cause) {
        console.error("Could not load Board versions for comparison:", cause);
        if (!cancelled) setError(cause instanceof Error ? cause.message : String(cause));
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    };
    void loadSnapshots();
    return () => { cancelled = true; };
  }, [isOpen, onGetSnapshot, selectedVersionIds]);

  const selectedIds = useMemo(() => new Set(selectedVersionIds), [selectedVersionIds]);
  const versionLabel = (version: BoardVersionSummary): string =>
    `${version.version}${version.forked_from_version ? ` - ${version.forked_from_version}` : ""}`;
  const diff = loadedDiff?.result;
  const isEmpty = diff !== undefined &&
    diff.addedCards.length === 0 &&
    diff.removedCards.length === 0 &&
    diff.modifiedCards.length === 0 &&
    diff.addedObjects.length === 0 &&
    diff.removedObjects.length === 0 &&
    diff.modifiedObjects.length === 0 &&
    diff.addedEdges.length === 0 &&
    diff.removedEdges.length === 0;

  const toggleVersion = (versionId: string): void => {
    setSelectedVersionIds((current) => {
      if (current.includes(versionId)) return current.filter((id) => id !== versionId);
      if (current.length === 2) return current;
      return [...current, versionId];
    });
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[70] flex items-center justify-center bg-black/70 p-4 backdrop-blur-sm">
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="board-diff-title"
        className="flex max-h-[90vh] w-full max-w-3xl flex-col overflow-hidden rounded-2xl border border-[#303543] bg-[#171a22] text-zinc-200 shadow-2xl"
      >
        <header className="flex items-center justify-between border-b border-[#2a2f3d] px-5 py-4">
          <div className="flex items-center gap-2 text-violet-300">
            <GitCompare size={18} />
            <div>
              <h2 id="board-diff-title" className="text-sm font-semibold">Compare Board versions</h2>
              <p className="mt-1 text-[11px] text-zinc-500">{boardTitle}</p>
            </div>
          </div>
          <button type="button" onClick={onClose} aria-label="Close" className="rounded p-1 text-zinc-400 hover:bg-[#252936] hover:text-white">
            <X size={17} />
          </button>
        </header>

        <div className="grid min-h-0 flex-1 gap-4 overflow-y-auto p-5 md:grid-cols-[minmax(220px,0.8fr)_minmax(0,1.4fr)]">
          <div>
            <p className="mb-2 text-xs font-semibold text-zinc-300">Select two versions in comparison order</p>
            <div className="flex flex-col gap-1">
              {versions.map((version) => (
                <label key={version.id} className="flex cursor-pointer items-start gap-2 rounded-lg border border-transparent bg-[#11141b] p-2.5 hover:border-[#343b4b]">
                  <input
                    type="checkbox"
                    checked={selectedIds.has(version.id)}
                    disabled={!selectedIds.has(version.id) && selectedVersionIds.length >= 2}
                    onChange={() => toggleVersion(version.id)}
                    className="mt-0.5 accent-violet-500"
                  />
                  <span className="min-w-0">
                    <span className="block text-xs text-zinc-200">{versionLabel(version)}</span>
                    <time className="mt-1 block text-[10px] text-zinc-500" dateTime={new Date(version.created_at).toISOString()}>
                      {new Date(version.created_at).toLocaleString()}
                    </time>
                    {version.reason && <span className="mt-1 block break-words text-[10px] text-zinc-400">{version.reason}</span>}
                  </span>
                </label>
              ))}
            </div>
          </div>

          <div className="min-w-0">
            {selectedVersionIds.length < 2 && (
              <p className="rounded-lg border border-dashed border-[#343b4b] p-4 text-xs text-zinc-500">
                Choose two versions to compare their cards, objects, and links.
              </p>
            )}
            {isLoading && <p role="status" className="text-xs text-zinc-400">Loading snapshots...</p>}
            {error && <p role="alert" className="rounded-lg bg-rose-950/40 p-3 text-xs text-rose-300">{error}</p>}
            {isEmpty && <p className="rounded-lg bg-[#11141b] p-4 text-xs text-zinc-400">These versions have no structural differences.</p>}
            {diff && !isEmpty && (
              <div className="flex flex-col gap-4">
                <section>
                  <h3 className="mb-2 text-xs font-semibold text-emerald-300">Added Cards ({diff.addedCards.length})</h3>
                  <ul className="flex flex-col gap-2">{diff.addedCards.map((card) => <CardRow key={card.id} card={card} kind="added" />)}</ul>
                </section>
                <section>
                  <h3 className="mb-2 text-xs font-semibold text-rose-300">Removed Cards ({diff.removedCards.length})</h3>
                  <ul className="flex flex-col gap-2">{diff.removedCards.map((card) => <CardRow key={card.id} card={card} kind="removed" />)}</ul>
                </section>
                <section>
                  <h3 className="mb-2 text-xs font-semibold text-amber-300">Modified Cards ({diff.modifiedCards.length})</h3>
                  <ul className="flex flex-col gap-2">
                    {diff.modifiedCards.map((change) => (
                      <li key={change.current.id} className="rounded-lg border border-[#303543] bg-[#11141b] p-3">
                        <p className="mb-2 text-xs font-semibold">{change.current.title || "Untitled"}</p>
                        {change.titleChanged && <TextDiff label="Title" before={change.previous.title} after={change.current.title} />}
                        {change.contentChanged && <TextDiff label="Content" before={change.previous.content} after={change.current.content} />}
                        {change.positionChanged && (
                          <p className="mt-2 text-[10px] text-amber-200">
                            Position: ({formatCoordinate(change.previous.x)}, {formatCoordinate(change.previous.y)})
                            {" -> "}({formatCoordinate(change.current.x)}, {formatCoordinate(change.current.y)})
                          </p>
                        )}
                      </li>
                    ))}
                  </ul>
                </section>
                <section>
                  <h3 className="mb-2 text-xs font-semibold text-emerald-300">Added Objects ({diff.addedObjects.length})</h3>
                  <ul className="flex flex-col gap-2">{diff.addedObjects.map((node) => <ObjectRow key={node.id} node={node} />)}</ul>
                </section>
                <section>
                  <h3 className="mb-2 text-xs font-semibold text-rose-300">Removed Objects ({diff.removedObjects.length})</h3>
                  <ul className="flex flex-col gap-2">{diff.removedObjects.map((node) => <ObjectRow key={node.id} node={node} />)}</ul>
                </section>
                <section>
                  <h3 className="mb-2 text-xs font-semibold text-amber-300">Modified Objects ({diff.modifiedObjects.length})</h3>
                  <ul className="flex flex-col gap-2">
                    {diff.modifiedObjects.map((change) => (
                      <li key={change.current.id} className="rounded-lg border border-[#303543] bg-[#11141b] p-3">
                        <p className="mb-2 text-xs font-semibold">{change.current.title || change.current.node_type}</p>
                        {change.titleChanged && <TextDiff label="Title" before={change.previous.title} after={change.current.title} />}
                        {change.contentChanged && <TextDiff label="Content" before={change.previous.content} after={change.current.content} />}
                        {change.positionChanged && <p className="mt-2 text-[10px] text-amber-200">Position changed</p>}
                      </li>
                    ))}
                  </ul>
                </section>
                <section>
                  <h3 className="mb-2 text-xs font-semibold text-emerald-300">Added Links ({diff.addedEdges.length})</h3>
                  <ul className="flex flex-col gap-1">{diff.addedEdges.map((edge) => <EdgeRow key={`${edge.id}-${edge.source}-${edge.target}`} edge={edge} />)}</ul>
                </section>
                <section>
                  <h3 className="mb-2 text-xs font-semibold text-rose-300">Removed Links ({diff.removedEdges.length})</h3>
                  <ul className="flex flex-col gap-1">{diff.removedEdges.map((edge) => <EdgeRow key={`${edge.id}-${edge.source}-${edge.target}`} edge={edge} />)}</ul>
                </section>
              </div>
            )}
          </div>
        </div>
        <footer className="flex justify-end border-t border-[#2a2f3d] px-5 py-3">
          <button type="button" onClick={onClose} className="rounded-lg bg-[#252936] px-4 py-2 text-xs text-zinc-300 hover:bg-[#303543]">Close</button>
        </footer>
      </section>
    </div>
  );
};

function CardRow({ card, kind }: { card: BoardCardState; kind: "added" | "removed" }): React.ReactElement {
  const color = kind === "added" ? "text-emerald-200" : "text-rose-200";
  return (
    <li className="rounded-lg border border-[#303543] bg-[#11141b] p-3">
      <p className={`text-xs font-semibold ${color}`}>{card.title || "Untitled"}</p>
      {card.content && <p className="mt-1 line-clamp-3 whitespace-pre-wrap text-[10px] text-zinc-400">{card.content}</p>}
      {card.x !== null && card.y !== null && <p className="mt-1 text-[10px] text-zinc-500">Position: ({formatCoordinate(card.x)}, {formatCoordinate(card.y)})</p>}
    </li>
  );
}

function ObjectRow({ node }: { node: BoardVersionSnapshot["nodes"][number] }): React.ReactElement {
  return (
    <li className="rounded-lg border border-[#303543] bg-[#11141b] p-3">
      <p className="text-xs font-semibold text-zinc-200">{node.title || node.node_type}</p>
      <p className="mt-1 text-[10px] text-zinc-500">{node.node_type} · ({formatCoordinate(node.x)}, {formatCoordinate(node.y)})</p>
    </li>
  );
}

function EdgeRow({ edge }: { edge: BoardVersionSnapshot["edges"][number] }): React.ReactElement {
  return (
    <li className="rounded-lg bg-[#11141b] px-3 py-2 text-[10px] text-zinc-300">
      {edge.source} → {edge.target} <span className="text-zinc-500">({edge.link_type ?? "connects_to"})</span>
    </li>
  );
}

function TextDiff({ label, before, after }: { label: string; before: string; after: string }): React.ReactElement {
  const changes = diffLines(before, after).filter((part) => part.added || part.removed);
  return (
    <div className="mt-2">
      <p className="mb-1 text-[10px] font-medium text-zinc-400">{label}</p>
      <pre className="max-h-32 overflow-auto whitespace-pre-wrap break-words rounded bg-[#0b0d12] p-2 text-[10px] leading-relaxed">
        {changes.map((part, index) => (
          <span key={`${index}-${part.value}`} className={part.added ? "text-emerald-300" : "text-rose-300"}>
            {part.added ? "+ " : "- "}{part.value}
            {part.value.endsWith("\n") ? "" : "\n"}
          </span>
        ))}
      </pre>
    </div>
  );
}

function formatCoordinate(value: number | null): string {
  return value === null ? "—" : String(Math.round(value * 100) / 100);
}
