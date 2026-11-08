import dagre from "dagre";
import { MarkerType, type Edge, type Node } from "@xyflow/react";

interface GeneratedNodeInput {
  id: string;
  type: "voraCard" | "voraText";
  title: string;
  content: string;
}

interface GeneratedEdgeInput {
  source: string;
  target: string;
  label?: string;
}

interface GeneratedBoardInput {
  nodes: GeneratedNodeInput[];
  edges: GeneratedEdgeInput[];
}

export interface GeneratedBoardGraph {
  nodes: Node[];
  edges: Edge[];
}

const MAX_NODES = 60;
const MAX_EDGES = 120;
const MAX_TEXT_LENGTH = 12_000;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function parseGeneratedBoard(raw: string): GeneratedBoardInput {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (error) {
    throw new Error(`DeepSeek returned invalid JSON: ${error instanceof Error ? error.message : String(error)}`);
  }
  if (!isRecord(parsed) || !Array.isArray(parsed.nodes) || !Array.isArray(parsed.edges)) {
    throw new Error("Generated graph must contain node and edge arrays.");
  }
  if (parsed.nodes.length === 0 || parsed.nodes.length > MAX_NODES) {
    throw new Error(`Generated graph must contain between 1 and ${MAX_NODES} nodes.`);
  }
  if (parsed.edges.length > MAX_EDGES) {
    throw new Error(`Generated graph cannot contain more than ${MAX_EDGES} edges.`);
  }

  const seenIds = new Set<string>();
  const nodes = parsed.nodes.map((item, index): GeneratedNodeInput => {
    if (!isRecord(item)) throw new Error(`Generated node ${index + 1} is invalid.`);
    const id = typeof item.id === "string" ? item.id.trim() : "";
    const type = item.type;
    const title = typeof item.title === "string" ? item.title.trim() : "";
    const content = typeof item.content === "string" ? item.content : "";
    if (!id || id.length > 128 || seenIds.has(id)) {
      throw new Error(`Generated node ${index + 1} has a missing, duplicate, or invalid ID.`);
    }
    if (type !== "voraCard" && type !== "voraText") {
      throw new Error(`Generated node ${index + 1} has an unsupported type.`);
    }
    if (!title || title.length > 500 || content.length > MAX_TEXT_LENGTH) {
      throw new Error(`Generated node ${index + 1} has an empty title or exceeds the content limit.`);
    }
    seenIds.add(id);
    return { id, type, title, content };
  });

  const edges = parsed.edges.map((item, index): GeneratedEdgeInput => {
    if (!isRecord(item)) throw new Error(`Generated edge ${index + 1} is invalid.`);
    const source = typeof item.source === "string" ? item.source.trim() : "";
    const target = typeof item.target === "string" ? item.target.trim() : "";
    const label = typeof item.label === "string" ? item.label.trim() : undefined;
    if (!seenIds.has(source) || !seenIds.has(target) || source === target) {
      throw new Error(`Generated edge ${index + 1} references an invalid node.`);
    }
    if (label && label.length > 200) {
      throw new Error(`Generated edge ${index + 1} label exceeds the 200-character limit.`);
    }
    return { source, target, label };
  });

  return { nodes, edges };
}

export function applyDagreLayout(
  raw: string,
  createCard: (title: string, content: string) => string,
): GeneratedBoardGraph {
  const generated = parseGeneratedBoard(raw);
  const layout = new dagre.graphlib.Graph();
  layout.setDefaultEdgeLabel(() => ({}));
  layout.setGraph({
    rankdir: "LR",
    nodesep: 64,
    ranksep: 100,
    marginx: 48,
    marginy: 48,
  });

  const dimensions = new Map<string, { width: number; height: number }>();
  for (const node of generated.nodes) {
    const width = node.type === "voraCard" ? 352 : 320;
    const height = node.type === "voraCard"
      ? 220
      : Math.max(60, Math.min(560, Math.ceil(node.content.length / 44) * 24 + 36));
    dimensions.set(node.id, { width, height });
    layout.setNode(node.id, { width, height });
  }
  for (const edge of generated.edges) layout.setEdge(edge.source, edge.target);
  dagre.layout(layout);

  const localIds = new Map<string, string>();
  const nodes = generated.nodes.map((item): Node => {
    const size = dimensions.get(item.id);
    const positioned = layout.node(item.id);
    if (!size || !positioned) {
      throw new Error(`Could not calculate a position for generated node "${item.title}".`);
    }
    const position = {
      x: positioned.x - size.width / 2,
      y: positioned.y - size.height / 2,
    };
    if (item.type === "voraCard") {
      const id = createCard(item.title, item.content);
      localIds.set(item.id, id);
      return {
        id,
        type: "voraCard",
        position,
        width: size.width,
        selected: true,
        data: {
          label: item.title,
          title: item.title,
          content: item.content,
          cardId: id,
          version: "v0.1.0",
        },
      };
    }
    const id = `text-${crypto.randomUUID()}`;
    localIds.set(item.id, id);
    return {
      id,
      type: "voraText",
      position,
      selected: true,
      data: { text: `${item.title}\n${item.content}` },
    };
  });

  const edges = generated.edges.map((item): Edge => {
    const source = localIds.get(item.source);
    const target = localIds.get(item.target);
    if (!source || !target) {
      throw new Error("Could not resolve a generated edge endpoint.");
    }
    return {
      id: `edge-${crypto.randomUUID()}`,
      source,
      target,
      type: "default",
      label: item.label,
      markerEnd: { type: MarkerType.ArrowClosed },
      style: { stroke: "#4b5563", strokeWidth: 2 },
    };
  });

  return { nodes, edges };
}
