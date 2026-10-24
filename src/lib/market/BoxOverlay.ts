// ═══════════════════════════════════════════════════════════════
// BoxOverlay — hollow rectangles for detected boxes
// Reference: docs/box-anatomy.md v1.0.0 (A=top-left, B=top-right,
//            C=bottom-left, D=bottom-right, E/F=engulf continuation)
// ═══════════════════════════════════════════════════════════════

import type {
  IChartApi,
  ISeriesApi,
  ISeriesPrimitive,
  IPrimitivePaneView,
  IPrimitivePaneRenderer,
  SeriesType,
  Time,
  UTCTimestamp,
} from "lightweight-charts";
import type { BoxDto, BoxExtend } from "./marketApi";

// CanvasRenderingTarget2D lives in `fancy-canvas` (transitive dep of
// lightweight-charts) and is NOT re-exported by lightweight-charts.
// Derive the type structurally instead of importing a new dependency.
type PaneRenderTarget = Parameters<IPrimitivePaneRenderer["draw"]>[0];

const ROLE_COLORS: Record<string, string> = {
  large:  "#9c27b0",
  medium: "#ff9800",
  small:  "#4caf50",
};

const BORDER_WIDTH = 2;

class BoxPaneRenderer implements IPrimitivePaneRenderer {
  constructor(
    private _boxes: BoxDto[],
    private _series: ISeriesApi<SeriesType>,
    private _chart: IChartApi,
    private _extend: BoxExtend = { left: 0, right: 0 },
    private _showEngulfZone = true,
    // v5.2.4: chain root scale (for border style)
    private _chainRootScale: Map<number, number> = new Map(),
    // v6.4.1: registry scale colors (key = scale index)
    private _scaleColors: Map<number, string> = new Map(),
  ) {}

  draw(target: PaneRenderTarget): void {
    if (this._boxes.length === 0) return;
    target.useBitmapCoordinateSpace((scope) => {
      const ctx = scope.context;
      const hr  = scope.horizontalPixelRatio;
      const vr  = scope.verticalPixelRatio;
      const k   = Math.min(hr, vr);
      const ts  = this._chart.timeScale();

      ctx.save();
      ctx.lineJoin  = "miter";
      ctx.lineCap   = "butt";

      for (const box of this._boxes) {
        const scaleIdx =
          typeof box.scale === "number"
            ? box.scale
            : parseInt(String(box.scale), 10);
        const color =
          (Number.isFinite(scaleIdx)
            ? this._scaleColors.get(scaleIdx)
            : undefined) ??
          ROLE_COLORS[box.role] ??
          "#ffffff";
        const bs = (ts.options() as unknown as { barSpacing?: number }).barSpacing ?? 10;
        const shiftL = bs * this._extend.left;
        const shiftR = bs * this._extend.right;

        const xRawL = ts.timeToCoordinate(box.ts_start as Time);
        const xRawR = ts.timeToCoordinate(box.ts_end as Time);
        if (xRawL === null || xRawR === null) continue;

        const xA = xRawL + shiftL;   // Left edge (AC)
        const xB = xRawR + shiftR;   // Right edge (BD)
        if (xB <= xA) continue;

        const yA = this._series.priceToCoordinate(box.high);
        const yC = this._series.priceToCoordinate(box.low);
        if (yA === null || yC === null) continue;

        ctx.strokeStyle = color;
        ctx.lineWidth = BORDER_WIDTH * k;
        // v5.2.4: solid if box.scale == chain root, else dashed.
        //   Color is UNCHANGED — only line style differs.
        //   Full saturation, no globalAlpha on the border.
        const boxChainId = box.chain_id;
        const boxScale = box.scale;
        const rootScale = boxChainId != null
          ? this._chainRootScale.get(boxChainId)
          : undefined;
        const isRootBox =
          rootScale === undefined || boxScale === rootScale;
        ctx.setLineDash(isRootBox ? [] : [4 * k, 4 * k]);
        ctx.beginPath();
        ctx.moveTo(xA * hr, yA * vr);   // A — TopLeft
        ctx.lineTo(xB * hr, yA * vr);   // B — Top edge (AB)
        ctx.lineTo(xB * hr, yC * vr);   // D — Right edge (BD)
        ctx.lineTo(xA * hr, yC * vr);   // C — Bottom edge (CD)
        ctx.closePath();                // C → A — Left edge (AC)
        ctx.stroke();
        ctx.setLineDash([]);   // reset — do not leak to next draw

        if (this._showEngulfZone && box.engulf_ts != null) {
          const xE = ts.timeToCoordinate(box.engulf_ts as UTCTimestamp as Time);
          if (xE !== null && xE > xB) {
            ctx.save();
            ctx.globalAlpha = 0.4;
            ctx.strokeStyle = color;
            ctx.lineWidth = BORDER_WIDTH * k;
            ctx.setLineDash([4 * k, 4 * k]);
            ctx.beginPath();
            // v5.2.2: one-sided engulf zone.
            //   RBR → bottom only (invalidated from below)
            //   DBD → top only    (invalidated from above)
            const isRBR = box.pattern === "RBR";
            if (isRBR) {
              ctx.moveTo(xB * hr, yC * vr);   // D → F (bottom)
              ctx.lineTo(xE * hr, yC * vr);
            } else {
              ctx.moveTo(xB * hr, yA * vr);   // B → E (top)
              ctx.lineTo(xE * hr, yA * vr);
            }
            ctx.stroke();
            ctx.restore();
          }
        }
      }

      ctx.restore();
    });
  }
}

