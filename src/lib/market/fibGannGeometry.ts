import type { FibGannToolId } from "./drawingTypes";
import type { DrawingFibGann } from "./lineDrawings";

export interface ScreenAnchor {
  x: number;
  y: number;
}

export interface GeometryLabel {
  x: number;
  y: number;
  text: string;
}

export interface FibGannGeometry {
  paths: ScreenAnchor[][];
  labels: GeometryLabel[];
}

const FIB_LEVELS = [0, 0.236, 0.382, 0.5, 0.618, 0.786, 1, 1.272, 1.618, 2, 2.618, 3.618, 4.236];
const EXTENSION_LEVELS = [0, 0.382, 0.618, 1, 1.272, 1.618, 2, 2.618, 3.618, 4.236];
const TIME_LEVELS = [0, 1, 2, 3, 5, 8, 13, 21];
const FIB_GANN_TOOL_IDS: ReadonlySet<string> = new Set([
  "fib-retracement", "fib-extension", "fib-channel", "fib-time-zone", "fib-speed-resistance-fan",
  "trend-based-fib-time", "fib-circles", "fib-spiral", "fib-speed-resistance-arcs", "fib-wedge",
  "pitchfan", "gann-box", "gann-square-fixed", "gann-square", "gann-fan",
]);

export function isFibGannTool(tool: string | undefined): tool is FibGannToolId {
  return tool !== undefined && FIB_GANN_TOOL_IDS.has(tool);
}

export function fibGannAnchorCount(tool: FibGannToolId): 2 | 3 {
  return tool === "fib-extension" || tool === "fib-channel" || tool === "pitchfan" ? 3 : 2;
}

export function fibGannTimeLevels(drawing: DrawingFibGann): number[] {
  if (drawing.tool !== "fib-time-zone" && drawing.tool !== "trend-based-fib-time") return [];
  const duration = drawing.ts2 - drawing.ts1;
  return TIME_LEVELS.map((ratio) => drawing.ts1 + duration * ratio);
}

export function fibGannAlertLevels(drawing: DrawingFibGann, timestamp: number): number[] {
  const delta = drawing.price2 - drawing.price1;
  const lowerTime = Math.min(drawing.ts1, drawing.ts2);
  const upperTime = Math.max(drawing.ts1, drawing.ts2);
  if (drawing.tool === "fib-time-zone" || drawing.tool === "trend-based-fib-time") return [];
  if (drawing.tool === "fib-retracement") {
    if (timestamp < lowerTime || timestamp > upperTime) return [];
    return FIB_LEVELS.slice(0, 7).map((ratio) => drawing.price1 + delta * ratio);
  }
  if (drawing.tool === "fib-extension") {
    const thirdPrice = drawing.price3;
    if (drawing.ts3 === undefined || thirdPrice === undefined || timestamp < Math.min(drawing.ts3, upperTime)) return [];
    return EXTENSION_LEVELS.map((ratio) => thirdPrice + delta * ratio);
  }
  if (drawing.tool === "fib-channel" && drawing.ts3 !== undefined && drawing.price3 !== undefined) {
    const duration = drawing.ts2 - drawing.ts1;
    const baselineAtThird = duration === 0 ? drawing.price1 : drawing.price1 + ((drawing.ts3 - drawing.ts1) / duration) * delta;
    const offset = drawing.price3 - baselineAtThird;
    const baseline = duration === 0 ? drawing.price1 : drawing.price1 + ((timestamp - drawing.ts1) / duration) * delta;
    return [0, 0.236, 0.382, 0.5, 0.618, 1].map((ratio) => baseline + offset * ratio);
  }
  if (drawing.tool === "gann-box" || drawing.tool === "gann-square" || drawing.tool === "gann-square-fixed") {
    return Array.from({ length: 9 }, (_, index) => drawing.price1 + delta * index / 8);
  }
  if (drawing.tool === "gann-fan" || drawing.tool === "fib-speed-resistance-fan" || drawing.tool === "pitchfan") {
    const duration = drawing.ts2 - drawing.ts1;
    if (duration === 0) return [drawing.price1];
    const timeRatio = (timestamp - drawing.ts1) / duration;
    const slopes = drawing.tool === "gann-fan" ? [1 / 8, 1 / 4, 1 / 3, 1 / 2, 1, 2, 3, 4, 8] : FIB_LEVELS.slice(0, 7);
    return slopes.map((ratio) => drawing.price1 + delta * timeRatio * ratio);
  }
  return FIB_LEVELS.slice(0, 7).map((ratio) => drawing.price1 + delta * ratio);
}

