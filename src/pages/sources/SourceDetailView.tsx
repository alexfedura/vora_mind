import React from "react";
import { SourceBundle } from "@/source/services/sourceService";
import { SegmentTreeItem } from "./SegmentTreeItem";
import { formatDuration, getSourceIcon } from "./sourceUtils";
import {
  FolderOpen,
  Calendar,
  User,
  Globe,
  FileCode,
  Layers,
  FileText,
  Plus,
} from "lucide-react";

interface SourceDetailViewProps {
  bundle: SourceBundle | null;
  selectedSegmentId: string | null;
  onSelectSegment: (id: string) => void;
  onOpenAddSegment: (parentId?: string | null) => void;
}

export const SourceDetailView: React.FC<SourceDetailViewProps> = ({
  bundle,
  selectedSegmentId,
  onSelectSegment,
  onOpenAddSegment,
}) => {
  if (!bundle || !bundle.source) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-8 text-center text-gray-500">
        <FolderOpen size={48} className="mb-3 text-gray-600" />
        <h3 className="text-sm font-semibold text-gray-300 mb-1">No source selected</h3>
        <p className="text-xs text-gray-500 max-w-sm">
          Select a source from the list on the left to inspect its segments, metadata, and extracts, or click &ldquo;Add Source&rdquo; to add a new one.
        </p>
      </div>
    );
  }

  const currentSource = bundle.source;

  return (
    <div className="p-6 flex flex-col gap-6 max-w-5xl">
      {/* Header Card */}
      <div className="p-5 rounded-2xl bg-[#12141a] border border-[#212532] shadow-sm">
        <div className="flex items-start justify-between gap-4">
          <div className="flex items-start gap-3.5">
            <div className="p-3 rounded-xl bg-[#181b23] border border-[#262b3a] shrink-0 mt-0.5">
              {getSourceIcon(currentSource.kind, 22)}
            </div>
            <div>
              <div className="flex items-center gap-2.5">
                <h1 className="text-lg font-bold text-gray-100">{currentSource.title}</h1>
                <span className="text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-amber-500/10 text-amber-300 border border-amber-500/20 font-semibold">
                  {currentSource.kind}
                </span>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-[#1c202b] text-gray-400">
                  {currentSource.id}
                </span>
              </div>
              {currentSource.author && (
                <div className="flex items-center gap-1.5 mt-1.5 text-xs text-gray-300 font-medium">
                  <User size={13} className="text-gray-400" />
                  <span>{currentSource.author}</span>
                </div>
              )}
              {currentSource.notes && (
                <p className="mt-2 text-xs text-gray-400 leading-relaxed bg-[#161821] p-2.5 rounded-lg border border-[#202431]">
                  {currentSource.notes}
                </p>
              )}
            </div>
          </div>

          <button
            onClick={() => onOpenAddSegment(null)}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#181b23] hover:bg-amber-500 hover:text-black text-xs font-semibold text-gray-200 transition border border-[#282d3c] shadow-sm shrink-0"
          >
            <Plus size={14} />
            <span>Add Segment</span>
          </button>
        </div>

        {/* Metadata badges */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mt-4 pt-4 border-t border-[#1b1f2b] text-xs">
          {currentSource.duration_sec != null && (
            <div className="flex items-center gap-2 text-gray-400">
              <span className="text-gray-500">Duration:</span>
              <span className="font-mono text-gray-200">
                {formatDuration(currentSource.duration_sec)}
              </span>
            </div>
          )}
          {currentSource.page_count != null && (
            <div className="flex items-center gap-2 text-gray-400">
              <span className="text-gray-500">Pages:</span>
              <span className="font-mono text-gray-200">{currentSource.page_count}</span>
            </div>
          )}
          {currentSource.language && (
            <div className="flex items-center gap-2 text-gray-400">
              <Globe size={13} className="text-gray-500" />
              <span className="uppercase font-mono text-gray-200">
                {currentSource.language}
              </span>
            </div>
          )}
          <div className="flex items-center gap-2 text-gray-400">
            <Layers size={13} className="text-gray-500" />
            <span>{bundle.segment_count} segments</span>
          </div>
          <div className="flex items-center gap-2 text-gray-400">
            <FileText size={13} className="text-gray-500" />
            <span>{bundle.extract_count} extracts</span>
          </div>
          <div className="flex items-center gap-2 text-gray-400">
            <Calendar size={13} className="text-gray-500" />
            <span className="font-mono text-[11px] text-gray-300">
              {new Date(currentSource.created_at * 1000).toLocaleDateString()}
            </span>
          </div>
        </div>

        {currentSource.path && (
          <div className="mt-3 flex items-center gap-2 text-[11px] font-mono text-gray-400 bg-[#161821] px-3 py-1.5 rounded-lg border border-[#222633] overflow-hidden">
            <FileCode size={13} className="text-gray-500 shrink-0" />
            <span className="truncate">{currentSource.path}</span>
          </div>
        )}
      </div>

      {/* Segment Tree Section */}
      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <h2 className="text-sm font-bold text-gray-100">Segments & Structure</h2>
            <span className="text-xs text-gray-500 font-mono">
              ({bundle.segment_count})
            </span>
          </div>
          <button
            onClick={() => onOpenAddSegment(null)}
            className="text-xs text-amber-400 hover:text-amber-300 font-medium flex items-center gap-1 transition"
          >
            <Plus size={12} />
            <span>New Root Segment</span>
          </button>
        </div>

        {bundle.segment_tree.length === 0 ? (
          <div className="p-8 rounded-xl bg-[#12141a] border border-dashed border-[#232837] text-center">
            <p className="text-xs text-gray-400 font-medium">No segments yet</p>
            <p className="text-[11px] text-gray-500 mt-1 mb-3">
              Break down this source into chapters, time segments, or topics.
            </p>
            <button
              onClick={() => onOpenAddSegment(null)}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-amber-500 hover:bg-amber-400 text-black font-semibold text-xs transition"
            >
              <Plus size={13} />
              <span>Add First Segment</span>
            </button>
          </div>
        ) : (
          <div className="p-2 rounded-2xl bg-[#12141a] border border-[#212532] space-y-1">
            {bundle.segment_tree.map((node) => (
              <SegmentTreeItem
                key={node.id}
                node={node}
                depth={0}
                selectedSegmentId={selectedSegmentId}
                onSelectSegment={onSelectSegment}
                onAddSubSegment={(parentId, e) => {
                  e.stopPropagation();
                  onOpenAddSegment(parentId);
                }}
              />
            ))}
          </div>
        )}
      </div>

      {/* Segment detail placeholder for SRC-6 */}
      {selectedSegmentId && (
        <div className="p-4 rounded-xl bg-[#141720] border border-amber-500/20 text-xs text-gray-400 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="text-amber-400 font-semibold">Selected Segment:</span>
            <span className="font-mono text-gray-200">{selectedSegmentId}</span>
          </div>
          <span className="text-[11px] text-gray-500">
            Extracts management view incoming in SRC-6
          </span>
        </div>
      )}
    </div>
  );
};
