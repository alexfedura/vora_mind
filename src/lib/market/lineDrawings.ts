// ═══════════════════════════════════════════════════════════════
// lineDrawings v7.4.4 — shared data model for vline/hline/hray/trendline.
// Same localStorage key family as rectangles (vora.drawings.<symbol>).
// ═══════════════════════════════════════════════════════════════

import type { DrawingRect } from "./RectangleOverlay";
import type { DrawingAnchor, DrawingCandleSample, DrawingIconCategory, DrawingPresentation, FibGannToolId, GeometryToolId, LineStyle } from "./drawingTypes";
export type { DrawingPresentation, LineStyle } from "./drawingTypes";

export interface DrawingVerticalLine extends DrawingPresentation {
  id: string;
  type: "vline";
  ts: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  /** v0.82.0 — optional per-drawing hover flag (external LineHover remains live path). */
  hovered?: boolean;
  /** v0.82.0 — locked drawings hide handles and cannot be dragged/resized. */
  locked?: boolean;
  /** v0.82.0 — line dash style (default "solid"). */
  style?: LineStyle;
}

export interface DrawingHorizontalLine extends DrawingPresentation {
  id: string;
  type: "hline";
  price: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
  style?: LineStyle;
}

export interface DrawingHorizontalRay extends DrawingPresentation {
  id: string;
  type: "hray";
  ts: number;
  price: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
  style?: LineStyle;
}

export interface DrawingTrendLine extends DrawingPresentation {
  id: string;
  type: "trendline";
  ts1: number;
  price1: number;
  ts2: number;
  price2: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
  style?: LineStyle;
}

export interface DrawingRay extends DrawingPresentation {
  id: string;
  type: "ray";
  ts1: number;
  price1: number;
  ts2: number;
  price2: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
  style?: LineStyle;
}

export interface DrawingInfoLine extends DrawingPresentation {
  id: string;
  type: "info-line";
  ts1: number;
  price1: number;
  ts2: number;
  price2: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
  style?: LineStyle;
}

export interface DrawingExtendedLine extends DrawingPresentation {
  id: string;
  type: "extended-line";
  ts1: number;
  price1: number;
  ts2: number;
  price2: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
  style?: LineStyle;
}

export interface DrawingTrendAngle extends DrawingPresentation {
  id: string;
  type: "trend-angle";
  ts1: number;
  price1: number;
  ts2: number;
  price2: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
  style?: LineStyle;
}

export interface DrawingCrossline extends DrawingPresentation {
  id: string;
  type: "crossline";
  ts: number;
  price: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
  style?: LineStyle;
}

export interface DrawingParallelChannel extends DrawingPresentation {
  id: string;
  type: "parallel-channel";
  ts1: number;
  price1: number;
  ts2: number;
  price2: number;
  ts3: number;
  price3: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
  style?: LineStyle;
}

export interface DrawingPitchfork extends DrawingPresentation {
  id: string;
  type: "pitchfork";
  variant?: "standard" | "schiff" | "modified-schiff";
  ts1: number;
  price1: number;
  ts2: number;
  price2: number;
  ts3: number;
  price3: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
  style?: LineStyle;
}

export interface DrawingRegressionTrend extends DrawingPresentation {
  id: string;
  type: "regression-trend";
  ts1: number;
  price1: number;
  ts2: number;
  price2: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
  style?: LineStyle;
}

export interface DrawingFibGann extends DrawingPresentation {
  id: string;
  type: "fib-gann";
  tool: FibGannToolId;
  ts1: number;
  price1: number;
  ts2: number;
  price2: number;
  ts3?: number;
  price3?: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
}

export interface DrawingIcon extends DrawingPresentation {
  id: string;
  type: "icon";
  category: DrawingIconCategory;
  symbol: string;
  ts: number;
  price: number;
  size: number;
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
}

export interface DrawingGeometry extends DrawingPresentation {
  id: string;
  type: "geometry";
  tool: GeometryToolId;
  points: DrawingAnchor[];
  segmentStyles?: Record<number, DrawingGeometrySegmentStyle>;
  selectedSegments?: number[];
  startArrow?: boolean;
  endArrow?: boolean;
  sourceBars?: DrawingCandleSample[];
  color: string;
  lineWidth: number;
  selected: boolean;
  hovered?: boolean;
  locked?: boolean;
  style?: LineStyle;
}

export interface GradedRectangle extends DrawingPresentation {
  id: string;
  type: "graded-rectangle";
  structureType: "RBR" | "DBD";
  anchorPrice: number;
  anchorTime: number;
  startTime: number;
  endTime: number;
  topPrice: number;
  bottomPrice: number;
  baselineSide: "top" | "bottom";
  correctionStartPrice?: number;
  correctionStartTime?: number;
  selected: boolean;
  hidden?: boolean;
  locked?: boolean;
  colorOpacity?: number;
  lineWidth: number;
}

export interface DrawingGeometrySegmentStyle {
  color?: string;
  colorOpacity?: number;
  lineWidth?: number;
  style?: LineStyle;
}

export type DrawingGeometryStylePatch = Partial<DrawingGeometrySegmentStyle>;

export function getGeometrySegmentStyle(
  drawing: DrawingGeometry,
  segmentIndex: number,
): Required<DrawingGeometrySegmentStyle> {
  const override = drawing.segmentStyles?.[segmentIndex];
  return {
    color: override?.color ?? drawing.color,
    colorOpacity: override?.colorOpacity ?? drawing.colorOpacity ?? 1,
    lineWidth: override?.lineWidth ?? drawing.lineWidth,
    style: override?.style ?? drawing.style ?? "solid",
  };
}

