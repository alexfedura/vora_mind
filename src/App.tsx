import React, { useMemo, useState, useEffect } from "react";
import { Link } from "react-router-dom";
import {
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  SelectionMode,
  Position,
  type OnConnectEnd,
  type NodeChange,
  type Edge,
  type Node,
  type Viewport,
} from "@xyflow/react";
import {
  PanelLeft,
  LayoutGrid,
  PanelRight,
  PanelRightClose,
  History,
  CandlestickChart,
  Type,
  PlusSquare,
  Minus,
  Plus,
  Sparkles,
  Group,
  Ungroup,
  BookOpen,
} from "lucide-react";

import { ForgeAgentDrawer } from "./forge/components/agent/ForgeAgentDrawer";
import { ForgeAstNode } from "./forge/components/canvas/ForgeAstNode";
import { useVoraFlow } from "./shared/canvas/useVoraFlow";
import { useDocumentStore } from "./features/file-store/useDocumentStore";
import { CreateItemGroupPanel } from "./shared/canvas/CreateItemGroupPanel";
import { useAppStore } from "./shared/store/useAppStore";
import {
  createLibraryItemGroupSnapshot,
  dbService,
  type BoardVersionIncrement,
  type BoardVersionSnapshot,
} from "./mind/services/dbService";
import { getSetting, setSetting } from "./shared/store/useSettingsStore";
import {
  type BoardItem,
  type CanvasGroupMembership,
  type LibraryItemGroup,
  type TextLinkSelection,
  type VoraPathSegment,
} from "./mind/types";
import { VoraCardNode } from "./mind/components/canvas/VoraCardNode";
import { VoraTextNode } from "./mind/components/canvas/VoraTextNode";
import { VoraShapeNode } from "./mind/components/canvas/VoraShapeNode";
import { CanvasZoomContext } from "./mind/components/canvas/CanvasZoomContext";
import type { VoraShapeData, VoraShapeDefaults, VoraShapePoint, VoraShapeStyleSettings } from "./mind/types";
import {
  canJoinPathNodes,
  cutPathSegment,
  getPathSegments,
  joinPathNodes,
  makeSingleSegmentNode,
  type VoraPathNode,
} from "./mind/utils/pathSegments";
import { PathToolIcon } from "./components/market/PathToolIcon";
import { RectangleToolIcon } from "./components/market/RectangleToolIcon";
import { LeftSidebar } from "./shared/layout/LeftSidebar";
import { TaskFlowPage } from "./shared/layout/TaskFlowPage";
import VoraTraceApp from "./features/vora-trace/App";
import { JournalPage } from "./shared/layout/JournalPage";
import { QuestionsBoardPage } from "./shared/layout/QuestionsBoardPage";
import { BoardsPanel } from "./shared/layout/BoardsPanel";
import { RightDrawer } from "./shared/layout/RightDrawer";
import { ReplayController } from "./shared/canvas/ReplayController";
import { CommitModal } from "./shared/modals/CommitModal";
import { FullscreenDocModal } from "./shared/modals/FullscreenDocModal";
import { OpenPanel } from "./mind/types";

type ShapeTool = "rectangle" | "path" | null;
type ShapePreview = { shape: "rectangle" | "path"; points: { x: number; y: number }[] };

interface CanvasShapePoint {
  flow: VoraShapePoint;
  screen: { x: number; y: number };
}

interface CanvasMarquee {
  start: { x: number; y: number };
  additive: boolean;
}

interface MarqueePreview {
  left: number;
  top: number;
  width: number;
  height: number;
}

interface CanvasSelectionBounds {
  left: number;
  top: number;
  width: number;
  height: number;
}

interface ShapeResizeDimensions {
  width: number;
  height: number;
}

interface CanvasNodeContextMenu {
  x: number;
  y: number;
  textLinkSelection?: TextLinkSelection;
}

interface ConnectionCreateMenu {
  x: number;
  y: number;
  position: { x: number; y: number };
  sourcePoint: { x: number; y: number };
  sourceNodeId: string;
  sourceHandleId: string;
  sourceHandleType: "source" | "target";
  boardId: string;
}

interface LibraryBoardSession {
  group: LibraryItemGroup;
  baseVersion: string;
  boardId: string;
  returnViewport: Viewport;
  returnSelectedNodeIds: string[];
  returnSelectedEdgeIds: string[];
  returnSelectedCardId: string | null;
  returnDrawerWasOpen: boolean;
}

const ZOOM_LEVEL_STEP = 0.1;
const MIN_ZOOM_SCALE = 0.05;
const MAX_ZOOM_SCALE = 12;

function normalizeZoomLevel(level: number): number {
  if (!Number.isFinite(level)) return 0;
  return Number(Math.min(MAX_ZOOM_SCALE - 1, Math.max(MIN_ZOOM_SCALE - 1, level)).toFixed(3));
}

function zoomLevelToScale(level: number): number {
  return Number((1 + normalizeZoomLevel(level)).toFixed(3));
}

function formatZoomLevel(level: number): string {
  return Number(level).toFixed(3);
}

function getFacingHandlePosition(
  from: { x: number; y: number },
  to: { x: number; y: number },
): Position {
  const deltaX = to.x - from.x;
  const deltaY = to.y - from.y;
  if (Math.abs(deltaX) >= Math.abs(deltaY)) {
    return deltaX >= 0 ? Position.Left : Position.Right;
  }
  return deltaY >= 0 ? Position.Top : Position.Bottom;
}

function getHandleId(type: "source" | "target", position: Position): string {
  return `${type}-${position}`;
}

