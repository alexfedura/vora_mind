import type { DrawingAnchor, DrawingCandleSample, GeometryToolId } from "./drawingTypes";
import { isGeometryToolId, type ScreenPoint } from "./geometryToolGeometry";

export interface ForecastPath {
  points: ScreenPoint[];
  closed: boolean;
  stroke?: string;
  fill?: string;
  dashed?: boolean;
}

export interface ForecastCandle {
  x: number;
  highY: number;
  lowY: number;
  openY: number;
  closeY: number;
  up: boolean;
  ghost?: boolean;
}

export interface ForecastLabel {
  x: number;
  y: number;
  text: string;
  color: string;
  background?: string;
  border?: string;
  fontSize?: number;
  bold?: boolean;
}

export interface ForecastGeometry {
  paths: ForecastPath[];
  candles: ForecastCandle[];
  labels: ForecastLabel[];
}

type Project = (anchor: DrawingAnchor) => ScreenPoint | null;

function path(points: ScreenPoint[], options: Partial<Omit<ForecastPath, "points" | "closed">> = {}): ForecastPath {
  return { points, closed: false, ...options };
}

function projected(anchors: readonly DrawingAnchor[], project: Project): ScreenPoint[] | null {
  const points = anchors.map(project);
  return points.some((point) => point === null) ? null : points.filter((point): point is ScreenPoint => point !== null);
}

function labelAt(x: number, y: number, text: string, color: string): ForecastLabel {
  return { x, y, text, color };
}

function valueLabel(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}`;
}

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ${Math.floor(seconds % 3600 / 60)}m`;
  return `${Math.floor(seconds / 86400)}d ${Math.floor(seconds % 86400 / 3600)}h`;
}

function pricePrecision(value: number): number {
  const fixed = value.toFixed(8).replace(/0+$/, "");
  const decimalIndex = fixed.indexOf(".");
  return decimalIndex < 0 ? 0 : fixed.length - decimalIndex - 1;
}

function formattedPrice(value: number, precision: number): string {
  return new Intl.NumberFormat("en-US", {
    minimumFractionDigits: precision,
    maximumFractionDigits: precision,
  }).format(value);
}

