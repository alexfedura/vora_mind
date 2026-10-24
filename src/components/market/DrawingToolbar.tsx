// DrawingToolbar v0.82.1 — exact TradingView 11-button floating toolbar.
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { ChevronDown, ChevronRight, Copy, EyeOff, LockKeyhole, Plus, Trash, X } from "lucide-react";
import type { DrawingAny, LineStyle } from "@/lib/market/lineDrawings";
import { DrawingSettingsDialog } from "@/components/market/DrawingSettingsDialog";
import { CreateDrawingAlertDialog } from "@/components/market/CreateDrawingAlertDialog";
import type { ChartDrawingAlert, DrawingStyleTemplate } from "@/lib/market/drawingTypes";
import { isValidHex } from "@/lib/market/colorUtils";
import { CHART_DRAWING_COLORS } from "@/shared/canvas/DrawingColorPicker";
import { getSetting, setSetting } from "@/shared/store/useSettingsStore";

export interface DrawingToolbarActions {
  onColor: (c: string, opacity: number) => void;
  onColorOpacity: (opacity: number) => void;
  onBgColor: (c: string) => void;
  onBgOpacity: (opacity: number) => void;
  onTextColor: (c: string, opacity: number) => void;
  onWidth: (w: number) => void;
  onStyle: (s: LineStyle) => void;
  onUpdate: (update: (drawing: DrawingAny) => DrawingAny) => void;
  onToggleLock: () => void;
  onDelete: () => void;
  onClone: () => void;
  onCopy: () => void;
  onPaste: () => void;
  onHide: () => void;
  onReorder: (position: "front" | "back") => void;
  templates: DrawingStyleTemplate[];
  onSaveTemplate: (name: string, makeDefault: boolean, settings?: DrawingStyleTemplate["settings"]) => Promise<void>;
  onApplyTemplate: (template: DrawingStyleTemplate) => void;
  onDeleteTemplate: (id: string) => void;
  symbol: string;
  timeframe: number;
  onCreateAlert: (alert: ChartDrawingAlert) => Promise<void>;
}

const POS_KEY = "vora.drawing_toolbar_pos";
const CUSTOM_COLORS_KEY = "vora.drawing_custom_colors";
const REMOVED_COLORS_KEY = "vora.drawing_removed_colors";
type MenuKey = "color" | "background" | "text" | "width" | "style" | "templates" | "more";

function colOf(d: DrawingAny): string {
  if (d.type === "rectangle") return d.borderColor;
  return d.color;
}

function withAlpha(hex: string, alpha: number): string {
  const m = /^#([0-9a-fA-F]{6})$/.exec(hex);
  if (!m) return hex;
  const n = parseInt(m[1], 16);
  const r = (n >> 16) & 255;
  const g = (n >> 8) & 255;
  const b = n & 255;
  return `rgba(${r},${g},${b},${alpha})`;
}

function bgOf(d: DrawingAny): string {
  if (d.type === "rectangle") return d.fillColor ? withAlpha(d.fillColor, d.fillOpacity) : "rgba(156,39,176,0)";
  return withAlpha(colOf(d), 0.2);
}

function loadCustomColors(): string[] {
  try {
    const stored: unknown = JSON.parse(getSetting(CUSTOM_COLORS_KEY) ?? "[]");
    return Array.isArray(stored) ? stored.filter((color): color is string => typeof color === "string" && isValidHex(color)).map((color) => color.toLowerCase()) : [];
  } catch {
    return [];
  }
}

function loadRemovedColors(): string[] {
  try {
    const stored: unknown = JSON.parse(getSetting(REMOVED_COLORS_KEY) ?? "[]");
    return Array.isArray(stored) ? stored.filter((color): color is string => typeof color === "string" && isValidHex(color)).map((color) => color.toLowerCase()) : [];
  } catch {
    return [];
  }
}

interface HsvColor {
  hue: number;
  saturation: number;
  value: number;
}

function hexToHsv(hex: string): HsvColor {
  const value = parseInt(hex.slice(1), 16);
  const red = ((value >> 16) & 255) / 255;
  const green = ((value >> 8) & 255) / 255;
  const blue = (value & 255) / 255;
  const maximum = Math.max(red, green, blue);
  const minimum = Math.min(red, green, blue);
  const delta = maximum - minimum;
  let hue = 0;
  if (delta > 0) {
    if (maximum === red) hue = 60 * (((green - blue) / delta) % 6);
    else if (maximum === green) hue = 60 * ((blue - red) / delta + 2);
    else hue = 60 * ((red - green) / delta + 4);
  }
  return { hue: (hue + 360) % 360, saturation: maximum === 0 ? 0 : delta / maximum, value: maximum };
}

function hsvToHex({ hue, saturation, value }: HsvColor): string {
  const chroma = value * saturation;
  const secondary = chroma * (1 - Math.abs((hue / 60) % 2 - 1));
  const offset = value - chroma;
  const sector = Math.floor(hue / 60) % 6;
  const channels = sector === 0 ? [chroma, secondary, 0]
    : sector === 1 ? [secondary, chroma, 0]
      : sector === 2 ? [0, chroma, secondary]
        : sector === 3 ? [0, secondary, chroma]
          : sector === 4 ? [secondary, 0, chroma]
            : [chroma, 0, secondary];
  return `#${channels.map((channel) => Math.round((channel + offset) * 255).toString(16).padStart(2, "0")).join("")}`;
}

function matchesTemplateSearch(name: string, query: string): boolean {
  const normalizedName = name.normalize("NFKC").toLocaleLowerCase();
  const normalizedQuery = query.normalize("NFKC").trim().toLocaleLowerCase();
  if (!normalizedQuery || normalizedName.includes(normalizedQuery)) return true;
  let queryIndex = 0;
  for (const character of normalizedName) {
    if (character === normalizedQuery[queryIndex]) queryIndex += 1;
    if (queryIndex === normalizedQuery.length) return true;
  }
  return false;
}

