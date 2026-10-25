import { memo, useEffect, useRef, useState } from "react";
import { NodeResizeControl, NodeResizer, useReactFlow, useStore, useViewport, type Node, type NodeProps } from "@xyflow/react";
import {
  Copy,
  Group,
  LockKeyhole,
  Link2,
  Trash2,
  Type,
  Ungroup,
} from "lucide-react";
import { DrawingColorPicker } from "@/shared/canvas/DrawingColorPicker";
import { getSetting, setSetting } from "@/shared/store/useSettingsStore";
import type { VoraPathSegment, VoraShapeData, VoraShapeStyleSettings } from "@/mind/types";
import { canJoinPathNodes, getPathSegments } from "@/mind/utils/pathSegments";
import { useCanvasZoomScales } from "./CanvasZoomContext";

export type VoraShapeNodeModel = Node<VoraShapeData, "voraShape">;

const SHAPE_TOOLBAR_POSITION_KEY = "vora.home.shape_toolbar_position.v1";
const MULTI_SHAPE_TOOLBAR_POSITION_KEY = "vora.home.multi_shape_toolbar_position.v1";
const DEFAULT_SHAPE_TOOLBAR_POSITION = { left: 0, top: -42 };
const RECTANGLE_SIDE_RESIZE_POSITIONS = ["top", "right", "bottom", "left"] as const;
const SHAPE_LINE_WIDTHS = [1, 2, 3, 4] as const;
const SHAPE_LINE_STYLES: VoraShapeData["lineStyle"][] = ["solid", "dashed", "dotted"];
const SHAPE_TEMPLATES_KEY = "vora.home.shape_templates.v1";

interface ShapeStyleTemplate {
  id: string;
  name: string;
  settings: VoraShapeStyleSettings;
}

interface ShapeToolbarPosition {
  left: number;
  top: number;
}

interface PathPointDragStart {
  position: { x: number; y: number };
  width: number;
  height: number;
}

interface PathSegmentSelectionDetail {
  nodeId: string;
  segmentId: string;
  mode: "replace" | "add" | "toggle";
}

function dispatchPathSegmentSelection(
  nodeId: string,
  segmentId: string,
  mode: PathSegmentSelectionDetail["mode"],
): void {
  window.dispatchEvent(new CustomEvent<PathSegmentSelectionDetail>(
    "vora:update-path-segment-selection",
    { detail: { nodeId, segmentId, mode } },
  ));
}

function handlePathSegmentClick(
  nodeId: string,
  segmentId: string,
  shiftKey: boolean,
  ctrlKey: boolean,
  metaKey: boolean,
): void {
  const mode = shiftKey ? "add" : ctrlKey || metaKey ? "toggle" : "replace";
  dispatchPathSegmentSelection(nodeId, segmentId, mode);
}

function loadShapeToolbarPosition(key = SHAPE_TOOLBAR_POSITION_KEY): ShapeToolbarPosition {
  try {
    const saved = getSetting(key);
    if (!saved) return DEFAULT_SHAPE_TOOLBAR_POSITION;
    const parsed: unknown = JSON.parse(saved);
    if (
      typeof parsed === "object" &&
      parsed !== null &&
      "left" in parsed &&
      "top" in parsed &&
      typeof parsed.left === "number" &&
      Number.isFinite(parsed.left) &&
      typeof parsed.top === "number" &&
      Number.isFinite(parsed.top)
    ) {
      return { left: parsed.left, top: parsed.top };
    }
  } catch (error) {
    console.warn("Could not load Home shape toolbar position", error);
  }
  return DEFAULT_SHAPE_TOOLBAR_POSITION;
}

