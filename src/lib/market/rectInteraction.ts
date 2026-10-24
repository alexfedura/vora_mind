// v7.1.0 — rectangle drawing interaction (hit-test, handle drag, body move).
import type { IChartApi } from "lightweight-charts";
import {
  getRectHandleScreenPositions,
  HANDLE_CURSOR,
  type DrawingRect,
  type RectHandleId,
  type RectHover,
} from "./RectangleOverlay";
import { drawingTimeToCoordinate } from "./drawingCoordinates";

/** v7.2.0 D2 — single shared 8-color swatch source (toolbar + palette reuse). */
export const RECT_SWATCH_COLORS = [
  "#ef5350", // red
  "#ff9800", // orange
  "#ffeb3b", // yellow
  "#4caf50", // green
  "#00bcd4", // cyan
  "#2962ff", // blue
  "#e91e63", // magenta
  "#9c27b0", // purple
] as const;

export type RectDrag =
  | { kind: "new"; startT: number; startP: number; id: string; preview: boolean; anchorCorner: AnchorCorner | null }
  | { kind: "handle"; id: string; handle: RectHandleId; orig: DrawingRect }
  | { kind: "body"; id: string; grabT: number; grabP: number; orig: DrawingRect };

export type AnchorCorner = "NW" | "NE" | "SE" | "SW";

/** v7.1.1 — live 8-handle preview state between first and second click. */
export interface DrawingPreview {
  active: boolean;
  anchorTs: number;
  anchorPrice: number;
  currentTs: number;
  currentPrice: number;
  anchorCorner: AnchorCorner | null;
}

interface CandleLike { ts: number; open: number; high: number; low: number; close: number }

export function timestampAtLogical(logical: number, candles: CandleLike[]): number | null {
  if (candles.length === 0 || !Number.isFinite(logical)) return null;
  const intervalSample = logical < 0 ? candles.slice(0, 17) : candles.slice(-17);
  const intervals = intervalSample.slice(1).map((candle, index) =>
    Math.abs(candle.ts - intervalSample[index].ts),
  ).filter((seconds) => Number.isFinite(seconds) && seconds > 0).sort((a, b) => a - b);
  const fallbackInterval = intervals[Math.floor(intervals.length / 2)] ?? 60;
  const lastIndex = candles.length - 1;
  const timestampAtIndex = (index: number): number => {
    if (index < 0) return candles[0].ts + index * fallbackInterval;
    if (index > lastIndex) return candles[lastIndex].ts + (index - lastIndex) * fallbackInterval;
    return candles[index].ts;
  };
  const lowerIndex = Math.floor(logical);
  const fraction = logical - lowerIndex;
  const lowerTime = timestampAtIndex(lowerIndex);
  const upperTime = timestampAtIndex(lowerIndex + 1);
  return lowerTime + (upperTime - lowerTime) * fraction;
}

export function logicalAtCoordinate(chart: IChartApi, x: number): number | null {
  const timeScale = chart.timeScale();
  const rightIndex = timeScale.coordinateToLogical(x);
  if (rightIndex === null) return null;
  const index = rightIndex as number;
  const leftX = timeScale.logicalToCoordinate((index - 1) as import("lightweight-charts").Logical);
  const rightX = timeScale.logicalToCoordinate(rightIndex);
  if (leftX === null || rightX === null || rightX === leftX) return index;
  const fraction = (x - (leftX as unknown as number)) / ((rightX - leftX) as number);
  return index - 1 + fraction;
}

export function hasCandleAtCoordinate(chart: IChartApi, candles: CandleLike[], x: number): boolean {
  if (candles.length === 0) return false;
  const logical = chart.timeScale().coordinateToLogical(x);
  if (logical === null) return false;
  const index = logical as number;
  const nearestIndex = Math.round(index);
  return nearestIndex >= 0
    && nearestIndex < candles.length
    && Math.abs(index - nearestIndex) <= 0.5;
}

export type SeriesLike = {
  coordinateToPrice: (y: number) => number | null;
  priceToCoordinate: (p: number) => number | null;
};

/**
 * Snap a data-space price to the nearest OHLC of the nearest candle.
 * Directional rules:
 *   - cursor above the candle's high  → snap to candle.high
 *   - cursor below the candle's low   → snap to candle.low
 *   - cursor inside [low, high]       → snap to nearest of {open, high, low, close}
 *
 * @param cursorTs     data-space timestamp of the cursor
 * @param cursorPrice  data-space price of the cursor
 * @param candles      array of candles
 * @returns { ts, price } snapped in data space
 */
