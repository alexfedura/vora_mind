import type { IChartApi, UTCTimestamp } from "lightweight-charts";

export interface CapturedViewport {
  barSpacing: number;
  rightOffset: number;
  fromTs: number;
  toTs: number;
  fromLogical?: number;
  toLogical?: number;
  priceFrom?: number;
  priceTo?: number;
  autoScale: boolean;
  // v0.83.6 — center anchor point (chart center at capture time)
  anchorTs?: number;
  anchorPrice?: number;
  anchorXRatio?: number;
}

function num(v: unknown, fallback: number): number {
  return typeof v === "number" && Number.isFinite(v) ? v : fallback;
}

export function captureViewport(chart: IChartApi, anchorXRatio?: number): CapturedViewport | null {
  try {
    const timeScale = chart.timeScale();
    const visRange = timeScale.getVisibleRange() as unknown as {
      from?: unknown;
      to?: unknown;
    } | null;
    if (!visRange || typeof visRange.from !== "number" || typeof visRange.to !== "number") {
      return null;
    }

    const opts = timeScale.options() as unknown as {
      barSpacing?: unknown;
      rightOffset?: unknown;
    };
    let autoScale = true;
    let priceFrom: number | undefined;
    let priceTo: number | undefined;
    try {
      const priceScale = chart.priceScale("right");
      try {
        autoScale = priceScale.options().autoScale;
      } catch {
        /* ignore */
      }
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
      } catch {
        /* ignore */
      }
    } catch {
      /* ignore */
    }

    return {
      barSpacing: num(opts.barSpacing, 10),
      rightOffset: num(opts.rightOffset, 0),
      fromTs: visRange.from,
      toTs: visRange.to,
      priceFrom,
      priceTo,
      autoScale,
      ...captureAnchorPoint(chart, anchorXRatio),
    };
  } catch {
    return null;
  }
}

export function applyViewport(chart: IChartApi, vp: CapturedViewport): void {
  const timeScale = chart.timeScale();
  let priceScale: ReturnType<IChartApi["priceScale"]> | null = null;
  try {
    priceScale = chart.priceScale("right");
  } catch {
    priceScale = null;
  }

  // 1. X zoom (bar spacing first, before any range set)
  if (typeof vp.barSpacing === "number" && vp.barSpacing > 0) {
    try {
      timeScale.applyOptions({ barSpacing: vp.barSpacing });
    } catch {
      /* ignore */
    }
  }
  // v0.83.7 Part F — right-offset edge case: set barSpacing, temp
  // rightOffset: 0, then setVisibleRange, then restore captured
  // rightOffset. Prevents right-offset bias when candle counts
  // differ between old and new timeframe.
  if (typeof vp.rightOffset === "number") {
    try {
      timeScale.applyOptions({ rightOffset: 0 });
    } catch {
      /* ignore */
    }
  }

  // 2. Time range — v0.83.8 Part B (Suspect #2): logical-priority.
  // Apply the logical range FIRST; if it succeeded, do NOT apply
  // the time range at all. The old-TF timestamps would map to
  // wrong positions on the new data and undo the logical write.
  let appliedLogical = false;
  // v0.83.7: prefer logical range when present — it survives different
  // candle counts and is immune to calendar gaps.
  if (
    typeof vp.fromLogical === "number" &&
    typeof vp.toLogical === "number" &&
    (vp.fromLogical !== 0 || vp.toLogical !== 0)
  ) {
    try {
      timeScale.setVisibleLogicalRange({
        from: vp.fromLogical,
        to: vp.toLogical,
      });
      appliedLogical = true;
    } catch {
      appliedLogical = false;
    }
  }
  // Time range ONLY if logical failed (or was absent).
  if (!appliedLogical && vp.fromTs > 0 && vp.toTs > vp.fromTs) {
    try {
      timeScale.setVisibleRange({
        from: vp.fromTs as UTCTimestamp,
        to: vp.toTs as UTCTimestamp,
      });
    } catch {
      // Range may fall outside current data — ignore.
    }
  }

  // v0.83.7 Part F — restore the captured rightOffset AFTER
  // setVisibleRange so it does not bias the range computation.
  if (typeof vp.rightOffset === "number") {
    try {
      timeScale.applyOptions({ rightOffset: vp.rightOffset });
    } catch {
      /* ignore */
    }
  }

  // 3. Y zoom
  if (typeof vp.autoScale === "boolean" && priceScale) {
    try {
      priceScale.applyOptions({ autoScale: vp.autoScale });
    } catch {
      /* ignore */
    }
  }
  if (
    vp.autoScale === false &&
    typeof vp.priceFrom === "number" &&
    typeof vp.priceTo === "number" &&
    priceScale
  ) {
    try {
      priceScale.setVisibleRange({
        from: vp.priceFrom,
        to: vp.priceTo,
      });
    } catch {
      try {
        priceScale.applyOptions({ autoScale: true });
      } catch {
        /* ignore */
      }
    }
  }

  // v0.83.6 — center the anchor point after X/Y zoom is applied.
  if (
    typeof vp.anchorTs === "number" &&
    typeof vp.anchorPrice === "number"
  ) {
    applyAnchorPoint(chart, vp.anchorTs, vp.anchorPrice);
  }
}

