// LineOverlay v7.4.4 - vline/hline/hray/trendline renderer.
import type { IChartApi, ISeriesApi, ISeriesPrimitive, IPrimitivePaneView, IPrimitivePaneRenderer, SeriesType, Time, UTCTimestamp } from "lightweight-charts";
import type { DrawingAny, DrawingFibGann, DrawingHorizontalLine, DrawingHorizontalRay, DrawingTrendLine, DrawingVerticalLine } from "./lineDrawings";
import type { DrawingCandleSample } from "./drawingTypes";
import {
  DRAWING_HANDLE_COLOR,
  DRAWING_HANDLE_HOVER_LINE_WIDTH,
  DRAWING_HANDLE_IDLE_LINE_WIDTH,
  DRAWING_HANDLE_RADIUS,
  DRAWING_HANDLE_SELECTED_LINE_WIDTH,
  getDrawingHandleBackground,
  type TrendPreview,
} from "./RectangleOverlay";
import { pitchforkMedianOrigin } from "./pitchforkGeometry";
import { buildFibGannGeometry, fibGannAnchorCount, isFibGannTool, type ScreenAnchor } from "./fibGannGeometry";
import { buildGeometryPath, isGeometryToolId, type GeometryPath } from "./geometryToolGeometry";
import { buildForecastMeasurementGeometry, isForecastMeasurementTool, type ForecastGeometry } from "./forecastMeasurementGeometry";
import { drawingTimeToCoordinate } from "./drawingCoordinates";
export interface LineHover { id: string; handle: string | null; }
type PT = Parameters<IPrimitivePaneRenderer["draw"]>[0];
function nn(v: unknown): number { return v as unknown as number; }

function paintGeometryArrowheads(
  ctx: CanvasRenderingContext2D,
  geometry: GeometryPath,
  horizontalRatio: number,
  verticalRatio: number,
  scale: number,
  size = 7,
): void {
  if (geometry.points.length < 2) return;
  const first = geometry.points[0];
  const second = geometry.points[1];
  const end = geometry.points[geometry.points.length - 1];
  const previous = geometry.points[geometry.points.length - 2];
  const draw = (tip: ScreenAnchor | undefined, adjacent: ScreenAnchor | undefined): void => {
    if (!tip || !adjacent) return;
    const angle = Math.atan2(tip.y - adjacent.y, tip.x - adjacent.x);
    const head = size * scale;
    ctx.moveTo(tip.x * horizontalRatio, tip.y * verticalRatio);
    ctx.lineTo((tip.x - Math.cos(angle - Math.PI / 6) * head) * horizontalRatio, (tip.y - Math.sin(angle - Math.PI / 6) * head) * verticalRatio);
    ctx.moveTo(tip.x * horizontalRatio, tip.y * verticalRatio);
    ctx.lineTo((tip.x - Math.cos(angle + Math.PI / 6) * head) * horizontalRatio, (tip.y - Math.sin(angle + Math.PI / 6) * head) * verticalRatio);
  };
  const hasStartArrow = geometry.startArrowHead === true;
  const hasEndArrow = geometry.endArrowHead ?? geometry.arrowHead;
  if (!hasStartArrow && !hasEndArrow) return;
  ctx.beginPath();
  if (hasStartArrow) draw(first, second);
  if (hasEndArrow) draw(end, previous);
  ctx.stroke();
}

function rayViewportEnd(x1: number, y1: number, x2: number, y2: number, width: number, height: number): { x: number; y: number } {
  const dx = x2 - x1;
  const dy = y2 - y1;
  if (dx === 0 && dy === 0) return { x: x2, y: y2 };
  const intersections: number[] = [];
  if (dx > 0) intersections.push((width - x1) / dx);
  if (dx < 0) intersections.push((0 - x1) / dx);
  if (dy > 0) intersections.push((height - y1) / dy);
  if (dy < 0) intersections.push((0 - y1) / dy);
  const exits = intersections.filter((value) => value >= 0);
  const t = exits.length === 0 ? 1 : Math.max(1, Math.min(...exits));
  return { x: x1 + dx * t, y: y1 + dy * t };
}

function parallelChannelOffset(ts1: number, price1: number, ts2: number, price2: number, ts3: number, price3: number): number {
  const duration = ts2 - ts1;
  const baseAtThird = duration === 0 ? price1 : price1 + ((ts3 - ts1) / duration) * (price2 - price1);
  return price3 - baseAtThird;
}

function extendedViewportSegment(x1: number, y1: number, x2: number, y2: number, width: number, height: number): { start: { x: number; y: number }; end: { x: number; y: number } } {
  const dx = x2 - x1;
  const dy = y2 - y1;
  let lower = Number.NEGATIVE_INFINITY;
  let upper = Number.POSITIVE_INFINITY;
  const clip = (origin: number, delta: number, minimum: number, maximum: number): boolean => {
    if (delta === 0) return origin >= minimum && origin <= maximum;
    const first = (minimum - origin) / delta;
    const second = (maximum - origin) / delta;
    lower = Math.max(lower, Math.min(first, second));
    upper = Math.min(upper, Math.max(first, second));
    return lower <= upper;
  };
  if (!clip(x1, dx, 0, width) || !clip(y1, dy, 0, height)) {
    return { start: { x: x1, y: y1 }, end: { x: x2, y: y2 } };
  }
  return {
    start: { x: x1 + dx * lower, y: y1 + dy * lower },
    end: { x: x1 + dx * upper, y: y1 + dy * upper },
  };
}