export function snapPriceToOhlc(
  cursorTs: number,
  cursorPrice: number,
  candles: CandleLike[],
): { ts: number; price: number } {
  if (candles.length === 0) return { ts: cursorTs, price: cursorPrice };

  // 1. Find nearest candle by time
  let nearest = candles[0];
  let bestDt = Math.abs(candles[0].ts - cursorTs);
  for (const c of candles) {
    const dt = Math.abs(c.ts - cursorTs);
    if (dt < bestDt) { bestDt = dt; nearest = c; }
  }

  // 2. Directional snap
  let snappedPrice: number;
  if (cursorPrice > nearest.high) {
    snappedPrice = nearest.high;
  } else if (cursorPrice < nearest.low) {
    snappedPrice = nearest.low;
  } else {
    // Inside [low, high] — nearest of 4
    const candidates = [nearest.open, nearest.high, nearest.low, nearest.close];
    snappedPrice = candidates[0];
    let bestD = Math.abs(candidates[0] - cursorPrice);
    for (const p of candidates) {
      const d = Math.abs(p - cursorPrice);
      if (d < bestD) { bestD = d; snappedPrice = p; }
    }
  }
  return { ts: nearest.ts, price: snappedPrice };
}

/**
 * v7.1.1 — corner determination from anchor → cursor direction.
 * dx = currentTs − anchorTs, dy = currentPrice − anchorPrice.
 */
export function cornerFromDirection(dx: number, dy: number): AnchorCorner {
  if (dx >= 0 && dy >= 0) return "NW"; // mouse went right-down
  if (dx < 0 && dy >= 0) return "NE"; // mouse went left-down
  if (dx < 0 && dy < 0) return "SE"; // mouse went left-up
  return "SW"; // mouse went right-up
}

export function snapStrong(
  chart: IChartApi,
  srs: SeriesLike,
  candles: CandleLike[],
  px: number,
  py: number,
): { t: number; p: number } | null {
  if (!hasCandleAtCoordinate(chart, candles, px)) return null;
  const t = chart.timeScale().coordinateToTime(px);
  if (t == null) return null;
  const time = t as number;
  let best = candles[0];
  let bestDt = Math.abs(candles[0].ts - time);
  for (let i = 1; i < candles.length; i++) {
    const dt = Math.abs(candles[i].ts - time);
    if (dt < bestDt) { bestDt = dt; best = candles[i]; }
  }
  const ohlc = [best.open, best.high, best.low, best.close];
  let pick = ohlc[0];
  let pickDy = Math.abs((srs.priceToCoordinate(ohlc[0]) ?? Infinity) - py);
  for (let i = 1; i < ohlc.length; i++) {
    const y = srs.priceToCoordinate(ohlc[i]);
    if (y == null) continue;
    const dy = Math.abs(y - py);
    if (dy < pickDy) { pickDy = dy; pick = ohlc[i]; }
  }
  if (pickDy > 30) return null;
  return { t: best.ts, p: pick };
}

// Apply a handle drag while normalizing bounds after opposing edges cross.
export function applyHandleDrag(
  d: DrawingRect,
  handle: RectHandleId,
  newTs: number,
  newPrice: number,
): DrawingRect {
  let ts_start = d.ts_start;
  let ts_end = d.ts_end;
  let price_low = d.price_low;
  let price_high = d.price_high;
  switch (handle) {
    case "NW": ts_start = newTs; price_high = newPrice; break;
    case "N": price_high = newPrice; break;
    case "NE": ts_end = newTs; price_high = newPrice; break;
    case "E": ts_end = newTs; break;
    case "SE": ts_end = newTs; price_low = newPrice; break;
    case "S": price_low = newPrice; break;
    case "SW": ts_start = newTs; price_low = newPrice; break;
    case "W": ts_start = newTs; break;
  }
  if (ts_start > ts_end) { const tmp = ts_start; ts_start = ts_end; ts_end = tmp; }
  if (price_low > price_high) { const tmp = price_low; price_low = price_high; price_high = tmp; }
  return { ...d, ts_start, ts_end, price_low, price_high };
}

export function findHandleAt(
  drawings: DrawingRect[],
  chart: IChartApi,
  series: object,
  px: number,
  py: number,
): { d: DrawingRect; handle: RectHandleId } | null {
  const srs = series as unknown as import("lightweight-charts").ISeriesApi<import("lightweight-charts").SeriesType>;
  for (let index = drawings.length - 1; index >= 0; index--) {
    const d = drawings[index];
    if (d.locked === true) continue;
    const hs = getRectHandleScreenPositions(d, chart, srs);
    for (const h of hs) {
      if (Math.hypot(h.x - px, h.y - py) <= 10) return { d, handle: h.id };
    }
  }
  return null;
}

