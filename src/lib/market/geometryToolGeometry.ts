import { GEOMETRY_TOOL_LABELS, type DrawingAnchor, type GeometryToolId } from "./drawingTypes";

export interface ScreenPoint {
  x: number;
  y: number;
}

export interface GeometryPath {
  points: ScreenPoint[];
  closed: boolean;
  arrowHead: boolean;
  startArrowHead?: boolean;
  endArrowHead?: boolean;
}

export interface GeometryPathOptions {
  startArrow?: boolean;
  endArrow?: boolean;
}

export function isGeometryToolId(id: string): id is GeometryToolId {
  return Object.prototype.hasOwnProperty.call(GEOMETRY_TOOL_LABELS, id);
}

export function geometryAnchorCount(tool: GeometryToolId): number {
  if (tool === "arrow-mark-up" || tool === "arrow-mark-down") return 1;
  if (tool === "rr_long" || tool === "rr_short" || tool === "bars-pattern" || tool === "sector" || tool === "rotated-rectangle" || tool === "arc" || tool === "curve" || tool === "triangle-shape") return 3;
  if (tool === "double-curve") return 4;
  if (tool === "anchored-vwap" || tool === "anchored-volume-profile") return 1;
  if (tool === "brush" || tool === "highlighter" || tool === "path" || tool === "polyline" || tool === "ghost-feed") return 0;
  return 2;
}

function pointAt(points: readonly ScreenPoint[], index: number): ScreenPoint | null {
  return points[index] ?? null;
}

function quadratic(start: ScreenPoint, control: ScreenPoint, end: ScreenPoint): ScreenPoint[] {
  return Array.from({ length: 25 }, (_, index) => {
    const t = index / 24;
    const inverse = 1 - t;
    return {
      x: inverse * inverse * start.x + 2 * inverse * t * control.x + t * t * end.x,
      y: inverse * inverse * start.y + 2 * inverse * t * control.y + t * t * end.y,
    };
  });
}

function cubic(start: ScreenPoint, first: ScreenPoint, second: ScreenPoint, end: ScreenPoint): ScreenPoint[] {
  return Array.from({ length: 33 }, (_, index) => {
    const t = index / 32;
    const inverse = 1 - t;
    return {
      x: inverse ** 3 * start.x + 3 * inverse ** 2 * t * first.x + 3 * inverse * t ** 2 * second.x + t ** 3 * end.x,
      y: inverse ** 3 * start.y + 3 * inverse ** 2 * t * first.y + 3 * inverse * t ** 2 * second.y + t ** 3 * end.y,
    };
  });
}

function arcThroughThreePoints(start: ScreenPoint, control: ScreenPoint, end: ScreenPoint): ScreenPoint[] {
  const determinant = 2 * (start.x * (control.y - end.y) + control.x * (end.y - start.y) + end.x * (start.y - control.y));
  if (Math.abs(determinant) < 0.001) return [start, control, end];
  const startSquared = start.x ** 2 + start.y ** 2;
  const controlSquared = control.x ** 2 + control.y ** 2;
  const endSquared = end.x ** 2 + end.y ** 2;
  const center = {
    x: (startSquared * (control.y - end.y) + controlSquared * (end.y - start.y) + endSquared * (start.y - control.y)) / determinant,
    y: (startSquared * (end.x - control.x) + controlSquared * (start.x - end.x) + endSquared * (control.x - start.x)) / determinant,
  };
  const startAngle = Math.atan2(start.y - center.y, start.x - center.x);
  const controlAngle = Math.atan2(control.y - center.y, control.x - center.x);
  const endAngle = Math.atan2(end.y - center.y, end.x - center.x);
  const positiveSweep = (angle: number): number => (angle + Math.PI * 2) % (Math.PI * 2);
  const controlSweep = positiveSweep(controlAngle - startAngle);
  const endSweep = positiveSweep(endAngle - startAngle);
  const sweep = controlSweep <= endSweep ? endSweep : endSweep - Math.PI * 2;
  const radius = Math.hypot(start.x - center.x, start.y - center.y);
  return Array.from({ length: 33 }, (_, index) => {
    const angle = startAngle + sweep * index / 32;
    return { x: center.x + Math.cos(angle) * radius, y: center.y + Math.sin(angle) * radius };
  });
}

