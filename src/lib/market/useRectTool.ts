// v7.1.1 — rectangle tool wiring (click-click preview + directional magnet).
import { useEffect } from "react";
import type { IChartApi } from "lightweight-charts";
import { getSetting } from "@/shared/store/useSettingsStore";
import {
  snapPriceToOhlc,
  snapStrong,
  hasCandleAtCoordinate,
  logicalAtCoordinate,
  timestampAtLogical,
  cornerFromDirection,
  applyHandleDrag,
  findHandleAt,
  findRectBorderAt,
  findRectAt,
  cursorForHover,
  hoverForPoint,
  type RectDrag,
  type SeriesLike,
  type DrawingPreview,
  type ScaleLike,
} from "./rectInteraction";
import type { DrawingRect, RectHover } from "./RectangleOverlay";

export interface RectToolCtx {
  container: HTMLDivElement;
  chart: IChartApi;
  series: object;
  drawingsRef: { current: import("./lineDrawings").DrawingAny[] };
  rectToolRef: { current: boolean };
  rectDragRef: { current: RectDrag | null };
  magnetRef: { current: string };
  /** Ctrl-held temporarily forces strong magnet (shared with line tools). */
  ctrlHeldRef?: { current: boolean };
  candlesRef: { current: { ts: number; open: number; high: number; low: number; close: number }[] };
  setDrawings: React.Dispatch<React.SetStateAction<import("./lineDrawings").DrawingAny[]>>;
  /** v7.1.1 — live preview state (anchor → cursor) + snap-marker setter. */
  previewRef: { current: DrawingPreview | null };
  setPreview: (p: DrawingPreview | null) => void;
  setSnap: (s: { t: number; p: number } | null) => void;
  /** v7.2.0 C2/C3 — hover state (rect id + handle) for handle visibility. */
  setHover: (h: RectHover | null) => void;
  /** v7.2.0 Part B — live S-scale registry for auto-coloring finished rects. */
  scalesRef: { current: ScaleLike[] | null };
  /** v7.3.0 Part A — push an explicit pre-mutation snapshot onto the undo stack. */
  pushHistorySnapshot?: (snap: import("./lineDrawings").DrawingAny[]) => void;
  /** v7.3.0 Part B — auto-deactivate the rect tool after finalize (2nd click). */
  onFinalizeRect?: () => void;
}