// v0.83.7 Part C — verified viewport application: apply, settle
// two frames, verify the anchor sits at screen center within
// 2px, and re-apply up to 3 times if drift persists.
// v0.83.8 Part C (Suspect #3): anchor-first positioning — anchor
// is portable across TFs, raw logical indices are not.
export async function applyViewportVerified(
  chart: IChartApi,
  vp: CapturedViewport,
): Promise<void> {
  const MAX_ATTEMPTS = 3;

  for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
    // v0.83.15 Part A: captured spans for the anchor fallback.
    const targetSpan =
      typeof vp.toTs === "number" &&
      typeof vp.fromTs === "number" &&
      vp.toTs > vp.fromTs
        ? vp.toTs - vp.fromTs
        : undefined;
    const targetPriceSpan =
      typeof vp.priceTo === "number" &&
      typeof vp.priceFrom === "number" &&
      vp.priceTo > vp.priceFrom
        ? vp.priceTo - vp.priceFrom
        : undefined;
    // 1. X zoom (barSpacing) — must be restored even when an anchor is
    // present. The anchor logic is for centering, not for re-deriving the
    // saved horizontal zoom. Skipping this step makes the restored range
    // drift back to the default/auto width and loses the exact saved X zoom.
    if (typeof vp.barSpacing === "number" && vp.barSpacing > 0) {
      try {
        chart.timeScale().applyOptions({ barSpacing: vp.barSpacing });
      } catch {
        /* ignore */
      }
    }
    // 2. Y zoom (price range) — always.
    try {
      if (typeof vp.autoScale === "boolean") {
        chart.priceScale("right").applyOptions({ autoScale: vp.autoScale });
      }
      if (
        vp.autoScale === false &&
        typeof vp.priceFrom === "number" &&
        typeof vp.priceTo === "number"
      ) {
        try {
          chart.priceScale("right").setVisibleRange({
            from: vp.priceFrom,
            to: vp.priceTo,
          });
        } catch {
          chart.priceScale("right").applyOptions({ autoScale: true });
        }
      }
    } catch {
      /* ignore */
    }

    // 3. Position — ANCHOR FIRST (portable across TFs).
    // v0.83.11 Part D — partial anchors accepted.
    const hasAnyAnchor =
      (typeof vp.anchorTs === "number" && vp.anchorTs > 0) ||
      (typeof vp.anchorPrice === "number" &&
        Number.isFinite(vp.anchorPrice) &&
        vp.anchorPrice !== 0);
    if (hasAnyAnchor) {
      const applied = applyAnchorPointReturningSuccess(
        chart,
        typeof vp.anchorTs === "number" ? vp.anchorTs : undefined,
        typeof vp.anchorPrice === "number" ? vp.anchorPrice : undefined,
        targetSpan,
        targetPriceSpan,
        vp.anchorXRatio,
      );
      if (!applied) {
        applyLogicalOrTimeRange(chart, vp);
      }
    } else {
      applyLogicalOrTimeRange(chart, vp);
    }

    // 4. Anchor again to double-check. Keep the saved X scale stable while
    // re-centering on the saved anchor point.
    if (hasAnyAnchor) {
      applyAnchorPointReturningSuccess(
        chart,
        typeof vp.anchorTs === "number" ? vp.anchorTs : undefined,
        typeof vp.anchorPrice === "number" ? vp.anchorPrice : undefined,
        targetSpan,
        targetPriceSpan,
        vp.anchorXRatio,
      );
    }

    // 5. Re-apply the saved bar spacing after the anchor-based centering pass;
    // this preserves the exact horizontal zoom instead of letting the chart
    // drift back to an auto-sized range.
    if (typeof vp.barSpacing === "number" && vp.barSpacing > 0) {
      try {
        chart.timeScale().applyOptions({ barSpacing: vp.barSpacing });
      } catch {
        /* ignore */
      }
    }

    // 6. Settle 2 frames + verify.
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() =>
        requestAnimationFrame(() => resolve()),
      ),
    );

    if (verifyAnchor(chart, vp)) return;
  }

  if ((import.meta as unknown as { env?: { DEV?: boolean } }).env?.DEV) {
    console.warn(
      "[viewport] anchor drift persists after",
      MAX_ATTEMPTS,
      "attempts — vp:",
      vp,
    );
  }
}

