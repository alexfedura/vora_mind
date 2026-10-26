import { useEffect } from "react";
import type { IChartApi } from "lightweight-charts";
import type { CandleDto } from "./CandlestickChart";
import type { PivotDto } from "./marketApi";
import type { DrawingAny, GradedRectangle } from "./lineDrawings";
import type { GradedRectConfig } from "./gradedRectStore";
import { drawingTimeToCoordinate } from "./drawingCoordinates";
import { logicalAtCoordinate, timestampAtLogical, type SeriesLike } from "./rectInteraction";

export interface GradedRectContextMenu {
  drawingId: string;
  x: number;
  y: number;
}

interface GradedDrag {
  drawingId: string;
  kind: "body" | "left" | "right";
  original: GradedRectangle;
  startTime: number;
  endTime: number;
  anchorPrice: number;
  resizedWidth?: number;
}

export interface GradedRectToolContext {
  container: HTMLDivElement;
  chart: IChartApi;
  series: object;
  activeToolRef: { current: string };
  drawingsRef: { current: DrawingAny[] };
  candlesRef: { current: CandleDto[] };
  pivotsRef: { current: PivotDto[] };
  configRef: { current: GradedRectConfig };
  setConfig: (config: GradedRectConfig) => void;
  setDrawings: React.Dispatch<React.SetStateAction<DrawingAny[]>>;
  pushHistory: (drawings: DrawingAny[]) => void;
  setHoverPivot: (pivot: PivotDto | null) => void;
  onContextMenu: (menu: GradedRectContextMenu | null) => void;
}

function nearestPivot(
  pivots: readonly PivotDto[],
  chart: IChartApi,
  series: SeriesLike,
  x: number,
  y: number,
): PivotDto | null {
  let closest: PivotDto | null = null;
  let closestDistance = 50;
  for (const pivot of pivots) {
    const pivotX = drawingTimeToCoordinate(chart.timeScale(), pivot.ts);
    const pivotY = series.priceToCoordinate(pivot.price);
    if (pivotX === null || pivotY === null) continue;
    const distance = Math.hypot(pivotX - x, pivotY - y);
    if (distance <= closestDistance) {
      closest = pivot;
      closestDistance = distance;
    }
  }
  return closest;
}

function screenRect(
  drawing: GradedRectangle,
  chart: IChartApi,
  series: SeriesLike,
): { left: number; right: number; top: number; bottom: number } | null {
  const left = drawingTimeToCoordinate(chart.timeScale(), drawing.startTime);
  const right = drawingTimeToCoordinate(chart.timeScale(), drawing.endTime);
  const top = series.priceToCoordinate(drawing.topPrice);
  const bottom = series.priceToCoordinate(drawing.bottomPrice);
  if (left === null || right === null || top === null || bottom === null) return null;
  return {
    left: Math.min(left, right),
    right: Math.max(left, right),
    top: Math.min(top, bottom),
    bottom: Math.max(top, bottom),
  };
}

