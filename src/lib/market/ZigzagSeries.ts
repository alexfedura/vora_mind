// ═══════════════════════════════════════════════════════════════
// ZigzagSeries — Custom Series for Lightweight Charts v5
// Draws O-H-L-C zigzag: 4 points per candle slot ordered by firstExtreme.
// ═══════════════════════════════════════════════════════════════

import type {
  CustomData,
  CustomSeriesOptions,
  CustomSeriesPricePlotValues,
  CustomSeriesWhitespaceData,
  ICustomSeriesPaneRenderer,
  ICustomSeriesPaneView,
  PaneRendererCustomData,
  PriceToCoordinateConverter,
  Time,
  UTCTimestamp,
} from "lightweight-charts";
import { TV_DARK } from "./palette";

// CanvasRenderingTarget2D lives in `fancy-canvas` (transitive dep of
// lightweight-charts) and is NOT re-exported by lightweight-charts.
// Derive the type structurally instead of importing a new dependency.
type PaneRenderTarget = Parameters<ICustomSeriesPaneRenderer["draw"]>[0];
type BitmapScope = Parameters<
  Parameters<PaneRenderTarget["useBitmapCoordinateSpace"]>[0]
>[0];

export type FirstExtreme = "high_first" | "low_first" | "same";

export interface ZigzagPoint extends CustomData<Time> {
  time: UTCTimestamp;
  open: number;
  high: number;
  low: number;
  close: number;
  firstExtreme: FirstExtreme;
}

export interface ZigzagSeriesOptions extends CustomSeriesOptions {
  lineColor: string;
  lineWidth: number;
  dotColor: string;
  dotRadius: number;
  /** v7.0.0: display mode — "special" (pivot-line) or "candlestick" (OHLC). */
  displayMode: "special" | "candlestick";
  scaffoldOnly: boolean;
}

const DEFAULTS: ZigzagSeriesOptions = {
  // CustomSeriesOptions (SeriesOptions<CustomStyleOptions>) requires the
  // full base set; keep TV-like dark defaults and override style at creation.
  ...({
    layout: { background: { type: 0 as never, color: "transparent" }, textColor: "#9ca3af" },
    priceLineVisible: false,
    priceLineWidth: 1,
    priceLineColor: "",
    priceLineStyle: 0,
    priceFormat: { type: "price", precision: 2, minMove: 0.01 },
    baseLineVisible: false,
    baseLineWidth: 1,
    baseLineColor: "#000000",
    baseLineStyle: 0,
    crosshairMarkerVisible: true,
    crosshairMarkerRadius: 4,
    crosshairMarkerBorderColor: "",
    crosshairMarkerBackgroundColor: "",
    crosshairMarkerBorderWidth: 2,
    lastValueVisible: false,
    priceScaleId: "right",
    visible: true,
    title: "",
    color: "#2962ff",
    autoscaleInfoProvider: undefined,
  } as unknown as CustomSeriesOptions),
  lineColor: "#2962ff",
  lineWidth: 2,
  dotColor: "#ef5350",
  dotRadius: 3,
  displayMode: "special",
  scaffoldOnly: false,
};

class ZigzagRenderer implements ICustomSeriesPaneRenderer {
  private _data: PaneRendererCustomData<Time, ZigzagPoint> | null = null;
  private _options: ZigzagSeriesOptions = DEFAULTS;

  update(
    data: PaneRendererCustomData<Time, ZigzagPoint>,
    options: ZigzagSeriesOptions,
  ): void {
    this._data = data;
    this._options = options;
  }