export function registerRectTool(c: RectToolCtx): () => void {
  const { container, chart, series } = c;
  const srs = series as unknown as SeriesLike;
  // Raw data coords (no magnet).
  const rawAt = (px: number, py: number): { t: number; p: number } | null => {
    const logical = logicalAtCoordinate(chart, px);
    const p = srs.coordinateToPrice(py);
    if (logical == null || p == null) return null;
    const candles = c.candlesRef.current;
    const chartTime = candles.length === 0
      ? chart.timeScale().coordinateToTime(px)
      : null;
    const t = candles.length === 0
      ? (typeof chartTime === "number" ? chartTime : null)
      : timestampAtLogical(logical as number, candles);
    if (t == null) return null;
    return { t, p };
  };
  // Magnet snaps PRICE only, never time; Ctrl temporarily toggles it.
  const snapOn = (): boolean => c.ctrlHeldRef?.current
    ? c.magnetRef.current === "off"
    : c.magnetRef.current !== "off";
  const strongMagnetOn = (): boolean => c.ctrlHeldRef?.current
    ? c.magnetRef.current === "off"
    : c.magnetRef.current === "strong";
  const dataAt = (px: number, py: number): { t: number; p: number } | null => {
    const raw = rawAt(px, py);
    if (!raw) return null;
    if (!snapOn()) return raw;
    if (!hasCandleAtCoordinate(chart, c.candlesRef.current, px)) return raw;
    if (strongMagnetOn()) {
      const snapped = snapPriceToOhlc(raw.t, raw.p, c.candlesRef.current);
      return { t: raw.t, p: snapped.price };
    }
    const snapped = snapStrong(chart, srs, c.candlesRef.current, px, py);
    return snapped ? { t: raw.t, p: snapped.p } : raw;
  };
  const snapMarkerAt = (px: number, point: { t: number; p: number }): { t: number; p: number } | null =>
    snapOn() && hasCandleAtCoordinate(chart, c.candlesRef.current, px) ? point : null;
  // v7.1.2: removed legacy screen-space snap helper (dead code; data-space
  // snap via dataAt/snapPriceToOhlc is the live path).
  const pt = (e: MouseEvent): { px: number; py: number } => {
    const r = container.getBoundingClientRect();
    return { px: e.clientX - r.left, py: e.clientY - r.top };
  };
  const setPan = (en: boolean): void => {
    try {
      chart.applyOptions({ handleScroll: { pressedMouseMove: en } } as unknown as Parameters<typeof chart.applyOptions>[0]);
    } catch { /* ignore */ }
  };
  const onDown = (e: MouseEvent): void => {
    if (e.target instanceof Element && e.target.closest(".vora-floating-ui")) return;
    if (e.button !== 0) return;
    const { px, py } = pt(e);
    const prev = c.previewRef.current;
    // Rect-only view of shared drawings (lines ignored by rect tool).
    const rects = c.drawingsRef.current.filter((q): q is DrawingRect => q.type === "rectangle");
    void rects;
    if (prev && prev.active && c.rectToolRef.current) {
      const dp2 = dataAt(px, py);
      if (!dp2) return;
      const id = `rect-${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
      const lo = Math.min(prev.anchorTs, dp2.t);
      const hi = Math.max(prev.anchorTs, dp2.t);
      const lp = Math.min(prev.anchorPrice, dp2.p);
      const hp = Math.max(prev.anchorPrice, dp2.p);
      // v7.3.0 Part E — hollow border only by default (no fill). Border color
      // still follows the S-scale when matched, else the amber fallback.
      const pct = ((hp - lp) / hp) * 100;
      const scales = c.scalesRef.current;
      let idx: number | null = null;
      if (scales) {
        for (let i = 0; i < scales.length; i++) {
          const s = scales[i];
          if (s.start_pct <= pct && pct < s.end_pct) { idx = i; break; }
        }
      }
      let borderColor: string;
      const fillColor: string | null = null;
      const fillOpacity = 0.0;
      if (idx != null && scales) {
        borderColor = scales[idx].color;
      } else {
        borderColor = "#ffb74d";
      }
      // v7.4.4 — rectangle width comes from Policy Panel pref.
      let rectLW = 2;
      try {
        const v = getSetting("vora.drawings.rect_width");
        if (v != null) {
          const n = Math.round(Number(v));
          if (Number.isFinite(n)) rectLW = Math.max(1, Math.min(10, n));
        }
      } catch { /* ignore */ }
      const seed: DrawingRect = {
        id, type: "rectangle",
        ts_start: lo, ts_end: hi,
        price_low: lp, price_high: hp,
        borderColor, fillColor, fillOpacity, lineWidth: rectLW,
        selected: true,
      };
      // v7.3.0 Part A — snapshot BEFORE the push so Ctrl+Z can recover.
      c.pushHistorySnapshot?.(c.drawingsRef.current);
      c.setDrawings((p) => [...p.map((d) => ({ ...d, selected: false })), seed]);
      c.setPreview(null);
      c.rectDragRef.current = null;
      c.setSnap(snapMarkerAt(px, { t: dp2.t, p: dp2.p }));
      setPan(true);
      // v7.3.0 Part B — auto-deactivate the rect tool after the second click.
      c.onFinalizeRect?.();
      e.preventDefault();
      e.stopPropagation();
      return;
    }
    const hitH = findHandleAt(rects, chart, series, px, py);
    if (hitH) {
      // v0.82.0 — locked drawings cannot be dragged/resized (select only, no drag).
      if ((hitH.d as DrawingRect).locked === true) {
        c.setDrawings((prev) => prev.map((d) => ({ ...d, selected: d.id === hitH.d.id })));
        e.preventDefault();
        e.stopImmediatePropagation();
        return;
      }
      c.rectDragRef.current = { kind: "handle", id: hitH.d.id, handle: hitH.handle, orig: { ...hitH.d } };
      // v7.3.0 Part A — snapshot on drag start (not on every mousemove).
      c.pushHistorySnapshot?.(c.drawingsRef.current);
      c.setDrawings((prev) => prev.map((d) => ({ ...d, selected: d.id === hitH.d.id })));
      setPan(false);
      e.preventDefault();
      e.stopImmediatePropagation();
      return;
    }
    const hitBorder = findRectBorderAt(rects, chart, series, px, py);
    const hitSelectedBody = findRectAt(rects.filter((rect) => rect.selected), chart, series, px, py);
    const hitRect = hitBorder ?? hitSelectedBody;
    if (hitRect) {
      const dpRaw = rawAt(px, py);
      // v0.82.0 — locked: select only, no body drag.
      if (hitRect.locked === true) {
        c.setDrawings((prev) => prev.map((d) => ({ ...d, selected: d.id === hitRect.id })));
        e.preventDefault();
        e.stopImmediatePropagation();
        return;
      }
      // Border selects an unselected shape; its interior moves it only after selection.
      c.pushHistorySnapshot?.(c.drawingsRef.current);
      c.setDrawings((prev) => prev.map((d) => ({ ...d, selected: d.id === hitRect.id })));
      if (dpRaw) c.rectDragRef.current = { kind: "body", id: hitRect.id, grabT: dpRaw.t, grabP: dpRaw.p, orig: { ...hitRect } };
      setPan(false);
      e.preventDefault();
      e.stopImmediatePropagation();
      return;
    }
    const dp = dataAt(px, py);
    if (c.rectToolRef.current && dp) {
      c.setPreview({
        active: true,
        anchorTs: dp.t, anchorPrice: dp.p,
        currentTs: dp.t, currentPrice: dp.p,
        anchorCorner: null,
      });
      c.rectDragRef.current = { kind: "new", startT: dp.t, startP: dp.p, id: "", preview: true, anchorCorner: null };
      c.setSnap(snapMarkerAt(px, { t: dp.t, p: dp.p }));
      setPan(false);
      e.preventDefault();
      e.stopImmediatePropagation();
    } else {
      c.setDrawings((prev) => prev.some((d) => d.selected)
        ? prev.map((d) => ({ ...d, selected: false }))
        : prev);
    }
  };
  const onMove = (e: MouseEvent): void => {
    const { px, py } = pt(e);
    const drag = c.rectDragRef.current;
    const preview = c.previewRef.current;
    const canvas = container.querySelector("canvas");
    const rects = c.drawingsRef.current.filter((q): q is DrawingRect => q.type === "rectangle");
    if (preview && preview.active && drag && drag.kind === "new" && drag.preview) {
      const dpv = dataAt(px, py);
      if (!dpv) return;
      const dx = dpv.t - preview.anchorTs;
      const dy = dpv.p - preview.anchorPrice;
      const anchorCorner = (dx === 0 && dy === 0)
        ? preview.anchorCorner
        : cornerFromDirection(dx, dy);
      c.setPreview({ ...preview, currentTs: dpv.t, currentPrice: dpv.p, anchorCorner });
      drag.anchorCorner = anchorCorner;
      c.setSnap(snapMarkerAt(px, { t: dpv.t, p: dpv.p }));
      return;
    }
    if (!drag) {
      const cur = cursorForHover(rects, chart, series, px, py);
      c.setHover(hoverForPoint(rects, chart, series, px, py));
      // v7.1.3 Part D — only the main drawing canvas gets the tool cursor;
      // price-axis + time-axis canvases (sibling panes) stay `default`.
      const canvases = container.querySelectorAll("canvas");
      if (canvases.length > 0) {
        if (cur) (canvases[0] as HTMLCanvasElement).style.setProperty("cursor", cur, "important");
        else (canvases[0] as HTMLCanvasElement).style.removeProperty("cursor");
        for (let i = 1; i < canvases.length; i++) {
          (canvases[i] as HTMLCanvasElement).style.setProperty("cursor", "default", "important");
        }
      } else if (canvas) {
        if (cur) canvas.style.setProperty("cursor", cur, "important");
        else canvas.style.removeProperty("cursor");
      } else if (cur) container.style.setProperty("cursor", cur, "important");
      else container.style.removeProperty("cursor");
      // Keep the visual crosshair aligned with the selected magnet strength.
      if (c.rectToolRef.current && snapOn() && !c.previewRef.current?.active) {
        const raw = rawAt(px, py);
        if (raw && hasCandleAtCoordinate(chart, c.candlesRef.current, px)) {
          const snapped = strongMagnetOn()
            ? (() => {
                const point = snapPriceToOhlc(raw.t, raw.p, c.candlesRef.current);
                return { t: point.ts, p: point.price };
              })()
            : snapStrong(chart, srs, c.candlesRef.current, px, py);
          if (snapped) {
            c.setSnap({ t: snapped.t, p: snapped.p });
            try {
              chart.setCrosshairPosition(snapped.p, snapped.t as unknown as Parameters<typeof chart.setCrosshairPosition>[1], srs as unknown as Parameters<typeof chart.setCrosshairPosition>[2]);
            } catch { /* ignore — fall back to native crosshair + snap ring */ }
          } else {
            c.setSnap(null);
            try { chart.clearCrosshairPosition(); } catch { /* ignore */ }
          }
        } else {
          c.setSnap(null);
          try { chart.clearCrosshairPosition(); } catch { /* ignore */ }
        }
      } else if (!c.previewRef.current?.active) {
        c.setSnap(null);
        try { chart.clearCrosshairPosition(); } catch { /* ignore */ }
      }
      return;
    }
    const dp = dataAt(px, py);
    if (!dp) return;
    // v7.1.2: legacy `kind:"new"` non-preview path is unreachable (all "new"
    // drags carry preview:true and return via the preview branch above; onUp
    // also early-returns for preview drags). Kept as an explicit throw so
    // the union still narrows for the handle/body branches below.
    if (drag.kind === "new") {
      throw new Error("[rect] unreachable: non-preview new drag");
    } else if (drag.kind === "handle") {
      c.setDrawings((prev) => prev.map((d) => d.type === "rectangle" && d.id === drag.id
        ? { ...applyHandleDrag(drag.orig, drag.handle, dp.t, dp.p), selected: true }
        : { ...d, selected: false }));
      c.setSnap(snapMarkerAt(px, { t: dp.t, p: dp.p }));
    } else {
      // v7.3.1 — body move is delta-based from the RAW grab point (matches onDown).
      const raw = rawAt(px, py) ?? dp;
      const dT = raw.t - drag.grabT;
      const dP = raw.p - drag.grabP;
      const lo = Math.min(drag.orig.ts_start, drag.orig.ts_end) + dT;
      const lp = Math.min(drag.orig.price_low, drag.orig.price_high) + dP;
      const w = Math.abs(drag.orig.ts_end - drag.orig.ts_start);
      const h = Math.abs(drag.orig.price_high - drag.orig.price_low);
      const moved: DrawingRect = { ...drag.orig, ts_start: lo, ts_end: lo + w, price_low: lp, price_high: lp + h, selected: true };
      c.setDrawings((prev) => prev.map((d) => d.id === drag.id ? moved : { ...d, selected: false }));
    }
  };
  const onUp = (): void => {
    const drag = c.rectDragRef.current;
    if (drag && drag.kind === "new" && drag.preview) return;
    if (drag) { c.rectDragRef.current = null; setPan(true); }
  };
  const onKey = (ev: KeyboardEvent): void => {
    if (ev.key === "Escape" && c.previewRef.current) {
      c.setPreview(null);
      c.rectDragRef.current = null;
      c.setSnap(null);
      setPan(true);
    } else if (ev.key === "Escape") {
      // v0.82.0 D3 — Escape deselects (toolbar hides).
      c.setDrawings((prev) => prev.some((d) => d.selected) ? prev.map((d) => ({ ...d, selected: false })) : prev);
    }
  };
  container.addEventListener("mousedown", onDown, true);
  window.addEventListener("mousemove", onMove, true);
  window.addEventListener("mouseup", onUp, true);
  window.addEventListener("keydown", onKey, true);
  return () => {
    container.removeEventListener("mousedown", onDown, true);
    window.removeEventListener("mousemove", onMove, true);
    window.removeEventListener("mouseup", onUp, true);
    window.removeEventListener("keydown", onKey, true);
  };
}

export function useRectTool(c: RectToolCtx | null): void {
  useEffect(() => {
    if (!c) return;
    return registerRectTool(c);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [c?.chart, c?.container]);
}
