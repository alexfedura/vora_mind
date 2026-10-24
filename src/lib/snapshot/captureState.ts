import type { IChartApi } from "lightweight-charts";
import type {
  SnapshotChartState,
  SnapshotDrawing,
  SnapshotPolicy,
  SnapshotTheme,
} from "./types";
import type { DrawingAny } from "@/lib/market/lineDrawings";
import type { DrawingRect } from "@/lib/market/RectangleOverlay";
import type { PolicyDefaults } from "@/components/market/PolicyPanel";
import type { ScaleRegistry } from "@/lib/market/marketApi";
import { TV_DARK } from "@/lib/market/palette";

export type ChartDisplayMode = "special" | "candlestick";

export function captureChartState(opts: {
  symbol: string;
  timeframe: string;
  displayMode: ChartDisplayMode;
}): SnapshotChartState {
  return {
    symbol: opts.symbol,
    exchange: "VORA",
    interval: opts.timeframe,
    dataType: "candles",
    displayMode: opts.displayMode === "special" ? "special" : "candlestick",
    priceScale: "linear",
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
    currency: "USD",
  };
}

function styleOf(d: DrawingAny): "solid" | "dashed" | "dotted" {
  const s = (d as { style?: unknown }).style;
  return s === "dashed" || s === "dotted" ? s : "solid";
}

function lockedOf(d: DrawingAny): boolean | undefined {
  return (d as { locked?: unknown }).locked === true ? true : undefined;
}

function widthOf(d: DrawingAny): number {
  const w = (d as { lineWidth?: unknown }).lineWidth;
  return typeof w === "number" && Number.isFinite(w) ? w : 1;
}

function colorOf(d: DrawingAny, fallback: string): string {
  const rec = d as unknown as Record<string, unknown>;
  const c = rec["color"] ?? (d as unknown as DrawingRect).borderColor;
  return typeof c === "string" && c.length > 0 ? c : fallback;
}

