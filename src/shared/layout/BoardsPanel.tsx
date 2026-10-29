import React, { useState } from "react";
import { Check, Folder, FolderPlus, GitBranch, History, LayoutGrid, Pencil, Trash2, X } from "lucide-react";
import type { BoardItem } from "@/mind/types";
import type { BoardVersionIncrement, BoardVersionSummary } from "@/mind/services/dbService";
import { WorkspaceMenuFooter } from "@/shared/layout/WorkspaceMenuFooter";

interface BoardsPanelProps {
  boards: BoardItem[];
  boardsLoaded: boolean;
  shouldStartCreatingBoard: boolean;
  onStartCreatingBoardHandled: () => void;
  onFirstBoardCreationStateChange: (isCreating: boolean) => void;
  onGoToVoraMenu: () => void;
  activeBoardId: string;
  onSelectBoard: (boardId: string) => Promise<void>;
  onCreateBoard: (title: string) => Promise<void>;
  onRenameBoard: (boardId: string, title: string) => Promise<void>;
  onDeleteBoard: (boardId: string) => Promise<void>;
  onGetBoardVersions: (boardId: string) => Promise<BoardVersionSummary[]>;
  onCreateBoardVersion: (
    boardId: string,
    increment: BoardVersionIncrement,
  ) => Promise<BoardVersionSummary[]>;
  onCheckoutBoardVersion: (boardId: string, versionId: string) => Promise<void>;
}

