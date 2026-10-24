import type { IChartApi } from "lightweight-charts";
import type { SnapshotViewport } from "./types";
import { applyAnchorPoint, type CapturedViewport } from "@/lib/chart/viewportPreserve";

// v0.83.7 Part D — adapt a snapshot viewport (ms timestamps) to the
// timeframe CapturedViewport shape (LC-native seconds) so the restore
// path can use the verified apply (applyViewportVerified).
export function snapshotToCapturedViewport(v: SnapshotViewport): CapturedViewport {
  return {
    barSpacing: v.barSpacing,
    rightOffset: v.rightOffset,
    fromTs: v.fromTs / 1000,
    toTs: v.toTs / 1000,
    fromLogical: v.fromLogical,
    toLogical: v.toLogical,
    priceFrom: v.priceFrom,
    priceTo: v.priceTo,
    autoScale: v.autoScale ?? true,
    anchorTs: v.anchorTs,
    anchorPrice: v.anchorPrice,
  };
}

export function applySnapshotViewport(chart: IChartApi, v: SnapshotViewport): string[] {
  const warnings: string[] = [];
  const timeScale = chart.timeScale();
  let priceScale: ReturnType<IChartApi["priceScale"]> | null = null;
  try {
    priceScale = chart.priceScale("right");
  } catch {
    priceScale = null;
  }

  // ---- Time (horizontal) ----

  // 1. Bar spacing first so the visible range is computed against
  //    the correct pixel-per-bar.
  if (typeof v.barSpacing === "number" && v.barSpacing > 0) {
    try {
      timeScale.applyOptions({ barSpacing: v.barSpacing });
    } catch (e: unknown) {
      warnings.push("barSpacing failed: " + String(e));
    }
  }

  // 2. Prefer logical range — it survives different candle counts
  //    and is immune to timezone / calendar gaps.
  if (
    typeof v.fromLogical === "number" &&
    typeof v.toLogical === "number" &&
    (v.fromLogical !== 0 || v.toLogical !== 0)
  ) {
    try {
      timeScale.setVisibleLogicalRange({ from: v.fromLogical, to: v.toLogical });
    } catch (e: unknown) {
      warnings.push("setVisibleLogicalRange failed: " + String(e));
    }
  } else if (
    typeof v.fromTs === "number" &&
    typeof v.toTs === "number" &&
    v.fromTs > 0 &&
    v.toTs > 0
  ) {
    try {
      timeScale.setVisibleRange({
        from: (v.fromTs / 1000) as never,
        to: (v.toTs / 1000) as never,
      });
    } catch (e: unknown) {
      warnings.push("setVisibleRange failed: " + String(e));
    }
  }

  if (typeof v.rightOffset === "number") {
    try {
      timeScale.applyOptions({ rightOffset: v.rightOffset });
    } catch (e: unknown) {
      warnings.push("rightOffset failed: " + String(e));
    }
  }

  // ---- Price (vertical) ----

  // 3. Restore autoScale state first.
  if (typeof v.autoScale === "boolean" && priceScale) {
    try {
      priceScale.applyOptions({ autoScale: v.autoScale });
    } catch (e: unknown) {
      warnings.push("priceScale autoScale failed: " + String(e));
    }
  }

  // 4. If autoScale was OFF and a saved price range exists, restore
  //    it explicitly. If autoScale was ON, do nothing — the chart
  //    will pick the range from the visible candles.
  if (
    v.autoScale === false &&
    typeof v.priceFrom === "number" &&
    typeof v.priceTo === "number" &&
    priceScale
  ) {
    try {
      priceScale.setVisibleRange({ from: v.priceFrom, to: v.priceTo });
    } catch {
      // Range may be rejected if outside data bounds — fall back
      // to autoScale so the chart doesn't stay blank.
      try {
        priceScale.applyOptions({ autoScale: true });
      } catch (e: unknown) {
        warnings.push("priceScale fallback failed: " + String(e));
      }
    }
  }

  // v0.83.6 — center the anchor point (after X zoom + Y range applied).
  if (
    typeof v.anchorTs === "number" &&
    typeof v.anchorPrice === "number"
  ) {
    try {
      applyAnchorPoint(chart, v.anchorTs, v.anchorPrice);
    } catch (e: unknown) {
      warnings.push("anchor centering failed: " + String(e));
    }
  }

  return warnings;
}