function formattedAmount(value: number): string {
  return new Intl.NumberFormat("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value);
}

function measurementLabel(tool: GeometryToolId, first: DrawingAnchor, second: DrawingAnchor, candles: readonly DrawingCandleSample[]): string {
  const delta = second.price - first.price;
  const percent = first.price === 0 ? 0 : delta / Math.abs(first.price) * 100;
  const seconds = Math.abs(second.ts - first.ts);
  if (tool === "price_range") return `${valueLabel(delta)} (${valueLabel(percent)}%)`;
  const start = Math.min(first.ts, second.ts);
  const end = Math.max(first.ts, second.ts);
  const barCount = candles.filter((candle) => candle.ts >= start && candle.ts <= end).length;
  const dateValues = `${formatDuration(seconds)} · ${barCount} bars`;
  if (tool === "date_range") return dateValues;
  return `${valueLabel(delta)} (${valueLabel(percent)}%) · ${dateValues}`;
}

function makeVolumeProfile(
  values: readonly DrawingCandleSample[],
  anchor: DrawingAnchor,
  project: Project,
  width: number,
): ForecastPath[] {
  if (values.length === 0) return [];
  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  for (const candle of values) {
    min = Math.min(min, candle.low);
    max = Math.max(max, candle.high);
  }
  const spread = Math.max(max - min, Math.abs(max) * 0.0001, 0.000001);
  const bins = 28;
  const volumes = Array.from({ length: bins }, () => 0);
  for (const candle of values) {
    const start = Math.max(0, Math.min(bins - 1, Math.floor((candle.low - min) / spread * bins)));
    const end = Math.max(start, Math.min(bins - 1, Math.floor((candle.high - min) / spread * bins)));
    const contribution = Math.max(0, candle.volume) / (end - start + 1);
    for (let index = start; index <= end; index += 1) volumes[index] = (volumes[index] ?? 0) + contribution;
  }
  const maxVolume = Math.max(...volumes, 1);
  const anchorPoint = project(anchor);
  if (!anchorPoint) return [];
  const maxBarWidth = Math.min(124, Math.max(48, width * 0.22));
  const paths: ForecastPath[] = [];
  for (let index = 0; index < bins; index += 1) {
    const lower = min + spread * index / bins;
    const upper = min + spread * (index + 1) / bins;
    const top = project({ ts: anchor.ts, price: upper });
    const bottom = project({ ts: anchor.ts, price: lower });
    const volume = volumes[index] ?? 0;
    if (!top || !bottom || volume <= 0) continue;
    const barWidth = Math.max(1, maxBarWidth * volume / maxVolume);
    const right = Math.min(width, anchorPoint.x + maxBarWidth);
    const left = right - barWidth;
    paths.push({
      points: [{ x: left, y: top.y }, { x: right, y: top.y }, { x: right, y: bottom.y }, { x: left, y: bottom.y }],
      closed: true,
      fill: "rgba(83, 157, 142, 0.58)",
      stroke: "rgba(132, 198, 180, 0.65)",
    });
  }
  return paths;
}

function profileCandles(candles: readonly DrawingCandleSample[], tool: GeometryToolId, anchors: readonly DrawingAnchor[]): DrawingCandleSample[] {
  const first = anchors[0];
  if (!first) return [];
  if (tool === "anchored-volume-profile") return candles.filter((candle) => candle.ts >= first.ts);
  const second = anchors[1];
  if (!second) return [];
  const start = Math.min(first.ts, second.ts);
  const end = Math.max(first.ts, second.ts);
  return candles.filter((candle) => candle.ts >= start && candle.ts <= end);
}

export function isForecastMeasurementTool(tool: string): tool is GeometryToolId {
  return isGeometryToolId(tool) && tool !== "brush" && tool !== "highlighter" && tool !== "arrow-marker" && tool !== "arrow" && tool !== "arrow-mark-up" && tool !== "arrow-mark-down" && tool !== "rotated-rectangle" && tool !== "path" && tool !== "circle" && tool !== "ellipse" && tool !== "polyline" && tool !== "triangle-shape" && tool !== "arc" && tool !== "curve" && tool !== "double-curve";
}

export function buildForecastMeasurementGeometry(
  tool: GeometryToolId,
  anchors: readonly DrawingAnchor[],
  candles: readonly DrawingCandleSample[],
  sourceBars: readonly DrawingCandleSample[],
  project: Project,
  width: number,
  barSpacing: number,
): ForecastGeometry | null {
  if (!isForecastMeasurementTool(tool)) return null;
  const points = projected(anchors, project);
  if (!points) return null;
  const first = anchors[0];
  const firstPoint = points[0];
  if (!first || !firstPoint) return null;
  const output: ForecastGeometry = { paths: [], candles: [], labels: [] };

  if (tool === "rr_long" || tool === "rr_short") {
    const entry = anchors[0];
    const stop = anchors[1];
    const target = anchors[2];
    const entryPoint = points[0];
    const stopPoint = points[1];
    const targetPoint = points[2];
    if (!entry || !stop || !target || !entryPoint || !stopPoint || !targetPoint) return output;
    const left = Math.min(entryPoint.x, stopPoint.x, targetPoint.x);
    const right = Math.max(entryPoint.x, stopPoint.x, targetPoint.x);
    const riskColor = "rgba(242, 54, 69, 0.20)";
    const rewardColor = "rgba(0, 150, 136, 0.18)";
    output.paths.push(
      { points: [{ x: left, y: entryPoint.y }, { x: right, y: entryPoint.y }, { x: right, y: stopPoint.y }, { x: left, y: stopPoint.y }], closed: true, fill: riskColor, stroke: "#f23645" },
      { points: [{ x: left, y: entryPoint.y }, { x: right, y: entryPoint.y }, { x: right, y: targetPoint.y }, { x: left, y: targetPoint.y }], closed: true, fill: rewardColor, stroke: "#009688" },
    );
    const risk = Math.abs(entry.price - stop.price);
    const reward = Math.abs(target.price - entry.price);
    const ratio = risk === 0 ? 0 : reward / risk;
    const precision = Math.min(8, Math.max(2, pricePrecision(entry.price)));
    const tickSize = 10 ** -Math.max(2, precision - 1);
    const targetPercent = entry.price === 0 ? 0 : reward / Math.abs(entry.price) * 100;
    const stopPercent = entry.price === 0 ? 0 : risk / Math.abs(entry.price) * 100;
    const quantity = 1;
    const isLong = tool === "rr_long";
    const latestClose = candles[candles.length - 1]?.close;
    const openPnl = latestClose === undefined
      ? "—"
      : formattedPrice((latestClose - entry.price) * (isLong ? 1 : -1) * quantity, precision);
    output.labels.push({
      ...labelAt(
        (left + right) / 2,
        targetPoint.y + (isLong ? -20 : 20),
        `Target: ${formattedPrice(reward, precision)} (${targetPercent.toFixed(3)}%) ${formattedPrice(reward / tickSize, 1)}, Amount: ${formattedAmount(reward * quantity)}`,
        "#ffffff",
      ),
      background: "#009688",
      fontSize: 12,
      bold: true,
    });
    output.labels.push({
      ...labelAt(
        (left + right) / 2,
        (entryPoint.y + stopPoint.y) / 2,
        `Open P&L: ${openPnl}, Qty: ${quantity}\nRisk/reward ratio: ${ratio.toFixed(2)}`,
        "#ffffff",
      ),
      background: "#f23645",
      border: "#ffffff",
      fontSize: 12,
      bold: true,
    });
    output.labels.push({
      ...labelAt(
        (left + right) / 2,
        stopPoint.y + (isLong ? 20 : -20),
        `Stop: ${formattedPrice(risk, precision)} (${stopPercent.toFixed(3)}%) ${formattedPrice(risk / tickSize, 1)}, Amount: ${formattedAmount(risk * quantity)}`,
        "#ffffff",
      ),
      background: "#f23645",
      fontSize: 12,
      bold: true,
    });
    return output;
  }

  if (tool === "position-forecast") {
    const end = points[1];
    const second = anchors[1];
    if (!end || !second) return output;
    output.paths.push(path([firstPoint, end], { stroke: "#89a9ff", dashed: true }));
    const delta = second.price - first.price;
    const percent = first.price === 0 ? 0 : delta / Math.abs(first.price) * 100;
    const barCount = candles.filter((candle) => candle.ts >= Math.min(first.ts, second.ts) && candle.ts <= Math.max(first.ts, second.ts)).length;
    output.labels.push(labelAt(end.x, end.y, `${valueLabel(delta)} (${valueLabel(percent)}%) · ${formatDuration(Math.abs(second.ts - first.ts))} · ${barCount} bars`, "#d6e1ff"));
    return output;
  }

  if (tool === "bars-pattern") {
    const destination = anchors[2];
    const destinationPoint = points[2];
    const sampleStart = anchors[0];
    const sampleEnd = anchors[1];
    if (!destination || !destinationPoint || !sampleStart || !sampleEnd) return output;
    const offset = destination.price - sampleStart.price;
    const spacing = Math.max(2, barSpacing);
    for (let index = 0; index < sourceBars.length; index += 1) {
      const sample = sourceBars[index];
      if (!sample) continue;
      const translated = [sample.open, sample.high, sample.low, sample.close].map((price) => project({ ts: destination.ts, price: price + offset }));
      const [open, high, low, close] = translated;
      if (!open || !high || !low || !close) continue;
      output.candles.push({ x: destinationPoint.x + index * spacing, highY: high.y, lowY: low.y, openY: open.y, closeY: close.y, up: sample.close >= sample.open });
    }
    return output;
  }

  if (tool === "ghost-feed") {
    output.paths.push(path(points, { stroke: "rgba(173, 184, 208, 0.76)", dashed: true }));
    for (let index = 1; index < points.length; index += 1) {
      const previous = points[index - 1];
      const current = points[index];
      if (!previous || !current) continue;
      const open = project(anchors[index - 1] ?? first);
      const close = project(anchors[index] ?? first);
      if (!open || !close) continue;
      const half = Math.max(2, barSpacing * 0.23);
      output.candles.push({ x: current.x, highY: Math.min(open.y, close.y) - 3, lowY: Math.max(open.y, close.y) + 3, openY: open.y, closeY: close.y, up: close.y < open.y, ghost: true });
      output.paths.push(path([{ x: current.x - half, y: current.y }, { x: current.x + half, y: current.y }], { stroke: "rgba(173, 184, 208, 0.45)" }));
    }
    return output;
  }

  if (tool === "sector") {
    const start = points[1];
    const end = points[2];
    if (!start || !end) return output;
    const startAngle = Math.atan2(start.y - firstPoint.y, start.x - firstPoint.x);
    const endAngle = Math.atan2(end.y - firstPoint.y, end.x - firstPoint.x);
    const radius = Math.hypot(start.x - firstPoint.x, start.y - firstPoint.y);
    let sweep = (endAngle - startAngle + Math.PI * 2) % (Math.PI * 2);
    if (sweep > Math.PI) sweep -= Math.PI * 2;
    const arc = Array.from({ length: 25 }, (_, index) => {
      const angle = startAngle + sweep * index / 24;
      return { x: firstPoint.x + Math.cos(angle) * radius, y: firstPoint.y + Math.sin(angle) * radius };
    });
    output.paths.push({ points: [firstPoint, ...arc, firstPoint], closed: true, fill: "rgba(102, 137, 210, 0.18)", stroke: "#829ee0" });
    const startAnchor = anchors[1];
    const endAnchor = anchors[2];
    const measurements = startAnchor && endAnchor ? `${formatDuration(Math.abs(endAnchor.ts - startAnchor.ts))} · ${valueLabel(endAnchor.price - startAnchor.price)}` : "";
    output.labels.push(labelAt(firstPoint.x, firstPoint.y, `${Math.abs(sweep * 180 / Math.PI).toFixed(1)}° · ${measurements}`, "#c6d4ff"));
    return output;
  }

  if (tool === "anchored-vwap") {
    let totalVolume = 0;
    let weightedPrice = 0;
    const vwapPoints: ScreenPoint[] = [];
    for (const candle of candles) {
      if (candle.ts < first.ts || candle.volume <= 0) continue;
      const typical = (candle.high + candle.low + candle.close) / 3;
      totalVolume += candle.volume;
      weightedPrice += typical * candle.volume;
      const point = project({ ts: candle.ts, price: weightedPrice / totalVolume });
      if (point) vwapPoints.push(point);
    }
    if (vwapPoints.length > 0) output.paths.push(path(vwapPoints, { stroke: "#f1c76a" }));
    return output;
  }

  if (tool === "fixed-range-volume-profile" || tool === "anchored-volume-profile") {
    const profileAnchor = tool === "fixed-range-volume-profile" ? anchors[1] ?? first : first;
    output.paths = makeVolumeProfile(profileCandles(candles, tool, anchors), profileAnchor, project, width);
    return output;
  }

  if (tool === "price_range" || tool === "date_range" || tool === "date_price_range") {
    const second = anchors[1];
    const secondPoint = points[1];
    if (!second || !secondPoint) return output;
    if (tool === "date_price_range") {
      output.paths.push({ points: [firstPoint, { x: secondPoint.x, y: firstPoint.y }, secondPoint, { x: firstPoint.x, y: secondPoint.y }, firstPoint], closed: true, fill: "rgba(116, 145, 201, 0.12)", stroke: "#829ee0" });
    } else {
      output.paths.push(path([firstPoint, secondPoint], { stroke: tool === "price_range" ? "#69b8a0" : "#82a5e8" }));
    }
    output.labels.push(labelAt((firstPoint.x + secondPoint.x) / 2, (firstPoint.y + secondPoint.y) / 2, measurementLabel(tool, first, second, candles), tool === "price_range" ? "#8ed3be" : "#c6d4ff"));
    return output;
  }
  return output;
}