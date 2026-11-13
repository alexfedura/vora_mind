import type { VoraShapeData } from "@/mind/types";

export function isVoraDiagramShape(value: unknown): value is VoraShapeData["shape"] {
  return value === "rectangle" ||
    value === "path" ||
    value === "freehand" ||
    value === "process" ||
    value === "decision" ||
    value === "terminator" ||
    value === "inputOutput" ||
    value === "document" ||
    value === "database";
}

export function isVoraPathLikeShape(value: unknown): value is "path" | "freehand" {
  return value === "path" || value === "freehand";
}

export function getVoraFreehandPath(points: readonly VoraShapeData["points"][number][]): string {
  const [first, ...remaining] = points;
  if (!first) return "";
  if (remaining.length === 0) return `M ${first.x} ${first.y} l 0.01 0`;

  let path = `M ${first.x} ${first.y}`;
  for (let index = 0; index < points.length - 1; index += 1) {
    const start = points[index];
    const end = points[index + 1];
    if (!start || !end) continue;
    const previous = points[index - 1] ?? start;
    const next = points[index + 2] ?? end;
    const control1 = {
      x: start.x + (end.x - previous.x) / 6,
      y: start.y + (end.y - previous.y) / 6,
    };
    const control2 = {
      x: end.x - (next.x - start.x) / 6,
      y: end.y - (next.y - start.y) / 6,
    };
    path += ` C ${control1.x} ${control1.y} ${control2.x} ${control2.y} ${end.x} ${end.y}`;
  }
  return path;
}

export function getVoraShapePath(
  shape: VoraShapeData["shape"],
  width: number,
  height: number,
  inset: number,
): string {
  const left = inset;
  const top = inset;
  const right = Math.max(left, width - inset);
  const bottom = Math.max(top, height - inset);
  const centerX = width / 2;
  const centerY = height / 2;

  switch (shape) {
    case "rectangle":
      return `M ${left} ${top} H ${right} V ${bottom} H ${left} Z`;
    case "process":
      return `M ${left + 6} ${top} H ${right - 6} Q ${right} ${top} ${right} ${top + 6} V ${bottom - 6} Q ${right} ${bottom} ${right - 6} ${bottom} H ${left + 6} Q ${left} ${bottom} ${left} ${bottom - 6} V ${top + 6} Q ${left} ${top} ${left + 6} ${top} Z`;
    case "decision":
      return `M ${centerX} ${top} L ${right} ${centerY} L ${centerX} ${bottom} L ${left} ${centerY} Z`;
    case "terminator": {
      const radiusX = (right - left) / 2;
      const radiusY = (bottom - top) / 2;
      return `M ${centerX - radiusX} ${centerY} A ${radiusX} ${radiusY} 0 1 0 ${centerX + radiusX} ${centerY} A ${radiusX} ${radiusY} 0 1 0 ${centerX - radiusX} ${centerY} Z`;
    }
    case "inputOutput": {
      const slant = Math.min(24, (right - left) / 5);
      return `M ${left + slant} ${top} H ${right} L ${right - slant} ${bottom} H ${left} Z`;
    }
    case "document": {
      const wave = Math.min(14, (bottom - top) / 6);
      return `M ${left} ${top} H ${right} V ${bottom - wave} C ${right - 12} ${bottom - wave * 2} ${left + 12} ${bottom} ${left} ${bottom - wave} Z`;
    }
    case "database": {
      const radius = Math.max(8, Math.min(20, (bottom - top) / 6));
      const ellipseCenterY = top + radius;
      return `M ${left} ${ellipseCenterY} A ${(right - left) / 2} ${radius} 0 0 1 ${right} ${ellipseCenterY} V ${bottom - radius} A ${(right - left) / 2} ${radius} 0 0 1 ${left} ${bottom - radius} Z M ${left} ${centerY} A ${(right - left) / 2} ${radius} 0 0 0 ${right} ${centerY}`;
    }
    case "path":
    case "freehand":
      return "";
  }
}