export function findRectAt(
  drawings: DrawingRect[],
  chart: IChartApi,
  series: object,
  px: number,
  py: number,
  tolerance = 3,
): DrawingRect | null {
  const srs = series as unknown as import("lightweight-charts").ISeriesApi<import("lightweight-charts").SeriesType>;
  const timeScale = chart.timeScale();
  for (let index = drawings.length - 1; index >= 0; index--) {
    const drawing = drawings[index];
    const startX = drawingTimeToCoordinate(timeScale, drawing.ts_start);
    const endX = drawingTimeToCoordinate(timeScale, drawing.ts_end);
    if (startX === null || endX === null) continue;

    const left = Math.min(startX, endX);
    const right = Math.max(startX, endX);
    const highY = srs.priceToCoordinate(Math.max(drawing.price_low, drawing.price_high));
    const lowY = srs.priceToCoordinate(Math.min(drawing.price_low, drawing.price_high));
    if (highY === null || lowY === null) continue;
    const top = Math.min(highY, lowY);
    const bottom = Math.max(highY, lowY);
    if (
      px >= left - tolerance && px <= right + tolerance &&
      py >= top - tolerance && py <= bottom + tolerance
    ) return drawing;
  }
  return null;
}

export function findRectBorderAt(
  drawings: DrawingRect[],
  chart: IChartApi,
  series: object,
  px: number,
  py: number,
  tolerance = 18,
): DrawingRect | null {
  const srs = series as unknown as import("lightweight-charts").ISeriesApi<import("lightweight-charts").SeriesType>;
  const timeScale = chart.timeScale();
  for (let index = drawings.length - 1; index >= 0; index--) {
    const d = drawings[index];
    const startX = drawingTimeToCoordinate(timeScale, d.ts_start);
    const endX = drawingTimeToCoordinate(timeScale, d.ts_end);
    if (startX == null || endX == null) continue;
    const x1 = startX;
    const x2 = endX;
    if (Math.max(x1, x2) < 0 || Math.min(x1, x2) > timeScale.width()) continue;

    const highY = srs.priceToCoordinate(Math.max(d.price_low, d.price_high));
    const lowY = srs.priceToCoordinate(Math.min(d.price_low, d.price_high));
    if (highY == null || lowY == null) continue;
    const left = Math.min(x1, x2);
    const right = Math.max(x1, x2);
    const top = Math.min(highY, lowY);
    const bottom = Math.max(highY, lowY);
    const withinX = px >= left - tolerance && px <= right + tolerance;
    const withinY = py >= top - tolerance && py <= bottom + tolerance;
    if (
      (withinY && (Math.abs(px - left) <= tolerance || Math.abs(px - right) <= tolerance)) ||
      (withinX && (Math.abs(py - top) <= tolerance || Math.abs(py - bottom) <= tolerance))
    ) return d;
  }
  return null;
}

export function cursorForHover(
  drawings: DrawingRect[],
  chart: IChartApi,
  series: object,
  px: number,
  py: number,
): string {
  const hitH = findHandleAt(drawings, chart, series, px, py);
  if (hitH) return HANDLE_CURSOR[hitH.handle];
  if (findRectBorderAt(drawings, chart, series, px, py)) return "grab";
  return "";
}

/** Hover only the topmost rectangle's border/handles, never its interior. */
export function hoverForPoint(
  drawings: DrawingRect[],
  chart: IChartApi,
  series: object,
  px: number,
  py: number,
): RectHover | null {
  const hitH = findHandleAt(drawings, chart, series, px, py);
  if (hitH) return { id: hitH.d.id, handle: hitH.handle };
  const hitBorder = findRectBorderAt(drawings, chart, series, px, py);
  if (hitBorder) return { id: hitBorder.id, handle: null };
  return null;
}

export interface ScaleLike {
  color: string;
  start_pct: number;
  end_pct: number;
}

/**
 * v7.2.0 Part B — auto-color a finished rectangle by S-scale.
 * v7.3.0 Part E — border-only: match → scale border color (no fill);
 * else amber, no fill. Fill stays transparent (Policy Panel is the only
 * fill path going forward).
 */
export function autoColorForRect(
  price_low: number,
  price_high: number,
  scales: ScaleLike[] | null | undefined,
): { borderColor: string; fillColor: string | null; fillOpacity: number } {
  const pct = ((price_high - price_low) / price_high) * 100;
  if (scales) {
    for (let i = 0; i < scales.length; i++) {
      const s = scales[i];
      if (s.start_pct <= pct && pct < s.end_pct) {
        return { borderColor: s.color, fillColor: null, fillOpacity: 0.0 };
      }
    }
  }
  return { borderColor: "#ffb74d", fillColor: null, fillOpacity: 0.0 };
}
