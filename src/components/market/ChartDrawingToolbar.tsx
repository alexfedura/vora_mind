import { Fragment, useEffect, useRef, useState, type ReactElement } from "react";
import { createPortal } from "react-dom";
import { BrushToolIcon } from "@/components/market/BrushToolIcon";
import { PathToolIcon } from "@/components/market/PathToolIcon";
import { RectangleToolIcon } from "@/components/market/RectangleToolIcon";
import {
  CircleDot,
  Eraser,
  Info,
  Magnet,
  MousePointer2,
  MousePointerClick,
  Repeat,
  Ruler,
  Star,
  WandSparkles,
  ZoomIn,
} from "lucide-react";
import { TOOL_GROUPS, type DrawingToolOption } from "@/components/market/drawingToolGroups";
import { ChartIconPicker, type ChartIconCategory } from "@/components/market/ChartIconPicker";
import { ChevronRight } from "lucide-react";

interface Props {
  activeTool: string;
  magnetMode: "weak" | "strong";
  magnetEnabled: boolean;
  keepDrawing: boolean;
  drawingsLocked: boolean;
  drawingsHidden: boolean;
  removableDrawingCount: number;
  indicatorCount: number;
  alwaysRemoveLockedDrawings: boolean;
  favoriteToolbarVisible: boolean;
  onSelectTool: (id: string) => void;
  onSelectIcon: (category: ChartIconCategory, symbol: string) => void;
  onToggleFavoriteToolbar: () => void;
  onToggleMagnet: () => void;
  onSetMagnetMode: (mode: "weak" | "strong") => void;
  onToggleKeepDrawing: () => void;
  onToggleLock: () => void;
  onLockDrawings: () => void;
  onLockIndicators: () => void;
  onLockAll: () => void;
  onHideDrawings: () => void;
  onHideIndicators: () => void;
  onHidePositionsAndOrders: () => void;
  onHideAllObjects: () => void;
  onRemoveDrawings: () => void;
  onRemoveIndicators: () => void;
  onRemoveAllObjects: () => void;
  onToggleAlwaysRemoveLockedDrawings: () => void;
  onZoomIn: () => void;
  cursorFavorites: string[];
  onToggleCursorFavorite: (id: string) => void;
  toolFavorites: string[];
  onToggleToolFavorite: (id: string) => void;
  valuesTooltipOnLongPress: boolean;
  onToggleValuesTooltip: () => void;
}

const CURSOR_TOOLS: DrawingToolOption[] = [
  { id: "cursor", label: "Cross", icon: CircleDot, implemented: true },
  { id: "cursor-dot", label: "Dot", icon: CircleDot },
  { id: "cursor-arrow", label: "Arrow", icon: MousePointer2 },
  { id: "cursor-demo", label: "Demonstration", icon: MousePointerClick },
  { id: "cursor-magic", label: "Magic", icon: WandSparkles },
  { id: "cursor-eraser", label: "Eraser", icon: Eraser },
];

