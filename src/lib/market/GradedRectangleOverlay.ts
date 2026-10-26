import type {
  IChartApi,
  IPrimitivePaneRenderer,
  IPrimitivePaneView,
  ISeriesApi,
  ISeriesPrimitive,
  SeriesType,
  Time,
} from "lightweight-charts";
import type { CandleDto } from "./CandlestickChart";
import type { PivotDto } from "./marketApi";
import type { GradedRectangle } from "./lineDrawings";
import { drawingTimeToCoordinate } from "./drawingCoordinates";

const GRADED_RECT_COLOR = "#a855f7";

type PaneRenderTarget = Parameters<IPrimitivePaneRenderer["draw"]>[0];

function priceCoordinate(series: ISeriesApi<SeriesType>, price: number): number | null {
  return series.priceToCoordinate(price);
}

class GradedRectangleRenderer implements IPrimitivePaneRenderer {
  constructor(
    private readonly drawings: readonly GradedRectangle[],
    private readonly candles: readonly CandleDto[],
    private readonly hoverPivot: PivotDto | null,
    private readonly chart: IChartApi,
    private readonly series: ISeriesApi<SeriesType>,
  ) {}

  draw(target: PaneRenderTarget): void {
    target.useBitmapCoordinateSpace((scope) => {
      const ctx = scope.context;
      const hr = scope.horizontalPixelRatio;
      const vr = scope.verticalPixelRatio;
      const width = scope.mediaSize.width;
      const timeScale = this.chart.timeScale();
      const latest = this.candles[this.candles.length - 1];
      ctx.save();
      try {
        for (const drawing of this.drawings) {
          if (drawing.hidden) continue;
          const left = drawingTimeToCoordinate(timeScale, drawing.startTime);
          const right = drawingTimeToCoordinate(timeScale, drawing.endTime);
          const top = priceCoordinate(this.series, drawing.topPrice);
          const bottom = priceCoordinate(this.series, drawing.bottomPrice);
          if (left === null || right === null || top === null || bottom === null) continue;
          if (Math.max(left, right) < 0 || Math.min(left, right) > width) continue;

          const x = Math.min(left, right);
          const y = Math.min(top, bottom);
          const rectWidth = Math.abs(right - left);
          const rectHeight = Math.abs(bottom - top);
          const engulfed = latest !== undefined && (
            drawing.structureType === "RBR"
              ? latest.close <= drawing.bottomPrice
              : latest.close >= drawing.topPrice
          );

          ctx.fillStyle = engulfed ? "rgba(239,68,68,0.20)" : "rgba(168,85,247,0.13)";
          ctx.fillRect(x * hr, y * vr, rectWidth * hr, rectHeight * vr);
          ctx.strokeStyle = engulfed ? "#ef4444" : GRADED_RECT_COLOR;
          ctx.lineWidth = drawing.selected ? 2.5 * Math.min(hr, vr) : 1.8 * Math.min(hr, vr);
          ctx.strokeRect(x * hr, y * vr, rectWidth * hr, rectHeight * vr);

          const anchorX = drawingTimeToCoordinate(timeScale, drawing.anchorTime);
          const anchorY = priceCoordinate(this.series, drawing.anchorPrice);
          if (anchorX !== null && anchorY !== null) {
            ctx.beginPath();
            ctx.fillStyle = "#f5d0fe";
            ctx.strokeStyle = GRADED_RECT_COLOR;
            ctx.lineWidth = 2 * Math.min(hr, vr);
            ctx.arc(anchorX * hr, anchorY * vr, 5 * Math.min(hr, vr), 0, Math.PI * 2);
            ctx.fill();
            ctx.stroke();
          }

          if (drawing.correctionStartPrice !== undefined && drawing.correctionStartTime !== undefined && latest) {
            const correctionX = drawingTimeToCoordinate(timeScale, drawing.correctionStartTime);
            const correctionY = priceCoordinate(this.series, drawing.correctionStartPrice);
            const currentX = drawingTimeToCoordinate(timeScale, latest.ts);
            const currentY = priceCoordinate(this.series, latest.close);
            if (correctionX !== null && correctionY !== null && currentX !== null && currentY !== null) {
              ctx.beginPath();
              ctx.strokeStyle = engulfed ? "#ef4444" : "#e879f9";
              ctx.lineWidth = 1.5 * Math.min(hr, vr);
              ctx.setLineDash([5 * hr, 3 * hr]);
              ctx.moveTo(correctionX * hr, correctionY * vr);
              ctx.lineTo(currentX * hr, currentY * vr);
              ctx.stroke();
              ctx.setLineDash([]);
              const height = Math.max(Math.abs(drawing.topPrice - drawing.bottomPrice), Number.EPSILON);
              const percentage = Math.abs(latest.close - drawing.correctionStartPrice) / height * 100;
              const label = engulfed
                ? `ENGULFED (${percentage.toFixed(1)}%)`
                : `${percentage.toFixed(1)}%`;
              ctx.font = `${11 * vr}px ui-monospace, monospace`;
              ctx.textBaseline = "middle";
              ctx.fillStyle = engulfed ? "#fecaca" : "#f5d0fe";
              ctx.fillText(label, currentX * hr + 8 * hr, currentY * vr - 10 * vr);
            }
          }

          if (drawing.selected) {
            for (const handleX of [left, right]) {
              const handleY = (top + bottom) / 2;
              ctx.beginPath();
              ctx.fillStyle = "#18181b";
              ctx.strokeStyle = "#d8b4fe";
              ctx.lineWidth = 2 * Math.min(hr, vr);
              ctx.arc(handleX * hr, handleY * vr, 5 * Math.min(hr, vr), 0, Math.PI * 2);
              ctx.fill();
              ctx.stroke();
            }
          }
        }

        if (this.hoverPivot) {
          const x = drawingTimeToCoordinate(timeScale, this.hoverPivot.ts);
          const y = priceCoordinate(this.series, this.hoverPivot.price);
          if (x !== null && y !== null) {
            ctx.beginPath();
            ctx.strokeStyle = "#facc15";
            ctx.lineWidth = 2 * Math.min(hr, vr);
            ctx.arc(x * hr, y * vr, 8 * Math.min(hr, vr), 0, Math.PI * 2);
            ctx.stroke();
          }
        }
      } finally {
        ctx.restore();
      }
    });
  }
}

