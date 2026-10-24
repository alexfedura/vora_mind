// ═══════════════════════════════════════════════════════════════
// RectColorPalette v7.2.0 Part D — floating color palette for a selected rect.
// EXACTLY 2 color controls (border + fill) + 1 opacity slider. No other controls.
// ═══════════════════════════════════════════════════════════════

import { useState } from "react";
import type { DrawingRect } from "@/lib/market/RectangleOverlay";
import { RECT_SWATCH_COLORS } from "@/lib/market/rectInteraction";
import { isValidHex } from "@/lib/market/colorUtils";

interface Props {
  rect: DrawingRect;
  onBorderColor: (c: string) => void;
  onFillColor: (c: string | null) => void;
  onOpacity: (v: number) => void;
}

type OpenPicker = "border" | "fill" | null;

export function RectColorPalette({ rect, onBorderColor, onFillColor, onOpacity }: Props) {
  const [open, setOpen] = useState<OpenPicker>(null);

  const renderSwatches = (apply: (c: string) => void, allowNone: boolean) => (
    <div
      className="grid grid-cols-8 gap-1 p-1.5 rounded"
      style={{ background: "#181818", border: "1px solid #2a2a2a" }}
      onMouseDown={(e) => e.stopPropagation()}
      onClick={(e) => e.stopPropagation()}
    >
      {RECT_SWATCH_COLORS.map((c) => (
        <button
          key={c}
          type="button"
          title={c}
          onMouseDown={(e) => e.stopPropagation()}
          onClick={(e) => { e.stopPropagation(); apply(c); setOpen(null); }}
          className="w-5 h-5 rounded-sm border border-black/40 hover:scale-110 transition"
          style={{ background: c }}
        />
      ))}
      {allowNone && (
        <button
          type="button"
          title="No fill"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={(e) => { e.stopPropagation(); apply("__none__"); setOpen(null); }}
          className="w-5 h-5 rounded-sm border border-black/40 text-[11px] leading-none text-white/70 hover:scale-110 transition"
          style={{
            background:
              "repeating-conic-gradient(#666 0% 25%, #333 0% 50%) 0 0 / 8px 8px",
          }}
        >
          ✕
        </button>
      )}
    </div>
  );

  const applyFill = (c: string): void => {
    if (c === "__none__") onFillColor(null);
    else if (isValidHex(c)) onFillColor(c);
  };

  const fillPreview = rect.fillColor && rect.fillOpacity > 0 ? rect.fillColor : null;

  return (
    <div
      className="flex items-center gap-2 px-1.5 py-1.5 rounded"
      style={{ background: "#181818", border: "1px solid #2a2a2a", borderRadius: 4, padding: 6 }}
      onMouseDown={(e) => e.stopPropagation()}
      onClick={(e) => e.stopPropagation()}
    >
      {/* 1 — Border color control */}
      <div className="relative">
        <button
          type="button"
          title="Border color"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={(e) => { e.stopPropagation(); setOpen((v) => (v === "border" ? null : "border")); }}
          className="w-6 h-6 rounded border border-white/30 hover:scale-105 transition"
          style={{ background: rect.borderColor }}
        />
        {open === "border" && (
          <div className="absolute left-0 top-7 z-50">
            {renderSwatches((c) => { if (isValidHex(c)) onBorderColor(c); }, false)}
          </div>
        )}
      </div>
      {/* 2 — Fill color control (checkerboard when null) */}
      <div className="relative">
        <button
          type="button"
          title="Fill color"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={(e) => { e.stopPropagation(); setOpen((v) => (v === "fill" ? null : "fill")); }}
          className="w-6 h-6 rounded border border-white/30 hover:scale-105 transition"
          style={fillPreview
            ? { background: fillPreview }
            : { background: "repeating-conic-gradient(#666 0% 25%, #333 0% 50%) 0 0 / 8px 8px" }}
        />
        {open === "fill" && (
          <div className="absolute left-0 top-7 z-50">
            {renderSwatches(applyFill, true)}
          </div>
        )}
      </div>
      {/* 3 — Opacity slider (0–100%, controls fillOpacity) */}
      <input
        type="range"
        min={0}
        max={100}
        value={Math.round(rect.fillOpacity * 100)}
        onMouseDown={(e) => e.stopPropagation()}
        onClick={(e) => e.stopPropagation()}
        onChange={(e) => onOpacity(Number(e.target.value) / 100)}
        title={`Fill opacity ${Math.round(rect.fillOpacity * 100)}%`}
        className="w-20 accent-[#ffb74d]"
      />
    </div>
  );
}
