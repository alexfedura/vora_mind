// v7.4.4 - line tools wiring (vline/hline/hray/trendline).
import type { IChartApi, Time, UTCTimestamp } from "lightweight-charts";
import { hasCandleAtCoordinate, snapPriceToOhlc, snapStrong, type SeriesLike } from "./rectInteraction";
import type { DrawingAny, DrawingFibGann, DrawingGeometry, DrawingIcon, LineToolId } from "./lineDrawings";
import { GEOMETRY_TOOL_LABELS, type DrawingAnchor, type DrawingCandleSample, type DrawingIconSelection, type GeometryToolId } from "./drawingTypes";
import { buildFibGannGeometry, fibGannAnchorCount, isFibGannTool, type ScreenAnchor } from "./fibGannGeometry";
import { buildGeometryPath, geometryAnchorCount, isGeometryToolId } from "./geometryToolGeometry";
import { buildForecastMeasurementGeometry, isForecastMeasurementTool } from "./forecastMeasurementGeometry";
import { drawingTimeToCoordinate } from "./drawingCoordinates";
function nn(v: unknown): number { return v as unknown as number; }
function tt(ts: number): Time { return ts as UTCTimestamp as Time; }
import type { TrendPreview } from "./RectangleOverlay";
import type { LineHover } from "./LineOverlay";
import { ANNOTATION_DRAWING_PRESETS, isAnnotationToolId, readLineColor, readLineWidth } from "./lineDrawings";
import { pitchforkMedianOrigin } from "./pitchforkGeometry";
/**
 * v0.83.14: Defensive filter — the drawings array can contain null
 * elements if localStorage was corrupted or if a race pushed a null.
 * Every consumer must pass through this. `Array.filter(Boolean)` does
 * NOT narrow types for TS (and misses non-object junk), so this helper
 * both prunes at runtime AND narrows the type.
 */
export function pruneNullDrawings<T extends { id: unknown }>(
  arr: readonly (T | null | undefined)[] | null | undefined,
): T[] {
  if (!Array.isArray(arr)) return [];
  return arr.filter(
    (l): l is T =>
      l != null &&
      typeof l === "object" &&
      "id" in (l as Record<string, unknown>) &&
      (l as { id: unknown }).id != null,
  );
}
export interface LineCandle { ts: number; open: number; high: number; low: number; close: number; volume: number; }
export interface LineToolCtx {
  container: HTMLDivElement; chart: IChartApi; series: object;
  drawingsRef: { current: DrawingAny[] };
  setDrawings: React.Dispatch<React.SetStateAction<DrawingAny[]>>;
  activeToolRef: { current: LineToolId | null };
  magnetRef: { current: string }; ctrlHeldRef: { current: boolean };
  iconSelectionRef: { current: DrawingIconSelection | null };
  candlesRef: { current: LineCandle[] };
  pushHistorySnapshot?: (snap: DrawingAny[]) => void;
  onFinalizeLine?: () => void;
  trendPreviewRef: { current: TrendPreview | null };
  setTrendPreview: (p: TrendPreview | null) => void;
  setLineHover: (h: LineHover | null) => void;
  setSnap: (s: { t: number; p: number } | null) => void;
  onBlankContextMenu?: () => boolean;
}
export function snapActiveForLines(magnet: string, ctrlHeld: boolean): boolean {
  return ctrlHeld ? magnet === "off" : magnet !== "off";
}
function pointSegmentDistance(point: ScreenAnchor, start: ScreenAnchor, end: ScreenAnchor): number {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const lengthSquared = dx * dx + dy * dy;
  const ratio = lengthSquared === 0 ? 0 : Math.max(0, Math.min(1, ((point.x - start.x) * dx + (point.y - start.y) * dy) / lengthSquared));
  return Math.hypot(point.x - (start.x + ratio * dx), point.y - (start.y + ratio * dy));
}

function pointInPolygon(point: ScreenAnchor, polygon: readonly ScreenAnchor[]): boolean {
  let inside = false;
  for (let current = 0, previous = polygon.length - 1; current < polygon.length; previous = current, current += 1) {
    const currentPoint = polygon[current];
    const previousPoint = polygon[previous];
    if (!currentPoint || !previousPoint) continue;
    const intersects = (currentPoint.y > point.y) !== (previousPoint.y > point.y)
      && point.x < ((previousPoint.x - currentPoint.x) * (point.y - currentPoint.y)) / (previousPoint.y - currentPoint.y) + currentPoint.x;
    if (intersects) inside = !inside;
  }
  return inside;
}

