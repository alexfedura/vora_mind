import type {
  IChartApi,
  ISeriesApi,
  ISeriesPrimitive,
  IPrimitivePaneView,
  IPrimitivePaneRenderer,
  SeriesType,
  Time,
} from 'lightweight-charts';
import { drawingTimeToCoordinate } from "./drawingCoordinates";

// CanvasRenderingTarget2D lives in `fancy-canvas` (transitive dep of
// lightweight-charts) and is NOT re-exported by lightweight-charts.
// Derive the type structurally instead of importing a new dependency.
type PaneRenderTarget = Parameters<IPrimitivePaneRenderer["draw"]>[0];

export interface MeasurePending {
  t: number;
  p: number;
}

export interface MeasureComplete {
  tA: number; pA: number;
  tB: number; pB: number;
}

export interface MeasureState {
  active: boolean;
  pending: MeasurePending | null;
  complete: MeasureComplete | null;
}

export interface XButtonRect {
  x: number; y: number; w: number; h: number;
}

class MeasurePaneRenderer implements IPrimitivePaneRenderer {
  constructor(
    private _state: MeasureState,
    private _series: ISeriesApi<SeriesType>,
    private _chart: IChartApi,
    private _setXRect: (r: XButtonRect | null) => void,
    private _getSnap: () => { t: number; p: number } | null,
  ) {}

  draw(target: PaneRenderTarget): void {
    const s = this._state;
    target.useBitmapCoordinateSpace((scope) => {
      const ctx = scope.context;
      const hr = scope.horizontalPixelRatio;
      const vr = scope.verticalPixelRatio;
      ctx.save();
      try {
        // v7.0.0 B5: snap marker — 5px orange ring, no fill (always on top)
        const drawSnap = () => {
          const snap = this._getSnap();
          if (!snap) return;
          const sx = drawingTimeToCoordinate(this._chart.timeScale(), snap.t);
          const sy = this._series.priceToCoordinate(snap.p);
          if (sx == null || sy == null) return;
          ctx.strokeStyle = '#ff9800';
          ctx.lineWidth = 1.5 * Math.min(hr, vr);
          ctx.beginPath();
          ctx.arc(sx * hr, sy * vr, 5 * Math.min(hr, vr), 0, Math.PI * 2);
          ctx.stroke();
        };
        if (s.pending && !s.complete) {
          const yMedia = this._series.priceToCoordinate(s.pending.p);
          if (yMedia == null) { this._setXRect(null); return; }
          const y = yMedia * vr;
          const wCss = scope.mediaSize.width;
          ctx.strokeStyle = '#2196f3';
          ctx.lineWidth = 1 * Math.min(hr, vr);
          ctx.setLineDash([4 * hr, 4 * hr]);
          ctx.beginPath();
          ctx.moveTo(0, y);
          ctx.lineTo(wCss * hr, y);
          ctx.stroke();
          ctx.setLineDash([]);
          this._setXRect(null);
          drawSnap();
          return;
        }
        if (s.complete) {
          const c = s.complete;
          const ts = this._chart.timeScale();
          const xA = drawingTimeToCoordinate(ts, c.tA);
          const xB = drawingTimeToCoordinate(ts, c.tB);
          if (xA == null || xB == null) { this._setXRect(null); return; }
          const yA = this._series.priceToCoordinate(c.pA);
          const yB = this._series.priceToCoordinate(c.pB);
          if (yA == null || yB == null) { this._setXRect(null); return; }
          const rx = Math.min(xA, xB);
          const ry = Math.min(yA, yB);
          const rw = Math.abs(xB - xA);
          const rh = Math.abs(yB - yA);
          ctx.fillStyle = 'rgba(33,150,243,0.10)';
          ctx.fillRect(rx * hr, ry * vr, rw * hr, rh * vr);
          ctx.strokeStyle = 'rgba(33,150,243,1)';
          ctx.lineWidth = 1 * Math.min(hr, vr);
          ctx.strokeRect(rx * hr, ry * vr, rw * hr, rh * vr);
          const topPrice = Math.max(c.pA, c.pB);
          const botPrice = Math.min(c.pA, c.pB);
          const dPrice = topPrice - botPrice;
          const dPct = topPrice !== 0 ? (dPrice / topPrice) * 100 : 0;
          const text = `\u0394 ${dPrice.toFixed(4)} | ${dPct.toFixed(4)}%`;
          ctx.font = `${11 * vr}px ui-monospace, monospace`;
          ctx.textBaseline = 'top';
          ctx.fillStyle = '#2196f3';
          const textWMedia = ctx.measureText(text).width / hr;
          const labelX = rx;
          const labelY = ry + rh + 4;
          ctx.fillText(text, labelX * hr, labelY * vr);
          const xBtnSize = 16;
          const xBtnX = labelX + textWMedia + 8;
          const xBtnY = ry + rh + 4;
          ctx.fillStyle = 'rgba(33,150,243,0.9)';
          ctx.fillRect(xBtnX * hr, xBtnY * vr, xBtnSize * hr, xBtnSize * vr);
          ctx.fillStyle = '#fff';
          ctx.font = `${11 * vr}px ui-monospace, monospace`;
          ctx.textBaseline = 'middle';
          const glyph = '×';
          const gw = ctx.measureText(glyph).width;
          ctx.fillText(glyph, xBtnX * hr + (xBtnSize * hr - gw) / 2, xBtnY * vr + (xBtnSize * vr) / 2);
          this._setXRect({ x: xBtnX, y: xBtnY, w: xBtnSize, h: xBtnSize });
          drawSnap();
          return;
        }
        this._setXRect(null);
        drawSnap();
      } finally {
        ctx.restore();
      }
    });
  }
}

