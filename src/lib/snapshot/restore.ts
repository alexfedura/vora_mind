import type { VoraSnapshot } from "./types";
import { setSetting } from "@/shared/store/useSettingsStore";

export interface RestoreContext {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  chart: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  series: any | null;
  setSymbol: (s: string) => void;
  setExchange: (e: string) => void;
  setInterval: (i: string) => void;
  setDataType: (dt: VoraSnapshot["chart"]["dataType"]) => void;
  setDisplayMode: (dm: VoraSnapshot["chart"]["displayMode"]) => void;
  setPriceScale: (ps: VoraSnapshot["chart"]["priceScale"]) => void;
  loadCandles: (
    symbol: string,
    interval: string
  ) => Promise<Array<{
    time: number;
    open: number;
    high: number;
    low: number;
    close: number;
  }>>;
  setDrawings: (drawings: VoraSnapshot["drawings"]) => void;
  setScaleBase: (n: number) => void;
  setRoles: (r: VoraSnapshot["policy"]["roles"]) => void;
  setRegistry: (r: VoraSnapshot["policy"]["registry"]) => void;
  setTheme: (t: VoraSnapshot["theme"]) => void;
}

export interface RestoreResult {
  ok: boolean;
  warnings: string[];
  errors: string[];
}

export async function restoreSnapshot(
  snapshot: VoraSnapshot,
  ctx: RestoreContext
): Promise<RestoreResult> {
  const warnings: string[] = [];
  const errors: string[] = [];

  // 1. Chart metadata
  try {
    ctx.setSymbol(snapshot.chart.symbol);
    ctx.setExchange(snapshot.chart.exchange);
    ctx.setInterval(snapshot.chart.interval);
    ctx.setDataType(snapshot.chart.dataType);
    ctx.setDisplayMode(snapshot.chart.displayMode);
    ctx.setPriceScale(snapshot.chart.priceScale);
  } catch (e: unknown) {
    errors.push("setChartMetadata failed: " + (e instanceof Error ? e.message : String(e)));
  }

  // 2. Load candles
  let candles: Array<{ time: number; open: number; high: number; low: number; close: number }> = [];
  try {
    candles = await ctx.loadCandles(snapshot.chart.symbol, snapshot.chart.interval);
    if (!Array.isArray(candles) || candles.length === 0) {
      errors.push(
        "No candles for " + snapshot.chart.symbol + " @ " + snapshot.chart.interval
      );
      return { ok: false, warnings, errors };
    }
  } catch (e: unknown) {
    errors.push("loadCandles failed: " + (e instanceof Error ? e.message : String(e)));
    return { ok: false, warnings, errors };
  }

  // 3. Apply candles to series (if we own the series)
  try {
    if (ctx.series && typeof ctx.series.setData === "function") {
      ctx.series.setData(candles);
    } else {
      warnings.push(
        "series not accessible from restore helper; candles loaded via context only."
      );
    }
  } catch (e: unknown) {
    errors.push("series.setData failed: " + (e instanceof Error ? e.message : String(e)));
  }

  // 4. Apply viewport AFTER candles are set
  try {
    const timeScale = ctx.chart && ctx.chart.timeScale ? ctx.chart.timeScale() : null;
    if (timeScale) {
      const v = snapshot.viewport;

      // Prefer logical range (survives candle count differences)
      if (typeof v.fromLogical === "number" && typeof v.toLogical === "number") {
        try {
          timeScale.setVisibleLogicalRange({ from: v.fromLogical, to: v.toLogical });
        } catch {
          // fall through to time range
        }
      }

      // Also try time range if logical failed or is missing
      if (typeof v.fromTs === "number" && typeof v.toTs === "number" && v.fromTs > 0 && v.toTs > 0) {
        try {
          timeScale.setVisibleRange({
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            from: (v.fromTs / 1000) as any,
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            to: (v.toTs / 1000) as any,
          });
        } catch (err: unknown) {
          warnings.push("setVisibleRange failed: " + (err instanceof Error ? err.message : String(err)));
        }
      }

      if (typeof v.barSpacing === "number") {
        try {
          timeScale.applyOptions({ barSpacing: v.barSpacing });
        } catch (err: unknown) {
          warnings.push("applyOptions(barSpacing) failed: " + (err instanceof Error ? err.message : String(err)));
        }
      }
      if (typeof v.rightOffset === "number") {
        try {
          timeScale.applyOptions({ rightOffset: v.rightOffset });
        } catch (err: unknown) {
          warnings.push("applyOptions(rightOffset) failed: " + (err instanceof Error ? err.message : String(err)));
        }
      }
    } else {
      warnings.push("timeScale() unavailable on chart instance.");
    }
  } catch (e: unknown) {
    errors.push("applyViewport failed: " + (e instanceof Error ? e.message : String(e)));
  }

  // 5. Drawings
  try {
    ctx.setDrawings(snapshot.drawings || []);
  } catch (e: unknown) {
    errors.push("setDrawings failed: " + (e instanceof Error ? e.message : String(e)));
  }

  // 6. Policy panel
  try {
    if (typeof snapshot.policy.scaleBase === "number") {
      ctx.setScaleBase(snapshot.policy.scaleBase);
    }
    if (snapshot.policy.roles) {
      ctx.setRoles(snapshot.policy.roles);
    }
    if (Array.isArray(snapshot.policy.registry)) {
      ctx.setRegistry(snapshot.policy.registry);
    }
  } catch (e: unknown) {
    errors.push("applyPolicy failed: " + (e instanceof Error ? e.message : String(e)));
  }

  // 7. Theme
  try {
    if (snapshot.theme) {
      ctx.setTheme(snapshot.theme);
    }
  } catch (e: unknown) {
    errors.push("setTheme failed: " + (e instanceof Error ? e.message : String(e)));
  }

  // 8. Persist last-opened id
  try {
    setSetting("vora.snapshots.last", snapshot.id);
  } catch { /* ignore */ }

  return { ok: errors.length === 0, warnings, errors };
}
