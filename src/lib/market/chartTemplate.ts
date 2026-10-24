//! Chart Template — enforces a fixed visual structure.
//!
//! Per docs/scale-standard.md section 4:
//!   - 180 candles on screen at the default zoom
//!   - TradingView-style candle width and spacing
//!   - Fixed vertical range (±2% default, ATR-based in MKT-7)
//!
//! This decouples the visual structure (template) from the data
//! semantics (M-scale). Changing M changes the DATA, not the visual.

import type { IChartApi, ISeriesApi } from "lightweight-charts";

/**
 * Canonical Zoom State — ONLY integers.
 * These are the ONLY persisted values.
 * Everything else is derived deterministically.
 */
export interface ZoomState {
  x_off_milli: number;   // i32, display X = x_off_milli / 1000
  y_off_milli: number;   // i32, display Y = y_off_milli / 1000
}

export const DEFAULT_VISIBLE_CANDLE_COUNT = 180;

export const DEFAULT_ZOOM: ZoomState = {
  x_off_milli: 0,
  y_off_milli: 0,
};

export function offsetToCandleCount(xOffMilli: number): number {
  const exp = -xOffMilli / 1000;
  const raw = DEFAULT_VISIBLE_CANDLE_COUNT * Math.pow(2, exp);
  return Math.max(1, Math.min(100000, raw));
}

export function offsetToYMilli(yOffMilli: number): number {
  const exp = -yOffMilli / 1000;
  const raw = 4000 * Math.pow(2, exp);
  return Math.max(1, Math.min(1000000, raw));
}

export interface TemplateOptions {
  candleCount?: number;       // default DEFAULT_VISIBLE_CANDLE_COUNT
  priceRangePct?: number;     // legacy ±% fallback if ATR not used
  kMultiplier?: number;       // default 4.0
  atrPeriod?: number;         // default 14
}

export const DEFAULT_TEMPLATE: TemplateOptions = {
  candleCount: DEFAULT_VISIBLE_CANDLE_COUNT,
  priceRangePct: 2,
  kMultiplier: 4.0,
  atrPeriod: 14,
};

export interface CandleLite {
  ts: number;      // unix seconds
  high: number;
  low: number;
  close: number;
}

/**
 * Calculates Wilder's Average True Range (ATR) with period n (default 14).
 * Returns null if candles length < 2.
 */
export function calculateATR(candles: CandleLite[], period: number = 14): number | null {
  if (candles.length < 2) return null;

  const trs: number[] = [];
  for (let i = 1; i < candles.length; i++) {
    const cur = candles[i];
    const prev = candles[i - 1];
    const tr = Math.max(
      cur.high - cur.low,
      Math.abs(cur.high - prev.close),
      Math.abs(cur.low - prev.close)
    );
    trs.push(tr);
  }

  if (trs.length === 0) return null;

  const actualPeriod = Math.min(period, trs.length);
  // Initial SMA of first actualPeriod TRs
  let atr = 0;
  for (let i = 0; i < actualPeriod; i++) {
    atr += trs[i];
  }
  atr /= actualPeriod;

  // Wilder's RMA for the rest
  for (let i = actualPeriod; i < trs.length; i++) {
    atr = (atr * (actualPeriod - 1) + trs[i]) / actualPeriod;
  }

  return atr;
}

/**
 * Apply template using Integer Canonical State.
 * Accepts ZoomState (derives values internally).
 * ATR must be provided; if invalid, falls back to ±1% of price.
 */
export function applyTemplateAtr(
  chart: IChartApi,
  candles: CandleLite[],
  atrValue: number,
  state: ZoomState = DEFAULT_ZOOM,
): void {
  if (candles.length === 0) return;

  const total = candles.length;

  // ─── Horizontal: show last candle + right padding ───
  const cc_raw = offsetToCandleCount(state.x_off_milli);
  const cc = Math.min(cc_raw, Math.max(total * 2, 10));

  // Right padding: 10% of visible width, min 3 candles
  const rightOffset = Math.max(3, Math.ceil(cc * 0.10));

  // Position: last candle near right edge with padding
  const to   = total - 0.5 + rightOffset;
  const from = to - cc;

  chart.timeScale().setVisibleLogicalRange({ from, to });

  // ─── Vertical: anchor at last close ───
  const lastClose = candles[total - 1].close;
  const yMult = offsetToYMilli(state.y_off_milli) / 1000;
  let halfRange: number;
  if (isFinite(atrValue) && atrValue > 0) {
    halfRange = yMult * atrValue;
  } else if (total < 14) {
    halfRange = lastClose * 0.05 * (yMult / 4);
  } else {
    halfRange = lastClose * 0.01 * (yMult / 4);
  }

  const priceScale = chart.priceScale("right");
  priceScale.applyOptions({ autoScale: false });
  priceScale.setVisibleRange({
    from: lastClose - halfRange,
    to: lastClose + halfRange,
  });
}

