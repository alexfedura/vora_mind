import type { Node } from "@xyflow/react";
import type { VoraPathSegment, VoraShapeData, VoraShapePoint } from "@/mind/types";

export type VoraPathNode = Node<VoraShapeData, "voraShape">;

const MIN_PATH_SIZE = 36;
const JOIN_TOLERANCE = 12;

export function getPathSegments(
  nodeId: string,
  data: VoraShapeData,
): VoraPathSegment[] {
  const count = Math.max(0, data.points.length - 1);
  return Array.from({ length: count }, (_, index) => {
    const saved = data.segments?.[index];
    return saved && typeof saved.id === "string" && saved.id.length > 0
      ? {
        ...saved,
        name: saved.name.trim() || `Section ${index + 1}`,
        details: typeof saved.details === "string" ? saved.details : "",
        color: /^#[0-9a-fA-F]{6}$/.test(saved.color) ? saved.color : data.color,
      }
      : {
        id: `path-segment-${nodeId}-${index}`,
        name: `Section ${index + 1}`,
        details: "",
        color: data.color,
      };
  });
}

function getWorldPoints(node: VoraPathNode): VoraShapePoint[] {
  const width = node.measured?.width ?? node.width ?? MIN_PATH_SIZE;
  const height = node.measured?.height ?? node.height ?? MIN_PATH_SIZE;
  return node.data.points.map((point) => ({
    x: node.position.x + point.x * width,
    y: node.position.y + point.y * height,
  }));
}

function createPathChunk(
  source: VoraPathNode,
  worldPoints: VoraShapePoint[],
  segments: VoraPathSegment[],
  suffix: string,
  startArrow: boolean,
  endArrow: boolean,
): VoraPathNode {
  const minX = Math.min(...worldPoints.map((point) => point.x));
  const minY = Math.min(...worldPoints.map((point) => point.y));
  const maxX = Math.max(...worldPoints.map((point) => point.x));
  const maxY = Math.max(...worldPoints.map((point) => point.y));
  const spanX = maxX - minX;
  const spanY = maxY - minY;
  const width = Math.max(MIN_PATH_SIZE, spanX);
  const height = Math.max(MIN_PATH_SIZE, spanY);
  const position = {
    x: minX - (width - spanX) / 2,
    y: minY - (height - spanY) / 2,
  };
  const points = worldPoints.map((point) => ({
    x: (point.x - position.x) / width,
    y: (point.y - position.y) / height,
  }));
  return {
    ...source,
    id: `${source.id}-${suffix}-${crypto.randomUUID()}`,
    position,
    width,
    height,
    selected: true,
    data: {
      ...source.data,
      label: segments.length === 1 ? segments[0]?.name ?? source.data.label : source.data.label,
      points,
      segments,
      startArrow,
      endArrow,
    },
  };
}

export function cutPathSegment(
  node: VoraPathNode,
  segmentIndex: number,
): VoraPathNode[] | null {
  const segmentCount = node.data.points.length - 1;
  if (segmentIndex < 0 || segmentIndex >= segmentCount || segmentCount < 2) return null;

  const worldPoints = getWorldPoints(node);
  const segments = getPathSegments(node.id, node.data);
  const chunks: VoraPathNode[] = [];
  const ranges = [
    { start: 0, end: segmentIndex - 1 },
    { start: segmentIndex, end: segmentIndex },
    { start: segmentIndex + 1, end: segmentCount - 1 },
  ];

  ranges.forEach(({ start, end }, index) => {
    if (start > end) return;
    chunks.push(createPathChunk(
      node,
      worldPoints.slice(start, end + 2),
      segments.slice(start, end + 1),
      `cut-${index}`,
      start === 0 && node.data.startArrow,
      end === segmentCount - 1 && node.data.endArrow,
    ));
  });
  return chunks;
}

function getEndpointPairs(node: VoraPathNode): [VoraShapePoint, VoraShapePoint] {
  const points = getWorldPoints(node);
  const first = points[0];
  const last = points[points.length - 1];
  return [first ?? node.position, last ?? node.position];
}