export function registerGradedRectTool(context: GradedRectToolContext): () => void {
  const { container, chart } = context;
  const series = context.series as SeriesLike;
  let drag: GradedDrag | null = null;

  const point = (event: MouseEvent): { x: number; y: number } => {
    const bounds = container.getBoundingClientRect();
    return { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
  };

  const pivotAt = (x: number, y: number): PivotDto | null =>
    nearestPivot(context.pivotsRef.current, chart, series, x, y);

  const setPan = (enabled: boolean): void => {
    chart.applyOptions({ handleScroll: { pressedMouseMove: enabled } });
  };

  const onMove = (event: MouseEvent): void => {
    const { x, y } = point(event);
    if (drag) {
      const logical = logicalAtCoordinate(chart, x);
      if (logical === null) return;
      const nextTime = timestampAtLogical(logical, context.candlesRef.current);
      const nextPrice = series.coordinateToPrice(y);
      if (nextTime === null || nextPrice === null) return;

      if (drag.kind === "left" || drag.kind === "right") {
        const nextStartTime = drag.kind === "left"
          ? Math.min(nextTime, drag.endTime - 1)
          : drag.startTime;
        const nextEndTime = drag.kind === "right"
          ? Math.max(nextTime, drag.startTime + 1)
          : drag.endTime;
        const startLogical = chart.timeScale().coordinateToLogical(
          drawingTimeToCoordinate(chart.timeScale(), nextStartTime) ?? x,
        );
        const endLogical = chart.timeScale().coordinateToLogical(
          drawingTimeToCoordinate(chart.timeScale(), nextEndTime) ?? x,
        );
        if (startLogical !== null && endLogical !== null) {
          drag.resizedWidth = Math.max(1, Math.abs(Number(endLogical) - Number(startLogical)));
        }
        context.setDrawings((current) => current.map((drawing) =>
          drawing.type === "graded-rectangle" && drawing.id === drag?.drawingId
            ? { ...drawing, startTime: nextStartTime, endTime: nextEndTime, selected: true }
            : { ...drawing, selected: false },
        ));
      } else {
        const deltaTime = nextTime - drag.startTime;
        const deltaPrice = nextPrice - drag.anchorPrice;
        context.setDrawings((current) => current.map((drawing) => {
          if (drawing.type !== "graded-rectangle" || drawing.id !== drag?.drawingId) {
            return { ...drawing, selected: false };
          }
          return {
            ...drawing,
            startTime: drag.original.startTime + deltaTime,
            endTime: drag.original.endTime + deltaTime,
            anchorTime: drag.original.anchorTime + deltaTime,
            anchorPrice: drag.original.anchorPrice + deltaPrice,
            topPrice: drag.original.topPrice + deltaPrice,
            bottomPrice: drag.original.bottomPrice + deltaPrice,
            ...(drag.original.correctionStartPrice === undefined ? {} : {
              correctionStartPrice: drag.original.correctionStartPrice + deltaPrice,
            }),
            ...(drag.original.correctionStartTime === undefined ? {} : {
              correctionStartTime: drag.original.correctionStartTime + deltaTime,
            }),
            selected: true,
          };
        }));
      }
      event.preventDefault();
      event.stopImmediatePropagation();
      return;
    }

    const gradedDrawings = context.drawingsRef.current.filter(
      (drawing): drawing is GradedRectangle => drawing.type === "graded-rectangle" && !drawing.hidden,
    );
    const hovered = gradedDrawings.find((drawing) => {
      const bounds = screenRect(drawing, chart, series);
      return bounds !== null && x >= bounds.left && x <= bounds.right && y >= bounds.top && y <= bounds.bottom;
    });
    if (hovered) {
      const bounds = screenRect(hovered, chart, series);
      const nearHandle = bounds !== null && Math.abs(y - (bounds.top + bounds.bottom) / 2) <= 12
        ? Math.abs(x - bounds.left) <= 10 ? "ew-resize" : Math.abs(x - bounds.right) <= 10 ? "ew-resize" : "move"
        : "move";
      const canvas = container.querySelector("canvas");
      canvas?.style.setProperty("cursor", nearHandle, "important");
    } else if (context.activeToolRef.current === "graded-rectangle") {
      container.querySelector("canvas")?.style.setProperty("cursor", "crosshair", "important");
    }

    const pivot = context.activeToolRef.current === "graded-rectangle" ? pivotAt(x, y) : null;
    context.setHoverPivot(pivot);
  };

  const onDown = (event: MouseEvent): void => {
    if (event.target instanceof Element && event.target.closest(".vora-floating-ui")) return;
    const { x, y } = point(event);
    const drawings = context.drawingsRef.current.filter(
      (drawing): drawing is GradedRectangle => drawing.type === "graded-rectangle" && !drawing.hidden,
    );
    const hitDrawing = [...drawings].reverse().find((drawing) => {
      const bounds = screenRect(drawing, chart, series);
      return bounds !== null && x >= bounds.left && x <= bounds.right && y >= bounds.top && y <= bounds.bottom;
    });

    if (event.button === 2 && hitDrawing) {
      event.preventDefault();
      event.stopImmediatePropagation();
      context.onContextMenu({ drawingId: hitDrawing.id, x: event.clientX, y: event.clientY });
      return;
    }
    if (event.button !== 0) return;
    context.onContextMenu(null);

    if (hitDrawing) {
      const bounds = screenRect(hitDrawing, chart, series);
      if (!bounds) return;
      const nearCenterY = Math.abs(y - (bounds.top + bounds.bottom) / 2) <= 12;
      const handle: GradedDrag["kind"] | null = nearCenterY && Math.abs(x - bounds.left) <= 10
        ? "left"
        : nearCenterY && Math.abs(x - bounds.right) <= 10
          ? "right"
          : x > bounds.left && x < bounds.right && y > bounds.top && y < bounds.bottom
            ? "body"
            : null;
      context.setDrawings((current) => current.map((drawing) => ({
        ...drawing,
        selected: drawing.type === "graded-rectangle" && drawing.id === hitDrawing.id,
      })));
      if (handle && hitDrawing.locked !== true) {
        const logical = logicalAtCoordinate(chart, x);
        const price = series.coordinateToPrice(y);
        if (logical !== null && price !== null) {
          context.pushHistory(context.drawingsRef.current);
          drag = {
            drawingId: hitDrawing.id,
            kind: handle,
            original: hitDrawing,
            startTime: hitDrawing.startTime,
            endTime: hitDrawing.endTime,
            anchorPrice: price,
          };
          setPan(false);
          event.preventDefault();
          event.stopImmediatePropagation();
        }
      } else if (handle === "body" && hitDrawing.locked !== true) {
        const price = series.coordinateToPrice(y);
        if (price !== null) {
          context.pushHistory(context.drawingsRef.current);
          drag = {
            drawingId: hitDrawing.id,
            kind: "body",
            original: hitDrawing,
            startTime: hitDrawing.startTime,
            endTime: hitDrawing.endTime,
            anchorPrice: price,
          };
          setPan(false);
          event.preventDefault();
          event.stopImmediatePropagation();
        }
      }
      return;
    }

    if (context.activeToolRef.current !== "graded-rectangle") return;
    const pivot = pivotAt(x, y);
    if (!pivot) return;

    const selected = [...drawings].reverse().find((drawing) => drawing.selected && drawing.correctionStartPrice === undefined);
    const correctionKind = selected?.structureType === "RBR" ? "L" : "H";
    if (selected && pivot.kind === correctionKind && pivot.ts > selected.endTime) {
      context.pushHistory(context.drawingsRef.current);
      context.setDrawings((current) => current.map((drawing) =>
        drawing.type === "graded-rectangle" && drawing.id === selected.id
          ? { ...drawing, correctionStartPrice: pivot.price, correctionStartTime: pivot.ts, selected: true }
          : { ...drawing, selected: false },
      ));
      event.preventDefault();
      event.stopImmediatePropagation();
      return;
    }

    const logical = logicalAtCoordinate(chart, x);
    if (logical === null) return;
    const config = context.configRef.current;
    const anchorTime = timestampAtLogical(logical, context.candlesRef.current);
    const startTime = timestampAtLogical(logical - config.width / 2, context.candlesRef.current);
    const endTime = timestampAtLogical(logical + config.width / 2, context.candlesRef.current);
    if (anchorTime === null || startTime === null || endTime === null) return;

    const structureType = pivot.kind === "H" ? "RBR" : "DBD";
    const baselineSide = structureType === "DBD" ? "bottom" : config.defaultBaseline;
    const topPrice = structureType === "RBR" ? pivot.price : pivot.price + config.height;
    const bottomPrice = structureType === "RBR" ? pivot.price - config.height : pivot.price;
    const drawing: GradedRectangle = {
      id: `graded-rect-${crypto.randomUUID()}`,
      type: "graded-rectangle",
      structureType,
      anchorPrice: pivot.price,
      anchorTime,
      startTime: Math.min(startTime, endTime),
      endTime: Math.max(startTime, endTime),
      topPrice,
      bottomPrice,
      baselineSide,
      color: "#a855f7",
      selected: true,
      lineWidth: 2,
    };
    context.pushHistory(context.drawingsRef.current);
    context.setDrawings((current) => [
      ...current.map((drawing) => ({ ...drawing, selected: false })),
      drawing,
    ]);
    event.preventDefault();
    event.stopImmediatePropagation();
  };

  const onUp = (): void => {
    if (!drag) return;
    if (drag.resizedWidth !== undefined) {
      context.setConfig({ ...context.configRef.current, width: drag.resizedWidth });
    }
    drag = null;
    setPan(true);
  };

  const onContextMenu = (event: MouseEvent): void => {
    if (event.target instanceof Element && event.target.closest(".vora-floating-ui")) return;
    const { x, y } = point(event);
    const hitDrawing = context.drawingsRef.current
      .filter((drawing): drawing is GradedRectangle => drawing.type === "graded-rectangle" && !drawing.hidden)
      .find((drawing) => {
        const bounds = screenRect(drawing, chart, series);
        return bounds !== null && x >= bounds.left && x <= bounds.right && y >= bounds.top && y <= bounds.bottom;
      });
    if (!hitDrawing) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    context.onContextMenu({ drawingId: hitDrawing.id, x: event.clientX, y: event.clientY });
  };

  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.key === "Escape") context.onContextMenu(null);
  };

  container.addEventListener("mousedown", onDown, true);
  container.addEventListener("contextmenu", onContextMenu, true);
  window.addEventListener("mousemove", onMove, true);
  window.addEventListener("mouseup", onUp, true);
  window.addEventListener("keydown", onKeyDown, true);
  return () => {
    container.removeEventListener("mousedown", onDown, true);
    container.removeEventListener("contextmenu", onContextMenu, true);
    window.removeEventListener("mousemove", onMove, true);
    window.removeEventListener("mouseup", onUp, true);
    window.removeEventListener("keydown", onKeyDown, true);
  };
}

export function useGradedRectTool(context: GradedRectToolContext | null): void {
  useEffect(() => {
    if (!context) return;
    return registerGradedRectTool(context);
  }, [context]);
}
