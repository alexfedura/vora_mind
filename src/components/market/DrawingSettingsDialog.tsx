import { useEffect, useRef, useState } from "react";
import { Bold, ChevronDown, Italic, Trash2 } from "lucide-react";
import type { DrawingAny, LineStyle } from "@/lib/market/lineDrawings";
import type { DrawingAnchor } from "@/lib/market/drawingTypes";
import type { DrawingTextAlign, DrawingTextVerticalAlign } from "@/lib/market/drawingTypes";
import type { DrawingStyleTemplate, DrawingStyleTemplateSettings } from "@/lib/market/drawingTypes";
import { FIB_GANN_TOOL_LABELS, GEOMETRY_TOOL_LABELS } from "@/lib/market/drawingTypes";
import { CHART_DRAWING_COLORS } from "@/shared/canvas/DrawingColorPicker";

type SettingsTab = "Style" | "Text" | "Coordinates";
type PathEndpoint = "none" | "arrow";
type CoordinateKey = "ts_start" | "ts_end" | "price_low" | "price_high" | "ts" | "price" | "ts1" | "ts2" | "price1" | "price2" | "ts3" | "price3";
type Coordinates = Partial<Record<CoordinateKey, number>> & { points?: DrawingAnchor[] };

interface DrawingDraft {
  name: string;
  color: string;
  colorOpacity: number;
  lineWidth: number;
  style: LineStyle;
  pathStart: PathEndpoint;
  pathEnd: PathEndpoint;
  fillColor: string;
  fillOpacity: number;
  text: string;
  textColor: string;
  textOpacity: number;
  textSize: number;
  textBold: boolean;
  textItalic: boolean;
  textAlignHorizontal: DrawingTextAlign;
  textAlignVertical: DrawingTextVerticalAlign;
  coordinates: Coordinates;
}

interface DrawingSettingsDialogProps {
  drawing: DrawingAny;
  onClose: () => void;
  onApply: (update: (drawing: DrawingAny) => DrawingAny) => void;
  templates: DrawingStyleTemplate[];
  popupPosition?: { left: number; top: number };
  onSaveTemplate?: (name: string, makeDefault: boolean, settings?: DrawingStyleTemplateSettings) => Promise<void>;
  onDeleteTemplate?: (id: string) => void;
}

const STYLES: LineStyle[] = ["solid", "dashed", "dotted"];

function coordinatesOf(drawing: DrawingAny): Coordinates {
  switch (drawing.type) {
    case "rectangle": return { ts_start: drawing.ts_start, ts_end: drawing.ts_end, price_low: drawing.price_low, price_high: drawing.price_high };
    case "vline": return { ts: drawing.ts };
    case "hline": return { price: drawing.price };
    case "hray": return { ts: drawing.ts, price: drawing.price };
    case "trendline": return { ts1: drawing.ts1, ts2: drawing.ts2, price1: drawing.price1, price2: drawing.price2 };
    case "ray": return { ts1: drawing.ts1, ts2: drawing.ts2, price1: drawing.price1, price2: drawing.price2 };
    case "info-line": return { ts1: drawing.ts1, ts2: drawing.ts2, price1: drawing.price1, price2: drawing.price2 };
    case "extended-line": return { ts1: drawing.ts1, ts2: drawing.ts2, price1: drawing.price1, price2: drawing.price2 };
    case "trend-angle": return { ts1: drawing.ts1, ts2: drawing.ts2, price1: drawing.price1, price2: drawing.price2 };
    case "crossline": return { ts: drawing.ts, price: drawing.price };
    case "parallel-channel": return { ts1: drawing.ts1, ts2: drawing.ts2, ts3: drawing.ts3, price1: drawing.price1, price2: drawing.price2, price3: drawing.price3 };
    case "pitchfork": return { ts1: drawing.ts1, ts2: drawing.ts2, ts3: drawing.ts3, price1: drawing.price1, price2: drawing.price2, price3: drawing.price3 };
    case "regression-trend": return { ts1: drawing.ts1, ts2: drawing.ts2, price1: drawing.price1, price2: drawing.price2 };
    case "fib-gann": return { ts1: drawing.ts1, ts2: drawing.ts2, ts3: drawing.ts3, price1: drawing.price1, price2: drawing.price2, price3: drawing.price3 };
    case "icon": return { ts: drawing.ts, price: drawing.price };
    case "geometry": return { points: drawing.points.map((point) => ({ ...point })) };
  }
}