function clippedRay(origin: ScreenAnchor, direction: ScreenAnchor, width: number, height: number): ScreenAnchor {
  const dx = direction.x - origin.x;
  const dy = direction.y - origin.y;
  const factors = [
    dx > 0 ? (width - origin.x) / dx : Number.POSITIVE_INFINITY,
    dx < 0 ? -origin.x / dx : Number.POSITIVE_INFINITY,
    dy > 0 ? (height - origin.y) / dy : Number.POSITIVE_INFINITY,
    dy < 0 ? -origin.y / dy : Number.POSITIVE_INFINITY,
  ].filter((factor) => Number.isFinite(factor) && factor >= 0);
  const factor = factors.length ? Math.max(1, Math.min(...factors)) : 1;
  return { x: origin.x + dx * factor, y: origin.y + dy * factor };
}

function circlePath(center: ScreenAnchor, radius: number, start = 0, end = Math.PI * 2, steps = 64): ScreenAnchor[] {
  return Array.from({ length: steps + 1 }, (_, index) => {
    const angle = start + (end - start) * index / steps;
    return { x: center.x + Math.cos(angle) * radius, y: center.y + Math.sin(angle) * radius };
  });
}

function line(paths: ScreenAnchor[][], first: ScreenAnchor, second: ScreenAnchor): void {
  paths.push([first, second]);
}

function ratioLabel(ratio: number): string {
  return `${ratio.toFixed(3).replace(/0+$/, "").replace(/\.$/, "")}`;
}