export function ChartDrawingToolbar({
  activeTool,
  magnetMode,
  magnetEnabled,
  keepDrawing,
  drawingsLocked,
  drawingsHidden,
  removableDrawingCount,
  indicatorCount,
  alwaysRemoveLockedDrawings,
  favoriteToolbarVisible,
  onSelectTool,
  onSelectIcon,
  onToggleFavoriteToolbar,
  onToggleMagnet,
  onSetMagnetMode,
  onToggleKeepDrawing,
  onToggleLock,
  onLockDrawings,
  onLockIndicators,
  onLockAll,
  onHideDrawings,
  onHideIndicators,
  onHidePositionsAndOrders,
  onHideAllObjects,
  onRemoveDrawings,
  onRemoveIndicators,
  onRemoveAllObjects,
  onToggleAlwaysRemoveLockedDrawings,
  onZoomIn,
  cursorFavorites,
  onToggleCursorFavorite,
  valuesTooltipOnLongPress,
  onToggleValuesTooltip,
  toolFavorites,
  onToggleToolFavorite,
}: Props): ReactElement {
  const [openGroup, setOpenGroup] = useState<string | null>(null);
  const [helpToolId, setHelpToolId] = useState<string | null>(null);
  const rootRef = useRef<HTMLElement | null>(null);
  const cursorMenuRef = useRef<HTMLDivElement | null>(null);
  const toolMenuRef = useRef<HTMLDivElement | null>(null);
  const [cursorMenuPosition, setCursorMenuPosition] = useState<{ left: number; top: number } | null>(null);
  const [toolMenuPosition, setToolMenuPosition] = useState<{ left: number; top: number } | null>(null);
  useEffect(() => {
    if (!openGroup) return;
    const closeOnEscape = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        setHelpToolId(null);
        setOpenGroup(null);
      }
    };
    const closeOnOutsidePointer = (event: PointerEvent): void => {
      const target = event.target;
      if (!rootRef.current?.contains(target as Node) && !cursorMenuRef.current?.contains(target as Node) && !toolMenuRef.current?.contains(target as Node)) {
        setHelpToolId(null);
        setOpenGroup(null);
      }
    };
    document.addEventListener("keydown", closeOnEscape);
    document.addEventListener("pointerdown", closeOnOutsidePointer);
    return () => {
      document.removeEventListener("keydown", closeOnEscape);
      document.removeEventListener("pointerdown", closeOnOutsidePointer);
    };
  }, [openGroup]);

  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent): void => {
      if (!event.altKey || event.ctrlKey || event.metaKey || event.key.toLowerCase() !== "f") return;
      if (event.target instanceof HTMLElement && (event.target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(event.target.tagName))) return;
      const tool = TOOL_GROUPS.flatMap((group) => group.tools).find((candidate) => candidate.shortcut === "Alt + F" && candidate.implemented === true);
      if (!tool) return;
      event.preventDefault();
      setOpenGroup(null);
      onSelectTool(tool.id);
    };
    document.addEventListener("keydown", handleShortcut);
    return () => document.removeEventListener("keydown", handleShortcut);
  }, [onSelectTool]);

  const utilityButtons = [
    { id: "measure", label: "Measure", icon: Ruler, active: activeTool === "measure", disabled: false, onClick: () => onSelectTool("measure") },
    { id: "zoom-in", label: "Zoom in", icon: ZoomIn, active: false, disabled: false, onClick: onZoomIn },
    { id: "magnet", label: "Magnet", icon: Magnet, active: magnetEnabled, disabled: false, onClick: onToggleMagnet },
    { id: "keep-drawing", label: "Keep drawing", icon: Repeat, active: keepDrawing, disabled: false, onClick: onToggleKeepDrawing },
  ];
  const toggleLockMenu = (event: React.MouseEvent<HTMLButtonElement>): void => {
    if (openGroup === "lock-objects") {
      setOpenGroup(null);
      return;
    }
    const controlRect = event.currentTarget.parentElement?.getBoundingClientRect();
    if (controlRect) {
      setToolMenuPosition({
        left: Math.max(8, Math.min(controlRect.right + 1, window.innerWidth - 131)),
        top: Math.max(8, Math.min(controlRect.top, window.innerHeight - 116)),
      });
    }
    setOpenGroup("lock-objects");
  };
  const toggleMagnetMenu = (event: React.MouseEvent<HTMLButtonElement>): void => {
    if (openGroup === "magnet") {
      setOpenGroup(null);
      return;
    }
    const controlRect = event.currentTarget.parentElement?.getBoundingClientRect();
    if (controlRect) {
      setToolMenuPosition({
        left: Math.max(8, Math.min(controlRect.right + 1, window.innerWidth - 211)),
        top: Math.max(8, Math.min(controlRect.top - 8, window.innerHeight - 145)),
      });
    }
    setOpenGroup("magnet");
  };
  const toggleIconPicker = (event: React.MouseEvent<HTMLButtonElement>): void => {
    if (openGroup === "icons") {
      setOpenGroup(null);
      return;
    }
    const controlRect = event.currentTarget.parentElement?.getBoundingClientRect();
    if (controlRect) {
      const pickerHeight = Math.min(window.innerHeight - 16, 680);
      setToolMenuPosition({
        left: Math.max(8, Math.min(controlRect.right + 1, window.innerWidth - 362)),
        top: Math.max(8, Math.min(controlRect.top + controlRect.height / 2 - pickerHeight / 2, window.innerHeight - pickerHeight - 8)),
      });
    }
    setOpenGroup("icons");
  };
  const toggleHideMenu = (event: React.MouseEvent<HTMLButtonElement>): void => {
    if (openGroup === "hide-objects") {
      setOpenGroup(null);
      return;
    }
    const controlRect = event.currentTarget.parentElement?.getBoundingClientRect();
    if (controlRect) {
      setToolMenuPosition({
        left: Math.max(8, Math.min(controlRect.right + 1, window.innerWidth - 195)),
        top: Math.max(8, Math.min(controlRect.top, window.innerHeight - 136)),
      });
    }
    setOpenGroup("hide-objects");
  };
  const toggleRemoveMenu = (event: React.MouseEvent<HTMLButtonElement>): void => {
    if (openGroup === "remove-objects") {
      setOpenGroup(null);
      return;
    }
    const controlRect = event.currentTarget.parentElement?.getBoundingClientRect();
    if (controlRect) {
      setToolMenuPosition({
        left: Math.max(8, Math.min(controlRect.right + 1, window.innerWidth - 297)),
        top: Math.max(8, Math.min(controlRect.top - 52, window.innerHeight - 161)),
      });
    }
    setOpenGroup("remove-objects");
  };
  const drawingLabel = (count: number): string => `Remove ${count} drawing${count === 1 ? "" : "s"}`;
  const indicatorCountLabel = `${indicatorCount} indicator${indicatorCount === 1 ? "" : "s"}`;
  const indicatorLabel = `Remove ${indicatorCountLabel}`;
  const removeBothLabel = `${drawingLabel(removableDrawingCount)} & ${indicatorCountLabel}`;

  return (
    <aside
      ref={rootRef}
      aria-label="Drawing tools"
      className="chart-drawing-toolbar"
    >
      <div className="chart-drawing-toolbar__cursor-control">
        <button
          type="button"
          aria-label="Cross"
          aria-pressed={activeTool === "cursor"}
          title="Cross"
          onClick={() => {
            setOpenGroup(null);
            onSelectTool("cursor");
          }}
          className={`chart-drawing-toolbar__button chart-drawing-toolbar__button--cursor${activeTool === "cursor" ? " is-active" : ""}`}
        >
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true">
            <g fill="currentColor">
              <path d="M18 15h8v-1h-8z" />
              <path d="M14 18v8h1v-8zM14 3v8h1V3zM3 15h8v-1H3z" />
            </g>
          </svg>
        </button>
        <button
          type="button"
          aria-label="Cursors"
          aria-pressed={activeTool === "cursor"}
          aria-expanded={openGroup === "cursors"}
          aria-haspopup="menu"
          title="Cursors"
          onClick={(event) => {
            if (openGroup === "cursors") {
              setOpenGroup(null);
              return;
            }
            const controlRect = event.currentTarget.parentElement?.getBoundingClientRect();
            if (controlRect) {
              setCursorMenuPosition({
                left: Math.max(4, Math.min(controlRect.right + 1, window.innerWidth - 266)),
                top: Math.max(4, Math.min(controlRect.top, window.innerHeight - 302)),
              });
            }
            setOpenGroup("cursors");
          }}
          className={`chart-drawing-toolbar__cursor-arrow${openGroup === "cursors" ? " is-open" : ""}`}
        >
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 16" width="10" height="16" aria-hidden="true">
            <path d="M.6 1.4l1.4-1.4 8 8-8 8-1.4-1.4 6.389-6.532-6.389-6.668z" fill="currentColor" />
          </svg>
        </button>
      </div>
      {openGroup === "cursors" && cursorMenuPosition && createPortal(
          <div
            ref={cursorMenuRef}
            role="menu"
            aria-label="Cursors"
            className="chart-drawing-toolbar__menu chart-drawing-toolbar__menu--cursors"
            style={cursorMenuPosition}
          >
            {CURSOR_TOOLS.map((tool) => {
              const ToolIcon = tool.icon;
              return (
                <Fragment key={tool.id}>
                  <div role="none" className="chart-drawing-toolbar__cursor-item">
                    <button
                      type="button"
                      role="menuitem"
                      disabled={tool.implemented !== true}
                      title={tool.implemented === true ? tool.label : `${tool.label} (not available yet)`}
                      aria-pressed={activeTool === tool.id}
                      onClick={() => {
                        if (tool.implemented !== true) return;
                        onSelectTool(tool.id);
                        setOpenGroup(null);
                      }}
                      className={`chart-drawing-toolbar__cursor-item-main${activeTool === tool.id ? " is-selected" : ""}`}
                    >
                      {tool.id === "cursor" ? (
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true">
                          <g fill="currentColor">
                            <path d="M18 15h8v-1h-8z" />
                            <path d="M14 18v8h1v-8zM14 3v8h1V3zM3 15h8v-1H3z" />
                          </g>
                        </svg>
                      ) : tool.id === "cursor-dot" ? (
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true"><circle cx="14" cy="14" r="3" fill="currentColor" /></svg>
                      ) : (
                        <ToolIcon size={28} strokeWidth={1.45} />
                      )}
                      <span>{tool.label}</span>
                      {tool.implemented !== true && <span className="chart-drawing-toolbar__unavailable">Soon</span>}
                    </button>
                    <button
                      type="button"
                      aria-label={`${cursorFavorites.includes(tool.id) ? "Remove" : "Add"} ${tool.label} ${cursorFavorites.includes(tool.id) ? "from" : "to"} favorites`}
                      aria-pressed={cursorFavorites.includes(tool.id)}
                      title={cursorFavorites.includes(tool.id) ? "Remove from favorites" : "Add to favorites"}
                      onClick={() => onToggleCursorFavorite(tool.id)}
                      className="chart-drawing-toolbar__cursor-favorite"
                    >
                      <Star size={18} strokeWidth={1.5} fill={cursorFavorites.includes(tool.id) ? "currentColor" : "none"} />
                    </button>
                  </div>
                  {(tool.id === "cursor-magic" || tool.id === "cursor-eraser") && <div role="separator" className="chart-drawing-toolbar__cursor-divider" />}
                </Fragment>
              );
            })}
            <div role="none" className="chart-drawing-toolbar__cursor-setting">
              <span>Values tooltip on long press</span>
              <button
                type="button"
                role="switch"
                aria-label="Values tooltip on long press"
                aria-checked={valuesTooltipOnLongPress}
                onClick={onToggleValuesTooltip}
                className={`chart-drawing-toolbar__cursor-switch${valuesTooltipOnLongPress ? " is-checked" : ""}`}
              >
                <span />
              </button>
            </div>
          </div>,
          document.body,
        )}
      <div className="chart-drawing-toolbar__groups">
        {TOOL_GROUPS.map((group) => {
          const Icon = group.icon;
          const isOpen = openGroup === group.id;
          const selectedTool = group.tools.find((tool) => tool.id === activeTool);
          const primaryTool = selectedTool ?? group.tools.find((tool) => tool.implemented === true);
          const primaryDisplayTool = primaryTool ?? group.tools[0];
          const PrimaryIcon = primaryDisplayTool?.icon ?? Icon;
          return (
            <div key={group.id} className="chart-drawing-toolbar__group">
              <button
                type="button"
                aria-label={primaryDisplayTool?.label ?? group.label}
                aria-pressed={selectedTool !== undefined}
                aria-expanded={group.id === "icons" ? isOpen : undefined}
                aria-haspopup={group.id === "icons" ? "dialog" : undefined}
                title={primaryDisplayTool?.label ?? group.label}
                disabled={primaryTool === undefined}
                onClick={(event) => {
                  if (!primaryTool) return;
                  if (group.id === "icons") {
                    toggleIconPicker(event);
                    return;
                  }
                  setOpenGroup(null);
                  onSelectTool(primaryTool.id);
                }}
                className={`chart-drawing-toolbar__button chart-drawing-toolbar__tool-main${primaryDisplayTool?.id === "brush" ? " chart-drawing-toolbar__tool-main--brush" : ""}${selectedTool ? " is-active" : ""}${isOpen ? " is-open" : ""}`}
              >
                {primaryDisplayTool?.id === "path" ? (
                  <PathToolIcon />
                ) : primaryDisplayTool?.id === "rect" ? (
                  <RectangleToolIcon />
                ) : primaryDisplayTool?.id === "brush" ? (
                  <BrushToolIcon />
                ) : (
                  <PrimaryIcon size={28} strokeWidth={1.45} />
                )}
              </button>
              <button
                type="button"
                aria-label={group.label}
                aria-expanded={isOpen}
                aria-haspopup="menu"
                title={group.label}
                onClick={(event) => {
                  if (group.id === "icons") {
                    toggleIconPicker(event);
                    return;
                  }
                  if (isOpen) {
                    setOpenGroup(null);
                    setHelpToolId(null);
                    return;
                  }
                  const controlRect = event.currentTarget.parentElement?.getBoundingClientRect();
                  if (controlRect) {
                    const sectionCount = group.tools.filter((tool) => tool.sectionBefore).length;
                    const rowHeight = group.id === "annotation" ? 48 : 44;
                    const estimatedHeight = group.tools.length * rowHeight + sectionCount * 32 + (group.id === "trend" ? 12 : 48);
                    const menuHeight = Math.min(window.innerHeight - 16, estimatedHeight);
                    setToolMenuPosition({
                      left: Math.max(8, Math.min(controlRect.right + 4, window.innerWidth - 276)),
                      top: Math.max(8, Math.min(controlRect.top, window.innerHeight - menuHeight - 8)),
                    });
                  }
                  setHelpToolId(null);
                  setOpenGroup(group.id);
                }}
                className={`chart-drawing-toolbar__group-arrow${isOpen ? " is-open" : ""}`}
              >
                <ChevronRight size={13} strokeWidth={1.8} aria-hidden="true" />
              </button>
              {isOpen && group.id === "icons" && toolMenuPosition && createPortal(
                <div
                  ref={toolMenuRef}
                  role="dialog"
                  aria-label="Icon picker"
                  className="chart-drawing-toolbar__menu chart-icon-picker__popover"
                  style={toolMenuPosition}
                >
                  <ChartIconPicker onSelect={(category, symbol) => {
                    onSelectIcon(category, symbol);
                    setOpenGroup(null);
                  }} />
                </div>,
                document.body,
              )}
              {isOpen && group.id !== "icons" && toolMenuPosition && createPortal(
                <div
                  ref={toolMenuRef}
                  role="menu"
                  aria-label={group.label}
                  className={`chart-drawing-toolbar__menu${group.id === "annotation" ? " chart-drawing-toolbar__menu--annotation" : ""}`}
                  style={toolMenuPosition}
                >
                  {group.id !== "trend" && group.id !== "gann-fibonacci" && group.id !== "patterns" && group.id !== "forecast-measurement" && <div className="chart-drawing-toolbar__menu-title">{group.label}</div>}
                  {group.tools.map((tool) => {
                    const ToolIcon = tool.icon;
                    const isFavorite = toolFavorites.includes(tool.id);
                    const isHelpOpen = helpToolId === tool.id;
                    const helpText = tool.implemented !== true
                      ? `${tool.label} is not available yet.`
                      : tool.id === "rr_long"
                        ? "Mark a long trade setup by placing the entry, stop-loss, and take-profit levels on the chart."
                        : tool.id === "rr_short"
                          ? "Mark a short trade setup by placing the entry, stop-loss, and take-profit levels on the chart."
                          : `Select ${tool.label}, then use the chart to place and complete the drawing.`;
                    return (
                      <Fragment key={tool.id}>
                        {tool.sectionBefore && <div className={`chart-drawing-toolbar__menu-section${((group.id === "trend" && tool.id === group.tools[0]?.id) || (group.id === "gann-fibonacci" && tool.id === "fib-retracement") || (group.id === "patterns" && tool.id === group.tools[0]?.id) || (group.id === "forecast-measurement" && tool.id === group.tools[0]?.id)) ? " chart-drawing-toolbar__menu-section--first" : ""}`}>{tool.sectionBefore}</div>}
                        <div role="none" className={`chart-drawing-toolbar__menu-row${activeTool === tool.id ? " is-selected" : ""}${isHelpOpen ? " is-help-open" : ""}`}>
                          <button
                            type="button"
                            role="menuitem"
                            disabled={tool.implemented !== true}
                            title={tool.implemented === true ? tool.label : `${tool.label} (not available yet)`}
                            aria-pressed={activeTool === tool.id}
                            onClick={() => {
                              if (tool.implemented !== true) return;
                              onSelectTool(tool.id);
                              setOpenGroup(null);
                            }}
                            className="chart-drawing-toolbar__menu-item"
                          >
                            {tool.id === "rr_long" ? (
                              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true">
                                <path fill="currentColor" d="M5.5 20c1.2 0 2.22.86 2.45 2H25v1H7.95a2.5 2.5 0 1 1-2.45-3m0 1a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3M25 18H5v-1h20zm-11-4h3v1h-4V9h1zM5.5 4c1.2 0 2.22.86 2.45 2H25v1H7.95A2.5 2.5 0 1 1 5.5 4m0 1a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3" />
                              </svg>
                            ) : tool.id === "rr_short" ? (
                              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true">
                                <path transform="translate(0 28) scale(1 -1)" fill="currentColor" d="M5.5 20c1.2 0 2.22.86 2.45 2H25v1H7.95a2.5 2.5 0 1 1-2.45-3m0 1a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3M25 18H5v-1h20zm-11-4h3v1h-4V9h1zM5.5 4c1.2 0 2.22.86 2.45 2H25v1H7.95A2.5 2.5 0 1 1 5.5 4m0 1a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3" />
                              </svg>
                            ) : tool.id === "path" ? (
                              <PathToolIcon />
                            ) : tool.id === "rect" ? (
                              <RectangleToolIcon />
                            ) : tool.id === "brush" ? (
                              <BrushToolIcon />
                            ) : (
                              <ToolIcon size={28} strokeWidth={1.45} />
                            )}
                            <span className="chart-drawing-toolbar__menu-label">{tool.label}</span>
                            {tool.shortcut && <span className="chart-drawing-toolbar__shortcut">{tool.shortcut}</span>}
                            {tool.implemented !== true && <span className="chart-drawing-toolbar__unavailable">Unavailable</span>}
                          </button>
                          <button
                            type="button"
                            aria-label={`About ${tool.label}`}
                            aria-expanded={isHelpOpen}
                            aria-controls={isHelpOpen ? `chart-tool-help-${tool.id}` : undefined}
                            title={`About ${tool.label}`}
                            onClick={() => setHelpToolId(isHelpOpen ? null : tool.id)}
                            className="chart-drawing-toolbar__menu-help"
                          >
                            <Info size={17} strokeWidth={1.8} aria-hidden="true" />
                          </button>
                          <button
                            type="button"
                            aria-label={`${isFavorite ? "Remove" : "Add"} ${tool.label} ${isFavorite ? "from" : "to"} favorites`}
                            aria-pressed={isFavorite}
                            title={isFavorite ? "Remove from favorites" : "Add to favorites"}
                            onClick={() => onToggleToolFavorite(tool.id)}
                            className="chart-drawing-toolbar__menu-favorite"
                          >
                            <Star size={18} strokeWidth={1.5} fill={isFavorite ? "currentColor" : "none"} />
                          </button>
                        </div>
                        {isHelpOpen && (
                          <div id={`chart-tool-help-${tool.id}`} role="note" className="chart-drawing-toolbar__menu-help-content">
                            <strong>{tool.label}</strong>
                            <span>{helpText}</span>
                            {tool.shortcut && <span className="chart-drawing-toolbar__menu-help-shortcut">Shortcut: {tool.shortcut}</span>}
                          </div>
                        )}
                      </Fragment>
                    );
                  })}
                </div>,
                document.body,
              )}
            </div>
          );
        })}
      </div>
      <div className="chart-drawing-toolbar__utilities">
        {utilityButtons.map(({ id, label, icon: Icon, active, disabled, onClick }) => id === "magnet" ? (
          <Fragment key={id}>
            <div className="chart-drawing-toolbar__magnet-control">
              <button
                type="button"
                aria-label={label}
                aria-pressed={active}
                aria-expanded={openGroup === "magnet"}
                aria-haspopup="menu"
                title={label}
                onClick={onClick}
                className={`chart-drawing-toolbar__button chart-drawing-toolbar__magnet-main${active ? " is-active" : ""}`}
              >
                {magnetMode === "strong" ? (
                  <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" aria-hidden="true">
                    <path fill="currentColor" fillRule="nonzero" d="M14 5a7 7 0 0 0-7 7v3h4v-3a3 3 0 1 1 6 0v3h4v-3a7 7 0 0 0-7-7zm7 11h-4v3h4v-3zm-10 0H7v3h4v-3zm-5-4a8 8 0 1 1 16 0v8h-6v-8a2 2 0 1 0-4 0v8H6v-8zm3.293 11.294l-1.222-2.037.858-.514 1.777 2.963-2 1 1.223 2.037-.858.514-1.778-2.963 2-1zm9.778-2.551l.858.514-1.223 2.037 2 1-1.777 2.963-.858-.514 1.223-2.037-2-1 1.777-2.963z" />
                  </svg>
                ) : (
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true">
                    <g fill="currentColor" fillRule="evenodd">
                      <path fillRule="nonzero" d="M14 10a2 2 0 0 0-2 2v11H6V12c0-4.416 3.584-8 8-8s8 3.584 8 8v11h-6V12a2 2 0 0 0-2-2zm-3 2a3 3 0 0 1 6 0v10h4V12c0-3.864-3.136-7-7-7s-7 3.136-7 7v10h4V12z" />
                      <path d="M6.5 18h5v1h-5zm10 0h5v1h-5z" />
                    </g>
                  </svg>
                )}
              </button>
              <button
                type="button"
                aria-label="Magnet options"
                title="Magnet options"
                aria-expanded={openGroup === "magnet"}
                aria-haspopup="menu"
                onClick={toggleMagnetMenu}
                className={`chart-drawing-toolbar__magnet-arrow${openGroup === "magnet" ? " is-open" : ""}`}
              >
                <ChevronRight size={12} strokeWidth={1.8} aria-hidden="true" />
              </button>
            </div>
            {openGroup === "magnet" && toolMenuPosition && createPortal(
              <div ref={toolMenuRef} role="menu" aria-label="Magnet" className="chart-drawing-toolbar__menu chart-drawing-toolbar__magnet-menu" style={toolMenuPosition}>
                <button type="button" role="menuitemradio" aria-checked={magnetMode === "weak"} onClick={() => { onSetMagnetMode("weak"); setOpenGroup(null); }} className="chart-drawing-toolbar__magnet-option">
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true"><g fill="currentColor" fillRule="evenodd"><path fillRule="nonzero" d="M14 10a2 2 0 0 0-2 2v11H6V12c0-4.416 3.584-8 8-8s8 3.584 8 8v11h-6V12a2 2 0 0 0-2-2zm-3 2a3 3 0 0 1 6 0v10h4V12c0-3.864-3.136-7-7-7s-7 3.136-7 7v10h4V12z"/><path d="M6.5 18h5v1h-5zm10 0h5v1h-5z"/></g></svg>
                  <span>Weak magnet</span>
                </button>
                <button type="button" role="menuitemradio" aria-checked={magnetMode === "strong"} onClick={() => { onSetMagnetMode("strong"); setOpenGroup(null); }} className="chart-drawing-toolbar__magnet-option">
                  <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" aria-hidden="true"><path fill="currentColor" fillRule="nonzero" d="M14 5a7 7 0 0 0-7 7v3h4v-3a3 3 0 1 1 6 0v3h4v-3a7 7 0 0 0-7-7zm7 11h-4v3h4v-3zm-10 0H7v3h4v-3zm-5-4a8 8 0 1 1 16 0v8h-6v-8a2 2 0 1 0-4 0v8H6v-8zm3.293 11.294l-1.222-2.037.858-.514 1.777 2.963-2 1 1.223 2.037-.858.514-1.778-2.963 2-1zm9.778-2.551l.858.514-1.223 2.037 2 1-1.777 2.963-.858-.514 1.223-2.037-2-1 1.777-2.963z"/></svg>
                  <span>Strong magnet</span>
                </button>
                <div role="separator" className="chart-drawing-toolbar__magnet-divider" />
                <button type="button" role="switch" aria-label="Snap to indicators" aria-checked="false" disabled className="chart-drawing-toolbar__magnet-snap-option">
                  <span>Snap to indicators</span>
                  <span className="chart-drawing-toolbar__magnet-switch" aria-hidden="true"><span /></span>
                </button>
              </div>,
              document.body,
            )}
          </Fragment>
        ) : (
          <button
            key={id}
            type="button"
            aria-label={label}
            aria-pressed={active}
            title={label}
            disabled={disabled}
            onClick={onClick}
            className={`chart-drawing-toolbar__button${active ? " is-active" : ""}`}
          >
            {id === "measure" ? (
              <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" aria-hidden="true">
                <path fill="currentColor" d="M2 9.75a1.5 1.5 0 0 0-1.5 1.5v5.5a1.5 1.5 0 0 0 1.5 1.5h24a1.5 1.5 0 0 0 1.5-1.5v-5.5a1.5 1.5 0 0 0-1.5-1.5zm0 1h3v2.5h1v-2.5h3.25v3.9h1v-3.9h3.25v2.5h1v-2.5h3.25v3.9h1v-3.9H22v2.5h1v-2.5h3a.5.5 0 0 1 .5.5v5.5a.5.5 0 0 1-.5.5H2a.5.5 0 0 1-.5-.5v-5.5a.5.5 0 0 1 .5-.5z" transform="rotate(-45 14 14)" />
              </svg>
            ) : id === "zoom-in" ? (
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" fill="currentColor" aria-hidden="true">
                <path d="M17.646 18.354l4 4 .708-.708-4-4z" />
                <path d="M12.5 21a8.5 8.5 0 1 1 0-17 8.5 8.5 0 0 1 0 17zm0-1a7.5 7.5 0 1 0 0-15 7.5 7.5 0 0 0 0 15z" />
                <path d="M9 13h7v-1H9z" />
                <path d="M13 16V9h-1v7z" />
              </svg>
            ) : id === "keep-drawing" ? (
              <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" aria-hidden="true">
                <path fill="currentColor" d="M17.27 4.56a2.5 2.5 0 0 0-3.54 0l-.58.59-9 9-1 1-.15.14V20h4.7l.15-.15 1-1 9-9 .59-.58a2.5 2.5 0 0 0 0-3.54l-1.17-1.17Zm-2.83.7a1.5 1.5 0 0 1 2.12 0l1.17 1.18a1.5 1.5 0 0 1 0 2.12l-.23.23-3.3-3.29.24-.23Zm-.94.95 3.3 3.29-8.3 8.3-3.3-3.3 8.3-8.3Zm-9 9 3.3 3.29-.5.5H4v-3.3l.5-.5Zm16.5.29a1.5 1.5 0 0 0-3 0V18h4.5c.83 0 1.5.67 1.5 1.5v4c0 .83-.67 1.5-1.5 1.5h-6a1.5 1.5 0 0 1-1.5-1.5v-4c0-.83.67-1.5 1.5-1.5h.5v-2.5a2.5 2.5 0 0 1 5 0v.5h-1v-.5ZM16.5 19a.5.5 0 0 0-.5.5v4c0 .28.22.5.5.5h6a.5.5 0 0 0 .5-.5v-4a.5.5 0 0 0-.5-.5h-6Zm2.5 4v-2h1v2h-1Z" />
              </svg>
            ) : (
              <Icon size={18} strokeWidth={1.65} />
            )}
          </button>
        ))}
      </div>
      <div className="chart-drawing-toolbar__lock-control">
        <button
          type="button"
          aria-label={drawingsLocked ? "Unlock drawings" : "Lock drawings"}
          title={drawingsLocked ? "Unlock drawings" : "Lock drawings"}
          aria-pressed={drawingsLocked}
          aria-expanded={openGroup === "lock-objects"}
          aria-haspopup="menu"
          onClick={onToggleLock}
          className="chart-drawing-toolbar__button chart-drawing-toolbar__lock-main"
        >
          {drawingsLocked ? (
            <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" aria-hidden="true">
              <path fill="currentColor" d="M14 4a4 4 0 0 1 4 4v3h1.5a2.5 2.5 0 0 1 2.5 2.5v7a2.5 2.5 0 0 1-2.5 2.5h-11A2.5 2.5 0 0 1 6 20.5v-7A2.5 2.5 0 0 1 8.5 11H10V8a4 4 0 0 1 4-4m-5.5 8A1.5 1.5 0 0 0 7 13.5v7A1.5 1.5 0 0 0 8.5 22h11a1.5 1.5 0 0 0 1.5-1.5v-7a1.5 1.5 0 0 0-1.5-1.5zm5.5 3a1 1 0 0 1 1 1v2a1 1 0 1 1-2 0v-2a1 1 0 0 1 1-1m0-10a3 3 0 0 0-3 3v3h6V8a3 3 0 0 0-3-3" />
            </svg>
          ) : (
            <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" aria-hidden="true">
              <path fill="currentColor" d="M9.877 3.607a3.997 3.997 0 0 1 5.508 1.27l.026.043-.848.53-.01-.017-.005.004a2.998 2.998 0 0 0-5.474 2.226 3 3 0 0 0 .302.79l.08.133L10.964 11H19.5l.256.012A2.5 2.5 0 0 1 22 13.5v7a2.5 2.5 0 0 1-2.5 2.5h-11A2.5 2.5 0 0 1 6 20.5v-7a2.5 2.5 0 0 1 2.244-2.488L8.5 11h1.27L8.612 9.15a4 4 0 0 1-.58-1.645 4 4 0 0 1 1.845-3.898M8.5 12A1.5 1.5 0 0 0 7 13.5v7A1.5 1.5 0 0 0 8.5 22h11a1.5 1.5 0 0 0 1.5-1.5v-7a1.5 1.5 0 0 0-1.5-1.5zm5.5 3a1 1 0 0 1 1 1v2a1 1 0 0 1-2 0v-2a1 1 0 0 1 1-1" />
            </svg>
          )}
        </button>
        <button
          type="button"
          aria-label="Lock objects options"
          title="Lock objects"
          aria-expanded={openGroup === "lock-objects"}
          aria-haspopup="menu"
          onClick={toggleLockMenu}
          className={`chart-drawing-toolbar__lock-arrow${openGroup === "lock-objects" ? " is-open" : ""}`}
        >
          <ChevronRight size={12} strokeWidth={1.8} aria-hidden="true" />
        </button>
      </div>
      {openGroup === "lock-objects" && toolMenuPosition && createPortal(
        <div
          ref={toolMenuRef}
          role="menu"
          aria-label="Lock objects"
          className="chart-drawing-toolbar__menu chart-drawing-toolbar__lock-menu"
          style={toolMenuPosition}
        >
          <button type="button" role="menuitem" onClick={() => { onLockDrawings(); setOpenGroup(null); }} className="chart-drawing-toolbar__lock-menu-item">Lock drawings</button>
          <button type="button" role="menuitem" onClick={() => { onLockIndicators(); setOpenGroup(null); }} className="chart-drawing-toolbar__lock-menu-item">Lock indicators</button>
          <button type="button" role="menuitem" onClick={() => { onLockAll(); setOpenGroup(null); }} className="chart-drawing-toolbar__lock-menu-item">Lock all</button>
        </div>,
        document.body,
      )}
      <div className="chart-drawing-toolbar__hide-control">
        <button
          type="button"
          aria-label="Hide objects"
          title="Hide objects"
          aria-pressed={drawingsHidden}
          aria-expanded={openGroup === "hide-objects"}
          aria-haspopup="menu"
          onClick={toggleHideMenu}
          className={`chart-drawing-toolbar__button chart-drawing-toolbar__hide-main${drawingsHidden ? " is-active" : ""}`}
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" aria-hidden="true">
            <path fill="currentColor" fillRule="evenodd" d="M5 10.76l-.41-.72-.03-.04.03-.04a15 15 0 012.09-2.9c1.47-1.6 3.6-3.12 6.32-3.12 2.73 0 4.85 1.53 6.33 3.12a15.01 15.01 0 012.08 2.9l.03.04-.03.04a15 15 0 01-2.09 2.9c-1.47 1.6-3.6 3.12-6.32 3.12-2.73 0-4.85-1.53-6.33-3.12a15 15 0 01-1.66-2.18zm17.45-.98L22 10l.45.22-.01.02a5.04 5.04 0 01-.15.28 16.01 16.01 0 01-2.23 3.1c-1.56 1.69-3.94 3.44-7.06 3.44-3.12 0-5.5-1.75-7.06-3.44a16 16 0 01-2.38-3.38v-.02h-.01L4 10l-.45-.22.01-.02a5.4 5.4 0 01.15-.28 16 16 0 012.23-3.1C7.5 4.69 9.88 2.94 13 2.94c3.12 0 5.5 1.75 7.06 3.44a16.01 16.01 0 012.38 3.38v.02h.01zM22 10l.45-.22.1.22-.1.22L22 10zM3.55 9.78L4 10l-.45.22-.1-.22.1-.22zm6.8.22A2.6 2.6 0 0113 7.44 2.6 2.6 0 0115.65 10 2.6 2.6 0 0113 12.56 2.6 2.6 0 0110.35 10zM13 6.44A3.6 3.6 0 009.35 10 3.6 3.6 0 0013 13.56c2 0 3.65-1.58 3.65-3.56A3.6 3.6 0 0013 6.44zm7.85 12l.8-.8.7.71-.79.8a.5.5 0 000 .7l.59.59c.2.2.5.2.7 0l1.8-1.8.7.71-1.79 1.8a1.5 1.5 0 01-2.12 0l-.59-.59a1.5 1.5 0 010-2.12zM16.5 21.5l-.35-.35a.5.5 0 00-.07.07l-1 1.5-1 1.5a.5.5 0 00.42.78h4a2.5 2.5 0 001.73-.77A2.5 2.5 0 0021 22.5a2.5 2.5 0 00-.77-1.73A2.5 2.5 0 0018.5 20a3.1 3.1 0 00-1.65.58 5.28 5.28 0 00-.69.55v.01h-.01l.35.36zm.39.32l-.97 1.46-.49.72h3.07c.34 0 .72-.17 1.02-.48.3-.3.48-.68.48-1.02 0-.34-.17-.72-.48-1.02-.3-.3-.68-.48-1.02-.48-.35 0-.75.18-1.1.42a4.27 4.27 0 00-.51.4z" />
          </svg>
        </button>
        <button
          type="button"
          aria-label="Hide objects options"
          title="Hide objects"
          aria-expanded={openGroup === "hide-objects"}
          aria-haspopup="menu"
          onClick={toggleHideMenu}
          className={`chart-drawing-toolbar__hide-arrow${openGroup === "hide-objects" ? " is-open" : ""}`}
        >
          <ChevronRight size={12} strokeWidth={1.8} aria-hidden="true" />
        </button>
      </div>
      {openGroup === "hide-objects" && toolMenuPosition && createPortal(
        <div
          ref={toolMenuRef}
          role="menu"
          aria-label="Hide objects"
          className="chart-drawing-toolbar__menu chart-drawing-toolbar__hide-menu"
          style={toolMenuPosition}
        >
          <button type="button" role="menuitem" onClick={() => { onHideDrawings(); setOpenGroup(null); }} className="chart-drawing-toolbar__hide-menu-item">Hide drawings</button>
          <button type="button" role="menuitem" onClick={() => { onHideIndicators(); setOpenGroup(null); }} className="chart-drawing-toolbar__hide-menu-item">Hide indicators</button>
          <button type="button" role="menuitem" onClick={() => { onHidePositionsAndOrders(); setOpenGroup(null); }} className="chart-drawing-toolbar__hide-menu-item">Hide positions and orders</button>
          <button type="button" role="menuitem" onClick={() => { onHideAllObjects(); setOpenGroup(null); }} className="chart-drawing-toolbar__hide-menu-item">Hide all</button>
        </div>,
        document.body,
      )}
      <div className="chart-drawing-toolbar__remove-control">
        <button
          type="button"
          aria-label="Remove objects"
          title="Remove objects"
          aria-expanded={openGroup === "remove-objects"}
          aria-haspopup="menu"
          onClick={toggleRemoveMenu}
          className="chart-drawing-toolbar__button chart-drawing-toolbar__remove-main"
        >
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true">
            <path fill="currentColor" d="M18 7h5v1h-2.01l-1.33 14.64a1.5 1.5 0 0 1-1.5 1.36H9.84a1.5 1.5 0 0 1-1.49-1.36L7.01 8H5V7h5V6c0-1.1.9-2 2-2h4a2 2 0 0 1 2 2v1Zm-6-2a1 1 0 0 0-1 1v1h6V6a1 1 0 0 0-1-1h-4ZM8.02 8l1.32 14.54a.5.5 0 0 0 .5.46h8.33a.5.5 0 0 0 .5-.46L19.99 8H8.02Z" />
          </svg>
        </button>
        <button
          type="button"
          aria-label="Remove objects options"
          title="Remove objects"
          aria-expanded={openGroup === "remove-objects"}
          aria-haspopup="menu"
          onClick={toggleRemoveMenu}
          className={`chart-drawing-toolbar__remove-arrow${openGroup === "remove-objects" ? " is-open" : ""}`}
        >
          <ChevronRight size={12} strokeWidth={1.8} aria-hidden="true" />
        </button>
      </div>
      {openGroup === "remove-objects" && toolMenuPosition && createPortal(
        <div
          ref={toolMenuRef}
          role="menu"
          aria-label="Remove objects"
          className="chart-drawing-toolbar__menu chart-drawing-toolbar__remove-menu"
          style={toolMenuPosition}
        >
          <button
            type="button"
            role="menuitem"
            aria-label={drawingLabel(removableDrawingCount)}
            onClick={() => { onRemoveDrawings(); setOpenGroup(null); }}
            className="chart-drawing-toolbar__remove-menu-item"
          >
            {drawingLabel(removableDrawingCount)}
          </button>
          <button
            type="button"
            role="menuitem"
            aria-label={indicatorLabel}
            onClick={() => { onRemoveIndicators(); setOpenGroup(null); }}
            className="chart-drawing-toolbar__remove-menu-item"
          >
            {indicatorLabel}
          </button>
          <button
            type="button"
            role="menuitem"
            aria-label={removeBothLabel}
            onClick={() => { onRemoveAllObjects(); setOpenGroup(null); }}
            className="chart-drawing-toolbar__remove-menu-item"
          >
            {removeBothLabel}
          </button>
          <div role="separator" className="chart-drawing-toolbar__remove-menu-divider" />
          <button
            type="button"
            role="menuitemcheckbox"
            aria-checked={alwaysRemoveLockedDrawings}
            onClick={onToggleAlwaysRemoveLockedDrawings}
            className="chart-drawing-toolbar__remove-menu-setting"
          >
            <span>Always remove locked drawings</span>
            <span className={`chart-drawing-toolbar__remove-switch${alwaysRemoveLockedDrawings ? " is-checked" : ""}`} aria-hidden="true">
              <span />
            </span>
          </button>
        </div>,
        document.body,
      )}
      <button
        type="button"
        id="drawing-toolbar-favorite-drawings"
        aria-label={favoriteToolbarVisible ? "Hide Favorite Drawing Tools Toolbar" : "Show Favorite Drawing Tools Toolbar"}
        aria-pressed={favoriteToolbarVisible}
        title={favoriteToolbarVisible ? "Hide Favorite Drawing Tools Toolbar" : "Show Favorite Drawing Tools Toolbar"}
        onClick={onToggleFavoriteToolbar}
        className={`chart-drawing-toolbar__button${favoriteToolbarVisible ? " is-active" : ""}`}
        style={{ marginTop: "auto" }}
      >
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" fill="none" aria-hidden="true">
          <path
            fill="currentColor"
            fillRule="evenodd"
            d="m17.13 9.74 7.37.9-5.44 5.06L20.4 23 14 19.38 7.6 23l1.34-7.3-5.44-5.06 7.37-.9L14 3l3.13 6.74Zm5.11 1.63-4.26 3.97 1.04 5.74L14 18.24l-5.02 2.84 1.04-5.74-4.26-3.97 5.79-.7L14 5.37l2.45 5.3 5.8.7Z"
          />
        </svg>
      </button>
    </aside>
  );
}