function initialDraft(drawing: DrawingAny): DrawingDraft {
  const color = drawing.type === "rectangle" ? drawing.borderColor : drawing.color;
  return {
    name: drawing.name ?? drawingTitle(drawing),
    color,
    colorOpacity: drawing.colorOpacity ?? 1,
    lineWidth: drawing.lineWidth,
    style: drawing.style ?? "solid",
    pathStart: drawing.type === "geometry" && drawing.tool === "path" && drawing.startArrow === true ? "arrow" : "none",
    pathEnd: drawing.type === "geometry" && drawing.tool === "path" && drawing.endArrow === false ? "none" : "arrow",
    fillColor: drawing.type === "rectangle" ? drawing.fillColor ?? color : color,
    fillOpacity: drawing.type === "rectangle" ? drawing.fillOpacity : 0,
    text: drawing.text ?? "",
    textColor: drawing.textColor ?? color,
    textOpacity: drawing.textOpacity ?? 1,
    textSize: drawing.textSize ?? 14,
    textBold: drawing.textBold ?? false,
    textItalic: drawing.textItalic ?? false,
    textAlignHorizontal: drawing.textAlignHorizontal ?? "left",
    textAlignVertical: drawing.textAlignVertical ?? "top",
    coordinates: coordinatesOf(drawing),
  };
}

function applyCoordinates(drawing: DrawingAny, coordinates: Coordinates): DrawingAny {
  switch (drawing.type) {
    case "rectangle":
      return { ...drawing, ts_start: coordinates.ts_start ?? drawing.ts_start, ts_end: coordinates.ts_end ?? drawing.ts_end, price_low: coordinates.price_low ?? drawing.price_low, price_high: coordinates.price_high ?? drawing.price_high };
    case "vline": return { ...drawing, ts: coordinates.ts ?? drawing.ts };
    case "hline": return { ...drawing, price: coordinates.price ?? drawing.price };
    case "hray": return { ...drawing, ts: coordinates.ts ?? drawing.ts, price: coordinates.price ?? drawing.price };
    case "trendline":
      return { ...drawing, ts1: coordinates.ts1 ?? drawing.ts1, ts2: coordinates.ts2 ?? drawing.ts2, price1: coordinates.price1 ?? drawing.price1, price2: coordinates.price2 ?? drawing.price2 };
    case "ray":
      return { ...drawing, ts1: coordinates.ts1 ?? drawing.ts1, ts2: coordinates.ts2 ?? drawing.ts2, price1: coordinates.price1 ?? drawing.price1, price2: coordinates.price2 ?? drawing.price2 };
    case "info-line":
      return { ...drawing, ts1: coordinates.ts1 ?? drawing.ts1, ts2: coordinates.ts2 ?? drawing.ts2, price1: coordinates.price1 ?? drawing.price1, price2: coordinates.price2 ?? drawing.price2 };
    case "extended-line":
      return { ...drawing, ts1: coordinates.ts1 ?? drawing.ts1, ts2: coordinates.ts2 ?? drawing.ts2, price1: coordinates.price1 ?? drawing.price1, price2: coordinates.price2 ?? drawing.price2 };
    case "trend-angle":
      return { ...drawing, ts1: coordinates.ts1 ?? drawing.ts1, ts2: coordinates.ts2 ?? drawing.ts2, price1: coordinates.price1 ?? drawing.price1, price2: coordinates.price2 ?? drawing.price2 };
    case "crossline": return { ...drawing, ts: coordinates.ts ?? drawing.ts, price: coordinates.price ?? drawing.price };
    case "parallel-channel": return { ...drawing, ts1: coordinates.ts1 ?? drawing.ts1, ts2: coordinates.ts2 ?? drawing.ts2, ts3: coordinates.ts3 ?? drawing.ts3, price1: coordinates.price1 ?? drawing.price1, price2: coordinates.price2 ?? drawing.price2, price3: coordinates.price3 ?? drawing.price3 };
    case "pitchfork": return { ...drawing, ts1: coordinates.ts1 ?? drawing.ts1, ts2: coordinates.ts2 ?? drawing.ts2, ts3: coordinates.ts3 ?? drawing.ts3, price1: coordinates.price1 ?? drawing.price1, price2: coordinates.price2 ?? drawing.price2, price3: coordinates.price3 ?? drawing.price3 };
    case "regression-trend": return { ...drawing, ts1: coordinates.ts1 ?? drawing.ts1, ts2: coordinates.ts2 ?? drawing.ts2, price1: coordinates.price1 ?? drawing.price1, price2: coordinates.price2 ?? drawing.price2 };
    case "fib-gann": return { ...drawing, ts1: coordinates.ts1 ?? drawing.ts1, ts2: coordinates.ts2 ?? drawing.ts2, ts3: coordinates.ts3 ?? drawing.ts3, price1: coordinates.price1 ?? drawing.price1, price2: coordinates.price2 ?? drawing.price2, price3: coordinates.price3 ?? drawing.price3 };
    case "icon": return { ...drawing, ts: coordinates.ts ?? drawing.ts, price: coordinates.price ?? drawing.price };
    case "geometry": return { ...drawing, points: coordinates.points ?? drawing.points };
  }
}