export function canJoinPathNodes(nodes: VoraPathNode[]): boolean {
  if (nodes.length !== 2 || nodes.some((node) => node.data.shape !== "path" || node.data.points.length < 2)) {
    return false;
  }
  const firstNode = nodes[0];
  const secondNode = nodes[1];
  if (!firstNode || !secondNode) return false;
  const [firstStart, firstEnd] = getEndpointPairs(firstNode);
  const [secondStart, secondEnd] = getEndpointPairs(secondNode);
  return [
    [firstStart, secondStart],
    [firstStart, secondEnd],
    [firstEnd, secondStart],
    [firstEnd, secondEnd],
  ].some(([first, second]) =>
    first !== undefined &&
    second !== undefined &&
    Math.hypot(first.x - second.x, first.y - second.y) <= JOIN_TOLERANCE
  );
}

export function joinPathNodes(nodes: VoraPathNode[]): VoraPathNode | null {
  if (!canJoinPathNodes(nodes)) return null;
  const firstNode = nodes[0];
  const secondNode = nodes[1];
  if (!firstNode || !secondNode) return null;
  const [firstStart, firstEnd] = getEndpointPairs(firstNode);
  const [secondStart, secondEnd] = getEndpointPairs(secondNode);
  const firstPoints = getWorldPoints(firstNode);
  const secondPoints = getWorldPoints(secondNode);
  let points: VoraShapePoint[];
  let segments: VoraPathSegment[];
  let startArrow: boolean;
  let endArrow: boolean;

  if (Math.hypot(firstEnd.x - secondStart.x, firstEnd.y - secondStart.y) <= JOIN_TOLERANCE) {
    points = [...firstPoints, ...secondPoints.slice(1)];
    segments = [...getPathSegments(firstNode.id, firstNode.data), ...getPathSegments(secondNode.id, secondNode.data)];
    startArrow = firstNode.data.startArrow;
    endArrow = secondNode.data.endArrow;
  } else if (Math.hypot(firstEnd.x - secondEnd.x, firstEnd.y - secondEnd.y) <= JOIN_TOLERANCE) {
    points = [...firstPoints, ...secondPoints.slice(0, -1).reverse()];
    segments = [
      ...getPathSegments(firstNode.id, firstNode.data),
      ...getPathSegments(secondNode.id, secondNode.data).reverse(),
    ];
    startArrow = firstNode.data.startArrow;
    endArrow = secondNode.data.startArrow;
  } else if (Math.hypot(firstStart.x - secondEnd.x, firstStart.y - secondEnd.y) <= JOIN_TOLERANCE) {
    points = [...secondPoints, ...firstPoints.slice(1)];
    segments = [...getPathSegments(secondNode.id, secondNode.data), ...getPathSegments(firstNode.id, firstNode.data)];
    startArrow = secondNode.data.startArrow;
    endArrow = firstNode.data.endArrow;
  } else {
    points = [...secondPoints.reverse(), ...firstPoints.slice(1)];
    segments = [
      ...getPathSegments(secondNode.id, secondNode.data).reverse(),
      ...getPathSegments(firstNode.id, firstNode.data),
    ];
    startArrow = secondNode.data.endArrow;
    endArrow = firstNode.data.endArrow;
  }

  return createPathChunk(firstNode, points, segments, "joined", startArrow, endArrow);
}

export function makeSingleSegmentNode(
  node: VoraPathNode,
  segmentIndex: number,
): VoraPathNode | null {
  if (segmentIndex < 0 || segmentIndex >= node.data.points.length - 1) return null;
  const points = getWorldPoints(node);
  const segments = getPathSegments(node.id, node.data);
  const segment = segments[segmentIndex];
  if (!segment) return null;
  return createPathChunk(
    node,
    points.slice(segmentIndex, segmentIndex + 2),
    [segment],
    "library",
    false,
    false,
  );
}
