import React from "react";
import { Search, FolderOpen } from "lucide-react";
import { SourceSummary } from "@/source/services/sourceService";
import { formatDuration, getSourceIcon } from "./sourceUtils";

interface SourceMasterListProps {
  sources: SourceSummary[];
  selectedSourceId: string | null;
  searchQuery: string;
  onSearchChange: (q: string) => void;
  onSelectSource: (id: string) => void;
}

export const SourceMasterList: React.FC<SourceMasterListProps> = ({
  sources,
  selectedSourceId,
  searchQuery,
  onSearchChange,
  onSelectSource,
}) => {
  const filteredSources = sources.filter((s) => {
    const q = searchQuery.toLowerCase().trim();
    if (!q) return true;
    return (
      s.title.toLowerCase().includes(q) ||
      (s.author && s.author.toLowerCase().includes(q)) ||
      s.kind.toLowerCase().includes(q)
    );
  });

  return (
    <aside className="w-80 flex-shrink-0 bg-[#0e1015] border-r border-[#1f2430] flex flex-col justify-between">
      <div className="p-3 border-b border-[#1f2430]">
        <div className="relative">
          <Search
            size={14}
            className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-500"
          />
          <input
            type="text"
            placeholder="Search sources by title or author..."
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
            className="w-full pl-9 pr-3 py-1.5 bg-[#161821] border border-[#232736] focus:border-amber-500/70 rounded-xl text-xs text-gray-200 placeholder-gray-500 outline-none transition"
          />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-2 space-y-1">
        {filteredSources.length === 0 ? (
          <div className="flex flex-col items-center justify-center p-8 text-center text-gray-500 text-xs">
            {sources.length === 0 ? (
              <>
                <FolderOpen size={32} className="mb-2 text-gray-600" />
                <p className="font-semibold text-gray-400">No sources yet</p>
                <p className="mt-1 text-[11px] text-gray-500">
                  Click "+ Add Source" to register your first source.
                </p>
              </>
            ) : (
              <p>No sources match &ldquo;{searchQuery}&rdquo;</p>
            )}
          </div>
        ) : (
          filteredSources.map((s) => {
            const isSelected = s.id === selectedSourceId;
            return (
              <div
                key={s.id}
                onClick={() => onSelectSource(s.id)}
                className={`flex items-start gap-3 p-2.5 rounded-xl cursor-pointer transition border ${
                  isSelected
                    ? "bg-amber-500/10 border-amber-500/40 text-gray-100 shadow-sm shadow-amber-500/5"
                    : "bg-transparent border-transparent hover:bg-[#161821] hover:border-[#222633] text-gray-300"
                }`}
              >
                <div className="p-2 rounded-lg bg-[#181b23] border border-[#262b3a] shrink-0 mt-0.5">
                  {getSourceIcon(s.kind, 16)}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center justify-between gap-1">
                    <span className="text-xs font-semibold truncate text-gray-100">
                      {s.title}
                    </span>
                    <span className="text-[10px] uppercase font-mono px-1.5 py-0.5 rounded bg-[#1f232e] text-gray-400">
                      {s.kind}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 mt-1 text-[11px] text-gray-400">
                    {s.author && <span className="truncate max-w-[120px]">{s.author}</span>}
                    {s.author && <span>·</span>}
                    {s.duration_sec != null && (
                      <span>{formatDuration(s.duration_sec)}</span>
                    )}
                    {s.duration_sec == null && (
                      <span className="font-mono text-[10px] text-gray-500">{s.id}</span>
                    )}
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>

      <div className="p-3 border-t border-[#1f2430] bg-[#101217] text-[11px] text-gray-500 flex items-center justify-between">
        <span>{sources.length} total source{sources.length === 1 ? "" : "s"}</span>
        <span className="font-mono text-[10px]">SRC MODULE</span>
      </div>
    </aside>
  );
};