  draw(
    target: PaneRenderTarget,
    priceConverter: PriceToCoordinateConverter,
  ): void {
    const d = this._data;
    if (!d || !d.bars || d.bars.length === 0) return;

    const opt = this._options;
    if (opt.scaffoldOnly) return;
    const barSpacingRaw = d.barSpacing;
    if (
      typeof barSpacingRaw !== "number" ||
      !Number.isFinite(barSpacingRaw) ||
      barSpacingRaw <= 0
    ) {
      return;
    }
    const bs = barSpacingRaw;
    const spread = bs * 0.4;
    const third = spread / 3;

    // ─── visibleRange filter (LOGICAL INDEX, not timestamp) ───
    // d.bars is sorted & complete; array index == logical index.
    const visRange = (
      d as unknown as {
        visibleRange?: { from: number; to: number } | null;
      }
    ).visibleRange;

    let startI = 0;
    let endI = d.bars.length - 1;
    if (
      visRange &&
      Number.isFinite(visRange.from) &&
      Number.isFinite(visRange.to)
    ) {
      startI = Math.max(0, Math.floor(visRange.from));
      endI = Math.min(d.bars.length - 1, Math.ceil(visRange.to));
    }
    if (endI < startI) return;

    target.useBitmapCoordinateSpace((scope: BitmapScope) => {
      const ctx = scope.context as CanvasRenderingContext2D;
      const hr = scope.horizontalPixelRatio;
      const vpr = scope.verticalPixelRatio;
      const k = Math.min(hr, vpr);

      const bmpW = ctx.canvas ? ctx.canvas.width : 0;
      const bmpH = ctx.canvas ? ctx.canvas.height : 0;
      const hasBounds = bmpW > 0 && bmpH > 0;

      // ─── ONE-TIME DIAGNOSTIC ───
      const w = window as unknown as { __zz_diag_48?: boolean };
      if (!w.__zz_diag_48 && d.bars.length > 0) {
        w.__zz_diag_48 = true;
        const bars = d.bars as unknown as Array<{ x?: unknown }>;
        const xs: number[] = [];
        for (let i = 0; i < bars.length && xs.length < 10; i++) {
          const xv = bars[i]?.x;
          if (typeof xv === "number" && Number.isFinite(xv)) xs.push(xv);
        }
        // eslint-disable-next-line no-console
        console.log("[zz-diag-48]", {
          barsCount: bars.length,
          first10X: xs,
          barSpacing: bs,
          bmpW,
          bmpH,
          hr,
          vpr,
          visibleRange: visRange,
          startI,
          endI,
          rangeSize: endI - startI + 1,
        });
      }

      ctx.save();
      if (hasBounds) {
        ctx.beginPath();
        ctx.rect(0, 0, bmpW, bmpH);
        ctx.clip();
      }
      ctx.lineJoin = "round";
      ctx.lineCap = "round";

      const yOf = (p: number): number => {
        const y = priceConverter(p);
        return y === null ? NaN : (y as number);
      };

      // ─── v7.0.0: CANDLESTICK MODE ───
      // Non-destructive OHLC rendering. Same data, same viewport, same overlays.
      if (opt.displayMode === "candlestick") {
        // TradingView-style bodies preserve a visible gap while scaling with
        // the 180-candle default range; align dimensions to physical pixels.
        for (let i = startI; i <= endI; i++) {
          const bar = d.bars[i];
          const row = (bar as unknown as { originalData: ZigzagPoint })
            .originalData;
          const xRaw = (bar as unknown as { x?: unknown }).x;
          if (typeof xRaw !== "number" || !Number.isFinite(xRaw)) continue;
          const yO = yOf(row.open);
          const yH = yOf(row.high);
          const yL = yOf(row.low);
          const yC = yOf(row.close);
          if (![yO, yH, yL, yC].every(Number.isFinite)) continue;
          const up = row.close >= row.open;
          // v7.4.1 — TradingView Dark 2024 candle colors.
          const col = up ? TV_DARK.upColor : TV_DARK.downColor;
          const xCenter = Math.round(xRaw * hr);
          const bodyWidth = Math.max(1, Math.round(bs * 0.82 * hr));
          const bodyHeight = Math.max(1, Math.round(Math.abs(yC - yO) * vpr));
          ctx.strokeStyle = col;
          ctx.fillStyle = col;
          // 1px crisp wick (high → low through the body center).
          ctx.lineWidth = Math.max(1, Math.round(k));
          ctx.beginPath();
          ctx.moveTo(xCenter + 0.5, Math.round(yH * vpr) + 0.5);
          ctx.lineTo(xCenter + 0.5, Math.round(yL * vpr) + 0.5);
          ctx.stroke();
          // Body (no border, pure fill).
          const top = Math.min(yO, yC);
          ctx.fillRect(
            xCenter - Math.floor(bodyWidth / 2),
            Math.round(top * vpr),
            bodyWidth,
            bodyHeight,
          );
        }
        ctx.restore();
        return;
      }

      let prevX: number | null = null;
      let prevY: number | null = null;

      // ─── ITERATE ONLY VISIBLE RANGE ───
      for (let i = startI; i <= endI; i++) {
        const bar = d.bars[i];
        const row = (bar as unknown as { originalData: ZigzagPoint })
          .originalData;
        const xRaw = (bar as unknown as { x?: unknown }).x;

        if (typeof xRaw !== "number" || !Number.isFinite(xRaw)) {
          prevX = null;
          prevY = null;
          continue;
        }

        const xC = xRaw;
        const xOpen = xC - spread;
        const xP1 = xC - third;
        const xP2 = xC + third;
        const xClose = xC + spread;

        let pts: Array<[number, number]>;
        if (row.firstExtreme === "same") {
          pts = [
            [xOpen, yOf(row.open)],
            [xClose, yOf(row.close)],
          ];
        } else if (row.firstExtreme === "high_first") {
          pts = [
            [xOpen, yOf(row.open)],
            [xP1, yOf(row.high)],
            [xP2, yOf(row.low)],
            [xClose, yOf(row.close)],
          ];
        } else {
          pts = [
            [xOpen, yOf(row.open)],
            [xP1, yOf(row.low)],
            [xP2, yOf(row.high)],
            [xClose, yOf(row.close)],
          ];
        }

        pts = pts.filter(
          ([x, y]) => Number.isFinite(x) && Number.isFinite(y),
        );
        if (pts.length < 2) {
          prevX = null;
          prevY = null;
          continue;
        }

        // Chain threshold: adjacent only
        const gap = prevX === null ? Infinity : Math.abs(xC - prevX);
        const canChain =
          prevX !== null && prevY !== null && gap < bs * 5;

        ctx.strokeStyle = opt.lineColor;
        ctx.lineWidth = opt.lineWidth * k;
        ctx.beginPath();
        if (canChain) {
          ctx.moveTo(prevX! * hr, prevY! * vpr);
        } else {
          ctx.moveTo(pts[0][0] * hr, pts[0][1] * vpr);
        }
        for (const [x, y] of pts) {
          ctx.lineTo(x * hr, y * vpr);
        }
        ctx.stroke();

        ctx.fillStyle = opt.dotColor;
        for (const [x, y] of pts) {
          ctx.beginPath();
          ctx.arc(
            x * hr,
            y * vpr,
            opt.dotRadius * k,
            0,
            Math.PI * 2,
          );
          ctx.fill();
        }

        prevX = pts[pts.length - 1][0];
        prevY = pts[pts.length - 1][1];
      }

      ctx.restore();
    });

}
}

export class ZigzagSeries
  implements ICustomSeriesPaneView<Time, ZigzagPoint, ZigzagSeriesOptions>
{
  private _renderer = new ZigzagRenderer();

  renderer(): ICustomSeriesPaneRenderer {
    return this._renderer;
  }

  priceValueBuilder(plotRow: ZigzagPoint): CustomSeriesPricePlotValues {
    return [plotRow.low, plotRow.high, plotRow.close];
  }

  isWhitespace(
    data: ZigzagPoint | CustomSeriesWhitespaceData<Time>,
  ): data is CustomSeriesWhitespaceData<Time> {
    return (data as Partial<ZigzagPoint>).close === undefined;
  }

  defaultOptions(): ZigzagSeriesOptions {
    return { ...DEFAULTS };
  }

  update(
    data: PaneRendererCustomData<Time, ZigzagPoint>,
    options: ZigzagSeriesOptions,
  ): void {
    this._renderer.update(data, options);
  }
}