function DrawingColorPicker(p: {
  color: string;
  opacity: number;
  label: string;
  colorMixed?: boolean;
  opacityMixed?: boolean;
  onColor: (color: string) => void;
  onOpacity: (opacity: number) => void;
  onKeepOpen: () => void;
}): React.ReactElement {
  const opacityPercent = Math.round(p.opacity * 100);
  const [customOpen, setCustomOpen] = useState(false);
  const [customColors, setCustomColors] = useState<string[]>(loadCustomColors);
  const [removedColors, setRemovedColors] = useState<string[]>(loadRemovedColors);
  const [removeMenu, setRemoveMenu] = useState<{ color: string; x: number; y: number } | null>(null);
  const [hexValue, setHexValue] = useState(p.color.toLowerCase());
  const [hsv, setHsv] = useState<HsvColor>(() => hexToHsv(p.color));
  const saturationRef = useRef<HTMLDivElement | null>(null);
  const hueRef = useRef<HTMLDivElement | null>(null);
  const removeMenuRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (!removeMenu) return;
    const closeMenu = (event: PointerEvent): void => {
      if (event.target instanceof Node && !removeMenuRef.current?.contains(event.target)) setRemoveMenu(null);
    };
    const closeOnEscape = (event: KeyboardEvent): void => {
      if (event.key === "Escape") setRemoveMenu(null);
    };
    document.addEventListener("pointerdown", closeMenu);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeMenu);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [removeMenu]);
  const setOpacity = (value: number): void => p.onOpacity(Math.max(0, Math.min(100, value)) / 100);
  const openRemoveMenu = (event: React.MouseEvent<HTMLButtonElement>, color: string): void => {
    event.preventDefault();
    event.stopPropagation();
    const swatch = event.currentTarget.getBoundingClientRect();
    const maxLeft = Math.max(8, window.innerWidth - 149);
    const maxTop = Math.max(8, window.innerHeight - 52);
    setRemoveMenu({
      color,
      x: Math.max(8, Math.min(swatch.left + 3, maxLeft)),
      y: Math.max(8, Math.min(swatch.bottom + 16, maxTop)),
    });
  };
  const swatches = (colors: string[]): React.ReactElement[] => colors.filter((color) => !removedColors.includes(color)).map((color) => <button key={color} type="button" className="drawing-toolbar__swatch" style={{ color }} aria-label={`Set ${p.label.toLowerCase()} ${color}`} aria-pressed={p.color.toLowerCase() === color} title={`${color} · right-click to remove`} onClick={() => p.onColor(color)} onContextMenu={(event) => openRemoveMenu(event, color)} />);
  const persistCustomColors = (colors: string[]): void => {
    setCustomColors(colors);
    setSetting(CUSTOM_COLORS_KEY, JSON.stringify(colors));
  };
  const persistRemovedColors = (colors: string[]): void => {
    setRemovedColors(colors);
    setSetting(REMOVED_COLORS_KEY, JSON.stringify(colors));
  };
  const setHsvColor = (next: HsvColor): void => {
    setHsv(next);
    setHexValue(hsvToHex(next));
  };
  const updateSaturation = (event: React.PointerEvent<HTMLDivElement>): void => {
    const bounds = saturationRef.current?.getBoundingClientRect();
    if (!bounds) return;
    setHsvColor({
      ...hsv,
      saturation: Math.max(0, Math.min(1, (event.clientX - bounds.left) / bounds.width)),
      value: 1 - Math.max(0, Math.min(1, (event.clientY - bounds.top) / bounds.height)),
    });
  };
  const updateHue = (event: React.PointerEvent<HTMLDivElement>): void => {
    const bounds = hueRef.current?.getBoundingClientRect();
    if (!bounds) return;
    setHsvColor({ ...hsv, hue: Math.max(0, Math.min(359, ((event.clientY - bounds.top) / bounds.height) * 360)) });
  };
  const openCustomPicker = (): void => {
    setHexValue(p.color.toLowerCase());
    setHsv(hexToHsv(p.color));
    setCustomOpen(true);
  };
  const applyCustomColor = (): void => {
    if (!isValidHex(hexValue)) return;
    const color = hexValue.toLowerCase();
    if (!CHART_DRAWING_COLORS.some((candidate) => candidate === color) && !customColors.includes(color)) persistCustomColors([...customColors, color]);
    p.onColor(color);
    setCustomOpen(false);
  };
  const removeColor = (color: string): void => {
    persistCustomColors(customColors.filter((customColor) => customColor !== color));
    if (CHART_DRAWING_COLORS.some((candidate) => candidate === color) && !removedColors.includes(color)) persistRemovedColors([...removedColors, color]);
    p.onKeepOpen();
    setRemoveMenu(null);
  };

  if (customOpen) return <div className="drawing-toolbar__custom-picker">
    <div className="drawing-toolbar__custom-picker-form">
      <span className="drawing-toolbar__custom-preview" style={{ backgroundColor: isValidHex(hexValue) ? hexValue : p.color }} />
      <label className="drawing-toolbar__hex-input"><span>#</span><input value={hexValue.replace(/^#/, "")} maxLength={6} aria-label="Hex color" onChange={(event) => {
        const next = event.currentTarget.value.replace(/^#/, "");
        if (!/^[\da-fA-F]{0,6}$/.test(next)) return;
        setHexValue(`#${next}`);
        if (isValidHex(`#${next}`)) setHsv(hexToHsv(`#${next}`));
      }} /></label>
      <button type="button" className="drawing-toolbar__custom-add" disabled={!isValidHex(hexValue)} onClick={applyCustomColor}>Add</button>
    </div>
    <div className="drawing-toolbar__hsv-controls">
      <div
        ref={saturationRef}
        className="drawing-toolbar__saturation"
        role="slider"
        tabIndex={0}
        aria-label="Color saturation and brightness"
        aria-valuetext={`${Math.round(hsv.saturation * 100)}% saturation, ${Math.round(hsv.value * 100)}% brightness`}
        style={{ backgroundImage: `linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, hsl(${hsv.hue} 100% 50%))` }}
        onPointerDown={(event) => { event.currentTarget.setPointerCapture(event.pointerId); updateSaturation(event); }}
        onPointerMove={(event) => { if (event.buttons > 0) updateSaturation(event); }}
        onKeyDown={(event) => {
          const step = event.shiftKey ? 0.1 : 0.02;
          if (event.key === "ArrowLeft") { event.preventDefault(); setHsvColor({ ...hsv, saturation: Math.max(0, hsv.saturation - step) }); }
          if (event.key === "ArrowRight") { event.preventDefault(); setHsvColor({ ...hsv, saturation: Math.min(1, hsv.saturation + step) }); }
          if (event.key === "ArrowUp") { event.preventDefault(); setHsvColor({ ...hsv, value: Math.min(1, hsv.value + step) }); }
          if (event.key === "ArrowDown") { event.preventDefault(); setHsvColor({ ...hsv, value: Math.max(0, hsv.value - step) }); }
        }}
      ><span className="drawing-toolbar__saturation-pointer" style={{ left: `${hsv.saturation * 100}%`, top: `${(1 - hsv.value) * 100}%` }} /></div>
      <div
        ref={hueRef}
        className="drawing-toolbar__hue"
        role="slider"
        tabIndex={0}
        aria-label="Color hue"
        aria-valuemin={0}
        aria-valuemax={359}
        aria-valuenow={Math.round(hsv.hue)}
        onPointerDown={(event) => { event.currentTarget.setPointerCapture(event.pointerId); updateHue(event); }}
        onPointerMove={(event) => { if (event.buttons > 0) updateHue(event); }}
        onKeyDown={(event) => {
          if (event.key === "ArrowUp" || event.key === "ArrowRight") { event.preventDefault(); setHsvColor({ ...hsv, hue: (hsv.hue + 1) % 360 }); }
          if (event.key === "ArrowDown" || event.key === "ArrowLeft") { event.preventDefault(); setHsvColor({ ...hsv, hue: (hsv.hue + 359) % 360 }); }
        }}
      ><span className="drawing-toolbar__hue-pointer" style={{ top: `${(hsv.hue / 360) * 100}%` }} /></div>
    </div>
  </div>;

  return <>
    {p.colorMixed && <div className="drawing-toolbar__mixed-state">Mixed selection</div>}
    <div className="drawing-toolbar__swatch-group" role="group" aria-label="Basic colors">{swatches(CHART_DRAWING_COLORS.slice(0, 20))}</div>
    <div className="drawing-toolbar__swatch-group drawing-toolbar__swatch-group--tints" role="group" aria-label="Color shades">{swatches(CHART_DRAWING_COLORS.slice(20))}</div>
    <div className="drawing-toolbar__custom-color-row" role="group" aria-label="Custom colors">
      {customColors.filter((color) => !removedColors.includes(color)).map((color) => <button key={color} type="button" className="drawing-toolbar__custom-swatch" style={{ color }} aria-label={`Set ${p.label.toLowerCase()} ${color}`} aria-pressed={p.color.toLowerCase() === color} title={`${color} · right-click to remove`} onClick={() => p.onColor(color)} onContextMenu={(event) => openRemoveMenu(event, color)} />)}
      <button type="button" className="drawing-toolbar__custom-color" title="Add custom color" aria-label="Add custom color" onClick={openCustomPicker}><Plus size={14} aria-hidden="true" /></button>
    </div>
    {removeMenu && createPortal(<div ref={removeMenuRef} className="drawing-toolbar__remove-color-menu" role="menu" style={{ left: removeMenu.x, top: removeMenu.y }} onPointerDown={(event) => { event.stopPropagation(); p.onKeepOpen(); }} onClick={(event) => event.stopPropagation()}><button type="button" role="menuitem" onPointerUp={(event) => { if (event.button !== 0) return; event.stopPropagation(); removeColor(removeMenu.color); }} onClick={(event) => { event.stopPropagation(); if (event.detail === 0) removeColor(removeMenu.color); }}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true"><path fill="currentColor" d="M18 7h5v1h-2.01l-1.33 14.64a1.5 1.5 0 0 1-1.5 1.36H9.84a1.5 1.5 0 0 1-1.49-1.36L7.01 8H5V7h5V6c0-1.1.9-2 2-2h4a2 2 0 0 1 2 2v1Zm-6-2a1 1 0 0 0-1 1v1h6V6a1 1 0 0 0-1-1h-4ZM8.02 8l1.32 14.54a.5.5 0 0 0 .5.46h8.33a.5.5 0 0 0 .5-.46L19.99 8H8.02Z" /></svg><span>Remove color</span></button></div>, document.body)}
    <div className="drawing-toolbar__opacity">
      <div className="drawing-toolbar__opacity-title">{p.opacityMixed ? "Opacity (mixed)" : "Opacity"}</div>
      <div className="drawing-toolbar__opacity-controls">
        <input type="range" min="0" max="100" step="1" value={opacityPercent} onChange={(event) => setOpacity(Number(event.currentTarget.value))} aria-label={`${p.label} opacity${p.opacityMixed ? " (mixed)" : ""}`} style={{ backgroundImage: `linear-gradient(90deg, transparent, ${p.color}), repeating-conic-gradient(#444 0% 25%, #666 0% 50%)`, backgroundSize: "100% 100%, 8px 8px" }} />
        <label className="drawing-toolbar__opacity-value"><input type="number" min="0" max="100" step="1" value={opacityPercent} onChange={(event) => setOpacity(Number(event.currentTarget.value))} aria-label={`${p.label} opacity percent${p.opacityMixed ? " (mixed)" : ""}`} /><span>%</span></label>
      </div>
    </div>
  </>;
}

export function DrawingToolbar(p: { drawing: DrawingAny | null; selection?: DrawingAny[]; actions: DrawingToolbarActions; selectionCount?: number; allSelectedLocked?: boolean }): React.ReactElement | null {
  return <DrawingToolbarInner drawing={p.drawing} selection={p.selection ?? (p.drawing ? [p.drawing] : [])} actions={p.actions} selectionCount={p.selectionCount ?? 1} allSelectedLocked={p.allSelectedLocked ?? p.drawing?.locked === true} />;
}

function DrawingToolbarInner(p: { drawing: DrawingAny | null; selection: DrawingAny[]; actions: DrawingToolbarActions; selectionCount: number; allSelectedLocked: boolean }): React.ReactElement | null {
  const multiSelection = p.selectionCount > 1;
  const toolbarRef = useRef<HTMLDivElement | null>(null);
  const dragCleanupRef = useRef<(() => void) | null>(null);
  const actionsRef = useRef(p.actions);
  const drawingRef = useRef(p.drawing);
  const selectionCountRef = useRef(p.selectionCount);
  actionsRef.current = p.actions;
  drawingRef.current = p.drawing;
  selectionCountRef.current = p.selectionCount;
  const [pos, setPos] = useState<{ x: number; y: number }>(() => {
    try {
      const saved = getSetting(POS_KEY);
      if (saved) {
        const q = JSON.parse(saved) as { x?: unknown; y?: unknown };
        if (typeof q.x === "number" && typeof q.y === "number") return { x: q.x, y: q.y };
      }
    } catch { /* ignore */ }
    return { x: 340, y: 180 };
  });
  const [openMenu, setOpenMenu] = useState<MenuKey | null>(null);
  const [moreSubmenu, setMoreSubmenu] = useState<"order" | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsPopupPosition, setSettingsPopupPosition] = useState({ left: 8, top: 8 });
  const [templateName, setTemplateName] = useState("");
  const [templateDialogOpen, setTemplateDialogOpen] = useState(false);
  const [templateSuggestionsOpen, setTemplateSuggestionsOpen] = useState(false);
  const [activeTemplateSuggestion, setActiveTemplateSuggestion] = useState(0);
  const [alertOpen, setAlertOpen] = useState(false);
  const posRef = useRef(pos);
  posRef.current = pos;

  useEffect(() => {
    if (!multiSelection) return;
    setOpenMenu(null);
    setMoreSubmenu(null);
    setSettingsOpen(false);
    setAlertOpen(false);
    setTemplateDialogOpen(false);
  }, [multiSelection]);

  useEffect(() => {
    const clampPosition = (): void => {
      const toolbar = toolbarRef.current;
      if (!toolbar) return;
      const maxX = Math.max(0, window.innerWidth - toolbar.offsetWidth);
      const maxY = Math.max(0, window.innerHeight - toolbar.offsetHeight);
      setPos((current) => {
        const next = {
          x: Math.max(0, Math.min(maxX, current.x)),
          y: Math.max(0, Math.min(maxY, current.y)),
        };
        posRef.current = next;
        return next;
      });
    };
    clampPosition();
    window.addEventListener("resize", clampPosition);
    const closeOnOutsideClick = (event: PointerEvent): void => {
      if (event.target instanceof Element && event.target.closest(".drawing-toolbar__remove-color-menu")) return;
      if (event.target instanceof Node && !toolbarRef.current?.contains(event.target)) setOpenMenu(null);
    };
    const closeOnEscape = (event: KeyboardEvent): void => {
      if (event.key === "Escape") setOpenMenu(null);
    };
    document.addEventListener("pointerdown", closeOnOutsideClick);
    document.addEventListener("keydown", closeOnEscape);
    const handleClipboardShortcut = (event: KeyboardEvent): void => {
      if (!drawingRef.current || selectionCountRef.current !== 1 || event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement || event.target instanceof HTMLSelectElement) return;
      if (!(event.ctrlKey || event.metaKey)) return;
      if (event.key.toLowerCase() === "c") { event.preventDefault(); actionsRef.current.onCopy(); }
      if (event.key.toLowerCase() === "v") { event.preventDefault(); actionsRef.current.onPaste(); }
      if (event.key.toLowerCase() === "d") { event.preventDefault(); actionsRef.current.onClone(); }
    };
    window.addEventListener("keydown", handleClipboardShortcut);
    return () => {
      window.removeEventListener("resize", clampPosition);
      document.removeEventListener("pointerdown", closeOnOutsideClick);
      document.removeEventListener("keydown", closeOnEscape);
      window.removeEventListener("keydown", handleClipboardShortcut);
      dragCleanupRef.current?.();
    };
  }, []);

  useEffect(() => {
    setOpenMenu(null);
    setMoreSubmenu(null);
    setSettingsOpen(false);
    setAlertOpen(false);
    setTemplateDialogOpen(false);
  }, [p.selectionCount]);

  const startDrag = (event: React.PointerEvent<HTMLDivElement>): void => {
    if (event.button !== 0 || dragCleanupRef.current) return;
    event.preventDefault();
    event.stopPropagation();
    const handle = event.currentTarget;
    const pointerId = event.pointerId;
    const startX = event.clientX;
    const startY = event.clientY;
    const origin = posRef.current;

    const updatePosition = (moveEvent: PointerEvent): void => {
      if (moveEvent.pointerId !== pointerId) return;
      const toolbar = toolbarRef.current;
      const maxX = Math.max(0, window.innerWidth - (toolbar?.offsetWidth ?? 100));
      const maxY = Math.max(0, window.innerHeight - (toolbar?.offsetHeight ?? 40));
      const next = {
        x: Math.max(0, Math.min(maxX, origin.x + moveEvent.clientX - startX)),
        y: Math.max(0, Math.min(maxY, origin.y + moveEvent.clientY - startY)),
      };
      posRef.current = next;
      setPos(next);
    };
    const finishDrag = (upEvent?: PointerEvent): void => {
      if (upEvent && upEvent.pointerId !== pointerId) return;
      window.removeEventListener("pointermove", updatePosition, true);
      window.removeEventListener("pointerup", finishDrag, true);
      window.removeEventListener("pointercancel", finishDrag, true);
      window.removeEventListener("blur", cancelDrag);
      try {
        if (handle.hasPointerCapture(pointerId)) handle.releasePointerCapture(pointerId);
      } catch { /* ignore */ }
      setSetting(POS_KEY, JSON.stringify(posRef.current));
      dragCleanupRef.current = null;
    };
    const cancelDrag = (): void => finishDrag();

    window.addEventListener("pointermove", updatePosition, true);
    window.addEventListener("pointerup", finishDrag, true);
    window.addEventListener("pointercancel", finishDrag, true);
    window.addEventListener("blur", cancelDrag);
    try { handle.setPointerCapture(pointerId); } catch { /* ignore */ }
    dragCleanupRef.current = cancelDrag;
  };

  if (!p.drawing) return null;
  const d = p.drawing;
  const a = p.actions;
  const isPathDrawing = d.type === "geometry" && d.tool === "path";
  const templateSuggestions = a.templates.filter((template) => matchesTemplateSearch(template.name, templateName));
  const locked = p.allSelectedLocked;
  const openDrawingSettings = (event: React.MouseEvent<HTMLButtonElement>): void => {
    if (isPathDrawing) {
      const bounds = event.currentTarget.getBoundingClientRect();
      const popupWidth = 340;
      const popupHeight = 420;
      const topBelowButton = bounds.bottom + 8;
      setSettingsPopupPosition({
        left: Math.max(8, Math.min(bounds.left, window.innerWidth - popupWidth - 8)),
        top: topBelowButton + popupHeight <= window.innerHeight - 8
          ? topBelowButton
          : Math.max(8, bounds.top - popupHeight - 8),
      });
    }
    setOpenMenu(null);
    setSettingsOpen(true);
  };
  const lw = d.lineWidth ?? 2;
  const raw = colOf(d);
  const colorMixed = multiSelection && p.selection.some((drawing) => colOf(drawing) !== raw);
  const sw = colorMixed ? "#808080" : /^#[0-9a-fA-F]{6}$/.test(raw) ? raw : "#ffb74d";
  const opacityMixed = multiSelection && p.selection.some((drawing) => (drawing.colorOpacity ?? 1) !== (d.colorOpacity ?? 1));
  const bg = bgOf(d);
  const styles: LineStyle[] = ["solid", "dashed", "dotted"];
  const curStyle: LineStyle = d.style ?? "solid";
  const styleMixed = multiSelection && p.selection.some((drawing) => (drawing.style ?? "solid") !== curStyle);
  const widthMixed = multiSelection && p.selection.some((drawing) => (drawing.lineWidth ?? 2) !== lw);
  const isRectangle = d.type === "rectangle";
  const fillOpacity = isRectangle ? d.fillOpacity : 0;
  const toggleMenu = (menu: MenuKey): void => setOpenMenu((current) => current === menu ? null : menu);
  const chooseColor = (color: string): void => {
    a.onColor(color, d.colorOpacity ?? 1);
    setOpenMenu(null);
  };
  const chooseTextColor = (color: string): void => {
    a.onTextColor(color, d.textOpacity ?? 1);
    setOpenMenu(null);
  };
  const chooseBackground = (color: string): void => {
    a.onBgColor(color);
    setOpenMenu(null);
  };
  const closeTemplateDialog = (): void => {
    setTemplateDialogOpen(false);
    setTemplateName("");
  };
  const saveTemplate = (event: React.FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    const name = templateName.trim();
    if (!name) return;
    void a.onSaveTemplate(name, false).then(closeTemplateDialog);
  };

  return (
    <div
      ref={toolbarRef}
      className={`drawing-toolbar tv-floating-toolbar ui-draggable vora-floating-ui${multiSelection ? " is-multi-selection" : ""}`}
      data-selection-count={p.selectionCount}
      style={{ left: pos.x, top: pos.y, zIndex: 21 }}
      data-name="drawing-toolbar"
      hidden={settingsOpen && !isPathDrawing}
    >
      <div
        className="tv-floating-toolbar__drag js-drag ui-draggable-handle"
        data-vora-drag-handle
        title="Drag"
        style={{ touchAction: "none" }}
        onPointerDown={startDrag}
      >
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 12" width="8" height="12" fill="currentColor" aria-hidden="true"><rect width="2" height="2" rx="1" /><rect width="2" height="2" rx="1" y="5" /><rect width="2" height="2" rx="1" y="10" /><rect width="2" height="2" rx="1" x="6" /><rect width="2" height="2" rx="1" x="6" y="5" /><rect width="2" height="2" rx="1" x="6" y="10" /></svg>
      </div>
      <div className="tv-floating-toolbar__widget-wrapper">
        <div className="tv-floating-toolbar__content js-content" aria-hidden="true" />
        <div className="floating-toolbar-react-widgets">
          {!multiSelection && (
          <div className="drawing-toolbar__item">
            <button type="button" className="floating-toolbar-react-widgets__button" data-name="templates" aria-label="Drawing templates" title="Drawing templates" aria-expanded={openMenu === "templates"} onClick={() => toggleMenu("templates")}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" fill="none" stroke="currentColor" aria-hidden="true"><path strokeLinecap="round" d="M15.5 18.5h6m-3 3v-6" /><rect width="6" height="6" rx="1.5" x="6.5" y="6.5" /><rect width="6" height="6" rx="1.5" x="15.5" y="6.5" /><rect width="6" height="6" rx="1.5" x="6.5" y="15.5" /></svg></button>
            {openMenu === "templates" && <div className="drawing-toolbar__popover drawing-toolbar__templates" role="menu" aria-label="Drawing templates">
              <button type="button" role="menuitem" className="drawing-toolbar__template-action" onClick={() => { setTemplateName(""); setOpenMenu(null); setTemplateDialogOpen(true); }}>Save Drawing Template As…</button>
              <button type="button" role="menuitem" className="drawing-toolbar__template-action" disabled={!a.templates.some((template) => template.id === "default")} onClick={() => { const template = a.templates.find((item) => item.id === "default"); if (template) { a.onApplyTemplate(template); setOpenMenu(null); } }}>Apply Default Drawing Template</button>
              {a.templates.filter((template) => template.id !== "default").map((template) => <div className="drawing-toolbar__template-row" key={template.id}><button type="button" onClick={() => { a.onApplyTemplate(template); setOpenMenu(null); }}>{template.name}</button><button type="button" aria-label={`Delete template ${template.name}`} onClick={() => a.onDeleteTemplate(template.id)}><Trash size={13} /></button></div>)}
            </div>}
          </div>
          )}
          <div className="drawing-toolbar__item">
            <button type="button" className="floating-toolbar-react-widgets__button" data-name="line-tool-color" aria-label={multiSelection ? "Color selected drawings" : "Drawing color"} title={multiSelection ? "Color selected drawings" : "Drawing color"} aria-expanded={openMenu === "color"} onClick={() => toggleMenu("color")}>
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path fill="currentColor" d="M10.62.72a2.47 2.47 0 0 1 3.5 0l1.16 1.16c.96.97.96 2.54 0 3.5l-.58.58-8.9 8.9-1 1-.14.14H0v-4.65l.14-.15 1-1 8.9-8.9.58-.58Zm2.8.7a1.48 1.48 0 0 0-2.1 0l-.23.23 3.26 3.26.23-.23c.58-.58.58-1.52 0-2.1l-1.16-1.16Zm.23 4.2-3.26-3.27-8.2 8.2 3.25 3.27 8.2-8.2Zm-8.9 8.9-3.27-3.26-.5.5V15h3.27l.5-.5Z" /></svg><span className="color-bg"><span className="color-swatch" style={{ background: colorMixed ? "linear-gradient(135deg, #ff5252 0 25%, #ffca28 25% 50%, #42a5f5 50% 75%, #ab47bc 75%)" : withAlpha(sw, d.colorOpacity ?? 1) }} /></span>
            </button>
            {openMenu === "color" && <div className="drawing-toolbar__popover drawing-toolbar__palette" role="group" aria-label="Drawing color"><DrawingColorPicker color={sw} opacity={d.colorOpacity ?? 1} label="Drawing color" colorMixed={colorMixed} opacityMixed={opacityMixed} onColor={chooseColor} onOpacity={a.onColorOpacity} onKeepOpen={() => setOpenMenu("color")} /></div>}
          </div>
          {!multiSelection && (
          <div className="drawing-toolbar__item">
            <button type="button" className="floating-toolbar-react-widgets__button" data-name="background-color" aria-label="Rectangle fill color" title={isRectangle ? "Rectangle fill color" : "Fill color is only available for rectangles"} disabled={!isRectangle} aria-expanded={openMenu === "background"} onClick={() => toggleMenu("background")}>
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" width="20" height="20" fill="none" aria-hidden="true"><path stroke="currentColor" d="M13.5 6.5l-3-3-7 7 7.59 7.59a2 2 0 0 0 2.82 0l4.18-4.18a2 2 0 0 0 0-2.82L13.5 6.5zm0 0v-4a2 2 0 0 0-2-2v0a2 2 0 0 0-2 2v6" /><path fill="currentColor" d="M0 16.5C0 15 2.5 12 2.5 12S5 15 5 16.5 4 19 2.5 19 0 18 0 16.5z" /><circle fill="currentColor" cx="9.5" cy="9.5" r="1.5" /></svg><span className="color-bg"><span className="color-swatch" style={{ backgroundColor: bg }} /></span>
            </button>
            {openMenu === "background" && isRectangle && <div className="drawing-toolbar__popover drawing-toolbar__palette" role="group" aria-label="Rectangle fill"><DrawingColorPicker color={d.fillColor ?? sw} opacity={fillOpacity} label="Fill color" onColor={chooseBackground} onOpacity={a.onBgOpacity} onKeepOpen={() => setOpenMenu("background")} /></div>}
          </div>
          )}
          {!multiSelection && (
          <div className="drawing-toolbar__item">
            <button type="button" className="floating-toolbar-react-widgets__button" data-name="text-color" aria-label="Text color" title="Text color" aria-expanded={openMenu === "text"} onClick={() => toggleMenu("text")}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 13 15" width="13" height="15" fill="none" aria-hidden="true"><path stroke="currentColor" d="M4 14.5h2.5m2.5 0H6.5m0 0V.5m0 0h-5a1 1 0 0 0-1 1V4m6-3.5h5a1 1 0 0 1 1 1V4" /></svg><span className="color-bg"><span className="color-swatch" style={{ backgroundColor: withAlpha(d.textColor ?? sw, d.textOpacity ?? 1) }} /></span></button>
            {openMenu === "text" && <div className="drawing-toolbar__popover drawing-toolbar__palette" role="group" aria-label="Text color"><DrawingColorPicker color={d.textColor ?? sw} opacity={d.textOpacity ?? 1} label="Text color" onColor={chooseTextColor} onOpacity={(opacity) => a.onTextColor(d.textColor ?? sw, opacity)} onKeepOpen={() => setOpenMenu("text")} /></div>}
          </div>
          )}
          <div className="drawing-toolbar__item drawing-toolbar__item--width">
            <button type="button" className="floating-toolbar-react-widgets__button drawing-toolbar__width" data-name="line-tool-width" aria-label={widthMixed ? "Line width (mixed)" : `Line width ${lw} pixels`} title="Line width" aria-expanded={openMenu === "width"} onClick={() => toggleMenu("width")}><span className="drawing-toolbar__width-wrap"><span className="drawing-toolbar__width-icon" role="img" aria-hidden="true"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 1" width="18" height="1"><rect width="18" height="1" fill="currentColor" rx=".5" /></svg></span><span className="drawing-toolbar__width-label">{widthMixed ? "—" : `${lw}px`}</span></span></button>
            {openMenu === "width" && <div className="drawing-toolbar__popover drawing-toolbar__width-menu" role="group" aria-label="Line width">{[1, 2, 3, 4].map((width) => <button key={width} type="button" className="drawing-toolbar__menu-option" aria-pressed={!widthMixed && lw === width} onClick={() => { a.onWidth(width); setOpenMenu(null); }}><svg className="drawing-toolbar__width-option-icon" xmlns="http://www.w3.org/2000/svg" viewBox={`0 0 18 ${width}`} width="18" height={width} aria-hidden="true"><rect width="18" height={width} fill="currentColor" rx={width / 2} /></svg><span className="drawing-toolbar__width-option-label">{width}px</span></button>)}</div>}
          </div>
          <div className="drawing-toolbar__item">
            <button type="button" className="floating-toolbar-react-widgets__button drawing-toolbar__style" data-name="style" aria-label={styleMixed ? "Line style (mixed)" : `Line style ${curStyle}`} title="Line style" aria-expanded={openMenu === "style"} onClick={() => toggleMenu("style")}><svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" aria-hidden="true"><path stroke="currentColor" d="M4 13.5h20" /></svg></button>
            {openMenu === "style" && <div className="drawing-toolbar__popover drawing-toolbar__style-menu" role="group" aria-label="Line style">{styles.map((style) => <button key={style} type="button" className="drawing-toolbar__menu-option" aria-pressed={!styleMixed && curStyle === style} onClick={() => { a.onStyle(style); setOpenMenu(null); }}><span className="drawing-toolbar__style-menu-icon-cell"><svg className="drawing-toolbar__style-menu-icon" xmlns="http://www.w3.org/2000/svg" width="28" height="28" aria-hidden="true">{style === "solid" ? <path stroke="currentColor" d="M4 13.5h20" /> : style === "dashed" ? <path fill="currentColor" d="M4 13h5v1H4v-1zM12 13h5v1h-5v-1zM20 13h5v1h-5v-1z" /> : <path fill="currentColor" d="M3 13h2v2H3v-2Zm5 0h2v2H8v-2Zm7 0h-2v2h2v-2Zm3 0h2v2h-2v-2Zm7 0h-2v2h2v-2Z" />}</svg></span><span className="drawing-toolbar__style-menu-label">{style === "solid" ? "Line" : style === "dashed" ? "Dashed line" : "Dotted line"}</span></button>)}</div>}
          </div>
          {!multiSelection && (
          <div className="drawing-toolbar__item">
            <button type="button" className="floating-toolbar-react-widgets__button" data-name="settings" aria-label="Drawing settings" title="Drawing settings" onClick={openDrawingSettings}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" fill="currentColor" aria-hidden="true"><path fillRule="evenodd" d="M18 14a4 4 0 1 1-8 0 4 4 0 0 1 8 0Zm-1 0a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" /><path fillRule="evenodd" d="M8.5 5h11l5 9-5 9h-11l-5-9 5-9Zm-3.86 9L9.1 6h9.82l4.45 8-4.45 8H9.1l-4.45-8Z" /></svg></button>
          </div>
          )}
          {!multiSelection && (
          <div className="drawing-toolbar__item"><button type="button" className="floating-toolbar-react-widgets__button" data-name="add-alert" aria-label="Add price alert" title="Add price alert" onClick={() => setAlertOpen(true)}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true"><path fill="currentColor" d="m19.54 4.5 3.96 4.32-.74.68-3.96-4.32.74-.68ZM7.46 4.5 3.5 8.82l.74.68L8.2 5.18l-.74-.68ZM19.74 10.33A7.5 7.5 0 0 1 21 14.5v.5h1v-.5a8.5 8.5 0 1 0-8.5 8.5h.5v-1h-.5a7.5 7.5 0 1 1 6.24-11.67Z" /><path fill="currentColor" d="M13 9v5h-3v1h4V9h-1ZM19 20v-4h1v4h4v1h-4v4h-1v-4h-4v-1h4Z" /></svg></button></div>
          )}
          <div className="drawing-toolbar__item"><button type="button" className={`floating-toolbar-react-widgets__button${locked ? " is-active" : ""}`} data-name="lock" aria-label={locked ? `Unlock ${multiSelection ? `${p.selectionCount} drawings` : "drawing"}` : `Lock ${multiSelection ? `${p.selectionCount} drawings` : "drawing"}`} aria-pressed={locked} title={locked ? "Unlock" : "Lock"} onClick={a.onToggleLock}><LockKeyhole size={16} aria-hidden="true" /></button></div>
          {multiSelection && <div className="drawing-toolbar__item"><button type="button" className="floating-toolbar-react-widgets__button" data-name="hide" aria-label={`Hide ${p.selectionCount} drawings`} title="Hide drawings" onClick={a.onHide}><EyeOff size={16} aria-hidden="true" /></button></div>}
          <div className="drawing-toolbar__item"><button type="button" className="floating-toolbar-react-widgets__button" data-name="remove" aria-label={multiSelection ? `Remove ${p.selectionCount} drawings` : "Remove drawing"} title="Remove" onClick={a.onDelete}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true"><path fill="currentColor" d="M18 7h5v1h-2.01l-1.33 14.64a1.5 1.5 0 0 1-1.5 1.36H9.84a1.5 1.5 0 0 1-1.49-1.36L7.01 8H5V7h5V6c0-1.1.9-2 2-2h4a2 2 0 0 1 2 2v1Zm-6-2a1 1 0 0 0-1 1v1h6V6a1 1 0 0 0-1-1h-4ZM8.02 8l1.32 14.54a.5.5 0 0 0 .5.46h8.33a.5.5 0 0 0 .5-.46L19.99 8H8.02Z" /></svg></button></div>
          {!multiSelection && (
          <div className="drawing-toolbar__item">
            <button type="button" className="floating-toolbar-react-widgets__button" data-name="more" aria-label="More drawing actions" title="More drawing actions" aria-expanded={openMenu === "more"} onClick={() => { toggleMenu("more"); setMoreSubmenu(null); }}><svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" fill="none" aria-hidden="true"><path fill="currentColor" fillRule="evenodd" clipRule="evenodd" d="M7.5 13a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM5 14.5a2.5 2.5 0 1 1 5 0 2.5 2.5 0 0 1-5 0zm9.5-1.5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM12 14.5a2.5 2.5 0 1 1 5 0 2.5 2.5 0 0 1-5 0zm9.5-1.5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM19 14.5a2.5 2.5 0 1 1 5 0 2.5 2.5 0 0 1-5 0z" /></svg></button>
            {openMenu === "more" && <div className="drawing-toolbar__popover drawing-toolbar__more-menu" role="menu" aria-label="More drawing actions">
              <button type="button" role="menuitem" onClick={() => setMoreSubmenu((value) => value === "order" ? null : "order")}>Visual order <ChevronRight size={14} /></button>
              {moreSubmenu === "order" && <div className="drawing-toolbar__submenu"><button type="button" role="menuitem" onClick={() => { a.onReorder("front"); setOpenMenu(null); }}>Bring to front</button><button type="button" role="menuitem" onClick={() => { a.onReorder("back"); setOpenMenu(null); }}>Send to back</button></div>}
              <button type="button" role="menuitem" onClick={() => { a.onClone(); setOpenMenu(null); }}>Clone <span>Ctrl+D</span></button>
              <button type="button" role="menuitem" onClick={() => { a.onCopy(); setOpenMenu(null); }}><Copy size={14} />Copy <span>Ctrl+C</span></button>
              <button type="button" role="menuitem" onClick={() => { a.onPaste(); setOpenMenu(null); }}>Paste <span>Ctrl+V</span></button>
              <button type="button" role="menuitem" onClick={() => { a.onHide(); setOpenMenu(null); }}><EyeOff size={14} />Hide</button>
            </div>}
          </div>
          )}
        </div>
      </div>
      {!multiSelection && settingsOpen && <DrawingSettingsDialog drawing={d} onClose={() => setSettingsOpen(false)} onApply={a.onUpdate} templates={a.templates} popupPosition={isPathDrawing ? settingsPopupPosition : undefined} onSaveTemplate={a.onSaveTemplate} onDeleteTemplate={a.onDeleteTemplate} />}
      {!multiSelection && alertOpen && <CreateDrawingAlertDialog symbol={a.symbol} timeframe={a.timeframe} drawing={d} onClose={() => setAlertOpen(false)} onCreate={a.onCreateAlert} />}
      {!multiSelection && templateDialogOpen && <div className="drawing-template-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) closeTemplateDialog(); }}>
        <form className="drawing-template-dialog" role="dialog" aria-modal="true" aria-labelledby="drawing-template-title" onKeyDown={(event) => { if (event.key === "Escape") { event.preventDefault(); closeTemplateDialog(); } }} onSubmit={saveTemplate}>
          <button type="button" className="drawing-template-dialog__close" aria-label="Close dialog" onClick={closeTemplateDialog}><X size={22} /></button>
          <h2 id="drawing-template-title">Save drawing template</h2>
          <label className="drawing-template-dialog__field" htmlFor="drawing-template-name">New template name</label>
          <div className="drawing-template-dialog__picker" onBlur={(event) => { if (!(event.relatedTarget instanceof Node) || !event.currentTarget.contains(event.relatedTarget)) setTemplateSuggestionsOpen(false); }}>
            <input
              id="drawing-template-name"
              className="drawing-template-dialog__input"
              role="combobox"
              aria-autocomplete="list"
              aria-expanded={templateSuggestionsOpen && a.templates.length > 0}
              aria-controls="drawing-template-options"
              aria-activedescendant={templateSuggestionsOpen && templateSuggestions.length > 0 ? `drawing-template-option-${activeTemplateSuggestion}` : undefined}
              autoFocus
              autoComplete="off"
              maxLength={64}
              value={templateName}
              onChange={(event) => { setTemplateName(event.currentTarget.value); setActiveTemplateSuggestion(0); setTemplateSuggestionsOpen(true); }}
              onKeyDown={(event) => {
                if (event.key === "Escape" && templateSuggestionsOpen) { event.preventDefault(); event.stopPropagation(); setTemplateSuggestionsOpen(false); return; }
                if (!templateSuggestionsOpen || templateSuggestions.length === 0) return;
                if (event.key === "ArrowDown") { event.preventDefault(); setActiveTemplateSuggestion((index) => (index + 1) % templateSuggestions.length); }
                if (event.key === "ArrowUp") { event.preventDefault(); setActiveTemplateSuggestion((index) => (index - 1 + templateSuggestions.length) % templateSuggestions.length); }
                if (event.key === "Enter") { event.preventDefault(); setTemplateName(templateSuggestions[activeTemplateSuggestion]?.name ?? templateSuggestions[0].name); setTemplateSuggestionsOpen(false); }
              }}
            />
            {a.templates.length > 0 && <button type="button" className="drawing-template-dialog__picker-toggle" aria-label="Show drawing templates" aria-expanded={templateSuggestionsOpen} onMouseDown={(event) => event.preventDefault()} onClick={() => { setActiveTemplateSuggestion(0); setTemplateSuggestionsOpen((open) => !open); }}><ChevronDown size={18} aria-hidden="true" /></button>}
            {templateSuggestionsOpen && a.templates.length > 0 && <div id="drawing-template-options" className="drawing-template-dialog__options" role="listbox" aria-label="Drawing templates">
              {templateSuggestions.length > 0 ? templateSuggestions.map((template, index) => <button key={template.id} id={`drawing-template-option-${index}`} type="button" role="option" aria-selected={index === activeTemplateSuggestion} className={index === activeTemplateSuggestion ? "is-active" : ""} onMouseDown={(event) => event.preventDefault()} onMouseEnter={() => setActiveTemplateSuggestion(index)} onClick={() => { setTemplateName(template.name); setTemplateSuggestionsOpen(false); }}><span>{template.name}</span><small>{template.drawingType}</small></button>) : <div className="drawing-template-dialog__no-results" role="option" aria-disabled="true">No matching templates</div>}
            </div>}
          </div>
          <footer className="drawing-template-dialog__footer"><button type="button" onClick={closeTemplateDialog}>Cancel</button><button type="submit" className="is-primary" disabled={!templateName.trim()}>Save</button></footer>
        </form>
      </div>}
    </div>
  );
}

export default DrawingToolbarInner;