// v0.83.8 Part C — logical-or-time fallback (logical first,
// time only if logical failed). rightOffset zeroed during the
// write, restored after (v0.83.7F symmetry).
function applyLogicalOrTimeRange(chart: IChartApi, vp: CapturedViewport): void {
  const timeScale = chart.timeScale();
  try {
    timeScale.applyOptions({ rightOffset: 0 });
  } catch {
    /* ignore */
  }
  let done = false;
  if (
    typeof vp.fromLogical === "number" &&
    typeof vp.toLogical === "number" &&
    (vp.fromLogical !== 0 || vp.toLogical !== 0)
  ) {
    try {
      timeScale.setVisibleLogicalRange({
        from: vp.fromLogical,
        to: vp.toLogical,
      });
      done = true;
    } catch {
      /* ignore */
    }
  }
  if (!done && vp.fromTs > 0 && vp.toTs > vp.fromTs) {
    try {
      timeScale.setVisibleRange({
        from: vp.fromTs as UTCTimestamp,
        to: vp.toTs as UTCTimestamp,
      });
    } catch {
      /* ignore */
    }
  }
  try {
    timeScale.applyOptions({ rightOffset: vp.rightOffset });
  } catch {
    /* ignore */
  }
}

// v0.83.8 Part C — anchor positioning reporting success.
// v0.83.11 Part B — null-tolerant: falls back to setVisibleRange
// when timeToCoordinate / priceToCoordinate return null.
// v0.83.15 Part A/B — X/Y fallback uses CAPTURED span (not current).
function applyAnchorPointReturningSuccess(
  chart: IChartApi,
  anchorTs: number | undefined,
  anchorPrice: number | undefined,
  targetSpanSec?: number,
  targetPriceSpan?: number,
  anchorXRatio?: number,
): boolean {
  try {
    const timeScale = chart.timeScale();
    const priceScale = chart.priceScale("right") as unknown as {
      priceToCoordinate?: (p: number) => number | null;
      getVisibleRange?: () => { from: number; to: number } | null;
      setVisibleRange?: (range: { from: number; to: number }) => void;
    };
    const chartEl =
      (chart as unknown as { chartElement?: () => HTMLElement | null }).chartElement?.() ?? null;
    const el = chartEl as HTMLElement | null;
    const width = el?.clientWidth ?? (typeof window !== "undefined" ? window.innerWidth : 0);
    const height = el?.clientHeight ?? (typeof window !== "undefined" ? window.innerHeight : 0);
    const cx = Math.floor(width / 2);
    const cy = Math.floor(height / 2);
    const hasAnchorXRatio = typeof anchorXRatio === "number" && Number.isFinite(anchorXRatio) && anchorXRatio >= 0 && anchorXRatio <= 1;
    const targetX = hasAnchorXRatio ? Math.floor(timeScale.width() * anchorXRatio) : cx;
    const horizontalWidth = Math.max(1, timeScale.width());
    let anyApplied = false;
    // ---- Time (X) ----
    if (typeof anchorTs === "number" && anchorTs > 0) {
      const px = timeScale.timeToCoordinate(anchorTs as UTCTimestamp);
      if (px != null) {
        const delta = targetX - (px as number);
        if (Math.abs(delta) > 0.5) {
          const logical = timeScale.getVisibleLogicalRange?.();
          if (logical) {
            const barsPerPixel = (logical.to - logical.from) / horizontalWidth;
            const deltaBars = delta * barsPerPixel;
            timeScale.setVisibleLogicalRange({
              from: logical.from - deltaBars,
              to: logical.to - deltaBars,
            });
          }
        }
        anyApplied = true;
      } else {
        // v0.83.15 Part B: use the CAPTURED span (from vp.fromTs/toTs) if
        // available — NOT the current range, which may already be
        // shrunk by the barSpacing apply. Fall back to current span
        // only when no captured span is provided.
        let span: number | undefined;

        if (
          typeof targetSpanSec === "number" &&
          Number.isFinite(targetSpanSec) &&
          targetSpanSec > 0
        ) {
          span = targetSpanSec;
        } else {
          const cur = timeScale.getVisibleRange?.() as unknown as {
            from: unknown;
            to: unknown;
          } | null;
          if (cur) {
            const from = Number(cur.from);
            const to = Number(cur.to);
            const visibleSpan = to - from;
            if (Number.isFinite(visibleSpan) && visibleSpan > 0) {
              span = visibleSpan;
            }
          }
        }

        if (span != null && span > 0) {
          const ratio = hasAnchorXRatio ? anchorXRatio : 0.5;
          try {
            timeScale.setVisibleRange({
              from: (anchorTs - span * ratio) as UTCTimestamp,
              to: (anchorTs + span * (1 - ratio)) as UTCTimestamp,
            });
            anyApplied = true;
          } catch { /* ignore */ }
        }
      }
    }
    // ---- Price (Y) ----
    if (
      typeof anchorPrice === "number" &&
      Number.isFinite(anchorPrice) &&
      anchorPrice !== 0
    ) {
      const py = priceScale.priceToCoordinate?.(anchorPrice);
      if (py != null) {
        const delta = cy - py;
        if (Math.abs(delta) > 0.5) {
          const range = priceScale.getVisibleRange?.();
          if (range) {
            const pricePerPixel = (Number(range.to) - Number(range.from)) / Math.max(1, height);
            const deltaPrice = delta * pricePerPixel;
            priceScale.setVisibleRange?.({
              from: Number(range.from) + deltaPrice,
              to: Number(range.to) + deltaPrice,
            });
            anyApplied = true;
          }
        } else {
          anyApplied = true;
        }
      } else {
        // v0.83.15 Part B: use the CAPTURED price span (from
        // vp.priceFrom/priceTo) if available, else current range.
        let half: number | undefined;
        if (
          typeof targetPriceSpan === "number" &&
          Number.isFinite(targetPriceSpan) &&
          targetPriceSpan > 0
        ) {
          half = targetPriceSpan / 2;
        } else {
          const cur = priceScale.getVisibleRange?.();
          if (cur) {
            const rf = Number(cur.from);
            const rt = Number(cur.to);
            const span = rt - rf;
            if (Number.isFinite(span) && span > 0) half = span / 2;
          }
        }
        if (half != null && half > 0) {
          try {
            priceScale.setVisibleRange?.({
              from: anchorPrice - half,
              to: anchorPrice + half,
            });
            anyApplied = true;
          } catch { /* ignore */ }
        }
      }
    }
    return anyApplied;
  } catch {
    return false;
  }
}