/**
 * Apply zoom while preserving the current visible center.
 * Used for Ctrl/Shift+wheel and any user-driven zoom that should
 * not snap the chart back to lastClose.
 */
export function applyZoomPreservingCenter(
  chart: IChartApi,
  candles: CandleLite[],
  atrValue: number,
  state: ZoomState,
): void {
  if (candles.length === 0) return;

  const timeScale = chart.timeScale();
  const priceScale = chart.priceScale("right");
  const total = candles.length;

  // ─── DATA-AWARE CLAMP (fixes huge-chart bug) ───
  const cc_raw = offsetToCandleCount(state.x_off_milli);
  const cc = Math.min(cc_raw, Math.max(total * 2, 10));

  // Horizontal: keep center, change width
  const currentRange = timeScale.getVisibleLogicalRange();
  if (currentRange) {
    const center = (currentRange.from + currentRange.to) / 2;
    const halfWidth = cc / 2;
    timeScale.setVisibleLogicalRange({
      from: center - halfWidth,
      to: center + halfWidth,
    });
  } else {
    const from = Math.max(0, total - cc);
    timeScale.setVisibleLogicalRange({ from: from - 0.5, to: total - 0.5 });
  }

  // Vertical: keep center, change half-range
  const currentPriceRange = priceScale.getVisibleRange();
  const centerPrice = currentPriceRange
    ? (currentPriceRange.from + currentPriceRange.to) / 2
    : candles[candles.length - 1].close;

  const yMult = offsetToYMilli(state.y_off_milli) / 1000;
  let halfRange: number;
  if (isFinite(atrValue) && atrValue > 0) {
    halfRange = yMult * atrValue;
  } else if (total < 14) {
    // sparse data: use ±5% fallback
    halfRange = centerPrice * 0.05 * (yMult / 4);
  } else {
    halfRange = centerPrice * 0.01 * (yMult / 4);
  }

  priceScale.applyOptions({ autoScale: false });
  priceScale.setVisibleRange({
    from: centerPrice - halfRange,
    to: centerPrice + halfRange,
  });
}

/**
 * @deprecated Use applyTemplateAtr(chart, candles, atr, ZoomState)
 * Apply the fixed template to a chart.
 */
export function applyTemplate(
  chart: IChartApi,
  _series: ISeriesApi<"Custom">,
  candles: CandleLite[],
  opts: TemplateOptions = DEFAULT_TEMPLATE,
): void {
  if (candles.length === 0) return;

  const candleCount = opts.candleCount ?? DEFAULT_VISIBLE_CANDLE_COUNT;
  const k = opts.kMultiplier ?? 4.0;
  const period = opts.atrPeriod ?? 14;

  // --- X-axis: last N candles ---
  const total = candles.length;
  const from = Math.max(0, total - candleCount);
  chart.timeScale().setVisibleLogicalRange({
    from: from - 0.5,
    to: total - 0.5,
  });

  // --- Y-axis: ATR-based locked vertical range ---
  const lastClose = candles[total - 1].close;
  const atr = calculateATR(candles, period);

  let halfRange: number;
  if (atr !== null && atr > 0) {
    halfRange = k * atr;
  } else {
    const pct = opts.priceRangePct ?? 2;
    halfRange = (lastClose * pct) / 200;
  }

  chart.priceScale("right").applyOptions({
    autoScale: false,
  });

  chart.priceScale("right").setVisibleRange({
    from: lastClose - halfRange,
    to: lastClose + halfRange,
  });
}

/**
 * Compute the visible logical range for a given M-level,
 * used to reconcile zoom if needed.
 */
export function computeVisibleRange(
  totalCandles: number,
  candleCount: number = DEFAULT_VISIBLE_CANDLE_COUNT,
): { from: number; to: number } {
  const from = Math.max(0, totalCandles - candleCount);
  return { from: from - 0.5, to: totalCandles - 0.5 };
}


