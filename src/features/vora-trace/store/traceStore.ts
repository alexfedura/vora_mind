import { create } from "zustand";
import type {
  TraceEdgeData, TraceGraph, TraceNodeData, TraceNodeKind, TraceNodeStatus,
} from "@/types";
import { mockGraph } from "../data/mockGraph";
import {
  chooseGraphExportPath, chooseGraphImportPath, loadGraph, saveGraph, saveDecisionMd, isTauri,
  scanEcosystem, readAllDecisions,
  exportGraphJson, importGraphJson, recentDecisions,
} from "@/mind/services/traceService";

interface TraceState {
  graph: TraceGraph;
  selectedNodeId: string | null;
  hoveredNodeId: string | null;
  dirty: boolean;
  saving: boolean;
  lastError: string | null;
  projects: import("@/types").TraceProjectInfo[];
  projectsLoading: boolean;
  syncing: boolean;
  lastSyncAt: string | null;

  filters: { search: string; kinds: TraceNodeKind[]; statuses: TraceNodeStatus[] };
  setFilters: (patch: Partial<{ search: string; kinds: TraceNodeKind[]; statuses: TraceNodeStatus[] }>) => void;

  selectNode: (id: string | null) => void;
  hoverNode: (id: string | null) => void;
  getNode: (id: string) => TraceNodeData | undefined;

  loadFromDisk: () => Promise<void>;
  saveToDisk: () => Promise<void>;
  loadProjects: () => Promise<void>;
  syncFromDisk: () => Promise<void>;

  recent: TraceNodeData[];
  loadRecent: () => Promise<void>;
  exportGraph: () => Promise<string | null>;
  importGraph: () => Promise<void>;

  addNode: (kind: TraceNodeKind, title: string) => string;
  updateNode: (id: string, patch: Partial<TraceNodeData>) => void;
  deleteNode: (id: string) => void;

  addEdge: (source: string, target: string, kind: TraceEdgeData["kind"]) => void;
  deleteEdge: (id: string) => void;
}

function nextId(prefix: string, existing: string[]): string {
  let max = 0;
  for (const id of existing) {
    const m = id.match(new RegExp(`^${prefix}-(\\d+)$`));
    if (m) max = Math.max(max, parseInt(m[1], 10));
  }
  return `${prefix}-${String(max + 1).padStart(3, "0")}`;
}

function prefixFor(kind: TraceNodeKind): string {
  switch (kind) {
    case "decision": return "D";
    case "experiment": return "E";
    case "rejected": return "D"; // rejected alternatives are decisions
    case "state": return "S";
  }
}

type FilterState = { search: string; kinds: TraceNodeKind[]; statuses: TraceNodeStatus[] };

