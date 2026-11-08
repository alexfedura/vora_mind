import React, { useState } from "react";
import { Paperclip, Highlighter, X } from "lucide-react";

export type HighlightColor = "yellow" | "green" | "blue" | "pink";

export interface PendingDocumentHighlight {
  content: string;
  pageNumber: number | null;
  positionData: unknown;
}

interface HighlightPopoverProps {
  selection: PendingDocumentHighlight | null;
  position: { x: number; y: number } | null;
  onSaveHighlight: (
    selection: PendingDocumentHighlight,
    color: HighlightColor,
    note: string,
  ) => Promise<string>;
  onSendToLibrary: (highlightId: string, content: string) => Promise<void>;
  onClose: () => void;
}

const colors: Array<{ value: HighlightColor; className: string; label: string }> = [
  { value: "yellow", className: "bg-yellow-300", label: "Yellow" },
  { value: "green", className: "bg-emerald-300", label: "Green" },
  { value: "blue", className: "bg-sky-300", label: "Blue" },
  { value: "pink", className: "bg-pink-300", label: "Pink" },
];

export const HighlightPopover: React.FC<HighlightPopoverProps> = ({
  selection,
  position,
  onSaveHighlight,
  onSendToLibrary,
  onClose,
}) => {
  const [color, setColor] = useState<HighlightColor>("yellow");
  const [note, setNote] = useState("");
  const [highlightId, setHighlightId] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!selection || !position) return null;

  const saveHighlight = async (): Promise<void> => {
    setIsSaving(true);
    setError(null);
    try {
      const id = await onSaveHighlight(selection, color, note);
      setHighlightId(id);
    } catch (cause) {
      console.error("Could not save document highlight:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSaving(false);
    }
  };

  const sendToLibrary = async (): Promise<void> => {
    if (!highlightId) return;
    setIsSaving(true);
    setError(null);
    try {
      await onSendToLibrary(highlightId, selection.content);
      onClose();
    } catch (cause) {
      console.error("Could not send highlight to Library:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div
      role="dialog"
      aria-label="Document highlight actions"
      className="fixed z-[120] w-64 rounded-xl border border-[#353b49] bg-[#171a22] p-3 text-zinc-200 shadow-2xl"
      style={{
        left: Math.max(12, Math.min(position.x, window.innerWidth - 280)),
        top: Math.max(12, Math.min(position.y + 10, window.innerHeight - 230)),
      }}
      onMouseDown={(event) => event.preventDefault()}
    >
      <div className="flex items-start gap-2">
        <p className="line-clamp-3 flex-1 text-[11px] leading-relaxed text-zinc-400">{selection.content}</p>
        <button type="button" onClick={onClose} aria-label="Close highlight menu" className="rounded p-1 text-zinc-500 hover:bg-[#292e39] hover:text-white">
          <X size={13} />
        </button>
      </div>
      <div className="mt-2 flex items-center gap-2">
        <span className="text-[10px] text-zinc-500">Color</span>
        {colors.map((option) => (
          <button
            key={option.value}
            type="button"
            aria-label={option.label}
            aria-pressed={color === option.value}
            onClick={() => setColor(option.value)}
            className={`h-4 w-4 rounded-full ${option.className} ${color === option.value ? "ring-2 ring-white ring-offset-1 ring-offset-[#171a22]" : ""}`}
          />
        ))}
      </div>
      <input
        value={note}
        onChange={(event) => setNote(event.currentTarget.value)}
        placeholder="Add a note (optional)"
        className="mt-2 w-full rounded-md border border-[#303644] bg-[#101217] px-2 py-1.5 text-[11px] outline-none focus:border-violet-500"
      />
      {error && <p role="alert" className="mt-2 text-[10px] text-rose-300">{error}</p>}
      <div className="mt-2 flex justify-end gap-2">
        {!highlightId ? (
          <button
            type="button"
            disabled={isSaving}
            onClick={() => void saveHighlight()}
            className="flex items-center gap-1.5 rounded-md bg-amber-400/15 px-2.5 py-1.5 text-[11px] font-medium text-amber-200 hover:bg-amber-400/25 disabled:opacity-50"
          >
            <Highlighter size={13} />
            {isSaving ? "Saving..." : "Highlight"}
          </button>
        ) : (
          <button
            type="button"
            disabled={isSaving}
            onClick={() => void sendToLibrary()}
            className="flex items-center gap-1.5 rounded-md bg-violet-500/20 px-2.5 py-1.5 text-[11px] font-medium text-violet-200 hover:bg-violet-500/30 disabled:opacity-50"
          >
            <Paperclip size={13} />
            {isSaving ? "Sending..." : "Send to Library"}
          </button>
        )}
      </div>
    </div>
  );
};
