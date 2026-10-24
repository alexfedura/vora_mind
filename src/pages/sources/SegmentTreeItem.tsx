import React, { useState } from "react";
import { ChevronRight, ChevronDown, Plus } from "lucide-react";
import { SegmentNode } from "@/source/services/sourceService";

function formatDuration(sec: number | null): string {
  if (sec === null || sec === undefined) return "--:--";
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = Math.floor(sec % 60);
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export interface SegmentTreeItemProps {
  node: SegmentNode;
  depth: number;
  selectedSegmentId: string | null;
  onSelectSegment: (id: string) => void;
  onAddSubSegment: (parentId: string, e: React.MouseEvent) => void;
}

export const SegmentTreeItem: React.FC<SegmentTreeItemProps> = ({
  node,
  depth,
  selectedSegmentId,
  onSelectSegment,
  onAddSubSegment,
}) => {
  const [expanded, setExpanded] = useState(true);
  const isSelected = node.id === selectedSegmentId;
  const hasChildren = node.children && node.children.length > 0;

  let rangeLabel = "";
  if (node.start_sec != null && node.end_sec != null) {
    rangeLabel = `${formatDuration(node.start_sec)} - ${formatDuration(node.end_sec)}`;
  } else if (node.start_sec != null) {
    rangeLabel = `from ${formatDuration(node.start_sec)}`;
  } else if (node.page_start != null && node.page_end != null) {
    rangeLabel = `pp. ${node.page_start}-${node.page_end}`;
  } else if (node.page_start != null) {
    rangeLabel = `p. ${node.page_start}`;
  }

  return (
    <div className="flex flex-col">
      <div
        onClick={() => onSelectSegment(node.id)}
        style={{ paddingLeft: `${depth * 18 + 8}px` }}
        className={`group flex items-center justify-between py-2 pr-3 rounded-xl cursor-pointer transition border ${
          isSelected
            ? "bg-amber-500/10 border-amber-500/40 text-gray-100"
            : "bg-transparent border-transparent hover:bg-[#161821] hover:border-[#222633] text-gray-300"
        }`}
      >
        <div className="flex items-center gap-2 min-w-0 flex-1">
          {hasChildren ? (
            <button
              onClick={(e) => {
                e.stopPropagation();
                setExpanded(!expanded);
              }}
              className="p-1 rounded hover:bg-[#1f2430] text-gray-400 hover:text-white transition"
            >
              {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
            </button>
          ) : (
            <div className="w-5" />
          )}

          <span className="text-xs font-semibold truncate text-gray-200 group-hover:text-white">
            {node.label}
          </span>

          {rangeLabel && (
            <span className="text-[11px] font-mono text-amber-400/80 bg-amber-500/5 px-2 py-0.5 rounded border border-amber-500/20">
              {rangeLabel}
            </span>
          )}
        </div>

        <div className="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition">
          <button
            onClick={(e) => onAddSubSegment(node.id, e)}
            title="Add sub-segment"
            className="flex items-center gap-1 px-2 py-1 rounded-lg bg-[#181b23] hover:bg-[#202532] text-[11px] font-medium text-gray-300 hover:text-amber-300 transition border border-[#262b3a]"
          >
            <Plus size={11} />
            <span>Sub</span>
          </button>
        </div>
      </div>

      {hasChildren && expanded && (
        <div className="flex flex-col">
          {node.children.map((child) => (
            <SegmentTreeItem
              key={child.id}
              node={child}
              depth={depth + 1}
              selectedSegmentId={selectedSegmentId}
              onSelectSegment={onSelectSegment}
              onAddSubSegment={onAddSubSegment}
            />
          ))}
        </div>
      )}
    </div>
  );
};
