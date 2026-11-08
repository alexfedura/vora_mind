import dagre from "dagre";
import { MarkerType, type Edge, type Node } from "@xyflow/react";

export interface GeneratedBoardNode {
  id: string;
  type: "voraCard" | "voraText";
  title: string;
  content: string;
}

export interface GeneratedBoardEdge {
  source: string;
  target: string;
  label?: string;
}

export interface GeneratedBoard {
  nodes: GeneratedBoardNode[];
  edges: GeneratedBoardEdge[];
}

const MAX_NODES = 100;
const MAX_EDGES = 250;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function parseGeneratedBoard(raw: string): GeneratedBoard {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (cause) {
    throw new Error(`DeepSeek returned invalid JSON: ${cause instanceof Error ? cause.message : String(cause)}`);
  }
  if (!isRecord(parsed) || !Array.isArray(parsed.nodes) || !Array.isArray(parsed.edges)) {
    throw new Error("Generated Board must contain nodes and edges arrays.");
  }
  if (parsed.nodes.length === 0 || parsed.nodes.length > MAX_NODES || parsed.edges.length > MAX_EDGES) {
    throw new Error(`Generated Board must contain 1-${MAX_NODES} nodes and no more than ${MAX_EDGES} edges.`);
  }

  const ids = new Set<string>();
  const nodes = parsed.nodes.map((rawNode, index): GeneratedBoardNode => {
    if (!isRecord(rawNode)) throw new Error(`Generated node ${index + 1} is invalid.`);
    const { id, type, title, content } = rawNode;
    if (
      typeof id !== "string" || !id.trim() || id.length > 120 ||
      (type !== "voraCard" && type !== "voraText") ||
      typeof content !== "string" || content.length > 20_000
    ) {
      throw new Error(`Generated node ${index + 1} has invalid id, type, or content.`);
    }
    if (ids.has(id)) throw new Error(`Generated Board contains duplicate node id "${id}".`);
    ids.add(id);
    const cleanTitle = typeof title === "string" && title.trim()
      ? title.trim().slice(0, 500)
      : content.trim().slice(0, 80) || `Idea ${index + 1}`;
    return { id, type, title: cleanTitle, content };
  });

  const edges = parsed.edges.map((rawEdge, index): GeneratedBoardEdge => {
    if (!isRecord(rawEdge) || typeof rawEdge.source !== "string" || typeof rawEdge.target !== "string") {
      throw new Error(`Generated edge ${index + 1} is invalid.`);
    }
    if (!ids.has(rawEdge.source) || !ids.has(rawEdge.target)) {
      throw new Error(`Generated edge ${index + 1} references a missing node.`);
    }
    if (rawEdge.label !== undefined && typeof rawEdge.label !== "string") {
      throw new Error(`Generated edge ${index + 1} has an invalid label.`);
    }
    return {
      source: rawEdge.source,
      target: rawEdge.target,
      ...(typeof rawEdge.label === "string" && rawEdge.label.trim()
        ? { label: rawEdge.label.trim().slice(0, 250) }
        : {}),
    };
  });
  return { nodes, edges };
}

export function applyDagreLayout(
  generatedNodes: GeneratedBoardNode[],
  generatedEdges: GeneratedBoardEdge[],
  origin: { x: number; y: number },
): { nodes: Node[]; edges: Edge[] } {
  const graph = new dagre.graphlib.Graph();
  graph.setDefaultEdgeLabel(() => ({}));
  graph.setGraph({
    rankdir: "LR",
    nodesep: 48,
    ranksep: 96,
    marginx: 24,
    marginy: 24,
  });

  for (const node of generatedNodes) {
    graph.setNode(node.id, {
      width: node.type === "voraCard" ? 320 : 240,
      height: node.type === "voraCard" ? 180 : 110,
    });
  }
  for (const edge of generatedEdges) graph.setEdge(edge.source, edge.target);
  dagre.layout(graph);

  const dimensions = graph.graph();
  const graphWidth = dimensions.width ?? 0;
  const graphHeight = dimensions.height ?? 0;
  const nodes: Node[] = generatedNodes.map((generated) => {
    const positioned = graph.node(generated.id) as { x: number; y: number; width: number; height: number };
    const position = {
      x: origin.x + positioned.x - graphWidth / 2,
      y: origin.y + positioned.y - graphHeight / 2,
    };
    return generated.type === "voraCard"
      ? {
        id: generated.id,
        type: "voraCard",
        position,
        width: positioned.width,
        data: {
          cardId: generated.id,
          title: generated.title,
          label: generated.title,
          content: generated.content,
          version: "v0.1.0",
        },
      }
      : {
        id: generated.id,
        type: "voraText",
        position,
        data: { text: generated.content, autoFocus: false },
      };
  });
  const edges: Edge[] = generatedEdges.map((edge) => ({
    id: `ai-edge-${crypto.randomUUID()}`,
    source: edge.source,
    target: edge.target,
    type: "default",
    ...(edge.label ? { label: edge.label } : {}),
    markerEnd: { type: MarkerType.ArrowClosed },
    style: { stroke: "#7c6ee6", strokeWidth: 2 },
  }));
  return { nodes, edges };
}
