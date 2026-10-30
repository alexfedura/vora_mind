import { useState, useEffect, useCallback, useRef } from "react";
import {
  useNodesState,
  useEdgesState,
  addEdge,
  Connection,
  Edge,
  Node,
  MarkerType,
  useReactFlow,
} from "@xyflow/react";
import { useAppStore } from "@/shared/store/useAppStore";
import { dbService, SnapshotSummary, type BoardVersionSnapshot } from "@/mind/services/dbService";
import { VoraCard } from "@/mind/types";

const DEFAULT_CARD_WIDTH = 352;

interface FlowHistoryState {
  nodes: Node[];
  edges: Edge[];
  cards?: Record<string, VoraCard>;
}

interface FlowClipboard {
  nodes: Node[];
  edges: Edge[];
  mode: "copy" | "cut";
  boardId: string | null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function restoreNodeStyle(styleJson: string | undefined): React.CSSProperties | undefined {
  if (!styleJson) return undefined;
  const parsed: unknown = JSON.parse(styleJson);
  if (!isRecord(parsed)) {
    throw new Error("The saved Board version contains invalid node styling.");
  }
  const style: React.CSSProperties = {};
  for (const [key, value] of Object.entries(parsed)) {
    if (typeof value === "string" || typeof value === "number") {
      Object.assign(style, { [key]: value });
    }
  }
  return style;
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    target.isContentEditable;
}

function getCanvasSignature(
  nodes: Node[],
  edges: Edge[],
  cards: Record<string, VoraCard>
): string {
  return JSON.stringify({
    nodes: nodes.map((node) => ({
      id: node.id,
      type: node.type ?? "voraCard",
      position: node.position,
      width: node.measured?.width || node.width || 320,
      height: node.measured?.height || node.height || 180,
      data: node.data,
      style: node.style,
      draggable: node.draggable,
    })),
    edges: edges.map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      type: edge.type,
    })),
    cards: Object.values(cards).map((card) => ({
      id: card.id,
      title: card.title,
      content: card.content,
      version: card.version,
      deleted_at: card.deleted_at,
      librarySourceCardId: card.librarySourceCardId,
    })),
  });
}