// v0.83.8 Part C — anchor verification extracted for reuse.
// v0.83.11 Part C — fraction-based fallback when timeToCoordinate /
// priceToCoordinate return null.
function verifyAnchor(chart: IChartApi, vp: CapturedViewport): boolean {
  try {
    const timeScale = chart.timeScale();
    const priceScale = chart.priceScale("right");
    const TOL_PX = 2;
    const TOL_FRAC = 0.15; // v0.83.15 Part D — LC snaps ranges to bar boundaries
    const container =
      (chart as unknown as { chartElement?: () => HTMLElement | null }).chartElement?.() ?? null;
    const cEl = container as HTMLElement | null;
    const width = cEl?.clientWidth ?? (typeof window !== "undefined" ? window.innerWidth : 800);
    const height = cEl?.clientHeight ?? (typeof window !== "undefined" ? window.innerHeight : 600);
    const cx = Math.floor(width / 2);
    const cy = Math.floor(height / 2);
    const targetX = typeof vp.anchorXRatio === "number" && Number.isFinite(vp.anchorXRatio) && vp.anchorXRatio >= 0 && vp.anchorXRatio <= 1
      ? Math.floor(timeScale.width() * vp.anchorXRatio)
      : cx;
    // ---- X check ----
    if (typeof vp.anchorTs === "number" && vp.anchorTs > 0) {
      const px = timeScale.timeToCoordinate(vp.anchorTs as UTCTimestamp);
      if (px != null) {
        if (Math.abs((px as number) - targetX) > TOL_PX) return false;
      } else {
        // Fallback: check fraction within visible range
        const vr = timeScale.getVisibleRange?.() as unknown as {
          from: unknown;
          to: unknown;
        } | null;
        if (!vr) return false;
        const from = Number(vr.from);
        const to = Number(vr.to);
        if (to <= from) return false;
        const frac = (vp.anchorTs - from) / (to - from);
        const targetRatio = typeof vp.anchorXRatio === "number" ? vp.anchorXRatio : 0.5;
        if (Math.abs(frac - targetRatio) > TOL_FRAC) return false;
      }
    }
    // ---- Y check ----
    if (
      typeof vp.anchorPrice === "number" &&
      Number.isFinite(vp.anchorPrice) &&
      vp.anchorPrice !== 0
    ) {
      const py = (
        priceScale as unknown as {
          priceToCoordinate?: (p: number) => number | null;
        }
      ).priceToCoordinate?.(vp.anchorPrice);
      if (py != null) {
        if (Math.abs(py - cy) > TOL_PX) return false;
      } else {
        const pr = (
          priceScale as unknown as {
            getVisibleRange?: () => { from: unknown; to: unknown } | null;
          }
        ).getVisibleRange?.();
        if (!pr) return false;
        const from = Number(pr.from);
        const to = Number(pr.to);
        if (to <= from) return false;
        const frac = (vp.anchorPrice - from) / (to - from);
        if (Math.abs(frac - 0.5) > TOL_FRAC) return false;
      }
    }
    return true;
  } catch {
    return false;
  }
}

