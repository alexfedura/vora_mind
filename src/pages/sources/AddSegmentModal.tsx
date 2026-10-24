import React, { useState } from "react";
import { X, Plus, AlertCircle, Clock, BookOpen, FileText } from "lucide-react";
import { CreateSegmentReq, srcCreateSegment } from "@/source/services/sourceService";

interface AddSegmentModalProps {
  isOpen: boolean;
  sourceId: string;
  sourceKind: string;
  parentSegmentId?: string | null;
  onClose: () => void;
  onSegmentCreated: (newSegmentId: string) => void;
}

export function parseTimeToSeconds(input: string): number | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  if (/^\d+$/.test(trimmed)) return parseInt(trimmed, 10);
  const parts = trimmed.split(":").map((p) => p.trim());
  if (parts.length === 2) {
    const [m, s] = parts.map((n) => parseInt(n, 10));
    if (!isNaN(m) && !isNaN(s)) return m * 60 + s;
  } else if (parts.length === 3) {
    const [h, m, s] = parts.map((n) => parseInt(n, 10));
    if (!isNaN(h) && !isNaN(m) && !isNaN(s)) return h * 3600 + m * 60 + s;
  }
  return null;
}

export const AddSegmentModal: React.FC<AddSegmentModalProps> = ({
  isOpen,
  sourceId,
  sourceKind,
  parentSegmentId = null,
  onClose,
  onSegmentCreated,
}) => {
  const [label, setLabel] = useState("");
  const [startTime, setStartTime] = useState("");
  const [endTime, setEndTime] = useState("");
  const [pageStart, setPageStart] = useState("");
  const [pageEnd, setPageEnd] = useState("");
  const [lineStart, setLineStart] = useState("");
  const [lineEnd, setLineEnd] = useState("");
  const [notes, setNotes] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const isMedia = sourceKind === "video" || sourceKind === "audio";
  const isDocument = sourceKind === "book" || sourceKind === "doc";

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!label.trim()) {
      setError("Segment label is required");
      return;
    }
    setLoading(true);
    setError(null);
    const req: CreateSegmentReq = {
      source_id: sourceId,
      label: label.trim(),
      parent_segment_id: parentSegmentId || null,
      start_sec: isMedia ? parseTimeToSeconds(startTime) : null,
      end_sec: isMedia ? parseTimeToSeconds(endTime) : null,
      page_start: isDocument && pageStart ? parseInt(pageStart, 10) || null : null,
      page_end: isDocument && pageEnd ? parseInt(pageEnd, 10) || null : null,
      line_start: lineStart ? parseInt(lineStart, 10) || null : null,
      line_end: lineEnd ? parseInt(lineEnd, 10) || null : null,
      notes: notes.trim() || null,
    };
    try {
      const newId = await srcCreateSegment(req);
      setLabel("");
      setStartTime("");
      setEndTime("");
      setPageStart("");
      setPageEnd("");
      setLineStart("");
      setLineEnd("");
      setNotes("");
      onSegmentCreated(newId);
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm">
      <div className="bg-[#12141a] border border-[#242835] rounded-2xl w-full max-w-md p-6 shadow-2xl relative flex flex-col max-h-[90vh]">
        <div className="flex items-center justify-between pb-4 border-b border-[#1f232e]">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-amber-500 to-orange-600 flex items-center justify-center font-bold text-white shadow-md">
              <Plus size={18} />
            </div>
            <div>
              <h2 className="text-base font-bold text-gray-100">
                {parentSegmentId ? "Add Sub-Segment" : "Add Segment"}
              </h2>
              <p className="text-xs text-gray-400">Segment bounds for {sourceKind}</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-gray-400 hover:text-white hover:bg-[#1a1d25] transition"
          >
            <X size={18} />
          </button>
        </div>

        {error && (
          <div className="mt-4 p-3 rounded-xl bg-red-950/40 border border-red-500/30 flex items-center gap-2 text-xs text-red-300">
            <AlertCircle size={15} className="shrink-0 text-red-400" />
            <span className="truncate">{error}</span>
          </div>
        )}

        <form onSubmit={handleSubmit} className="flex-1 overflow-y-auto pt-4 space-y-4 pr-1">
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-semibold text-gray-300">
              Segment Label <span className="text-amber-400">*</span>
            </label>
            <input
              type="text"
              required
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              placeholder="e.g. Chapter 1: Introduction or Core Thesis"
              className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition"
            />
          </div>

          {isMedia && (
            <div className="space-y-1.5">
              <div className="flex items-center gap-1.5 text-xs font-semibold text-gray-300">
                <Clock size={14} className="text-amber-400" />
                <span>Time Range (HH:MM:SS or MM:SS)</span>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <input
                  type="text"
                  value={startTime}
                  onChange={(e) => setStartTime(e.target.value)}
                  placeholder="Start (00:05:30)"
                  className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition"
                />
                <input
                  type="text"
                  value={endTime}
                  onChange={(e) => setEndTime(e.target.value)}
                  placeholder="End (00:15:45)"
                  className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition"
                />
              </div>
            </div>
          )}

          {isDocument && (
            <div className="space-y-1.5">
              <div className="flex items-center gap-1.5 text-xs font-semibold text-gray-300">
                <BookOpen size={14} className="text-amber-400" />
                <span>Page Range</span>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <input
                  type="number"
                  min="1"
                  value={pageStart}
                  onChange={(e) => setPageStart(e.target.value)}
                  placeholder="Page start (15)"
                  className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition"
                />
                <input
                  type="number"
                  min="1"
                  value={pageEnd}
                  onChange={(e) => setPageEnd(e.target.value)}
                  placeholder="Page end (24)"
                  className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition"
                />
              </div>
            </div>
          )}

          {!isMedia && !isDocument && (
            <div className="space-y-1.5">
              <div className="flex items-center gap-1.5 text-xs font-semibold text-gray-300">
                <FileText size={14} className="text-amber-400" />
                <span>Line Range (Optional)</span>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <input
                  type="number"
                  min="1"
                  value={lineStart}
                  onChange={(e) => setLineStart(e.target.value)}
                  placeholder="Line start (1)"
                  className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition"
                />
                <input
                  type="number"
                  min="1"
                  value={lineEnd}
                  onChange={(e) => setLineEnd(e.target.value)}
                  placeholder="Line end (50)"
                  className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition"
                />
              </div>
            </div>
          )}

          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-semibold text-gray-300">Notes</label>
            <textarea
              rows={3}
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder="Summary or context of this segment..."
              className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition resize-none"
            />
          </div>

          <div className="flex items-center justify-end gap-3 pt-3 border-t border-[#1f232e]">
            <button
              type="button"
              onClick={onClose}
              disabled={loading}
              className="px-4 py-2 rounded-xl bg-[#181b23] hover:bg-[#202532] text-xs font-medium text-gray-300 transition"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading}
              className="px-5 py-2 rounded-xl bg-amber-500 hover:bg-amber-600 active:scale-[0.98] text-xs font-bold text-black transition shadow-md shadow-amber-500/20 disabled:opacity-50"
            >
              {loading ? "Adding..." : "Add Segment"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
