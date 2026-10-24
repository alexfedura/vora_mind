// ═══════════════════════════════════════════════════════════════
// RectangleOverlay v7.1.1 — user-drawn rectangles + 8-handle resize
// + live click-click preview + orange snap marker.
// Client-only. NOT RBR/DBD boxes. Data space: ts (sec) + price.
// ═══════════════════════════════════════════════════════════════

import type {
  Background,
  IChartApi,
  ISeriesApi,
  ISeriesPrimitive,
  IPrimitivePaneView,
  IPrimitivePaneRenderer,
  SeriesType,
  Time,
  UTCTimestamp,
} from "lightweight-charts";
import { drawingTimeToCoordinate } from "./drawingCoordinates";
import type { DrawingAnchor, DrawingCandleSample, DrawingPresentation, FibGannToolId, GeometryToolId } from "./drawingTypes";

export interface DrawingRect extends DrawingPresentation {
  id: string;
  type: "rectangle";
  ts_start: number;
  ts_end: number;
  price_low: number;
  price_high: number;
  borderColor: string;
  fillColor: string | null;
  fillOpacity: number;
  lineWidth: number;
  selected: boolean;
  /** v0.82.0 — optional hover flag (external RectHover remains live path). */
  hovered?: boolean;
  /** v0.82.0 — locked drawings hide handles and cannot be dragged/resized. */
  locked?: boolean;
  /** v7.2.0 legacy compat (pre-rename fields, removed after migration). */
  color?: string;
  fill?: string | null;
}

/** v7.2.0 — hover state: rect id + handle under cursor (null = body hover). */
export interface RectHover {
  id: string;
  handle: RectHandleId | null;
}

export type RectHandleId = "NW" | "N" | "NE" | "E" | "SE" | "S" | "SW" | "W";
export const DRAWING_HANDLE_COLOR = "#2962ff";
export const DRAWING_HANDLE_RADIUS = 5;
export const DRAWING_HANDLE_IDLE_LINE_WIDTH = 2;
export const DRAWING_HANDLE_HOVER_LINE_WIDTH = 2.5;
export const DRAWING_HANDLE_SELECTED_LINE_WIDTH = 2;

export function getDrawingHandleBackground(ctx: CanvasRenderingContext2D, background: Background): string | CanvasGradient {
  if ("color" in background) return background.color;
  const gradient = ctx.createLinearGradient(0, 0, 0, ctx.canvas.height);
  gradient.addColorStop(0, background.topColor);
  gradient.addColorStop(1, background.bottomColor);
  return gradient;
}

export interface RectHandlePoint {
  id: RectHandleId;
  ts: number;
  price: number;
}

/** Data-space positions of the 8 handles for a rectangle. */
export function getRectHandlePositions(d: DrawingRect): RectHandlePoint[] {
  const midTs = (d.ts_start + d.ts_end) / 2;
  const midPrice = (d.price_low + d.price_high) / 2;
  return [
    { id: "NW", ts: d.ts_start, price: d.price_high },
    { id: "N", ts: midTs, price: d.price_high },
    { id: "NE", ts: d.ts_end, price: d.price_high },
    { id: "E", ts: d.ts_end, price: midPrice },
    { id: "SE", ts: d.ts_end, price: d.price_low },
    { id: "S", ts: midTs, price: d.price_low },
    { id: "SW", ts: d.ts_start, price: d.price_low },
    { id: "W", ts: d.ts_start, price: midPrice },
  ];
}

/** v7.1.1 — live preview rectangle (anchor → cursor) rendered with 8 handles. */
export interface RectPreview {
  anchorTs: number;
  anchorPrice: number;
  currentTs: number;
  currentPrice: number;
}