export const useTraceStore = create<TraceState>()((set, get) => ({
  graph: mockGraph,
  selectedNodeId: null,
  hoveredNodeId: null,
  dirty: false,
  saving: false,
  lastError: null,
  projects: [],
  projectsLoading: false,
  syncing: false,
  lastSyncAt: null,
  recent: [],
  filters: { search: "", kinds: [] as TraceNodeKind[], statuses: [] as TraceNodeStatus[] },

  setFilters: (patch: Partial<FilterState>) => {
    const cur = get().filters;
    set({ filters: { ...cur, ...patch } });
  },

  selectNode: (id) => set({ selectedNodeId: id }),
  hoverNode: (id) => set({ hoveredNodeId: id }),
  getNode: (id) => get().graph.nodes.find((n) => n.id === id),

  loadFromDisk: async () => {
    if (!isTauri()) {
      console.info("[Vora Trace] Not in Tauri — using mock graph");
      return;
    }
    try {
      const g = await loadGraph();
      set({ graph: g, dirty: false, lastError: null });
    } catch (e) {
      console.warn("[Vora Trace] load_from_disk failed:", e);
      set({ lastError: String(e) });
    }
  },

  saveToDisk: async () => {
    if (!isTauri()) {
      console.info("[Vora Trace] Not in Tauri — skip save");
      return;
    }
    set({ saving: true });
    try {
      await saveGraph(get().graph);
      // Also write each decision as .md
      for (const n of get().graph.nodes) {
        try { await saveDecisionMd(n); } catch (e) {
          console.warn("save_decision_md failed for", n.id, e);
        }
      }
      set({ dirty: false, saving: false, lastError: null });
    } catch (e) {
      set({ saving: false, lastError: String(e) });
    }
  },

  loadProjects: async () => {
    if (!isTauri()) return;
    set({ projectsLoading: true });
    try {
      const projects = await scanEcosystem();
      set({ projects, projectsLoading: false });
    } catch (e) {
      console.warn("scan_ecosystem failed:", e);
      set({ projectsLoading: false });
    }
  },

  syncFromDisk: async () => {
    if (!isTauri()) return;
    set({ syncing: true });
    try {
      const diskNodes = await readAllDecisions();
      const g = get().graph;
      const byId = new Map(g.nodes.map((n) => [n.id, n] as const));
      for (const n of diskNodes) {
        const existing = byId.get(n.id);
        if (existing) {
          // preserve non-md fields (alternatives, gitCommit, next)
          byId.set(n.id, { ...existing, ...n,
            alternatives: existing.alternatives,
            gitCommit: existing.gitCommit,
            next: existing.next,
          });
        } else {
          byId.set(n.id, n);
        }
      }
      const merged = Array.from(byId.values())
        .sort((a, b) => a.id.localeCompare(b.id));
      set({
        graph: { ...g, nodes: merged },
        dirty: false,
        syncing: false,
        lastSyncAt: new Date().toISOString(),
      });
    } catch (e) {
      console.warn("sync_from_disk failed:", e);
      set({ syncing: false });
    }
  },

  loadRecent: async () => {
    if (!isTauri()) return;
    try {
      const list = await recentDecisions(10);
      set({ recent: list });
    } catch (e) { console.warn("recent_decisions failed:", e); }
  },

  exportGraph: async () => {
    if (!isTauri()) return null;
    const dest = await chooseGraphExportPath();
    if (!dest) return null;
    try {
      await exportGraphJson(dest);
      return dest;
    } catch (e) {
      set({ lastError: String(e) });
      return null;
    }
  },

  importGraph: async () => {
    if (!isTauri()) return;
    const src = await chooseGraphImportPath();
    if (!src) return;
    try {
      const g = await importGraphJson(src);
      set({ graph: g, dirty: true });
    } catch (e) {
      set({ lastError: String(e) });
    }
  },

  addNode: (kind, title) => {
    const g = get().graph;
    const ids = g.nodes.map((n) => n.id);
    const id = nextId(prefixFor(kind), ids);
    const newNode: TraceNodeData = {
      id,
      kind,
      title: title || `New ${kind}`,
      status: kind === "rejected" ? "rejected" : "pending",
      createdAt: new Date().toISOString(),
      parentId: null,
    };
    set({
      graph: { ...g, nodes: [...g.nodes, newNode] },
      dirty: true,
      selectedNodeId: id,
    });
    return id;
  },

  updateNode: (id, patch) => {
    const g = get().graph;
    set({
      graph: {
        ...g,
        nodes: g.nodes.map((n) => (n.id === id ? { ...n, ...patch } : n)),
      },
      dirty: true,
    });
  },

  deleteNode: (id) => {
    const g = get().graph;
    set({
      graph: {
        ...g,
        nodes: g.nodes.filter((n) => n.id !== id),
        edges: g.edges.filter((e) => e.source !== id && e.target !== id),
      },
      dirty: true,
      selectedNodeId: get().selectedNodeId === id ? null : get().selectedNodeId,
    });
  },

  addEdge: (source, target, kind) => {
    if (source === target) return;
    const g = get().graph;
    const exists = g.edges.some((e) => e.source === source && e.target === target);
    if (exists) return;
    const edge: TraceEdgeData = {
      id: `e-${source}-${target}`,
      source,
      target,
      kind,
    };
    set({ graph: { ...g, edges: [...g.edges, edge] }, dirty: true });
  },

  deleteEdge: (id) => {
    const g = get().graph;
    set({
      graph: { ...g, edges: g.edges.filter((e) => e.id !== id) },
      dirty: true,
    });
  },
}));