export function applyGeometryPathStyle(
  drawing: DrawingGeometry,
  patch: DrawingGeometryStylePatch,
): DrawingGeometry {
  const selectedSegments = drawing.selectedSegments?.filter(
    (index) => Number.isInteger(index) && index >= 0 && index < drawing.points.length - 1,
  );
  if (!selectedSegments?.length) {
    const segmentStyles = Object.fromEntries(
      Object.entries(drawing.segmentStyles ?? {}).flatMap(([index, style]) => {
        const remaining = { ...style };
        for (const key of Object.keys(patch) as (keyof DrawingGeometryStylePatch)[]) {
          delete remaining[key];
        }
        return Object.keys(remaining).length > 0 ? [[index, remaining]] : [];
      }),
    );
    return {
      ...drawing,
      ...patch,
      segmentStyles: Object.keys(segmentStyles).length > 0 ? segmentStyles : undefined,
      selectedSegments: undefined,
    };
  }

  const segmentStyles = { ...drawing.segmentStyles };
  for (const index of selectedSegments) {
    segmentStyles[index] = { ...segmentStyles[index], ...patch };
  }
  return { ...drawing, segmentStyles };
}

/** v7.4.4 — shared drawings union (rectangle + line tools). */
export type DrawingAny =
  | DrawingRect
  | GradedRectangle
  | DrawingVerticalLine
  | DrawingHorizontalLine
  | DrawingHorizontalRay
  | DrawingTrendLine
  | DrawingRay
  | DrawingInfoLine
  | DrawingExtendedLine
  | DrawingTrendAngle
  | DrawingCrossline
  | DrawingParallelChannel
  | DrawingPitchfork
  | DrawingRegressionTrend
  | DrawingFibGann
  | DrawingIcon
  | DrawingGeometry;

/** v7.4.4 — migrate any persisted drawing (rect legacy handled by caller). */
export function migrateDrawingAny(d: DrawingAny): DrawingAny {
  if ((d as DrawingRect).type === "rectangle") return d;
  const dd = d as unknown as Record<string, unknown>;
  if (typeof dd["lineWidth"] !== "number" || !Number.isFinite(dd["lineWidth"] as number)) {
    dd["lineWidth"] = 1;
  }
  if (typeof dd["color"] !== "string" || !(dd["color"] as string)) {
    dd["color"] = "#ffb74d";
  }
  if (typeof dd["selected"] !== "boolean") {
    dd["selected"] = false;
  }
  if (typeof dd["locked"] !== "boolean") {
    dd["locked"] = false;
  }
  const st = dd["style"] as unknown;
  if (st !== "solid" && st !== "dashed" && st !== "dotted") {
    dd["style"] = "solid";
  }
  return d;
}

/** v7.4.4 — Policy Panel prefs (localStorage-backed). */
import { getSetting } from "@/shared/store/useSettingsStore";

export const LINE_COLOR_KEY = "vora.drawings.line_color";
export const RECT_WIDTH_KEY = "vora.drawings.rect_width";
export const HLINE_WIDTH_KEY = "vora.drawings.hline_width";
export const VLINE_WIDTH_KEY = "vora.drawings.vline_width";
export const HRAY_WIDTH_KEY = "vora.drawings.hray_width";
export const TREND_WIDTH_KEY = "vora.drawings.trendline_width";
/** v0.82.0 — default line dash style (solid|dashed|dotted). */
export const LINE_STYLE_KEY = "vora.drawings.line_style";

export function readLineStyle(): LineStyle {
  try {
    const v = getSetting(LINE_STYLE_KEY);
    if (v === "dashed" || v === "dotted" || v === "solid") return v;
  } catch { /* ignore */ }
  return "solid";
}

export function readLineColor(): string {
  try {
    const v = getSetting(LINE_COLOR_KEY);
    if (v && /^#[0-9a-fA-F]{6}$/.test(v)) return v;
  } catch { /* ignore */ }
  return "#ffb74d";
}

export function readLineWidth(key: string, fallback: number): number {
  try {
    const v = getSetting(key);
    if (v != null) {
      const n = Math.round(Number(v));
      if (Number.isFinite(n)) return Math.max(1, Math.min(10, n));
    }
  } catch { /* ignore */ }
  return fallback;
}

export type AnnotationToolId = "annotation-text" | "annotation-note" | "annotation-price-note" | "annotation-pin" | "annotation-table" | "annotation-callout" | "annotation-comment" | "annotation-price-label" | "annotation-signpost" | "annotation-flag-mark";

export const ANNOTATION_DRAWING_PRESETS: Record<AnnotationToolId, { label: string; symbol: string }> = {
  "annotation-text": { label: "Text", symbol: "T" },
  "annotation-note": { label: "Note", symbol: "▤" },
  "annotation-price-note": { label: "Price note", symbol: "$" },
  "annotation-pin": { label: "Pin", symbol: "📍" },
  "annotation-table": { label: "Table", symbol: "▦" },
  "annotation-callout": { label: "Callout", symbol: "▱" },
  "annotation-comment": { label: "Comment", symbol: "◯" },
  "annotation-price-label": { label: "Price label", symbol: "$" },
  "annotation-signpost": { label: "Signpost", symbol: "☆" },
  "annotation-flag-mark": { label: "Flag mark", symbol: "⚑" },
};

export function isAnnotationToolId(id: string): id is AnnotationToolId {
  return Object.prototype.hasOwnProperty.call(ANNOTATION_DRAWING_PRESETS, id);
}

export type LineToolId = "vertline" | "horzline" | "horzray" | "trendline" | "ray" | "info-line" | "extended-line" | "trend-angle" | "crossline" | "parallel-channel" | "pitchfork" | "schiff-pitchfork" | "modified-schiff-pitchfork" | "regression-trend" | "icon" | AnnotationToolId | FibGannToolId | GeometryToolId;