export function buildFibGannGeometry(
  tool: FibGannToolId,
  anchors: readonly ScreenAnchor[],
  width: number,
  height: number,
): FibGannGeometry {
  if (anchors.length < fibGannAnchorCount(tool)) return { paths: [], labels: [] };
  const [first, second, third] = anchors;
  if (!first || !second) return { paths: [], labels: [] };
  const paths: ScreenAnchor[][] = [];
  const labels: GeometryLabel[] = [];
  const spanX = second.x - first.x;
  const spanY = second.y - first.y;
  const distance = Math.hypot(spanX, spanY);
  const angle = Math.atan2(spanY, spanX);

  if (tool === "fib-retracement") {
    for (const ratio of FIB_LEVELS.slice(0, 7)) {
      const y = first.y + spanY * ratio;
      line(paths, { x: Math.min(first.x, second.x), y }, { x: Math.max(first.x, second.x), y });
      labels.push({ x: Math.max(first.x, second.x), y, text: ratioLabel(ratio) });
    }
  } else if (tool === "fib-extension") {
    if (!third) return { paths, labels };
    const projectedSpan = second.y - first.y;
    for (const ratio of EXTENSION_LEVELS) {
      const y = third.y + projectedSpan * ratio;
      line(paths, { x: third.x, y }, { x: width, y });
      labels.push({ x: width - 4, y, text: ratioLabel(ratio) });
    }
  } else if (tool === "fib-channel") {
    if (!third) return { paths, labels };
    const length = Math.max(distance, 1);
    const normal = { x: -spanY / length, y: spanX / length };
    const offset = (third.x - first.x) * normal.x + (third.y - first.y) * normal.y;
    line(paths, first, second);
    for (const ratio of FIB_LEVELS.slice(1, 7)) {
      const shift = offset * ratio;
      const a = { x: first.x + normal.x * shift, y: first.y + normal.y * shift };
      const b = { x: second.x + normal.x * shift, y: second.y + normal.y * shift };
      line(paths, a, clippedRay(a, b, width, height));
      labels.push({ x: b.x, y: b.y, text: ratioLabel(ratio) });
    }
  } else if (tool === "fib-time-zone" || tool === "trend-based-fib-time") {
    for (const ratio of TIME_LEVELS) {
      const x = first.x + spanX * ratio;
      line(paths, { x, y: 0 }, { x, y: height });
      labels.push({ x, y: 12, text: `${ratio}` });
    }
  } else if (tool === "fib-speed-resistance-fan") {
    for (const ratio of FIB_LEVELS.slice(0, 7)) {
      const target = { x: width, y: first.y + spanY * ratio };
      line(paths, first, clippedRay(first, target, width, height));
      labels.push({ x: target.x - 4, y: target.y, text: ratioLabel(ratio) });
    }
  } else if (tool === "fib-circles") {
    for (const ratio of FIB_LEVELS.slice(1, 8)) {
      const radius = Math.max(2, distance * ratio);
      paths.push(circlePath(first, radius));
      labels.push({ x: first.x + radius, y: first.y, text: ratioLabel(ratio) });
    }
  } else if (tool === "fib-spiral") {
    const turns = 5;
    const steps = 240;
    const spiral = Array.from({ length: steps + 1 }, (_, index) => {
      const progress = index / steps;
      const theta = progress * Math.PI * 2 * turns;
      const radius = Math.max(1, distance * progress);
      return { x: first.x + Math.cos(theta + angle) * radius, y: first.y + Math.sin(theta + angle) * radius };
    });
    paths.push(spiral);
  } else if (tool === "fib-speed-resistance-arcs" || tool === "fib-wedge") {
    const startAngle = angle;
    const arcStart = tool === "fib-wedge" ? startAngle - Math.PI / 3 : startAngle - Math.PI / 2;
    const arcEnd = tool === "fib-wedge" ? startAngle + Math.PI / 3 : startAngle + Math.PI / 2;
    if (tool === "fib-wedge") {
      const firstEdge = { x: first.x + Math.cos(arcStart) * distance, y: first.y + Math.sin(arcStart) * distance };
      const secondEdge = { x: first.x + Math.cos(arcEnd) * distance, y: first.y + Math.sin(arcEnd) * distance };
      line(paths, first, clippedRay(first, firstEdge, width, height));
      line(paths, first, clippedRay(first, secondEdge, width, height));
    }
    for (const ratio of FIB_LEVELS.slice(1, 7)) {
      const radius = Math.max(2, distance * ratio);
      paths.push(circlePath(first, radius, arcStart, arcEnd));
      labels.push({ x: first.x + Math.cos(startAngle) * radius, y: first.y + Math.sin(startAngle) * radius, text: ratioLabel(ratio) });
    }
  } else if (tool === "pitchfan") {
    if (!third) return { paths, labels };
    const direction = { x: (second.x + third.x) / 2 - first.x, y: (second.y + third.y) / 2 - first.y };
    const spread = Math.hypot(third.x - second.x, third.y - second.y) / 2;
    for (const ratio of [-1, -0.618, 0, 0.618, 1]) {
      const target = { x: first.x + direction.x, y: first.y + direction.y + spread * ratio };
      line(paths, first, clippedRay(first, target, width, height));
    }
  } else if (tool === "gann-box" || tool === "gann-square" || tool === "gann-square-fixed") {
    let left = Math.min(first.x, second.x);
    let right = Math.max(first.x, second.x);
    let top = Math.min(first.y, second.y);
    let bottom = Math.max(first.y, second.y);
    if (tool === "gann-square-fixed") {
      const side = Math.max(1, Math.min(width - first.x, height - first.y, distance));
      left = first.x;
      right = first.x + side;
      top = first.y;
      bottom = first.y + side;
    } else if (tool === "gann-square") {
      const side = Math.max(1, Math.min(Math.abs(spanX), Math.abs(spanY)));
      right = left + side;
      bottom = top + side;
    }
    line(paths, { x: left, y: top }, { x: right, y: top });
    line(paths, { x: right, y: top }, { x: right, y: bottom });
    line(paths, { x: right, y: bottom }, { x: left, y: bottom });
    line(paths, { x: left, y: bottom }, { x: left, y: top });
    line(paths, { x: left, y: top }, { x: right, y: bottom });
    line(paths, { x: right, y: top }, { x: left, y: bottom });
    for (let division = 1; division < 8; division += 1) {
      const ratio = division / 8;
      const x = left + (right - left) * ratio;
      const y = top + (bottom - top) * ratio;
      line(paths, { x, y: top }, { x, y: bottom });
      line(paths, { x: left, y }, { x: right, y });
    }
  } else if (tool === "gann-fan") {
    for (const ratio of [1 / 8, 1 / 4, 1 / 3, 1 / 2, 1, 2, 3, 4, 8]) {
      const target = { x: width, y: first.y + spanY * ratio };
      line(paths, first, clippedRay(first, target, width, height));
    }
  }

  return { paths, labels };
}