// v0.83.6 — capture the (timestamp, price) at the chart container center.
// v0.83.11 — robust fallbacks: mid of visible range when
// coordinateToTime / coordinateToPrice return null.
function captureAnchorPoint(chart: IChartApi, anchorXRatio?: number): {
  anchorTs?: number;
  anchorPrice?: number;
  anchorXRatio?: number;
} {
  try {
    const timeScale = chart.timeScale();
    const chartEl = (
      chart as unknown as { chartElement?: () => HTMLElement | null }
    ).chartElement?.() ?? null;
    const el = chartEl as HTMLElement | null;
    const width =
      el?.clientWidth ?? (typeof window !== "undefined" ? window.innerWidth : 0);
    const height =
      el?.clientHeight ?? (typeof window !== "undefined" ? window.innerHeight : 0);
    const cx = Math.floor(width / 2);
    const cy = Math.floor(height / 2);
    const hasAnchorXRatio = typeof anchorXRatio === "number" && Number.isFinite(anchorXRatio) && anchorXRatio >= 0 && anchorXRatio <= 1;
    const anchorX = hasAnchorXRatio ? Math.floor(timeScale.width() * anchorXRatio) : cx;
    let anchorTs: number | undefined;
    let anchorPrice: number | undefined;
    // --- anchorTs ---
    try {
      const t = timeScale.coordinateToTime(anchorX) as unknown;
      if (t != null && Number.isFinite(Number(t))) anchorTs = Number(t);
    } catch {
      /* ignore */
    }
    // Fallback: if coordinateToTime failed, use mid of visible range
    if (anchorTs == null) {
      try {
        const vr = timeScale.getVisibleRange() as unknown as {
          from?: unknown;
          to?: unknown;
        } | null;
        if (
          vr &&
          typeof vr.from === "number" &&
          typeof vr.to === "number"
        ) {
          const ratio = hasAnchorXRatio ? anchorXRatio : 0.5;
          anchorTs = Math.floor(Number(vr.from) + (Number(vr.to) - Number(vr.from)) * ratio);
        }
      } catch {
        /* ignore */
      }
    }
    // --- anchorPrice ---
    try {
      const priceScale = chart.priceScale("right") as unknown as {
        coordinateToPrice?: (y: number) => unknown;
      };
      const p = priceScale.coordinateToPrice?.(cy);
      if (p != null && Number.isFinite(Number(p))) {
        anchorPrice = Number(p);
      }
    } catch {
      /* ignore */
    }
    // Fallback: if coordinateToPrice failed, use mid of price range
    if (anchorPrice == null) {
      try {
        const priceScale = chart.priceScale("right") as unknown as {
          getVisibleRange?: () => { from: unknown; to: unknown } | null;
        };
        const pr = priceScale.getVisibleRange?.();
        if (pr) {
          anchorPrice = (Number(pr.from) + Number(pr.to)) / 2;
        }
      } catch {
        /* ignore */
      }
    }
    return { anchorTs, anchorPrice, ...(hasAnchorXRatio ? { anchorXRatio } : {}) };
  } catch {
    return {};
  }
}