export function captureDrawings(drawings: DrawingAny[]): SnapshotDrawing[] {
  const out: SnapshotDrawing[] = [];
  for (const d of drawings) {
    if (!d) continue;
    const style = styleOf(d);
    const locked = lockedOf(d);
    const lineWidth = widthOf(d);
    if (d.type === "geometry") {
      out.push({
        id: d.id,
        type: "geometry",
        tool: d.tool,
        name: d.name,
        points: d.points.map((point) => ({ ts: point.ts * 1000, price: point.price })),
        sourceBars: d.sourceBars?.map((bar) => ({ ...bar, ts: bar.ts * 1000 })),
        color: colorOf(d, "#ffb74d"),
        colorOpacity: d.colorOpacity,
        lineWidth,
        style,
        locked,
        hidden: d.hidden,
        visibleOnIntervals: d.visibleOnIntervals ? [...d.visibleOnIntervals] : undefined,
        text: d.text,
        textColor: d.textColor,
        textOpacity: d.textOpacity,
        textSize: d.textSize,
        textBold: d.textBold,
        textItalic: d.textItalic,
        textAlignHorizontal: d.textAlignHorizontal,
        textAlignVertical: d.textAlignVertical,
      });
      continue;
    }
    if (d.type === "icon") {
      out.push({
        id: d.id,
        type: "icon",
        category: d.category,
        symbol: d.symbol,
        size: d.size,
        name: d.name,
        points: [{ ts: d.ts * 1000, price: d.price }],
        text: d.text,
        textColor: d.textColor,
        textOpacity: d.textOpacity,
        textSize: d.textSize,
        textBold: d.textBold,
        textItalic: d.textItalic,
        textAlignHorizontal: d.textAlignHorizontal,
        textAlignVertical: d.textAlignVertical,
        color: colorOf(d, "#ffb74d"), lineWidth, style, locked,
      });
      continue;
    }
    if (d.type === "fib-gann") {
      out.push({
        id: d.id,
        type: "fib-gann",
        tool: d.tool,
        points: [
          { ts: d.ts1 * 1000, price: d.price1 },
          { ts: d.ts2 * 1000, price: d.price2 },
          ...(d.ts3 !== undefined && d.price3 !== undefined ? [{ ts: d.ts3 * 1000, price: d.price3 }] : []),
        ],
        color: colorOf(d, "#ffb74d"), lineWidth, style, locked,
      });
      continue;
    }
    if (d.type === "rectangle") {
      const r = d as DrawingRect;
      out.push({
        id: r.id, type: "rect",
        points: [
          { ts: r.ts_start * 1000, price: r.price_low },
          { ts: r.ts_end * 1000, price: r.price_high },
        ],
        color: colorOf(d, "#ffb74d"), lineWidth, style,
        bgColor: r.fillColor ?? undefined, locked,
      });
      continue;
    }
    if (d.type === "vline") {
      out.push({
        id: d.id, type: "vline",
        points: [{ ts: d.ts * 1000, price: 0 }],
        color: colorOf(d, "#ffb74d"), lineWidth, style, locked,
      });
      continue;
    }
    if (d.type === "hline") {
      out.push({
        id: d.id, type: "hline",
        points: [{ ts: 0, price: d.price }],
        color: colorOf(d, "#ffb74d"), lineWidth, style, locked,
      });
      continue;
    }
    if (d.type === "hray") {
      out.push({
        id: d.id, type: "hray",
        points: [{ ts: d.ts * 1000, price: d.price }],
        color: colorOf(d, "#ffb74d"), lineWidth, style, locked,
      });
      continue;
    }
    if (d.type === "trendline") {
      out.push({
        id: d.id, type: "trendline",
        points: [
          { ts: d.ts1 * 1000, price: d.price1 },
          { ts: d.ts2 * 1000, price: d.price2 },
        ],
        color: colorOf(d, "#ffb74d"), lineWidth, style, locked,
      });
      continue;
    }
    if (
      d.type === "ray" ||
      d.type === "info-line" ||
      d.type === "extended-line" ||
      d.type === "trend-angle" ||
      d.type === "regression-trend"
    ) {
      out.push({
        id: d.id, type: d.type,
        points: [
          { ts: d.ts1 * 1000, price: d.price1 },
          { ts: d.ts2 * 1000, price: d.price2 },
        ],
        color: colorOf(d, "#ffb74d"), lineWidth, style, locked,
      });
      continue;
    }
    if (d.type === "crossline") {
      out.push({
        id: d.id, type: "crossline",
        points: [{ ts: d.ts * 1000, price: d.price }],
        color: colorOf(d, "#ffb74d"), lineWidth, style, locked,
      });
      continue;
    }
    if (d.type === "parallel-channel") {
      out.push({
        id: d.id, type: "parallel-channel",
        points: [
          { ts: d.ts1 * 1000, price: d.price1 },
          { ts: d.ts2 * 1000, price: d.price2 },
          { ts: d.ts3 * 1000, price: d.price3 },
        ],
        color: colorOf(d, "#ffb74d"), lineWidth, style, locked,
      });
      continue;
    }
    if (d.type === "pitchfork") {
      out.push({
        id: d.id, type: "pitchfork",
        points: [
          { ts: d.ts1 * 1000, price: d.price1 },
          { ts: d.ts2 * 1000, price: d.price2 },
          { ts: d.ts3 * 1000, price: d.price3 },
        ],
        color: colorOf(d, "#ffb74d"), lineWidth, style, locked,
      });
    }
  }
  return out;
}

export function capturePolicy(policy: PolicyDefaults, registry: ScaleRegistry | null): SnapshotPolicy {
  const reg = (registry?.scales ?? []).map((s, i) => {
    const digits = String(s.id).replace(/[^0-9]/g, "");
    const parsed = digits.length > 0 ? parseInt(digits, 10) : NaN;
    return {
      id: Number.isFinite(parsed) ? parsed : i,
      color: s.color, min: s.start_pct, max: s.end_pct,
    };
  });
  return {
    scaleBase: policy.user_step,
    roles: { small: policy.s_small, medium: policy.s_medium, large: policy.s_large },
    registry: reg,
  };
}

export function captureTheme(chart: IChartApi | null): SnapshotTheme {
  let background: string = TV_DARK.chartBg;
  let textColor: string = TV_DARK.textPrimary;
  try {
    const opts = chart?.options() as unknown as {
      layout?: { background?: { color?: unknown }; textColor?: unknown };
    } | undefined;
    const bgc = opts?.layout?.background?.color;
    if (typeof bgc === "string" && bgc.length > 0) background = bgc;
    const tc = opts?.layout?.textColor;
    if (typeof tc === "string" && tc.length > 0) textColor = tc;
  } catch { /* ignore */ }
  return {
    background, gridColor: TV_DARK.gridMinor,
    upColor: TV_DARK.upColor, downColor: TV_DARK.downColor, textColor,
  };
}