function loadShapeTemplates(shape: VoraShapeData["shape"]): ShapeStyleTemplate[] {
  try {
    const stored: unknown = JSON.parse(getSetting(`${SHAPE_TEMPLATES_KEY}:${shape}`) ?? "[]");
    if (!Array.isArray(stored)) return [];
    return stored.filter((item): item is ShapeStyleTemplate => {
      if (!isRecord(item) || !isRecord(item.settings)) return false;
      const settings = item.settings;
      return typeof item.id === "string" && typeof item.name === "string" &&
        typeof settings.color === "string" && /^#[0-9a-fA-F]{6}$/.test(settings.color) &&
        typeof settings.textColor === "string" && /^#[0-9a-fA-F]{6}$/.test(settings.textColor) &&
        typeof settings.fillColor === "string" && /^#[0-9a-fA-F]{6}$/.test(settings.fillColor) &&
        [settings.lineOpacity, settings.textOpacity, settings.fillOpacity].every((value) => typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 1) &&
        typeof settings.lineWidth === "number" && Number.isInteger(settings.lineWidth) && settings.lineWidth >= 1 && settings.lineWidth <= 4 &&
        (settings.lineStyle === "solid" || settings.lineStyle === "dashed" || settings.lineStyle === "dotted") &&
        typeof settings.startArrow === "boolean" && typeof settings.endArrow === "boolean";
    });
  } catch (error) {
    console.warn("Could not load shape style templates", error);
    return [];
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export const VoraShapeNode = memo(({ id, data, selected, width, height }: NodeProps<VoraShapeNodeModel>) => {
  const { getNode, getZoom } = useReactFlow<VoraShapeNodeModel>();
  const selectedShapeNodes = useStore(
    (state) => state.nodes.filter((node): node is VoraShapeNodeModel => node.selected === true && node.type === "voraShape"),
    (previous, next) => previous.length === next.length && previous.every((node, index) => node === next[index])
  );
  const selectedCanvasNodes = useStore(
    (state) => state.nodes.filter((node) => node.selected === true),
    (previous, next) => previous.length === next.length && previous.every((node, index) => node === next[index])
  );
  const hasGroupedCanvasSelection = selectedCanvasNodes.some((node) =>
    isRecord(node.data) &&
    typeof node.data.canvasGroupId === "string" &&
    node.data.canvasGroupId.length > 0,
  );
  const { zoom } = useViewport();
  const topmostSelectedShape = [...selectedShapeNodes].sort((first, second) =>
    first.position.y - second.position.y || first.position.x - second.position.x || first.id.localeCompare(second.id)
  )[0];
  const isMultiShapeSelection = selectedShapeNodes.length > 1;
  const selectedPathNodes = selectedShapeNodes.filter((node) => node.data.shape === "path");
  const canJoinSelectedPaths = selectedCanvasNodes.length === 2 &&
    selectedPathNodes.length === 2 &&
    canJoinPathNodes(selectedPathNodes);
  const isToolbarOwner = selected && topmostSelectedShape?.id === id;
  const allSelectedShapesLocked = selectedShapeNodes.length > 0 && selectedShapeNodes.every((node) => node.draggable === false);
  const allSelectedShapesAreRectangles = selectedShapeNodes.every((node) => node.data.shape === "rectangle");
  const nodeLocked = getNode(id)?.draggable === false;
  const defaultLabel = data.shape === "rectangle" ? "Rectangle" : "Path";
  const label = data.label || defaultLabel;
  const hasCustomLabel = label !== defaultLabel;
  const color = data.color || "#e8ad55";
  const lineWidth = Number.isFinite(data.lineWidth) ? data.lineWidth : 2;
  const selectionHitStrokeWidth = lineWidth + 10 / Math.max(zoom, 0.01);
  const lineOpacity = Number.isFinite(data.lineOpacity) ? data.lineOpacity : 1;
  const lineStyle = data.lineStyle ?? "solid";
  const textColor = data.textColor || "#dbdbdb";
  const textOpacity = Number.isFinite(data.textOpacity) ? data.textOpacity : 1;
  const fillColor = data.fillColor || color;
  const fillOpacity = Number.isFinite(data.fillOpacity) ? data.fillOpacity : data.shape === "rectangle" ? 0.12 : 0;
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [openControlMenu, setOpenControlMenu] = useState<"templates" | "line" | "fill" | "text" | "width" | "style" | null>(null);
  const [labelEditorOpen, setLabelEditorOpen] = useState(false);
  const [moreMenuOpen, setMoreMenuOpen] = useState(false);
  const [labelDraft, setLabelDraft] = useState(label);
  const [templateName, setTemplateName] = useState("");
  const [templates, setTemplates] = useState(() => loadShapeTemplates(data.shape));
  const [toolbarPosition, setToolbarPosition] = useState(loadShapeToolbarPosition);
  const [multiToolbarPosition, setMultiToolbarPosition] = useState(() => loadShapeToolbarPosition(MULTI_SHAPE_TOOLBAR_POSITION_KEY));
  const [hoveredPathPointIndex, setHoveredPathPointIndex] = useState<number | null>(null);
  const [selectedPathSegmentId, setSelectedPathSegmentId] = useState<string | null>(null);
  const [selectedPathSegmentIds, setSelectedPathSegmentIds] = useState<Set<string>>(() => new Set());
  const pathSegments = data.shape === "path" ? getPathSegments(id, data) : [];
  const selectedPathSegment = pathSegments.find((segment) => segment.id === selectedPathSegmentId)
    ?? pathSegments[0];
  const [draftPoints, setDraftPoints] = useState(data.points);
  const pointDraftRef = useRef(data.points);
  const draggingPointRef = useRef<number | null>(null);
  const pathPointDragStartRef = useRef<PathPointDragStart | null>(null);
  const cancelLabelBlurRef = useRef(false);
  const toolbarPositionRef = useRef(toolbarPosition);
  const toolbarRef = useRef<HTMLDivElement | null>(null);
  const toolbarDragRef = useRef<{
    pointerId: number;
    startX: number;
    startY: number;
    origin: ShapeToolbarPosition;
    zoomX: number;
    zoomY: number;
  } | null>(null);
  const svgRef = useRef<SVGSVGElement | null>(null);
  const shapeWidth = Math.max(1, width ?? 240);
  const shapeHeight = Math.max(1, height ?? 160);
  const canvasZoom = useCanvasZoomScales();
  const pathHandleScaleX = Math.max(0.01, zoom * canvasZoom.x);
  const pathHandleScaleY = Math.max(0.01, zoom * canvasZoom.y);
  const markerId = `vora-path-arrow-${id.replace(/[^a-zA-Z0-9_-]/g, "-")}`;
  const pathData = draftPoints
    .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x * shapeWidth} ${point.y * shapeHeight}`)
    .join(" ");
  const pathSegmentGeometry = pathSegments.map((segment, index) => {
    const start = draftPoints[index];
    const end = draftPoints[index + 1];
    return start && end ? {
      segment,
      d: `M ${start.x * shapeWidth} ${start.y * shapeHeight} L ${end.x * shapeWidth} ${end.y * shapeHeight}`,
    } : null;
  }).filter((item): item is { segment: VoraPathSegment; d: string } => item !== null);
  const dashArray = lineStyle === "dashed"
    ? `${lineWidth * 4} ${lineWidth * 3}`
    : lineStyle === "dotted" ? `${lineWidth} ${lineWidth * 2}` : undefined;

  useEffect(() => {
    if (draggingPointRef.current !== null) return;
    pointDraftRef.current = data.points;
    setDraftPoints(data.points);
  }, [data.points]);

  useEffect(() => {
    setLabelDraft(label);
  }, [label]);

  useEffect(() => {
    if (isToolbarOwner && !isMultiShapeSelection) return;
    setSettingsOpen(false);
    setOpenControlMenu(null);
    setLabelEditorOpen(false);
    setMoreMenuOpen(false);
  }, [isMultiShapeSelection, isToolbarOwner]);

  useEffect(() => {
    if (!isToolbarOwner) return;
    const centerToolbarAtPointer = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const { x, y } = event.detail;
      if (typeof x !== "number" || !Number.isFinite(x) ||
        typeof y !== "number" || !Number.isFinite(y)) return;
      const toolbar = toolbarRef.current;
      const nodeElement = toolbar?.closest(".react-flow__node");
      if (!toolbar || !nodeElement) return;
      const nodeBounds = nodeElement.getBoundingClientRect();
      const scaleX = zoom * canvasZoom.x;
      const scaleY = zoom * canvasZoom.y;
      moveToolbarTo({
        left: (x - nodeBounds.left - toolbar.offsetWidth / 2) / scaleX,
        top: (y - nodeBounds.top - toolbar.offsetHeight / 2) / scaleY,
      });
    };
    window.addEventListener("vora:center-floating-toolbar", centerToolbarAtPointer);
    return () => window.removeEventListener("vora:center-floating-toolbar", centerToolbarAtPointer);
  }, [canvasZoom.x, canvasZoom.y, isMultiShapeSelection, isToolbarOwner, zoom]);

  useEffect(() => {
    if (!openControlMenu) return;
    const closeOnOutsidePointer = (event: globalThis.PointerEvent): void => {
      if (event.target instanceof Node && !toolbarRef.current?.contains(event.target)) setOpenControlMenu(null);
    };
    const closeOnEscape = (event: KeyboardEvent): void => {
      if (event.key === "Escape") setOpenControlMenu(null);
    };
    document.addEventListener("pointerdown", closeOnOutsidePointer);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOnOutsidePointer);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [openControlMenu]);

  useEffect(() => {
    const updatePathSegmentSelection = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const { nodeId, segmentId, mode } = event.detail;
      if (
        typeof nodeId !== "string" ||
        typeof segmentId !== "string" ||
        (mode !== "replace" && mode !== "add" && mode !== "toggle")
      ) return;

      if (mode === "replace") {
        if (nodeId !== id) {
          setSelectedPathSegmentIds(new Set());
          return;
        }
        if (!pathSegments.some((segment) => segment.id === segmentId)) return;
        setSelectedPathSegmentIds(new Set([segmentId]));
      } else if (mode === "add") {
        if (nodeId !== id || !pathSegments.some((segment) => segment.id === segmentId)) return;
        setSelectedPathSegmentIds((current) => new Set(current).add(segmentId));
      } else {
        if (nodeId !== id || !pathSegments.some((segment) => segment.id === segmentId)) return;
        setSelectedPathSegmentIds((current) => {
          const next = new Set(current);
          if (next.has(segmentId)) next.delete(segmentId);
          else next.add(segmentId);
          return next;
        });
      }
      if (nodeId === id) setSelectedPathSegmentId(segmentId);
    };
    const selectPathSegment = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      if (event.detail.nodeId !== id || typeof event.detail.segmentId !== "string") return;
      if (!pathSegments.some((segment) => segment.id === event.detail.segmentId)) return;
      dispatchPathSegmentSelection(id, event.detail.segmentId, "replace");
      setSettingsOpen(true);
    };
    window.addEventListener("vora:update-path-segment-selection", updatePathSegmentSelection);
    window.addEventListener("vora:select-path-segment", selectPathSegment);
    return () => {
      window.removeEventListener("vora:update-path-segment-selection", updatePathSegmentSelection);
      window.removeEventListener("vora:select-path-segment", selectPathSegment);
    };
  }, [id, pathSegments]);

  const update = (patch: Partial<VoraShapeData>): void => {
    const detail = isMultiShapeSelection
      ? { nodeIds: selectedShapeNodes.map((node) => node.id), patch }
      : { nodeId: id, shape: data.shape, patch };
    window.dispatchEvent(new CustomEvent("vora:update-shape", { detail }));
  };

  const updatePathSegment = (segmentId: string, patch: Partial<VoraPathSegment>): void => {
    const segments = pathSegments.map((segment) =>
      segment.id === segmentId ? { ...segment, ...patch } : segment
    );
    update({ segments });
  };

  const saveTemplate = (): void => {
    const name = templateName.trim();
    if (!name) return;
    const template: ShapeStyleTemplate = {
      id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      name,
      settings: { color, lineOpacity, lineWidth, lineStyle, textColor, textOpacity, fillColor, fillOpacity, startArrow: data.startArrow, endArrow: data.endArrow },
    };
    const next = [...templates, template];
    setTemplates(next);
    try {
      setSetting(`${SHAPE_TEMPLATES_KEY}:${data.shape}`, JSON.stringify(next));
    } catch (error) {
      console.warn("Could not save shape style template", error);
    }
    setTemplateName("");
    setOpenControlMenu(null);
  };

  const applyTemplate = (template: ShapeStyleTemplate): void => {
    update(template.settings);
    setOpenControlMenu(null);
  };

  const deleteTemplate = (templateId: string): void => {
    const next = templates.filter((template) => template.id !== templateId);
    setTemplates(next);
    try {
      setSetting(`${SHAPE_TEMPLATES_KEY}:${data.shape}`, JSON.stringify(next));
    } catch (error) {
      console.warn("Could not save shape style templates", error);
    }
  };

  const saveResizeHistory = (): void => {
    window.dispatchEvent(new CustomEvent<string>("vora:shape-resize-start", { detail: id }));
    window.dispatchEvent(new Event("vora:save-history-snapshot"));
  };
  const finishResize = (): void => {
    window.dispatchEvent(new CustomEvent<string>("vora:shape-resize-end", { detail: id }));
  };
  const updatePointAtPointer = (event: React.PointerEvent<HTMLDivElement>): void => {
    const pointIndex = draggingPointRef.current;
    const bounds = svgRef.current?.getBoundingClientRect();
    if (pointIndex === null || !bounds || bounds.width === 0 || bounds.height === 0) return;
    const updated = pointDraftRef.current.map((point, index) => index === pointIndex
      ? {
        x: (event.clientX - bounds.left) / bounds.width,
        y: (event.clientY - bounds.top) / bounds.height,
      }
      : point);
    pointDraftRef.current = updated;
    setDraftPoints(updated);
  };

  const finishPointDrag = (): void => {
    if (draggingPointRef.current === null) return;
    draggingPointRef.current = null;
    setHoveredPathPointIndex(null);
    const dragStart = pathPointDragStartRef.current;
    pathPointDragStartRef.current = null;
    if (!dragStart) return;

    const minX = Math.min(0, ...pointDraftRef.current.map((point) => point.x));
    const minY = Math.min(0, ...pointDraftRef.current.map((point) => point.y));
    const maxX = Math.max(1, ...pointDraftRef.current.map((point) => point.x));
    const maxY = Math.max(1, ...pointDraftRef.current.map((point) => point.y));
    const spanX = maxX - minX;
    const spanY = maxY - minY;
    const points = pointDraftRef.current.map((point) => ({
      x: (point.x - minX) / spanX,
      y: (point.y - minY) / spanY,
    }));
    window.dispatchEvent(new CustomEvent("vora:update-path-geometry", {
      detail: {
        nodeId: id,
        position: {
          x: dragStart.position.x + minX * dragStart.width,
          y: dragStart.position.y + minY * dragStart.height,
        },
        width: dragStart.width * spanX,
        height: dragStart.height * spanY,
        points,
      },
    }));
  };

  const moveToolbarTo = (next: ShapeToolbarPosition): void => {
    toolbarPositionRef.current = next;
    if (isMultiShapeSelection) setMultiToolbarPosition(next);
    else setToolbarPosition(next);
  };

  const startToolbarDrag = (event: React.PointerEvent<HTMLButtonElement>): void => {
    if (event.button !== 0 || toolbarDragRef.current) return;
    event.preventDefault();
    event.stopPropagation();
    toolbarDragRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      origin: isMultiShapeSelection ? multiToolbarPosition : toolbarPosition,
      zoomX: getZoom() * canvasZoom.x,
      zoomY: getZoom() * canvasZoom.y,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const moveToolbar = (event: React.PointerEvent<HTMLButtonElement>): void => {
    const drag = toolbarDragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopPropagation();
    moveToolbarTo({
      left: drag.origin.left + (event.clientX - drag.startX) / drag.zoomX,
      top: drag.origin.top + (event.clientY - drag.startY) / drag.zoomY,
    });
  };

  const finishToolbarDrag = (event: React.PointerEvent<HTMLButtonElement>): void => {
    if (toolbarDragRef.current?.pointerId !== event.pointerId) return;
    toolbarDragRef.current = null;
    event.stopPropagation();
    try {
      const key = isMultiShapeSelection ? MULTI_SHAPE_TOOLBAR_POSITION_KEY : SHAPE_TOOLBAR_POSITION_KEY;
      setSetting(key, JSON.stringify(toolbarPositionRef.current));
    } catch (error) {
      console.warn("Could not save Home shape toolbar position", error);
    }
  };

  const moveToolbarWithKeyboard = (event: React.KeyboardEvent<HTMLButtonElement>): void => {
    const direction = {
      ArrowLeft: [-1, 0],
      ArrowRight: [1, 0],
      ArrowUp: [0, -1],
      ArrowDown: [0, 1],
    }[event.key];
    if (!direction) return;
    event.preventDefault();
    event.stopPropagation();
    const step = event.shiftKey ? 10 : 1;
    const position = isMultiShapeSelection ? multiToolbarPosition : toolbarPosition;
    const next = {
      left: position.left + direction[0] * step,
      top: position.top + direction[1] * step,
    };
    moveToolbarTo(next);
    try {
      const key = isMultiShapeSelection ? MULTI_SHAPE_TOOLBAR_POSITION_KEY : SHAPE_TOOLBAR_POSITION_KEY;
      setSetting(key, JSON.stringify(next));
    } catch (error) {
      console.warn("Could not save Home shape toolbar position", error);
    }
  };

  const saveToolbarPosition = (next: ShapeToolbarPosition): void => {
    moveToolbarTo(next);
    try {
      setSetting(SHAPE_TOOLBAR_POSITION_KEY, JSON.stringify(next));
    } catch (error) {
      console.warn("Could not save Home shape toolbar position", error);
    }
  };

  const saveLabel = (): void => {
    if (cancelLabelBlurRef.current) {
      cancelLabelBlurRef.current = false;
      return;
    }
    const nextLabel = labelDraft.trim();
    if (nextLabel && nextLabel !== label) update({ label: nextLabel });
    else setLabelDraft(label);
    setLabelEditorOpen(false);
  };

  return (
    <div
      className="h-full w-full overflow-visible"
      onPointerMove={updatePointAtPointer}
      onPointerUp={finishPointDrag}
      onPointerCancel={finishPointDrag}
    >
      {data.shape === "rectangle" && (
        <>
          <NodeResizer
            isVisible={selected && !nodeLocked}
            onResizeStart={saveResizeHistory}
            onResizeEnd={finishResize}
            minWidth={48}
            minHeight={36}
            color="#e8ad55"
            handleClassName="vora-shape-resize-handle"
            handleStyle={{ width: 10, height: 10 }}
          />
          {selected && !nodeLocked && (
            RECTANGLE_SIDE_RESIZE_POSITIONS.map((position) => (
              <NodeResizeControl
                key={position}
                position={position}
                onResizeStart={saveResizeHistory}
                onResizeEnd={finishResize}
                minWidth={48}
                minHeight={36}
                color="#e8ad55"
                className="vora-shape-resize-handle"
                style={{ width: 10, height: 10 }}
              />
            ))
          )}
        </>
      )}
      {isToolbarOwner && (
        <div
          ref={toolbarRef}
          className="drawing-toolbar tv-floating-toolbar ui-draggable vora-floating-ui vora-shape-toolbar nodrag nopan"
          data-selection-count={selectedShapeNodes.length}
          data-name="drawing-toolbar"
          style={{ ...(isMultiShapeSelection ? multiToolbarPosition : toolbarPosition), zIndex: 21, scale: `${1 / (zoom * canvasZoom.x)} ${1 / (zoom * canvasZoom.y)}`, transformOrigin: "top left" }}
          onPointerDown={(event) => event.stopPropagation()}
          onClick={(event) => event.stopPropagation()}
        >
          <button
            type="button"
            className="tv-floating-toolbar__drag js-drag ui-draggable-handle"
            data-vora-drag-handle="true"
            aria-label={isMultiShapeSelection ? "Move selected shapes toolbar" : "Move shape toolbar"}
            title="Drag"
            onPointerDown={startToolbarDrag}
            onPointerMove={moveToolbar}
            onPointerUp={finishToolbarDrag}
            onPointerCancel={finishToolbarDrag}
            onKeyDown={moveToolbarWithKeyboard}
          >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 12" width="8" height="12" fill="currentColor" aria-hidden="true"><rect width="2" height="2" rx="1" /><rect width="2" height="2" rx="1" y="5" /><rect width="2" height="2" rx="1" y="10" /><rect width="2" height="2" rx="1" x="6" /><rect width="2" height="2" rx="1" x="6" y="5" /><rect width="2" height="2" rx="1" x="6" y="10" /></svg>
          </button>
          <div className="tv-floating-toolbar__widget-wrapper">
            <div className="floating-toolbar-react-widgets">
              {!isMultiShapeSelection && (
              <div className="drawing-toolbar__item vora-shape-toolbar__palette-item">
                <button type="button" className={`floating-toolbar-react-widgets__button vora-shape-toolbar__icon${openControlMenu === "templates" ? " is-active" : ""}`} data-name="templates" aria-label="Drawing templates" title="Drawing templates" aria-expanded={openControlMenu === "templates"} onClick={() => { setOpenControlMenu((open) => open === "templates" ? null : "templates"); setMoreMenuOpen(false); }}>
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" fill="none" stroke="currentColor" aria-hidden="true"><path strokeLinecap="round" d="M15.5 18.5h6m-3 3v-6" /><rect width="6" height="6" rx="1.5" x="6.5" y="6.5" /><rect width="6" height="6" rx="1.5" x="15.5" y="6.5" /><rect width="6" height="6" rx="1.5" x="6.5" y="15.5" /></svg>
                </button>
                {openControlMenu === "templates" && <div className="drawing-toolbar__popover drawing-toolbar__templates vora-shape-toolbar__templates" role="menu" aria-label="Drawing templates">
                  <form onSubmit={(event) => { event.preventDefault(); saveTemplate(); }}>
                    <input aria-label="Template name" placeholder="Template name" maxLength={64} value={templateName} onChange={(event) => setTemplateName(event.currentTarget.value)} />
                    <button type="submit" disabled={!templateName.trim()}>Save current style</button>
                  </form>
                  {templates.map((template) => <div className="drawing-toolbar__template-row" key={template.id}>
                    <button type="button" onClick={() => applyTemplate(template)}>{template.name}</button>
                    <button type="button" aria-label={`Delete template ${template.name}`} onClick={() => deleteTemplate(template.id)}><Trash2 size={13} /></button>
                  </div>)}
                </div>}
              </div>
              )}
              <div className="drawing-toolbar__item vora-shape-toolbar__palette-item">
                <button
                  type="button"
                  className={`floating-toolbar-react-widgets__button vora-shape-toolbar__icon${openControlMenu === "line" ? " is-active" : ""}`}
                  data-name="line-tool-color"
                  aria-label="Shape line color"
                  title="Line color"
                  aria-expanded={openControlMenu === "line"}
                  onClick={() => setOpenControlMenu((open) => open === "line" ? null : "line")}
                >
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path fill="currentColor" d="M10.62.72a2.47 2.47 0 0 1 3.5 0l1.16 1.16c.96.97.96 2.54 0 3.5l-.58.58-8.9 8.9-1 1-.14.14H0v-4.65l.14-.15 1-1 8.9-8.9.58-.58Zm2.8.7a1.48 1.48 0 0 0-2.1 0l-.23.23 3.26 3.26.23-.23c.58-.58.58-1.52 0-2.1l-1.16-1.16Zm.23 4.2-3.26-3.27-8.2 8.2 3.25 3.27 8.2-8.2Zm-8.9 8.9-3.27-3.26-.5.5V15h3.27l.5-.5Z" /></svg>
                  <span className="color-bg"><span className="color-swatch" style={{ backgroundColor: color, opacity: lineOpacity }} /></span>
                </button>
                {openControlMenu === "line" && (
                  <div className="drawing-toolbar__popover drawing-toolbar__palette vora-shape-toolbar__color-palette" role="group" aria-label="Shape line color">
                    <DrawingColorPicker
                      color={color}
                      opacity={lineOpacity}
                      label="Line color"
                      onColor={(nextColor) => { update({ color: nextColor }); setOpenControlMenu(null); }}
                      onOpacity={(opacity) => update({ lineOpacity: opacity })}
                      onKeepOpen={() => setOpenControlMenu("line")}
                    />
                  </div>
                )}
              </div>
              <div className={`drawing-toolbar__item vora-shape-toolbar__palette-item${!allSelectedShapesAreRectangles ? " is-disabled" : ""}`}>
                <button
                  type="button"
                  className={`floating-toolbar-react-widgets__button vora-shape-toolbar__icon${openControlMenu === "fill" ? " is-active" : ""}`}
                  data-name="background-color"
                  aria-label="Shape fill color"
                  title={allSelectedShapesAreRectangles ? "Fill color" : "Fill is available only when all selected shapes are rectangles"}
                  aria-expanded={openControlMenu === "fill"}
                  disabled={!allSelectedShapesAreRectangles}
                  onClick={() => setOpenControlMenu((open) => open === "fill" ? null : "fill")}
                >
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" width="20" height="20" fill="none" aria-hidden="true"><path stroke="currentColor" d="M13.5 6.5l-3-3-7 7 7.59 7.59a2 2 0 0 0 2.82 0l4.18-4.18a2 2 0 0 0 0-2.82L13.5 6.5zm0 0v-4a2 2 0 0 0-2-2v0a2 2 0 0 0-2 2v6" /><path fill="currentColor" d="M0 16.5C0 15 2.5 12 2.5 12S5 15 5 16.5 4 19 2.5 19 0 18 0 16.5z" /><circle fill="currentColor" cx="9.5" cy="9.5" r="1.5" /></svg>
                  <span className="color-bg"><span className="color-swatch" style={{ backgroundColor: fillColor, opacity: fillOpacity }} /></span>
                </button>
                {openControlMenu === "fill" && allSelectedShapesAreRectangles && (
                  <div className="drawing-toolbar__popover drawing-toolbar__palette vora-shape-toolbar__color-palette" role="group" aria-label="Shape fill color">
                    <DrawingColorPicker
                      color={fillColor}
                      opacity={fillOpacity}
                      label="Fill color"
                      onColor={(nextColor) => { update({ fillColor: nextColor }); setOpenControlMenu(null); }}
                      onOpacity={(opacity) => update({ fillOpacity: opacity })}
                      onKeepOpen={() => setOpenControlMenu("fill")}
                    />
                  </div>
                )}
              </div>
              <div className="drawing-toolbar__item vora-shape-toolbar__palette-item">
                <button type="button" className={`floating-toolbar-react-widgets__button vora-shape-toolbar__icon${openControlMenu === "text" ? " is-active" : ""}`} data-name="text-color" aria-label="Text color" title="Text color" aria-expanded={openControlMenu === "text"} onClick={() => setOpenControlMenu((open) => open === "text" ? null : "text")}>
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 13 15" width="13" height="15" fill="none" aria-hidden="true"><path stroke="currentColor" d="M4 14.5h2.5m2.5 0H6.5m0 0V.5m0 0h-5a1 1 0 0 0-1 1V4m6-3.5h5a1 1 0 0 1 1 1V4" /></svg>
                  <span className="color-bg"><span className="color-swatch" style={{ backgroundColor: textColor, opacity: textOpacity }} /></span>
                </button>
                {openControlMenu === "text" && <div className="drawing-toolbar__popover drawing-toolbar__palette vora-shape-toolbar__color-palette" role="group" aria-label="Text color">
                  <DrawingColorPicker color={textColor} opacity={textOpacity} label="Text color" onColor={(nextColor) => { update({ textColor: nextColor }); setOpenControlMenu(null); }} onOpacity={(opacity) => update({ textOpacity: opacity })} onKeepOpen={() => setOpenControlMenu("text")} />
                </div>}
              </div>
              <div className="drawing-toolbar__item drawing-toolbar__item--width">
                <button
                  type="button"
                  className="floating-toolbar-react-widgets__button drawing-toolbar__width"
                  data-name="line-tool-width"
                  aria-label={`Line width ${lineWidth} pixels`}
                  title="Line width"
                  aria-expanded={openControlMenu === "width"}
                  onClick={() => setOpenControlMenu((open) => open === "width" ? null : "width")}
                >
                  <span className="drawing-toolbar__width-wrap">
                    <span className="drawing-toolbar__width-icon" role="img" aria-hidden="true"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 1" width="18" height="1"><rect width="18" height="1" fill="currentColor" rx=".5" /></svg></span>
                    <span className="drawing-toolbar__width-label">{lineWidth}px</span>
                  </span>
                </button>
                {openControlMenu === "width" && (
                  <div className="drawing-toolbar__popover drawing-toolbar__width-menu" role="group" aria-label="Line width">
                    {SHAPE_LINE_WIDTHS.map((widthValue) => (
                      <button
                        key={widthValue}
                        type="button"
                        className="drawing-toolbar__menu-option"
                        aria-pressed={lineWidth === widthValue}
                        onClick={() => { update({ lineWidth: widthValue }); setOpenControlMenu(null); }}
                      >
                        <svg className="drawing-toolbar__width-option-icon" xmlns="http://www.w3.org/2000/svg" viewBox={`0 0 18 ${widthValue}`} width="18" height={widthValue} aria-hidden="true">
                          <rect width="18" height={widthValue} fill="currentColor" rx={widthValue / 2} />
                        </svg>
                        <span className="drawing-toolbar__width-option-label">{widthValue}px</span>
                      </button>
                    ))}
                  </div>
                )}
              </div>
              <div className="drawing-toolbar__item vora-shape-toolbar__style-item">
                <button
                  type="button"
                  className="floating-toolbar-react-widgets__button drawing-toolbar__style"
                  data-name="style"
                  aria-label={`Line style ${lineStyle}`}
                  title="Line style"
                  aria-expanded={openControlMenu === "style"}
                  onClick={() => setOpenControlMenu((open) => open === "style" ? null : "style")}
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" aria-hidden="true"><path stroke="currentColor" d="M4 13.5h20" /></svg>
                </button>
                {openControlMenu === "style" && (
                  <div className="drawing-toolbar__popover drawing-toolbar__style-menu" role="group" aria-label="Line style">
                    {SHAPE_LINE_STYLES.map((style) => (
                      <button
                        key={style}
                        type="button"
                        className="drawing-toolbar__menu-option"
                        aria-pressed={lineStyle === style}
                        onClick={() => { update({ lineStyle: style }); setOpenControlMenu(null); }}
                      >
                        <span className="drawing-toolbar__style-menu-icon-cell">
                          <span className={`drawing-toolbar__line-preview is-${style}`} />
                        </span>
                        <span className="drawing-toolbar__style-menu-label">{style === "solid" ? "Solid" : style === "dashed" ? "Dashed line" : "Dotted line"}</span>
                      </button>
                    ))}
                  </div>
                )}
              </div>
              <span className="vora-shape-toolbar__separator" aria-hidden="true" />
              {!isMultiShapeSelection && (
                <button type="button" data-name="settings" className={`floating-toolbar-react-widgets__button vora-shape-toolbar__icon${settingsOpen ? " is-active" : ""}`} aria-label="Drawing settings" title="Drawing settings" aria-expanded={settingsOpen} onClick={() => { setSettingsOpen((open) => !open); setMoreMenuOpen(false); setOpenControlMenu(null); }}>
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" fill="currentColor" aria-hidden="true"><path fillRule="evenodd" d="M18 14a4 4 0 1 1-8 0 4 4 0 0 1 8 0Zm-1 0a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" /><path fillRule="evenodd" d="M8.5 5h11l5 9-5 9h-11l-5-9 5-9Zm-3.86 9L9.1 6h9.82l4.45 8-4.45 8H9.1l-4.45-8Z" /></svg>
                </button>
              )}
                {selectedCanvasNodes.length >= 2 && (
                  <button
                    type="button"
                    data-name={hasGroupedCanvasSelection ? "ungroup-selection" : "group-selection"}
                    className="floating-toolbar-react-widgets__button vora-shape-toolbar__icon"
                    aria-label={hasGroupedCanvasSelection ? "Ungroup selection" : "Group selection"}
                    title={hasGroupedCanvasSelection ? "Ungroup selection" : "Group selection"}
                    onClick={() => window.dispatchEvent(new CustomEvent(
                      hasGroupedCanvasSelection ? "vora:ungroup-canvas-selection" : "vora:group-canvas-selection",
                    ))}
                  >
                    {hasGroupedCanvasSelection ? <Ungroup size={16} /> : <Group size={16} />}
                  </button>
                )}
                {canJoinSelectedPaths && (
                  <button
                    type="button"
                    data-name="join-path-sections"
                    className="floating-toolbar-react-widgets__button vora-shape-toolbar__icon"
                    aria-label="Rejoin selected Path sections"
                    title="Rejoin selected Path sections"
                    onClick={() => window.dispatchEvent(new CustomEvent("vora:join-selected-paths"))}
                  >
                    <Link2 size={16} />
                  </button>
                )}
                <button type="button" data-name="add-alert" className="floating-toolbar-react-widgets__button vora-shape-toolbar__icon" aria-label="Add price alert" title="Price alerts are unavailable on the Mind canvas" disabled>
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true"><path fill="currentColor" d="m19.54 4.5 3.96 4.32-.74.68-3.96-4.32.74-.68ZM7.46 4.5 3.5 8.82l.74.68L8.2 5.18l-.74-.68ZM19.74 10.33A7.5 7.5 0 0 1 21 14.5v.5h1v-.5a8.5 8.5 0 1 0-8.5 8.5h.5v-1h-.5a7.5 7.5 0 1 1 6.24-11.67Z" /><path fill="currentColor" d="M13 9v5h-3v1h4V9h-1ZM19 20v-4h1v4h4v1h-4v4h-1v-4h-4v-1h4Z" /></svg>
              </button>
              <button type="button" data-name="lock" className={`floating-toolbar-react-widgets__button vora-shape-toolbar__icon${allSelectedShapesLocked ? " is-active" : ""}`} aria-label={allSelectedShapesLocked ? "Unlock selected shapes" : "Lock selected shapes"} aria-pressed={allSelectedShapesLocked} title={allSelectedShapesLocked ? "Unlock" : "Lock"} onClick={() => window.dispatchEvent(new CustomEvent("vora:toggle-shape-lock", { detail: isMultiShapeSelection ? { nodeIds: selectedShapeNodes.map((node) => node.id), locked: !allSelectedShapesLocked } : { nodeId: id, locked: !allSelectedShapesLocked } }))}>
                <LockKeyhole size={16} />
              </button>
              <button type="button" data-name="remove" className="floating-toolbar-react-widgets__button vora-shape-toolbar__icon" aria-label={isMultiShapeSelection ? `Delete ${selectedShapeNodes.length} selected shapes` : `Delete ${label}`} title="Remove" onClick={() => window.dispatchEvent(new CustomEvent("vora:remove-from-canvas", { detail: isMultiShapeSelection ? { nodeIds: selectedShapeNodes.map((node) => node.id) } : { nodeId: id } }))}>
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true"><path fill="currentColor" d="M18 7h5v1h-2.01l-1.33 14.64a1.5 1.5 0 0 1-1.5 1.36H9.84a1.5 1.5 0 0 1-1.49-1.36L7.01 8H5V7h5V6c0-1.1.9-2 2-2h4a2 2 0 0 1 2 2v1Zm-6-2a1 1 0 0 0-1 1v1h6V6a1 1 0 0 0-1-1h-4ZM8.02 8l1.32 14.54a.5.5 0 0 0 .5.46h8.33a.5.5 0 0 0 .5-.46L19.99 8H8.02Z" /></svg>
              </button>
              {!isMultiShapeSelection && (
                <button type="button" data-name="more" className={`floating-toolbar-react-widgets__button vora-shape-toolbar__icon${moreMenuOpen ? " is-active" : ""}`} aria-label="More drawing actions" title="More drawing actions" aria-expanded={moreMenuOpen} onClick={() => { setMoreMenuOpen((open) => !open); setSettingsOpen(false); setOpenControlMenu(null); }}>
                  <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" fill="none" aria-hidden="true"><path fill="currentColor" fillRule="evenodd" clipRule="evenodd" d="M7.5 13a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM5 14.5a2.5 2.5 0 1 1 5 0 2.5 2.5 0 0 1-5 0zm9.5-1.5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM12 14.5a2.5 2.5 0 1 1 5 0 2.5 2.5 0 0 1-5 0zm9.5-1.5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM19 14.5a2.5 2.5 0 1 1 5 0 2.5 2.5 0 0 1-5 0z" /></svg>
                </button>
              )}
            </div>
          </div>
          {!isMultiShapeSelection && labelEditorOpen && (
            <div className="drawing-toolbar__popover vora-shape-toolbar__popover vora-shape-toolbar__label-popover">
              <label htmlFor={`vora-shape-label-${id}`}>Shape label</label>
              <input id={`vora-shape-label-${id}`} autoFocus value={labelDraft} maxLength={80} onChange={(event) => setLabelDraft(event.currentTarget.value)} onKeyDown={(event) => {
                event.stopPropagation();
                if (event.key === "Enter") event.currentTarget.blur();
                if (event.key === "Escape") {
                  cancelLabelBlurRef.current = true;
                  setLabelDraft(label);
                  setLabelEditorOpen(false);
                  event.currentTarget.blur();
                }
              }} onBlur={saveLabel} />
            </div>
          )}
          {!isMultiShapeSelection && moreMenuOpen && (
            <div className="drawing-toolbar__popover drawing-toolbar__more-menu vora-shape-toolbar__popover" role="menu">
              <button type="button" role="menuitem" onClick={() => { setLabelEditorOpen((open) => !open); setMoreMenuOpen(false); }}><Type size={14} />Edit shape label</button>
              <button type="button" role="menuitem" onClick={() => { window.dispatchEvent(new CustomEvent("vora:duplicate-shape", { detail: { nodeId: id } })); setMoreMenuOpen(false); }}><Copy size={14} />Duplicate shape</button>
              <button type="button" role="menuitem" onClick={() => { saveToolbarPosition(DEFAULT_SHAPE_TOOLBAR_POSITION); setMoreMenuOpen(false); }}>Reset toolbar position</button>
            </div>
          )}
        </div>
      )}
      {isToolbarOwner && !isMultiShapeSelection && settingsOpen && (
        <div
          className="nodrag nopan absolute left-[calc(100%+8px)] top-0 z-30 w-56 rounded-lg border border-[#303543] bg-[#171a22] p-3 text-xs text-[#d6dbe2] shadow-2xl"
          style={{
            scale: `${1 / (zoom * canvasZoom.x)} ${1 / (zoom * canvasZoom.y)}`,
            transformOrigin: "top left",
          }}
          onPointerDownCapture={(event) => event.stopPropagation()}
          onMouseDownCapture={(event) => event.stopPropagation()}
          onClick={(event) => event.stopPropagation()}
        >
          <label className="mb-2 flex items-center justify-between gap-2">Name
            <input aria-label="Shape name" value={label} onChange={(event) => update({ label: event.currentTarget.value })} className="w-32 rounded border border-[#383f4e] bg-[#0f1117] px-2 py-1" />
          </label>
          <label className="mb-2 flex items-center justify-between gap-2">Line color
            <input aria-label="Shape line color" type="color" value={color} onChange={(event) => update({ color: event.currentTarget.value })} className="h-7 w-10 rounded border border-[#383f4e] bg-transparent" />
          </label>
          <label className="mb-2 flex items-center justify-between gap-2">Text color
            <input aria-label="Shape text color" type="color" value={textColor} onChange={(event) => update({ textColor: event.currentTarget.value })} className="h-7 w-10 rounded border border-[#383f4e] bg-transparent" />
          </label>
          <label className="mb-2 flex items-center justify-between gap-2">Text opacity
            <input aria-label="Shape text opacity" type="range" min={0} max={100} value={Math.round(textOpacity * 100)} onChange={(event) => update({ textOpacity: Number(event.currentTarget.value) / 100 })} className="w-24 accent-violet-500" />
          </label>
          <label className="mb-2 flex items-center justify-between gap-2">Border width
            <input aria-label="Shape border width" type="number" min={1} max={4} step={1} value={lineWidth} onChange={(event) => {
              const value = Number(event.currentTarget.value);
              if (Number.isInteger(value) && value >= 1 && value <= 4) update({ lineWidth: value });
            }} className="w-20 rounded border border-[#383f4e] bg-[#0f1117] px-2 py-1" />
          </label>
          <label className="mb-2 flex items-center justify-between gap-2">Line opacity
            <input aria-label="Shape line opacity" type="range" min={0} max={100} value={Math.round(lineOpacity * 100)} onChange={(event) => update({ lineOpacity: Number(event.currentTarget.value) / 100 })} className="w-24 accent-violet-500" />
          </label>
          <label className="mb-2 flex items-center justify-between gap-2">Line style
            <select aria-label="Shape line style" value={lineStyle} onChange={(event) => {
              const value = event.currentTarget.value;
              if (value === "solid" || value === "dashed" || value === "dotted") update({ lineStyle: value });
            }} className="rounded border border-[#383f4e] bg-[#0f1117] px-2 py-1">
              <option value="solid">Solid</option>
              <option value="dashed">Dashed</option>
              <option value="dotted">Dotted</option>
            </select>
          </label>
          {data.shape === "rectangle" ? <>
            <label className="mb-2 flex items-center justify-between gap-2">Fill color
              <input aria-label="Rectangle fill color" type="color" value={fillColor} onChange={(event) => update({ fillColor: event.currentTarget.value })} className="h-7 w-10 rounded border border-[#383f4e] bg-transparent" />
            </label>
            <label className="flex items-center justify-between gap-2">Fill opacity
              <input aria-label="Rectangle fill opacity" type="range" min={0} max={100} value={Math.round(fillOpacity * 100)} onChange={(event) => update({ fillOpacity: Number(event.currentTarget.value) / 100 })} className="w-24 accent-violet-500" />
            </label>
          </> : <>
            <label className="mb-2 flex items-center justify-between gap-2">Start point
              <select aria-label="Path start marker" value={data.startArrow ? "arrow" : "none"} onChange={(event) => update({ startArrow: event.currentTarget.value === "arrow" })} className="rounded border border-[#383f4e] bg-[#0f1117] px-2 py-1">
                <option value="none">None</option>
                <option value="arrow">Arrow</option>
              </select>
            </label>
            <label className="mb-2 flex items-center justify-between gap-2">End point
              <select aria-label="Path end marker" value={data.endArrow ? "arrow" : "none"} onChange={(event) => update({ endArrow: event.currentTarget.value === "arrow" })} className="rounded border border-[#383f4e] bg-[#0f1117] px-2 py-1">
                <option value="none">None</option>
                <option value="arrow">Arrow</option>
              </select>
            </label>
            <div className="mb-1 font-medium text-[#aeb4c0]">Path points</div>
            {data.points.map((point, index) => (
              <div key={index} className="mb-1 grid grid-cols-[auto_1fr_1fr] items-center gap-1">
                <span>{index + 1}</span>
                <input aria-label={`Path point ${index + 1} x`} type="number" min={0} max={1} step={0.01} value={point.x} onChange={(event) => {
                  const value = Number(event.currentTarget.value);
                  if (Number.isFinite(value) && value >= 0 && value <= 1) update({ points: data.points.map((item, itemIndex) => itemIndex === index ? { ...item, x: value } : item) });
                }} className="min-w-0 rounded border border-[#383f4e] bg-[#0f1117] px-1 py-1" />
                <input aria-label={`Path point ${index + 1} y`} type="number" min={0} max={1} step={0.01} value={point.y} onChange={(event) => {
                  const value = Number(event.currentTarget.value);
                  if (Number.isFinite(value) && value >= 0 && value <= 1) update({ points: data.points.map((item, itemIndex) => itemIndex === index ? { ...item, y: value } : item) });
                }} className="min-w-0 rounded border border-[#383f4e] bg-[#0f1117] px-1 py-1" />
              </div>
            ))}
            <div className="mt-3 border-t border-[#303543] pt-3">
              <div className="mb-2 font-medium text-[#aeb4c0]">Path sections</div>
              <div className="mb-2 flex flex-wrap gap-1">
                {pathSegments.map((segment, index) => (
                  <button
                    key={segment.id}
                    type="button"
                    aria-pressed={selectedPathSegmentIds.has(segment.id)}
                    onClick={(event) => handlePathSegmentClick(id, segment.id, event.shiftKey, event.ctrlKey, event.metaKey)}
                    className={`rounded border px-2 py-1 text-[10px] ${selectedPathSegmentIds.has(segment.id) ? "border-violet-400 bg-violet-500/20 text-white" : "border-[#383f4e] text-[#aeb4c0]"}`}
                  >
                    <span className="mr-1 inline-block h-2 w-2 rounded-full" style={{ backgroundColor: segment.color }} />
                    {segment.name || `Section ${index + 1}`}
                  </button>
                ))}
              </div>
              {selectedPathSegment && <>
                <label className="mb-2 flex items-center justify-between gap-2">Section name
                  <input
                    aria-label="Path section name"
                    maxLength={80}
                    value={selectedPathSegment.name}
                    onChange={(event) => updatePathSegment(selectedPathSegment.id, { name: event.currentTarget.value })}
                    className="w-32 rounded border border-[#383f4e] bg-[#0f1117] px-2 py-1"
                  />
                </label>
                <label className="mb-2 flex flex-col gap-1">Details
                  <textarea
                    aria-label="Path section details"
                    maxLength={2000}
                    value={selectedPathSegment.details}
                    onChange={(event) => updatePathSegment(selectedPathSegment.id, { details: event.currentTarget.value })}
                    className="min-h-14 resize-y rounded border border-[#383f4e] bg-[#0f1117] px-2 py-1"
                  />
                </label>
                <label className="mb-2 flex items-center justify-between gap-2">Section color
                  <input
                    aria-label="Path section color"
                    type="color"
                    value={selectedPathSegment.color}
                    onChange={(event) => updatePathSegment(selectedPathSegment.id, { color: event.currentTarget.value })}
                    className="h-7 w-10 rounded border border-[#383f4e] bg-transparent"
                  />
                </label>
                <div className="flex gap-2">
                  <button
                    type="button"
                    disabled={pathSegments.length < 2}
                    title={pathSegments.length < 2 ? "This Path section is already separate" : "Cut this section into its own Path"}
                    onClick={() => window.dispatchEvent(new CustomEvent("vora:cut-path-segment", {
                      detail: { nodeId: id, segmentId: selectedPathSegment.id },
                    }))}
                    className="flex-1 rounded border border-[#51415c] px-2 py-1 text-rose-300 hover:bg-rose-500/10 disabled:cursor-not-allowed disabled:opacity-40"
                  >
                    Cut section
                  </button>
                  <button
                    type="button"
                    disabled={Boolean(selectedPathSegment.libraryItemGroupId)}
                    title={selectedPathSegment.libraryItemGroupId ? "This section is already linked to a Library item" : "Save this section to the Library"}
                    onClick={() => window.dispatchEvent(new CustomEvent("vora:add-path-segment-to-library", {
                      detail: { nodeId: id, segmentId: selectedPathSegment.id },
                    }))}
                    className="flex-1 rounded border border-[#51415c] px-2 py-1 text-violet-300 hover:bg-violet-500/10 disabled:cursor-not-allowed disabled:opacity-40"
                  >
                    Add to Library
                  </button>
                </div>
              </>}
            </div>
          </>}
        </div>
      )}
      <svg
        ref={svgRef}
        width="100%"
        height="100%"
        viewBox={`0 0 ${shapeWidth} ${shapeHeight}`}
        preserveAspectRatio="none"
        className="overflow-visible vora-shape-artwork"
        pointerEvents="none"
        aria-label={label}
      >
        {data.shape === "rectangle" ? (
          <>
            <rect
              className="vora-shape-visual"
              x={data.lineWidth / 2}
              y={data.lineWidth / 2}
              width={Math.max(0, shapeWidth - data.lineWidth)}
              height={Math.max(0, shapeHeight - data.lineWidth)}
              fill={fillColor}
              fillOpacity={fillOpacity}
              stroke={color}
              strokeOpacity={lineOpacity}
              strokeWidth={lineWidth}
              strokeDasharray={dashArray}
              pointerEvents="none"
            />
            <rect
              className="vora-shape-border-hit-target"
              x={data.lineWidth / 2}
              y={data.lineWidth / 2}
              width={Math.max(0, shapeWidth - data.lineWidth)}
              height={Math.max(0, shapeHeight - data.lineWidth)}
              fill="none"
              stroke="transparent"
              strokeWidth={selectionHitStrokeWidth}
              pointerEvents="stroke"
            />
          </>
        ) : (
          <>
            <defs>
              <marker id={`${markerId}-arrow`} markerWidth="9" markerHeight="9" refX="8" refY="4.5" orient="auto" markerUnits="userSpaceOnUse">
                <path d="M1 1L8 4.5L1 8" fill="none" stroke={pathSegments[pathSegments.length - 1]?.color ?? color} strokeOpacity={lineOpacity} strokeWidth={lineWidth} strokeLinecap="round" strokeLinejoin="round" />
              </marker>
              <marker id={`${markerId}-start-arrow`} markerWidth="9" markerHeight="9" refX="1" refY="4.5" orient="auto-start-reverse" markerUnits="userSpaceOnUse">
                <path d="M8 1L1 4.5L8 8" fill="none" stroke={pathSegments[0]?.color ?? color} strokeOpacity={lineOpacity} strokeWidth={lineWidth} strokeLinecap="round" strokeLinejoin="round" />
              </marker>
            </defs>
            <path
              d={pathData}
              fill="none"
              stroke="transparent"
              strokeOpacity={0}
              strokeWidth={lineWidth}
              strokeDasharray={dashArray}
              strokeLinecap="round"
              strokeLinejoin="round"
              markerStart={data.startArrow ? `url(#${markerId}-start-arrow)` : undefined}
              markerEnd={data.endArrow ? `url(#${markerId}-arrow)` : undefined}
              pointerEvents="none"
            />
            {pathSegmentGeometry.map(({ segment, d }) => (
              <g key={segment.id}>
                <path
                  d={d}
                  fill="none"
                  stroke="transparent"
                  strokeWidth={selectionHitStrokeWidth}
                  strokeLinecap="round"
                  pointerEvents="stroke"
                  role="button"
                  tabIndex={0}
                  aria-label={`Select path section ${segment.name}`}
                  aria-pressed={selectedPathSegmentIds.has(segment.id)}
                  onClick={(event) => handlePathSegmentClick(id, segment.id, event.shiftKey, event.ctrlKey, event.metaKey)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      dispatchPathSegmentSelection(id, segment.id, "replace");
                    }
                  }}
                />
                <path
                  d={d}
                  fill="none"
                  stroke={segment.color}
                  strokeOpacity={lineOpacity}
                  strokeWidth={lineWidth + (selectedPathSegmentIds.has(segment.id) ? 1 : 0)}
                  strokeDasharray={dashArray}
                  strokeLinecap="round"
                  pointerEvents="none"
                />
              </g>
            ))}
            {selected && !nodeLocked && draftPoints.map((point, index) => (
              <g key={index} className="nodrag nopan">
                <ellipse
                  cx={point.x * shapeWidth}
                  cy={point.y * shapeHeight}
                  rx={14 / pathHandleScaleX}
                  ry={14 / pathHandleScaleY}
                  fill="transparent"
                  pointerEvents="all"
                  onPointerEnter={() => setHoveredPathPointIndex(index)}
                  onPointerLeave={() => {
                    if (draggingPointRef.current === null) setHoveredPathPointIndex(null);
                  }}
                  onPointerDown={(event) => {
                    event.preventDefault();
                    event.stopPropagation();
                    draggingPointRef.current = index;
                    setHoveredPathPointIndex(index);
                    const node = getNode(id);
                    pathPointDragStartRef.current = {
                      position: node?.position ?? { x: 0, y: 0 },
                      width: shapeWidth,
                      height: shapeHeight,
                    };
                    pointDraftRef.current = data.points;
                    setDraftPoints(data.points);
                    event.currentTarget.setPointerCapture(event.pointerId);
                  }}
                />
                <ellipse
                  cx={point.x * shapeWidth}
                  cy={point.y * shapeHeight}
                  rx={(hoveredPathPointIndex === index ? 6 : 5) / pathHandleScaleX}
                  ry={(hoveredPathPointIndex === index ? 6 : 5) / pathHandleScaleY}
                  fill="#0a0c10"
                  stroke="#2962ff"
                  strokeWidth={hoveredPathPointIndex === index ? 2.5 : 2}
                  vectorEffect="non-scaling-stroke"
                  pointerEvents="none"
                />
              </g>
            ))}
          </>
        )}
        {hasCustomLabel && (
          <text x="8" y="19" fill={textColor} fillOpacity={textOpacity} fontSize="12" fontFamily="sans-serif" pointerEvents="none" aria-hidden="true">{label}</text>
        )}
      </svg>
    </div>
  );
});

VoraShapeNode.displayName = "VoraShapeNode";