class BoxPaneView implements IPrimitivePaneView {
  constructor(
    private _boxes: BoxDto[],
    private _series: ISeriesApi<SeriesType>,
    private _chart: IChartApi,
    private _extend: BoxExtend = { left: 0, right: 0 },
    private _showEngulfZone = true,
    private _chainRootScale: Map<number, number> = new Map(),
    private _scaleColors: Map<number, string> = new Map(),
  ) {}

  zOrder(): "top" | "normal" | "bottom" { return "top"; }

  renderer(): IPrimitivePaneRenderer {
    return new BoxPaneRenderer(this._boxes, this._series, this._chart, this._extend, this._showEngulfZone, this._chainRootScale, this._scaleColors);
  }
}

export class BoxOverlay implements ISeriesPrimitive<Time> {
  private _boxes: BoxDto[] = [];
  // v5.2.4: chain root scale (for border style)
  private _chainRootScale: Map<number, number> = new Map();
  // v6.4.1: registry scale colors (key = scale index 0..N-1)
  private scaleColors: Map<number, string> = new Map();
  private _extend: BoxExtend = { left: 0, right: 0 };
  private _showEngulfZone = true;
  private _series: ISeriesApi<SeriesType>;
  private _chart: IChartApi;
  private _requestUpdate?: () => void;

  constructor(chart: IChartApi, series: ISeriesApi<SeriesType>) {
    this._chart  = chart;
    this._series = series;
  }

  attached(param: { requestUpdate: () => void }): void {
    this._requestUpdate = param.requestUpdate;
  }

  detached(): void {
    this._requestUpdate = undefined;
    this._boxes = [];
    this._chainRootScale.clear();
  }

  setBoxes(boxes: BoxDto[]): void {
    this._boxes = boxes.slice();
    // v5.2.4: precompute chain root scale.
    this._chainRootScale.clear();
    for (const b of boxes) {
      const cid = b.chain_id;
      const pos = b.chain_pos;
      const sc = b.scale;
      if (cid != null && pos === 0 && typeof sc === "number") {
        this._chainRootScale.set(cid, sc);
      }
    }
    this._requestUpdate?.();
  }

  public setExtend(v: BoxExtend): void {
    const clamp = (n: number) => Math.max(-20, Math.min(20, n));
    const next = { left: clamp(v.left), right: clamp(v.right) };
    if (this._extend.left === next.left &&
        this._extend.right === next.right) return;
    this._extend = next;
    this._requestUpdate?.();
  }

  public setShowEngulfZone(v: boolean): void {
    if (this._showEngulfZone === v) return;
    this._showEngulfZone = v;
    this._requestUpdate?.();
  }

  public setScaleColors(map: Map<number, string>): void {
    let changed = map.size !== this.scaleColors.size;
    if (!changed) {
      for (const [k, v] of map) {
        if (this.scaleColors.get(k) !== v) {
          changed = true;
          break;
        }
      }
    }
    if (!changed) return;
    this.scaleColors = new Map(map);
    this._requestUpdate?.();
  }

  clear(): void {
    this._boxes = [];
    this._chainRootScale.clear();
    this._requestUpdate?.();
  }

  updateAllViews(): void { /* no-op */ }

  paneViews(): readonly IPrimitivePaneView[] {
    return [ new BoxPaneView(this._boxes, this._series, this._chart, this._extend, this._showEngulfZone, this._chainRootScale, this.scaleColors) ];
  }
}