/** Screen-space centers of the 8 handles (null when off-screen). */
export function getRectHandleScreenPositions(
  d: DrawingRect,
  chart: IChartApi,
  series: ISeriesApi<SeriesType>,
): { id: RectHandleId; x: number; y: number }[] {
  const out: { id: RectHandleId; x: number; y: number }[] = [];
  const timeScale = chart.timeScale();
  const startX = drawingTimeToCoordinate(timeScale, d.ts_start);
  const endX = drawingTimeToCoordinate(timeScale, d.ts_end);
  for (const h of getRectHandlePositions(d)) {
    const x = h.id === "N" || h.id === "S"
      ? startX !== null && endX !== null ? (startX + endX) / 2 : null
      : drawingTimeToCoordinate(timeScale, h.ts);
    const y = series.priceToCoordinate(h.price);
    if (x == null || y == null) continue;
    out.push({ id: h.id, x, y });
  }
  return out;
}

export const HANDLE_CURSOR: Record<RectHandleId, string> = {
  NW: "nwse-resize",
  SE: "nwse-resize",
  NE: "nesw-resize",
  SW: "nesw-resize",
  N: "ns-resize",
  S: "ns-resize",
  E: "ew-resize",
  W: "ew-resize",
};

const CORNER_RADIUS = DRAWING_HANDLE_RADIUS;
const MID_SIZE = 9;
const MID_CORNER_RADIUS = 2;

function isCorner(id: RectHandleId): boolean {
  return id === "NW" || id === "NE" || id === "SE" || id === "SW";
}

/** v7.4.4 — trend-line live preview (anchor → cursor) before 2nd click. */
export interface TrendPreview {
  ts1: number;
  price1: number;
  ts2: number;
  price2: number;
  ts3?: number;
  price3?: number;
  stage?: "base" | "offset";
  tool?: "trendline" | "ray" | "info-line" | "extended-line" | "trend-angle" | "parallel-channel" | "pitchfork" | "schiff-pitchfork" | "modified-schiff-pitchfork" | FibGannToolId | GeometryToolId;
  geometryPoints?: DrawingAnchor[];
  geometryFixedCount?: number;
  geometrySourceBars?: DrawingCandleSample[];
  active: boolean;
}

/** v7.2.0 — hex (#rrggbb) + opacity → rgba() fill string. */
export function hexWithOpacity(hex: string, opacity: number): string {
  const m = /^#([0-9a-fA-F]{6})$/.exec(hex);
  if (!m) return hex;
  const n = parseInt(m[1], 16);
  const r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  const o = Math.max(0, Math.min(1, opacity));
  return `rgba(${r},${g},${b},${o})`;
}

/** v7.2.0 — parse a legacy `fill` string into { fillColor, fillOpacity }. */
export function parseLegacyFill(fill: string | null | undefined): { fillColor: string | null; fillOpacity: number } {
  if (!fill) return { fillColor: null, fillOpacity: 0 };
  const m = /^rgba\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*([0-9.]+)\s*\)$/i.exec(fill);
  if (m) {
    const r = Math.max(0, Math.min(255, parseInt(m[1], 10)));
    const g = Math.max(0, Math.min(255, parseInt(m[2], 10)));
    const b = Math.max(0, Math.min(255, parseInt(m[3], 10)));
    const toHex = (v: number): string => v.toString(16).padStart(2, "0");
    return { fillColor: `#${toHex(r)}${toHex(g)}${toHex(b)}`, fillOpacity: Math.max(0, Math.min(1, parseFloat(m[4]))) };
  }
  if (/^#[0-9a-fA-F]{6}$/.test(fill)) return { fillColor: fill, fillOpacity: 0.15 };
  return { fillColor: null, fillOpacity: 0 };
}

/** v7.2.0 — migrate a persisted/legacy rect to the new data model. */
export function migrateDrawingRect(d: DrawingRect): DrawingRect {
  if (d.borderColor && d.fillOpacity !== undefined && d.fillColor !== undefined) {
    return { ...d, fillOpacity: Math.max(0, Math.min(1, d.fillOpacity)) };
  }
  const legacy = parseLegacyFill(d.fill ?? null);
  return {
    ...d,
    borderColor: d.borderColor ?? d.color ?? "#ffb74d",
    fillColor: d.fillColor ?? legacy.fillColor,
    fillOpacity: d.fillOpacity ?? legacy.fillOpacity,
  };
}

