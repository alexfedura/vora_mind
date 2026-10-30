import type { Edge, Node } from "@xyflow/react";
import type { VoraShapeData, VoraShapeDefaults } from "@/mind/types";

export type FlowchartTemplateId = "basic-process" | "decision-branch";

export const FLOWCHART_TEMPLATES: ReadonlyArray<{
  id: FlowchartTemplateId;
  title: string;
  description: string;
}> = [
  {
    id: "basic-process",
    title: "Basic process",
    description: "Start → process → end",
  },
  {
    id: "decision-branch",
    title: "Decision branch",
    description: "A decision with two editable outcomes",
  },
];

interface TemplateShape {
  shape: VoraShapeData["shape"];
  label: string;
  x: number;
  y: number;
}

function createShapeNode(
  boardId: string,
  shape: TemplateShape,
  center: { x: number; y: number },
  defaults: VoraShapeDefaults,
): Node {
  const width = 180;
  const height = 100;
  const id = `vora-node-${boardId}-${crypto.randomUUID()}`;
  const data: VoraShapeData = {
    shape: shape.shape,
    label: shape.label,
    ...defaults[shape.shape],
    points: [],
    segments: [],
  };
  return {
    id,
    type: "voraShape",
    className: "vora-shape-border-hit",
    position: {
      x: center.x + shape.x - width / 2,
      y: center.y + shape.y - height / 2,
    },
    width,
    height,
    data,
    selected: true,
  };
}

export function createFlowchartTemplate(
  templateId: FlowchartTemplateId,
  boardId: string,
  center: { x: number; y: number },
  defaults: VoraShapeDefaults,
): { nodes: Node[]; edges: Edge[] } {
  const shapes: TemplateShape[] = templateId === "basic-process"
    ? [
      { shape: "terminator", label: "Start", x: 0, y: -150 },
      { shape: "process", label: "Process", x: 0, y: 0 },
      { shape: "terminator", label: "End", x: 0, y: 150 },
    ]
    : [
      { shape: "terminator", label: "Start", x: 0, y: -210 },
      { shape: "decision", label: "Decision", x: 0, y: -60 },
      { shape: "process", label: "Yes", x: -180, y: 100 },
      { shape: "process", label: "No", x: 180, y: 100 },
    ];
  const nodes = shapes.map((shape) => createShapeNode(boardId, shape, center, defaults));
  const connections = templateId === "basic-process"
    ? [[0, 1], [1, 2]]
    : [[0, 1], [1, 2], [1, 3]];
  const edges = connections.flatMap(([sourceIndex, targetIndex]): Edge[] => {
    const source = nodes[sourceIndex];
    const target = nodes[targetIndex];
    return source && target
      ? [{
        id: `vora-edge-${boardId}-${crypto.randomUUID()}`,
        source: source.id,
        target: target.id,
        type: "default",
        selected: true,
      }]
      : [];
  });
  return { nodes, edges };
}
