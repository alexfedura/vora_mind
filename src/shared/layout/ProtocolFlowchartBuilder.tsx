import React, { useEffect, useMemo, useState } from "react";
import {
  addEdge,
  Background,
  Controls,
  Handle,
  Position,
  ReactFlow,
  useEdgesState,
  useNodesState,
} from "@xyflow/react";
import type { Connection, Edge, Node, NodeProps } from "@xyflow/react";
import type { Hypothesis, ProtocolNodeKind } from "@/types";
import { useAppStore } from "@/shared/store/useAppStore";

interface ProtocolNodeData extends Record<string, unknown> {
  label: string;
  kind: ProtocolNodeKind;
  completed?: boolean;
  readOnly?: boolean;
  disabled?: boolean;
  onToggle?: (nodeId: string, completed: boolean) => void;
}

type ProtocolNode = Node<ProtocolNodeData, "protocol">;

interface ProtocolGraph {
  nodes: ProtocolNode[];
  edges: Edge[];
}

interface ProtocolFlowchartBuilderProps {
  hypothesis: Hypothesis;
  onUpdated: (hypothesis: Hypothesis) => void;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isProtocolNodeKind(value: unknown): value is ProtocolNodeKind {
  return value === "step" || value === "decision" || value === "start-end";
}

function parseProtocolGraph(graphJson: string | null | undefined): ProtocolGraph {
  if (!graphJson) return { nodes: [], edges: [] };
  const parsed: unknown = JSON.parse(graphJson);
  if (!isRecord(parsed) || !Array.isArray(parsed.nodes) || !Array.isArray(parsed.edges)) {
    throw new Error("The saved protocol flowchart is not a valid graph.");
  }
  const nodes: ProtocolNode[] = [];
  for (const value of parsed.nodes) {
    if (!isRecord(value) || typeof value.id !== "string" || !isRecord(value.position)
      || typeof value.position.x !== "number" || typeof value.position.y !== "number"
      || !isRecord(value.data) || typeof value.data.label !== "string"
      || !isProtocolNodeKind(value.data.kind)) {
      throw new Error("The saved protocol flowchart contains an invalid node.");
    }
    nodes.push({
      id: value.id,
      type: "protocol",
      position: { x: value.position.x, y: value.position.y },
      data: { label: value.data.label, kind: value.data.kind },
    });
  }
  const edges: Edge[] = [];
  for (const value of parsed.edges) {
    if (!isRecord(value) || typeof value.id !== "string"
      || typeof value.source !== "string" || typeof value.target !== "string") {
      throw new Error("The saved protocol flowchart contains an invalid connection.");
    }
    edges.push({ id: value.id, source: value.source, target: value.target });
  }
  return { nodes, edges };
}

function parseCompletedNodeIds(value: string | null | undefined): string[] {
  if (!value) return [];
  const parsed: unknown = JSON.parse(value);
  if (!Array.isArray(parsed) || !parsed.every((item) => typeof item === "string")) {
    throw new Error("Saved protocol progress is not a list of node IDs.");
  }
  return parsed;
}

function ProtocolNodeView({ id, data, selected }: NodeProps<ProtocolNode>): React.ReactElement {
  const isStartEnd = data.kind === "start-end";
  const isDecision = data.kind === "decision";
  return (
    <div
      dir="auto"
      className={`min-w-40 rounded-xl border bg-[#17191f] px-3 py-2.5 shadow-lg ${
        selected ? "border-amber-300" : isDecision ? "border-violet-400/70" : "border-sky-400/60"
      } ${isStartEnd ? "rounded-full" : ""}`}
    >
      <Handle type="target" position={Position.Top} className="!bg-amber-300" />
      <span className="block text-[10px] font-semibold uppercase tracking-wide text-gray-400">
        {isDecision ? "Decision" : isStartEnd ? "Start / End" : "Step"}
      </span>
      <p dir="auto" className="mt-1 whitespace-pre-wrap text-sm text-gray-100">
        {data.label}
      </p>
      {data.readOnly && (
        <label dir="auto" className="mt-2 flex items-center gap-2 text-xs text-gray-300">
          <input
            dir="auto"
            type="checkbox"
            checked={data.completed ?? false}
            disabled={data.disabled}
            onChange={(event) => data.onToggle?.(id, event.currentTarget.checked)}
            className="accent-amber-400"
          />
          Done
        </label>
      )}
      <Handle type="source" position={Position.Bottom} className="!bg-amber-300" />
    </div>
  );
}

const nodeTypes = { protocol: ProtocolNodeView };

export const ProtocolFlowchartBuilder: React.FC<ProtocolFlowchartBuilderProps> = ({
  hypothesis,
  onUpdated,
}) => {
  const saveProtocolFlowchart = useAppStore((state) => state.saveProtocolFlowchart);
  const updateProtocolCompletion = useAppStore((state) => state.updateProtocolCompletion);
  const [nodes, setNodes, onNodesChange] = useNodesState<ProtocolNode>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [isUpdatingCompletion, setIsUpdatingCompletion] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const isReadOnly = hypothesis.protocolLockedAt != null;

  const graphReadError = useMemo(() => {
    try {
      return parseProtocolGraph(hypothesis.protocolFlowchartJson).nodes.length > 0
        ? null
        : isReadOnly ? "The locked flowchart protocol contains no nodes." : null;
    } catch (cause) {
      return cause instanceof Error ? cause.message : String(cause);
    }
  }, [hypothesis.protocolFlowchartJson, isReadOnly]);

  const completedIdsResult = useMemo(() => {
    try {
      return { ids: parseCompletedNodeIds(hypothesis.protocolCompletedSteps), error: null };
    } catch (cause) {
      return {
        ids: [] as string[],
        error: cause instanceof Error ? cause.message : String(cause),
      };
    }
  }, [hypothesis.protocolCompletedSteps]);

  useEffect(() => {
    try {
      const graph = parseProtocolGraph(hypothesis.protocolFlowchartJson);
      setNodes(graph.nodes);
      setEdges(graph.edges);
      setSelectedNodeId(null);
      setError(null);
    } catch (cause) {
      setNodes([]);
      setEdges([]);
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, [hypothesis.id, hypothesis.protocolFlowchartJson, setEdges, setNodes]);

  const selectedNode = nodes.find((node) => node.id === selectedNodeId);
  const completedIds = new Set(completedIdsResult.ids);
  const visibleNodes = useMemo(
    () => nodes.map((node) => ({
      ...node,
      data: {
        ...node.data,
        readOnly: isReadOnly,
        completed: completedIds.has(node.id),
        disabled: isUpdatingCompletion || completedIdsResult.error !== null,
        onToggle: isReadOnly
          ? (nodeId: string, completed: boolean) => void handleToggleCompleted(nodeId, completed)
          : undefined,
      },
    })),
    [nodes, isReadOnly, completedIdsResult, isUpdatingCompletion],
  );
  const progressCount = nodes.filter((node) => completedIds.has(node.id)).length;
  const progressPercent = nodes.length > 0 ? (progressCount / nodes.length) * 100 : 0;

  const handleToggleCompleted = async (nodeId: string, completed: boolean): Promise<void> => {
    if (isUpdatingCompletion) return;
    const updatedIds = new Set(completedIdsResult.ids);
    if (completed) updatedIds.add(nodeId);
    else updatedIds.delete(nodeId);
    setIsUpdatingCompletion(true);
    setError(null);
    try {
      const updated = await updateProtocolCompletion(
        hypothesis.id,
        JSON.stringify([...updatedIds]),
      );
      onUpdated(updated);
    } catch (cause) {
      console.error("Could not update protocol completion:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsUpdatingCompletion(false);
    }
  };

  const addProtocolNode = (kind: ProtocolNodeKind): void => {
    const id = `protocol-node-${crypto.randomUUID()}`;
    const label = kind === "step" ? "Describe this step"
      : kind === "decision" ? "Describe the decision"
        : "Start or end";
    const node: ProtocolNode = {
      id,
      type: "protocol",
      position: { x: 80 + (nodes.length % 3) * 210, y: 80 + Math.floor(nodes.length / 3) * 140 },
      data: { label, kind },
    };
    setNodes((current) => [...current, node]);
    setSelectedNodeId(id);
  };

  const handleSave = async (): Promise<void> => {
    if (nodes.length === 0) {
      setError("Add at least one protocol node before saving.");
      return;
    }
    const graph = {
      nodes: nodes.map((node) => ({
        id: node.id,
        type: "protocol",
        position: node.position,
        data: { label: node.data.label.trim(), kind: node.data.kind },
      })),
      edges: edges.map((edge) => ({
        id: edge.id,
        source: edge.source,
        target: edge.target,
      })),
    };
    if (graph.nodes.some((node) => !node.data.label)) {
      setError("Give every protocol node a label before saving.");
      return;
    }
    setIsSaving(true);
    setError(null);
    try {
      const updated = await saveProtocolFlowchart(hypothesis.id, JSON.stringify(graph));
      onUpdated(updated);
    } catch (cause) {
      console.error("Could not save protocol flowchart:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsSaving(false);
    }
  };

  const handleConnect = (connection: Connection): void => {
    setEdges((current) => addEdge(connection, current));
  };

  return (
    <div dir="auto" className="mt-4 rounded-xl border border-[#292c34] bg-[#101114] p-3">
      {!isReadOnly && (
        <div dir="auto" className="mb-3 flex flex-wrap items-center gap-2">
          <button type="button" dir="auto" onClick={() => addProtocolNode("step")} className="rounded-lg border border-sky-500/30 px-3 py-2 text-sm text-sky-200 hover:bg-sky-500/10">
            + Step
          </button>
          <button type="button" dir="auto" onClick={() => addProtocolNode("decision")} className="rounded-lg border border-violet-500/30 px-3 py-2 text-sm text-violet-200 hover:bg-violet-500/10">
            + Decision
          </button>
          <button type="button" dir="auto" onClick={() => addProtocolNode("start-end")} className="rounded-lg border border-emerald-500/30 px-3 py-2 text-sm text-emerald-200 hover:bg-emerald-500/10">
            + Start / End
          </button>
          {selectedNode && (
            <label dir="auto" className="flex min-w-48 flex-1 items-center gap-2">
              <span className="text-xs text-gray-400">Selected node</span>
              <input
                dir="auto"
                value={selectedNode.data.label}
                onChange={(event) => {
                  const label = event.currentTarget.value;
                  setNodes((current) => current.map((node) =>
                    node.id === selectedNode.id
                      ? { ...node, data: { ...node.data, label } }
                      : node
                  ));
                }}
                className="min-w-0 flex-1 rounded-lg border border-[#343741] bg-[#17191f] px-3 py-2 text-sm text-gray-200 outline-none focus:border-amber-400/70"
              />
            </label>
          )}
          <button
            type="button"
            dir="auto"
            disabled={isSaving || graphReadError !== null}
            onClick={() => void handleSave()}
            className="ml-auto rounded-lg bg-amber-500 px-4 py-2 text-sm font-semibold text-gray-950 hover:bg-amber-400 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {isSaving ? "Saving…" : "💾 Save Protocol"}
          </button>
        </div>
      )}

      {isReadOnly && (
        <div dir="auto" className="mb-3">
          <div className="flex items-center justify-between gap-3 text-sm text-gray-300">
            <span>Protocol Progress: {progressCount}/{nodes.length} steps completed</span>
            <span>{Math.round(progressPercent)}%</span>
          </div>
          <div
            role="progressbar"
            aria-label="Protocol progress"
            aria-valuemin={0}
            aria-valuemax={nodes.length}
            aria-valuenow={progressCount}
            className="mt-2 h-2 overflow-hidden rounded-full bg-[#292c34]"
          >
            <div className="h-full rounded-full bg-emerald-400 transition-all" style={{ width: `${progressPercent}%` }} />
          </div>
        </div>
      )}

      {(graphReadError || completedIdsResult.error || error) && (
        <p dir="auto" role="alert" className="mb-3 text-sm text-rose-300">
          {error ?? completedIdsResult.error ?? graphReadError}
        </p>
      )}

      <div className="h-[420px] overflow-hidden rounded-lg border border-[#292c34]">
        <ReactFlow
          dir="auto"
          nodes={visibleNodes}
          edges={edges}
          nodeTypes={nodeTypes}
          onNodesChange={isReadOnly ? undefined : onNodesChange}
          onEdgesChange={isReadOnly ? undefined : onEdgesChange}
          onConnect={isReadOnly ? undefined : handleConnect}
          onNodeClick={(_event, node) => setSelectedNodeId(node.id)}
          nodesDraggable={!isReadOnly}
          nodesConnectable={!isReadOnly}
          elementsSelectable={!isReadOnly}
          fitView
          colorMode="dark"
        >
          <Background color="#343741" gap={20} />
          <Controls />
        </ReactFlow>
      </div>
      {isReadOnly && hypothesis.protocolLockedAt != null && (
        <p dir="auto" className="mt-2 text-xs text-emerald-300">
          🔒 Protocol locked on {new Date(hypothesis.protocolLockedAt).toLocaleString()}.
        </p>
      )}
    </div>
  );
};