export function applyAnchorPoint(
  chart: IChartApi,
  anchorTs: number,
  anchorPrice: number,
): void {
  const timeScale = chart.timeScale();
  const priceScale = chart.priceScale("right") as unknown as {
    priceToCoordinate?: (price: number) => number | null;
    getVisibleRange?: () => { from: number; to: number } | null;
    setVisibleRange?: (range: { from: number; to: number }) => void;
  };

  const chartEl = (
    chart as unknown as { chartElement?: () => HTMLElement | null }
  ).chartElement?.() ?? null;
  const el = chartEl as HTMLElement | null;
  const width =
    el?.clientWidth ?? (typeof window !== "undefined" ? window.innerWidth : 0);
  const height =
    el?.clientHeight ?? (typeof window !== "undefined" ? window.innerHeight : 0);
  const cx = Math.floor(width / 2);
  const cy = Math.floor(height / 2);

  // ---- Time (X) shift ----
  try {
    const t = anchorTs as unknown as Parameters<typeof timeScale.timeToCoordinate>[0];
    const px = timeScale.timeToCoordinate(t);
    if (px != null) {
      const delta = cx - (px as number);
      if (Math.abs(delta) > 0.5) {
        const logical = timeScale.getVisibleLogicalRange?.();
        if (logical) {
          const barsPerPixel =
            (logical.to - logical.from) / Math.max(1, width);
          const deltaBars = delta * barsPerPixel;
          timeScale.setVisibleLogicalRange({
            from: logical.from - deltaBars,
            to: logical.to - deltaBars,
          });
        }
      }
    }
  } catch { /* ignore */ }

  // ---- Price (Y) shift ----
  try {
    const py = priceScale.priceToCoordinate?.(anchorPrice);
    if (py != null) {
      const delta = cy - py;
      if (Math.abs(delta) > 0.5) {
        // Shift the price scale by the pixel delta.
        // LC does not expose a direct scrollBy for price;
        // the standard approach is to read the current price
        // range, compute the price delta, and set the new
        // range.
        const range = priceScale.getVisibleRange?.();
        if (range) {
          const pricePerPixel =
            (Number(range.to) - Number(range.from)) /
            Math.max(1, height);
          const deltaPrice = delta * pricePerPixel;
          priceScale.setVisibleRange?.({
            from: Number(range.from) + deltaPrice,
            to: Number(range.to) + deltaPrice,
          });
        }
      }
    }
  } catch { /* ignore */ }

  // v0.83.6 Part F — dev-only drift warning (no user-facing error).
  try {
    if (typeof import.meta !== "undefined" && import.meta.env?.DEV) {
      const px2 = timeScale.timeToCoordinate(
        anchorTs as unknown as Parameters<typeof timeScale.timeToCoordinate>[0],
      );
      const py2 = priceScale.priceToCoordinate?.(anchorPrice);
      const dx = px2 == null ? NaN : Math.abs((px2 as number) - cx);
      const dy = py2 == null ? NaN : Math.abs(py2 - cy);
      if (
        (Number.isFinite(dx) && dx > 10) ||
        (Number.isFinite(dy) && dy > 10)
      ) {
        console.warn(
          `[viewport] anchor drift: dx=${Number.isFinite(dx) ? dx.toFixed(1) : "?"}px dy=${Number.isFinite(dy) ? dy.toFixed(1) : "?"}px (tolerance 10px)`,
        );
      }
    }
  } catch { /* ignore */ }
}
