import {
  ReactFlow, Background, Controls, MiniMap,
  useNodesState, useEdgesState, addEdge,
  type Connection, type Edge, type Node, type NodeMouseHandler,
} from "@xyflow/react";
import { useEffect, useMemo, useCallback } from "react";
import { useTraceStore } from "../../store/traceStore";
import type { TraceNodeKind, TraceNodeStatus } from "@/types";
import { StateNode } from "./StateNode";
import { DecisionNode } from "./DecisionNode";
import { ExperimentNode } from "./ExperimentNode";
import { RejectedNode } from "./RejectedNode";

const nodeTypes = {
  state: StateNode,
  decision: DecisionNode,
  experiment: ExperimentNode,
  rejected: RejectedNode,
};

export function DecisionGraph() {
  const graph = useTraceStore((s) => s.graph);
  const filters = useTraceStore((s) => s.filters);
  const hoverNode = useTraceStore((s) => s.hoverNode);
  const selectNode = useTraceStore((s) => s.selectNode);
  const addEdgeToStore = useTraceStore((s) => s.addEdge);
  const deleteNode = useTraceStore((s) => s.deleteNode);

  const initialNodes = useMemo<Node[]>(
    () => {
      const search = filters.search.toLowerCase();
      const visibleNodes = graph.nodes.filter((n) => {
        if (filters.kinds.length > 0 && !filters.kinds.includes(n.kind as TraceNodeKind)) return false;
        if (filters.statuses.length > 0 && !filters.statuses.includes(n.status as TraceNodeStatus)) return false;
        if (search) {
          const hay = `${n.title} ${n.reason ?? ""} ${n.whyAmIHere ?? ""}`.toLowerCase();
          if (!hay.includes(search)) return false;
        }
        return true;
      });
      return visibleNodes.map((n, i) => ({
        id: n.id,
        type: n.kind,
        position: { x: i * 260, y: (i % 2) * 90 },
        data: { id: n.id, title: n.title, status: n.status, label: n.title },
      }));
    },
    [graph.nodes, filters]
  );

  const initialEdges = useMemo<Edge[]>(
    () => {
      const visibleIds = new Set(initialNodes.map((n) => n.id));
      return graph.edges
        .filter((e) => visibleIds.has(e.source) && visibleIds.has(e.target))
        .map((e) => ({
          id: e.id,
          source: e.source,
          target: e.target,
          animated: e.kind === "main",
          style: {
            stroke:
              e.kind === "main" ? "#8b5cf6" :
              e.kind === "rejected" ? "#ef4444" : "#2a2e3a",
            strokeWidth: e.kind === "main" ? 2.5 : 1.5,
            strokeDasharray: e.kind === "rejected" ? "5 5" : undefined,
          },
          label: e.label,
        }));
    },
    [graph.edges, initialNodes]
  );

  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  // Keep RF state in sync with store (create / delete)
  useEffect(() => {
    setNodes(initialNodes);
  }, [initialNodes, setNodes]);
  useEffect(() => {
    setEdges(initialEdges);
  }, [initialEdges, setEdges]);

  const onConnect = useCallback(
    (c: Connection) => {
      if (!c.source || !c.target) return;
      addEdgeToStore(c.source, c.target, "main");
      setEdges((eds) => addEdge({ ...c, id: `e-${c.source}-${c.target}` }, eds));
    },
    [addEdgeToStore, setEdges]
  );

  const onNodeMouseEnter: NodeMouseHandler = useCallback(
    (_, n) => hoverNode(n.id),
    [hoverNode]
  );
  const onNodeMouseLeave: NodeMouseHandler = useCallback(
    () => hoverNode(null),
    [hoverNode]
  );
  const onNodeClick: NodeMouseHandler = useCallback(
    (_, n) => selectNode(n.id),
    [selectNode]
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Delete" || e.key === "Backspace") {
        const sel = document.activeElement;
        if (sel && ["INPUT", "TEXTAREA"].includes(sel.tagName)) return;
        const id = useTraceStore.getState().selectedNodeId;
        if (id) {
          deleteNode(id);
          setNodes((ns) => ns.filter((n) => n.id !== id));
          setEdges((es) => es.filter((x) => x.source !== id && x.target !== id));
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [deleteNode, setNodes, setEdges]);

  return (
    <div className="h-full w-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeMouseEnter={onNodeMouseEnter}
        onNodeMouseLeave={onNodeMouseLeave}
        onNodeClick={onNodeClick}
        nodeTypes={nodeTypes}
        fitView
        proOptions={{ hideAttribution: true }}
      >
        <Background color="#2a2e3a" gap={24} />
        <Controls className="!bg-trace-panel !border-trace-border" />
        <MiniMap
          nodeColor={() => "#8b5cf6"}
          maskColor="rgba(15,17,23,0.8)"
          className="!bg-trace-panel !border !border-trace-border"
        />
      </ReactFlow>
    </div>
  );
}
