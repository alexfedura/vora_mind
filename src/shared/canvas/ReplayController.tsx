import React, { useState, useEffect } from "react";
import {
  Play,
  Pause,
  ChevronLeft,
  ChevronRight,
  History,
  Clock,
  GitCommit,
  Layers,
} from "lucide-react";
import { SnapshotSummary } from "@/mind/services/dbService";
import { forgeService, GitCommitItem } from "@/forge/services/forgeService";

interface ReplayControllerProps {
  snapshots: SnapshotSummary[];
  currentSnapshotIndex: number;
  onSelectSnapshot: (index: number) => void;
  isOpenModal: () => void;
}

export const ReplayController: React.FC<ReplayControllerProps> = ({
  snapshots,
  currentSnapshotIndex,
  onSelectSnapshot,
  isOpenModal,
}) => {
  const [mode, setMode] = useState<"snapshots" | "git">("snapshots");
  const [gitCommits, setGitCommits] = useState<GitCommitItem[]>([]);
  const [currentGitIndex, setCurrentGitIndex] = useState(0);

  const [isPlaying, setIsPlaying] = useState(false);
  const [intervalSec, setIntervalSec] = useState(5);

  useEffect(() => {
    if (mode === "git") {
      forgeService
        .getGitCommitHistory(30)
        .then((commits: GitCommitItem[]) => {
          setGitCommits(commits);
          setCurrentGitIndex(0);
        })
        .catch((err: any) => {
          console.error("Failed to load Git history:", err);
        });
    }
  }, [mode]);

  const totalItems = mode === "snapshots" ? snapshots.length : gitCommits.length;
  const currentIndex = mode === "snapshots" ? currentSnapshotIndex : currentGitIndex;

  const handleSelectIndex = (nextIndex: number) => {
    if (mode === "snapshots") {
      onSelectSnapshot(nextIndex);
    } else {
      setCurrentGitIndex(nextIndex);
    }
  };

  useEffect(() => {
    let timer: any;
    if (isPlaying && totalItems > 0) {
      timer = setInterval(() => {
        if (currentIndex < totalItems - 1) {
          handleSelectIndex(currentIndex + 1);
        } else {
          setIsPlaying(false);
        }
      }, intervalSec * 1000);
    }
    return () => clearInterval(timer);
  }, [isPlaying, currentIndex, totalItems, intervalSec, mode]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement
      )
        return;
      if (e.key === "ArrowRight" && currentIndex < totalItems - 1) {
        handleSelectIndex(currentIndex + 1);
      } else if (e.key === "ArrowLeft" && currentIndex > 0) {
        handleSelectIndex(currentIndex - 1);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [currentIndex, totalItems, mode]);

  const currentSnapshot = snapshots[currentSnapshotIndex];
  const currentCommit = gitCommits[currentGitIndex];

  return (
    <div className="fixed bottom-6 left-1/2 -translate-x-1/2 z-40 flex items-center gap-3 bg-[#16181f]/95 border border-[#2b303c] rounded-2xl px-5 py-3 shadow-2xl backdrop-blur-md text-gray-200">
      <div className="flex bg-[#101217] p-1 rounded-xl border border-[#262b37] gap-1">
        <button
          onClick={() => {
            setMode("snapshots");
            setIsPlaying(false);
          }}
          className={`flex items-center gap-1 px-2.5 py-1 rounded-lg text-xs font-semibold transition ${
            mode === "snapshots"
              ? "bg-purple-600/30 text-purple-300 border border-purple-500/40"
              : "text-gray-400 hover:text-gray-200"
          }`}
          title="Canvas Database Snapshots"
        >
          <Layers size={13} />
          <span>Canvas</span>
        </button>

        <button
          onClick={() => {
            setMode("git");
            setIsPlaying(false);
          }}
          className={`flex items-center gap-1 px-2.5 py-1 rounded-lg text-xs font-semibold transition ${
            mode === "git"
              ? "bg-emerald-600/30 text-emerald-300 border border-emerald-500/40"
              : "text-gray-400 hover:text-gray-200"
          }`}
          title="Git Repository Commit Timeline"
        >
          <GitCommit size={13} />
          <span>Git SSOT</span>
        </button>
      </div>

      {mode === "snapshots" && (
        <button
          onClick={isOpenModal}
          className="flex items-center gap-1.5 px-3 py-1.5 bg-purple-600/20 hover:bg-purple-600/30 border border-purple-500/40 text-purple-300 rounded-xl text-xs font-semibold transition"
        >
          <History size={14} />
          <span>Commit Snapshot</span>
        </button>
      )}

      <div className="h-5 w-[1px] bg-[#2a2f3d]" />

      <div className="flex items-center gap-2">
        <button
          disabled={currentIndex <= 0}
          onClick={() => handleSelectIndex(currentIndex - 1)}
          className="p-1.5 hover:bg-[#252936] text-gray-400 hover:text-white rounded-lg transition disabled:opacity-30"
          title="Previous Step (Arrow Left)"
        >
          <ChevronLeft size={18} />
        </button>

        <button
          onClick={() => setIsPlaying(!isPlaying)}
          className={`p-2 rounded-xl transition ${
            isPlaying
              ? "bg-amber-500/20 text-amber-300 border border-amber-500/40"
              : mode === "git"
              ? "bg-emerald-600 hover:bg-emerald-500 text-white shadow-lg shadow-emerald-600/30"
              : "bg-purple-600 hover:bg-purple-500 text-white shadow-lg shadow-purple-600/30"
          }`}
          title={isPlaying ? "Pause Replay" : "Play Sequence"}
        >
          {isPlaying ? <Pause size={16} /> : <Play size={16} />}
        </button>

        <button
          disabled={currentIndex >= totalItems - 1}
          onClick={() => handleSelectIndex(currentIndex + 1)}
          className="p-1.5 hover:bg-[#252936] text-gray-400 hover:text-white rounded-lg transition disabled:opacity-30"
          title="Next Step (Arrow Right)"
        >
          <ChevronRight size={18} />
        </button>
      </div>

      <div className="flex flex-col gap-0.5 px-2 min-w-[170px] max-w-[240px] text-center">
        {mode === "snapshots" ? (
          <>
            <span className="text-xs font-bold text-white truncate">
              {currentSnapshot ? currentSnapshot.name : "Live State"}
            </span>
            <span className="text-[10px] text-gray-400">
              {snapshots.length > 0
                ? `${currentSnapshotIndex + 1} of ${snapshots.length} Snapshots`
                : "No Snapshots"}
            </span>
          </>
        ) : (
          <>
            <div className="flex items-center justify-center gap-1.5 overflow-hidden">
              <span className="text-[10px] font-mono bg-emerald-950/70 text-emerald-300 px-1 rounded border border-emerald-500/30">
                {currentCommit ? currentCommit.short_hash : "HEAD"}
              </span>
              <span className="text-xs font-bold text-white truncate">
                {currentCommit ? currentCommit.message : "Repository Log"}
              </span>
            </div>
            <span className="text-[10px] text-gray-400 font-mono">
              {gitCommits.length > 0
                ? `${currentGitIndex + 1} of ${gitCommits.length} Commits`
                : "No Git Commits"}
            </span>
          </>
        )}
      </div>

      <div className="h-5 w-[1px] bg-[#2a2f3d]" />

      <div className="flex items-center gap-1.5 bg-[#111216] px-2.5 py-1 rounded-lg border border-[#262b37]">
        <Clock size={12} className="text-gray-400" />
        <select
          value={intervalSec}
          onChange={(e) => setIntervalSec(Number(e.target.value))}
          className="bg-transparent text-[11px] text-gray-300 focus:outline-none cursor-pointer"
        >
          <option value={2} className="bg-[#16181f]">
            2s
          </option>
          <option value={5} className="bg-[#16181f]">
            5s
          </option>
          <option value={10} className="bg-[#16181f]">
            10s
          </option>
          <option value={20} className="bg-[#16181f]">
            20s
          </option>
        </select>
      </div>
    </div>
  );
};