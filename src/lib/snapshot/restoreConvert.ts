import type { SnapshotDrawing, VoraSnapshot } from "./types";
import type { DrawingAny } from "@/lib/market/lineDrawings";
import { isChartIntervalKey } from "@/lib/market/chartIntervals";

export function snapshotToDrawings(list: SnapshotDrawing[]): DrawingAny[] {
  const out: DrawingAny[] = [];
  for (const d of list) {
    if (d.type === "geometry") {
      out.push({
        id: d.id,
        type: "geometry",
        tool: d.tool,
        name: d.name,
        points: d.points.map((point) => ({ ts: Math.floor(point.ts / 1000), price: point.price })),
        sourceBars: d.sourceBars?.map((bar) => ({ ...bar, ts: Math.floor(bar.ts / 1000) })),
        color: d.color,
        colorOpacity: d.colorOpacity,
        lineWidth: d.lineWidth,
        style: d.style,
        locked: d.locked,
        hidden: d.hidden,
        visibleOnIntervals: d.visibleOnIntervals ? [...d.visibleOnIntervals] : undefined,
        text: d.text,
        textColor: d.textColor,
        textOpacity: d.textOpacity,
        textSize: d.textSize,
        textBold: d.textBold,
        textItalic: d.textItalic,
        textAlignHorizontal: d.textAlignHorizontal,
        textAlignVertical: d.textAlignVertical,
        selected: false,
      });
    } else if (d.type === "icon") {
      const point = d.points[0];
      if (!point) continue;
      out.push({ id: d.id, type: "icon", category: d.category, symbol: d.symbol, size: d.size, name: d.name, text: d.text, textColor: d.textColor, textOpacity: d.textOpacity, textSize: d.textSize, textBold: d.textBold, textItalic: d.textItalic, textAlignHorizontal: d.textAlignHorizontal, textAlignVertical: d.textAlignVertical, ts: Math.floor(point.ts / 1000), price: point.price, color: d.color, lineWidth: d.lineWidth, selected: false, locked: d.locked, style: d.style });
    }
    else if (d.type === "vline") out.push({ id: d.id, type: "vline", ts: Math.floor((d.points[0]?.ts ?? 0) / 1000), color: d.color, lineWidth: d.lineWidth, selected: false, locked: d.locked, style: d.style });
    else if (d.type === "hline") out.push({ id: d.id, type: "hline", price: d.points[0]?.price ?? 0, color: d.color, lineWidth: d.lineWidth, selected: false, locked: d.locked, style: d.style });
    else if (d.type === "hray") out.push({ id: d.id, type: "hray", ts: Math.floor((d.points[0]?.ts ?? 0) / 1000), price: d.points[0]?.price ?? 0, color: d.color, lineWidth: d.lineWidth, selected: false, locked: d.locked, style: d.style });
    else if (d.type === "trendline") out.push({ id: d.id, type: "trendline", ts1: Math.floor((d.points[0]?.ts ?? 0) / 1000), price1: d.points[0]?.price ?? 0, ts2: Math.floor((d.points[1]?.ts ?? 0) / 1000), price2: d.points[1]?.price ?? 0, color: d.color, lineWidth: d.lineWidth, selected: false, locked: d.locked, style: d.style });
    else if (d.type === "ray" || d.type === "info-line" || d.type === "extended-line" || d.type === "trend-angle" || d.type === "regression-trend") {
      out.push({
        id: d.id,
        type: d.type,
        ts1: Math.floor((d.points[0]?.ts ?? 0) / 1000),
        price1: d.points[0]?.price ?? 0,
        ts2: Math.floor((d.points[1]?.ts ?? 0) / 1000),
        price2: d.points[1]?.price ?? 0,
        color: d.color,
        lineWidth: d.lineWidth,
        selected: false,
        locked: d.locked,
        style: d.style,
      });
    } else if (d.type === "crossline") {
      out.push({ id: d.id, type: "crossline", ts: Math.floor((d.points[0]?.ts ?? 0) / 1000), price: d.points[0]?.price ?? 0, color: d.color, lineWidth: d.lineWidth, selected: false, locked: d.locked, style: d.style });
    } else if (d.type === "parallel-channel") {
      out.push({ id: d.id, type: "parallel-channel", ts1: Math.floor((d.points[0]?.ts ?? 0) / 1000), price1: d.points[0]?.price ?? 0, ts2: Math.floor((d.points[1]?.ts ?? 0) / 1000), price2: d.points[1]?.price ?? 0, ts3: Math.floor((d.points[2]?.ts ?? 0) / 1000), price3: d.points[2]?.price ?? 0, color: d.color, lineWidth: d.lineWidth, selected: false, locked: d.locked, style: d.style });
    } else if (d.type === "pitchfork") {
      out.push({ id: d.id, type: "pitchfork", ts1: Math.floor((d.points[0]?.ts ?? 0) / 1000), price1: d.points[0]?.price ?? 0, ts2: Math.floor((d.points[1]?.ts ?? 0) / 1000), price2: d.points[1]?.price ?? 0, ts3: Math.floor((d.points[2]?.ts ?? 0) / 1000), price3: d.points[2]?.price ?? 0, color: d.color, lineWidth: d.lineWidth, selected: false, locked: d.locked, style: d.style });
    } else if (d.type === "rect") {
      const tsA = Math.floor((d.points[0]?.ts ?? 0) / 1000);
      const tsB = Math.floor((d.points[1]?.ts ?? 0) / 1000);
      const pA = d.points[0]?.price ?? 0;
      const pB = d.points[1]?.price ?? 0;
      out.push({ id: d.id, type: "rectangle", ts_start: Math.min(tsA, tsB), ts_end: Math.max(tsA, tsB), price_low: Math.min(pA, pB), price_high: Math.max(pA, pB), borderColor: d.color, fillColor: d.bgColor ?? null, fillOpacity: 0.12, lineWidth: d.lineWidth, selected: false, locked: d.locked, style: d.style });
    } else if (d.type === "fib-gann") {
      const first = d.points[0];
      const second = d.points[1];
      if (!first || !second) continue;
      const third = d.points[2];
      out.push({
        id: d.id,
        type: "fib-gann",
        tool: d.tool,
        ts1: Math.floor(first.ts / 1000),
        price1: first.price,
        ts2: Math.floor(second.ts / 1000),
        price2: second.price,
        ...(third ? { ts3: Math.floor(third.ts / 1000), price3: third.price } : {}),
        color: d.color,
        lineWidth: d.lineWidth,
        selected: false,
        locked: d.locked,
        style: d.style,
      });
    }
  }
  return out;
}

export function parseSnapshotInterval(interval: string): string | null {
  return isChartIntervalKey(interval) ? interval : null;
}

export type SnapshotRegistry = VoraSnapshot["policy"]["registry"];
