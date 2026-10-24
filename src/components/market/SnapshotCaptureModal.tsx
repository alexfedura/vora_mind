import React, { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { acquireInteractionLock } from "@/lib/ui/interactionLock";
import type { NarrativeMode } from "@/lib/snapshot/types";

type CaptureMode = "STANDARD" | NarrativeMode;

interface Props {
  open: boolean;
  defaultName: string;
  frameCount: number;
  onClose: () => void;
  onSave: (input: { name: string; notes: string; tags: string[]; saveMode: "STANDARD" | NarrativeMode }) => Promise<void> | void;
}

export function SnapshotCaptureModal({ open, defaultName, frameCount, onClose, onSave }: Props): React.ReactElement | null {
  const [name, setName] = useState(defaultName);
  const [notes, setNotes] = useState("");
  const [tagsRaw, setTagsRaw] = useState("");
  const [saveMode, setSaveMode] = useState<CaptureMode>("STANDARD");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const nameInputRef = useRef<HTMLInputElement | null>(null);

  React.useEffect(() => {
    if (open) {
      setName(defaultName);
      setNotes("");
      setTagsRaw("");
      setSaveMode("STANDARD");
      setError(null);
      setSaving(false);
    }
  }, [open, defaultName]);

  // v0.83.2.1 — hold the global interaction lock while open so chart
  // pointer/keyboard handlers stay disabled behind the modal.
  useEffect(() => {
    if (!open) return;
    const release = acquireInteractionLock();
    return release;
  }, [open]);

  // v0.83.2.1 — autofocus the Name input on open.
  useEffect(() => {
    if (open) {
      const t = window.setTimeout(() => nameInputRef.current?.focus(), 0);
      return () => window.clearTimeout(t);
    }
  }, [open]);

  // v0.83.2.1 — Escape closes the modal (capture phase, stops chart Esc zoom-reset).
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [open, onClose]);

  if (!open) return null;

  const handleSubmit = async (e: React.FormEvent): Promise<void> => {
    e.preventDefault();
    const trimmed = name.trim();
    if (!trimmed) {
      setError("Name is required.");
      return;
    }
    const tags = tagsRaw
      .split(",")
      .map((t) => t.trim())
      .filter((t) => t.length > 0);
    const input = { name: trimmed, notes, tags, saveMode };
    setError(null);
    setSaving(true);
    try {
      await onSave(input);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setSaving(false);
    }
  };

  return createPortal(
    <div
      className="fixed inset-0 flex items-center justify-center bg-black/70 p-4"
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 9999,
        background: "rgba(0,0,0,0.65)",
        pointerEvents: "auto",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      }}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="w-full max-w-md bg-[#1e1e1e] border border-[#383838] rounded p-5 shadow-[0_8px_24px_rgba(0,0,0,0.45)] flex flex-col gap-4 text-[#d6dbe2]"
        style={{ pointerEvents: "auto" }}
        onMouseDown={(e) => e.stopPropagation()}
        onPointerDown={(e) => e.stopPropagation()}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-[#383838] pb-3">
          <div className="flex items-center gap-2 text-[#ff9800] font-semibold text-sm">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="18" height="18" fill="none">
              <path fill="currentColor" fillRule="evenodd" clipRule="evenodd" d="M9.5 6L8 8H5.5A1.5 1.5 0 0 0 4 9.5v10A1.5 1.5 0 0 0 5.5 21h17a1.5 1.5 0 0 0 1.5-1.5v-10A1.5 1.5 0 0 0 22.5 8H20l-1.5-2h-9zm-.4 1.83a.5.5 0 0 0 .4-.19l.75-1h6.5l.75 1a.5.5 0 0 0 .4.19h4.6a.5.5 0 0 1 .5.5v10a.5.5 0 0 1-.5.5h-17a.5.5 0 0 1-.5-.5v-10a.5.5 0 0 1 .5-.5h4.6z" />
              <path fill="currentColor" fillRule="evenodd" clipRule="evenodd" d="M14 18.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zm0-1a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z" />
            </svg>
            <span>Save Snapshot</span>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-1 hover:bg-[#2a2a2a] text-[#787b86] hover:text-[#d6dbe2] rounded transition"
            aria-label="Close"
          >
            ✕
          </button>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          <div className="flex flex-col gap-1.5">
            <label className="text-xs text-[#9a9a9a] font-medium" htmlFor="snapshot-save-mode">Save as</label>
            <select
              id="snapshot-save-mode"
              value={saveMode}
              onChange={(event) => {
                const value = event.target.value;
                if (value === "STANDARD" || value === "FULL_VIDEO" || value === "COMPACT_STEPS") {
                  setSaveMode(value);
                }
              }}
              className="bg-[#0f0f0f] border border-[#383838] focus:border-[#ff9800] rounded px-3 py-2 text-xs text-[#d6dbe2] focus:outline-none disabled:opacity-60"
            >
              <option value="STANDARD">Standard Snapshot</option>
              <option value="COMPACT_STEPS">Compact Steps</option>
              <option value="FULL_VIDEO">Full Narrative</option>
            </select>
          </div>
          <div className="flex flex-col gap-1.5">
            <label className="text-xs text-[#9a9a9a] font-medium">Name</label>
            <input
              ref={nameInputRef}
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="BTC 4h setup"
              className="bg-[#0f0f0f] border border-[#383838] focus:border-[#ff9800] rounded px-3 py-2 text-xs text-[#d6dbe2] focus:outline-none transition"
              style={{ pointerEvents: "auto" }}
              required
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-xs text-[#9a9a9a] font-medium">Notes</label>
            <textarea
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder="Optional notes about this setup..."
              rows={3}
              className="bg-[#0f0f0f] border border-[#383838] focus:border-[#ff9800] rounded px-3 py-2 text-xs text-[#d6dbe2] placeholder:text-[#787b86] focus:outline-none resize-none transition"
              style={{ pointerEvents: "auto" }}
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-xs text-[#9a9a9a] font-medium">Tags (comma-separated)</label>
            <input
              type="text"
              value={tagsRaw}
              onChange={(e) => setTagsRaw(e.target.value)}
              placeholder="scalp, xauusd, m3"
              className="bg-[#0f0f0f] border border-[#383838] focus:border-[#ff9800] rounded px-3 py-2 text-xs text-[#d6dbe2] focus:outline-none transition"
              style={{ pointerEvents: "auto" }}
            />
          </div>

          {error && <div className="text-xs text-[#ef5350]">{error}</div>}
          <div role="status" className="text-xs text-[#9a9a9a]">
            Recorded {frameCount} frames
          </div>

          <div className="flex items-center justify-end gap-2.5 pt-3 border-t border-[#383838]">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded bg-[#2a2a2a] hover:bg-[#333333] text-xs text-[#d6dbe2] transition"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={saving}
              className="px-4 py-2 rounded bg-[#ff9800] hover:bg-[#ffad33] text-xs font-semibold text-black transition disabled:opacity-50"
            >
              {saving ? "Saving..." : "Save"}
            </button>
          </div>
        </form>
      </div>
    </div>,
    document.body,
  );
}
