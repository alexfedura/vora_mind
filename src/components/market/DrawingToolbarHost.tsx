// v0.82.1 host
import { useEffect, useRef, useState } from "react";
import { DrawingToolbar } from "@/components/market/DrawingToolbar";
import { isValidHex } from "@/lib/market/colorUtils";
import { applyGeometryPathStyle, getGeometrySegmentStyle, type DrawingAny, type DrawingGeometryStylePatch } from "@/lib/market/lineDrawings";
import { HLINE_WIDTH_KEY, HRAY_WIDTH_KEY, LINE_COLOR_KEY, RECT_WIDTH_KEY, TREND_WIDTH_KEY, VLINE_WIDTH_KEY } from "@/lib/market/lineDrawings";
import { pruneNullDrawings } from "@/lib/market/useLineTool";
import { deleteDrawingTemplate, listDrawingTemplates, saveDrawingTemplate } from "@/lib/market/marketApi";
import type { DrawingStyleTemplate } from "@/lib/market/drawingTypes";
import type { ChartDrawingAlert } from "@/lib/market/drawingTypes";
import { saveDrawingAlert } from "@/lib/market/marketApi";
import { setSetting } from "@/shared/store/useSettingsStore";
interface HostProps {
  drawings: DrawingAny[];
  drawingsRef: { current: DrawingAny[] };
  setDrawings: React.Dispatch<React.SetStateAction<DrawingAny[]>>;
  setToast: (m: string | null) => void;
  pushHist: (snap: DrawingAny[]) => void;
  persistPrefs: (p: { color?: string; rectW?: number; hW?: number; vW?: number; rW?: number; tW?: number }) => void;
  symbol: string;
  timeframe: number;
  latestCandleTs: number | null;
  onAlertCreated: (alert: ChartDrawingAlert) => void;
}

function cloneDrawing(drawing: DrawingAny): DrawingAny {
  const id = crypto.randomUUID();
  switch (drawing.type) {
    case "rectangle": {
      const timeOffset = Math.max(60, Math.abs(drawing.ts_end - drawing.ts_start) * 0.08);
      const priceOffset = Math.max(0.0001, Math.abs(drawing.price_high - drawing.price_low) * 0.08);
      return { ...drawing, id, ts_start: drawing.ts_start + timeOffset, ts_end: drawing.ts_end + timeOffset, price_low: drawing.price_low + priceOffset, price_high: drawing.price_high + priceOffset, hidden: false, selected: true };
    }
    case "vline": return { ...drawing, id, ts: drawing.ts + 60, hidden: false, selected: true };
    case "hline": return { ...drawing, id, price: drawing.price + Math.max(0.0001, Math.abs(drawing.price) * 0.001), hidden: false, selected: true };
    case "hray": return { ...drawing, id, ts: drawing.ts + 60, price: drawing.price + Math.max(0.0001, Math.abs(drawing.price) * 0.001), hidden: false, selected: true };
    case "trendline": {
      const priceOffset = Math.max(0.0001, Math.abs(drawing.price2 - drawing.price1) * 0.08);
      return { ...drawing, id, ts1: drawing.ts1 + 60, ts2: drawing.ts2 + 60, price1: drawing.price1 + priceOffset, price2: drawing.price2 + priceOffset, hidden: false, selected: true };
    }
    case "ray": {
      const priceOffset = Math.max(0.0001, Math.abs(drawing.price2 - drawing.price1) * 0.08);
      return { ...drawing, id, ts1: drawing.ts1 + 60, ts2: drawing.ts2 + 60, price1: drawing.price1 + priceOffset, price2: drawing.price2 + priceOffset, hidden: false, selected: true };
    }
    case "info-line": {
      const priceOffset = Math.max(0.0001, Math.abs(drawing.price2 - drawing.price1) * 0.08);
      return { ...drawing, id, ts1: drawing.ts1 + 60, ts2: drawing.ts2 + 60, price1: drawing.price1 + priceOffset, price2: drawing.price2 + priceOffset, hidden: false, selected: true };
    }
    case "extended-line": {
      const priceOffset = Math.max(0.0001, Math.abs(drawing.price2 - drawing.price1) * 0.08);
      return { ...drawing, id, ts1: drawing.ts1 + 60, ts2: drawing.ts2 + 60, price1: drawing.price1 + priceOffset, price2: drawing.price2 + priceOffset, hidden: false, selected: true };
    }
    case "trend-angle": {
      const priceOffset = Math.max(0.0001, Math.abs(drawing.price2 - drawing.price1) * 0.08);
      return { ...drawing, id, ts1: drawing.ts1 + 60, ts2: drawing.ts2 + 60, price1: drawing.price1 + priceOffset, price2: drawing.price2 + priceOffset, hidden: false, selected: true };
    }
    case "crossline": return { ...drawing, id, ts: drawing.ts + 60, price: drawing.price + Math.max(0.0001, Math.abs(drawing.price) * 0.001), hidden: false, selected: true };
    case "icon": return { ...drawing, id, ts: drawing.ts + 60, price: drawing.price + Math.max(0.0001, Math.abs(drawing.price) * 0.001), hidden: false, selected: true };
    case "parallel-channel": {
      const offset = Math.max(60, Math.abs(drawing.ts2 - drawing.ts1) * 0.08);
      const priceOffset = Math.max(0.0001, Math.abs(drawing.price3 - drawing.price1) * 0.08);
      return { ...drawing, id, ts1: drawing.ts1 + offset, ts2: drawing.ts2 + offset, ts3: drawing.ts3 + offset, price1: drawing.price1 + priceOffset, price2: drawing.price2 + priceOffset, price3: drawing.price3 + priceOffset, hidden: false, selected: true };
    }
    case "pitchfork": {
      const offset = Math.max(60, Math.abs(drawing.ts2 - drawing.ts1) * 0.08);
      const priceOffset = Math.max(0.0001, Math.abs(drawing.price2 - drawing.price1) * 0.08);
      return { ...drawing, id, ts1: drawing.ts1 + offset, ts2: drawing.ts2 + offset, ts3: drawing.ts3 + offset, price1: drawing.price1 + priceOffset, price2: drawing.price2 + priceOffset, price3: drawing.price3 + priceOffset, hidden: false, selected: true };
    }
    case "regression-trend": return { ...drawing, id, ts1: drawing.ts1 + 60, ts2: drawing.ts2 + 60, hidden: false, selected: true };
    case "fib-gann": return { ...drawing, id, ts1: drawing.ts1 + 60, ts2: drawing.ts2 + 60, ts3: drawing.ts3 === undefined ? undefined : drawing.ts3 + 60, hidden: false, selected: true };
    case "geometry": return { ...drawing, id, points: drawing.points.map((point) => ({ ts: point.ts + 60, price: point.price + Math.max(0.0001, Math.abs(point.price) * 0.001) })), selectedSegments: undefined, hidden: false, selected: true };
  }
}