export function useVoraFlow(boardId: string, zoomXScale = 1, zoomYScale = 1, autosaveEnabled = true) {
  const { cards, setCards, addCard, updateCardVersion } = useAppStore();

  const [nodes, setNodes, onNodesChangeOriginal] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChangeOriginal] = useEdgesState<Edge>([]);
  const [snapshots, setSnapshots] = useState<SnapshotSummary[]>([]);
  const [currentSnapshotIndex, setCurrentSnapshotIndex] = useState<number>(-1);
  const [isCommitModalOpen, setIsCommitModalOpen] = useState(false);
  const [isReplayVisible, setIsReplayVisible] = useState(false);

  const { setCenter, screenToFlowPosition } = useReactFlow();
  const reactFlowWrapper = useRef<HTMLDivElement>(null);

  const undoStack = useRef<FlowHistoryState[]>([]);
  const redoStack = useRef<FlowHistoryState[]>([]);
  const flowClipboard = useRef<FlowClipboard>({ nodes: [], edges: [], mode: "copy", boardId: null });
  const lastPointerPosition = useRef<{ x: number; y: number } | null>(null);
  const isPerformingUndoRedo = useRef(false);
  const isCanvasReady = useRef(false);
  const autosaveEnabledRef = useRef(autosaveEnabled);
  const lastPersistedCanvas = useRef("");
  const pendingCanvasSave = useRef<Promise<void> | null>(null);
  const currentSnapshotId = useRef<string | null>(null);
  const autosaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const latestNodes = useRef(nodes);
  const latestEdges = useRef(edges);
  const latestCards = useRef(cards);
  const librarySourceCardIds = useRef(new Map<string, string>());
  const currentBoardId = useRef(boardId);
  const boardReadyWaiters = useRef(new Map<string, Array<{
    resolve: () => void;
    reject: (error: unknown) => void;
  }>>());

  latestNodes.current = nodes;
  latestEdges.current = edges;
  latestCards.current = cards;
  for (const [cardId, card] of Object.entries(cards)) {
    if (card.librarySourceCardId) {
      librarySourceCardIds.current.set(cardId, card.librarySourceCardId);
    }
  }
  currentBoardId.current = boardId;
  autosaveEnabledRef.current = autosaveEnabled;

  const saveHistorySnapshot = useCallback((includeCards = false) => {
    if (isPerformingUndoRedo.current) return;
    const snapshot: FlowHistoryState = {
      nodes: JSON.parse(JSON.stringify(nodes)),
      edges: JSON.parse(JSON.stringify(edges)),
    };
    if (includeCards) {
      snapshot.cards = JSON.parse(JSON.stringify(latestCards.current));
    }
    undoStack.current.push(snapshot);
    if (undoStack.current.length > 50) {
      undoStack.current.shift();
    }
    redoStack.current = [];
  }, [nodes, edges]);

  const undo = useCallback(() => {
    if (undoStack.current.length === 0) return;
    const previousState = undoStack.current.pop();
    if (!previousState) return;

    const nextState: FlowHistoryState = {
      nodes: JSON.parse(JSON.stringify(nodes)),
      edges: JSON.parse(JSON.stringify(edges)),
    };
    if (previousState.cards) {
      nextState.cards = JSON.parse(JSON.stringify(latestCards.current));
    }
    redoStack.current.push(nextState);

    isPerformingUndoRedo.current = true;
    setNodes(previousState.nodes);
    setEdges(previousState.edges);
    if (previousState.cards) {
      latestCards.current = previousState.cards;
      setCards(previousState.cards);
    }
    setTimeout(() => {
      isPerformingUndoRedo.current = false;
    }, 50);
  }, [nodes, edges, setCards, setEdges, setNodes]);

  const redo = useCallback(() => {
    if (redoStack.current.length === 0) return;
    const nextState = redoStack.current.pop();
    if (!nextState) return;

    const previousState: FlowHistoryState = {
      nodes: JSON.parse(JSON.stringify(nodes)),
      edges: JSON.parse(JSON.stringify(edges)),
    };
    if (nextState.cards) {
      previousState.cards = JSON.parse(JSON.stringify(latestCards.current));
    }
    undoStack.current.push(previousState);

    isPerformingUndoRedo.current = true;
    setNodes(nextState.nodes);
    setEdges(nextState.edges);
    if (nextState.cards) {
      latestCards.current = nextState.cards;
      setCards(nextState.cards);
    }
    setTimeout(() => {
      isPerformingUndoRedo.current = false;
    }, 50);
  }, [nodes, edges, setCards, setEdges, setNodes]);

  const copyNodesToClipboard = useCallback((sourceNodes: Node[]) => {
    if (sourceNodes.length === 0) return false;

    const selectedIds = new Set(sourceNodes.map((node) => node.id));
    flowClipboard.current = {
      nodes: JSON.parse(JSON.stringify(sourceNodes)),
      edges: JSON.parse(JSON.stringify(
        latestEdges.current.filter((edge) => selectedIds.has(edge.source) && selectedIds.has(edge.target))
      )),
      mode: "copy",
      boardId: null,
    };
    return true;
  }, []);

  const copySelectedNodes = useCallback(
    () => copyNodesToClipboard(latestNodes.current.filter((node) => node.selected)),
    [copyNodesToClipboard]
  );

  const cutNodes = useCallback((sourceNodes: Node[]) => {
    if (!copyNodesToClipboard(sourceNodes)) return false;

    const selectedIds = new Set(flowClipboard.current.nodes.map((node) => node.id));
    flowClipboard.current.mode = "cut";
    flowClipboard.current.boardId = currentBoardId.current;
    saveHistorySnapshot();
    setNodes((current) => current.filter((node) => !selectedIds.has(node.id)));
    setEdges((current) => current.filter((edge) =>
      !selectedIds.has(edge.source) && !selectedIds.has(edge.target)
    ));
    return true;
  }, [copyNodesToClipboard, saveHistorySnapshot, setEdges, setNodes]);

  const cutSelectedNodes = useCallback(
    () => cutNodes(latestNodes.current.filter((node) => node.selected)),
    [cutNodes]
  );

  const createNodeCopies = useCallback((
    sourceNodes: Node[],
    offset: { x: number; y: number },
    sourceEdges: Edge[] = latestEdges.current
  ): Pick<FlowClipboard, "nodes" | "edges"> => {
    const sourceIds = new Set(sourceNodes.map((node) => node.id));
    const idMap = new Map<string, string>();
    const groupIdMap = new Map<string, string>();
    sourceNodes.forEach((node) => {
      const groupId = isRecord(node.data) ? node.data.canvasGroupId : null;
      if (typeof groupId === "string" && !groupIdMap.has(groupId)) {
        groupIdMap.set(groupId, `vora-group-${crypto.randomUUID()}`);
      }
    });
    const copiedNodes = sourceNodes.map((node): Node => {
      const id = `vora-node-${crypto.randomUUID()}`;
      idMap.set(node.id, id);
      const data = structuredClone(node.data);
      const sourceGroupId = isRecord(data) ? data.canvasGroupId : null;
      if (typeof sourceGroupId === "string") {
        data.canvasGroupId = groupIdMap.get(sourceGroupId) ?? sourceGroupId;
      }

      if (node.type === "voraCard") {
        const originalCardId = typeof data.cardId === "string" ? data.cardId : node.id;
        const originalCard = latestCards.current[originalCardId];
        const title = originalCard?.title ??
          (typeof data.title === "string" ? data.title : typeof data.label === "string" ? data.label : "New card");
        const content = originalCard?.content ?? (typeof data.content === "string" ? data.content : "");
        const version = originalCard?.version ?? (typeof data.version === "string" ? data.version : "v0.1.0");
        const cardId = addCard(title, content);
        updateCardVersion(cardId, version);
        data.cardId = cardId;
        data.title = title;
        data.label = title;
        data.content = content;
        data.version = version;
      }

      return {
        ...node,
        id,
        data,
        position: { x: node.position.x + offset.x, y: node.position.y + offset.y },
        selected: false,
      };
    });

    const copiedEdges = sourceEdges
      .filter((edge) => sourceIds.has(edge.source) && sourceIds.has(edge.target))
      .flatMap((edge): Edge[] => {
        const source = idMap.get(edge.source);
        const target = idMap.get(edge.target);
        return source && target
          ? [{ ...edge, id: `vora-edge-${crypto.randomUUID()}`, source, target }]
          : [];
      });
    return { nodes: copiedNodes, edges: copiedEdges };
  }, [addCard, updateCardVersion]);

  const pasteCopiedNodes = useCallback(() => {
    const clipboard = flowClipboard.current;
    const { nodes: copiedNodes } = clipboard;
    if (copiedNodes.length === 0) return false;

    const bounds = reactFlowWrapper.current?.getBoundingClientRect();
    const pointerPosition = lastPointerPosition.current ?? (
      bounds ? { x: bounds.left + bounds.width / 2, y: bounds.top + bounds.height / 2 } : null
    );
    if (!pointerPosition) return false;

    const canvasPointerPosition = bounds
      ? {
        x: bounds.left + (pointerPosition.x - bounds.left) / zoomXScale,
        y: bounds.top + (pointerPosition.y - bounds.top) / zoomYScale,
      }
      : pointerPosition;
    const cursorPosition = screenToFlowPosition(canvasPointerPosition);
    const minX = Math.min(...copiedNodes.map((node) => node.position.x));
    const minY = Math.min(...copiedNodes.map((node) => node.position.y));
    const maxX = Math.max(...copiedNodes.map((node) =>
      node.position.x + (node.measured?.width ?? node.width ?? 0)
    ));
    const maxY = Math.max(...copiedNodes.map((node) =>
      node.position.y + (node.measured?.height ?? node.height ?? 0)
    ));
    const offset = {
      x: cursorPosition.x - (minX + maxX) / 2,
      y: cursorPosition.y - (minY + maxY) / 2,
    };

    saveHistorySnapshot();
    const currentNodeIds = new Set(latestNodes.current.map((node) => node.id));
    const restoringCutNodes = clipboard.mode === "cut" &&
      clipboard.boardId === currentBoardId.current &&
      copiedNodes.every((node) => !currentNodeIds.has(node.id));
    const copies = restoringCutNodes
      ? { nodes: copiedNodes, edges: clipboard.edges }
      : createNodeCopies(copiedNodes, offset, clipboard.edges);
    const pastedNodes = copies.nodes.map((node): Node => ({
      ...node,
      position: { x: node.position.x + (restoringCutNodes ? offset.x : 0), y: node.position.y + (restoringCutNodes ? offset.y : 0) },
      selected: true,
    }));
    setNodes([
      ...latestNodes.current.map((node) => ({ ...node, selected: false })),
      ...pastedNodes,
    ]);
    if (copies.edges.length > 0) {
      setEdges([...latestEdges.current, ...copies.edges]);
    }
    if (clipboard.mode === "cut") {
      flowClipboard.current = { ...clipboard, mode: "copy", boardId: null };
    }
    return true;
  }, [createNodeCopies, saveHistorySnapshot, screenToFlowPosition, setEdges, setNodes, zoomXScale, zoomYScale]);

  useEffect(() => {
    const handlePointerMove = (event: PointerEvent): void => {
      lastPointerPosition.current = { x: event.clientX, y: event.clientY };
    };
    window.addEventListener("pointermove", handlePointerMove);
    const handleKeyDown = (e: KeyboardEvent) => {
      const isInput = isEditableTarget(e.target);
      const isModifierPressed = e.ctrlKey || e.metaKey;
      const key = e.key.toLowerCase();

      if (isModifierPressed && !isInput && (key === "x" || key === "ط")) {
        if (cutSelectedNodes()) e.preventDefault();
      } else if (isModifierPressed && !isInput && (key === "c" || key === "ز")) {
        if (copySelectedNodes()) e.preventDefault();
      } else if (isModifierPressed && !isInput && (key === "v" || key === "ر")) {
        if (pasteCopiedNodes()) e.preventDefault();
      } else if (isModifierPressed && (key === "z" || key === "ظ")) {
        if (!e.shiftKey && !isInput) {
          e.preventDefault();
          undo();
        } else if (e.shiftKey && !isInput) {
          e.preventDefault();
          redo();
        }
      } else if (isModifierPressed && key === "y" && !isInput) {
        e.preventDefault();
        redo();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [copySelectedNodes, cutSelectedNodes, pasteCopiedNodes, undo, redo]);

  const refreshSnapshots = useCallback(async () => {
    try {
      const list = await dbService.getAllSnapshots(boardId);
      setSnapshots(list);
      if (list.length > 0) {
        setCurrentSnapshotIndex(list.length - 1);
        currentSnapshotId.current = list[list.length - 1]?.id ?? null;
      }
    } catch (err) {
      console.error("Failed to fetch snapshots:", err);
    }
  }, [boardId]);

  const onConnect = useCallback(
    (params: Connection) => {
      if (!currentBoardId.current) return;
      saveHistorySnapshot();
      setEdges((eds) =>
        addEdge(
          {
            ...params,
            type: "default",
            markerEnd: { type: MarkerType.ArrowClosed },
            style: { stroke: "#4b5563", strokeWidth: 2 },
          },
          eds
        )
      );
    },
    [setEdges, saveHistorySnapshot]
  );

  const handleAddNewCardOnCanvas = useCallback((position: Node["position"]) => {
    if (!currentBoardId.current) return "";
    saveHistorySnapshot();
    const newId = addCard();
    const newNode: Node = {
      id: newId,
      type: "voraCard",
      position: { x: position.x - DEFAULT_CARD_WIDTH / 2, y: position.y },
      width: DEFAULT_CARD_WIDTH,
      selected: true,
      data: { label: "New card", title: "New card", content: "", cardId: newId, version: "v0.1.0" },
    };
    setNodes((nds) => [...nds.map((node): Node => ({ ...node, selected: false })), newNode]);
    return newId;
  }, [addCard, setNodes, saveHistorySnapshot]);

  const handleCommitSnapshot = async (name: string, reason: string, summary: string) => {
    if (!autosaveEnabledRef.current) {
      throw new Error("Library board drafts must be saved as a new Library version.");
    }
    if (!boardId) throw new Error("Create a Board before saving a snapshot.");
    const parentId =
      currentSnapshotIndex >= 0 && snapshots[currentSnapshotIndex]
        ? snapshots[currentSnapshotIndex].id
        : null;

    const snapshotId = await dbService.commitSnapshot(
      boardId,
      name,
      parentId,
      reason,
      summary,
      nodes,
      edges,
      cards
    );
    currentSnapshotId.current = snapshotId;
    lastPersistedCanvas.current = getCanvasSignature(nodes, edges, cards);
    await refreshSnapshots();
  };

  const handleApplyResolvedSnapshot = useCallback(
    async (snapshotId: string) => {
      if (!boardId) return false;
      try {
        const resolved = await dbService.resolveSnapshot(boardId, snapshotId);

        const updatedNodes: Node[] = Object.entries(resolved.placements).map(([id, placement]) => {
          const snapshotNode = resolved.node_data[id];
          const existingCard = latestCards.current[id];
          const savedData = isRecord(snapshotNode?.data) ? snapshotNode.data : null;
          const nodeType = snapshotNode?.node_type || "voraCard";
          const legacyCardData = {
            label: snapshotNode?.title ?? existingCard?.title ?? "Card",
            title: snapshotNode?.title ?? existingCard?.title ?? "Card",
            content: snapshotNode?.content ?? existingCard?.content ?? "",
            cardId: id,
            version: snapshotNode?.version ?? existingCard?.version,
          };
          const restoredNode: Node = {
            id,
            type: nodeType,
            position: { x: placement.x, y: placement.y },
            width: placement.width,
            height: placement.height,
            data: savedData ?? legacyCardData,
          };
          if (nodeType === "voraShape") {
            restoredNode.className = savedData?.shape === "path"
              ? "vora-path-node vora-shape-border-hit"
              : "vora-shape-border-hit";
          }
          return restoredNode;
        });

        const updatedEdges: Edge[] = Object.entries(resolved.links).map(([edgeId, link]: [string, any]) => ({
          id: edgeId,
          source: link.source_object_id,
          target: link.target_object_id,
          type: "default",
          markerEnd: link.link_type === "directed" ? { type: MarkerType.ArrowClosed } : undefined,
          style: { stroke: "#7c3aed", strokeWidth: 2 },
        }));

        setNodes(updatedNodes);
        setEdges(updatedEdges);
        currentSnapshotId.current = snapshotId;
        lastPersistedCanvas.current = getCanvasSignature(
          updatedNodes,
          updatedEdges,
          latestCards.current
        );

        if (updatedNodes.length > 0) {
          setCenter(
            updatedNodes[0].position.x + 150,
            updatedNodes[0].position.y + 100,
            { zoom: 1.0, duration: 600 }
          );
        }
        return true;
      } catch (err) {
        console.error("Failed to resolve snapshot state:", err);
        return false;
      }
    },
    [boardId, setCenter, setEdges, setNodes]
  );

  useEffect(() => {
    let cancelled = false;
    currentBoardId.current = boardId;

    const loadLatestSnapshot = async (): Promise<void> => {
      try {
        isCanvasReady.current = false;
        setNodes([]);
        setEdges([]);
        setCards({});
        undoStack.current = [];
        redoStack.current = [];
        if (!boardId) {
          latestCards.current = {};
          librarySourceCardIds.current.clear();
          setSnapshots([]);
          setCurrentSnapshotIndex(-1);
          currentSnapshotId.current = null;
          lastPersistedCanvas.current = getCanvasSignature([], [], {});
          isCanvasReady.current = true;
          const waiters = boardReadyWaiters.current.get(boardId) ?? [];
          boardReadyWaiters.current.delete(boardId);
          waiters.forEach((waiter) => waiter.resolve());
          return;
        }
        const [savedCards, list] = await Promise.all([
          dbService.getAllCards(boardId),
          dbService.getAllSnapshots(boardId),
        ]);
        if (cancelled) return;
        const cardsMap: Record<string, VoraCard> = {};
        librarySourceCardIds.current.clear();
        for (const card of savedCards) {
          if (card.library_source_card_id) {
            librarySourceCardIds.current.set(card.id, card.library_source_card_id);
          }
          cardsMap[card.id] = {
            id: card.id,
            title: card.title,
            content: card.content,
            version: card.version || "v0.1.0",
            boardId,
            created_at: Date.now(),
            deleted_at: card.deleted_at ?? undefined,
            librarySourceCardId: card.library_source_card_id ?? undefined,
          };
        }
        latestCards.current = cardsMap;
        setCards(cardsMap);
        setSnapshots(list);
        const latestIndex = list.length - 1;
        const latestSnapshot = list[latestIndex];
        if (latestSnapshot) {
          setCurrentSnapshotIndex(latestIndex);
          currentSnapshotId.current = latestSnapshot.id;
          const restored = await handleApplyResolvedSnapshot(latestSnapshot.id);
          if (!restored || cancelled) return;
        } else {
          setNodes([]);
          setEdges([]);
          setCurrentSnapshotIndex(-1);
          currentSnapshotId.current = null;
          lastPersistedCanvas.current = getCanvasSignature([], [], cardsMap);
        }
        isCanvasReady.current = true;
        const waiters = boardReadyWaiters.current.get(boardId) ?? [];
        boardReadyWaiters.current.delete(boardId);
        waiters.forEach((waiter) => waiter.resolve());
      } catch (err) {
        console.error("Failed to load the saved canvas:", err);
        const waiters = boardReadyWaiters.current.get(boardId) ?? [];
        boardReadyWaiters.current.delete(boardId);
        waiters.forEach((waiter) => waiter.reject(err));
      }
    };

    void loadLatestSnapshot();
    return () => {
      cancelled = true;
      isCanvasReady.current = false;
    };
  }, [boardId, handleApplyResolvedSnapshot, setCards, setEdges, setNodes]);

  const waitForBoardReady = useCallback((requestedBoardId: string): Promise<void> => {
    if (currentBoardId.current === requestedBoardId && isCanvasReady.current) {
      return Promise.resolve();
    }
    return new Promise((resolve, reject) => {
      const waiters = boardReadyWaiters.current.get(requestedBoardId) ?? [];
      waiters.push({ resolve, reject });
      boardReadyWaiters.current.set(requestedBoardId, waiters);
    });
  }, []);

  const persistCanvas = useCallback(async (): Promise<void> => {
    if (!autosaveEnabledRef.current || !boardId || !isCanvasReady.current) return;
    if (pendingCanvasSave.current) {
      await pendingCanvasSave.current;
      if (!isCanvasReady.current) return;
      if (getCanvasSignature(
        latestNodes.current,
        latestEdges.current,
        latestCards.current
      ) !== lastPersistedCanvas.current) {
        await persistCanvas();
      }
      return;
    }
    const savedNodes = latestNodes.current;
    const savedEdges = latestEdges.current;
    const savedCards = latestCards.current;
    const savedBoardId = currentBoardId.current;
    const signature = getCanvasSignature(savedNodes, savedEdges, savedCards);
    if (signature === lastPersistedCanvas.current) return;

    const saveTask = (async (): Promise<void> => {
      try {
        const snapshotId = await dbService.commitSnapshot(
          savedBoardId,
          "Auto Save",
          currentSnapshotId.current,
          "auto-save",
          "Automatically saved canvas state",
          savedNodes,
          savedEdges,
          savedCards
        );
        currentSnapshotId.current = snapshotId;
        lastPersistedCanvas.current = signature;
        await refreshSnapshots();
      } catch (err) {
        console.error("Failed to save the canvas automatically:", err);
        throw err;
      } finally {
        pendingCanvasSave.current = null;
      }
    })();
    pendingCanvasSave.current = saveTask;
    try {
      await saveTask;
    } finally {
      if (getCanvasSignature(
        latestNodes.current,
        latestEdges.current,
        latestCards.current
      ) !== lastPersistedCanvas.current) {
        autosaveTimer.current = setTimeout(() => {
          autosaveTimer.current = null;
          void persistCanvas().catch(() => undefined);
        }, 700);
      }
    }
  }, [boardId, refreshSnapshots]);

  const applyBoardVersionSnapshot = useCallback(async (
    snapshot: BoardVersionSnapshot,
    targetBoardId: string,
  ): Promise<void> => {
    if (currentBoardId.current !== targetBoardId || !isCanvasReady.current) {
      throw new Error("The selected Board is not ready for version restore.");
    }
    const restoredNodes: Node[] = snapshot.nodes.map((node) => ({
      id: node.id,
      type: node.node_type,
      position: { x: node.x, y: node.y },
      width: node.width,
      height: node.height,
      style: restoreNodeStyle(node.style),
      data: node.data,
      className: node.node_type === "voraShape"
        ? node.data.shape === "path" ? "vora-path-node vora-shape-border-hit" : "vora-shape-border-hit"
        : undefined,
    }));
    const restoredEdges: Edge[] = snapshot.edges.map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      type: "default",
      markerEnd: edge.link_type === "directed" ? { type: MarkerType.ArrowClosed } : undefined,
      style: { stroke: "#7c3aed", strokeWidth: 2 },
    }));
    const restoredCards: Record<string, VoraCard> = {};
    for (const card of snapshot.cards) {
      restoredCards[card.id] = {
        id: card.id,
        title: card.title,
        content: card.content,
        version: card.version,
        boardId: targetBoardId,
        created_at: Date.now(),
        deleted_at: card.deleted_at ?? undefined,
        librarySourceCardId:
          librarySourceCardIds.current.get(card.id) ?? latestCards.current[card.id]?.librarySourceCardId,
      };
    }

    undoStack.current = [];
    redoStack.current = [];
    latestNodes.current = restoredNodes;
    latestEdges.current = restoredEdges;
    latestCards.current = restoredCards;
    setNodes(restoredNodes);
    setEdges(restoredEdges);
    setCards(restoredCards);
    lastPersistedCanvas.current = "";
    await persistCanvas();
  }, [persistCanvas, setCards, setEdges, setNodes]);

  useEffect(() => {
    if (!autosaveEnabled || !isCanvasReady.current) return;
    if (getCanvasSignature(nodes, edges, cards) === lastPersistedCanvas.current) return;
    if (autosaveTimer.current) clearTimeout(autosaveTimer.current);
    autosaveTimer.current = setTimeout(() => {
      autosaveTimer.current = null;
      void persistCanvas().catch(() => undefined);
    }, 700);
    return () => {
      if (autosaveTimer.current) {
        clearTimeout(autosaveTimer.current);
        autosaveTimer.current = null;
      }
    };
  }, [autosaveEnabled, nodes, edges, cards, persistCanvas]);

  useEffect(() => () => {
    if (!autosaveEnabledRef.current) return;
    if (!autosaveTimer.current) return;
    clearTimeout(autosaveTimer.current);
    autosaveTimer.current = null;
    void persistCanvas().catch(() => undefined);
  }, [persistCanvas]);

  const flushCanvas = useCallback(async (): Promise<void> => {
    if (!autosaveEnabledRef.current) return;
    if (autosaveTimer.current) {
      clearTimeout(autosaveTimer.current);
      autosaveTimer.current = null;
    }
    await persistCanvas();
  }, [persistCanvas]);

  const replaceCanvasState = useCallback((
    nextNodes: Node[],
    nextEdges: Edge[],
    nextCards: Record<string, VoraCard>,
  ): void => {
    if (autosaveTimer.current) {
      clearTimeout(autosaveTimer.current);
      autosaveTimer.current = null;
    }
    undoStack.current = [];
    redoStack.current = [];
    latestNodes.current = nextNodes;
    latestEdges.current = nextEdges;
    latestCards.current = nextCards;
    setNodes(nextNodes);
    setEdges(nextEdges);
    setCards(nextCards);
  }, [setCards, setEdges, setNodes]);

  const handleSelectSnapshot = async (index: number) => {
    if (index < 0 || index >= snapshots.length) return;
    setCurrentSnapshotIndex(index);
    const snap = snapshots[index];
    await handleApplyResolvedSnapshot(snap.id);
  };

  return {
    nodes,
    edges,
    onNodesChange: onNodesChangeOriginal,
    onEdgesChange: onEdgesChangeOriginal,
    onConnect,
    reactFlowWrapper,
    handleAddNewCardOnCanvas,
    snapshots,
    currentSnapshotIndex,
    isCommitModalOpen,
    setIsCommitModalOpen,
    isReplayVisible,
    setIsReplayVisible,
    handleCommitSnapshot,
    handleSelectSnapshot,
    handleApplyResolvedSnapshot,
    waitForBoardReady,
    applyBoardVersionSnapshot,
    setNodes,
    setEdges,
    replaceCanvasState,
    saveHistorySnapshot,
    flushCanvas,
    undo,
    redo,
    copyNodesToClipboard,
    cutNodes,
  };
}