type PaneRenderTarget = Parameters<IPrimitivePaneRenderer["draw"]>[0];

class RectPaneRenderer implements IPrimitivePaneRenderer {
  constructor(
    private _drawings: DrawingRect[],
    private _series: ISeriesApi<SeriesType>,
    private _chart: IChartApi,
    private _preview: RectPreview | null,
    private _snap: { t: number; p: number } | null,
    private _hover: RectHover | null,
  ) {}

  draw(target: PaneRenderTarget): void {
    if (this._drawings.length === 0 && !this._preview && !this._snap) return;
    target.useBitmapCoordinateSpace((scope) => {
      const ctx = scope.context;
      const hr = scope.horizontalPixelRatio;
      const vr = scope.verticalPixelRatio;
      const k = Math.min(hr, vr);
      const handleBackground = getDrawingHandleBackground(ctx, this._chart.options().layout.background);
      const ts = this._chart.timeScale();
      ctx.save();
      try {
        const mediaW = scope.mediaSize.width;
        const mediaH = scope.mediaSize.height;
        for (const d of this._drawings) {
          if (d.hidden) continue;
          const xLr = drawingTimeToCoordinate(ts, d.ts_start);
          const xRr = drawingTimeToCoordinate(ts, d.ts_end);
          if (xLr === null || xRr === null) continue;
          // Fully off-screen horizontally → skip.
          if (Math.max(xLr, xRr) < 0 || Math.min(xLr, xRr) > mediaW) continue;
          const xL = xLr;
          const xR = xRr;
          const yT = this._series.priceToCoordinate(d.price_high);
          const yB = this._series.priceToCoordinate(d.price_low);
          if (yT == null || yB == null) continue;
          void mediaH;
          const rx = Math.min(xL, xR);
          const ry = Math.min(yT, yB);
          const rw = Math.abs(xR - xL);
          const rh = Math.abs(yB - yT);
          // v7.2.0 Part E — fill from fillColor + fillOpacity.
          if (d.fillColor && d.fillOpacity > 0) {
            ctx.fillStyle = hexWithOpacity(d.fillColor, d.fillOpacity);
            ctx.fillRect(rx * hr, ry * vr, rw * hr, rh * vr);
          }
          ctx.strokeStyle = d.borderColor;
          // v0.82.0 — dash style support.
          ctx.setLineDash(d.style === "dashed" ? [6 * k, 4 * k] : d.style === "dotted" ? [2 * k, 3 * k] : []);
          // v0.82.0 A4 — locked: plain shape, no glow/halo.
          void d.locked;
          ctx.lineWidth = Math.max(d.lineWidth, 1) * k;
          ctx.globalAlpha = d.colorOpacity ?? 1;
          ctx.strokeRect(rx * hr, ry * vr, rw * hr, rh * vr);
          ctx.globalAlpha = 1;
          ctx.setLineDash([]);
          if (d.text) {
            ctx.save();
            ctx.beginPath();
            ctx.rect(rx * hr, ry * vr, rw * hr, rh * vr);
            ctx.clip();
            const fontSize = Math.max(8, Math.min(72, d.textSize ?? 14)) * k;
            ctx.font = `${d.textItalic ? "italic " : ""}${d.textBold ? "bold " : ""}${fontSize}px sans-serif`;
            ctx.fillStyle = d.textColor ?? d.borderColor;
            ctx.globalAlpha = d.textOpacity ?? 1;
            ctx.textAlign = d.textAlignHorizontal ?? "left";
            ctx.textBaseline = d.textAlignVertical === "top" ? "top" : d.textAlignVertical === "bottom" ? "bottom" : "middle";
            const inset = 6 * k;
            const textX = d.textAlignHorizontal === "center" ? (rx + rw / 2) * hr : d.textAlignHorizontal === "right" ? (rx + rw) * hr - inset : rx * hr + inset;
            const lineHeight = fontSize * 1.25;
            const lines = d.text.split(/\r?\n/);
            const textHeight = lineHeight * lines.length;
            const textY = d.textAlignVertical === "top" ? ry * vr + inset : d.textAlignVertical === "bottom" ? (ry + rh) * vr - inset - textHeight + lineHeight : (ry + rh / 2) * vr - textHeight / 2 + lineHeight / 2;
            lines.forEach((line, index) => ctx.fillText(line, textX, textY + index * lineHeight, Math.max(0, rw * hr - inset * 2)));
            ctx.restore();
          }
          // v7.2.0 C2 — handles visible only when hovered OR selected.
          // v0.82.0 A1 — locked hides handles even when selected. A2 hovered flag supported alongside external hover.
          const isHov = this._hover?.id === d.id;
          const showHandles = (d.selected === true || d.hovered === true || isHov) && d.locked !== true;
          if (showHandles) {
            const handles = getRectHandleScreenPositions(d, this._chart, this._series);
            for (const h of handles) {
              ctx.strokeStyle = DRAWING_HANDLE_COLOR;
              const isHovered = isHov || d.hovered === true;
              ctx.lineWidth = (d.selected ? DRAWING_HANDLE_SELECTED_LINE_WIDTH : isHovered ? DRAWING_HANDLE_HOVER_LINE_WIDTH : DRAWING_HANDLE_IDLE_LINE_WIDTH) * k;
              ctx.fillStyle = handleBackground;
              if (isCorner(h.id)) {
                ctx.beginPath();
                ctx.arc(h.x * hr, h.y * vr, CORNER_RADIUS * k, 0, Math.PI * 2);
                ctx.fill();
                ctx.stroke();
              } else {
                const hx = (h.x - MID_SIZE / 2) * hr;
                const hy = (h.y - MID_SIZE / 2) * vr;
                ctx.beginPath();
                ctx.roundRect(hx, hy, MID_SIZE * hr, MID_SIZE * vr, MID_CORNER_RADIUS * k);
                ctx.fill();
                ctx.stroke();
              }
            }
          }
        }
        // v7.1.1 B3 — live preview with 8 handles before second click.
        if (this._preview) {
          const pv = this._preview;
          const t0 = Math.min(pv.anchorTs, pv.currentTs);
          const t1 = Math.max(pv.anchorTs, pv.currentTs);
          const p0 = Math.min(pv.anchorPrice, pv.currentPrice);
          const p1 = Math.max(pv.anchorPrice, pv.currentPrice);
          const previewRect: DrawingRect = {
            id: "__preview__", type: "rectangle",
            ts_start: t0, ts_end: t1, price_low: p0, price_high: p1,
            borderColor: "#2962ff", fillColor: "#2962ff", fillOpacity: 0.10, lineWidth: 2,
            selected: true,
          };
          const xL = drawingTimeToCoordinate(ts, t0);
          const xR = drawingTimeToCoordinate(ts, t1);
          const yT = this._series.priceToCoordinate(p1);
          const yB = this._series.priceToCoordinate(p0);
          if (xL != null && xR != null && yT != null && yB != null) {
            const rx = Math.min(xL, xR);
            const ry = Math.min(yT, yB);
            const rw = Math.abs(xR - xL);
            const rh = Math.abs(yB - yT);
            ctx.fillStyle = hexWithOpacity(previewRect.fillColor as string, previewRect.fillOpacity);
            ctx.fillRect(rx * hr, ry * vr, rw * hr, rh * vr);
            ctx.strokeStyle = previewRect.borderColor;
            ctx.lineWidth = previewRect.lineWidth * k;
            ctx.strokeRect(rx * hr, ry * vr, rw * hr, rh * vr);
            const handles = getRectHandleScreenPositions(previewRect, this._chart, this._series);
            for (const h of handles) {
              ctx.strokeStyle = DRAWING_HANDLE_COLOR;
              ctx.lineWidth = DRAWING_HANDLE_SELECTED_LINE_WIDTH * k;
              ctx.fillStyle = handleBackground;
              if (isCorner(h.id)) {
                ctx.beginPath();
                ctx.arc(h.x * hr, h.y * vr, CORNER_RADIUS * k, 0, Math.PI * 2);
                ctx.fill();
                ctx.stroke();
              } else {
                const hx = (h.x - MID_SIZE / 2) * hr;
                const hy = (h.y - MID_SIZE / 2) * vr;
                ctx.beginPath();
                ctx.roundRect(hx, hy, MID_SIZE * hr, MID_SIZE * vr, MID_CORNER_RADIUS * k);
                ctx.fill();
                ctx.stroke();
              }
            }
          }
        }
        // v7.1.1 D — orange snap marker at snapped OHLC point.
        if (this._snap) {
          const sx = ts.timeToCoordinate(this._snap.t as UTCTimestamp as Time);
          const sy = this._series.priceToCoordinate(this._snap.p);
          if (sx != null && sy != null) {
            ctx.strokeStyle = "#ffb74d";
            ctx.lineWidth = 1.5 * k;
            ctx.beginPath();
            ctx.arc(sx * hr, sy * vr, 5 * k, 0, Math.PI * 2);
            ctx.stroke();
          }
        }
      } finally {
        ctx.restore();
      }
    });
  }
}