class GradedRectanglePaneView implements IPrimitivePaneView {
  constructor(
    private readonly drawings: readonly GradedRectangle[],
    private readonly candles: readonly CandleDto[],
    private readonly hoverPivot: PivotDto | null,
    private readonly chart: IChartApi,
    private readonly series: ISeriesApi<SeriesType>,
  ) {}

  zOrder(): "top" | "normal" | "bottom" { return "top"; }

  renderer(): IPrimitivePaneRenderer {
    return new GradedRectangleRenderer(this.drawings, this.candles, this.hoverPivot, this.chart, this.series);
  }
}

export class GradedRectangleOverlay implements ISeriesPrimitive<Time> {
  private drawings: GradedRectangle[] = [];
  private candles: CandleDto[] = [];
  private hoverPivot: PivotDto | null = null;
  private requestUpdate?: () => void;

  constructor(
    private readonly chart: IChartApi,
    private readonly series: ISeriesApi<SeriesType>,
  ) {}

  attached(param: { requestUpdate: () => void }): void {
    this.requestUpdate = param.requestUpdate;
  }

  detached(): void {
    this.requestUpdate = undefined;
    this.drawings = [];
  }

  setData(drawings: readonly GradedRectangle[], candles: readonly CandleDto[]): void {
    this.drawings = [...drawings];
    this.candles = [...candles];
    this.requestUpdate?.();
  }

  setHoverPivot(pivot: PivotDto | null): void {
    this.hoverPivot = pivot;
    this.requestUpdate?.();
  }

  updateAllViews(): void {}

  paneViews(): readonly IPrimitivePaneView[] {
    return [new GradedRectanglePaneView(this.drawings, this.candles, this.hoverPivot, this.chart, this.series)];
  }
}

export { GRADED_RECT_COLOR };