const SHAPE_DEFAULTS_KEY = "vora.canvas.shape_defaults.v1";
const CANVAS_TOOL_BUTTON_CLASS = "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg transition";
const CANVAS_TOOL_BUTTON_IDLE_CLASS = "text-zinc-400 hover:text-violet-300 hover:bg-[#1c202d]";
const CANVAS_TOOL_BUTTON_ACTIVE_CLASS = "bg-violet-600 text-white shadow-md shadow-violet-950/40";
const CANVAS_FLOATING_CONTROL_BORDER_CLASS = "border border-[#2a2a2a]";
const CANVAS_FLOATING_CONTROL_BACKGROUND_CLASS = "bg-[#181818]/90";
const DEFAULT_SHAPE_SETTINGS: VoraShapeStyleSettings = {
  color: "#e8ad55",
  lineOpacity: 1,
  lineWidth: 2,
  lineStyle: "solid",
  textColor: "#dbdbdb",
  textOpacity: 1,
  fillColor: "#e8ad55",
  fillOpacity: 0.12,
  startArrow: false,
  endArrow: true,
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isVoraPathSegment(value: unknown): value is VoraPathSegment {
  return isRecord(value) &&
    typeof value.id === "string" &&
    typeof value.name === "string" &&
    typeof value.details === "string" &&
    typeof value.color === "string" &&
    /^#[0-9a-fA-F]{6}$/.test(value.color) &&
    (value.libraryItemGroupId === undefined || typeof value.libraryItemGroupId === "string");
}

function isVoraShapePoint(value: unknown): value is VoraShapePoint {
  return isRecord(value) &&
    typeof value.x === "number" && Number.isFinite(value.x) &&
    typeof value.y === "number" && Number.isFinite(value.y);
}

function isVoraPathNode(node: Node): node is VoraPathNode {
  const data = node.data;
  return node.type === "voraShape" &&
    isRecord(data) &&
    data.shape === "path" &&
    typeof data.label === "string" &&
    typeof data.color === "string" &&
    typeof data.lineOpacity === "number" &&
    typeof data.lineWidth === "number" &&
    (data.lineStyle === "solid" || data.lineStyle === "dashed" || data.lineStyle === "dotted") &&
    typeof data.textColor === "string" &&
    typeof data.textOpacity === "number" &&
    typeof data.fillColor === "string" &&
    typeof data.fillOpacity === "number" &&
    Array.isArray(data.points) &&
    data.points.every(isVoraShapePoint) &&
    typeof data.startArrow === "boolean" &&
    typeof data.endArrow === "boolean";
}

function getCanvasGroupId(node: Node): string | null {
  return isRecord(node.data) &&
    typeof node.data.canvasGroupId === "string" &&
    node.data.canvasGroupId.length > 0
    ? node.data.canvasGroupId
    : null;
}

function getNodeIds(detail: Record<string, unknown>): string[] | null {
  if (Array.isArray(detail.nodeIds)) {
    const ids: string[] = [];
    for (const value of detail.nodeIds) {
      if (typeof value !== "string" || value.length === 0) return null;
      ids.push(value);
    }
    return ids.length > 0 ? ids : null;
  }
  return typeof detail.nodeId === "string" && detail.nodeId.length > 0 ? [detail.nodeId] : null;
}

function restoreItemGroupNodeStyle(styleJson: string | undefined): React.CSSProperties | undefined {
  if (!styleJson) return undefined;
  const parsed: unknown = JSON.parse(styleJson);
  if (!isRecord(parsed)) {
    throw new Error("The saved item group contains invalid node styling.");
  }
  const style: React.CSSProperties = {};
  for (const [key, value] of Object.entries(parsed)) {
    if (typeof value === "string" || typeof value === "number") {
      Object.assign(style, { [key]: value });
    }
  }
  return style;
}

function isShapePatch(value: unknown): value is Partial<VoraShapeData> {
  if (!isRecord(value)) return false;
  return Object.entries(value).every(([key, field]) => {
    switch (key) {
      case "label":
        return typeof field === "string";
      case "color":
      case "textColor":
      case "fillColor":
        return typeof field === "string" && /^#[0-9a-fA-F]{6}$/.test(field);
      case "lineOpacity":
      case "textOpacity":
      case "fillOpacity":
        return typeof field === "number" && Number.isFinite(field) && field >= 0 && field <= 1;
      case "lineWidth":
        return typeof field === "number" && Number.isInteger(field) && field >= 1 && field <= 4;
      case "lineStyle":
        return field === "solid" || field === "dashed" || field === "dotted";
      case "startArrow":
      case "endArrow":
        return typeof field === "boolean";
      case "points":
        return Array.isArray(field) && field.every((point) =>
          isRecord(point) &&
          typeof point.x === "number" &&
          Number.isFinite(point.x) &&
          point.x >= 0 &&
          point.x <= 1 &&
          typeof point.y === "number" &&
          Number.isFinite(point.y) &&
          point.y >= 0 &&
          point.y <= 1
        );
      case "segments":
        return Array.isArray(field) && field.every(isVoraPathSegment);
      default:
        return false;
    }
  });
}

function validOpacity(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 1 ? value : fallback;
}

function readShapeSettings(value: unknown, fallback: VoraShapeStyleSettings): VoraShapeStyleSettings {
  if (!isRecord(value)) return fallback;
  return {
    color: typeof value.color === "string" && /^#[0-9a-fA-F]{6}$/.test(value.color) ? value.color : fallback.color,
    lineOpacity: validOpacity(value.lineOpacity, fallback.lineOpacity),
    lineWidth: typeof value.lineWidth === "number" && Number.isInteger(value.lineWidth) && value.lineWidth >= 1 && value.lineWidth <= 4 ? value.lineWidth : fallback.lineWidth,
    lineStyle: value.lineStyle === "solid" || value.lineStyle === "dashed" || value.lineStyle === "dotted" ? value.lineStyle : fallback.lineStyle,
    textColor: typeof value.textColor === "string" && /^#[0-9a-fA-F]{6}$/.test(value.textColor) ? value.textColor : fallback.textColor,
    textOpacity: validOpacity(value.textOpacity, fallback.textOpacity),
    fillColor: typeof value.fillColor === "string" && /^#[0-9a-fA-F]{6}$/.test(value.fillColor) ? value.fillColor : fallback.fillColor,
    fillOpacity: validOpacity(value.fillOpacity, fallback.fillOpacity),
    startArrow: typeof value.startArrow === "boolean" ? value.startArrow : fallback.startArrow,
    endArrow: typeof value.endArrow === "boolean" ? value.endArrow : fallback.endArrow,
  };
}

function loadShapeDefaults(): VoraShapeDefaults {
  const defaults: VoraShapeDefaults = {
    rectangle: { ...DEFAULT_SHAPE_SETTINGS },
    path: { ...DEFAULT_SHAPE_SETTINGS, fillOpacity: 0 },
  };
  try {
    const stored = getSetting(SHAPE_DEFAULTS_KEY);
    if (!stored) return defaults;
    const parsed: unknown = JSON.parse(stored);
    if (!isRecord(parsed)) return defaults;
    return {
      rectangle: readShapeSettings(parsed.rectangle, defaults.rectangle),
      path: readShapeSettings(parsed.path, defaults.path),
    };
  } catch (error) {
    console.error("Could not load canvas shape defaults", error);
    return defaults;
  }
}

function AppContent() {
  const {
    toggleRightDrawer,
    isRightDrawerOpen,
    addCard,
    updateCardVersion,
    cards,
    activeBoardId,
    setActiveBoardId,
    setLibraryBoardEditing,
    toggleForgeAgent,
    fetchCanonicalEntities,
  } = useAppStore();

  const [activeOpenPanel, setActiveOpenPanel] = useState<OpenPanel>("none");
  const [isTaskFlowOpen, setIsTaskFlowOpen] = useState(false);
  const [isAiTasksOpen, setIsAiTasksOpen] = useState(false);
  const [isVoraTraceOpen, setIsVoraTraceOpen] = useState(false);
  const [isJournalOpen, setIsJournalOpen] = useState(false);
  const [isJournalMenuOpen, setIsJournalMenuOpen] = useState(false);
  const [isQuestionsBoardOpen, setIsQuestionsBoardOpen] = useState(false);
  const isWorkspacePageOpen = isTaskFlowOpen || isAiTasksOpen || isVoraTraceOpen || isJournalOpen || isQuestionsBoardOpen;
  const [lastActivePanel, setLastActivePanel] = useState<"boards" | "sidebar">("boards");
  const [boards, setBoards] = useState<BoardItem[]>([]);
  const [boardsLoaded, setBoardsLoaded] = useState(false);
  const [shouldStartCreatingBoard, setShouldStartCreatingBoard] = useState(false);
  const [isCreatingFirstBoard, setIsCreatingFirstBoard] = useState(false);
  const [itemGroups, setItemGroups] = useState<LibraryItemGroup[]>([]);
  const [itemGroupError, setItemGroupError] = useState<string | null>(null);
  const [libraryBoardSession, setLibraryBoardSession] = useState<LibraryBoardSession | null>(null);
  const [isLibraryVersionDialogOpen, setIsLibraryVersionDialogOpen] = useState(false);
  const [libraryVersionInput, setLibraryVersionInput] = useState("");
  const [libraryBoardError, setLibraryBoardError] = useState<string | null>(null);
  const [isCreateItemGroupOpen, setIsCreateItemGroupOpen] = useState(false);
  const [canvasNodeContextMenu, setCanvasNodeContextMenu] = useState<CanvasNodeContextMenu | null>(null);
  const [connectionCreateMenu, setConnectionCreateMenu] = useState<ConnectionCreateMenu | null>(null);
  const [selectedCardId, setSelectedCardId] = useState<string | null>(null);
  const [fullscreenCardId, setFullscreenCardId] = useState<string | null>(null);
  const [activeCanvasGroupId, setActiveCanvasGroupId] = useState<string | null>(null);
  const [zoomLevel, setZoomLevel] = useState(0);
  const [marqueePreview, setMarqueePreview] = useState<MarqueePreview | null>(null);
  const [selectionBounds, setSelectionBounds] = useState<CanvasSelectionBounds | null>(null);
  const [isSpacePanActive, setIsSpacePanActive] = useState(false);
  const isSpacePanActiveRef = React.useRef(false);
  const lastPointerPositionRef = React.useRef<{ x: number; y: number } | null>(null);

  // حالت ابزار متن
  const [isAddCardToolActive, setIsAddCardToolActive] = useState(false);
  const [isTextToolActive, setIsTextToolActive] = useState(false);
  const [activeShapeTool, setActiveShapeTool] = useState<ShapeTool>(null);
  const [shapePreview, setShapePreview] = useState<ShapePreview | null>(null);
  const [shapeDefaults, setShapeDefaults] = useState<VoraShapeDefaults>(loadShapeDefaults);
  const rectangleStartRef = React.useRef<CanvasShapePoint | null>(null);
  const pathPointsRef = React.useRef<CanvasShapePoint[]>([]);
  const dragPositionsRef = React.useRef(new Map<string, {
    raw: Node["position"];
    normalized: Node["position"];
  }>());
  const groupDragRef = React.useRef<{
    groupId: string;
    nodeId: string;
    startPosition: Node["position"];
    positions: Map<string, Node["position"]>;
  } | null>(null);
  const resizeDimensionsRef = React.useRef(new Map<string, ShapeResizeDimensions>());
  const marqueeRef = React.useRef<CanvasMarquee | null>(null);

  const zoomScale = zoomLevelToScale(zoomLevel);
  const canvasZoomScales = useMemo(
    () => ({ x: zoomScale, y: zoomScale }),
    [zoomScale]
  );
  const canvasBoardId = libraryBoardSession?.boardId ?? activeBoardId;
  const flow = useVoraFlow(canvasBoardId, zoomScale, zoomScale, libraryBoardSession === null);
  const selectedNodes = useMemo(() => flow.nodes.filter((node) => node.selected), [flow.nodes]);
  const selectedGroupIds = useMemo(
    () => new Set([
      ...selectedNodes.map(getCanvasGroupId).filter((groupId): groupId is string => groupId !== null),
      ...(activeCanvasGroupId ? [activeCanvasGroupId] : []),
    ]),
    [activeCanvasGroupId, selectedNodes]
  );
  const { screenToFlowPosition, getViewport, setViewport, setCenter, getNodes, fitView } = useReactFlow();

  useEffect(() => {
    setActiveCanvasGroupId(null);
  }, [canvasBoardId]);

  useEffect(() => {
    const selectedIds = new Set(selectedNodes.map((node) => node.id));
    if (activeCanvasGroupId) {
      flow.nodes.forEach((node) => {
        if (getCanvasGroupId(node) === activeCanvasGroupId) selectedIds.add(node.id);
      });
    }
    if (selectedIds.size === 0) {
      setSelectionBounds(null);
      return;
    }

    const frame = window.requestAnimationFrame(() => {
      const wrapper = flow.reactFlowWrapper.current;
      if (!wrapper) {
        setSelectionBounds(null);
        return;
      }
      const wrapperBounds = wrapper.getBoundingClientRect();
      const nodeBounds = [...wrapper.querySelectorAll<HTMLElement>(".react-flow__node[data-id]")]
        .filter((element) => {
          const nodeId = element.getAttribute("data-id");
          return nodeId !== null && selectedIds.has(nodeId);
        })
        .map((element) => element.getBoundingClientRect());
      if (nodeBounds.length === 0) {
        setSelectionBounds(null);
        return;
      }
      const left = Math.min(...nodeBounds.map((bounds) => bounds.left));
      const top = Math.min(...nodeBounds.map((bounds) => bounds.top));
      const right = Math.max(...nodeBounds.map((bounds) => bounds.right));
      const bottom = Math.max(...nodeBounds.map((bounds) => bounds.bottom));
      setSelectionBounds({
        left: left - wrapperBounds.left,
        top: top - wrapperBounds.top,
        width: right - left,
        height: bottom - top,
      });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [canvasBoardId, activeCanvasGroupId, flow.nodes, flow.reactFlowWrapper, selectedNodes]);

  useEffect(() => {
    const viewport = flow.reactFlowWrapper.current?.querySelector<HTMLElement>(".react-flow__viewport");
    if (!viewport) return;
    viewport.style.scale = `${zoomScale}`;
    viewport.style.transformOrigin = "0 0";
  }, [flow.reactFlowWrapper, zoomScale]);

  const toCanvasFlowPosition = (point: { x: number; y: number }): { x: number; y: number } => {
    const bounds = flow.reactFlowWrapper.current?.getBoundingClientRect();
    if (!bounds) return screenToFlowPosition(point);
    return screenToFlowPosition({
      x: bounds.left + (point.x - bounds.left) / zoomScale,
      y: bounds.top + (point.y - bounds.top) / zoomScale,
    });
  };

  const changeCanvasZoom = (
    delta: number,
    pointer?: { x: number; y: number }
  ): void => {
    const bounds = flow.reactFlowWrapper.current?.getBoundingClientRect();
    const viewport = getViewport();
    const nextZoomLevel = normalizeZoomLevel(zoomLevel + delta);
    const nextZoomScale = zoomLevelToScale(nextZoomLevel);
    const nextViewport = { ...viewport };

    if (bounds && zoomScale !== nextZoomScale) {
      const anchorX = pointer ? pointer.x - bounds.left : bounds.width / 2;
      const anchorY = pointer ? pointer.y - bounds.top : bounds.height / 2;
      nextViewport.x += anchorX * (1 / nextZoomScale - 1 / zoomScale);
      nextViewport.y += anchorY * (1 / nextZoomScale - 1 / zoomScale);
    }
    if (nextViewport.x !== viewport.x || nextViewport.y !== viewport.y) {
      void setViewport(nextViewport).catch((error: unknown) => {
        console.error("Could not preserve canvas position while changing canvas zoom:", error);
      });
    }

    setZoomLevel(nextZoomLevel);
  };

  const nodeTypes = useMemo(
    () => ({
      voraCard: VoraCardNode,
      voraText: VoraTextNode,
      voraShape: VoraShapeNode,
      forgeAst: ForgeAstNode,
    }),
    []
  );

  // بارگذاری خودکار و بازیابی وضعیت موجودیت‌های کانونیکال در آغاز به کار اپلیکیشن
  useEffect(() => {
    fetchCanonicalEntities();
  }, [fetchCanonicalEntities]);

  useEffect(() => {
    let cancelled = false;
    const loadBoards = async (): Promise<void> => {
      try {
        const savedBoards = await dbService.getAllBoards();
        if (cancelled) return;
        setBoards(savedBoards);
        setBoardsLoaded(true);
        if (!savedBoards.some((board) => board.id === activeBoardId)) {
          const nextBoardId = savedBoards[0]?.id ?? "";
          setActiveBoardId(nextBoardId);
        }
      } catch (error) {
        console.error("Failed to load Boards:", error);
      }
    };
    void loadBoards();
    return () => { cancelled = true; };
  }, [activeBoardId, setActiveBoardId]);

  useEffect(() => {
    let cancelled = false;
    const loadItemGroups = async (): Promise<void> => {
      try {
        const groups = await dbService.getLibraryItemGroups();
        if (!cancelled) {
          setItemGroups(groups);
          setItemGroupError(null);
        }
      } catch (error) {
        console.error("Failed to load Library item groups:", error);
        if (!cancelled) {
          setItemGroupError(error instanceof Error ? error.message : String(error));
        }
      }
    };
    void loadItemGroups();
    return () => { cancelled = true; };
  }, []);

  const handleSelectBoard = async (boardId: string): Promise<void> => {
    if (libraryBoardSession) return;
    if (boardId === activeBoardId) return;
    await flow.flushCanvas();
    setActiveBoardId(boardId);
    await flow.waitForBoardReady(boardId);
  };

  const handleCreateBoard = async (title: string): Promise<void> => {
    await flow.flushCanvas();
    const created = await dbService.createBoard(title);
    setBoards((current) => [...current, created]);
    setActiveBoardId(created.id);
  };

  const handleCreateLibraryItemGroup = async (name: string, version: string): Promise<void> => {
    if (!canvasBoardId) {
      throw new Error("Create a Board before adding items to the Library.");
    }
    if (selectedNodes.length < 1) {
      throw new Error("Select at least one canvas item to create an item group.");
    }
    const group = await dbService.createLibraryItemGroup(
      name,
      version,
      selectedNodes,
      flow.edges,
      cards,
    );
    setItemGroups((current) => [group, ...current.filter((item) => item.id !== group.id)]);
    setItemGroupError(null);
    setIsCreateItemGroupOpen(false);
  };

  const handleMoveLibraryItemGroupToTrash = async (groupId: string): Promise<void> => {
    await dbService.moveLibraryItemGroupToTrash(groupId);
    const deletedAt = Date.now();
    setItemGroups((current) => current.map((group) =>
      group.id === groupId ? { ...group, deleted_at: deletedAt, updated_at: deletedAt } : group
    ));
    setItemGroupError(null);
  };

  const handleRestoreLibraryItemGroup = async (groupId: string): Promise<void> => {
    await dbService.restoreLibraryItemGroup(groupId);
    const updatedAt = Date.now();
    setItemGroups((current) => current.map((group) =>
      group.id === groupId ? { ...group, deleted_at: null, updated_at: updatedAt } : group
    ));
    setItemGroupError(null);
  };

  const handleGetLibraryItemGroupVersions = (groupId: string) =>
    dbService.getLibraryItemGroupVersions(groupId);

  const restoreMyDeskAfterLibraryEdit = async (session: LibraryBoardSession): Promise<void> => {
    setLibraryBoardSession(null);
    await flow.waitForBoardReady(activeBoardId);
    flow.setNodes((nodes) => nodes.map((node) => ({
      ...node,
      selected: session.returnSelectedNodeIds.includes(node.id),
    })));
    flow.setEdges((edges) => edges.map((edge) => ({
      ...edge,
      selected: session.returnSelectedEdgeIds.includes(edge.id),
    })));
    await setViewport(session.returnViewport, { duration: 0 });
    setSelectedCardId(session.returnSelectedCardId);
    setLibraryBoardEditing(false);
    setIsLibraryVersionDialogOpen(false);
    setLibraryBoardError(null);
    if (session.returnDrawerWasOpen && !useAppStore.getState().isRightDrawerOpen) {
      toggleRightDrawer();
    }
  };

  const handleOpenLibraryBoard = async (
    group: LibraryItemGroup,
    snapshot: LibraryItemGroup["snapshot"],
    baseVersion: string,
  ): Promise<void> => {
    if (libraryBoardSession) {
      throw new Error("Close the current Library Board before opening another one.");
    }
    await flow.flushCanvas();
    const session: LibraryBoardSession = {
      group,
      baseVersion,
      boardId: group.board_id,
      returnViewport: getViewport(),
      returnSelectedNodeIds: flow.nodes.filter((node) => node.selected).map((node) => node.id),
      returnSelectedEdgeIds: flow.edges.filter((edge) => edge.selected).map((edge) => edge.id),
      returnSelectedCardId: selectedCardId,
      returnDrawerWasOpen: useAppStore.getState().isRightDrawerOpen,
    };
    setLibraryBoardError(null);
    setSelectedCardId(null);
    setCanvasNodeContextMenu(null);
    setConnectionCreateMenu(null);
    setIsAddCardToolActive(false);
    setIsTextToolActive(false);
    setActiveShapeTool(null);
    rectangleStartRef.current = null;
    pathPointsRef.current = [];
    setShapePreview(null);
    setLibraryBoardEditing(true, cards);
    setLibraryBoardSession(session);
    if (session.returnDrawerWasOpen) toggleRightDrawer();

    try {
      await flow.waitForBoardReady(session.boardId);
      const boardSnapshot: BoardVersionSnapshot = {
        nodes: snapshot.nodes,
        edges: snapshot.edges,
        cards: snapshot.cards,
      };
      await flow.applyBoardVersionSnapshot(boardSnapshot, session.boardId);
      await fitView({ padding: 0.2, duration: 0 });
    } catch (cause) {
      try {
        await restoreMyDeskAfterLibraryEdit(session);
      } catch (restoreError) {
        console.error("Could not restore My desk after failing to open a Library Board:", restoreError);
      }
      throw cause;
    }
  };

  const handleCloseLibraryBoard = async (): Promise<void> => {
    const session = libraryBoardSession;
    if (!session) return;
    await restoreMyDeskAfterLibraryEdit(session);
  };

  const handleCommitLibraryBoardVersion = async (): Promise<void> => {
    const session = libraryBoardSession;
    if (!session) {
      throw new Error("No Library Board is open.");
    }
    const version = libraryVersionInput.trim();
    if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
      throw new Error("Enter a version in three-part numeric format, such as 1.2.3.");
    }
    const snapshot = createLibraryItemGroupSnapshot(flow.nodes, flow.edges, cards);
    const updated = await dbService.commitLibraryItemGroupVersion(session.group.id, version, snapshot);
    setItemGroups((current) => current.map((group) => group.id === updated.id ? updated : group));
    setItemGroupError(null);
    await restoreMyDeskAfterLibraryEdit(session);
  };

  const handleInsertLibraryItemGroup = (
    group: LibraryItemGroup,
    dropPosition?: { x: number; y: number },
  ): void => {
    if (!canvasBoardId) {
      throw new Error("Create a Board before adding items to the canvas.");
    }
    if (group.deleted_at !== null) {
      throw new Error("Restore this item group from Trash before adding it to the canvas.");
    }
    const groupNodes = group.snapshot.nodes;
    if (groupNodes.length === 0) {
      throw new Error("This item group does not contain any canvas items.");
    }
    const minX = Math.min(...groupNodes.map((node) => node.x));
    const minY = Math.min(...groupNodes.map((node) => node.y));
    const maxX = Math.max(...groupNodes.map((node) => node.x + (node.width ?? 0)));
    const maxY = Math.max(...groupNodes.map((node) => node.y + (node.height ?? 0)));
    const bounds = flow.reactFlowWrapper.current?.getBoundingClientRect();
    const center = dropPosition ?? (bounds
      ? toCanvasFlowPosition({ x: bounds.left + bounds.width / 2, y: bounds.top + bounds.height / 2 })
      : { x: 300, y: 200 });
    const offsetX = center.x - (maxX - minX) / 2;
    const offsetY = center.y - (maxY - minY) / 2;

    const newCardIds = new Map<string, string>();
    const sourceCards = new Map(group.snapshot.cards.map((card) => [card.id, card]));
    for (const card of group.snapshot.cards) {
      const newCardId = addCard(card.title, card.content);
      updateCardVersion(newCardId, card.version);
      newCardIds.set(card.id, newCardId);
    }
    const newNodeIds = new Map(groupNodes.map((node) => [node.id, `group-${crypto.randomUUID()}`]));
    const newCanvasGroupIds = new Map<string, string>();
    groupNodes.forEach((node) => {
      const groupId = isRecord(node.data) ? node.data.canvasGroupId : null;
      if (typeof groupId === "string" && !newCanvasGroupIds.has(groupId)) {
        newCanvasGroupIds.set(groupId, `vora-group-${crypto.randomUUID()}`);
      }
    });
    const nodes: Node[] = groupNodes.map((savedNode) => {
      const data = structuredClone(savedNode.data);
      const savedGroupId = typeof data.canvasGroupId === "string" ? data.canvasGroupId : null;
      if (savedGroupId) {
        data.canvasGroupId = newCanvasGroupIds.get(savedGroupId) ?? savedGroupId;
      }
      const savedCardId = typeof data.cardId === "string"
        ? data.cardId
        : savedNode.node_type === "voraCard" ? savedNode.id : null;
      const newCardId = savedCardId ? newCardIds.get(savedCardId) : undefined;
      const sourceCard = savedCardId ? sourceCards.get(savedCardId) : undefined;
      if (newCardId && sourceCard) {
        data.cardId = newCardId;
        data.title = sourceCard.title;
        data.label = sourceCard.title;
        data.content = sourceCard.content;
        data.version = sourceCard.version;
      }
      if (
        data.libraryPathAsset === true &&
        savedNode.node_type === "voraShape" &&
        Array.isArray(data.segments)
      ) {
        if (!data.segments.every(isVoraPathSegment)) {
          throw new Error("This Library Path item contains invalid section data.");
        }
        data.segments = data.segments.map((segment) => ({
          ...segment,
          libraryItemGroupId: group.id,
        }));
      }
      const shape = data.shape;
      return {
        id: newNodeIds.get(savedNode.id) ?? `group-${crypto.randomUUID()}`,
        type: savedNode.node_type,
        position: { x: savedNode.x + offsetX, y: savedNode.y + offsetY },
        width: savedNode.width,
        height: savedNode.height,
        style: restoreItemGroupNodeStyle(savedNode.style),
        className: savedNode.node_type === "voraShape"
          ? shape === "path" ? "vora-path-node vora-shape-border-hit" : "vora-shape-border-hit"
          : undefined,
        data,
        selected: false,
      };
    });
    const edges = group.snapshot.edges.flatMap((savedEdge) => {
      const source = newNodeIds.get(savedEdge.source);
      const target = newNodeIds.get(savedEdge.target);
      if (!source || !target) return [];
      return [{
        id: `group-edge-${crypto.randomUUID()}`,
        source,
        target,
        type: "default",
        style: { stroke: "#7c3aed", strokeWidth: 2 },
      }];
    });
    flow.saveHistorySnapshot();
    flow.setNodes((current) => [
      ...current.map((node) => ({ ...node, selected: false })),
      ...nodes,
    ]);
    flow.setEdges((current) => [
      ...current.map((edge) => ({ ...edge, selected: false })),
      ...edges,
    ]);
    setSelectedCardId(null);
    setItemGroupError(null);
  };

  const handleRenameBoard = async (boardId: string, title: string): Promise<void> => {
    await dbService.renameBoard(boardId, title);
    setBoards((current) => current.map((board) =>
      board.id === boardId ? { ...board, title, updated_at: Date.now() } : board
    ));
  };

  const handleDeleteBoard = async (boardId: string): Promise<void> => {
    const deletingActiveBoard = boardId === activeBoardId;
    if (deletingActiveBoard) await flow.flushCanvas();
    await dbService.deleteBoard(boardId);
    const remainingBoards = boards.filter((board) => board.id !== boardId);
    setBoards(remainingBoards);
    if (deletingActiveBoard) {
      setActiveShapeTool(null);
      setIsTextToolActive(false);
      rectangleStartRef.current = null;
      pathPointsRef.current = [];
      setShapePreview(null);
      setSelectedCardId(null);
      setActiveBoardId(remainingBoards[0]?.id ?? "");
    }
  };

  const handleGetBoardVersions = async (boardId: string) => {
    return await dbService.getBoardVersions(boardId);
  };

  const handleCreateBoardVersion = async (
    boardId: string,
    increment: BoardVersionIncrement,
  ) => {
    if (boardId !== activeBoardId) {
      throw new Error("Select this Board before creating a version.");
    }
    await flow.flushCanvas();
    await dbService.createBoardVersion(boardId, increment, flow.nodes, flow.edges, cards);
    const [updatedBoards, versions] = await Promise.all([
      dbService.getAllBoards(),
      dbService.getBoardVersions(boardId),
    ]);
    setBoards(updatedBoards);
    return versions;
  };

  const handleCheckoutBoardVersion = async (
    boardId: string,
    versionId: string,
  ): Promise<void> => {
    if (boardId !== activeBoardId) {
      await handleSelectBoard(boardId);
    }
    await flow.waitForBoardReady(boardId);
    await flow.flushCanvas();
    const snapshot = await dbService.checkoutBoardVersion(boardId, versionId);
    await flow.applyBoardVersionSnapshot(snapshot, boardId);
    setBoards(await dbService.getAllBoards());
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && (isTextToolActive || activeShapeTool !== null)) {
        setIsTextToolActive(false);
        setActiveShapeTool(null);
        rectangleStartRef.current = null;
        pathPointsRef.current = [];
        setShapePreview(null);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [activeShapeTool, isTextToolActive]);

  useEffect(() => {
    const setSpacePanActive = (active: boolean): void => {
      isSpacePanActiveRef.current = active;
      setIsSpacePanActive(active);
    };
    const isEditableTarget = (target: EventTarget | null): boolean =>
      target instanceof HTMLElement
      && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.code !== "Space" || isEditableTarget(event.target)) return;
      setSpacePanActive(true);
      if (flow.reactFlowWrapper.current?.matches(":hover")) event.preventDefault();
    };
    const handleKeyUp = (event: KeyboardEvent): void => {
      if (event.code === "Space") setSpacePanActive(false);
    };
    const handleWindowBlur = (): void => setSpacePanActive(false);

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);
    window.addEventListener("blur", handleWindowBlur);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
      window.removeEventListener("blur", handleWindowBlur);
    };
  }, [flow.reactFlowWrapper]);

  useEffect(() => {
    const handlePointerMove = (event: PointerEvent): void => {
      lastPointerPositionRef.current = { x: event.clientX, y: event.clientY };
    };
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.code !== "Backquote" || event.repeat) return;
      if (event.target instanceof HTMLElement && (
        event.target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(event.target.tagName)
      )) return;
      const selectedToolbar = document.querySelector(
        ".react-flow__node-voraShape.selected [data-name='drawing-toolbar'], .react-flow__node-voraText.selected .vora-text-toolbar"
      );
      if (!selectedToolbar) return;
      const pointer = lastPointerPositionRef.current;
      if (!pointer) return;
      event.preventDefault();
      window.dispatchEvent(new CustomEvent("vora:center-floating-toolbar", { detail: pointer }));
    };
    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  useEffect(() => {
    const handleSelectCard = (e: any) => {
      setSelectedCardId(e.detail.cardId);
    };

    const handleRemoveFromCanvas = (event: Event) => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const requestedIds = getNodeIds(event.detail);
      if (!requestedIds) return;
      const ids = new Set(requestedIds);
      flow.saveHistorySnapshot();
      flow.setNodes((nodes) => nodes.filter((node) => {
        const cardId = isRecord(node.data) ? node.data.cardId : null;
        return !ids.has(node.id) && !(typeof cardId === "string" && ids.has(cardId));
      }));
      flow.setEdges((edges) => edges.filter((edge) => !ids.has(edge.source) && !ids.has(edge.target)));
      if (selectedCardId && ids.has(selectedCardId)) setSelectedCardId(null);
    };

    const handleDuplicateShape = (event: Event): void => {
      if (!(event instanceof CustomEvent)) return;
      const detail: unknown = event.detail;
      if (!isRecord(detail) || typeof detail.nodeId !== "string") return;
      const source = flow.nodes.find((node) => node.id === detail.nodeId && node.type === "voraShape");
      if (!source) return;
      flow.saveHistorySnapshot();
      const duplicateData = structuredClone(source.data);
      delete duplicateData.canvasGroupId;
      const duplicate: Node = {
        ...source,
        id: `${source.id}-copy-${Date.now()}`,
        data: duplicateData,
        position: { x: source.position.x + 24, y: source.position.y + 24 },
        selected: true,
      };
      flow.setNodes((nodes) => [...nodes.map((node) => ({ ...node, selected: false })), duplicate]);
    };

    const handleToggleShapeLock = (event: Event): void => {
      if (!(event instanceof CustomEvent)) return;
      const detail: unknown = event.detail;
      if (!isRecord(detail)) return;
      const requestedIds = getNodeIds(detail);
      if (!requestedIds) return;
      const ids = new Set(requestedIds);
      const targets = flow.nodes.filter((node) => ids.has(node.id) && node.type === "voraShape");
      if (targets.length === 0) return;
      const locked = typeof detail.locked === "boolean" ? detail.locked : targets[0].draggable !== false;
      flow.saveHistorySnapshot();
      flow.setNodes((nodes) => nodes.map((node) => ids.has(node.id) && node.type === "voraShape" ? { ...node, draggable: !locked } : node));
    };

    const handleUpdateShape = (event: Event) => {
      if (!(event instanceof CustomEvent)) return;
      const detail: unknown = event.detail;
      if (!isRecord(detail)) return;
      const patch = detail.patch;
      if (!isShapePatch(patch)) return;
      const requestedIds = getNodeIds(detail);
      if (!requestedIds) return;
      const ids = new Set(requestedIds);
      const targets = flow.nodes.filter((node) => ids.has(node.id) && node.type === "voraShape");
      if (targets.length === 0) return;
      flow.saveHistorySnapshot();
      const newPathColor = patch.color;
      flow.setNodes((nodes) => nodes.map((node) => {
        if (!ids.has(node.id) || node.type !== "voraShape") return node;
        const data = { ...node.data, ...patch };
        if (newPathColor !== undefined && patch.segments === undefined && isVoraPathNode(node)) {
          data.segments = getPathSegments(node.id, node.data)
            .map((segment) => ({ ...segment, color: newPathColor }));
        }
        return { ...node, data };
      }));
      setShapeDefaults((current) => {
        const next: VoraShapeDefaults = { ...current };
        const shapes = new Set(targets.map((node) => node.data.shape).filter((shape): shape is "rectangle" | "path" => shape === "rectangle" || shape === "path"));
        for (const shape of shapes) {
          const previous = current[shape];
          const updated: VoraShapeStyleSettings = {
            color: patch.color ?? previous.color,
            lineOpacity: patch.lineOpacity ?? previous.lineOpacity,
            lineWidth: patch.lineWidth ?? previous.lineWidth,
            lineStyle: patch.lineStyle ?? previous.lineStyle,
            textColor: patch.textColor ?? previous.textColor,
            textOpacity: patch.textOpacity ?? previous.textOpacity,
            fillColor: patch.fillColor ?? previous.fillColor,
            fillOpacity: patch.fillOpacity ?? previous.fillOpacity,
            startArrow: patch.startArrow ?? previous.startArrow,
            endArrow: patch.endArrow ?? previous.endArrow,
          };
          next[shape] = updated;
        }
        return next;
      });
    };

    const handleUpdatePathGeometry = (event: Event): void => {
      if (!(event instanceof CustomEvent)) return;
      const detail: unknown = event.detail;
      if (
        !isRecord(detail) ||
        typeof detail.nodeId !== "string" ||
        !isRecord(detail.position) ||
        typeof detail.position.x !== "number" ||
        !Number.isFinite(detail.position.x) ||
        typeof detail.position.y !== "number" ||
        !Number.isFinite(detail.position.y) ||
        typeof detail.width !== "number" ||
        !Number.isFinite(detail.width) ||
        detail.width <= 0 ||
        typeof detail.height !== "number" ||
        !Number.isFinite(detail.height) ||
        detail.height <= 0 ||
        !isShapePatch({ points: detail.points })
      ) return;

      const nodeId = detail.nodeId;
      const position = { x: detail.position.x, y: detail.position.y };
      const width = detail.width;
      const height = detail.height;
      const points = detail.points;
      if (!flow.nodes.some((node) => node.id === nodeId && node.type === "voraShape")) return;
      flow.saveHistorySnapshot();
      flow.setNodes((nodes) => nodes.map((node) => node.id === nodeId && node.type === "voraShape"
        ? { ...node, position, width, height, data: { ...node.data, points } }
        : node));
    };

    const handleCutPathSegment = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const { nodeId, segmentId } = event.detail;
      if (typeof nodeId !== "string" || typeof segmentId !== "string") return;
      const source = flow.nodes.find((node) => node.id === nodeId);
      if (!source || !isVoraPathNode(source)) return;
      const segmentIndex = getPathSegments(source.id, source.data)
        .findIndex((segment) => segment.id === segmentId);
      const chunks = cutPathSegment(source, segmentIndex);
      if (!chunks) return;
      flow.saveHistorySnapshot();
      flow.setNodes((nodes): Node[] => nodes.flatMap((node): Node[] => node.id === source.id
        ? chunks
        : [{ ...node, selected: false }]));
    };

    const handleJoinSelectedPaths = (): void => {
      const selectedPaths = flow.nodes.filter(
        (node): node is VoraPathNode => node.selected === true && isVoraPathNode(node),
      );
      if (!canJoinPathNodes(selectedPaths)) {
        setItemGroupError("Select two Path sections whose endpoints meet to rejoin them.");
        return;
      }
      const joined = joinPathNodes(selectedPaths);
      if (!joined) {
        setItemGroupError("These Path sections cannot be rejoined because their endpoints do not meet.");
        return;
      }
      const sourceIds = new Set(selectedPaths.map((node) => node.id));
      const targetId = selectedPaths[0]?.id;
      if (!targetId) return;
      flow.saveHistorySnapshot();
      flow.setNodes((nodes): Node[] => nodes.flatMap((node): Node[] => {
        if (node.id === targetId) return [{ ...joined, id: targetId, selected: true }];
        return sourceIds.has(node.id) ? [] : [{ ...node, selected: false }];
      }));
      flow.setEdges((edges) => edges.flatMap((edge) => {
        const sourceId = sourceIds.has(edge.source) ? targetId : edge.source;
        const targetNodeId = sourceIds.has(edge.target) ? targetId : edge.target;
        if (sourceId === targetNodeId) return [];
        return [{ ...edge, source: sourceId, target: targetNodeId }];
      }));
      setItemGroupError(null);
    };

    const handleAddPathSegmentToLibrary = async (event: Event): Promise<void> => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const { nodeId, segmentId } = event.detail;
      if (typeof nodeId !== "string" || typeof segmentId !== "string") return;
      if (!canvasBoardId) {
        setItemGroupError("Create a Board before adding a Path section to the Library.");
        return;
      }
      if (libraryBoardSession) {
        setItemGroupError("Close the Library Board draft before saving one of its Path sections.");
        return;
      }
      const source = flow.nodes.find((node) => node.id === nodeId);
      if (!source || !isVoraPathNode(source)) return;
      const segments = getPathSegments(source.id, source.data);
      const segment = segments.find((item) => item.id === segmentId);
      const segmentIndex = segments.findIndex((item) => item.id === segmentId);
      if (!segment || segment.libraryItemGroupId) {
        setItemGroupError(segment ? "This Path section is already linked to a Library item." : "The selected Path section no longer exists.");
        return;
      }
      const savedNode = makeSingleSegmentNode(source, segmentIndex);
      if (!savedNode) {
        setItemGroupError("The selected Path section could not be prepared for the Library.");
        return;
      }
      savedNode.data.libraryPathAsset = true;
      try {
        const group = await dbService.createLibraryItemGroup(
          segment.name.trim() || "Path section",
          "0.1.0",
          [savedNode],
          [],
          cards,
        );
        const linkedSegments = getPathSegments(source.id, source.data).map((item) =>
          item.id === segmentId ? { ...item, libraryItemGroupId: group.id } : item
        );
        flow.saveHistorySnapshot();
        flow.setNodes((nodes) => nodes.map((node) =>
          node.id === nodeId && isVoraPathNode(node)
            ? { ...node, data: { ...node.data, segments: linkedSegments } }
            : node
        ));
        setItemGroups((current) => [group, ...current.filter((item) => item.id !== group.id)]);
        setItemGroupError(null);
      } catch (cause) {
        console.error("Could not add Path section to the Library:", cause);
        setItemGroupError(cause instanceof Error ? cause.message : String(cause));
      }
    };

    const handleAddPathSegmentToLibraryEvent = (event: Event): void => {
      void handleAddPathSegmentToLibrary(event);
    };

    const handleOpenFullscreenEvent = (e: any) => {
      setFullscreenCardId(e.detail.cardId);
    };

    const handleDirectDropCard = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      if (!canvasBoardId) return;
      const { id, clientX, clientY } = event.detail;
      if (
        typeof id !== "string" ||
        typeof clientX !== "number" ||
        !Number.isFinite(clientX) ||
        typeof clientY !== "number" ||
        !Number.isFinite(clientY)
      ) return;
      const sourceCard = cards[id];
      if (!sourceCard) {
        console.error("Could not copy Library card to canvas: the source card is no longer available.");
        return;
      }

      flow.saveHistorySnapshot();
      const copiedCardId = addCard(sourceCard.title, sourceCard.content, sourceCard.id);
      const copiedVersion = sourceCard.version || "v0.1.0";
      updateCardVersion(copiedCardId, copiedVersion);

      flow.setNodes((nds) =>
        nds.concat({
          id: `node-${crypto.randomUUID()}`,
          type: "voraCard",
          position: toCanvasFlowPosition({ x: clientX, y: clientY }),
          data: {
            label: sourceCard.title,
            title: sourceCard.title,
            content: sourceCard.content,
            cardId: copiedCardId,
            version: copiedVersion,
          },
        })
      );
    };

    const handleDirectDropItemGroup = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const { id, clientX, clientY } = event.detail;
      if (
        typeof id !== "string" ||
        typeof clientX !== "number" ||
        !Number.isFinite(clientX) ||
        typeof clientY !== "number" ||
        !Number.isFinite(clientY)
      ) return;
      const group = itemGroups.find((item) => item.id === id);
      if (!group) {
        setItemGroupError("This item group is no longer available in the Library.");
        return;
      }
      try {
        handleInsertLibraryItemGroup(group, toCanvasFlowPosition({ x: clientX, y: clientY }));
      } catch (cause) {
        console.error("Could not add dropped item group to canvas:", cause);
        setItemGroupError(cause instanceof Error ? cause.message : String(cause));
      }
    };

    const handleUpdateAstNode = (e: any) => {
      const { symbolName, signature, status } = e.detail;
      flow.saveHistorySnapshot();
      flow.setNodes((nds) =>
        nds.map((node) => {
          if (node.type === "forgeAst" && node.data?.name === symbolName) {
            return {
              ...node,
              data: {
                ...node.data,
                signature,
                status,
              },
            };
          }
          return node;
        })
      );
    };

    const handleHighlightImpactNodes = (e: any) => {
      const { dependents } = e.detail;
      flow.setNodes((nds) =>
        nds.map((node) => {
          if (node.type === "forgeAst") {
            const isNodeImpacted = dependents.some((depId: string) =>
              depId.endsWith(`:${node.data?.name}`)
            );
            return {
              ...node,
              data: {
                ...node.data,
                isImpacted: isNodeImpacted,
              },
            };
          }
          return node;
        })
      );
    };

    const handleUpdateCanvasChurnHeatmap = (e: any) => {
      const { metrics } = e.detail;
      if (!Array.isArray(metrics)) return;

      const churnMap = new Map<string, number>();
      metrics.forEach((m: any) => {
        churnMap.set(m.file_path, m.churn_score);
      });

      flow.setNodes((nds) =>
        nds.map((node) => {
          if (node.type === "forgeAst") {
            const nodeFile = (node.data?.filePath as string) || "";
            let score = 0;
            for (const [fPath, val] of churnMap.entries()) {
              if (nodeFile.endsWith(fPath) || fPath.endsWith(nodeFile)) {
                score = val;
                break;
              }
            }

            return {
              ...node,
              data: {
                ...node.data,
                churnScore: score,
              },
            };
          }
          return node;
        })
      );
    };

    const handleTimeTravelSnapshotEvent = (e: any) => {
      const { snapshotId } = e.detail;
      if (snapshotId) {
        flow.handleApplyResolvedSnapshot(snapshotId);
      }
    };

    const handleShapeResizeStart = (event: Event): void => {
      if (!(event instanceof CustomEvent) || typeof event.detail !== "string") return;
      const node = flow.nodes.find((item) => item.id === event.detail);
      const width = node?.measured?.width ?? node?.width;
      const height = node?.measured?.height ?? node?.height;
      if (!node || width === undefined || height === undefined) return;
      resizeDimensionsRef.current.set(node.id, { width, height });
      const position = { ...node.position };
      dragPositionsRef.current.set(node.id, { raw: position, normalized: position });
    };

    const handleShapeResizeEnd = (event: Event): void => {
      if (!(event instanceof CustomEvent) || typeof event.detail !== "string") return;
      resizeDimensionsRef.current.delete(event.detail);
      dragPositionsRef.current.delete(event.detail);
    };

    const handleSaveHistorySnapshot = (): void => flow.saveHistorySnapshot();

    window.addEventListener("vora:select-card", handleSelectCard);
    window.addEventListener("vora:remove-from-canvas", handleRemoveFromCanvas);
    window.addEventListener("vora:duplicate-shape", handleDuplicateShape);
    window.addEventListener("vora:toggle-shape-lock", handleToggleShapeLock);
    window.addEventListener("vora:update-shape", handleUpdateShape);
    window.addEventListener("vora:update-path-geometry", handleUpdatePathGeometry);
    window.addEventListener("vora:cut-path-segment", handleCutPathSegment);
    window.addEventListener("vora:join-selected-paths", handleJoinSelectedPaths);
    window.addEventListener("vora:add-path-segment-to-library", handleAddPathSegmentToLibraryEvent);
    window.addEventListener("vora:drop-card-to-canvas", handleDirectDropCard);
    window.addEventListener("vora:drop-item-group-to-canvas", handleDirectDropItemGroup);
    window.addEventListener("vora:open-fullscreen", handleOpenFullscreenEvent);
    window.addEventListener("vora:update-canvas-ast-node", handleUpdateAstNode);
    window.addEventListener("vora:highlight-impact-nodes", handleHighlightImpactNodes);
    window.addEventListener("vora:update-canvas-churn-heatmap", handleUpdateCanvasChurnHeatmap);
    window.addEventListener("vora:time-travel-snapshot", handleTimeTravelSnapshotEvent);
    window.addEventListener("vora:save-history-snapshot", handleSaveHistorySnapshot);
    window.addEventListener("vora:shape-resize-start", handleShapeResizeStart);
    window.addEventListener("vora:shape-resize-end", handleShapeResizeEnd);

    return () => {
      window.removeEventListener("vora:select-card", handleSelectCard);
      window.removeEventListener("vora:remove-from-canvas", handleRemoveFromCanvas);
      window.removeEventListener("vora:duplicate-shape", handleDuplicateShape);
      window.removeEventListener("vora:toggle-shape-lock", handleToggleShapeLock);
      window.removeEventListener("vora:update-shape", handleUpdateShape);
      window.removeEventListener("vora:update-path-geometry", handleUpdatePathGeometry);
      window.removeEventListener("vora:cut-path-segment", handleCutPathSegment);
      window.removeEventListener("vora:join-selected-paths", handleJoinSelectedPaths);
      window.removeEventListener("vora:add-path-segment-to-library", handleAddPathSegmentToLibraryEvent);
      window.removeEventListener("vora:drop-card-to-canvas", handleDirectDropCard);
      window.removeEventListener("vora:drop-item-group-to-canvas", handleDirectDropItemGroup);
      window.removeEventListener("vora:open-fullscreen", handleOpenFullscreenEvent);
      window.removeEventListener("vora:update-canvas-ast-node", handleUpdateAstNode);
      window.removeEventListener("vora:highlight-impact-nodes", handleHighlightImpactNodes);
      window.removeEventListener("vora:update-canvas-churn-heatmap", handleUpdateCanvasChurnHeatmap);
      window.removeEventListener("vora:time-travel-snapshot", handleTimeTravelSnapshotEvent);
      window.removeEventListener("vora:save-history-snapshot", handleSaveHistorySnapshot);
      window.removeEventListener("vora:shape-resize-start", handleShapeResizeStart);
      window.removeEventListener("vora:shape-resize-end", handleShapeResizeEnd);
    };
  }, [
    flow,
    selectedCardId,
    cards,
    screenToFlowPosition,
    canvasBoardId,
    libraryBoardSession,
    addCard,
    updateCardVersion,
    itemGroups,
  ]);

  useEffect(() => {
    const handleNavigatePathUsage = async (event: Event): Promise<void> => {
      if (libraryBoardSession || !(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const { boardId, nodeId, segmentId } = event.detail;
      if (
        typeof boardId !== "string" ||
        typeof nodeId !== "string" ||
        typeof segmentId !== "string"
      ) return;
      try {
        if (boardId !== canvasBoardId) {
          await flow.flushCanvas();
          setActiveBoardId(boardId);
          await flow.waitForBoardReady(boardId);
        }
        const targetNode = getNodes().find((node) => node.id === nodeId);
        if (!targetNode) {
          setItemGroupError("This Path section is no longer present on that Board.");
          return;
        }
        flow.setNodes((nodes) => nodes.map((node) => ({
          ...node,
          selected: node.id === nodeId,
        })));
        setCenter(
          targetNode.position.x + (targetNode.measured?.width ?? targetNode.width ?? 36) / 2,
          targetNode.position.y + (targetNode.measured?.height ?? targetNode.height ?? 36) / 2,
          { duration: 400 },
        );
        window.requestAnimationFrame(() => window.requestAnimationFrame(() => {
          window.dispatchEvent(new CustomEvent("vora:select-path-segment", {
            detail: { nodeId, segmentId },
          }));
        }));
      } catch (cause) {
        console.error("Could not navigate to the Path section usage:", cause);
        setItemGroupError(cause instanceof Error ? cause.message : String(cause));
      }
    };
    const handleNavigateEvent = (event: Event): void => {
      void handleNavigatePathUsage(event);
    };
    window.addEventListener("vora:navigate-path-usage", handleNavigateEvent);
    return () => window.removeEventListener("vora:navigate-path-usage", handleNavigateEvent);
  }, [canvasBoardId, flow, getNodes, libraryBoardSession, setActiveBoardId, setCenter]);

  useEffect(() => {
    const handleNavigateTextLink = async (event: Event): Promise<void> => {
      if (libraryBoardSession) return;
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const { boardId, cardId, field, start, end } = event.detail;
      if (
        typeof boardId !== "string" ||
        typeof cardId !== "string" ||
        (field !== "title" && field !== "content") ||
        typeof start !== "number" ||
        typeof end !== "number"
      ) return;

      try {
        if (boardId !== canvasBoardId) {
          await flow.flushCanvas();
          setActiveBoardId(boardId);
          await flow.waitForBoardReady(boardId);
        }
        await useDocumentStore.getState().loadDoc(cardId);
        setSelectedCardId(cardId);

        const targetNode = getNodes().find((node) => {
          const nodeCardId = isRecord(node.data) ? node.data.cardId : null;
          return node.id === cardId || nodeCardId === cardId;
        });
        if (targetNode) {
          flow.setNodes((nodes) => nodes.map((node) => ({
            ...node,
            selected: node.id === targetNode.id,
          })));
          setCenter(
            targetNode.position.x + (targetNode.measured?.width ?? targetNode.width ?? 320) / 2,
            targetNode.position.y + (targetNode.measured?.height ?? targetNode.height ?? 180) / 2,
            { duration: 400 },
          );
        }

        await new Promise<void>((resolve) => {
          window.requestAnimationFrame(() => window.requestAnimationFrame(() => resolve()));
        });
        window.dispatchEvent(new CustomEvent("vora:select-text-range", {
          detail: { cardId, field, start, end },
        }));
      } catch (error) {
        console.error("Failed to navigate to linked text:", error);
      }
    };

    window.addEventListener("vora:navigate-text-link", handleNavigateTextLink);
    return () => window.removeEventListener("vora:navigate-text-link", handleNavigateTextLink);
  }, [canvasBoardId, flow, getNodes, libraryBoardSession, setActiveBoardId, setCenter]);

  useEffect(() => {
    try {
      setSetting(SHAPE_DEFAULTS_KEY, JSON.stringify(shapeDefaults));
    } catch (error) {
      console.error("Could not save canvas shape defaults", error);
    }
  }, [shapeDefaults]);

  const handleCanvasWheel = (event: WheelEvent): void => {
    if (!event.target || !(event.target instanceof Element)) return;
    if (event.target.closest("[data-text-link-picker]")) return;
    if (!event.target.closest(".react-flow__pane, .react-flow__viewport, .react-flow__renderer")) return;
    if (event.deltaY === 0) return;

    if (event.ctrlKey) {
      event.preventDefault();
      return;
    }

    const delta = Math.abs(event.deltaY) > 0 ? Math.min(0.6, Math.max(0.06, Math.abs(event.deltaY) / 120 * 0.2)) : 0.1;
    const signedDelta = event.deltaY < 0 ? delta : -delta;

    event.preventDefault();
    changeCanvasZoom(signedDelta, { x: event.clientX, y: event.clientY });
  };

  useEffect(() => {
    const canvas = flow.reactFlowWrapper.current;
    if (!canvas) return;

    canvas.addEventListener("wheel", handleCanvasWheel, { capture: true, passive: false });
    return () => canvas.removeEventListener("wheel", handleCanvasWheel, true);
  }, [flow.reactFlowWrapper, handleCanvasWheel]);

  const handleCanvasClick = (event: React.MouseEvent) => {
    setActiveCanvasGroupId(null);
    if (isAddCardToolActive) {
      if (!canvasBoardId) return;
      flow.handleAddNewCardOnCanvas(toCanvasFlowPosition({
        x: event.clientX,
        y: event.clientY,
      }));
      setIsAddCardToolActive(false);
      return;
    }

    if (!canvasBoardId || !isTextToolActive) return;

    const position = toCanvasFlowPosition({
      x: event.clientX,
      y: event.clientY,
    });

    addTextNodeAtPosition(position);
    setIsTextToolActive(false);
  };

  const handleCanvasContextMenu = (event: React.MouseEvent<HTMLDivElement>): void => {
    if (!(event.target instanceof Element) || !event.target.closest(".react-flow__pane")) return;
    if (event.target.closest(".react-flow__node, .react-flow__edge, .react-flow__controls, .react-flow__minimap")) return;

    if (activeShapeTool !== null || shapePreview !== null || isTextToolActive || isAddCardToolActive) {
      event.preventDefault();
      setActiveShapeTool(null);
      setIsTextToolActive(false);
      setIsAddCardToolActive(false);
      rectangleStartRef.current = null;
      pathPointsRef.current = [];
      setShapePreview(null);
    }

    flow.setNodes((nodes) => nodes.some((node) => node.selected)
      ? nodes.map((node) => node.selected ? { ...node, selected: false } : node)
      : nodes);
    flow.setEdges((edges) => edges.some((edge) => edge.selected)
      ? edges.map((edge) => edge.selected ? { ...edge, selected: false } : edge)
      : edges);
    setSelectedCardId(null);
  };

  const handleCanvasNodeContextMenu = (
    event: React.MouseEvent,
    node: Node,
  ): void => {
    event.preventDefault();
    event.stopPropagation();
    setActiveCanvasGroupId(getCanvasGroupId(node));
    if (!node.selected) {
      flow.setNodes((nodes) => nodes.map((candidate) => ({
        ...candidate,
        selected: candidate.id === node.id,
      })));
      flow.setEdges((edges) => edges.map((edge) => ({ ...edge, selected: false })));
    }
    setCanvasNodeContextMenu({ x: event.clientX, y: event.clientY });
  };

  const handleCanvasNodeClick = (_event: React.MouseEvent, node: Node): void => {
    const groupId = getCanvasGroupId(node);
    setActiveCanvasGroupId(groupId);
    if (!groupId) return;
    flow.setNodes((nodes) => nodes.map((candidate) => ({
      ...candidate,
      selected: false,
    })));
    flow.setEdges((edges) => edges.map((edge) => ({ ...edge, selected: false })));
  };

  const handleCanvasNodeDoubleClick = (_event: React.MouseEvent, node: Node): void => {
    if (!getCanvasGroupId(node)) return;
    setActiveCanvasGroupId(null);
    flow.setNodes((nodes) => nodes.map((candidate) => ({
      ...candidate,
      selected: candidate.id === node.id,
    })));
    flow.setEdges((edges) => edges.map((edge) => ({ ...edge, selected: false })));
  };

  const handleAddCanvasSelectionToLibrary = (): void => {
    setCanvasNodeContextMenu(null);
    setIsCreateItemGroupOpen(true);
  };

  const handleGroupCanvasSelection = (): void => {
    if (selectedNodes.length < 2) return;
    const selectedIds = new Set(selectedNodes.map((node) => node.id));
    flow.nodes.forEach((node) => {
      if (selectedGroupIds.has(getCanvasGroupId(node) ?? "")) selectedIds.add(node.id);
    });
    const membership: CanvasGroupMembership = { canvasGroupId: crypto.randomUUID() };
    flow.saveHistorySnapshot();
    flow.setNodes((nodes) => nodes.map((node) => selectedIds.has(node.id)
      ? { ...node, data: { ...node.data, ...membership }, selected: false }
      : node));
    flow.setEdges((edges) => edges.map((edge) => ({ ...edge, selected: false })));
    setActiveCanvasGroupId(membership.canvasGroupId);
    setCanvasNodeContextMenu(null);
  };

  const handleUngroupCanvasSelection = (): void => {
    if (selectedGroupIds.size === 0) return;
    flow.saveHistorySnapshot();
    flow.setNodes((nodes) => nodes.map((node) => {
      if (!selectedGroupIds.has(getCanvasGroupId(node) ?? "")) return node;
      const data = { ...node.data };
      delete data.canvasGroupId;
      return { ...node, data, selected: false };
    }));
    setActiveCanvasGroupId(null);
    setCanvasNodeContextMenu(null);
  };

  useEffect(() => {
    const handleGroupSelection = (): void => handleGroupCanvasSelection();
    const handleUngroupSelection = (): void => handleUngroupCanvasSelection();
    window.addEventListener("vora:group-canvas-selection", handleGroupSelection);
    window.addEventListener("vora:ungroup-canvas-selection", handleUngroupSelection);
    return () => {
      window.removeEventListener("vora:group-canvas-selection", handleGroupSelection);
      window.removeEventListener("vora:ungroup-canvas-selection", handleUngroupSelection);
    };
  }, [handleGroupCanvasSelection, handleUngroupCanvasSelection]);

  const handleAddTextLink = (): void => {
    const selection = canvasNodeContextMenu?.textLinkSelection;
    setCanvasNodeContextMenu(null);
    if (!selection) return;
    window.dispatchEvent(new CustomEvent("vora:create-text-link", { detail: selection }));
  };

  useEffect(() => {
    const handleTextContextMenu = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const detail = event.detail;
      if (
        typeof detail.cardId !== "string" ||
        (detail.field !== "title" && detail.field !== "content") ||
        typeof detail.start !== "number" ||
        !Number.isInteger(detail.start) ||
        typeof detail.end !== "number" ||
        !Number.isInteger(detail.end) ||
        detail.start < 0 ||
        detail.end <= detail.start ||
        typeof detail.x !== "number" ||
        !Number.isFinite(detail.x) ||
        typeof detail.y !== "number" ||
        !Number.isFinite(detail.y)
      ) return;
      const selection: TextLinkSelection = {
        cardId: detail.cardId,
        field: detail.field,
        start: detail.start,
        end: detail.end,
      };
      setCanvasNodeContextMenu({ x: detail.x, y: detail.y, textLinkSelection: selection });
    };
    window.addEventListener("vora:open-text-context-menu", handleTextContextMenu);
    return () => window.removeEventListener("vora:open-text-context-menu", handleTextContextMenu);
  }, [canvasBoardId, libraryBoardSession]);

  const canvasShapePoint = (event: React.PointerEvent<HTMLDivElement>): CanvasShapePoint | null => {
    const bounds = flow.reactFlowWrapper.current?.getBoundingClientRect();
    if (!bounds) return null;
    return {
      flow: toCanvasFlowPosition({ x: event.clientX, y: event.clientY }),
      screen: { x: event.clientX - bounds.left, y: event.clientY - bounds.top },
    };
  };

  const addTextNodeAtPosition = (position: Node["position"]): string => {
    if (!canvasBoardId) return "";
    flow.saveHistorySnapshot();
    const newId = `text-${canvasBoardId}-${Date.now()}-${Math.floor(Math.random() * 10000)}`;
    const newTextNode: Node = {
      id: newId,
      type: "voraText",
      position,
      selected: true,
      data: { text: "", autoFocus: true },
    };
    flow.setNodes((nodes) => [
      ...nodes.map((node): Node => ({ ...node, selected: false })),
      newTextNode,
    ]);
    return newId;
  };

  const handleConnectEnd: OnConnectEnd = (event, connectionState) => {
    const sourceNode = connectionState.fromNode;
    const sourceHandle = connectionState.fromHandle;
    if (
      !canvasBoardId ||
      !sourceNode ||
      !sourceHandle?.id ||
      connectionState.toNode ||
      (sourceNode.type !== "voraCard" && sourceNode.type !== "voraText") ||
      (sourceHandle.type !== "source" && sourceHandle.type !== "target")
    ) return;

    const pointer = event instanceof MouseEvent
      ? { x: event.clientX, y: event.clientY }
      : event.changedTouches.length > 0
        ? { x: event.changedTouches[0]!.clientX, y: event.changedTouches[0]!.clientY }
        : null;
    const bounds = flow.reactFlowWrapper.current?.getBoundingClientRect();
    if (
      !pointer ||
      !bounds ||
      pointer.x < bounds.left ||
      pointer.x > bounds.right ||
      pointer.y < bounds.top ||
      pointer.y > bounds.bottom
    ) return;

    setConnectionCreateMenu({
      x: pointer.x,
      y: pointer.y,
      position: toCanvasFlowPosition(pointer),
      sourcePoint: connectionState.from,
      sourceNodeId: sourceNode.id,
      sourceHandleId: sourceHandle.id,
      sourceHandleType: sourceHandle.type,
      boardId: canvasBoardId,
    });
  };

  const handleCreateConnectedNode = (nodeType: "card" | "text"): void => {
    const connection = connectionCreateMenu;
    if (
      !connection ||
      connection.boardId !== canvasBoardId ||
      !flow.nodes.some((node) => node.id === connection.sourceNodeId)
    ) {
      setConnectionCreateMenu(null);
      return;
    }

    const nodeId = nodeType === "card"
      ? flow.handleAddNewCardOnCanvas(connection.position)
      : addTextNodeAtPosition(connection.position);
    if (!nodeId) {
      setConnectionCreateMenu(null);
      return;
    }

    const newNodeHandlePosition = getFacingHandlePosition(connection.sourcePoint, connection.position);
    const edge: Edge = {
      id: `connection-${connection.sourceNodeId}-${nodeId}-${Date.now()}`,
      source: connection.sourceHandleType === "source" ? connection.sourceNodeId : nodeId,
      sourceHandle: connection.sourceHandleType === "source"
        ? connection.sourceHandleId
        : getHandleId("source", newNodeHandlePosition),
      target: connection.sourceHandleType === "source" ? nodeId : connection.sourceNodeId,
      targetHandle: connection.sourceHandleType === "source"
        ? getHandleId("target", newNodeHandlePosition)
        : connection.sourceHandleId,
      type: "default",
      style: { stroke: "#4b5563", strokeWidth: 2 },
    };
    flow.setEdges((edges) => [...edges, edge]);
    setConnectionCreateMenu(null);
  };

  useEffect(() => {
    if (!connectionCreateMenu) return;
    const closeOnEscape = (event: KeyboardEvent): void => {
      if (event.key === "Escape") setConnectionCreateMenu(null);
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [connectionCreateMenu]);

  const addShapeNode = (shape: "rectangle" | "path", points: VoraShapePoint[]): void => {
    if (!canvasBoardId) return;
    if (points.length < (shape === "path" ? 2 : 2)) return;
    const xs = points.map((point) => point.x);
    const ys = points.map((point) => point.y);
    const minX = Math.min(...xs);
    const minY = Math.min(...ys);
    const spanX = Math.max(1, Math.max(...xs) - minX);
    const spanY = Math.max(1, Math.max(...ys) - minY);
    const width = Math.max(shape === "rectangle" ? 48 : 36, spanX);
    const height = Math.max(shape === "rectangle" ? 36 : 36, spanY);
    const normalizedPoints = shape === "path"
      ? points.map((point) => ({ x: spanX === 1 ? 0.5 : (point.x - minX) / spanX, y: spanY === 1 ? 0.5 : (point.y - minY) / spanY }))
      : [];
    const settings = shapeDefaults[shape];
    const data: VoraShapeData = {
      shape,
      label: shape === "rectangle" ? "Rectangle" : "Path",
      ...settings,
      points: normalizedPoints,
      segments: shape === "path"
        ? normalizedPoints.slice(0, -1).map((_, index): VoraPathSegment => ({
          id: `path-segment-${crypto.randomUUID()}`,
          name: `Section ${index + 1}`,
          details: "",
          color: settings.color,
        }))
        : [],
    };
    flow.saveHistorySnapshot();
    const node: Node = {
      id: `${shape}-${canvasBoardId}-${Date.now()}-${Math.floor(Math.random() * 10000)}`,
      type: "voraShape",
      className: shape === "path" ? "vora-path-node vora-shape-border-hit" : "vora-shape-border-hit",
      position: { x: minX, y: minY },
      width,
      height,
      selected: true,
      data,
    };
    flow.setNodes((nodes) => [...nodes.map((item) => ({ ...item, selected: false })), node]);
  };

  const isPanePointer = (event: React.PointerEvent<HTMLDivElement>): boolean => {
    if (!(event.target instanceof Element)) return false;
    return event.target.closest(".react-flow__pane") !== null
      && event.target.closest(".react-flow__node, .react-flow__edge, .react-flow__controls, .react-flow__minimap") === null;
  };

  const handleCanvasPointerDown = (event: React.PointerEvent<HTMLDivElement>): void => {
    if (!canvasBoardId) return;
    const target = event.target instanceof Element ? event.target : null;
    const clickedTextNode = target?.closest(".react-flow__node-voraText");
    const emptyTextNodeIds = [...(flow.reactFlowWrapper.current?.querySelectorAll<HTMLElement>(
      ".react-flow__node-voraText[data-id]"
    ) ?? [])].flatMap((nodeElement) => {
      if (nodeElement === clickedTextNode) return [];
      const textEditor = nodeElement.querySelector<HTMLTextAreaElement>("textarea");
      const nodeId = nodeElement.getAttribute("data-id");
      return textEditor && textEditor.value.trim().length === 0 && nodeId ? [nodeId] : [];
    });
    if (emptyTextNodeIds.length > 0) {
      window.dispatchEvent(new CustomEvent("vora:remove-from-canvas", {
        detail: { nodeIds: emptyTextNodeIds },
      }));
    }
    if (event.button !== 0 || !isPanePointer(event)) return;
    if (isSpacePanActiveRef.current) return;
    if (!activeShapeTool && !isTextToolActive && !isAddCardToolActive) {
      const bounds = flow.reactFlowWrapper.current?.getBoundingClientRect();
      if (!bounds) return;
      marqueeRef.current = {
        start: { x: event.clientX, y: event.clientY },
        additive: event.shiftKey || event.ctrlKey || event.metaKey,
      };
      setMarqueePreview({
        left: event.clientX - bounds.left,
        top: event.clientY - bounds.top,
        width: 0,
        height: 0,
      });
      event.preventDefault();
      event.stopPropagation();
      event.currentTarget.setPointerCapture(event.pointerId);
      return;
    }
    if (!activeShapeTool) return;
    const point = canvasShapePoint(event);
    if (!point) return;
    event.preventDefault();
    event.stopPropagation();
    if (activeShapeTool === "rectangle") {
      rectangleStartRef.current = point;
      setShapePreview({ shape: "rectangle", points: [point.screen, point.screen] });
      event.currentTarget.setPointerCapture(event.pointerId);
      return;
    }
    pathPointsRef.current = [...pathPointsRef.current, point];
    setShapePreview({ shape: "path", points: pathPointsRef.current.map((item) => item.screen) });
  };

  const handleCanvasPointerMove = (event: React.PointerEvent<HTMLDivElement>): void => {
    if (!canvasBoardId) return;
    const marquee = marqueeRef.current;
    if (marquee) {
      const bounds = flow.reactFlowWrapper.current?.getBoundingClientRect();
      if (!bounds) return;
      setMarqueePreview({
        left: Math.min(marquee.start.x, event.clientX) - bounds.left,
        top: Math.min(marquee.start.y, event.clientY) - bounds.top,
        width: Math.abs(event.clientX - marquee.start.x),
        height: Math.abs(event.clientY - marquee.start.y),
      });
      return;
    }
    if (!activeShapeTool) return;
    const point = canvasShapePoint(event);
    if (!point) return;
    if (activeShapeTool === "rectangle" && rectangleStartRef.current) {
      setShapePreview({ shape: "rectangle", points: [rectangleStartRef.current.screen, point.screen] });
    } else if (activeShapeTool === "path" && pathPointsRef.current.length > 0) {
      setShapePreview({ shape: "path", points: pathPointsRef.current.map((item) => item.screen).concat(point.screen) });
    }
  };

  const handleCanvasPointerUp = (event: React.PointerEvent<HTMLDivElement>): void => {
    if (!canvasBoardId) return;
    const marquee = marqueeRef.current;
    if (marquee) {
      marqueeRef.current = null;
      setMarqueePreview(null);
      const selectionBounds = {
        left: Math.min(marquee.start.x, event.clientX),
        top: Math.min(marquee.start.y, event.clientY),
        right: Math.max(marquee.start.x, event.clientX),
        bottom: Math.max(marquee.start.y, event.clientY),
      };
      const selectedNodeIds = new Set<string>();
      flow.reactFlowWrapper.current
        ?.querySelectorAll<HTMLElement>(".react-flow__node[data-id]")
        .forEach((element) => {
          const nodeId = element.getAttribute("data-id");
          const bounds = element.getBoundingClientRect();
          if (
            nodeId &&
            bounds.right >= selectionBounds.left &&
            bounds.left <= selectionBounds.right &&
            bounds.bottom >= selectionBounds.top &&
            bounds.top <= selectionBounds.bottom
          ) selectedNodeIds.add(nodeId);
        });
      const selectedIds = new Set([
        ...selectedNodeIds,
        ...(marquee.additive ? flow.nodes.filter((node) => node.selected).map((node) => node.id) : []),
      ]);
      setActiveCanvasGroupId(null);
      flow.setNodes((nodes) => nodes.map((node) => ({
        ...node,
        selected: selectedIds.has(node.id),
      })));
      return;
    }

    const start = rectangleStartRef.current;
    if (!start || activeShapeTool !== "rectangle") return;
    const end = canvasShapePoint(event);
    rectangleStartRef.current = null;
    setShapePreview(null);
    if (!end || Math.hypot(end.screen.x - start.screen.x, end.screen.y - start.screen.y) < 4) return;
    addShapeNode("rectangle", [start.flow, end.flow]);
    setActiveShapeTool(null);
  };

  const handleNodesChange = (changes: NodeChange<Node>[]): void => {
    const nodeById = new Map(flow.nodes.map((node) => [node.id, node]));
    const normalizedChanges = changes.map((change) => {
      if (change.type === "add" || change.type === "remove" || change.type === "replace") return change;
      const currentNode = nodeById.get(change.id);
      if (!currentNode) return change;
      if (change.type === "dimensions" && change.dimensions) {
        const resizeStart = resizeDimensionsRef.current.get(change.id);
        if (resizeStart) {
          return {
            ...change,
            dimensions: {
              width: resizeStart.width + (change.dimensions.width - resizeStart.width) / zoomScale,
              height: resizeStart.height + (change.dimensions.height - resizeStart.height) / zoomScale,
            },
          };
        }
        const currentWidth = currentNode.measured?.width ?? currentNode.width;
        const currentHeight = currentNode.measured?.height ?? currentNode.height;
        if (currentWidth === undefined || currentHeight === undefined) return change;
        return {
          ...change,
          dimensions: {
            width: currentWidth + (change.dimensions.width - currentWidth) / zoomScale,
            height: currentHeight + (change.dimensions.height - currentHeight) / zoomScale,
          },
        };
      }
      if (change.type !== "position" || !change.position) return change;
      const dragPosition = dragPositionsRef.current.get(change.id);
      if (dragPosition) {
        const normalizedPosition = {
          x: dragPosition.normalized.x + (change.position.x - dragPosition.raw.x) / zoomScale,
          y: dragPosition.normalized.y + (change.position.y - dragPosition.raw.y) / zoomScale,
        };
        dragPosition.raw = change.position;
        dragPosition.normalized = normalizedPosition;
        return { ...change, position: normalizedPosition };
      }
      return {
        ...change,
        position: {
          x: currentNode.position.x + (change.position.x - currentNode.position.x) / zoomScale,
          y: currentNode.position.y + (change.position.y - currentNode.position.y) / zoomScale,
        },
      };
    });
    const selectedGroupIdsFromChanges = new Set(
      changes.flatMap((change) => {
        if (change.type !== "select" || !change.selected) return [];
        const changedNode = nodeById.get(change.id);
        const groupId = changedNode ? getCanvasGroupId(changedNode) : null;
        return groupId ? [groupId] : [];
      })
    );
    if (selectedGroupIdsFromChanges.size === 1) {
      const [groupId] = selectedGroupIdsFromChanges;
      setActiveCanvasGroupId(groupId ?? null);
    }
    const groupSelectionChanges: NodeChange<Node>[] = flow.nodes
      .filter((node) => selectedGroupIdsFromChanges.has(getCanvasGroupId(node) ?? "") && node.selected)
      .map((node) => ({ id: node.id, type: "select", selected: false }));
    const changesWithoutGroupedSelection = normalizedChanges.map((change) => {
      if (change.type !== "select" || !change.selected) return change;
      const changedNode = nodeById.get(change.id);
      return changedNode && selectedGroupIdsFromChanges.has(getCanvasGroupId(changedNode) ?? "")
        ? { ...change, selected: false }
        : change;
    });
    flow.onNodesChange([...changesWithoutGroupedSelection, ...groupSelectionChanges]);
  };

  const finishPath = (): void => {
    const points = [...pathPointsRef.current];
    pathPointsRef.current = [];
    setShapePreview(null);
    if (points.length > 2 && Math.hypot(
      (points[points.length - 1]?.flow.x ?? 0) - (points[points.length - 2]?.flow.x ?? 0),
      (points[points.length - 1]?.flow.y ?? 0) - (points[points.length - 2]?.flow.y ?? 0),
    ) < 1) points.pop();
    if (points.length < 2) return;
    addShapeNode("path", points.map((point) => point.flow));
    setActiveShapeTool(null);
  };

  const handleCanvasDoubleClick = (event: React.MouseEvent<HTMLDivElement>): void => {
    if (activeShapeTool !== "path" || !(event.target instanceof Element) || !event.target.closest(".react-flow__pane")) return;
    event.preventDefault();
    event.stopPropagation();
    finishPath();
  };

  const handleCanvasKeyDown = (event: React.KeyboardEvent<HTMLDivElement>): void => {
    if (event.key === "Enter" && activeShapeTool === "path") {
      event.preventDefault();
      finishPath();
    }
  };

  const handleNodeDragStart = (_event: MouseEvent | TouchEvent, node: Node): void => {
    groupDragRef.current = null;
    dragPositionsRef.current.clear();
    flow.nodes.forEach((item) => {
      const position = { ...item.position };
      dragPositionsRef.current.set(item.id, { raw: position, normalized: position });
    });
    if (!dragPositionsRef.current.has(node.id)) {
      const position = { ...node.position };
      dragPositionsRef.current.set(node.id, { raw: position, normalized: position });
    }
    const groupId = getCanvasGroupId(node);
    flow.saveHistorySnapshot();
    if (!groupId || node.selected) return;

    setActiveCanvasGroupId(groupId);
    groupDragRef.current = {
      groupId,
      nodeId: node.id,
      startPosition: { ...node.position },
      positions: new Map(
        flow.nodes
          .filter((item) => getCanvasGroupId(item) === groupId)
          .map((item) => [item.id, { ...item.position }])
      ),
    };
  };

  const handleNodeDrag = (_event: MouseEvent | TouchEvent, node: Node): void => {
    const groupDrag = groupDragRef.current;
    if (!groupDrag || groupDrag.nodeId !== node.id) return;
    const deltaX = (node.position.x - groupDrag.startPosition.x) / zoomScale;
    const deltaY = (node.position.y - groupDrag.startPosition.y) / zoomScale;
    flow.setNodes((nodes) => nodes.map((item) => {
      const position = groupDrag.positions.get(item.id);
      return position
        ? { ...item, position: { x: position.x + deltaX, y: position.y + deltaY } }
        : item;
    }));
  };

  const handleNodeDragStop = (_event: MouseEvent | TouchEvent, _node: Node): void => {
    dragPositionsRef.current.clear();
    groupDragRef.current = null;
  };

  useEffect(() => {
    if (activeShapeTool !== "path") return;
    const handlePathFinish = (event: KeyboardEvent): void => {
      if (event.key !== "Enter") return;
      if (event.target instanceof HTMLElement && ["INPUT", "TEXTAREA", "SELECT"].includes(event.target.tagName)) return;
      event.preventDefault();
      finishPath();
    };
    document.addEventListener("keydown", handlePathFinish);
    return () => document.removeEventListener("keydown", handlePathFinish);
  }, [activeShapeTool, finishPath]);

  const handleLeftClick = () => {
    const next = activeOpenPanel === "none"
      ? lastActivePanel
      : activeOpenPanel === "boards" ? "sidebar" : "boards";
    if (next === "boards") {
      setIsTaskFlowOpen(false);
      setIsAiTasksOpen(false);
      setIsVoraTraceOpen(false);
      setIsJournalOpen(false);
      setIsQuestionsBoardOpen(false);
    }
    setActiveOpenPanel(next);
    setLastActivePanel(next);
  };

  const handleLeftContextMenu = (event: React.MouseEvent<HTMLButtonElement>): void => {
    event.preventDefault();
    const next = activeOpenPanel === "none" ? lastActivePanel : "none";
    if (next === "boards") {
      setIsTaskFlowOpen(false);
      setIsAiTasksOpen(false);
      setIsVoraTraceOpen(false);
      setIsJournalOpen(false);
      setIsQuestionsBoardOpen(false);
    }
    setActiveOpenPanel(next);
    if (next !== "none") setLastActivePanel(next);
  };

  const handleNodeDataSync = (cardId: string, title: string, content: string) => {
    flow.setNodes((nds) =>
      nds.map((n) =>
        n.id === cardId || n.data?.cardId === cardId
          ? { ...n, data: { ...n.data, label: title, title, content } }
          : n
      )
    );
  };

  const handleCreateCardInLibraryOnly = () => {
    if (!activeBoardId) {
      setActiveOpenPanel("boards");
      setLastActivePanel("boards");
      return;
    }
    const newId = addCard();
    setSelectedCardId(newId);
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target;
      const isEditableTarget = target instanceof HTMLElement && (
        target.isContentEditable ||
        target.closest("input, textarea, select, [contenteditable='true']") !== null
      );
      const isCreateGroupShortcut = e.code === "KeyG" || e.key.toLowerCase() === "g" || e.key === "ل";
      const isSelectAllShortcut =
        e.code === "KeyA" || e.key.toLowerCase() === "a" || e.key === "ش";

      if (
        activeCanvasGroupId &&
        (e.ctrlKey || e.metaKey) &&
        !e.altKey &&
        !e.shiftKey &&
        !isEditableTarget
      ) {
        const groupNodes = flow.nodes.filter((node) => getCanvasGroupId(node) === activeCanvasGroupId);
        if (e.key.toLowerCase() === "c" || e.key === "ز") {
          if (flow.copyNodesToClipboard(groupNodes)) {
            e.preventDefault();
            e.stopPropagation();
            return;
          }
        } else if (e.key.toLowerCase() === "x" || e.key === "ط") {
          if (flow.cutNodes(groupNodes)) {
            setActiveCanvasGroupId(null);
            e.preventDefault();
            e.stopPropagation();
            return;
          }
        }
      }

      if (
        (e.ctrlKey || e.metaKey) &&
        !e.altKey &&
        !e.shiftKey &&
        isCreateGroupShortcut &&
        selectedNodes.length > 0 &&
        !isEditableTarget
      ) {
        e.preventDefault();
        e.stopPropagation();
        setCanvasNodeContextMenu(null);
        setIsCreateItemGroupOpen(true);
        return;
      }

      if (
        (e.ctrlKey || e.metaKey) &&
        !e.altKey &&
        !e.shiftKey &&
        isSelectAllShortcut &&
        !isEditableTarget
      ) {
        e.preventDefault();
        e.stopPropagation();
        flow.setNodes((nodes) => nodes.map((node) => ({ ...node, selected: true })));
        flow.setEdges((edges) => edges.map((edge) => ({ ...edge, selected: true })));
        return;
      }

      if (!libraryBoardSession && activeBoardId && (e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "s") {
        e.preventDefault();
        flow.setIsCommitModalOpen(true);
      } else if ((e.ctrlKey || e.metaKey) && e.key === "/") {
        e.preventDefault();
        setActiveOpenPanel((prev) => (prev !== "none" ? "none" : lastActivePanel));
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [activeBoardId, activeCanvasGroupId, flow, lastActivePanel, libraryBoardSession, selectedNodes.length]);

  const previewSettings = shapePreview ? shapeDefaults[shapePreview.shape] : null;
  const previewDashArray = previewSettings?.lineStyle === "dashed"
    ? `${previewSettings.lineWidth * 4} ${previewSettings.lineWidth * 3}`
    : previewSettings?.lineStyle === "dotted"
      ? `${previewSettings.lineWidth} ${previewSettings.lineWidth * 2}`
      : undefined;
  const floatingControlsLeft = activeOpenPanel === "none" ? 14 : "calc(14rem + 10px)";
  const canvasControlsLeft = activeOpenPanel === "none" ? 16 : "calc(14rem + 16px)";

  return (
    <div className="vora-mind-theme flex h-screen w-screen bg-background text-gray-200 overflow-hidden select-none font-sans relative">
    {/* تولبار ابزارها */}
    {!isWorkspacePageOpen && canvasBoardId && <div
      className="absolute top-[117.5px] z-40 flex flex-col gap-1 p-1 bg-[#181818]/90 border border-[#2a2a2a] rounded-xl shadow-lg backdrop-blur transition-all duration-200"
      style={{ left: floatingControlsLeft }}
    >
      <button
          type="button"
          aria-label="Add New Card"
          aria-pressed={isAddCardToolActive}
          onClick={() => {
            setIsTextToolActive(false);
            setActiveShapeTool(null);
            rectangleStartRef.current = null;
            pathPointsRef.current = [];
            setShapePreview(null);
            setIsAddCardToolActive((active) => !active);
          }}
          className={`${CANVAS_TOOL_BUTTON_CLASS} ${isAddCardToolActive ? CANVAS_TOOL_BUTTON_ACTIVE_CLASS : CANVAS_TOOL_BUTTON_IDLE_CLASS}`}
          title="Add New Card"
        >
          <PlusSquare size={16} />
        </button>
        <button
          type="button"
          aria-label="Rectangle tool"
          aria-pressed={activeShapeTool === "rectangle"}
          onClick={() => {
            setIsAddCardToolActive(false);
            setIsTextToolActive(false);
            pathPointsRef.current = [];
            rectangleStartRef.current = null;
            setShapePreview(null);
            setActiveShapeTool((tool) => tool === "rectangle" ? null : "rectangle");
          }}
          className={`${CANVAS_TOOL_BUTTON_CLASS} ${activeShapeTool === "rectangle" ? CANVAS_TOOL_BUTTON_ACTIVE_CLASS : CANVAS_TOOL_BUTTON_IDLE_CLASS}`}
          title="Rectangle (drag to draw)"
        >
          <RectangleToolIcon size={16} />
        </button>
        <button
          type="button"
          aria-label="Path tool"
          aria-pressed={activeShapeTool === "path"}
          onClick={() => {
            setIsAddCardToolActive(false);
            setIsTextToolActive(false);
            rectangleStartRef.current = null;
            pathPointsRef.current = [];
            setShapePreview(null);
            setActiveShapeTool((tool) => tool === "path" ? null : "path");
          }}
          className={`${CANVAS_TOOL_BUTTON_CLASS} ${activeShapeTool === "path" ? CANVAS_TOOL_BUTTON_ACTIVE_CLASS : CANVAS_TOOL_BUTTON_IDLE_CLASS}`}
          title="Path (click to add points; double-click or Enter to finish)"
        >
          <PathToolIcon size={16} />
        </button>
        <button
          onClick={() => {
            setIsAddCardToolActive(false);
            setActiveShapeTool(null);
            rectangleStartRef.current = null;
            pathPointsRef.current = [];
            setShapePreview(null);
            setIsTextToolActive((prev) => !prev);
          }}
          className={`${CANVAS_TOOL_BUTTON_CLASS} ${isTextToolActive ? CANVAS_TOOL_BUTTON_ACTIVE_CLASS : CANVAS_TOOL_BUTTON_IDLE_CLASS}`}
          title="Text Tool (Click canvas to place text)"
        >
          <Type size={16} />
        </button>
      </div>}

      <div className="absolute top-3.5 z-40 flex w-[57px] justify-center" style={{ left: floatingControlsLeft }}>
          <button
            onClick={handleLeftClick}
            onContextMenu={handleLeftContextMenu}
            className={`${CANVAS_TOOL_BUTTON_CLASS} ${CANVAS_FLOATING_CONTROL_BORDER_CLASS} ${CANVAS_FLOATING_CONTROL_BACKGROUND_CLASS} ${CANVAS_TOOL_BUTTON_IDLE_CLASS}`}
            title="Left click: Switch menu | Right click: Open or close menu"
            aria-label={activeOpenPanel === "sidebar" ? "Switch to boards" : "Switch menu"}
          >
            {activeOpenPanel === "sidebar" ? <PanelLeft size={16} /> : <LayoutGrid size={16} />}
          </button>
      </div>
      {!isWorkspacePageOpen && <div className="absolute top-[67.5px] z-40 flex w-[57px] justify-center" style={{ left: floatingControlsLeft }}>
        <button
          type="button"
          onClick={toggleForgeAgent}
          className={`${CANVAS_TOOL_BUTTON_CLASS} ${CANVAS_FLOATING_CONTROL_BORDER_CLASS} ${CANVAS_FLOATING_CONTROL_BACKGROUND_CLASS} ${CANVAS_TOOL_BUTTON_IDLE_CLASS}`}
          title="AI Agent"
          aria-label="AI Agent"
        >
          <Sparkles size={23} />
        </button>
      </div>}
      {activeOpenPanel !== "none" && !libraryBoardSession && (
        <div className="absolute top-0 left-0 bottom-0 z-30 flex shadow-2xl animate-in fade-in slide-in-from-left duration-150">
          {activeOpenPanel === "sidebar" ? (
            <LeftSidebar
              activeOpenPanel={activeOpenPanel}
              hasActiveBoard={Boolean(activeBoardId)}
              isLibraryOpen={isRightDrawerOpen}
              isTaskFlowOpen={isTaskFlowOpen}
              isAiTasksOpen={isAiTasksOpen}
              isVoraTraceOpen={isVoraTraceOpen}
              isJournalOpen={isJournalOpen}
              isQuestionsBoardOpen={isQuestionsBoardOpen}
              onAddNewCard={handleCreateCardInLibraryOnly}
              onToggleLibrary={toggleRightDrawer}
              onOpenTaskFlow={() => {
                setIsAiTasksOpen(false);
                setIsVoraTraceOpen(false);
                setIsJournalOpen(false);
                setIsQuestionsBoardOpen(false);
                setIsTaskFlowOpen(true);
                setLastActivePanel("sidebar");
                setActiveOpenPanel("sidebar");
              }}
              onOpenAiTasks={() => {
                setIsTaskFlowOpen(false);
                setIsVoraTraceOpen(false);
                setIsJournalOpen(false);
                setIsQuestionsBoardOpen(false);
                setIsAiTasksOpen(true);
                setLastActivePanel("sidebar");
                setActiveOpenPanel("sidebar");
              }}
              onOpenVoraTrace={() => {
                setIsTaskFlowOpen(false);
                setIsAiTasksOpen(false);
                setIsJournalOpen(false);
                setIsQuestionsBoardOpen(false);
                setIsVoraTraceOpen(true);
                setLastActivePanel("sidebar");
                setActiveOpenPanel("sidebar");
              }}
              onOpenJournal={() => {
                setIsTaskFlowOpen(false);
                setIsAiTasksOpen(false);
                setIsVoraTraceOpen(false);
                setIsQuestionsBoardOpen(false);
                setIsJournalOpen(true);
                setLastActivePanel("sidebar");
                setActiveOpenPanel("sidebar");
              }}
              onOpenQuestionsBoard={() => {
                setIsTaskFlowOpen(false);
                setIsAiTasksOpen(false);
                setIsVoraTraceOpen(false);
                setIsJournalOpen(false);
                setIsQuestionsBoardOpen(true);
                setLastActivePanel("sidebar");
                setActiveOpenPanel("sidebar");
              }}
              onOpenBoards={() => {
                setIsTaskFlowOpen(false);
                setIsAiTasksOpen(false);
                setIsVoraTraceOpen(false);
                setIsJournalOpen(false);
                setIsQuestionsBoardOpen(false);
                setLastActivePanel("boards");
                setActiveOpenPanel("boards");
              }}
            />
          ) : (
            <BoardsPanel
              boards={boards}
              boardsLoaded={boardsLoaded}
              shouldStartCreatingBoard={shouldStartCreatingBoard}
              onStartCreatingBoardHandled={() => setShouldStartCreatingBoard(false)}
              onFirstBoardCreationStateChange={setIsCreatingFirstBoard}
              activeBoardId={activeBoardId}
              onSelectBoard={handleSelectBoard}
              onCreateBoard={handleCreateBoard}
              onRenameBoard={handleRenameBoard}
              onDeleteBoard={handleDeleteBoard}
              onGetBoardVersions={handleGetBoardVersions}
              onCreateBoardVersion={handleCreateBoardVersion}
              onCheckoutBoardVersion={handleCheckoutBoardVersion}
            />
          )}
        </div>
      )}

      {/* بوم اصلی */}
      <main
        ref={flow.reactFlowWrapper}
        onContextMenu={isWorkspacePageOpen ? undefined : handleCanvasContextMenu}
        onPointerDownCapture={isWorkspacePageOpen ? undefined : handleCanvasPointerDown}
        onPointerMoveCapture={isWorkspacePageOpen ? undefined : handleCanvasPointerMove}
        onPointerUpCapture={isWorkspacePageOpen ? undefined : handleCanvasPointerUp}
        onDoubleClickCapture={isWorkspacePageOpen ? undefined : handleCanvasDoubleClick}
        onKeyDown={isWorkspacePageOpen ? undefined : handleCanvasKeyDown}
        className={`w-full h-full relative bg-background flex-1 ${isSpacePanActive ? "space-pan-active" : ""} ${
          isTextToolActive ? "cursor-text" : activeShapeTool || isAddCardToolActive ? "cursor-crosshair" : ""
        }`}
      >
        {isVoraTraceOpen ? (
          <div className="absolute inset-0 z-20 overflow-hidden">
            <VoraTraceApp />
          </div>
        ) : isTaskFlowOpen || isAiTasksOpen ? (
          <TaskFlowPage taskType={isAiTasksOpen ? "ai" : "task"} />
        ) : isJournalOpen ? (
          <JournalPage
            onGoHome={() => {
              setIsJournalOpen(false);
              setIsTaskFlowOpen(false);
              setIsAiTasksOpen(false);
              setIsVoraTraceOpen(false);
              setIsQuestionsBoardOpen(false);
              setActiveOpenPanel("none");
            }}
          />
        ) : isQuestionsBoardOpen ? (
          <QuestionsBoardPage onGoHome={() => {
            setIsQuestionsBoardOpen(false);
            setIsJournalOpen(false);
            setIsTaskFlowOpen(false);
            setIsAiTasksOpen(false);
            setIsVoraTraceOpen(false);
            setActiveOpenPanel("none");
          }} />
        ) : (
          <>
        {libraryBoardSession && (
          <div className="absolute left-1/2 top-3 z-50 flex -translate-x-1/2 items-center gap-4 rounded-xl border border-amber-500/40 bg-[#171a22]/95 px-4 py-2.5 shadow-xl backdrop-blur">
            <div className="min-w-0 text-sm">
              <p className="truncate font-semibold text-amber-100">{libraryBoardSession.group.name}</p>
              <p className="text-xs text-zinc-400">Editing Library version {libraryBoardSession.baseVersion} · Draft</p>
            </div>
            {libraryBoardError && <p role="alert" className="max-w-64 text-xs text-rose-300">{libraryBoardError}</p>}
            <button
              type="button"
              onClick={() => {
                setLibraryBoardError(null);
                setLibraryVersionInput("");
                setIsLibraryVersionDialogOpen(true);
              }}
              className="whitespace-nowrap rounded-lg bg-amber-500/15 px-3 py-2 text-xs font-semibold text-amber-100 transition hover:bg-amber-500/25"
            >
              Save as new version
            </button>
            <button
              type="button"
              onClick={() => {
                setLibraryBoardError(null);
                void handleCloseLibraryBoard().catch((cause: unknown) => {
                  console.error("Could not discard Library Board draft:", cause);
                  setLibraryBoardError(cause instanceof Error ? cause.message : String(cause));
                });
              }}
              className="whitespace-nowrap rounded-lg border border-[#363b47] px-3 py-2 text-xs font-semibold text-zinc-200 transition hover:bg-[#242834]"
            >
              Discard and return
            </button>
          </div>
        )}
        {!libraryBoardSession && boardsLoaded && boards.length === 0 && !shouldStartCreatingBoard && !isCreatingFirstBoard && (
          <div className="pointer-events-none absolute inset-0 z-20 flex items-center justify-center">
            <div
              role="dialog"
              aria-labelledby="create-first-board-title"
              className="pointer-events-auto min-w-56 rounded-xl border border-[#2a2a2a] bg-[#181818]/95 px-8 py-7 text-center shadow-xl"
            >
              <h2 id="create-first-board-title" className="mb-4 text-base font-semibold text-zinc-200">
                Create New Board
              </h2>
              <button
                type="button"
                onClick={() => {
                  setLastActivePanel("boards");
                  setActiveOpenPanel("boards");
                  setShouldStartCreatingBoard(true);
                }}
                className="w-full rounded-lg bg-violet-600 px-4 py-2.5 text-sm font-medium text-white transition hover:bg-violet-500"
              >
                Create New Board
              </button>
            </div>
          </div>
        )}
        {isCreateItemGroupOpen && selectedNodes.length >= 1 && (
          <CreateItemGroupPanel
            selectedCount={selectedNodes.length}
            onCreate={handleCreateLibraryItemGroup}
            onClose={() => {
              setIsCreateItemGroupOpen(false);
              setCanvasNodeContextMenu(null);
            }}
          />
        )}
        {canvasNodeContextMenu && (
          <>
            <button
              type="button"
              className="fixed inset-0 z-[60] cursor-default"
              aria-label="Close canvas context menu"
              onClick={() => setCanvasNodeContextMenu(null)}
            />
            <div
              role="menu"
              className="fixed z-[61] min-w-40 rounded-lg border border-[#303543] bg-[#171a22] p-1.5 text-zinc-200 shadow-xl"
              style={{ left: canvasNodeContextMenu.x, top: canvasNodeContextMenu.y }}
            >
              <button
                type="button"
                role="menuitem"
                className="w-full rounded-md px-3 py-2 text-left text-xs hover:bg-[#282d3a]"
                onClick={handleAddCanvasSelectionToLibrary}
              >
                Add to Library
              </button>
              {selectedNodes.length > 1 && (
                <button
                  type="button"
                  role="menuitem"
                  className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-xs hover:bg-[#282d3a]"
                  onClick={handleGroupCanvasSelection}
                >
                  <Group size={14} />
                  Group selection
                </button>
              )}
              {selectedGroupIds.size > 0 && (
                <button
                  type="button"
                  role="menuitem"
                  className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-xs hover:bg-[#282d3a]"
                  onClick={handleUngroupCanvasSelection}
                >
                  <Ungroup size={14} />
                  Ungroup selection
                </button>
              )}
              {canvasNodeContextMenu.textLinkSelection && (
                <button
                  type="button"
                  role="menuitem"
                  className="w-full rounded-md px-3 py-2 text-left text-xs hover:bg-[#282d3a]"
                  onClick={handleAddTextLink}
                >
                  Add Link
                </button>
              )}
            </div>
          </>
        )}
        {connectionCreateMenu && (
          <>
            <button
              type="button"
              className="fixed inset-0 z-[70] cursor-default"
              aria-label="Cancel node connection"
              onPointerDown={(event) => {
                event.preventDefault();
                event.stopPropagation();
                setConnectionCreateMenu(null);
              }}
            />
            <div
              role="menu"
              aria-label="Create connected node"
              className="fixed z-[71] min-w-44 rounded-xl border border-[#303543] bg-[#171a22] p-2 text-zinc-200 shadow-xl"
              style={{
                left: Math.max(8, Math.min(connectionCreateMenu.x, window.innerWidth - 184)),
                top: Math.max(8, Math.min(connectionCreateMenu.y, window.innerHeight - 112)),
              }}
            >
              <p className="px-2 pb-1 text-[11px] text-zinc-500">Create connected</p>
              <button
                type="button"
                role="menuitem"
                className="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left text-sm hover:bg-[#282d3a]"
                onClick={() => handleCreateConnectedNode("card")}
              >
                <PlusSquare size={15} className="text-[#e8ad55]" />
                Card
              </button>
              <button
                type="button"
                role="menuitem"
                className="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left text-sm hover:bg-[#282d3a]"
                onClick={() => handleCreateConnectedNode("text")}
              >
                <Type size={15} className="text-[#e8ad55]" />
                Text
              </button>
            </div>
          </>
        )}
        {marqueePreview && (
          <div
            className="mind-selection-marquee"
            style={{
              left: marqueePreview.left,
              top: marqueePreview.top,
              width: marqueePreview.width,
              height: marqueePreview.height,
            }}
          />
        )}
        <CanvasZoomContext.Provider value={canvasZoomScales}>
          <ReactFlow
            nodes={flow.nodes}
            edges={flow.edges}
            onPaneClick={handleCanvasClick}
            onNodesChange={handleNodesChange}
            onEdgesChange={flow.onEdgesChange}
            onConnect={flow.onConnect}
            onConnectEnd={handleConnectEnd}
            onNodeDragStart={handleNodeDragStart}
            onNodeDrag={handleNodeDrag}
            onNodeDragStop={handleNodeDragStop}
            onNodeClick={handleCanvasNodeClick}
            onNodeDoubleClick={handleCanvasNodeDoubleClick}
            onBeforeDelete={async () => {
              flow.saveHistorySnapshot();
              return true;
            }}
            onNodeContextMenu={handleCanvasNodeContextMenu}
            nodeTypes={nodeTypes}
            deleteKeyCode="Delete"
            minZoom={0.2}
            maxZoom={2.5}
            zoomOnScroll={false}
            zoomOnPinch={false}
            zoomOnDoubleClick={false}
            nodeDragThreshold={0}
            nodesDraggable={!isTextToolActive && activeShapeTool === null && !isAddCardToolActive}
            elementsSelectable={!isTextToolActive && activeShapeTool === null && !isAddCardToolActive}
            multiSelectionKeyCode={["Control", "Meta"]}
            panActivationKeyCode="Space"
            selectionOnDrag={false}
            selectionMode={SelectionMode.Partial}
            panOnDrag={[1, 2]}
            panOnScroll={false}
            proOptions={{ hideAttribution: true }}
            defaultEdgeOptions={{ type: "default", style: { stroke: "#3f4656", strokeWidth: 1.5 } }}
          />
        </CanvasZoomContext.Provider>

        {selectionBounds && (selectedNodes.length > 1 || activeCanvasGroupId !== null) && (
          <div
            aria-hidden="true"
            className="pointer-events-none absolute z-20 border border-blue-400/80"
            style={{
              left: selectionBounds.left,
              top: selectionBounds.top,
              width: selectionBounds.width,
              height: selectionBounds.height,
            }}
          />
        )}
        {selectionBounds && activeCanvasGroupId && (
          <div
            role="toolbar"
            aria-label="Canvas group actions"
            className={`absolute z-30 flex items-center gap-1 rounded-lg border border-[#303543] bg-[#171a22]/95 p-1 text-zinc-200 shadow-lg ${
              selectionBounds.top >= 48 ? "-translate-y-full" : ""
            }`}
            style={{
              left: selectionBounds.left,
              top: selectionBounds.top >= 48
                ? selectionBounds.top - 8
                : selectionBounds.top + selectionBounds.height + 8,
            }}
          >
            <button
              type="button"
              className="flex items-center gap-1.5 rounded-md px-2 py-1.5 text-xs hover:bg-[#282d3a]"
              aria-label="Ungroup selection"
              title="Ungroup selection"
              onPointerDown={(event) => event.stopPropagation()}
              onClick={handleUngroupCanvasSelection}
            >
              <Ungroup size={14} />
              Ungroup
            </button>
          </div>
        )}

        {shapePreview && previewSettings && (
          <svg
            className="pointer-events-none absolute inset-0 z-30 h-full w-full"
            viewBox={`0 0 ${flow.reactFlowWrapper.current?.clientWidth ?? 1} ${flow.reactFlowWrapper.current?.clientHeight ?? 1}`}
            preserveAspectRatio="none"
            aria-hidden="true"
          >
            {shapePreview.shape === "rectangle" && shapePreview.points.length >= 2 ? (() => {
              const first = shapePreview.points[0] ?? { x: 0, y: 0 };
              const second = shapePreview.points[1] ?? first;
              return <rect x={Math.min(first.x, second.x)} y={Math.min(first.y, second.y)} width={Math.abs(second.x - first.x)} height={Math.abs(second.y - first.y)} fill={previewSettings.fillColor} fillOpacity={previewSettings.fillOpacity} stroke={previewSettings.color} strokeOpacity={previewSettings.lineOpacity} strokeWidth={previewSettings.lineWidth} strokeDasharray={previewDashArray} />;
            })() : shapePreview.shape === "path" && shapePreview.points.length > 0 ? (
              <>
                <defs>
                  <marker id="vora-mind-path-arrow" markerWidth="9" markerHeight="9" refX="8" refY="4.5" orient="auto" markerUnits="userSpaceOnUse">
                    <path d="M1 1L8 4.5L1 8" fill="none" stroke={previewSettings.color} strokeOpacity={previewSettings.lineOpacity} strokeWidth={previewSettings.lineWidth} strokeLinecap="round" strokeLinejoin="round" />
                  </marker>
                  <marker id="vora-mind-path-start-arrow" markerWidth="9" markerHeight="9" refX="1" refY="4.5" orient="auto-start-reverse" markerUnits="userSpaceOnUse">
                    <path d="M8 1L1 4.5L8 8" fill="none" stroke={previewSettings.color} strokeOpacity={previewSettings.lineOpacity} strokeWidth={previewSettings.lineWidth} strokeLinecap="round" strokeLinejoin="round" />
                  </marker>
                </defs>
                <path d={shapePreview.points.map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`).join(" ")} fill="none" stroke={previewSettings.color} strokeOpacity={previewSettings.lineOpacity} strokeWidth={previewSettings.lineWidth} strokeDasharray={previewDashArray} strokeLinecap="round" strokeLinejoin="round" markerStart={previewSettings.startArrow ? "url(#vora-mind-path-start-arrow)" : undefined} markerEnd={previewSettings.endArrow ? "url(#vora-mind-path-arrow)" : undefined} />
              </>
            ) : null}
          </svg>
        )}

        {/* دکمه Timeline */}
        {!libraryBoardSession && <div className="absolute bottom-4 z-40" style={{ left: canvasControlsLeft }}>
          <button
            onClick={() => flow.setIsReplayVisible(!flow.isReplayVisible)}
            className={`px-3 py-2 rounded-xl shadow-lg backdrop-blur transition-all flex items-center gap-2 text-xs font-medium ${CANVAS_FLOATING_CONTROL_BORDER_CLASS} ${
              flow.isReplayVisible
                ? "bg-violet-600 text-white shadow-violet-950/40"
                : "bg-[#141720]/90 hover:bg-violet-500/10 text-zinc-400 hover:text-violet-300"
            }`}
          >
            <History size={15} />
            <span>Timeline</span>
          </button>
        </div>}

        {/* کنترلر پخش تایم‌لاین */}
        {!libraryBoardSession && flow.isReplayVisible && (
          <div className="absolute bottom-16 z-40" style={{ left: canvasControlsLeft }}>
            <ReplayController
              snapshots={flow.snapshots}
              currentSnapshotIndex={flow.currentSnapshotIndex}
              onSelectSnapshot={flow.handleSelectSnapshot}
              isOpenModal={() => {
                if (!libraryBoardSession && activeBoardId) flow.setIsCommitModalOpen(true);
              }}
            />
          </div>
        )}

        <div className="absolute bottom-4 right-4 z-40 flex items-center gap-1 rounded-xl border border-[#2a2a2a] bg-[#181818]/90 p-1.5 shadow-lg backdrop-blur">
          <button
            type="button"
            onClick={() => changeCanvasZoom(-ZOOM_LEVEL_STEP)}
            className="mind-zoom-control"
            aria-label="Zoom out on both axes"
            title="Zoom out on both axes"
          >
            <Minus size={16} />
          </button>
          <button
            type="button"
            onClick={() => changeCanvasZoom(-zoomLevel)}
            className="min-w-14 rounded-md px-2 py-1 text-center font-mono text-xs text-zinc-200 hover:bg-[#242424]"
            aria-label={`Reset X and Y zoom, level ${formatZoomLevel(zoomLevel)}`}
            title="Reset X and Y zoom"
          >
            {formatZoomLevel(zoomLevel)}
          </button>
          <button
            type="button"
            onClick={() => changeCanvasZoom(ZOOM_LEVEL_STEP)}
            className="mind-zoom-control"
            aria-label="Zoom in on both axes"
            title="Zoom in on both axes"
          >
            <Plus size={16} />
          </button>
        </div>
          </>
        )}
      </main>

      {!isWorkspacePageOpen && <div className="absolute top-3.5 right-3.5 z-40 flex flex-row-reverse gap-2">
        <button
          type="button"
          onClick={toggleRightDrawer}
          aria-label={isRightDrawerOpen ? "Close library" : "Open library"}
          aria-pressed={isRightDrawerOpen}
          className={`${CANVAS_TOOL_BUTTON_CLASS} ${CANVAS_FLOATING_CONTROL_BORDER_CLASS} ${CANVAS_FLOATING_CONTROL_BACKGROUND_CLASS} ${CANVAS_TOOL_BUTTON_IDLE_CLASS}`}
          title={isRightDrawerOpen ? "Close library (Ctrl + I)" : "Library (Ctrl + I)"}
        >
          {isRightDrawerOpen ? <PanelRightClose size={16} /> : <PanelRight size={16} />}
        </button>
        <div
          className="group relative"
          onMouseEnter={() => setIsJournalMenuOpen(true)}
          onMouseLeave={() => setIsJournalMenuOpen(false)}
          onFocusCapture={() => setIsJournalMenuOpen(true)}
          onBlurCapture={(event) => {
            if (!event.currentTarget.contains(event.relatedTarget)) {
              setIsJournalMenuOpen(false);
            }
          }}
        >
          <button
            type="button"
            onClick={() => setIsJournalMenuOpen(true)}
            aria-label="Open Journal menu"
            aria-haspopup="menu"
            aria-expanded={isJournalMenuOpen}
            title="Journal"
            className={`${CANVAS_TOOL_BUTTON_CLASS} ${CANVAS_FLOATING_CONTROL_BORDER_CLASS} ${CANVAS_FLOATING_CONTROL_BACKGROUND_CLASS} ${CANVAS_TOOL_BUTTON_IDLE_CLASS}`}
          >
            <BookOpen size={16} />
          </button>
          <div
            role="menu"
            aria-label="Journal navigation"
            className={`absolute right-0 top-full z-50 mt-1 min-w-[11rem] rounded-xl border border-[#2a2a2a] bg-[#181818]/90 p-1.5 shadow-lg backdrop-blur transition-all duration-200 ${
              isJournalMenuOpen ? "visible translate-y-0 opacity-100" : "invisible -translate-y-1 opacity-0"
            }`}
          >
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                setIsTaskFlowOpen(false);
                setIsAiTasksOpen(false);
                setIsVoraTraceOpen(false);
                setIsQuestionsBoardOpen(false);
                setIsJournalOpen(true);
                setActiveOpenPanel("none");
                setIsJournalMenuOpen(false);
              }}
              className="flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-left text-sm font-medium text-gray-200 transition hover:bg-[#1c202d] hover:text-violet-300"
            >
              <BookOpen size={15} aria-hidden="true" />
              <span>Journal</span>
            </button>
            <Link
              to="/chart"
              role="menuitem"
              onClick={() => setIsJournalMenuOpen(false)}
              className="flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-sm font-medium text-gray-200 transition hover:bg-[#1c202d] hover:text-violet-300"
            >
              <CandlestickChart size={15} aria-hidden="true" />
              <span>Go to Chart</span>
            </Link>
          </div>
        </div>
      </div>}

      {/* سایدبار کتابخانه یادداشت‌ها (Mind) */}
      {!isWorkspacePageOpen && <RightDrawer
        selectedCardId={selectedCardId}
        setSelectedCardId={setSelectedCardId}
        onAddNewCard={handleCreateCardInLibraryOnly}
        onNodeDataSync={handleNodeDataSync}
        itemGroups={itemGroups}
        itemGroupError={itemGroupError}
        onMoveItemGroupToTrash={handleMoveLibraryItemGroupToTrash}
        onRestoreItemGroup={handleRestoreLibraryItemGroup}
        onInsertItemGroup={handleInsertLibraryItemGroup}
        onGetItemGroupVersions={handleGetLibraryItemGroupVersions}
        onGetItemGroupUsages={dbService.getLibraryItemGroupUsages}
        onOpenLibraryBoard={handleOpenLibraryBoard}
      />}

      {/* سایدبار تحلیل هوشمند کد (Forge AI Agent) */}
      {!isWorkspacePageOpen && <ForgeAgentDrawer
        onInjectGraphToCanvas={({ nodes, edges }) => {
          if (!canvasBoardId) return;
          flow.saveHistorySnapshot();
          flow.setNodes((nds) => nds.concat(nodes));
          flow.setEdges((eds) => eds.concat(edges));
        }}
      />}

      <CommitModal
        isOpen={!libraryBoardSession && flow.isCommitModalOpen}
        onClose={() => flow.setIsCommitModalOpen(false)}
        onCommit={flow.handleCommitSnapshot}
        currentSnapshotDepth={flow.snapshots.length}
      />

      {libraryBoardSession && isLibraryVersionDialogOpen && (
        <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/70 p-4">
          <form
            role="dialog"
            aria-modal="true"
            aria-labelledby="library-version-title"
            className="w-full max-w-md rounded-2xl border border-[#303543] bg-[#171a22] p-6 shadow-2xl"
            onSubmit={(event) => {
              event.preventDefault();
              setLibraryBoardError(null);
              void handleCommitLibraryBoardVersion().catch((cause: unknown) => {
                console.error("Could not save Library Board version:", cause);
                setLibraryBoardError(cause instanceof Error ? cause.message : String(cause));
              });
            }}
          >
            <h2 id="library-version-title" className="text-lg font-semibold text-white">
              Save {libraryBoardSession.group.name} as a new version
            </h2>
            <p className="mt-2 text-sm text-zinc-400">
              This saves the current draft to the Library and its owned Board. Older versions stay unchanged.
            </p>
            <label htmlFor="library-board-version" className="mt-5 block text-sm text-zinc-300">
              New version
            </label>
            <input
              id="library-board-version"
              autoFocus
              required
              value={libraryVersionInput}
              onChange={(event) => setLibraryVersionInput(event.target.value)}
              placeholder="1.2.3"
              inputMode="numeric"
              pattern="(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)"
              className="mt-2 w-full rounded-lg border border-[#363b47] bg-[#101217] px-3 py-2 font-mono text-sm text-white outline-none focus:border-amber-500/60"
            />
            {libraryBoardError && <p role="alert" className="mt-3 text-sm text-rose-300">{libraryBoardError}</p>}
            <div className="mt-6 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => {
                  setIsLibraryVersionDialogOpen(false);
                  setLibraryBoardError(null);
                }}
                className="rounded-lg px-3 py-2 text-sm text-zinc-300 transition hover:bg-[#242834]"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(libraryVersionInput.trim())}
                className="rounded-lg bg-amber-500/20 px-3 py-2 text-sm font-semibold text-amber-100 transition hover:bg-amber-500/30 disabled:cursor-not-allowed disabled:opacity-50"
              >
                Commit version
              </button>
            </div>
          </form>
        </div>
      )}

      {fullscreenCardId && (
        <FullscreenDocModal
          cardId={fullscreenCardId}
          onClose={() => setFullscreenCardId(null)}
        />
      )}
    </div>
  );
}

export default function App() {
  return (
    <>
      <ReactFlowProvider>
        <AppContent />
      </ReactFlowProvider>
    </>
  );
}