export const BoardsPanel: React.FC<BoardsPanelProps> = ({
  boards,
  boardsLoaded,
  shouldStartCreatingBoard,
  onStartCreatingBoardHandled,
  onFirstBoardCreationStateChange,
  onGoToVoraMenu,
  activeBoardId,
  onSelectBoard,
  onCreateBoard,
  onRenameBoard,
  onDeleteBoard,
  onGetBoardVersions,
  onCreateBoardVersion,
  onCheckoutBoardVersion,
}) => {
  const [isCreating, setIsCreating] = useState(false);
  const [newBoardTitle, setNewBoardTitle] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busyBoardId, setBusyBoardId] = useState<string | null>(null);
  const [expandedBoardId, setExpandedBoardId] = useState<string | null>(null);
  const [versionsByBoard, setVersionsByBoard] = useState<Record<string, BoardVersionSummary[]>>({});

  React.useEffect(() => {
    if (!shouldStartCreatingBoard) return;
    setError(null);
    setNewBoardTitle("");
    setIsCreating(true);
    if (boards.length === 0) onFirstBoardCreationStateChange(true);
    onStartCreatingBoardHandled();
  }, [boards.length, onFirstBoardCreationStateChange, onStartCreatingBoardHandled, shouldStartCreatingBoard]);

  const createBoard = async (): Promise<void> => {
    const title = newBoardTitle.trim();
    if (!title) return;
    setError(null);
    setBusyBoardId("create");
    try {
      await onCreateBoard(title);
      setNewBoardTitle("");
      setIsCreating(false);
      if (boards.length === 0) onFirstBoardCreationStateChange(false);
    } catch (cause) {
      console.error("Could not create Board:", cause);
      setError("Could not create Board.");
    } finally {
      setBusyBoardId(null);
    }
  };

  const renameBoard = async (board: BoardItem): Promise<void> => {
    const title = window.prompt("Rename Board", board.title)?.trim();
    if (!title || title === board.title) return;
    setError(null);
    setBusyBoardId(board.id);
    try {
      await onRenameBoard(board.id, title);
    } catch (cause) {
      console.error("Could not rename Board:", cause);
      setError("Could not rename Board.");
    } finally {
      setBusyBoardId(null);
    }
  };

  const deleteBoard = async (board: BoardItem): Promise<void> => {
    if (!window.confirm(`Delete "${board.title}" and its cards and canvas?`)) return;
    setError(null);
    setBusyBoardId(board.id);
    try {
      await onDeleteBoard(board.id);
    } catch (cause) {
      console.error("Could not delete Board:", cause);
      setError("Could not delete Board.");
    } finally {
      setBusyBoardId(null);
    }
  };

  const toggleVersionHistory = async (board: BoardItem): Promise<void> => {
    if (expandedBoardId === board.id) {
      setExpandedBoardId(null);
      return;
    }
    setError(null);
    setBusyBoardId(board.id);
    try {
      const versions = await onGetBoardVersions(board.id);
      setVersionsByBoard((current) => ({ ...current, [board.id]: versions }));
      setExpandedBoardId(board.id);
    } catch (cause) {
      console.error("Could not load Board version history:", cause);
      setError(`Could not load version history: ${errorText(cause)}`);
    } finally {
      setBusyBoardId(null);
    }
  };

  const createVersion = async (
    board: BoardItem,
    increment: BoardVersionIncrement,
  ): Promise<void> => {
    setError(null);
    setBusyBoardId(board.id);
    try {
      const versions = await onCreateBoardVersion(board.id, increment);
      setVersionsByBoard((current) => ({ ...current, [board.id]: versions }));
    } catch (cause) {
      console.error("Could not create Board version:", cause);
      setError(`Could not create version: ${errorText(cause)}`);
    } finally {
      setBusyBoardId(null);
    }
  };

  const checkoutVersion = async (board: BoardItem, version: BoardVersionSummary): Promise<void> => {
    if (version.is_active) return;
    setError(null);
    setBusyBoardId(board.id);
    try {
      await onCheckoutBoardVersion(board.id, version.id);
      const versions = await onGetBoardVersions(board.id);
      setVersionsByBoard((current) => ({ ...current, [board.id]: versions }));
    } catch (cause) {
      console.error("Could not check out Board version:", cause);
      setError(`Could not restore version: ${errorText(cause)}`);
    } finally {
      setBusyBoardId(null);
    }
  };

  return (
    <aside className="flex h-full w-56 flex-shrink-0 flex-col justify-between border-r border-[#2a2a2a] bg-background p-3 text-zinc-200">
      <div className="flex min-h-0 flex-1 flex-col">
        <header className="flex h-14 items-center justify-between border-b border-[#2a2a2a] px-4 pr-2">
          <span className="text-xs font-semibold text-zinc-200">Boards</span>
          <div className="flex items-center gap-2">
            <button
              type="button"
              aria-label="Create Board"
              title="Create Board"
              onClick={() => {
                setError(null);
                setIsCreating(true);
                if (boards.length === 0) onFirstBoardCreationStateChange(true);
              }}
              className="rounded-md p-1.5 text-zinc-400 transition hover:bg-[#202430] hover:text-white"
            >
              <FolderPlus size={16} />
            </button>
            <button
              type="button"
              aria-label="Go to Vora menu"
              title="Go to Vora menu"
              onClick={onGoToVoraMenu}
              className="rounded-md p-1.5 text-zinc-400 transition hover:bg-[#202430] hover:text-white"
            >
              <LayoutGrid size={16} />
            </button>
          </div>
        </header>

        {isCreating && (
          <form
            className="flex items-center gap-1 border-b border-[#222632] p-2"
            onSubmit={(event) => { event.preventDefault(); void createBoard(); }}
          >
            <input
              autoFocus
              maxLength={80}
              value={newBoardTitle}
              onChange={(event) => setNewBoardTitle(event.currentTarget.value)}
              placeholder="Board name"
              aria-label="New Board name"
              className="min-w-0 flex-1 rounded-md border border-[#303644] bg-[#171a22] px-2 py-1.5 text-xs outline-none focus:border-violet-500"
            />
            <button type="submit" disabled={!newBoardTitle.trim() || busyBoardId === "create"} aria-label="Save Board" className="rounded p-1 text-violet-300 hover:bg-[#242938] disabled:opacity-40">
              <Check size={15} />
            </button>
            <button
              type="button"
              onClick={() => {
                setIsCreating(false);
                setNewBoardTitle("");
                if (boards.length === 0) onFirstBoardCreationStateChange(false);
              }}
              aria-label="Cancel"
              className="rounded p-1 text-zinc-400 hover:bg-[#242938]"
            >
              <X size={15} />
            </button>
          </form>
        )}

        {error && <p role="alert" className="px-3 py-2 text-[11px] text-rose-300">{error}</p>}

        <div className="flex-1 overflow-y-auto p-2">
          {boards.map((board) => {
            const isActive = board.id === activeBoardId;
            const isBusy = busyBoardId === board.id;
            return (
              <div key={board.id} className="mb-1">
                <div
                  className={`group flex items-center rounded-lg transition ${
                    isActive ? "bg-[#202430] text-violet-300" : "text-zinc-400 hover:bg-[#191c24] hover:text-zinc-100"
                  }`}
                >
                  <button
                    type="button"
                    disabled={isBusy}
                    onClick={() => { void onSelectBoard(board.id).catch((cause: unknown) => {
                      console.error("Could not switch Board:", cause);
                      setError(`Could not open Board: ${errorText(cause)}`);
                    }); }}
                    className="flex min-w-0 flex-1 items-center gap-2 px-2 py-2 text-left text-xs"
                  >
                    <Folder size={15} className="flex-shrink-0" />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate">{board.title}</span>
                      <span className="block truncate text-[9px] opacity-70">{board.active_version}</span>
                    </span>
                  </button>
                  <div className="mr-1 flex items-center">
                    <button
                      type="button"
                      disabled={isBusy}
                      aria-label={`Version history for ${board.title}`}
                      title="Version history"
                      aria-expanded={expandedBoardId === board.id}
                      onClick={() => void toggleVersionHistory(board)}
                      className="rounded p-1 hover:bg-[#303543] hover:text-white disabled:opacity-50"
                    >
                      <History size={13} />
                    </button>
                    <button type="button" aria-label={`Rename ${board.title}`} title="Rename Board" onClick={() => void renameBoard(board)} className="hidden rounded p-1 hover:bg-[#303543] hover:text-white group-hover:inline-flex">
                      <Pencil size={13} />
                    </button>
                    <button type="button" aria-label={`Delete ${board.title}`} title="Delete Board" onClick={() => void deleteBoard(board)} className="hidden rounded p-1 hover:bg-rose-500/15 hover:text-rose-300 group-hover:inline-flex">
                      <Trash2 size={13} />
                    </button>
                  </div>
                </div>
                {expandedBoardId === board.id && (
                  <div className="ml-2 border-l border-[#303543] pl-2">
                    <div className="flex items-center gap-1 py-1">
                      <GitBranch size={12} className="mr-auto text-zinc-500" />
                      {(["major", "minor", "patch"] as const).map((increment) => (
                        <button
                          key={increment}
                          type="button"
                          disabled={!isActive || isBusy}
                          onClick={() => void createVersion(board, increment)}
                          title={`Create ${increment} version`}
                          aria-label={`Create ${increment} version`}
                          className="rounded px-1.5 py-1 text-[9px] uppercase text-zinc-400 hover:bg-[#303543] hover:text-white disabled:cursor-not-allowed disabled:opacity-40"
                        >
                          {increment === "major" ? "Major" : increment === "minor" ? "Minor" : "Patch"}
                        </button>
                      ))}
                    </div>
                    {!isActive && <p className="pb-1 text-[9px] text-zinc-500">Select this Board to create a version.</p>}
                    <div className="max-h-40 overflow-y-auto pb-1">
                      {(versionsByBoard[board.id] ?? []).slice().reverse().map((version) => (
                        <button
                          key={version.id}
                          type="button"
                          disabled={isBusy || version.is_active}
                          onClick={() => void checkoutVersion(board, version)}
                          className={`flex w-full items-center justify-between gap-1 rounded px-2 py-1.5 text-left text-[10px] ${
                            version.is_active ? "bg-[#202430] text-violet-300" : "text-zinc-400 hover:bg-[#191c24] hover:text-zinc-100"
                          } disabled:cursor-default`}
                        >
                          <span className="truncate">
                            {version.version}{version.forked_from_version ? ` - ${version.forked_from_version}` : ""}
                          </span>
                          <time
                            className="shrink-0 text-[8px] opacity-60"
                            dateTime={new Date(version.created_at).toISOString()}
                            title={new Date(version.created_at).toLocaleString()}
                          >
                            {new Date(version.created_at).toLocaleString()}
                          </time>
                        </button>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            );
          })}
          {boards.length === 0 && !isCreating && (
            <p className="px-2 py-3 text-xs text-zinc-500">
              {boardsLoaded ? "No Boards yet. Create one to start using My desk." : "Loading Boards…"}
            </p>
          )}
        </div>
      </div>
      <WorkspaceMenuFooter />
    </aside>
  );
};

function errorText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