export function buildGeometryPath(
  tool: GeometryToolId,
  anchors: readonly DrawingAnchor[],
  project: (anchor: DrawingAnchor) => ScreenPoint | null,
  options: GeometryPathOptions = {},
): GeometryPath | null {
  const points = anchors.map(project);
  if (points.some((point) => point === null)) return null;
  const screen = points.filter((point): point is ScreenPoint => point !== null);
  const first = pointAt(screen, 0);
  if (!first) return null;
  const second = pointAt(screen, 1);

  if (tool === "path") {
    return {
      points: screen,
      closed: false,
      arrowHead: false,
      startArrowHead: options.startArrow ?? false,
      endArrowHead: options.endArrow ?? true,
    };
  }
  if (tool === "brush" || tool === "highlighter" || tool === "polyline") {
    return { points: screen, closed: false, arrowHead: false };
  }
  if (tool === "arrow-mark-up" || tool === "arrow-mark-down") {
    const direction = tool === "arrow-mark-up" ? -1 : 1;
    return { points: [first, { x: first.x, y: first.y + direction * 20 }], closed: false, arrowHead: true };
  }
  if (!second) return null;
  if (tool === "arrow" || tool === "arrow-marker") return { points: [first, second], closed: false, arrowHead: true };
  if (tool === "ellipse" || tool === "circle") {
    const center = tool === "circle"
      ? first
      : { x: (first.x + second.x) / 2, y: (first.y + second.y) / 2 };
    const rx = tool === "circle" ? Math.hypot(second.x - first.x, second.y - first.y) : Math.abs(second.x - first.x) / 2;
    const ry = tool === "circle" ? rx : Math.abs(second.y - first.y) / 2;
    return {
      points: Array.from({ length: 49 }, (_, index) => {
        const angle = index / 48 * Math.PI * 2;
        return { x: center.x + Math.cos(angle) * rx, y: center.y + Math.sin(angle) * ry };
      }),
      closed: true,
      arrowHead: false,
    };
  }
  if (tool === "rotated-rectangle") {
    const third = pointAt(screen, 2);
    if (!third) return null;
    const dx = second.x - first.x;
    const dy = second.y - first.y;
    const length = Math.hypot(dx, dy);
    if (length === 0) return { points: [first, second, third, first], closed: true, arrowHead: false };
    const signedOffset = ((third.x - first.x) * -dy + (third.y - first.y) * dx) / length;
    const ox = -dy / length * signedOffset;
    const oy = dx / length * signedOffset;
    return { points: [first, second, { x: second.x + ox, y: second.y + oy }, { x: first.x + ox, y: first.y + oy }, first], closed: true, arrowHead: false };
  }
  if (tool === "triangle-shape") {
    const third = pointAt(screen, 2);
    return third ? { points: [first, second, third, first], closed: true, arrowHead: false } : null;
  }
  if (tool === "arc") {
    const control = pointAt(screen, 2);
    return control ? { points: arcThroughThreePoints(first, control, second), closed: false, arrowHead: false } : null;
  }
  if (tool === "curve") {
    const control = pointAt(screen, 2);
    return control ? { points: quadratic(first, control, second), closed: false, arrowHead: false } : null;
  }
  if (tool === "double-curve") {
    const control1 = pointAt(screen, 1);
    const control2 = pointAt(screen, 2);
    const end = pointAt(screen, 3);
    return control1 && control2 && end ? { points: cubic(first, control1, control2, end), closed: false, arrowHead: false } : null;
  }
  return { points: screen, closed: false, arrowHead: false };
}