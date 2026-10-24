import { useEffect } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { IChartApi } from "lightweight-charts";
import type { DrawingAny } from "./lineDrawings";
import type { SeriesLike } from "./rectInteraction";
import type { DrawingCandleSample } from "./drawingTypes";
import { drawingTimeToCoordinate } from "./drawingCoordinates";
import { buildGeometryPath, type ScreenPoint } from "./geometryToolGeometry";
import { buildForecastMeasurementGeometry, isForecastMeasurementTool } from "./forecastMeasurementGeometry";

export interface SelectionBox {
  left: number;
  top: number;
  width: number;
  height: number;
}

interface Bounds {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface MarqueeSelectionCtx {
  container: HTMLDivElement;
  chart: IChartApi;
  series: object;
  drawingsRef: { current: DrawingAny[] };
  candlesRef: { current: DrawingCandleSample[] };
  activeToolRef: { current: string | null };
  rectToolRef: { current: boolean };
  interactionLocked: boolean;
  setDrawings: Dispatch<SetStateAction<DrawingAny[]>>;
  setSelectionBox: Dispatch<SetStateAction<SelectionBox | null>>;
  isOverChrome: (clientX: number, clientY: number) => boolean;
}

function drawingBounds(
  drawing: DrawingAny,
  chart: IChartApi,
  series: SeriesLike,
  candles: readonly DrawingCandleSample[],
  width: number,
  height: number,
): Bounds | null {
  const xAt = (ts: number): number | null =>
    drawingTimeToCoordinate(chart.timeScale(), ts);
  const yAt = (price: number): number | null => series.priceToCoordinate(price);

  if (drawing.type === "vline") {
    const x = xAt(drawing.ts);
    return x === null ? null : { left: x, right: x, top: 0, bottom: height };
  }
  if (drawing.type === "hline") {
    const y = yAt(drawing.price);
    return y === null ? null : { left: 0, right: width, top: y, bottom: y };
  }
  if (drawing.type === "hray") {
    const x = xAt(drawing.ts);
    const y = yAt(drawing.price);
    return x === null || y === null ? null : { left: x, right: width, top: y, bottom: y };
  }
  if (drawing.type === "icon") {
    const x = xAt(drawing.ts);
    const y = yAt(drawing.price);
    if (x === null || y === null) return null;
    const radius = drawing.size / 2;
    const textLines = drawing.text?.split(/\r?\n/) ?? [];
    const textSize = drawing.textSize ?? 14;
    const textWidth = Math.max(0, ...textLines.map((line) => line.length)) * textSize * 0.6;
    const textTop = y + radius + 4;
    const textBottom = textTop + textLines.length * textSize * 1.25;
    return {
      left: Math.min(x - radius, x - textWidth / 2),
      right: Math.max(x + radius, x + textWidth / 2),
      top: y - radius,
      bottom: textLines.length > 0 ? textBottom : y + radius,
    };
  }
  if (drawing.type === "rectangle") {
    const x1 = xAt(drawing.ts_start);
    const x2 = xAt(drawing.ts_end);
    const y1 = yAt(drawing.price_low);
    const y2 = yAt(drawing.price_high);
    if (x1 === null || x2 === null || y1 === null || y2 === null) return null;
    return {
      left: Math.min(x1, x2),
      right: Math.max(x1, x2),
      top: Math.min(y1, y2),
      bottom: Math.max(y1, y2),
    };
  }
  if (drawing.type === "geometry") {
    const project = (anchor: { ts: number; price: number }): ScreenPoint | null => {
      const x = xAt(anchor.ts);
      const y = yAt(anchor.price);
      return x === null || y === null ? null : { x, y };
    };
    const points = buildGeometryPath(drawing.tool, drawing.points, project)?.points ?? [];
    const forecast = isForecastMeasurementTool(drawing.tool)
      ? buildForecastMeasurementGeometry(drawing.tool, drawing.points, candles, drawing.sourceBars ?? [], project, width, chart.timeScale().options().barSpacing ?? 6)
      : null;
    const allPoints: ScreenPoint[] = [
      ...points,
      ...(forecast?.paths.flatMap((shape) => shape.points) ?? []),
      ...(forecast?.candles.flatMap((candle) => [
        { x: candle.x, y: candle.highY },
        { x: candle.x, y: candle.lowY },
      ]) ?? []),
      ...(forecast?.labels.map((label) => ({ x: label.x, y: label.y })) ?? []),
    ];
    if (allPoints.length === 0) return null;
    const xs = allPoints.map((point) => point.x);
    const ys = allPoints.map((point) => point.y);
    const labelPadding = forecast?.labels.reduce((max, label) => Math.max(max, label.text.length * 4), 0) ?? 0;
    return { left: Math.min(...xs), right: Math.max(...xs) + labelPadding, top: Math.min(...ys) - 12, bottom: Math.max(...ys) + 12 };
  }

  if (!("ts1" in drawing && "price1" in drawing && "ts2" in drawing && "price2" in drawing)) {
    return null;
  }
  const anchors = [
    { ts: drawing.ts1, price: drawing.price1 },
    { ts: drawing.ts2, price: drawing.price2 },
    ...("ts3" in drawing && "price3" in drawing && drawing.ts3 !== undefined && drawing.price3 !== undefined
      ? [{ ts: drawing.ts3, price: drawing.price3 }]
      : []),
  ];
  const points: { x: number; y: number }[] = [];
  for (const anchor of anchors) {
    const x = xAt(anchor.ts);
    const y = yAt(anchor.price);
    if (x === null || y === null) return null;
    points.push({ x, y });
  }
  const xs = points.map((point) => point.x);
  const ys = points.map((point) => point.y);
  return {
    left: Math.min(...xs),
    right: Math.max(...xs),
    top: Math.min(...ys),
    bottom: Math.max(...ys),
  };
}

export function useMarqueeSelection(ctx: MarqueeSelectionCtx | null): void {
  useEffect(() => {
    if (!ctx) return;
    const { container, chart, setDrawings, setSelectionBox } = ctx;
    const series = ctx.series as SeriesLike;
    let start: { x: number; y: number } | null = null;
    let current: { x: number; y: number } | null = null;
    let dragging = false;

    const localPoint = (event: MouseEvent): { x: number; y: number } => {
      const rect = container.getBoundingClientRect();
      return { x: event.clientX - rect.left, y: event.clientY - rect.top };
    };
    const updateBox = (): void => {
      if (!start || !current) return;
      setSelectionBox({
        left: Math.min(start.x, current.x),
        top: Math.min(start.y, current.y),
        width: Math.abs(current.x - start.x),
        height: Math.abs(current.y - start.y),
      });
    };
    const onDown = (event: MouseEvent): void => {
      if (event.button !== 0 || !event.ctrlKey || ctx.interactionLocked) return;
      if (ctx.activeToolRef.current !== null || ctx.rectToolRef.current) return;
      if (event.target instanceof Element && event.target.closest(".vora-floating-ui")) return;
      if (!container.contains(event.target as Node) || ctx.isOverChrome(event.clientX, event.clientY)) return;
      start = localPoint(event);
      current = start;
      dragging = false;
      event.preventDefault();
      event.stopImmediatePropagation();
    };
    const onMove = (event: MouseEvent): void => {
      if (!start) return;
      current = localPoint(event);
      dragging ||= Math.hypot(current.x - start.x, current.y - start.y) >= 3;
      if (dragging) {
        updateBox();
      }
      event.preventDefault();
      event.stopImmediatePropagation();
    };
    const onUp = (event: MouseEvent): void => {
      if (!start) return;
      current = localPoint(event);
      if (dragging && current) {
        const selection: Bounds = {
          left: Math.min(start.x, current.x),
          right: Math.max(start.x, current.x),
          top: Math.min(start.y, current.y),
          bottom: Math.max(start.y, current.y),
        };
        const width = chart.timeScale().width();
        const height = container.clientHeight;
        const visibleIds = new Set(ctx.drawingsRef.current.map((drawing) => drawing.id));
        setDrawings((previous) => previous.map((drawing) => {
          const bounds = drawingBounds(drawing, chart, series, ctx.candlesRef.current, width, height);
          const selected = visibleIds.has(drawing.id)
            && bounds !== null
            && bounds.left <= selection.right
            && bounds.right >= selection.left
            && bounds.top <= selection.bottom
            && bounds.bottom >= selection.top;
          return drawing.selected === selected ? drawing : { ...drawing, selected };
        }));
        event.preventDefault();
        event.stopImmediatePropagation();
      }
      start = null;
      current = null;
      dragging = false;
      setSelectionBox(null);
    };
    const onCancel = (): void => {
      start = null;
      current = null;
      dragging = false;
      setSelectionBox(null);
    };

    container.addEventListener("mousedown", onDown, true);
    window.addEventListener("mousemove", onMove, true);
    window.addEventListener("mouseup", onUp, true);
    window.addEventListener("blur", onCancel);
    return () => {
      container.removeEventListener("mousedown", onDown, true);
      window.removeEventListener("mousemove", onMove, true);
      window.removeEventListener("mouseup", onUp, true);
      window.removeEventListener("blur", onCancel);
    };
  }, [ctx]);
}