function drawingTitle(drawing: DrawingAny): string {
  if (drawing.name) return drawing.name;
  if (drawing.type === "fib-gann") return FIB_GANN_TOOL_LABELS[drawing.tool];
  if (drawing.type === "icon") return drawing.symbol;
  if (drawing.type === "geometry") return GEOMETRY_TOOL_LABELS[drawing.tool];
  return ({ rectangle: "Rectangle", vline: "Vertical line", hline: "Horizontal line", hray: "Horizontal ray", trendline: "Trend line", ray: "Ray", "info-line": "Info line", "extended-line": "Extended line", "trend-angle": "Trend angle", crossline: "Crossline", "parallel-channel": "Parallel channel", pitchfork: "Pitchfork", "regression-trend": "Regression trend" }[drawing.type]);
}

export function DrawingSettingsDialog({ drawing, onClose, onApply, templates, popupPosition, onSaveTemplate, onDeleteTemplate }: DrawingSettingsDialogProps): React.ReactElement {
  const [tab, setTab] = useState<SettingsTab>("Text");
  const [draft, setDraft] = useState<DrawingDraft>(() => initialDraft(drawing));
  const [editingName, setEditingName] = useState(false);
  const [templateMenuOpen, setTemplateMenuOpen] = useState(false);
  const [saveTemplateOpen, setSaveTemplateOpen] = useState(false);
  const [templateName, setTemplateName] = useState("");
  const templateMenuRef = useRef<HTMLDivElement | null>(null);
  const set = <K extends keyof DrawingDraft>(key: K, value: DrawingDraft[K]): void => setDraft((current) => ({ ...current, [key]: value }));
  const setCoordinate = (key: CoordinateKey, value: number | undefined): void => setDraft((current) => ({ ...current, coordinates: { ...current.coordinates, [key]: value } }));
  const setGeometryCoordinate = (index: number, axis: keyof DrawingAnchor, value: number): void => setDraft((current) => ({
    ...current,
    coordinates: {
      ...current.coordinates,
      points: (current.coordinates.points ?? []).map((point, pointIndex) => pointIndex === index ? { ...point, [axis]: value } : point),
    },
  }));
  const displayName = draft.name.trim() || drawingTitle(drawing);
  const isPathDrawing = drawing.type === "geometry" && drawing.tool === "path";
  useEffect(() => {
    if (!templateMenuOpen) return;
    const closeOnOutsidePointer = (event: PointerEvent): void => {
      if (event.target instanceof Node && !templateMenuRef.current?.contains(event.target)) setTemplateMenuOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent): void => { if (event.key === "Escape") setTemplateMenuOpen(false); };
    document.addEventListener("pointerdown", closeOnOutsidePointer);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOnOutsidePointer);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [templateMenuOpen]);
  useEffect(() => {
    if (!isPathDrawing) return;
    const closeOnEscape = (event: KeyboardEvent): void => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      if (templateMenuOpen) setTemplateMenuOpen(false);
      else if (saveTemplateOpen) setSaveTemplateOpen(false);
      else onClose();
    };
    document.addEventListener("keydown", closeOnEscape);
    return () => document.removeEventListener("keydown", closeOnEscape);
  }, [isPathDrawing, onClose, saveTemplateOpen, templateMenuOpen]);
  const coordinateFields: { key: CoordinateKey; label: string }[] = drawing.type === "rectangle"
    ? [{ key: "ts_start", label: "Time start" }, { key: "ts_end", label: "Time end" }, { key: "price_high", label: "Price high" }, { key: "price_low", label: "Price low" }]
    : drawing.type === "vline" ? [{ key: "ts", label: "Time" }]
      : drawing.type === "hline" ? [{ key: "price", label: "Price" }]
        : drawing.type === "hray" ? [{ key: "ts", label: "Time" }, { key: "price", label: "Price" }]
          : drawing.type === "crossline" ? [{ key: "ts", label: "Time" }, { key: "price", label: "Price" }]
            : drawing.type === "parallel-channel" ? [{ key: "ts1", label: "Time 1" }, { key: "price1", label: "Price 1" }, { key: "ts2", label: "Time 2" }, { key: "price2", label: "Price 2" }, { key: "ts3", label: "Time 3" }, { key: "price3", label: "Price 3" }]
              : drawing.type === "fib-gann" && drawing.ts3 !== undefined && drawing.price3 !== undefined
                ? [{ key: "ts1", label: "Time 1" }, { key: "price1", label: "Price 1" }, { key: "ts2", label: "Time 2" }, { key: "price2", label: "Price 2" }, { key: "ts3", label: "Time 3" }, { key: "price3", label: "Price 3" }]
                : [{ key: "ts1", label: "Time 1" }, { key: "price1", label: "Price 1" }, { key: "ts2", label: "Time 2" }, { key: "price2", label: "Price 2" }];

  const apply = (): void => {
    onApply((current) => {
      const styled: DrawingAny = current.type === "rectangle"
        ? { ...current, borderColor: draft.color, colorOpacity: draft.colorOpacity, fillColor: draft.fillColor, fillOpacity: draft.fillOpacity, lineWidth: draft.lineWidth, style: draft.style }
        : {
          ...current,
          color: draft.color,
          colorOpacity: draft.colorOpacity,
          lineWidth: draft.lineWidth,
          style: draft.style,
          ...(current.type === "geometry" && current.tool === "path"
            ? { startArrow: draft.pathStart === "arrow", endArrow: draft.pathEnd === "arrow" }
            : {}),
        };
      const annotated: DrawingAny = {
        ...styled,
        name: displayName,
        text: draft.text,
        textColor: draft.textColor,
        textOpacity: draft.textOpacity,
        textSize: draft.textSize,
        textBold: draft.textBold,
        textItalic: draft.textItalic,
        textAlignHorizontal: draft.textAlignHorizontal,
        textAlignVertical: draft.textAlignVertical,
      };
      return applyCoordinates(annotated, draft.coordinates);
    });
    onClose();
  };
  const applyTemplate = (template: DrawingStyleTemplate): void => {
    const settings = template.settings;
    setDraft((current) => ({ ...current, color: settings.color, colorOpacity: settings.colorOpacity, lineWidth: settings.lineWidth, style: settings.style, fillColor: settings.fillColor ?? current.fillColor, fillOpacity: settings.fillOpacity, text: settings.text, textColor: settings.textColor, textOpacity: settings.textOpacity, textSize: settings.textSize, textBold: settings.textBold, textItalic: settings.textItalic, textAlignHorizontal: settings.textAlignHorizontal, textAlignVertical: settings.textAlignVertical }));
    setTemplateMenuOpen(false);
    setSaveTemplateOpen(false);
  };
  const saveTemplate = async (): Promise<void> => {
    const name = templateName.trim();
    if (!name || !onSaveTemplate) return;
    const settings: DrawingStyleTemplateSettings = {
      color: draft.color,
      colorOpacity: draft.colorOpacity,
      lineWidth: draft.lineWidth,
      style: draft.style,
      fillColor: null,
      fillOpacity: 0,
      text: draft.text,
      textColor: draft.textColor,
      textOpacity: draft.textOpacity,
      textSize: draft.textSize,
      textBold: draft.textBold,
      textItalic: draft.textItalic,
      textAlignHorizontal: draft.textAlignHorizontal,
      textAlignVertical: draft.textAlignVertical,
    };
    await onSaveTemplate(name, false, settings);
    setTemplateName("");
    setSaveTemplateOpen(false);
    setTemplateMenuOpen(false);
  };
  const settingsTab = isPathDrawing ? "Style" : tab;

  return (
    <div className={`drawing-settings-backdrop${isPathDrawing ? " drawing-settings-backdrop--path" : ""}`} onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <section className={`drawing-settings-dialog${isPathDrawing ? " drawing-settings-dialog--path" : ""}`} style={isPathDrawing ? { left: popupPosition?.left ?? 8, top: popupPosition?.top ?? 8 } : undefined} role="dialog" aria-modal={isPathDrawing ? undefined : true} aria-labelledby="drawing-settings-title" data-name="source-properties-editor" data-dialog-name={drawingTitle(drawing)} onKeyDown={(event) => {
        if (event.key !== "Escape") return;
        event.preventDefault();
        if (templateMenuOpen) setTemplateMenuOpen(false);
        else if (saveTemplateOpen) setSaveTemplateOpen(false);
        else onClose();
      }}>
        <header className="drawing-settings-dialog__header">
          <div className="drawing-settings-dialog__title-wrap">
            {editingName ? <input id="drawing-settings-title" className="drawing-settings-dialog__title-input" aria-label="Drawing name" value={draft.name} autoFocus onChange={(event) => set("name", event.currentTarget.value)} onBlur={() => setEditingName(false)} onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); setEditingName(false); } }} /> : <><h2 id="drawing-settings-title">{displayName}</h2><button type="button" className="drawing-settings-dialog__edit-title" aria-label="Rename drawing" onClick={() => setEditingName(true)}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" aria-hidden="true"><path fill="currentColor" d="M16.73 6.56a2.5 2.5 0 0 1 3.54 0l1.17 1.17a2.5 2.5 0 0 1 0 3.54l-.59.58-9 9-1 1-.14.15H6v-4.7l.15-.15 1-1 9-9 .58-.59Zm2.83.7a1.5 1.5 0 0 0-2.12 0l-.23.24 3.29 3.3.23-.24a1.5 1.5 0 0 0 0-2.12l-1.17-1.17Zm.23 4.24L16.5 8.2l-8.3 8.3 3.3 3.3 8.3-8.3Zm-9 9L7.5 17.2l-.5.5V21h3.3l.5-.5Z" /></svg></button></>}
          </div>
          <button type="button" className="drawing-settings-dialog__close" aria-label="Close drawing settings" onClick={onClose}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 14 14" width="14" height="14" aria-hidden="true"><path stroke="currentColor" strokeWidth="1.2" d="m1.5 1.5 11 11m0-11-11 11" vectorEffect="non-scaling-stroke" /></svg></button>
        </header>
        {!isPathDrawing && <div className="drawing-settings-dialog__tabs" role="tablist" aria-label="Drawing settings" aria-orientation="horizontal">
          {(["Style", "Text", "Coordinates"] as const).map((item) => (
            <button key={item} id={`source-properties-editor-tabs-${item.toLowerCase()}`} data-id={`source-properties-editor-tabs-${item.toLowerCase()}`} type="button" role="tab" aria-controls="source-properties-editor-panel" aria-selected={tab === item} onClick={() => setTab(item)}>{item}</button>
          ))}
        </div>}
        {isPathDrawing && <div className="drawing-settings-dialog__path-tabs" role="tablist" aria-label="Drawing settings"><button type="button" role="tab" aria-selected="true">Style</button></div>}
        <div id="source-properties-editor-panel" className={`drawing-settings-dialog__body${isPathDrawing ? " drawing-settings-dialog__path-body" : ""}`} role="tabpanel" aria-labelledby={isPathDrawing ? undefined : `source-properties-editor-tabs-${settingsTab.toLowerCase()}`}>
          {isPathDrawing && (
            <div className="drawing-settings-dialog__path-controls">
              <div className="drawing-settings-dialog__path-color-row">
                <span className="drawing-settings-dialog__path-label">Color</span>
                <div className="drawing-settings-dialog__path-colors" role="group" aria-label="Line color">
                  {CHART_DRAWING_COLORS.map((color) => <button key={color} type="button" aria-label={`Set line color ${color}`} aria-pressed={draft.color.toLowerCase() === color} style={{ backgroundColor: color }} onClick={() => set("color", color)} />)}
                  <label className="drawing-settings-dialog__path-custom-color" title="Custom color"><input type="color" aria-label="Custom line color" value={draft.color} onChange={(event) => set("color", event.currentTarget.value)} /></label>
                </div>
              </div>
              <div className="drawing-settings-dialog__path-opacity">
                <label htmlFor="path-line-opacity">Opacity</label>
                <input id="path-line-opacity" type="range" min="0" max="100" value={Math.round(draft.colorOpacity * 100)} onChange={(event) => set("colorOpacity", Number(event.currentTarget.value) / 100)} />
                <output>{Math.round(draft.colorOpacity * 100)}%</output>
              </div>
              <div className="drawing-settings-dialog__path-option-group" role="group" aria-label="Line thickness">
                <span>Thickness</span>
                {[1, 2, 3, 4].map((width) => <button key={width} type="button" aria-label={`${width} pixel${width === 1 ? "" : "s"}`} aria-pressed={draft.lineWidth === width} onClick={() => set("lineWidth", width)}><span style={{ height: width }} /></button>)}
              </div>
              <div className="drawing-settings-dialog__path-option-group" role="group" aria-label="Line style">
                <span>Line style</span>
                {STYLES.map((style) => <button key={style} type="button" aria-label={`${style} line`} aria-pressed={draft.style === style} onClick={() => set("style", style)}><span className={`drawing-toolbar__line-preview is-${style}`} /></button>)}
              </div>
              <div className="drawing-settings-dialog__path-option-group drawing-settings-dialog__path-endpoints" role="group" aria-label="Path endpoints">
                <span>Arrows</span>
                <button type="button" className="drawing-settings-dialog__path-arrow-button" aria-label="Toggle start arrow" aria-pressed={draft.pathStart === "arrow"} title="Start arrow" onClick={() => set("pathStart", draft.pathStart === "arrow" ? "none" : "arrow")}>
                  <svg viewBox="0 0 28 20" aria-hidden="true"><path d="M8 10h16M8 10l4-4M8 10l4 4" /></svg>
                </button>
                <button type="button" className="drawing-settings-dialog__path-arrow-button" aria-label="Toggle end arrow" aria-pressed={draft.pathEnd === "arrow"} title="End arrow" onClick={() => set("pathEnd", draft.pathEnd === "arrow" ? "none" : "arrow")}>
                  <svg viewBox="0 0 28 20" aria-hidden="true"><path d="M4 10h16m0 0-4-4m4 4-4 4" /></svg>
                </button>
              </div>
            </div>
          )}
          {!isPathDrawing && settingsTab === "Style" && (
            <div className="drawing-settings-dialog__form">
              <label>Line color <span><input type="color" value={draft.color} onChange={(event) => set("color", event.currentTarget.value)} />{Math.round(draft.colorOpacity * 100)}%</span></label>
              <input aria-label="Line opacity" type="range" min="0" max="100" value={Math.round(draft.colorOpacity * 100)} onChange={(event) => set("colorOpacity", Number(event.currentTarget.value) / 100)} />
              <label>Line width <select value={draft.lineWidth} onChange={(event) => set("lineWidth", Number(event.currentTarget.value))}>{[1, 2, 3, 4].map((width) => <option key={width} value={width}>{width}px</option>)}</select></label>
              {drawing.type === "geometry" && drawing.tool === "path" && <>
                <label>Start point
                  <select aria-label="Path start point" value={draft.pathStart} onChange={(event) => {
                    const value = event.currentTarget.value;
                    if (value === "none" || value === "arrow") set("pathStart", value);
                  }}>
                    <option value="none">None</option>
                    <option value="arrow">Arrow</option>
                  </select>
                </label>
                <label>End point
                  <select aria-label="Path end point" value={draft.pathEnd} onChange={(event) => {
                    const value = event.currentTarget.value;
                    if (value === "none" || value === "arrow") set("pathEnd", value);
                  }}>
                    <option value="none">None</option>
                    <option value="arrow">Arrow</option>
                  </select>
                </label>
              </>}
              <div className="drawing-settings-dialog__line-styles" aria-label="Line style">{STYLES.map((style) => <button key={style} type="button" aria-pressed={draft.style === style} onClick={() => set("style", style)}><span className={`drawing-toolbar__line-preview is-${style}`} />{style}</button>)}</div>
              {drawing.type === "rectangle" && <>
                <label>Fill color <input type="color" value={draft.fillColor} onChange={(event) => set("fillColor", event.currentTarget.value)} /></label>
                <label>Fill opacity <span>{Math.round(draft.fillOpacity * 100)}%</span></label>
                <input aria-label="Fill opacity" type="range" min="0" max="100" value={Math.round(draft.fillOpacity * 100)} onChange={(event) => set("fillOpacity", Number(event.currentTarget.value) / 100)} />
              </>}
            </div>
          )}
          {!isPathDrawing && settingsTab === "Text" && (
            <div className="drawing-settings-dialog__form drawing-settings-dialog__text-form">
              <div className="drawing-settings-dialog__text-toolbar">
                <label className="drawing-settings-dialog__text-color" title="Text color"><span className="drawing-settings-dialog__text-color-picker"><span className="drawing-settings-dialog__text-color-swatch" style={{ backgroundColor: draft.textColor, opacity: draft.textOpacity }} /></span><input type="color" aria-label="Text color" value={draft.textColor} onChange={(event) => set("textColor", event.currentTarget.value)} /></label>
                <select aria-label="Font size" value={draft.textSize} onChange={(event) => set("textSize", Number(event.currentTarget.value))}>{[10, 12, 14, 16, 18, 20, 24, 32].map((size) => <option key={size} value={size}>{size}</option>)}</select>
                <button type="button" aria-label="Bold" aria-pressed={draft.textBold} onClick={() => set("textBold", !draft.textBold)}><Bold size={18} /></button>
                <button type="button" aria-label="Italic" aria-pressed={draft.textItalic} onClick={() => set("textItalic", !draft.textItalic)}><Italic size={18} /></button>
              </div>
              <textarea aria-label="Drawing text" placeholder="Add text" value={draft.text} onChange={(event) => set("text", event.currentTarget.value)} />
              <div className="drawing-settings-dialog__alignment-title">Text alignment</div>
              <div className="drawing-settings-dialog__alignment-controls">
                <select aria-label="Vertical text alignment" value={draft.textAlignVertical} onChange={(event) => set("textAlignVertical", event.currentTarget.value as DrawingTextVerticalAlign)}>{[["top", "Top"], ["middle", "Middle"], ["bottom", "Bottom"]].map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select>
                <select aria-label="Horizontal text alignment" value={draft.textAlignHorizontal} onChange={(event) => set("textAlignHorizontal", event.currentTarget.value as DrawingTextAlign)}>{[["left", "Left"], ["center", "Center"], ["right", "Right"]].map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select>
              </div>
            </div>
          )}
          {!isPathDrawing && settingsTab === "Coordinates" && (drawing.type === "geometry" ? <div className="drawing-settings-dialog__form drawing-settings-dialog__coordinates">{(draft.coordinates.points ?? []).map((point, index) => <div key={index} className="drawing-settings-dialog__coordinate-point"><label>Point {index + 1} price<input aria-label={`Point ${index + 1} price`} type="number" step="any" value={point.price} onChange={(event) => setGeometryCoordinate(index, "price", Number(event.currentTarget.value))} /></label><label>Point {index + 1} time<input aria-label={`Point ${index + 1} time`} type="number" step="any" value={point.ts} onChange={(event) => setGeometryCoordinate(index, "ts", Number(event.currentTarget.value))} /></label></div>)}</div> : drawing.type === "rectangle" ? <div className="drawing-settings-dialog__coordinates drawing-settings-dialog__rectangle-coordinates">
            <div className="drawing-settings-dialog__coordinate-label">#1 (price, bar)</div>
            <div className="drawing-settings-dialog__coordinate-point">
              <input aria-label="#1 price" type="number" step="any" value={draft.coordinates.price_low ?? ""} onChange={(event) => setCoordinate("price_low", event.currentTarget.value === "" ? undefined : Number(event.currentTarget.value))} />
              <input aria-label="#1 bar" type="number" step="any" value={draft.coordinates.ts_start ?? ""} onChange={(event) => setCoordinate("ts_start", event.currentTarget.value === "" ? undefined : Number(event.currentTarget.value))} />
            </div>
            <div className="drawing-settings-dialog__coordinate-label">#2 (price, bar)</div>
            <div className="drawing-settings-dialog__coordinate-point">
              <input aria-label="#2 price" type="number" step="any" value={draft.coordinates.price_high ?? ""} onChange={(event) => setCoordinate("price_high", event.currentTarget.value === "" ? undefined : Number(event.currentTarget.value))} />
              <input aria-label="#2 bar" type="number" step="any" value={draft.coordinates.ts_end ?? ""} onChange={(event) => setCoordinate("ts_end", event.currentTarget.value === "" ? undefined : Number(event.currentTarget.value))} />
            </div>
          </div> : <div className="drawing-settings-dialog__form drawing-settings-dialog__coordinates">{coordinateFields.map(({ key, label }) => <label key={key}>{label}<input type="number" step="any" value={draft.coordinates[key] ?? ""} onChange={(event) => setCoordinate(key, event.currentTarget.value === "" ? undefined : Number(event.currentTarget.value))} /></label>)}</div>)}
        </div>
        <footer className="drawing-settings-dialog__footer">
          <div className="drawing-settings-dialog__template-select" ref={templateMenuRef}>
            <button type="button" aria-label="Apply drawing template" aria-haspopup="menu" aria-expanded={templateMenuOpen} onClick={() => setTemplateMenuOpen((open) => !open)}>Template <ChevronDown size={18} aria-hidden="true" /></button>
            {templateMenuOpen && <div className="drawing-settings-dialog__template-menu" role="menu" aria-label="Drawing templates">
              {isPathDrawing && onSaveTemplate && (saveTemplateOpen
                ? <form className="drawing-settings-dialog__template-save" onSubmit={(event) => { event.preventDefault(); void saveTemplate(); }}>
                  <input aria-label="Template name" autoFocus value={templateName} onChange={(event) => setTemplateName(event.currentTarget.value)} placeholder="Template name" />
                  <button type="submit" disabled={!templateName.trim()}>Save</button>
                  <button type="button" onClick={() => setSaveTemplateOpen(false)}>Cancel</button>
                </form>
                : <button type="button" role="menuitem" onClick={() => { setSaveTemplateOpen(true); setTemplateName(""); }}>Save as...</button>)}
              {isPathDrawing && <button type="button" role="menuitem" disabled={!templates.some((template) => template.id === "default")} onClick={() => {
                const template = templates.find((item) => item.id === "default");
                if (template) applyTemplate(template);
              }}>Apply defaults</button>}
              {templates.filter((template) => !(isPathDrawing && template.id === "default")).length > 0 && <div className="drawing-settings-dialog__template-divider" />}
              {templates.filter((template) => !(isPathDrawing && template.id === "default")).map((template) => <div className="drawing-settings-dialog__template-row" key={template.id}>
                <button type="button" role="menuitem" onClick={() => applyTemplate(template)}>{template.name}</button>
                {isPathDrawing && template.id !== "default" && onDeleteTemplate && <button type="button" aria-label={`Delete template ${template.name}`} onClick={() => onDeleteTemplate(template.id)}><Trash2 size={14} /></button>}
              </div>)}
              {templates.length === 0 && !isPathDrawing && <span className="drawing-settings-dialog__template-empty">No templates</span>}
            </div>}
          </div>
          <div className="drawing-settings-dialog__footer-actions">
            <button type="button" onClick={onClose}>Cancel</button>
            <button type="button" className="is-primary" data-name="submit-button" onClick={apply}>Ok</button>
          </div>
        </footer>
      </section>
    </div>
  );
}