function withDrawingStyle(drawing: DrawingAny, patch: DrawingGeometryStylePatch): DrawingAny {
  if (drawing.type === "geometry" && drawing.tool === "path") return applyGeometryPathStyle(drawing, patch);
  return { ...drawing, ...patch };
}

function effectiveToolbarDrawing(drawing: DrawingAny | null): DrawingAny | null {
  if (drawing?.type !== "geometry" || drawing.tool !== "path" || !drawing.selectedSegments?.length) return drawing;
  const style = getGeometrySegmentStyle(drawing, drawing.selectedSegments[0] ?? 0);
  return { ...drawing, ...style };
}

export function DrawingToolbarHost(p: HostProps): React.ReactElement {
  const clipboardRef = useRef<DrawingAny | null>(null);
  const [templates, setTemplates] = useState<DrawingStyleTemplate[]>([]);
  useEffect(() => {
    void listDrawingTemplates().then(setTemplates).catch(() => p.setToast("Could not load drawing templates"));
  }, [p.setToast]);
  const sel = pruneNullDrawings(p.drawings).filter((x) => x.selected);
  const one: DrawingAny | null = sel[0] ?? null;
  const toolbarDrawing = effectiveToolbarDrawing(one);
  const toolbarSelection = sel.map((drawing) => drawing.id === one?.id ? toolbarDrawing ?? drawing : drawing);
  const selectedIds = new Set(sel.map((drawing) => drawing.id));
  const pushU = (): void => { p.pushHist(pruneNullDrawings(p.drawingsRef.current)); };
  const insertClone = (source: DrawingAny): void => {
    pushU();
    const clone = cloneDrawing(source);
    p.setDrawings((prev) => [...pruneNullDrawings(prev).map((drawing) => ({ ...drawing, selected: false })), clone]);
  };
  const wk = (x: DrawingAny): string => x.type === "rectangle" ? RECT_WIDTH_KEY : x.type === "hline" ? HLINE_WIDTH_KEY : x.type === "vline" ? VLINE_WIDTH_KEY : x.type === "hray" ? HRAY_WIDTH_KEY : TREND_WIDTH_KEY;
  const templateSettings = (drawing: DrawingAny): DrawingStyleTemplate["settings"] => ({
    color: drawing.type === "rectangle" ? drawing.borderColor : drawing.color,
    colorOpacity: drawing.colorOpacity ?? 1,
    lineWidth: drawing.lineWidth,
    style: drawing.style ?? "solid",
    fillColor: drawing.type === "rectangle" ? drawing.fillColor : null,
    fillOpacity: drawing.type === "rectangle" ? drawing.fillOpacity : 0,
    text: drawing.text ?? "",
    textColor: drawing.textColor ?? (drawing.type === "rectangle" ? drawing.borderColor : drawing.color),
    textOpacity: drawing.textOpacity ?? 1,
    textSize: drawing.textSize ?? 14,
    textBold: drawing.textBold ?? false,
    textItalic: drawing.textItalic ?? false,
    textAlignHorizontal: drawing.textAlignHorizontal ?? "left",
    textAlignVertical: drawing.textAlignVertical ?? "top",
  });
  return (
    <DrawingToolbar
      drawing={toolbarDrawing}
      selection={toolbarSelection}
      selectionCount={sel.length}
      allSelectedLocked={sel.length > 0 && sel.every((drawing) => drawing.locked === true)}
      actions={{
        onColor: (c, opacity) => {
          if (!one || !isValidHex(c)) return;
          pushU();
          setSetting(LINE_COLOR_KEY, c.toLowerCase());
          p.setDrawings((prev) => pruneNullDrawings(prev).map((z) => {
            if (!selectedIds.has(z.id)) return z;
            const colorOpacity = sel.length === 1 ? opacity : z.colorOpacity;
            if (z.type === "rectangle") return { ...z, borderColor: c, colorOpacity, selected: true };
            return { ...withDrawingStyle(z, { color: c, ...(colorOpacity === undefined ? {} : { colorOpacity }) }), selected: true };
          }));
          p.persistPrefs({ color: c });
        },
        onColorOpacity: (opacity) => {
          if (!one) return;
          pushU();
          p.setDrawings((prev) => pruneNullDrawings(prev).map((z) => selectedIds.has(z.id) ? { ...withDrawingStyle(z, { colorOpacity: Math.max(0, Math.min(1, opacity)) }), selected: true } : z));
        },
        onBgColor: (c) => {
          if (!one || !isValidHex(c)) return;
          pushU();
          const id = one.id;
          p.setDrawings((prev) => pruneNullDrawings(prev).map((z) => {
            if (z.id !== id) return z;
            if (z.type === "rectangle") return { ...z, fillColor: c, fillOpacity: z.fillOpacity ?? 0.2, selected: true };
            return { ...withDrawingStyle(z, { color: c }), selected: true };
          }));
        },
        onBgOpacity: (opacity) => {
          if (!one || one.type !== "rectangle") return;
          pushU();
          const id = one.id;
          p.setDrawings((prev) => pruneNullDrawings(prev).map((z) => {
            if (z.id !== id) return z;
            if (z.type === "rectangle") return { ...z, fillOpacity: Math.max(0, Math.min(1, opacity)), selected: true };
            return z;
          }));
        },
        onTextColor: (c, opacity) => {
          if (!one || !isValidHex(c)) return;
          pushU();
          const id = one.id;
          p.setDrawings((prev) => pruneNullDrawings(prev).map((z) => z.id === id ? { ...z, textColor: c, textOpacity: Math.max(0, Math.min(1, opacity)), selected: true } : z));
        },
        onWidth: (w) => {
          if (!one) return;
          const k = wk(one);
          const cc = Math.max(1, Math.min(10, Math.round(w)));
          pushU();
          setSetting(k, String(cc));
          p.setDrawings((prev) => pruneNullDrawings(prev).map((z) => selectedIds.has(z.id) ? { ...withDrawingStyle(z, { lineWidth: cc }), selected: true } : z));
          if (k === RECT_WIDTH_KEY) p.persistPrefs({ rectW: cc });
          else if (k === HLINE_WIDTH_KEY) p.persistPrefs({ hW: cc });
          else if (k === VLINE_WIDTH_KEY) p.persistPrefs({ vW: cc });
          else if (k === HRAY_WIDTH_KEY) p.persistPrefs({ rW: cc });
          else p.persistPrefs({ tW: cc });
        },
        onStyle: (s) => {
          if (!one) return;
          pushU();
          setSetting("vora.drawings.line_style", s);
          p.setDrawings((prev) => pruneNullDrawings(prev).map((z) => selectedIds.has(z.id) ? { ...withDrawingStyle(z, { style: s }), selected: true } : z));
        },
        onUpdate: (update) => {
          if (!one) return;
          pushU();
          const id = one.id;
          p.setDrawings((prev) => pruneNullDrawings(prev).map((drawing) => {
            if (drawing.id !== id) return drawing;
            const updated = update(drawing);
            if (drawing.type !== "geometry" || drawing.tool !== "path" || !drawing.selectedSegments?.length || updated.type !== "geometry" || updated.tool !== "path") return updated;
            return applyGeometryPathStyle({
              ...updated,
              color: drawing.color,
              colorOpacity: drawing.colorOpacity,
              lineWidth: drawing.lineWidth,
              style: drawing.style,
            }, {
              color: updated.color,
              colorOpacity: updated.colorOpacity,
              lineWidth: updated.lineWidth,
              style: updated.style,
            });
          }));
        },
        onToggleLock: () => {
          if (!one) return;
          pushU();
          const shouldLock = !sel.every((drawing) => drawing.locked === true);
          p.setDrawings((prev) => pruneNullDrawings(prev).map((z) => {
            if (!selectedIds.has(z.id)) return z;
            return { ...z, locked: shouldLock, selected: true };
          }));
        },
        onDelete: () => {
          if (!one) return;
          pushU();
          p.setDrawings((prev) => pruneNullDrawings(prev).filter((z) => !selectedIds.has(z.id)));
        },
        onClone: () => { if (one) insertClone(one); },
        onCopy: () => { clipboardRef.current = one ? { ...one, selected: false } : null; },
        onPaste: () => { if (clipboardRef.current) insertClone(clipboardRef.current); },
        onHide: () => {
          if (!one) return;
          pushU();
          p.setDrawings((prev) => pruneNullDrawings(prev).map((drawing) => selectedIds.has(drawing.id) ? { ...drawing, hidden: true, selected: false } : drawing));
        },
        onReorder: (position) => {
          if (!one) return;
          pushU();
          const id = one.id;
          p.setDrawings((prev) => {
            const drawings = pruneNullDrawings(prev);
            const selected = drawings.find((drawing) => drawing.id === id);
            if (!selected) return drawings;
            const rest = drawings.filter((drawing) => drawing.id !== id);
            const reordered = position === "front" ? [...rest, selected] : [selected, ...rest];
            return reordered.map((drawing) => ({ ...drawing, selected: drawing.id === id }));
          });
        },
        templates,
        onSaveTemplate: async (name, makeDefault, settings) => {
          if (!one) return;
          const template: DrawingStyleTemplate = { id: makeDefault ? "default" : crypto.randomUUID(), name: makeDefault ? "Default" : name.trim(), drawingType: one.type, settings: settings ?? templateSettings(toolbarDrawing ?? one), createdAt: Math.floor(Date.now() / 1000) };
          try {
            await saveDrawingTemplate(template);
            setTemplates(await listDrawingTemplates());
          } catch {
            p.setToast("Could not save drawing template");
          }
        },
        onApplyTemplate: (template) => {
          if (!one) return;
          pushU();
          const settings = template.settings;
          p.setDrawings((prev) => pruneNullDrawings(prev).map((drawing) => {
            if (drawing.id !== one?.id) return drawing;
            const common = { ...drawing, text: settings.text, textColor: settings.textColor, textOpacity: settings.textOpacity, textSize: settings.textSize, textBold: settings.textBold, textItalic: settings.textItalic, textAlignHorizontal: settings.textAlignHorizontal, textAlignVertical: settings.textAlignVertical, selected: true };
            if (common.type === "rectangle") return { ...common, borderColor: settings.color, fillColor: settings.fillColor, fillOpacity: settings.fillOpacity };
            if (common.type === "geometry" && common.tool === "path") {
              return applyGeometryPathStyle(common, { color: settings.color, colorOpacity: settings.colorOpacity, lineWidth: settings.lineWidth, style: settings.style });
            }
            return { ...common, color: settings.color, colorOpacity: settings.colorOpacity, lineWidth: settings.lineWidth, style: settings.style };
          }));
        },
        onDeleteTemplate: (id) => {
          void deleteDrawingTemplate(id).then(() => listDrawingTemplates()).then(setTemplates).catch(() => p.setToast("Could not delete drawing template"));
        },
        symbol: p.symbol,
        timeframe: p.timeframe,
        onCreateAlert: async (alert) => {
          const withBaseline = { ...alert, lastEvaluatedAt: p.latestCandleTs };
          await saveDrawingAlert(withBaseline);
          p.onAlertCreated(withBaseline);
        },
      }}
    />
  );
}