class RectPaneView implements IPrimitivePaneView {
  constructor(
    private _drawings: DrawingRect[],
    private _series: ISeriesApi<SeriesType>,
    private _chart: IChartApi,
    private _preview: RectPreview | null,
    private _snap: { t: number; p: number } | null,
    private _hover: RectHover | null,
  ) {}
  zOrder(): "top" | "normal" | "bottom" { return "top"; }
  renderer(): IPrimitivePaneRenderer {
    return new RectPaneRenderer(this._drawings, this._series, this._chart, this._preview, this._snap, this._hover);
  }
}

export class RectangleOverlay implements ISeriesPrimitive<Time> {
  private _drawings: DrawingRect[] = [];
  private _preview: RectPreview | null = null;
  private _snap: { t: number; p: number } | null = null;
  private _hover: RectHover | null = null;
  private _series: ISeriesApi<SeriesType>;
  private _chart: IChartApi;
  private _requestUpdate?: () => void;

  constructor(chart: IChartApi, series: ISeriesApi<SeriesType>) {
    this._chart = chart;
    this._series = series;
  }

  attached(param: { requestUpdate: () => void }): void {
    this._requestUpdate = param.requestUpdate;
  }
  detached(): void {
    this._requestUpdate = undefined;
    this._drawings = [];
  }
  setDrawings(ds: DrawingRect[]): void {
    this._drawings = ds.slice();
    this._requestUpdate?.();
  }
  /** v7.1.1 — live preview (null = hide). */
  setPreview(p: RectPreview | null): void {
    this._preview = p;
    this._requestUpdate?.();
  }
  /** v7.1.1 — snap marker (null = hide). Only set while magnet is strong. */
  setSnap(s: { t: number; p: number } | null): void {
    this._snap = s;
    this._requestUpdate?.();
  }
  /** v7.2.0 C2/C3 — hover state for handle visibility + hover fill. */
  setHover(h: RectHover | null): void {
    this._hover = h;
    this._requestUpdate?.();
  }
  updateAllViews(): void { /* no-op */ }
  paneViews(): readonly IPrimitivePaneView[] {
    return [new RectPaneView(this._drawings, this._series, this._chart, this._preview, this._snap, this._hover)];
  }
}
