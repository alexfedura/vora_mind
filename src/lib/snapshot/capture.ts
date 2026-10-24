import type { IChartApi } from "lightweight-charts";
import type { SnapshotViewport } from "./types";

function num(v: unknown, fallback: number): number {
  return typeof v === "number" && Number.isFinite(v) ? v : fallback;
}

export function captureViewport(chart: IChartApi): SnapshotViewport {
  const timeScale = chart.timeScale();
  let fromTs = 0;
  let toTs = 0;
  let fromLogical = 0;
  let toLogical = 0;
  let barSpacing = 6;
  let rightOffset = 0;
  try {
    const visRange = timeScale.getVisibleRange() as unknown as {
      from?: unknown;
      to?: unknown;
    } | null;
    if (visRange && typeof visRange.from === "number" && typeof visRange.to === "number") {
      fromTs = visRange.from * 1000;
      toTs = visRange.to * 1000;
    }
  } catch { /* ignore */ }
  try {
    const visLogical = timeScale.getVisibleLogicalRange() as unknown as {
      from?: unknown;
      to?: unknown;
    } | null;
    if (visLogical) {
      fromLogical = num(visLogical.from, 0);
      toLogical = num(visLogical.to, 0);
    }
  } catch { /* ignore */ }
  try {
    const opts = timeScale.options() as unknown as {
      barSpacing?: unknown;
      rightOffset?: unknown;
    };
    barSpacing = num(opts.barSpacing, 10);
    rightOffset = num(opts.rightOffset, 0);
  } catch { /* ignore */ }
  // v0.83.4 — vertical price range + autoScale state (price scale id "right").
  let priceFrom: number | undefined;
  let priceTo: number | undefined;
  let autoScale: boolean | undefined;
  try {
    const priceScale = chart.priceScale("right");
    try {
      autoScale = priceScale.options().autoScale;
    } catch { /* ignore */ }
    try {
      const priceRange = priceScale.getVisibleRange();
      if (
        priceRange &&
        typeof priceRange.from === "number" &&
        typeof priceRange.to === "number" &&
        Number.isFinite(priceRange.from) &&
        Number.isFinite(priceRange.to)
      ) {
        priceFrom = priceRange.from;
        priceTo = priceRange.to;
      }
    } catch { /* ignore */ }
  } catch { /* ignore */ }
  // v0.83.6 — center anchor point (chart center at capture time).
  // Container size (fall back to window size if container missing).
  // v0.83.11 — robust fallbacks: mid of visible range when
  // coordinateToTime / coordinateToPrice return null.
  let anchorTs: number | undefined;
  let anchorPrice: number | undefined;
  try {
    const chartEl = (
      chart as unknown as { chartElement?: () => HTMLElement | null }
    ).chartElement?.() ?? document.querySelector(".chart-container") ?? null;
    const el = chartEl as HTMLElement | null;
    const width = el?.clientWidth ?? (typeof window !== "undefined" ? window.innerWidth : 0);
    const height = el?.clientHeight ?? (typeof window !== "undefined" ? window.innerHeight : 0);
    const cx = Math.floor(width / 2);
    const cy = Math.floor(height / 2);
    // --- anchorTs ---
    try {
      const t = timeScale.coordinateToTime(cx) as unknown;
      if (t != null && Number.isFinite(Number(t))) anchorTs = Number(t);
    } catch { /* ignore */ }
    // Fallback: if coordinateToTime failed, use mid of visible range
    if (anchorTs == null) {
      try {
        const vr = timeScale.getVisibleRange() as unknown as {
          from?: unknown;
          to?: unknown;
        } | null;
        if (vr && typeof vr.from === "number" && typeof vr.to === "number") {
          anchorTs = Math.floor((Number(vr.from) + Number(vr.to)) / 2);
        }
      } catch { /* ignore */ }
    }
    try {
      const priceScale = chart.priceScale("right") as unknown as {
        coordinateToPrice?: (y: number) => unknown;
        getVisibleRange?: () => { from: unknown; to: unknown } | null;
      };
      const p = priceScale.coordinateToPrice?.(cy);
      if (p != null && Number.isFinite(Number(p))) {
        anchorPrice = Number(p);
      } else {
        // Fallback: if coordinateToPrice failed, use mid of price range
        try {
          const pr = priceScale.getVisibleRange?.();
          if (pr) {
            anchorPrice = (Number(pr.from) + Number(pr.to)) / 2;
          }
        } catch { /* ignore */ }
      }
    } catch { /* ignore */ }
  } catch { /* ignore */ }
  return {
    fromTs,
    toTs,
    fromLogical,
    toLogical,
    barSpacing,
    rightOffset,
    windowW: typeof window !== "undefined" ? window.innerWidth : 0,
    windowH: typeof window !== "undefined" ? window.innerHeight : 0,
    dpr: typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1,
    priceFrom,
    priceTo,
    autoScale,
    anchorTs,
    anchorPrice,
  };
}

// The rest of capture* helpers depend on the existing app state.
// Wire them from the parent component that owns the state:
// - captureChart()    <- reads from the current chart page state
// - captureDrawings() <- reads from the drawing hook
// - capturePolicy()   <- reads from the policy panel state
// - captureTheme()    <- reads from chart options + theme config
//
// These are stubs; the parent component passes the data directly
// to buildSnapshot() to avoid coupling the snapshot lib to app
// state.