export function registerLineTool(c: LineToolCtx): () => void {
  const { container, chart, series } = c;
  const srs = series as unknown as SeriesLike;
  const tsc = chart.timeScale();
  const drawingX = (time: Time): number | null => drawingTimeToCoordinate(tsc, Number(time));
  let drag: { id: string; handle: string | null; grabT: number; grabP: number; orig: DrawingAny } | null = null;
  let geometryDrag: { tool: GeometryToolId; points: DrawingAnchor[]; last: ScreenAnchor } | null = null;
  const rawAt = (px: number, py: number): { t: number; p: number } | null => {
    const t = tsc.coordinateToTime(px);
    const p = srs.coordinateToPrice(py);
    if (t == null || p == null) return null;
    return { t: t as number, p };
  };
  const snapAt = (px: number, py: number): { t: number; p: number } | null => {
    const raw = rawAt(px, py);
    if (!raw) return null;
    if (!snapActiveForLines(c.magnetRef.current, c.ctrlHeldRef.current)) return raw;
    if (!hasCandleAtCoordinate(chart, c.candlesRef.current, px)) return raw;
    const strongMagnet = c.ctrlHeldRef.current
      ? c.magnetRef.current === "off"
      : c.magnetRef.current === "strong";
    if (strongMagnet) {
      const snapped = snapPriceToOhlc(raw.t, raw.p, c.candlesRef.current);
      return { t: snapped.ts, p: snapped.price };
    }
    return snapStrong(chart, srs, c.candlesRef.current, px, py) ?? raw;
  };
  const snapMarkerAt = (px: number, point: { t: number; p: number }): { t: number; p: number } | null =>
    snapActiveForLines(c.magnetRef.current, c.ctrlHeldRef.current)
      && hasCandleAtCoordinate(chart, c.candlesRef.current, px) ? point : null;
  const pt = (e: MouseEvent): { px: number; py: number } => {
    const r = container.getBoundingClientRect();
    return { px: e.clientX - r.left, py: e.clientY - r.top };
  };
  const setPan = (en: boolean): void => {
    try { chart.applyOptions({ handleScroll: { pressedMouseMove: en } } as unknown as Parameters<typeof chart.applyOptions>[0]); } catch { /* ignore */ }
  };
  const cancelPreview = (): void => {
    geometryDrag = null;
    c.setTrendPreview(null);
    drag = null;
    c.setSnap(null);
    setPan(true);
  };
  const uid = (pfx: string): string => `${pfx}-${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
  const makeGeometryDrawing = (tool: GeometryToolId, points: DrawingAnchor[]): DrawingGeometry => {
    const sourceBars: DrawingCandleSample[] | undefined = tool === "bars-pattern" && points.length >= 2
      ? c.candlesRef.current.filter((candle) => candle.ts >= Math.min(points[0]?.ts ?? 0, points[1]?.ts ?? 0) && candle.ts <= Math.max(points[0]?.ts ?? 0, points[1]?.ts ?? 0)).map(({ ts, open, high, low, close, volume }) => ({ ts, open, high, low, close, volume }))
      : undefined;
    return {
      id: uid("geometry"),
      type: "geometry",
      tool,
      name: GEOMETRY_TOOL_LABELS[tool],
      points,
      ...(sourceBars && sourceBars.length > 0 ? { sourceBars } : {}),
      color: readLineColor(),
      lineWidth: readLineWidth("vora.drawings.trendline_width", 2),
      selected: true,
    };
  };
  const midP = (): number => srs.coordinateToPrice((container.clientHeight || 400) / 2) ?? 0;
  const midT = (): number | null => {
    try {
      const r = tsc.getVisibleRange();
      if (r && typeof r.from === "number" && typeof r.to === "number") return (r.from + r.to) / 2;
    } catch { /* ignore */ }
    return null;
  };
  const screenAnchors = (drawing: DrawingFibGann): ScreenAnchor[] | null => {
    const values = [
      { ts: drawing.ts1, price: drawing.price1 },
      { ts: drawing.ts2, price: drawing.price2 },
      ...(drawing.ts3 !== undefined && drawing.price3 !== undefined ? [{ ts: drawing.ts3, price: drawing.price3 }] : []),
    ];
    const points: ScreenAnchor[] = [];
    for (const value of values) {
      const x = drawingX(tt(value.ts));
      const y = srs.priceToCoordinate(value.price);
      if (x == null || y == null) return null;
      points.push({ x: nn(x), y: nn(y) });
    }
    return points;
  };
  const cursorForLineHandle = (drawing: DrawingAny, handle: string): string => {
    if (drawing.type === "vline") return "ew-resize";
    if (drawing.type === "hline") return "ns-resize";
    if (drawing.type === "geometry" && (drawing.tool === "rr_long" || drawing.tool === "rr_short")) {
      if (handle === "p2-price") return "ns-resize";
      if (handle === "p2-time") return "ew-resize";
    }
    let anchors: { id: string; ts: number; price: number }[] = [];
    if (drawing.type === "geometry") {
      anchors = drawing.points.map((point, index) => ({ id: `p${index}`, ts: point.ts, price: point.price }));
    } else if (drawing.type === "parallel-channel" || drawing.type === "pitchfork") {
      anchors = [
        { id: "p1", ts: drawing.ts1, price: drawing.price1 },
        { id: "p2", ts: drawing.ts2, price: drawing.price2 },
        { id: "p3", ts: drawing.ts3, price: drawing.price3 },
      ];
    } else if (drawing.type === "fib-gann") {
      anchors = [
        { id: "p1", ts: drawing.ts1, price: drawing.price1 },
        { id: "p2", ts: drawing.ts2, price: drawing.price2 },
        ...(drawing.ts3 !== undefined && drawing.price3 !== undefined ? [{ id: "p3", ts: drawing.ts3, price: drawing.price3 }] : []),
      ];
    } else if (drawing.type === "trendline" || drawing.type === "ray" || drawing.type === "info-line" || drawing.type === "extended-line" || drawing.type === "trend-angle") {
      anchors = [
        { id: "p1", ts: drawing.ts1, price: drawing.price1 },
        { id: "p2", ts: drawing.ts2, price: drawing.price2 },
      ];
    }
    const points = anchors.flatMap((anchor) => {
      const x = drawingX(tt(anchor.ts));
      const y = srs.priceToCoordinate(anchor.price);
      return x == null || y == null ? [] : [{ id: anchor.id, x: nn(x), y: nn(y) }];
    });
    const target = points.find((point) => point.id === handle);
    if (!target || points.length < 2) return "move";
    const left = Math.min(...points.map((point) => point.x));
    const right = Math.max(...points.map((point) => point.x));
    const top = Math.min(...points.map((point) => point.y));
    const bottom = Math.max(...points.map((point) => point.y));
    const xEdge = right - left > 4 && (target.x <= left + 4 || target.x >= right - 4);
    const yEdge = bottom - top > 4 && (target.y <= top + 4 || target.y >= bottom - 4);
    if (xEdge && yEdge) {
      return (target.x - (left + right) / 2) * (target.y - (top + bottom) / 2) >= 0
        ? "nwse-resize"
        : "nesw-resize";
    }
    if (xEdge) return "ew-resize";
    if (yEdge) return "ns-resize";
    return "move";
  };
  const hit = (px: number, py: number): { d: DrawingAny; handle: string | null; segmentIndex?: number } | null => {
    const list = pruneNullDrawings(c.drawingsRef.current);
    for (let i = list.length - 1; i >= 0; i--) {
      const d = list[i];
      if (!d) continue;
      if (d.type === "rectangle" || d.type === "icon") continue;
      let hp: { id: string; ts: number; price: number }[] = [];
      if (d.type === "geometry" && (d.tool === "rr_long" || d.tool === "rr_short")) {
        const forecast = buildForecastMeasurementGeometry(d.tool, d.points, c.candlesRef.current, d.sourceBars ?? [], (anchor) => {
          const x = drawingX(tt(anchor.ts));
          const y = srs.priceToCoordinate(anchor.price);
          return x === null || y === null ? null : { x: nn(x), y: nn(y) };
        }, container.clientWidth, tsc.options().barSpacing ?? 6);
        const risk = forecast?.paths[0]?.points;
        const reward = forecast?.paths[1]?.points;
        if (risk?.[0] && risk[1] && risk[3] && reward?.[3]) {
          const handles = [
            { id: "p2-price", x: reward[3].x, y: reward[3].y },
            { id: "p0", x: risk[0].x, y: risk[0].y },
            { id: "p2-time", x: risk[1].x, y: risk[1].y },
            { id: "p1", x: risk[3].x, y: risk[3].y },
          ];
          const matched = handles.find((handle) => Math.hypot(handle.x - px, handle.y - py) <= 14);
          if (matched) return { d, handle: matched.id };
        }
        hp = [];
      } else if (d.type === "geometry") hp = d.points.map((point, index) => ({ id: `p${index}`, ts: point.ts, price: point.price }));
      else if (d.type === "vline") hp = [{ id: "move", ts: (d as { ts: number }).ts, price: midP() }];
      else if (d.type === "hline") hp = [{ id: "move", ts: midT() ?? 0, price: (d as { price: number }).price }];
      else if (d.type === "hray") hp = [{ id: "move", ts: (d as { ts: number }).ts, price: (d as { price: number }).price }];
      else if (d.type === "crossline") hp = [{ id: "move", ts: d.ts, price: d.price }];
      else if (d.type === "parallel-channel") hp = [
        { id: "p1", ts: d.ts1, price: d.price1 },
        { id: "p2", ts: d.ts2, price: d.price2 },
        { id: "p3", ts: d.ts3, price: d.price3 },
      ];
      else if (d.type === "pitchfork") hp = [
        { id: "p1", ts: d.ts1, price: d.price1 },
        { id: "p2", ts: d.ts2, price: d.price2 },
        { id: "p3", ts: d.ts3, price: d.price3 },
      ];
      else if (d.type === "fib-gann") hp = [
        { id: "p1", ts: d.ts1, price: d.price1 },
        { id: "p2", ts: d.ts2, price: d.price2 },
        ...(d.ts3 !== undefined && d.price3 !== undefined ? [{ id: "p3", ts: d.ts3, price: d.price3 }] : []),
      ];
      else if (d.type === "trendline" || d.type === "ray" || d.type === "info-line" || d.type === "extended-line" || d.type === "trend-angle") {
        const t = d as { ts1: number; price1: number; ts2: number; price2: number };
        hp = [{ id: "p1", ts: t.ts1, price: t.price1 }, { id: "p2", ts: t.ts2, price: t.price2 }];
      }
      for (const q of hp) {
        const sx = drawingX(tt(q.ts));
        const sy = srs.priceToCoordinate(q.price);
        if (sx == null || sy == null) continue;
        if (Math.hypot(nn(sx) - px, nn(sy) - py) <= 14) return { d, handle: q.id };
      }
    }
    const raw = rawAt(px, py);
    if (!raw) return null;
    for (let i = list.length - 1; i >= 0; i--) {
      const d = list[i];
      if (!d) continue;
      if (d.type === "rectangle") continue;
      if (d.type === "geometry") {
        if (isForecastMeasurementTool(d.tool)) {
          const forecast = buildForecastMeasurementGeometry(d.tool, d.points, c.candlesRef.current, d.sourceBars ?? [], (anchor) => {
            const x = drawingX(tt(anchor.ts));
            const y = srs.priceToCoordinate(anchor.price);
            return x === null || y === null ? null : { x: nn(x), y: nn(y) };
          }, container.clientWidth, tsc.options().barSpacing ?? 6);
          if (forecast) {
            for (const shape of forecast.paths) {
              for (let index = 1; index < shape.points.length; index += 1) {
                const start = shape.points[index - 1];
                const end = shape.points[index];
                if (start && end && pointSegmentDistance({ x: px, y: py }, start, end) <= 7) return { d, handle: null };
              }
              if (shape.closed && pointInPolygon({ x: px, y: py }, shape.points)) return { d, handle: null };
            }
            if (forecast.candles.some((candle) => Math.abs(px - candle.x) <= 7 && py >= candle.highY - 4 && py <= candle.lowY + 4)) return { d, handle: null };
          }
        }
        const path = buildGeometryPath(d.tool, d.points, (anchor) => {
          const x = drawingX(tt(anchor.ts));
          const y = srs.priceToCoordinate(anchor.price);
          return x === null || y === null ? null : { x: nn(x), y: nn(y) };
        });
        if (!path) continue;
        if (path.points.length === 1 && path.points[0] && Math.hypot(path.points[0].x - px, path.points[0].y - py) <= 10) return { d, handle: null };
        const hitTolerance = d.tool === "path" ? Math.max(18, d.lineWidth / 2 + 12) : 7;
        for (let index = 1; index < path.points.length; index += 1) {
          const start = path.points[index - 1];
          const end = path.points[index];
          if (start && end && pointSegmentDistance({ x: px, y: py }, start, end) <= hitTolerance) {
            return { d, handle: null, ...(d.tool === "path" ? { segmentIndex: index - 1 } : {}) };
          }
        }
      } else if (d.type === "icon") {
        const x = drawingX(tt(d.ts));
        const y = srs.priceToCoordinate(d.price);
        if (x == null || y == null) continue;
        const radius = Math.max(16, d.size * 0.65);
        const textLines = d.text?.split(/\r?\n/) ?? [];
        const textSize = d.textSize ?? 14;
        const textWidth = Math.max(0, ...textLines.map((line) => line.length)) * textSize * 0.6;
        const textTop = nn(y) + Math.max(12, Math.min(72, d.size)) / 2 + 4;
        const textBottom = textTop + textLines.length * textSize * 1.25;
        const hitSymbol = Math.abs(nn(x) - px) <= radius && Math.abs(nn(y) - py) <= radius;
        const hitText = textLines.length > 0 && Math.abs(nn(x) - px) <= textWidth / 2 + 4 && py >= textTop && py <= textBottom;
        if (hitSymbol || hitText) return { d, handle: null };
      } else if (d.type === "vline") {
        const x = drawingX(tt((d as { ts: number }).ts));
        if (x == null) continue;
        if (Math.abs(nn(x) - px) <= 6) return { d, handle: null };
      } else if (d.type === "hline") {
        const y = srs.priceToCoordinate((d as { price: number }).price);
        if (y == null) continue;
        if (Math.abs(nn(y) - py) <= 6) return { d, handle: null };
      } else if (d.type === "hray") {
        const r = d as { ts: number; price: number };
        const x = drawingX(tt(r.ts));
        const y = srs.priceToCoordinate(r.price);
        if (x == null || y == null) continue;
        if (px >= nn(x) - 6 && Math.abs(nn(y) - py) <= 6) return { d, handle: null };
      } else if (d.type === "crossline") {
        const x = drawingX(tt(d.ts));
        const y = srs.priceToCoordinate(d.price);
        if (x != null && Math.abs(nn(x) - px) <= 6) return { d, handle: null };
        if (y != null && Math.abs(nn(y) - py) <= 6) return { d, handle: null };
      } else if (d.type === "parallel-channel") {
        const duration = d.ts2 - d.ts1;
        const baseAtThird = duration === 0 ? d.price1 : d.price1 + ((d.ts3 - d.ts1) / duration) * (d.price2 - d.price1);
        const offset = d.price3 - baseAtThird;
        const x1 = drawingX(tt(d.ts1));
        const x2 = drawingX(tt(d.ts2));
        const y1 = srs.priceToCoordinate(d.price1);
        const y2 = srs.priceToCoordinate(d.price2);
        const y3 = srs.priceToCoordinate(d.price1 + offset);
        const y4 = srs.priceToCoordinate(d.price2 + offset);
        if (x1 == null || x2 == null || y1 == null || y2 == null || y3 == null || y4 == null) continue;
        const segments: [number, number, number, number][] = [
          [nn(x1), nn(y1), nn(x2), nn(y2)],
          [nn(x1), nn(y3), nn(x2), nn(y4)],
          [nn(x1), nn(y1), nn(x1), nn(y3)],
          [nn(x2), nn(y2), nn(x2), nn(y4)],
        ];
        for (const [ax, ay, bx, by] of segments) {
          const dx = bx - ax;
          const dy = by - ay;
          const lengthSquared = dx * dx + dy * dy;
          const ratio = lengthSquared === 0 ? 0 : Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / lengthSquared));
          if (Math.hypot(px - (ax + ratio * dx), py - (ay + ratio * dy)) <= 6) return { d, handle: null };
        }
      } else if (d.type === "pitchfork") {
        const origin = pitchforkMedianOrigin({ ts: d.ts1, price: d.price1 }, { ts: d.ts2, price: d.price2 }, d.variant);
        const x1 = drawingX(tt(origin.ts));
        const x2 = drawingX(tt(d.ts2));
        const x3 = drawingX(tt(d.ts3));
        const y1 = srs.priceToCoordinate(origin.price);
        const y2 = srs.priceToCoordinate(d.price2);
        const y3 = srs.priceToCoordinate(d.price3);
        if (x1 == null || x2 == null || x3 == null || y1 == null || y2 == null || y3 == null) continue;
        const midX = (nn(x2) + nn(x3)) / 2;
        const midY = (nn(y2) + nn(y3)) / 2;
        const dx = midX - nn(x1);
        const dy = midY - nn(y1);
        const starts = [[nn(x1), nn(y1)], [nn(x2), nn(y2)], [nn(x3), nn(y3)]];
        for (const [ax, ay] of starts) {
          const lengthSquared = dx * dx + dy * dy;
          const ratio = lengthSquared === 0 ? 0 : Math.max(0, ((px - ax) * dx + (py - ay) * dy) / lengthSquared);
          if (Math.hypot(px - (ax + ratio * dx), py - (ay + ratio * dy)) <= 6) return { d, handle: null };
        }
      } else if (d.type === "fib-gann") {
        const anchors = screenAnchors(d);
        if (!anchors || anchors.length < fibGannAnchorCount(d.tool)) continue;
        const geometry = buildFibGannGeometry(d.tool, anchors, container.clientWidth, container.clientHeight);
        for (const path of geometry.paths) {
          for (let index = 1; index < path.length; index += 1) {
            const start = path[index - 1];
            const end = path[index];
            if (start && end && pointSegmentDistance({ x: px, y: py }, start, end) <= 6) return { d, handle: null };
          }
        }
      } else if (d.type === "trendline" || d.type === "ray" || d.type === "info-line" || d.type === "extended-line" || d.type === "trend-angle") {
        const t = d as { ts1: number; price1: number; ts2: number; price2: number };
        const x1 = drawingX(tt(t.ts1));
        const x2 = drawingX(tt(t.ts2));
        const y1 = srs.priceToCoordinate(t.price1);
        const y2 = srs.priceToCoordinate(t.price2);
        if (x1 == null || x2 == null || y1 == null || y2 == null) continue;
        const ax = nn(x1); const ay = nn(y1); const bx = nn(x2); const by = nn(y2);
        const dx = bx - ax; const dy = by - ay;
        const l2 = dx * dx + dy * dy;
        let dist: number;
        if (l2 === 0) dist = Math.hypot(px - ax, py - ay);
        else {
          const projected = ((px - ax) * dx + (py - ay) * dy) / l2;
          const k2 = d.type === "ray" ? Math.max(0, projected) : d.type === "extended-line" ? projected : Math.max(0, Math.min(1, projected));
          dist = Math.hypot(px - (ax + k2 * dx), py - (ay + k2 * dy));
        }
        if (dist <= 6) return { d, handle: null };
      }
    }
    return null;
  };
  const place = (e: MouseEvent): void => {
    if (e.target instanceof Element && e.target.closest(".vora-floating-ui")) return;
    if (e.button !== 0) return;
    const tool = c.activeToolRef.current;
    const { px, py } = pt(e);
    const prev = c.trendPreviewRef.current;
    if (prev && prev.active && tool === (prev.tool ?? "trendline")) {
      const dp = snapAt(px, py);
      if (!dp) return;
      if (prev.tool && isGeometryToolId(prev.tool)) {
        const fixed = (prev.geometryPoints ?? []).slice(0, prev.geometryFixedCount ?? 1);
        const anchor: DrawingAnchor = { ts: dp.t, price: dp.p };
        const nextPoints = [...fixed, anchor];
        if (prev.tool === "path" || prev.tool === "polyline" || prev.tool === "ghost-feed") {
          if (e.detail >= 2 && fixed.length >= 2) {
            const seed = makeGeometryDrawing(prev.tool, fixed);
            c.pushHistorySnapshot?.(pruneNullDrawings(c.drawingsRef.current));
            c.setDrawings((current) => [...pruneNullDrawings(current).map((drawing) => ({ ...drawing, selected: false })), seed]);
            c.setTrendPreview(null); setPan(true); c.onFinalizeLine?.();
          } else {
            c.setTrendPreview({ ...prev, geometryPoints: nextPoints, geometryFixedCount: nextPoints.length });
          }
          e.preventDefault(); e.stopPropagation();
          return;
        }
        if (nextPoints.length >= geometryAnchorCount(prev.tool)) {
          const seed = makeGeometryDrawing(prev.tool, nextPoints);
          c.pushHistorySnapshot?.(pruneNullDrawings(c.drawingsRef.current));
          c.setDrawings((current) => [...pruneNullDrawings(current).map((drawing) => ({ ...drawing, selected: false })), seed]);
          c.setTrendPreview(null); c.setSnap(null); setPan(true); c.onFinalizeLine?.();
        } else {
          c.setTrendPreview({ ...prev, geometryPoints: nextPoints, geometryFixedCount: nextPoints.length });
        }
        e.preventDefault(); e.stopPropagation();
        return;
      }
      if (prev.tool === "parallel-channel" && prev.stage !== "offset") {
        c.setTrendPreview({ ...prev, ts2: dp.t, price2: dp.p, ts3: dp.t, price3: dp.p, stage: "offset" });
        e.preventDefault(); e.stopPropagation();
        return;
      }
      if ((prev.tool === "pitchfork" || prev.tool === "schiff-pitchfork" || prev.tool === "modified-schiff-pitchfork") && prev.stage !== "offset") {
        c.setTrendPreview({ ...prev, ts2: dp.t, price2: dp.p, ts3: dp.t, price3: dp.p, stage: "offset" });
        e.preventDefault(); e.stopPropagation();
        return;
      }
      if (prev.tool && isFibGannTool(prev.tool)) {
        const needsThreeAnchors = fibGannAnchorCount(prev.tool) === 3;
        if (needsThreeAnchors && prev.stage !== "offset") {
          c.setTrendPreview({ ...prev, ts2: dp.t, price2: dp.p, ts3: dp.t, price3: dp.p, stage: "offset" });
          e.preventDefault(); e.stopPropagation();
          return;
        }
        const seed: DrawingFibGann = {
          id: uid(prev.tool),
          type: "fib-gann",
          tool: prev.tool,
          ts1: prev.ts1,
          price1: prev.price1,
          ts2: needsThreeAnchors ? prev.ts2 : dp.t,
          price2: needsThreeAnchors ? prev.price2 : dp.p,
          ...(needsThreeAnchors ? { ts3: dp.t, price3: dp.p } : {}),
          color: readLineColor(),
          lineWidth: readLineWidth("vora.drawings.trendline_width", 1),
          selected: true,
        };
        c.pushHistorySnapshot?.(pruneNullDrawings(c.drawingsRef.current));
        c.setDrawings((current) => [...pruneNullDrawings(current).map((drawing) => ({ ...drawing, selected: false })), seed]);
        c.setTrendPreview(null);
        drag = null;
        c.setSnap(snapMarkerAt(px, { t: dp.t, p: dp.p }));
        setPan(true); c.onFinalizeLine?.();
        e.preventDefault(); e.stopPropagation();
        return;
      }
      const seed = prev.tool === "ray"
        ? { id: uid("ray"), type: "ray" as const, ts1: prev.ts1, price1: prev.price1, ts2: dp.t, price2: dp.p, color: readLineColor(), lineWidth: readLineWidth("vora.drawings.trendline_width", 2), selected: true }
        : prev.tool === "extended-line"
          ? { id: uid("extended-line"), type: "extended-line" as const, ts1: prev.ts1, price1: prev.price1, ts2: dp.t, price2: dp.p, color: readLineColor(), lineWidth: readLineWidth("vora.drawings.trendline_width", 2), selected: true }
          : prev.tool === "trend-angle"
            ? { id: uid("trend-angle"), type: "trend-angle" as const, ts1: prev.ts1, price1: prev.price1, ts2: dp.t, price2: dp.p, color: readLineColor(), lineWidth: readLineWidth("vora.drawings.trendline_width", 2), selected: true }
            : prev.tool === "parallel-channel"
              ? { id: uid("parallel-channel"), type: "parallel-channel" as const, ts1: prev.ts1, price1: prev.price1, ts2: prev.ts2, price2: prev.price2, ts3: prev.ts3 ?? dp.t, price3: prev.price3 ?? dp.p, color: readLineColor(), lineWidth: readLineWidth("vora.drawings.trendline_width", 2), selected: true }
              : prev.tool === "pitchfork"
                ? { id: uid("pitchfork"), type: "pitchfork" as const, ts1: prev.ts1, price1: prev.price1, ts2: prev.ts2, price2: prev.price2, ts3: prev.ts3 ?? dp.t, price3: prev.price3 ?? dp.p, color: readLineColor(), lineWidth: readLineWidth("vora.drawings.trendline_width", 2), selected: true }
                : prev.tool === "schiff-pitchfork"
                  ? { id: uid("schiff-pitchfork"), type: "pitchfork" as const, variant: "schiff" as const, name: "Schiff pitchfork", ts1: prev.ts1, price1: prev.price1, ts2: prev.ts2, price2: prev.price2, ts3: prev.ts3 ?? dp.t, price3: prev.price3 ?? dp.p, color: readLineColor(), lineWidth: readLineWidth("vora.drawings.trendline_width", 2), selected: true }
                  : prev.tool === "modified-schiff-pitchfork"
                    ? { id: uid("modified-schiff-pitchfork"), type: "pitchfork" as const, variant: "modified-schiff" as const, name: "Modified Schiff pitchfork", ts1: prev.ts1, price1: prev.price1, ts2: prev.ts2, price2: prev.price2, ts3: prev.ts3 ?? dp.t, price3: prev.price3 ?? dp.p, color: readLineColor(), lineWidth: readLineWidth("vora.drawings.trendline_width", 2), selected: true }
          : prev.tool === "info-line"
          ? { id: uid("info-line"), type: "info-line" as const, ts1: prev.ts1, price1: prev.price1, ts2: dp.t, price2: dp.p, color: readLineColor(), lineWidth: readLineWidth("vora.drawings.trendline_width", 2), selected: true }
          : { id: uid("trend"), type: "trendline" as const, ts1: prev.ts1, price1: prev.price1, ts2: dp.t, price2: dp.p, color: readLineColor(), lineWidth: readLineWidth("vora.drawings.trendline_width", 2), selected: true };
      c.pushHistorySnapshot?.(pruneNullDrawings(c.drawingsRef.current));
      c.setDrawings((q) => [...pruneNullDrawings(q).map((z) => ({ ...z, selected: false })), seed]);
      c.setTrendPreview(null);
      drag = null;
      c.setSnap(snapMarkerAt(px, { t: dp.t, p: dp.p }));
      setPan(true); c.onFinalizeLine?.();
      e.preventDefault(); e.stopPropagation();
      return;
    }
    const hh = hit(px, py);
    if (hh) {
      if (e.ctrlKey && hh.d.type === "geometry" && hh.d.tool === "path" && hh.segmentIndex !== undefined && hh.handle === null) {
        const segmentIndex = hh.segmentIndex;
        c.setDrawings((current) => pruneNullDrawings(current).map((drawing) => {
          if (drawing.id !== hh.d.id) return { ...drawing, selected: false };
          if (drawing.type !== "geometry" || drawing.tool !== "path") return drawing;
          const currentSelection = drawing.selectedSegments?.filter(
            (index) => Number.isInteger(index) && index >= 0 && index < drawing.points.length - 1,
          ) ?? [];
          const nextSelection = currentSelection.length === 0
            ? [segmentIndex]
            : currentSelection.includes(segmentIndex)
              ? currentSelection.filter((index) => index !== segmentIndex)
              : [...currentSelection, segmentIndex];
          return {
            ...drawing,
            selected: true,
            selectedSegments: nextSelection.length > 0 ? nextSelection.sort((left, right) => left - right) : undefined,
          };
        }));
        e.preventDefault(); e.stopPropagation();
        return;
      }
      // v0.82.0 — locked: select only, no drag.
      if ((hh.d as unknown as { locked?: boolean }).locked === true) {
        c.setDrawings((q) => pruneNullDrawings(q).map((z) => {
          if (z.id !== hh.d.id) return { ...z, selected: false };
          return z.type === "geometry" && z.tool === "path"
            ? { ...z, selected: true, selectedSegments: undefined }
            : { ...z, selected: true };
        }));
        e.preventDefault(); e.stopPropagation();
        return;
      }
      c.pushHistorySnapshot?.(pruneNullDrawings(c.drawingsRef.current));
      c.setDrawings((q) => pruneNullDrawings(q).map((z) => {
        if (z.id !== hh.d.id) return { ...z, selected: false };
        return z.type === "geometry" && z.tool === "path"
          ? { ...z, selected: true, selectedSegments: undefined }
          : { ...z, selected: true };
      }));
      const raw = rawAt(px, py) ?? { t: 0, p: 0 };
      drag = hh.handle
        ? { id: hh.d.id, handle: hh.handle, grabT: raw.t, grabP: raw.p, orig: { ...hh.d } as DrawingAny }
        : { id: hh.d.id, handle: null, grabT: raw.t, grabP: raw.p, orig: { ...hh.d } as DrawingAny };
      setPan(false);
      e.preventDefault(); e.stopPropagation();
      return;
    }
    if (!tool) {
      c.setDrawings((q) => pruneNullDrawings(q).some((z) => z.selected) ? pruneNullDrawings(q).map((z) => ({ ...z, selected: false })) : pruneNullDrawings(q));
      return;
    }
    const dp = snapAt(px, py);
    if (!dp) return;
    if (tool === "icon" || isAnnotationToolId(tool)) {
      const preset = isAnnotationToolId(tool) ? ANNOTATION_DRAWING_PRESETS[tool] : null;
      const choice: DrawingIconSelection | null = preset
        ? { category: "icon", symbol: preset.symbol }
        : c.iconSelectionRef.current;
      if (!choice) return;
      const seed: DrawingIcon = {
        id: uid("icon"),
        type: "icon",
        category: choice.category,
        symbol: choice.symbol,
        name: preset?.label,
        ts: dp.t,
        price: dp.p,
        size: 28,
        color: readLineColor(),
        lineWidth: 1,
        selected: true,
      };
      c.pushHistorySnapshot?.(pruneNullDrawings(c.drawingsRef.current));
      c.setDrawings((current) => [...pruneNullDrawings(current).map((drawing) => ({ ...drawing, selected: false })), seed]);
      c.setSnap(null);
      setPan(true);
      c.onFinalizeLine?.();
      e.preventDefault();
      e.stopPropagation();
      return;
    }
    if (isGeometryToolId(tool)) {
      const required = geometryAnchorCount(tool);
      if (required === 1) {
        const seed = makeGeometryDrawing(tool, [{ ts: dp.t, price: dp.p }]);
        c.pushHistorySnapshot?.(pruneNullDrawings(c.drawingsRef.current));
        c.setDrawings((current) => [...pruneNullDrawings(current).map((drawing) => ({ ...drawing, selected: false })), seed]);
        c.setSnap(null); setPan(true); c.onFinalizeLine?.();
      } else if (tool === "brush" || tool === "highlighter") {
        const anchor = { ts: dp.t, price: dp.p };
        geometryDrag = { tool, points: [anchor], last: { x: px, y: py } };
        c.setTrendPreview({ ts1: dp.t, price1: dp.p, ts2: dp.t, price2: dp.p, tool, geometryPoints: [anchor], active: true });
        setPan(false);
      } else {
        c.setTrendPreview({ ts1: dp.t, price1: dp.p, ts2: dp.t, price2: dp.p, tool, geometryPoints: [{ ts: dp.t, price: dp.p }], geometryFixedCount: 1, active: true });
        setPan(false);
      }
      e.preventDefault(); e.stopPropagation();
      return;
    }
    const color = readLineColor();
    const snap = snapActiveForLines(c.magnetRef.current, c.ctrlHeldRef.current)
      && hasCandleAtCoordinate(chart, c.candlesRef.current, px)
      ? { t: dp.t, p: dp.p }
      : null;
    if (tool === "vertline") {
      const seed = { id: uid("vline"), type: "vline" as const, ts: dp.t, color, lineWidth: readLineWidth("vora.drawings.vline_width", 1), selected: true };
      c.pushHistorySnapshot?.(pruneNullDrawings(c.drawingsRef.current));
      c.setDrawings((q) => [...pruneNullDrawings(q).map((z) => ({ ...z, selected: false })), seed]);
      c.setSnap(snap); setPan(true); c.onFinalizeLine?.();
      e.preventDefault(); e.stopPropagation();
    } else if (tool === "horzline") {
      const seed = { id: uid("hline"), type: "hline" as const, price: dp.p, color, lineWidth: readLineWidth("vora.drawings.hline_width", 1), selected: true };
      c.pushHistorySnapshot?.(pruneNullDrawings(c.drawingsRef.current));
      c.setDrawings((q) => [...pruneNullDrawings(q).map((z) => ({ ...z, selected: false })), seed]);
      c.setSnap(snap); setPan(true); c.onFinalizeLine?.();
      e.preventDefault(); e.stopPropagation();
    } else if (tool === "horzray") {
      const raw = rawAt(px, py);
      if (!raw) return;
      const price = snapActiveForLines(c.magnetRef.current, c.ctrlHeldRef.current)
        && hasCandleAtCoordinate(chart, c.candlesRef.current, px)
        ? snapPriceToOhlc(raw.t, raw.p, c.candlesRef.current).price
        : raw.p;
      const seed = { id: uid("hray"), type: "hray" as const, ts: raw.t, price, color, lineWidth: readLineWidth("vora.drawings.hray_width", 1), selected: true };
      c.pushHistorySnapshot?.(pruneNullDrawings(c.drawingsRef.current));
      c.setDrawings((q) => [...pruneNullDrawings(q).map((z) => ({ ...z, selected: false })), seed]);
      c.setSnap(snapActiveForLines(c.magnetRef.current, c.ctrlHeldRef.current)
        && hasCandleAtCoordinate(chart, c.candlesRef.current, px) ? { t: raw.t, p: price } : null);
      setPan(true); c.onFinalizeLine?.();
      e.preventDefault(); e.stopPropagation();
    } else if (tool === "crossline") {
      const seed = { id: uid("crossline"), type: "crossline" as const, ts: dp.t, price: dp.p, color, lineWidth: readLineWidth("vora.drawings.trendline_width", 1), selected: true };
      c.pushHistorySnapshot?.(pruneNullDrawings(c.drawingsRef.current));
      c.setDrawings((q) => [...pruneNullDrawings(q).map((z) => ({ ...z, selected: false })), seed]);
      c.setSnap(snap); setPan(true); c.onFinalizeLine?.();
      e.preventDefault(); e.stopPropagation();
    } else if (tool === "trendline" || tool === "ray" || tool === "info-line" || tool === "extended-line" || tool === "trend-angle" || tool === "parallel-channel" || tool === "pitchfork" || tool === "schiff-pitchfork" || tool === "modified-schiff-pitchfork" || isFibGannTool(tool)) {
      const needsThreeAnchors = tool === "parallel-channel" || tool === "pitchfork" || tool === "schiff-pitchfork" || tool === "modified-schiff-pitchfork" || (isFibGannTool(tool) && fibGannAnchorCount(tool) === 3);
      c.setTrendPreview({ ts1: dp.t, price1: dp.p, ts2: dp.t, price2: dp.p, tool, stage: needsThreeAnchors ? "base" : undefined, active: true });
      c.setSnap(snap); setPan(false);
      e.preventDefault(); e.stopPropagation();
    }
  };
  const move = (e: MouseEvent): void => {
    const { px, py } = pt(e);
    const tool = c.activeToolRef.current;
    const prev = c.trendPreviewRef.current;
    if (geometryDrag && prev?.tool === geometryDrag.tool) {
      const dp = snapAt(px, py);
      if (dp && Math.hypot(px - geometryDrag.last.x, py - geometryDrag.last.y) >= 3) {
        geometryDrag.points.push({ ts: dp.t, price: dp.p });
        geometryDrag.last = { x: px, y: py };
        c.setTrendPreview({ ...prev, geometryPoints: [...geometryDrag.points] });
      }
      try { container.style.cursor = "crosshair"; } catch { /* ignore */ }
      return;
    }
    if (prev && prev.active && tool === (prev.tool ?? "trendline") && !drag) {
      const dp = snapAt(px, py);
      if (dp) {
        if (prev.tool && isGeometryToolId(prev.tool)) {
          const fixed = (prev.geometryPoints ?? []).slice(0, prev.geometryFixedCount ?? 1);
          const geometryPoints = [...fixed, { ts: dp.t, price: dp.p }];
          const geometrySourceBars = prev.tool === "bars-pattern" && fixed.length >= 2
            ? c.candlesRef.current.filter((candle) => candle.ts >= Math.min(fixed[0]?.ts ?? 0, fixed[1]?.ts ?? 0) && candle.ts <= Math.max(fixed[0]?.ts ?? 0, fixed[1]?.ts ?? 0)).map(({ ts, open, high, low, close, volume }) => ({ ts, open, high, low, close, volume }))
            : prev.geometrySourceBars;
          c.setTrendPreview({ ...prev, ts2: dp.t, price2: dp.p, geometryPoints, geometrySourceBars });
          try { container.style.cursor = "crosshair"; } catch { /* ignore */ }
          return;
        }
        const offsetStage = (prev.tool === "parallel-channel" || prev.tool === "pitchfork" || prev.tool === "schiff-pitchfork" || prev.tool === "modified-schiff-pitchfork" || (isFibGannTool(prev.tool) && fibGannAnchorCount(prev.tool) === 3)) && prev.stage === "offset";
        c.setTrendPreview(offsetStage
          ? { ...prev, ts3: dp.t, price3: dp.p, tool }
          : { ...prev, ts2: dp.t, price2: dp.p, tool });
        c.setSnap(snapMarkerAt(px, { t: dp.t, p: dp.p }));
      }
      try { container.style.cursor = "crosshair"; } catch { /* ignore */ }
      return;
    }
    if (!drag) {
      const hh = hit(px, py);
      c.setLineHover(hh ? { id: hh.d.id, handle: hh.handle } : null);
      try {
        if (hh) container.style.cursor = hh.handle ? cursorForLineHandle(hh.d, hh.handle) : "move";
        else if (tool) container.style.cursor = "crosshair";
        else container.style.cursor = "";
      } catch { /* ignore */ }
      return;
    }
    const dp = snapAt(px, py);
    const raw = rawAt(px, py);
    if (!dp || !raw) return;
    if (drag.handle) {
      const dragId = drag.id; const dragHandle = drag.handle;
      c.setDrawings((list) => pruneNullDrawings(list).map((z) => {
        if (z.id !== dragId) return { ...z, selected: false };
        if (z.type === "geometry") return {
          ...z,
          points: z.points.map((point, index) => {
            if (dragHandle === `p${index}`) return { ts: dp.t, price: dp.p };
            if (z.tool === "rr_long" || z.tool === "rr_short") {
              if (index === 2 && dragHandle === "p2-price") return { ...point, price: dp.p };
              if (index === 2 && dragHandle === "p2-time") return { ...point, ts: dp.t };
            }
            return point;
          }),
          selected: true,
        };
        if (z.type === "icon") return { ...z, ts: dp.t, price: dp.p, selected: true };
        if (z.type === "vline") return { ...z, ts: dp.t, selected: true };
        if (z.type === "hline") return { ...z, price: dp.p, selected: true };
        if (z.type === "hray") return { ...z, ts: raw.t, price: dp.p, selected: true };
        if (z.type === "crossline") return { ...z, ts: dp.t, price: dp.p, selected: true };
        if (z.type === "parallel-channel") {
          if (dragHandle === "p1") return { ...z, ts1: dp.t, price1: dp.p, selected: true };
          if (dragHandle === "p2") return { ...z, ts2: dp.t, price2: dp.p, selected: true };
          return { ...z, ts3: dp.t, price3: dp.p, selected: true };
        }
        if (z.type === "pitchfork") {
          if (dragHandle === "p1") return { ...z, ts1: dp.t, price1: dp.p, selected: true };
          if (dragHandle === "p2") return { ...z, ts2: dp.t, price2: dp.p, selected: true };
          return { ...z, ts3: dp.t, price3: dp.p, selected: true };
        }
        if (z.type === "fib-gann") {
          if (dragHandle === "p1") return { ...z, ts1: dp.t, price1: dp.p, selected: true };
          if (dragHandle === "p2") return { ...z, ts2: dp.t, price2: dp.p, selected: true };
          if (z.ts3 !== undefined && z.price3 !== undefined) return { ...z, ts3: dp.t, price3: dp.p, selected: true };
          return { ...z, selected: true };
        }
        if (z.type === "trendline" || z.type === "ray" || z.type === "info-line" || z.type === "extended-line" || z.type === "trend-angle") {
          if (dragHandle === "p1") return { ...z, ts1: dp.t, price1: dp.p, selected: true };
          return { ...z, ts2: dp.t, price2: dp.p, selected: true };
        }
        return { ...z, selected: true };
      }));
      c.setSnap(snapMarkerAt(px, { t: dp.t, p: dp.p }));
      return;
    }
    const o = drag.orig;
    const dT = raw.t - drag.grabT;
    const dP = raw.p - drag.grabP;
    const moveId = drag.id;
    c.setDrawings((list) => pruneNullDrawings(list).map((z) => {
      if (z.id !== moveId) return { ...z, selected: false };
      if (z.type === "geometry") {
        const original = o;
        if (original.type !== "geometry") return z;
        return { ...z, points: original.points.map((point) => ({ ts: point.ts + dT, price: point.price + dP })), selected: true };
      }
      if (z.type === "icon") {
        const original = o as DrawingIcon;
        return { ...z, ts: original.ts + dT, price: original.price + dP, selected: true };
      }
      if (z.type === "vline") return { ...z, ts: (o as { ts: number }).ts + dT, selected: true };
      if (z.type === "hline") return { ...z, price: (o as { price: number }).price + dP, selected: true };
      if (z.type === "hray") {
        if (o.type !== "hray") return z;
        const original = o;
        return { ...z, ts: original.ts + dT, price: original.price + dP, selected: true };
      }
      if (z.type === "crossline") {
        const original = o as { ts: number; price: number };
        return { ...z, ts: original.ts + dT, price: original.price + dP, selected: true };
      }
      if (z.type === "parallel-channel") return { ...z, ts1: z.ts1 + dT, price1: z.price1 + dP, ts2: z.ts2 + dT, price2: z.price2 + dP, ts3: z.ts3 + dT, price3: z.price3 + dP, selected: true };
      if (z.type === "pitchfork") return { ...z, ts1: z.ts1 + dT, price1: z.price1 + dP, ts2: z.ts2 + dT, price2: z.price2 + dP, ts3: z.ts3 + dT, price3: z.price3 + dP, selected: true };
      if (z.type === "fib-gann") {
        const original = o as DrawingFibGann;
        return { ...z, ts1: original.ts1 + dT, price1: original.price1 + dP, ts2: original.ts2 + dT, price2: original.price2 + dP, ts3: original.ts3 === undefined ? undefined : original.ts3 + dT, price3: original.price3 === undefined ? undefined : original.price3 + dP, selected: true };
      }
      if (z.type === "trendline" || z.type === "ray" || z.type === "info-line" || z.type === "extended-line" || z.type === "trend-angle") {
        const t = o as { ts1: number; price1: number; ts2: number; price2: number };
        return { ...z, ts1: t.ts1 + dT, price1: t.price1 + dP, ts2: t.ts2 + dT, price2: t.price2 + dP, selected: true };
      }
      return { ...z, selected: true };
    }));
  };
  const up = (): void => {
    if (geometryDrag) {
      const completed = geometryDrag;
      geometryDrag = null;
      if (completed.points.length > 1) {
        c.pushHistorySnapshot?.(pruneNullDrawings(c.drawingsRef.current));
        const seed = { id: uid("geometry"), type: "geometry" as const, tool: completed.tool, points: completed.points, color: readLineColor(), lineWidth: readLineWidth("vora.drawings.trendline_width", completed.tool === "highlighter" ? 10 : 2), selected: true };
        c.setDrawings((current) => [...pruneNullDrawings(current).map((drawing) => ({ ...drawing, selected: false })), seed]);
      }
      c.setTrendPreview(null);
      setPan(true);
      c.onFinalizeLine?.();
    }
    if (drag) { drag = null; setPan(true); }
  };
  const key = (ev: KeyboardEvent): void => {
    if (ev.target instanceof HTMLElement && (ev.target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(ev.target.tagName))) return;
    const preview = c.trendPreviewRef.current;
    if (ev.key === "Enter" && (preview?.tool === "path" || preview?.tool === "polyline" || preview?.tool === "ghost-feed")) {
      const points = (preview.geometryPoints ?? []).slice(0, preview.geometryFixedCount ?? 0);
      if (points.length >= 2) {
        const seed = makeGeometryDrawing(preview.tool, points);
        c.pushHistorySnapshot?.(pruneNullDrawings(c.drawingsRef.current));
        c.setDrawings((current) => [...pruneNullDrawings(current).map((drawing) => ({ ...drawing, selected: false })), seed]);
        c.setTrendPreview(null); setPan(true); c.onFinalizeLine?.();
      }
      return;
    }
    if (ev.key === "Escape" && c.trendPreviewRef.current?.active) {
      cancelPreview();
    } else if (ev.key === "Escape") {
      // v0.82.0 D3 — Escape deselects (toolbar hides).
      // v0.83.14 — null-safe: prune before some/map.
      c.setDrawings((prev) => pruneNullDrawings(prev).some((z) => z.selected) ? pruneNullDrawings(prev).map((z) => ({ ...z, selected: false })) : pruneNullDrawings(prev));
    }
  };
  const contextMenu = (ev: MouseEvent): void => {
    if (ev.target instanceof Element && ev.target.closest(".vora-floating-ui")) return;
    const preview = c.trendPreviewRef.current;
    if (!preview?.active) return;
    ev.preventDefault();
    ev.stopPropagation();
    c.onBlankContextMenu?.();
    cancelPreview();
    c.onFinalizeLine?.();
  };
  container.addEventListener("mousedown", place, true);
  container.addEventListener("contextmenu", contextMenu, true);
  window.addEventListener("mousemove", move, true);
  window.addEventListener("mouseup", up, true);
  window.addEventListener("keydown", key, true);
  return () => {
    container.removeEventListener("mousedown", place, true);
    container.removeEventListener("contextmenu", contextMenu, true);
    window.removeEventListener("mousemove", move, true);
    window.removeEventListener("mouseup", up, true);
    window.removeEventListener("keydown", key, true);
  };
}
