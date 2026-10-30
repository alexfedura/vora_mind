import type { Edge, Node } from "@xyflow/react";
import type {
  VoraBoardTransfer,
  VoraBoardTransferEdge,
  VoraBoardTransferNode,
  VoraCard,
} from "@/mind/types";
import { isVoraDiagramShape } from "@/mind/utils/shapeGeometry";

const SUPPORTED_NODE_TYPES = new Set(["voraCard", "voraText", "voraShape", "forgeAst"]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isColor(value: unknown): value is string {
  return typeof value === "string" && /^#[\da-f]{6}$/i.test(value);
}

function isTransferNode(value: unknown): value is VoraBoardTransferNode {
  if (!isRecord(value) ||
    typeof value.id !== "string" ||
    typeof value.type !== "string" ||
    !SUPPORTED_NODE_TYPES.has(value.type) ||
    !isRecord(value.position) ||
    !isFiniteNumber(value.position.x) ||
    !isFiniteNumber(value.position.y) ||
    !isRecord(value.data)
  ) return false;

  if (value.width !== undefined && (!isFiniteNumber(value.width) || value.width <= 0)) return false;
  if (value.height !== undefined && (!isFiniteNumber(value.height) || value.height <= 0)) return false;
  if (value.draggable !== undefined && typeof value.draggable !== "boolean") return false;
  if (value.style !== undefined && (!isRecord(value.style) ||
    !Object.values(value.style).every((item) => typeof item === "string" || typeof item === "number")
  )) return false;

  if (value.type === "voraShape") {
    const data = value.data;
    if (!isVoraDiagramShape(data.shape) ||
      typeof data.label !== "string" ||
      !isColor(data.color) ||
      !isFiniteNumber(data.lineOpacity) || data.lineOpacity < 0 || data.lineOpacity > 1 ||
      !isFiniteNumber(data.lineWidth) || data.lineWidth < 1 || data.lineWidth > 4 ||
      !(data.lineStyle === "solid" || data.lineStyle === "dashed" || data.lineStyle === "dotted") ||
      !isColor(data.textColor) ||
      !isFiniteNumber(data.textOpacity) || data.textOpacity < 0 || data.textOpacity > 1 ||
      !isColor(data.fillColor) ||
      !isFiniteNumber(data.fillOpacity) || data.fillOpacity < 0 || data.fillOpacity > 1 ||
      !Array.isArray(data.points) ||
      !data.points.every((point) => isRecord(point) &&
        isFiniteNumber(point.x) && point.x >= 0 && point.x <= 1 &&
        isFiniteNumber(point.y) && point.y >= 0 && point.y <= 1) ||
      typeof data.startArrow !== "boolean" ||
      typeof data.endArrow !== "boolean"
    ) return false;
  }

  return true;
}

function isTransferNodeType(value: string): value is VoraBoardTransferNode["type"] {
  return value === "voraCard" ||
    value === "voraText" ||
    value === "voraShape" ||
    value === "forgeAst";
}

function isTransferEdge(value: unknown): value is VoraBoardTransferEdge {
  return isRecord(value) &&
    typeof value.id === "string" &&
    typeof value.source === "string" &&
    typeof value.target === "string";
}

function isTransferCard(value: unknown): value is VoraBoardTransfer["cards"][number] {
  return isRecord(value) &&
    typeof value.id === "string" &&
    typeof value.title === "string" &&
    typeof value.content === "string" &&
    typeof value.version === "string";
}

function isBoardTransfer(value: unknown): value is VoraBoardTransfer {
  if (!isRecord(value) ||
    value.format !== "vora-mind-board" ||
    value.version !== 1 ||
    typeof value.boardName !== "string" ||
    typeof value.exportedAt !== "string" ||
    !Array.isArray(value.nodes) ||
    !value.nodes.every(isTransferNode) ||
    !Array.isArray(value.edges) ||
    !value.edges.every(isTransferEdge) ||
    !Array.isArray(value.cards) ||
    !value.cards.every(isTransferCard)
  ) {
    return false;
  }
  return true;
}

export function parseBoardTransfer(value: unknown): VoraBoardTransfer {
  if (!isBoardTransfer(value)) {
    throw new Error("This file is not a valid Vora Mind Board export.");
  }

  const nodeIds = new Set(value.nodes.map((node) => node.id));
  if (nodeIds.size !== value.nodes.length) {
    throw new Error("The Board export contains duplicate node IDs.");
  }
  if (value.edges.some((edge) => !nodeIds.has(edge.source) || !nodeIds.has(edge.target))) {
    throw new Error("The Board export contains a connection to a missing node.");
  }
  const cardIds = new Set(value.cards.map((card) => card.id));
  if (cardIds.size !== value.cards.length) {
    throw new Error("The Board export contains duplicate card IDs.");
  }
  for (const node of value.nodes) {
    if (node.type !== "voraCard") continue;
    const cardId = typeof node.data.cardId === "string" ? node.data.cardId : node.id;
    if (!cardIds.has(cardId)) {
      throw new Error("The Board export is missing data for one of its cards.");
    }
  }

  return value;
}

export function createBoardTransfer(
  boardName: string,
  nodes: Node[],
  edges: Edge[],
  cards: Record<string, VoraCard>,
): VoraBoardTransfer {
  const transferNodes = nodes.map((node): VoraBoardTransferNode => {
    if (!node.type || !isTransferNodeType(node.type) || !isRecord(node.data)) {
      throw new Error(`Cannot export unsupported canvas node type "${node.type ?? "unknown"}".`);
    }
    const type = node.type;
    const position = { x: node.position.x, y: node.position.y };
    const transferNode: VoraBoardTransferNode = {
      id: node.id,
      type,
      position,
      data: structuredClone(node.data),
    };
    const width = node.measured?.width ?? node.width;
    const height = node.measured?.height ?? node.height;
    if (width !== undefined && Number.isFinite(width) && width > 0) transferNode.width = width;
    if (height !== undefined && Number.isFinite(height) && height > 0) transferNode.height = height;
    if (node.draggable !== undefined) transferNode.draggable = node.draggable;
    if (node.style) {
      const style: Record<string, string | number> = {};
      for (const [key, styleValue] of Object.entries(node.style)) {
        if (typeof styleValue === "string" || typeof styleValue === "number") {
          style[key] = styleValue;
        }
      }
      transferNode.style = style;
    }
    return transferNode;
  });
  const transferEdges = edges.map((edge): VoraBoardTransferEdge => ({
    id: edge.id,
    source: edge.source,
    target: edge.target,
  }));
  const transferCards = Object.values(cards).map((card) => ({
    id: card.id,
    title: card.title,
    content: card.content,
    version: card.version ?? "v0.1.0",
  }));

  return {
    format: "vora-mind-board",
    version: 1,
    boardName,
    exportedAt: new Date().toISOString(),
    nodes: transferNodes,
    edges: transferEdges,
    cards: transferCards,
  };
}

export function remapBoardTransfer(transfer: VoraBoardTransfer, boardId: string): {
  nodes: Node[];
  edges: Edge[];
  cards: Record<string, VoraCard>;
} {
  const nodeIds = new Map(transfer.nodes.map((node) => [node.id, `vora-node-${crypto.randomUUID()}`]));
  const cardIds = new Map(transfer.cards.map((card) => [card.id, `vora-card-${crypto.randomUUID()}`]));
  const groupIds = new Map<string, string>();
  for (const node of transfer.nodes) {
    const groupId = node.data.canvasGroupId;
    if (typeof groupId === "string" && !groupIds.has(groupId)) {
      groupIds.set(groupId, `vora-group-${crypto.randomUUID()}`);
    }
  }

  const cards: Record<string, VoraCard> = {};
  for (const card of transfer.cards) {
    const id = cardIds.get(card.id);
    if (!id) throw new Error("Could not assign a new ID to an imported card.");
    const timestamp = Date.now();
    cards[id] = {
      id,
      title: card.title,
      content: card.content,
      version: card.version,
      boardId,
      created_at: timestamp,
      updated_at: timestamp,
    };
  }

  const nodes: Node[] = transfer.nodes.map((item) => {
    const id = nodeIds.get(item.id);
    if (!id) throw new Error("Could not assign a new ID to an imported canvas item.");
    const data = structuredClone(item.data);
    if (typeof data.canvasGroupId === "string") {
      data.canvasGroupId = groupIds.get(data.canvasGroupId) ?? data.canvasGroupId;
    }
    if (item.type === "voraCard") {
      const originalCardId = typeof data.cardId === "string" ? data.cardId : item.id;
      const newCardId = cardIds.get(originalCardId);
      if (!newCardId) throw new Error("Could not reconnect an imported card to its canvas item.");
      const card = cards[newCardId];
      data.cardId = newCardId;
      data.title = card?.title ?? "";
      data.label = card?.title ?? "";
      data.content = card?.content ?? "";
      data.version = card?.version ?? "v0.1.0";
    }
    const node: Node = {
      id,
      type: item.type,
      position: { ...item.position },
      data,
      selected: false,
      className: item.type === "voraShape"
        ? data.shape === "path" ? "vora-path-node vora-shape-border-hit" : "vora-shape-border-hit"
        : undefined,
      draggable: item.draggable,
      style: item.style,
    };
    if (item.width !== undefined) node.width = item.width;
    if (item.height !== undefined) node.height = item.height;
    return node;
  });

  const edges: Edge[] = transfer.edges.map((item) => {
    const source = nodeIds.get(item.source);
    const target = nodeIds.get(item.target);
    if (!source || !target) throw new Error("Could not reconnect an imported canvas connection.");
    return {
      id: `vora-edge-${crypto.randomUUID()}`,
      source,
      target,
      type: "default",
      selected: false,
    };
  });

  return { nodes, edges, cards };
}