export class LineOverlay implements ISeriesPrimitive<Time> {
  private _d: DrawingAny[] = [];
  private _candles: DrawingCandleSample[] = [];
  private _p: TrendPreview | null = null;
  private _h: LineHover | null = null;
  private _m = 0;
  private _u?: () => void;
  constructor(private _c: IChartApi, private _s: ISeriesApi<SeriesType>) {}
  attached(p: { requestUpdate: () => void }): void { this._u = p.requestUpdate; }
  detached(): void { this._u = undefined; this._d = []; this._candles = []; }
  setDrawings(ds: DrawingAny[]): void { this._d = ds.filter(Boolean).slice(); this._u?.(); }
  setCandles(candles: readonly DrawingCandleSample[]): void { this._candles = candles.slice(); this._u?.(); }
  setPreview(p: TrendPreview | null): void { this._p = p; this._u?.(); }
  setHover(h: LineHover | null): void { this._h = h; this._u?.(); }
  setMidPrice(v: number): void { this._m = v; this._u?.(); }
  updateAllViews(): void { /* no-op */ }
  paneViews(): readonly IPrimitivePaneView[] {
    const d = this._d; const candles = this._candles; const s = this._s; const c = this._c;
    const p = this._p; const h = this._h; const m = this._m;
    const dot = (ctx: CanvasRenderingContext2D, x: number, y: number, k: number, selected: boolean, hovered: boolean): void => {
      ctx.save();
      ctx.strokeStyle = DRAWING_HANDLE_COLOR;
      ctx.lineWidth = (hovered ? DRAWING_HANDLE_HOVER_LINE_WIDTH : selected ? DRAWING_HANDLE_SELECTED_LINE_WIDTH : DRAWING_HANDLE_IDLE_LINE_WIDTH) * k;
      ctx.fillStyle = getDrawingHandleBackground(ctx, c.options().layout.background);
      const radius = hovered ? DRAWING_HANDLE_RADIUS + 1 : DRAWING_HANDLE_RADIUS;
      ctx.beginPath(); ctx.arc(x, y, radius * k, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
      ctx.restore();
    };
    const squareHandle = (ctx: CanvasRenderingContext2D, x: number, y: number, k: number): void => {
      const size = 12 * k;
      ctx.save();
      ctx.fillStyle = getDrawingHandleBackground(ctx, c.options().layout.background);
      ctx.strokeStyle = DRAWING_HANDLE_COLOR;
      ctx.lineWidth = 2 * k;
      ctx.beginPath();
      ctx.rect(x - size / 2, y - size / 2, size, size);
      ctx.fill();
      ctx.stroke();
      ctx.restore();
    };
    const paintText = (ctx: CanvasRenderingContext2D, drawing: DrawingAny, x: number, y: number, maxWidth: number, hr: number, vr: number, k: number): void => {
      if (!drawing.text) return;
      ctx.save();
      const fontSize = Math.max(8, Math.min(72, drawing.textSize ?? 14)) * k;
      ctx.font = `${drawing.textItalic ? "italic " : ""}${drawing.textBold ? "bold " : ""}${fontSize}px sans-serif`;
      ctx.fillStyle = drawing.textColor ?? (drawing.type === "rectangle" ? drawing.borderColor : drawing.color);
      ctx.globalAlpha *= drawing.textOpacity ?? 1;
      ctx.textAlign = drawing.textAlignHorizontal ?? "center";
      ctx.textBaseline = drawing.textAlignVertical === "top" ? "top" : drawing.textAlignVertical === "bottom" ? "bottom" : "middle";
      const lines = drawing.text.split(/\r?\n/);
      const lineHeight = fontSize * 1.25;
      const top = drawing.textAlignVertical === "top" ? y : drawing.textAlignVertical === "bottom" ? y - lineHeight * (lines.length - 1) : y - lineHeight * (lines.length - 1) / 2;
      lines.forEach((line, index) => ctx.fillText(line, x * hr, (top + index * lineHeight) * vr, Math.max(0, maxWidth * hr)));
      ctx.restore();
    };
    const paintForecast = (ctx: CanvasRenderingContext2D, geometry: ForecastGeometry, hr: number, vr: number, k: number, opacity: number): void => {
      ctx.save();
      ctx.globalAlpha *= opacity;
      for (const shape of geometry.paths) {
        const first = shape.points[0];
        if (!first) continue;
        ctx.beginPath();
        ctx.moveTo(first.x * hr, first.y * vr);
        for (const point of shape.points.slice(1)) ctx.lineTo(point.x * hr, point.y * vr);
        if (shape.closed) ctx.closePath();
        if (shape.fill) { ctx.fillStyle = shape.fill; ctx.fill(); }
        ctx.strokeStyle = shape.stroke ?? "#82a5e8";
        ctx.lineWidth = 1.5 * k;
        ctx.setLineDash(shape.dashed ? [5 * k, 4 * k] : []);
        ctx.stroke();
      }
      for (const bar of geometry.candles) {
        const color = bar.ghost ? "rgba(184, 196, 219, 0.62)" : bar.up ? "#4db68b" : "#dc665e";
        const candleWidth = bar.ghost ? 4 * k : Math.max(2 * k, 4 * k);
        ctx.strokeStyle = color;
        ctx.fillStyle = bar.ghost ? "rgba(184, 196, 219, 0.12)" : color;
        ctx.lineWidth = k;
        ctx.beginPath();
        ctx.moveTo(bar.x * hr, bar.highY * vr);
        ctx.lineTo(bar.x * hr, bar.lowY * vr);
        ctx.stroke();
        ctx.fillRect(bar.x * hr - candleWidth / 2, Math.min(bar.openY, bar.closeY) * vr, candleWidth, Math.max(k, Math.abs(bar.closeY - bar.openY) * vr));
      }
      ctx.setLineDash([]);
      for (const label of geometry.labels) {
        const fontSize = (label.fontSize ?? 11) * k;
        ctx.font = `${label.bold ? "600 " : ""}${fontSize}px sans-serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "alphabetic";
        const x = label.x * hr;
        const y = label.y * vr;
        const lines = label.text.split(/\r?\n/);
        const lineHeight = fontSize * 1.2;
        const paddingX = 5 * k;
        const paddingY = 4 * k;
        const textWidth = Math.max(...lines.map((line) => ctx.measureText(line).width)) + paddingX * 2;
        const boxHeight = lineHeight * lines.length + paddingY * 2;
        const boxX = x - textWidth / 2;
        const boxY = y - boxHeight / 2;
        if (label.background) {
          ctx.beginPath();
          ctx.roundRect(boxX, boxY, textWidth, boxHeight, 5 * k);
          ctx.fillStyle = label.background;
          ctx.fill();
          if (label.border) {
            ctx.strokeStyle = label.border;
            ctx.lineWidth = k;
            ctx.stroke();
          }
        } else {
          ctx.fillStyle = "rgba(26, 29, 33, 0.94)";
          ctx.fillRect(boxX, boxY, textWidth, boxHeight);
        }
        ctx.fillStyle = label.color;
        lines.forEach((line, index) => {
          const baseline = y - (lines.length - 1) * lineHeight / 2 + index * lineHeight + fontSize * 0.35;
          ctx.fillText(line, x, baseline);
        });
      }
      ctx.restore();
    };
    const paintLines = (ctx: CanvasRenderingContext2D, hr: number, vr: number, k: number, W: number, H: number): void => {
      const tsc = c.timeScale();
      const drawingX = (time: Time): number | null => drawingTimeToCoordinate(tsc, Number(time));
      for (const q of d) {
        if (!q || q.type === "rectangle" || q.hidden) continue;
        const qrec = q as unknown as { style?: string };
        const opacity = q.colorOpacity ?? 1;
        ctx.shadowBlur = 0;
        ctx.setLineDash(qrec.style === "dashed" ? [6 * k, 4 * k] : qrec.style === "dotted" ? [2 * k, 3 * k] : []);
        if (q.type === "icon") {
          const x = drawingX(q.ts as UTCTimestamp as Time);
          const y = s.priceToCoordinate(q.price);
          if (x == null || y == null) continue;
          const size = Math.max(12, Math.min(72, q.size)) * k;
          ctx.save();
          ctx.globalAlpha *= opacity;
          ctx.font = `${size}px "Segoe UI Emoji", "Apple Color Emoji", sans-serif`;
          ctx.textAlign = "center";
          ctx.textBaseline = "middle";
          ctx.fillStyle = q.color;
          ctx.fillText(q.symbol, nn(x) * hr, nn(y) * vr);
          ctx.restore();
          paintText(ctx, q, nn(x), nn(y) + size / 2 + 4, W, hr, vr, k);
          continue;
        }
        if (q.type === "geometry") {
          if (isForecastMeasurementTool(q.tool)) {
            const forecast = buildForecastMeasurementGeometry(q.tool, q.points, candles, q.sourceBars ?? [], (anchor) => {
              const x = drawingX(anchor.ts as UTCTimestamp as Time);
              const y = s.priceToCoordinate(anchor.price);
              return x === null || y === null ? null : { x: nn(x), y: nn(y) };
            }, W, tsc.options().barSpacing ?? 6);
            if (forecast) {
              paintForecast(ctx, forecast, hr, vr, k, q.colorOpacity ?? 1);
              continue;
            }
          }
          const path = buildGeometryPath(q.tool, q.points, (anchor) => {
            const x = drawingX(anchor.ts as UTCTimestamp as Time);
            const y = s.priceToCoordinate(anchor.price);
            return x === null || y === null ? null : { x: nn(x), y: nn(y) };
          }, q.tool === "path" ? { startArrow: q.startArrow, endArrow: q.endArrow } : undefined);
          if (!path || path.points.length === 0) continue;
          ctx.save();
          ctx.globalAlpha *= opacity * (q.tool === "highlighter" ? 0.28 : 1);
          ctx.strokeStyle = q.color;
          ctx.lineWidth = Math.max(q.tool === "highlighter" ? 10 : 1, q.lineWidth) * k;
          const first = path.points[0];
          if (first) {
            ctx.beginPath();
            ctx.moveTo(first.x * hr, first.y * vr);
            for (const point of path.points.slice(1)) ctx.lineTo(point.x * hr, point.y * vr);
            if (path.closed) ctx.closePath();
            ctx.stroke();
            paintGeometryArrowheads(ctx, path, hr, vr, k, q.tool === "arrow-marker" ? 10 : 7);
          }
          ctx.restore();
          const xs = path.points.map((point) => point.x);
          const ys = path.points.map((point) => point.y);
          const textWidth = Math.max(40, Math.max(...xs) - Math.min(...xs));
          paintText(ctx, q, (Math.min(...xs) + Math.max(...xs)) / 2, (Math.min(...ys) + Math.max(...ys)) / 2, textWidth, hr, vr, k);
          continue;
        }
        if (q.type === "vline") {
          const v = q as DrawingVerticalLine;
          const x = drawingX(v.ts as UTCTimestamp as Time);
          if (x == null) continue;
          if (nn(x) < 0 || nn(x) > W) continue;
          ctx.save(); ctx.globalAlpha *= opacity;
          ctx.strokeStyle = v.color; ctx.lineWidth = Math.max(1, v.lineWidth) * k;
          // v0.82.0 A4 — selection halo only when selected (locked shows plain, no glow).
          ctx.beginPath(); ctx.moveTo(nn(x) * hr, 0); ctx.lineTo(nn(x) * hr, H * vr); ctx.stroke(); ctx.shadowBlur = 0;
          ctx.restore();
          paintText(ctx, v, nn(x), H / 2, 180, hr, vr, k);
        } else if (q.type === "fib-gann") {
          const fibDrawing: DrawingFibGann = q;
          const dataAnchors = [
            { ts: fibDrawing.ts1, price: fibDrawing.price1 },
            { ts: fibDrawing.ts2, price: fibDrawing.price2 },
            ...(fibDrawing.ts3 !== undefined && fibDrawing.price3 !== undefined ? [{ ts: fibDrawing.ts3, price: fibDrawing.price3 }] : []),
          ];
          const anchors: ScreenAnchor[] = [];
          for (const anchor of dataAnchors) {
            const x = drawingX(anchor.ts as UTCTimestamp as Time);
            const y = s.priceToCoordinate(anchor.price);
            if (x == null || y == null) break;
            anchors.push({ x: nn(x), y: nn(y) });
          }
          if (anchors.length < fibGannAnchorCount(fibDrawing.tool)) continue;
          const geometry = buildFibGannGeometry(fibDrawing.tool, anchors, W, H);
          ctx.save(); ctx.globalAlpha *= opacity;
          ctx.strokeStyle = fibDrawing.color;
          ctx.lineWidth = Math.max(1, fibDrawing.lineWidth) * k;
          for (const path of geometry.paths) {
            const first = path[0];
            if (!first || path.length < 2) continue;
            ctx.beginPath();
            ctx.moveTo(first.x * hr, first.y * vr);
            for (const point of path.slice(1)) ctx.lineTo(point.x * hr, point.y * vr);
            ctx.stroke();
          }
          ctx.shadowBlur = 0;
          ctx.setLineDash([]);
          for (const label of geometry.labels) {
            ctx.font = `${10 * k}px sans-serif`;
            ctx.textAlign = "right";
            ctx.textBaseline = "middle";
            const x = label.x * hr;
            const y = label.y * vr;
            const labelWidth = ctx.measureText(label.text).width + 8 * k;
            ctx.fillStyle = "rgba(30,30,30,0.88)";
            ctx.fillRect(x - labelWidth, y - 8 * k, labelWidth, 16 * k);
            ctx.fillStyle = fibDrawing.color;
            ctx.fillText(label.text, x - 4 * k, y);
          }
          ctx.restore();
        } else if (q.type === "crossline") {
          const x = drawingX(q.ts as UTCTimestamp as Time);
          const y = s.priceToCoordinate(q.price);
          if (x == null || y == null) continue;
          ctx.save(); ctx.globalAlpha *= opacity;
          ctx.strokeStyle = q.color; ctx.lineWidth = Math.max(1, q.lineWidth) * k;
          ctx.beginPath();
          ctx.moveTo(nn(x) * hr, 0); ctx.lineTo(nn(x) * hr, H * vr);
          ctx.moveTo(0, nn(y) * vr); ctx.lineTo(W * hr, nn(y) * vr);
          ctx.stroke(); ctx.shadowBlur = 0; ctx.restore();
          paintText(ctx, q, nn(x), nn(y), 180, hr, vr, k);
        } else if (q.type === "parallel-channel") {
          const channel = q as unknown as { ts1: number; price1: number; ts2: number; price2: number; ts3: number; price3: number };
          const x1 = drawingX(channel.ts1 as UTCTimestamp as Time);
          const x2 = drawingX(channel.ts2 as UTCTimestamp as Time);
          const y1 = s.priceToCoordinate(channel.price1);
          const y2 = s.priceToCoordinate(channel.price2);
          const offset = parallelChannelOffset(channel.ts1, channel.price1, channel.ts2, channel.price2, channel.ts3, channel.price3);
          const y3 = s.priceToCoordinate(channel.price1 + offset);
          const y4 = s.priceToCoordinate(channel.price2 + offset);
          if (x1 == null || x2 == null || y1 == null || y2 == null || y3 == null || y4 == null) continue;
          ctx.save(); ctx.globalAlpha *= opacity;
          ctx.strokeStyle = q.color; ctx.lineWidth = Math.max(1, q.lineWidth) * k;
          ctx.beginPath();
          ctx.moveTo(nn(x1) * hr, nn(y1) * vr); ctx.lineTo(nn(x2) * hr, nn(y2) * vr);
          ctx.lineTo(nn(x2) * hr, nn(y4) * vr); ctx.lineTo(nn(x1) * hr, nn(y3) * vr);
          ctx.closePath(); ctx.stroke(); ctx.shadowBlur = 0; ctx.restore();
          paintText(ctx, q, (nn(x1) + nn(x2)) / 2, (nn(y1) + nn(y2) + nn(y3) + nn(y4)) / 4, Math.abs(nn(x2) - nn(x1)), hr, vr, k);
        } else if (q.type === "pitchfork") {
          const fork = q;
          const origin = pitchforkMedianOrigin({ ts: fork.ts1, price: fork.price1 }, { ts: fork.ts2, price: fork.price2 }, fork.variant);
          const x1 = drawingX(origin.ts as UTCTimestamp as Time);
          const x2 = drawingX(fork.ts2 as UTCTimestamp as Time);
          const x3 = drawingX(fork.ts3 as UTCTimestamp as Time);
          const y1 = s.priceToCoordinate(origin.price);
          const y2 = s.priceToCoordinate(fork.price2);
          const y3 = s.priceToCoordinate(fork.price3);
          if (x1 == null || x2 == null || x3 == null || y1 == null || y2 == null || y3 == null) continue;
          const midX = (nn(x2) + nn(x3)) / 2;
          const midY = (nn(y2) + nn(y3)) / 2;
          const dx = midX - nn(x1);
          const dy = midY - nn(y1);
          const ends = [
            rayViewportEnd(nn(x1), nn(y1), midX, midY, W, H),
            rayViewportEnd(nn(x2), nn(y2), nn(x2) + dx, nn(y2) + dy, W, H),
            rayViewportEnd(nn(x3), nn(y3), nn(x3) + dx, nn(y3) + dy, W, H),
          ];
          const starts = [[nn(x1), nn(y1)], [nn(x2), nn(y2)], [nn(x3), nn(y3)]];
          ctx.save(); ctx.globalAlpha *= opacity;
          ctx.strokeStyle = q.color; ctx.lineWidth = Math.max(1, q.lineWidth) * k;
          ctx.beginPath();
          starts.forEach(([x, y], index) => { ctx.moveTo(x * hr, y * vr); ctx.lineTo(ends[index].x * hr, ends[index].y * vr); });
          ctx.stroke(); ctx.shadowBlur = 0; ctx.restore();
        } else if (q.type === "hline") {
          const qh = q as DrawingHorizontalLine;
          const y = s.priceToCoordinate(qh.price);
          if (y == null) continue;
          ctx.save(); ctx.globalAlpha *= opacity;
          ctx.strokeStyle = qh.color; ctx.lineWidth = Math.max(1, qh.lineWidth) * k;
          ctx.beginPath(); ctx.moveTo(0, nn(y) * vr); ctx.lineTo(W * hr, nn(y) * vr); ctx.stroke(); ctx.shadowBlur = 0;
          ctx.restore();
          paintText(ctx, qh, W / 2, nn(y), W, hr, vr, k);
        } else if (q.type === "hray") {
          const qr = q as DrawingHorizontalRay;
          const x = drawingX(qr.ts as UTCTimestamp as Time);
          const y = s.priceToCoordinate(qr.price);
          if (x == null || y == null) continue;
          const x0 = Math.max(0, Math.min(W, nn(x)));
          ctx.save(); ctx.globalAlpha *= opacity;
          ctx.strokeStyle = qr.color; ctx.lineWidth = Math.max(1, qr.lineWidth) * k;
          ctx.beginPath(); ctx.moveTo(x0 * hr, nn(y) * vr); ctx.lineTo(W * hr, nn(y) * vr); ctx.stroke(); ctx.shadowBlur = 0;
          ctx.restore();
          paintText(ctx, qr, (x0 + W) / 2, nn(y), W - x0, hr, vr, k);
        } else if (q.type === "trendline" || q.type === "ray" || q.type === "info-line" || q.type === "extended-line" || q.type === "trend-angle") {
          const qt = q as DrawingTrendLine;
          const x1 = drawingX(qt.ts1 as UTCTimestamp as Time);
          const x2 = drawingX(qt.ts2 as UTCTimestamp as Time);
          const y1 = s.priceToCoordinate(qt.price1);
          const y2 = s.priceToCoordinate(qt.price2);
          if (x1 == null || x2 == null || y1 == null || y2 == null) continue;
          ctx.save(); ctx.globalAlpha *= opacity;
          ctx.strokeStyle = qt.color; ctx.lineWidth = Math.max(1, qt.lineWidth) * k;
          const extended = q.type === "extended-line" ? extendedViewportSegment(nn(x1), nn(y1), nn(x2), nn(y2), W, H) : null;
          const start = extended?.start ?? { x: nn(x1), y: nn(y1) };
          const end = q.type === "ray" ? rayViewportEnd(nn(x1), nn(y1), nn(x2), nn(y2), W, H) : extended?.end ?? { x: nn(x2), y: nn(y2) };
          ctx.beginPath(); ctx.moveTo(start.x * hr, start.y * vr); ctx.lineTo(end.x * hr, end.y * vr); ctx.stroke(); ctx.shadowBlur = 0;
          ctx.restore();
          paintText(ctx, qt, (start.x + end.x) / 2, (start.y + end.y) / 2, Math.hypot(end.x - start.x, end.y - start.y), hr, vr, k);
          if (q.type === "info-line") {
            const delta = qt.price2 - qt.price1;
            const percent = qt.price1 === 0 ? 0 : (delta / Math.abs(qt.price1)) * 100;
            const elapsed = Math.abs(qt.ts2 - qt.ts1);
            const label = `${delta >= 0 ? "+" : ""}${delta.toFixed(2)} (${percent.toFixed(2)}%) · ${elapsed}s`;
            const labelX = ((nn(x1) + nn(x2)) / 2) * hr;
            const labelY = ((nn(y1) + nn(y2)) / 2) * vr - 12 * k;
            ctx.save();
            ctx.font = `${11 * k}px sans-serif`;
            ctx.textAlign = "center";
            ctx.textBaseline = "middle";
            const labelWidth = ctx.measureText(label).width + 10 * k;
            ctx.fillStyle = "rgba(30,30,30,0.94)";
            ctx.fillRect(labelX - labelWidth / 2, labelY - 9 * k, labelWidth, 18 * k);
            ctx.fillStyle = qt.color;
            ctx.fillText(label, labelX, labelY);
            ctx.restore();
          }
          if (q.type === "trend-angle") {
            const dx = nn(x2) - nn(x1);
            const dy = nn(y2) - nn(y1);
            const angle = Math.atan2(-dy, dx) * 180 / Math.PI;
            const label = `${angle.toFixed(1)}°`;
            const originX = nn(x1) * hr;
            const originY = nn(y1) * vr;
            const radius = 15 * k;
            ctx.save();
            ctx.strokeStyle = qt.color;
            ctx.lineWidth = Math.max(1, qt.lineWidth) * k;
            ctx.beginPath();
            ctx.arc(originX, originY, radius, 0, Math.atan2(dy, dx));
            ctx.stroke();
            ctx.font = `${11 * k}px sans-serif`;
            ctx.textAlign = "center";
            ctx.textBaseline = "middle";
            const labelX = (nn(x1) + nn(x2)) / 2 * hr;
            const labelY = (nn(y1) + nn(y2)) / 2 * vr - 12 * k;
            const labelWidth = ctx.measureText(label).width + 10 * k;
            ctx.fillStyle = "rgba(30,30,30,0.94)";
            ctx.fillRect(labelX - labelWidth / 2, labelY - 9 * k, labelWidth, 18 * k);
            ctx.fillStyle = qt.color;
            ctx.fillText(label, labelX, labelY);
            ctx.restore();
          }
        }
        ctx.setLineDash([]);
      }
    };
    const paintPreview = (ctx: CanvasRenderingContext2D, hr: number, vr: number, k: number): void => {
      if (!p || !p.active) return;
      const tsc = c.timeScale();
      const drawingX = (time: Time): number | null => drawingTimeToCoordinate(tsc, Number(time));
      if (p.tool && isForecastMeasurementTool(p.tool) && p.geometryPoints) {
        const forecast = buildForecastMeasurementGeometry(p.tool, p.geometryPoints, candles, p.geometrySourceBars ?? [], (anchor) => {
          const x = drawingX(anchor.ts as UTCTimestamp as Time);
          const y = s.priceToCoordinate(anchor.price);
          return x === null || y === null ? null : { x: nn(x), y: nn(y) };
        }, c.chartElement().clientWidth, tsc.options().barSpacing ?? 6);
        if (forecast) paintForecast(ctx, forecast, hr, vr, k, 0.72);
        return;
      }
      if (p.tool && isGeometryToolId(p.tool) && p.geometryPoints) {
        const geometry = buildGeometryPath(p.tool, p.geometryPoints, (anchor) => {
          const x = drawingX(anchor.ts as UTCTimestamp as Time);
          const y = s.priceToCoordinate(anchor.price);
          return x === null || y === null ? null : { x: nn(x), y: nn(y) };
        }, p.tool === "path" ? { startArrow: false, endArrow: true } : undefined);
        if (!geometry || geometry.points.length === 0) return;
        const first = geometry.points[0];
        if (!first) return;
        ctx.strokeStyle = "#2962ff";
        ctx.lineWidth = (p.tool === "highlighter" ? 10 : 2) * k;
        ctx.beginPath();
        ctx.moveTo(first.x * hr, first.y * vr);
        for (const point of geometry.points.slice(1)) ctx.lineTo(point.x * hr, point.y * vr);
        if (geometry.closed) ctx.closePath();
        ctx.stroke();
        paintGeometryArrowheads(ctx, geometry, hr, vr, k);
        return;
      }
      if (p.tool && isFibGannTool(p.tool)) {
        const dataAnchors = [
          { ts: p.ts1, price: p.price1 },
          { ts: p.ts2, price: p.price2 },
          ...(p.ts3 !== undefined && p.price3 !== undefined ? [{ ts: p.ts3, price: p.price3 }] : []),
        ];
        const anchors: ScreenAnchor[] = [];
        for (const anchor of dataAnchors) {
          const x = drawingX(anchor.ts as UTCTimestamp as Time);
          const y = s.priceToCoordinate(anchor.price);
          if (x == null || y == null) break;
          anchors.push({ x: nn(x), y: nn(y) });
        }
        if (anchors.length < fibGannAnchorCount(p.tool)) return;
        const geometry = buildFibGannGeometry(p.tool, anchors, c.chartElement().clientWidth, c.chartElement().clientHeight);
        ctx.strokeStyle = "#2962ff"; ctx.lineWidth = 2 * k; ctx.shadowBlur = 0;
        for (const path of geometry.paths) {
          const first = path[0];
          if (!first || path.length < 2) continue;
          ctx.beginPath();
          ctx.moveTo(first.x * hr, first.y * vr);
          for (const point of path.slice(1)) ctx.lineTo(point.x * hr, point.y * vr);
          ctx.stroke();
        }
        return;
      }
      if (p.tool === "parallel-channel") {
        const x1 = drawingX(p.ts1 as UTCTimestamp as Time);
        const x2 = drawingX(p.ts2 as UTCTimestamp as Time);
        const y1 = s.priceToCoordinate(p.price1);
        const y2 = s.priceToCoordinate(p.price2);
        if (x1 == null || x2 == null || y1 == null || y2 == null) return;
        ctx.strokeStyle = "#2962ff"; ctx.lineWidth = 2 * k; ctx.shadowBlur = 0;
        ctx.beginPath(); ctx.moveTo(nn(x1) * hr, nn(y1) * vr); ctx.lineTo(nn(x2) * hr, nn(y2) * vr);
        if (p.stage === "offset" && p.ts3 !== undefined && p.price3 !== undefined) {
          const offset = parallelChannelOffset(p.ts1, p.price1, p.ts2, p.price2, p.ts3, p.price3);
          const y3 = s.priceToCoordinate(p.price1 + offset);
          const y4 = s.priceToCoordinate(p.price2 + offset);
          if (y3 != null && y4 != null) {
            ctx.lineTo(nn(x2) * hr, nn(y4) * vr); ctx.lineTo(nn(x1) * hr, nn(y3) * vr); ctx.closePath();
          }
        }
        ctx.stroke();
        return;
      }
      if (p.tool === "pitchfork") {
        const origin = pitchforkMedianOrigin({ ts: p.ts1, price: p.price1 }, { ts: p.ts2, price: p.price2 }, "standard");
        const x1 = drawingX(origin.ts as UTCTimestamp as Time);
        const x2 = drawingX(p.ts2 as UTCTimestamp as Time);
        const y1 = s.priceToCoordinate(p.price1);
        const y2 = s.priceToCoordinate(p.price2);
        if (x1 == null || x2 == null || y1 == null || y2 == null) return;
        ctx.strokeStyle = "#2962ff"; ctx.lineWidth = 2 * k; ctx.shadowBlur = 0;
        ctx.beginPath(); ctx.moveTo(nn(x1) * hr, nn(y1) * vr); ctx.lineTo(nn(x2) * hr, nn(y2) * vr);
        if (p.stage === "offset" && p.ts3 !== undefined && p.price3 !== undefined) {
          const x3 = drawingX(p.ts3 as UTCTimestamp as Time);
          const y3 = s.priceToCoordinate(p.price3);
          if (x3 != null && y3 != null) {
            const midX = (nn(x2) + nn(x3)) / 2;
            const midY = (nn(y2) + nn(y3)) / 2;
            ctx.moveTo(nn(x1) * hr, nn(y1) * vr); ctx.lineTo(midX * hr, midY * vr);
            ctx.moveTo(nn(x2) * hr, nn(y2) * vr); ctx.lineTo(nn(x2) * hr + (midX - nn(x1)) * hr, nn(y2) * vr + (midY - nn(y1)) * vr);
            ctx.moveTo(nn(x3) * hr, nn(y3) * vr); ctx.lineTo(nn(x3) * hr + (midX - nn(x1)) * hr, nn(y3) * vr + (midY - nn(y1)) * vr);
          }
        }
        ctx.stroke();
        return;
      }
      if (p.tool === "schiff-pitchfork" || p.tool === "modified-schiff-pitchfork") {
        const variant = p.tool === "schiff-pitchfork" ? "schiff" : "modified-schiff";
        const origin = pitchforkMedianOrigin({ ts: p.ts1, price: p.price1 }, { ts: p.ts2, price: p.price2 }, variant);
        const x1 = drawingX(origin.ts as UTCTimestamp as Time);
        const x2 = drawingX(p.ts2 as UTCTimestamp as Time);
        const y1 = s.priceToCoordinate(origin.price);
        const y2 = s.priceToCoordinate(p.price2);
        if (x1 == null || x2 == null || y1 == null || y2 == null) return;
        ctx.strokeStyle = "#2962ff"; ctx.lineWidth = 2 * k; ctx.shadowBlur = 0;
        ctx.beginPath(); ctx.moveTo(nn(x1) * hr, nn(y1) * vr); ctx.lineTo(nn(x2) * hr, nn(y2) * vr);
        if (p.stage === "offset" && p.ts3 !== undefined && p.price3 !== undefined) {
          const x3 = drawingX(p.ts3 as UTCTimestamp as Time);
          const y3 = s.priceToCoordinate(p.price3);
          if (x3 != null && y3 != null) {
            const midX = (nn(x2) + nn(x3)) / 2;
            const midY = (nn(y2) + nn(y3)) / 2;
            ctx.moveTo(nn(x1) * hr, nn(y1) * vr); ctx.lineTo(midX * hr, midY * vr);
            ctx.moveTo(nn(x2) * hr, nn(y2) * vr); ctx.lineTo(nn(x2) * hr + (midX - nn(x1)) * hr, nn(y2) * vr + (midY - nn(y1)) * vr);
            ctx.moveTo(nn(x3) * hr, nn(y3) * vr); ctx.lineTo(nn(x3) * hr + (midX - nn(x1)) * hr, nn(y3) * vr + (midY - nn(y1)) * vr);
          }
        }
        ctx.stroke();
        return;
      }
      const x1 = drawingX(p.ts1 as UTCTimestamp as Time);
      const x2 = drawingX(p.ts2 as UTCTimestamp as Time);
      const y1 = s.priceToCoordinate(p.price1);
      const y2 = s.priceToCoordinate(p.price2);
      if (x1 == null || x2 == null || y1 == null || y2 == null) return;
      ctx.strokeStyle = "#2962ff"; ctx.lineWidth = 2 * k; ctx.shadowBlur = 0;
      const width = c.chartElement().clientWidth;
      const height = c.chartElement().clientHeight;
      const extended = p.tool === "extended-line" ? extendedViewportSegment(nn(x1), nn(y1), nn(x2), nn(y2), width, height) : null;
      const start = extended?.start ?? { x: nn(x1), y: nn(y1) };
      const end = p.tool === "ray" ? rayViewportEnd(nn(x1), nn(y1), nn(x2), nn(y2), width, height) : extended?.end ?? { x: nn(x2), y: nn(y2) };
      ctx.beginPath(); ctx.moveTo(start.x * hr, start.y * vr); ctx.lineTo(end.x * hr, end.y * vr); ctx.stroke();
    };
    const paintHandles = (ctx: CanvasRenderingContext2D, hr: number, vr: number, k: number, W: number): void => {
      const tsc = c.timeScale();
      const drawingX = (time: Time): number | null => drawingTimeToCoordinate(tsc, Number(time));
      let midTs: number | null = null;
      try {
        const r = c.timeScale().getVisibleRange();
        if (r && typeof r.from === "number" && typeof r.to === "number") midTs = (r.from + r.to) / 2;
      } catch { /* ignore */ }
      for (const q of d) {
        if (!q || q.type === "rectangle" || q.hidden) continue;
        // v0.82.0 A1 — handles visible only when selected or hovered; locked hides handles even when selected.
        const qopt = q as unknown as { hovered?: boolean; locked?: boolean };
        const showHandles = (q.selected === true || q.hovered === true || h?.id === q.id) && qopt.locked !== true;
        if (!showHandles) continue;
        const isHovered = h?.id === q.id || qopt.hovered === true;
        if (q.type === "icon") {
          const x = drawingX(q.ts as UTCTimestamp as Time);
          const y = s.priceToCoordinate(q.price);
          if (x != null && y != null) dot(ctx, nn(x) * hr, nn(y) * vr, k, q.selected === true, isHovered);
        } else if (q.type === "geometry") {
          if (q.tool === "rr_long" || q.tool === "rr_short") {
            const forecast = buildForecastMeasurementGeometry(q.tool, q.points, candles, q.sourceBars ?? [], (anchor) => {
              const x = drawingX(anchor.ts as UTCTimestamp as Time);
              const y = s.priceToCoordinate(anchor.price);
              return x === null || y === null ? null : { x: nn(x), y: nn(y) };
            }, W, c.timeScale().options().barSpacing ?? 6);
            const risk = forecast?.paths[0]?.points;
            const reward = forecast?.paths[1]?.points;
            const entry = q.points[0];
            const entryY = entry ? s.priceToCoordinate(entry.price) : null;
            if (risk?.[0] && risk[1] && reward?.[3] && risk[3] && entryY !== null) {
              squareHandle(ctx, reward[3].x * hr, reward[3].y * vr, k);
              squareHandle(ctx, risk[0].x * hr, nn(entryY) * vr, k);
              squareHandle(ctx, risk[1].x * hr, nn(entryY) * vr, k);
              squareHandle(ctx, risk[3].x * hr, risk[3].y * vr, k);
            }
          } else {
            for (const anchor of q.points) {
              const x = drawingX(anchor.ts as UTCTimestamp as Time);
              const y = s.priceToCoordinate(anchor.price);
              if (x != null && y != null) dot(ctx, nn(x) * hr, nn(y) * vr, k, q.selected === true, isHovered);
            }
          }
        } else if (q.type === "vline") {
          const v = q as DrawingVerticalLine;
          const x = drawingX(v.ts as UTCTimestamp as Time);
          const y = s.priceToCoordinate(m);
          if (x == null || y == null) continue;
          dot(ctx, nn(x) * hr, nn(y) * vr, k, q.selected === true, isHovered);
        } else if (q.type === "crossline") {
          const x = drawingX(q.ts as UTCTimestamp as Time);
          const y = s.priceToCoordinate(q.price);
          if (x == null || y == null) continue;
          dot(ctx, nn(x) * hr, nn(y) * vr, k, q.selected === true, isHovered);
        } else if (q.type === "parallel-channel") {
          const channel = q as unknown as { ts1: number; price1: number; ts2: number; price2: number; ts3: number; price3: number };
          const points = [[channel.ts1, channel.price1], [channel.ts2, channel.price2], [channel.ts3, channel.price3]];
          for (const [ts, price] of points) {
            const x = drawingX(ts as UTCTimestamp as Time);
            const y = s.priceToCoordinate(price);
            if (x != null && y != null) dot(ctx, nn(x) * hr, nn(y) * vr, k, q.selected === true, isHovered);
          }
        } else if (q.type === "pitchfork") {
          const points = [[q.ts1, q.price1], [q.ts2, q.price2], [q.ts3, q.price3]];
          for (const [ts, price] of points) {
            const x = drawingX(ts as UTCTimestamp as Time);
            const y = s.priceToCoordinate(price);
            if (x != null && y != null) dot(ctx, nn(x) * hr, nn(y) * vr, k, q.selected === true, isHovered);
          }
        } else if (q.type === "fib-gann") {
          const points: { ts: number; price: number }[] = [
            { ts: q.ts1, price: q.price1 },
            { ts: q.ts2, price: q.price2 },
            ...(q.ts3 !== undefined && q.price3 !== undefined ? [{ ts: q.ts3, price: q.price3 }] : []),
          ];
          for (const point of points) {
            const x = drawingX(point.ts as UTCTimestamp as Time);
            const y = s.priceToCoordinate(point.price);
            if (x != null && y != null) dot(ctx, nn(x) * hr, nn(y) * vr, k, q.selected === true, isHovered);
          }
        } else if (q.type === "hline") {
          const qh = q as DrawingHorizontalLine;
          const y = s.priceToCoordinate(qh.price);
          if (y == null) continue;
          const xm = midTs != null ? drawingX(midTs as UTCTimestamp as Time) : null;
          dot(ctx, xm == null ? (W / 2) * hr : nn(xm) * hr, nn(y) * vr, k, q.selected === true, isHovered);
        } else if (q.type === "hray") {
          const qr = q as DrawingHorizontalRay;
          const x = drawingX(qr.ts as UTCTimestamp as Time);
          const y = s.priceToCoordinate(qr.price);
          if (x == null || y == null) continue;
          dot(ctx, nn(x) * hr, nn(y) * vr, k, q.selected === true, isHovered);
        } else if (q.type === "trendline" || q.type === "ray" || q.type === "info-line" || q.type === "extended-line" || q.type === "trend-angle") {
          const qt = q as DrawingTrendLine;
          const x1 = drawingX(qt.ts1 as UTCTimestamp as Time);
          const y1 = s.priceToCoordinate(qt.price1);
          const x2 = drawingX(qt.ts2 as UTCTimestamp as Time);
          const y2 = s.priceToCoordinate(qt.price2);
          if (x1 == null || y1 == null || x2 == null || y2 == null) continue;
          dot(ctx, nn(x1) * hr, nn(y1) * vr, k, q.selected === true, isHovered);
          dot(ctx, nn(x2) * hr, nn(y2) * vr, k, q.selected === true, isHovered);
        }
      }
    };
    const view: IPrimitivePaneView = {
      zOrder(): "top" | "normal" | "bottom" { return "top"; },
      renderer(): IPrimitivePaneRenderer {
        return {
          draw(t: PT): void {
            if (!d.some((q) => q.type !== "rectangle") && !p) return;
            t.useBitmapCoordinateSpace((sc) => {
              const ctx = sc.context;
              const hr = sc.horizontalPixelRatio;
              const vr = sc.verticalPixelRatio;
              const k = Math.min(hr, vr);
              ctx.save();
              try {
                paintLines(ctx, hr, vr, k, sc.mediaSize.width, sc.mediaSize.height);
                paintPreview(ctx, hr, vr, k);
                paintHandles(ctx, hr, vr, k, sc.mediaSize.width);
              } finally { ctx.restore(); }
            });
          },
        };
      },
    };
    return [view];
  }
}