class MeasurePaneView implements IPrimitivePaneView {
  constructor(
    private _state: MeasureState,
    private _series: ISeriesApi<SeriesType>,
    private _chart: IChartApi,
    private _setXRect: (r: XButtonRect | null) => void,
    private _getSnap: () => { t: number; p: number } | null,
  ) {}
  zOrder(): "top" | "normal" | "bottom" { return "top"; }
  renderer(): IPrimitivePaneRenderer {
    return new MeasurePaneRenderer(this._state, this._series, this._chart, this._setXRect, this._getSnap);
  }
}

export class MeasureOverlay implements ISeriesPrimitive<Time> {
  private _chart: IChartApi;
  private _series: ISeriesApi<SeriesType>;
  private _state: MeasureState = { active: false, pending: null, complete: null };
  // v7.0.0 B5: snap marker — snapped OHLC point drawn as orange ring
  private _snap: { t: number; p: number } | null = null;
  private _lastXRectMedia: XButtonRect | null = null;
  private _requestUpdate?: () => void;

  constructor(chart: IChartApi, series: ISeriesApi<SeriesType>) {
    this._chart = chart;
    this._series = series;
  }

  attached(param: { requestUpdate: () => void }): void {
    this._requestUpdate = param.requestUpdate;
  }

  detached(): void {
    this._requestUpdate = undefined;
  }

  setState(patch: Partial<MeasureState>): void {
    this._state = { ...this._state, ...patch };
    this._requestUpdate?.();
  }

  getState(): MeasureState { return this._state; }
  getSeries(): ISeriesApi<SeriesType> { return this._series; }
  getChart(): IChartApi { return this._chart; }
  getXButtonRect(): XButtonRect | null { return this._lastXRectMedia; }
  /** v7.0.0 B5: set the snap-feedback marker (null = hide). */
  setSnap(s: { t: number; p: number } | null): void {
    this._snap = s;
    this._requestUpdate?.();
  }
  clear(): void {
    this._state = { ...this._state, pending: null, complete: null };
    this._lastXRectMedia = null;
    this._requestUpdate?.();
  }

  updateAllViews(): void { /* no-op */ }

  paneViews(): readonly IPrimitivePaneView[] {
    return [new MeasurePaneView(this._state, this._series, this._chart, (r) => { this._lastXRectMedia = r; }, () => this._snap)];
  }
}
