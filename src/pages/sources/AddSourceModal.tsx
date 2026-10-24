import React, { useState } from "react";
import { X, Plus, AlertCircle } from "lucide-react";
import { CreateSourceReq, srcCreateSource } from "@/source/services/sourceService";

interface AddSourceModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSourceCreated: (newSourceId: string) => void;
}

const SOURCE_KINDS = [
  { value: "video", label: "Video", icon: "📹" },
  { value: "audio", label: "Audio", icon: "🎙️" },
  { value: "book", label: "Book", icon: "📚" },
  { value: "doc", label: "Document", icon: "📄" },
  { value: "url", label: "Web / URL", icon: "🌐" },
  { value: "image", label: "Image", icon: "🖼️" },
];

export const AddSourceModal: React.FC<AddSourceModalProps> = ({
  isOpen,
  onClose,
  onSourceCreated,
}) => {
  const [kind, setKind] = useState("video");
  const [title, setTitle] = useState("");
  const [path, setPath] = useState("");
  const [author, setAuthor] = useState("");
  const [language, setLanguage] = useState("en");
  const [durationSec, setDurationSec] = useState<string>("");
  const [pageCount, setPageCount] = useState<string>("");
  const [notes, setNotes] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) {
      setError("Title is required");
      return;
    }
    setLoading(true);
    setError(null);
    const req: CreateSourceReq = {
      kind,
      title: title.trim(),
      path: path.trim() || null,
      author: author.trim() || null,
      language: language.trim() || null,
      duration_sec: (kind === "video" || kind === "audio") && durationSec ? parseInt(durationSec, 10) || null : null,
      page_count: (kind === "book" || kind === "doc") && pageCount ? parseInt(pageCount, 10) || null : null,
      notes: notes.trim() || null,
    };
    try {
      const newId = await srcCreateSource(req);
      setTitle("");
      setPath("");
      setAuthor("");
      setDurationSec("");
      setPageCount("");
      setNotes("");
      onSourceCreated(newId);
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const isMedia = kind === "video" || kind === "audio";
  const isDocument = kind === "book" || kind === "doc";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm">
      <div className="bg-[#12141a] border border-[#242835] rounded-2xl w-full max-w-lg p-6 shadow-2xl relative flex flex-col max-h-[90vh]">
        <div className="flex items-center justify-between pb-4 border-b border-[#1f232e]">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-amber-500 to-orange-600 flex items-center justify-center font-bold text-white shadow-md">
              <Plus size={18} />
            </div>
            <div>
              <h2 className="text-base font-bold text-gray-100">Add New Source</h2>
              <p className="text-xs text-gray-400">Register media, document, or web source</p>
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
            <label className="text-xs font-semibold text-gray-300">Source Kind</label>
            <div className="grid grid-cols-3 gap-2">
              {SOURCE_KINDS.map((item) => (
                <button
                  type="button"
                  key={item.value}
                  onClick={() => setKind(item.value)}
                  className={`flex items-center gap-2 px-3 py-2 rounded-xl text-xs font-medium border transition ${
                    kind === item.value
                      ? "bg-amber-500/20 border-amber-500 text-amber-300 shadow-sm shadow-amber-500/10"
                      : "bg-[#161821] border-[#222633] text-gray-400 hover:bg-[#1c1f2b] hover:text-gray-200"
                  }`}
                >
                  <span>{item.icon}</span>
                  <span>{item.label}</span>
                </button>
              ))}
            </div>
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-semibold text-gray-300">
              Title <span className="text-amber-400">*</span>
            </label>
            <input
              type="text"
              required
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="e.g. Stanford CS229 Lecture 1"
              className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition"
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-semibold text-gray-300">
              {kind === "url" ? "URL" : "File Path / URI"}
            </label>
            <input
              type="text"
              value={path}
              onChange={(e) => setPath(e.target.value)}
              placeholder={kind === "url" ? "https://..." : "D:/Media/cs229-01.mp4"}
              className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition"
            />
          </div>

  const isMedia = kind === "video" || kind === "audio";
          <div className="grid grid-cols-2 gap-3">
            <div className="flex flex-col gap-1.5">
              <label className="text-xs font-semibold text-gray-300">Author / Speaker</label>
              <input
                type="text"
                value={author}
                onChange={(e) => setAuthor(e.target.value)}
                placeholder="Andrew Ng"
                className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition"
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <label className="text-xs font-semibold text-gray-300">Language</label>
              <input
                type="text"
                value={language}
                onChange={(e) => setLanguage(e.target.value)}
                placeholder="en"
                className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition"
              />
            </div>
          </div>

          {isMedia && (
            <div className="flex flex-col gap-1.5">
              <label className="text-xs font-semibold text-gray-300">Duration (seconds)</label>
              <input
                type="number"
                min="0"
                value={durationSec}
                onChange={(e) => setDurationSec(e.target.value)}
                placeholder="e.g. 3600"
                className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition"
              />
            </div>
          )}

          {isDocument && (
            <div className="flex flex-col gap-1.5">
              <label className="text-xs font-semibold text-gray-300">Page Count</label>
              <input
                type="number"
                min="1"
                value={pageCount}
                onChange={(e) => setPageCount(e.target.value)}
                placeholder="e.g. 350"
                className="bg-[#161821] border border-[#222633] focus:border-amber-500/80 rounded-xl px-3 py-2 text-xs text-gray-100 placeholder-gray-500 outline-none transition"
              />
            </div>
          )}

          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-semibold text-gray-300">Notes</label>
            <textarea
              rows={3}
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder="Context, tags, key takeaways..."
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
              {loading ? "Creating..." : "Create Source"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
