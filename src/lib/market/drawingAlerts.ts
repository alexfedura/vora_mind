import type { ChartDrawingAlert } from "./drawingTypes";
import type { DrawingAny } from "./lineDrawings";
import type { CandleDto } from "./marketApi";
import { pitchforkMedianOrigin } from "./pitchforkGeometry";
import { fibGannAlertLevels, fibGannTimeLevels } from "./fibGannGeometry";

function overlaps(candle: CandleDto, low: number, high: number): boolean {
  return candle.high >= low && candle.low <= high;
}

function linePriceAt(drawing: Extract<DrawingAny, { type: "trendline" | "ray" | "info-line" | "extended-line" | "trend-angle" | "parallel-channel" | "pitchfork" }>, ts: number): number {
  const duration = drawing.ts2 - drawing.ts1;
  if (duration === 0) return drawing.price2;
  const ratio = (ts - drawing.ts1) / duration;
  return drawing.price1 + ratio * (drawing.price2 - drawing.price1);
}

export function drawingAlertTriggered(
  alert: ChartDrawingAlert,
  drawing: DrawingAny,
  previous: CandleDto,
  current: CandleDto,
  candleHistory: readonly CandleDto[] = [previous, current],
): boolean {
  if (drawing.hidden) return false;
  if (drawing.type === "rectangle") {
    const low = Math.min(drawing.price_low, drawing.price_high);
    const high = Math.max(drawing.price_low, drawing.price_high);
    const currentInside = overlaps(current, low, high);
    if (alert.condition === "entering_drawing") return currentInside && !overlaps(previous, low, high);
    return (previous.close < low && current.high >= low) || (previous.close > high && current.low <= high);
  }
  if (drawing.type === "fib-gann") {
    const levels = fibGannAlertLevels(drawing, current.ts);
    const crossedPrice = levels.some((level) => previous.close !== level && ((previous.close < level && current.high >= level) || (previous.close > level && current.low <= level)));
    const crossedTime = fibGannTimeLevels(drawing).some((level) => (previous.ts <= level && current.ts >= level) || (previous.ts >= level && current.ts <= level));
    if (alert.condition === "entering_drawing") {
      const anchors = [drawing.price1, drawing.price2, ...(drawing.price3 === undefined ? [] : [drawing.price3])];
      const low = Math.min(...anchors);
      const high = Math.max(...anchors);
      return overlaps(current, low, high) && !overlaps(previous, low, high);
    }
    return crossedPrice || crossedTime;
  }
  if (drawing.type === "geometry") {
    if (drawing.tool === "anchored-vwap") {
      const anchor = drawing.points[0];
      if (!anchor) return false;
      const vwapAt = (timestamp: number): number | null => {
        let totalVolume = 0;
        let weightedPrice = 0;
        for (const candle of candleHistory) {
          if (candle.ts < anchor.ts || candle.ts > timestamp || candle.volume <= 0) continue;
          totalVolume += candle.volume;
          weightedPrice += ((candle.high + candle.low + candle.close) / 3) * candle.volume;
        }
        return totalVolume > 0 ? weightedPrice / totalVolume : null;
      };
      const previousVwap = vwapAt(previous.ts);
      const currentVwap = vwapAt(current.ts);
      if (previousVwap === null || currentVwap === null) return false;
      return (previous.close < previousVwap && current.high >= currentVwap) || (previous.close > previousVwap && current.low <= currentVwap);
    }
    const prices = drawing.points.map((point) => point.price);
    if (prices.length === 0) return false;
    const low = Math.min(...prices);
    const high = Math.max(...prices);
    const currentInside = overlaps(current, low, high);
    if (alert.condition === "entering_drawing") return currentInside && !overlaps(previous, low, high);
    const crosses = (level: number): boolean => previous.close !== level && ((previous.close < level && current.high >= level) || (previous.close > level && current.low <= level));
    const crossedPrice = prices.some(crosses);
    if (drawing.tool === "position-forecast" && drawing.points.length >= 2) {
      const first = drawing.points[0];
      const second = drawing.points[1];
      if (first && second) {
        const duration = second.ts - first.ts;
        const ratio = duration === 0 ? 1 : (current.ts - first.ts) / duration;
        return crossedPrice || (ratio >= 0 && crosses(first.price + ratio * (second.price - first.price)));
      }
    }
    return crossedPrice || drawing.points.some((point) => previous.ts < point.ts && current.ts >= point.ts);
  }
  let price: number | null = null;
  if (drawing.type === "parallel-channel") {
    const offset = drawing.price3 - linePriceAt(drawing, drawing.ts3);
    const firstBoundary = linePriceAt(drawing, current.ts);
    const secondBoundary = firstBoundary + offset;
    const crosses = (level: number): boolean => previous.close !== level && ((previous.close < level && current.high >= level) || (previous.close > level && current.low <= level));
    return crosses(firstBoundary) || crosses(secondBoundary);
  }
  if (drawing.type === "pitchfork") {
    const origin = pitchforkMedianOrigin({ ts: drawing.ts1, price: drawing.price1 }, { ts: drawing.ts2, price: drawing.price2 }, drawing.variant);
    const middleTime = (drawing.ts2 + drawing.ts3) / 2;
    const middlePrice = (drawing.price2 + drawing.price3) / 2;
    const duration = middleTime - origin.ts;
    const slope = duration === 0 ? 0 : (middlePrice - origin.price) / duration;
    const starts = [
      origin,
      { ts: drawing.ts2, price: drawing.price2 },
      { ts: drawing.ts3, price: drawing.price3 },
    ];
    const crosses = (level: number): boolean => previous.close !== level && ((previous.close < level && current.high >= level) || (previous.close > level && current.low <= level));
    return starts.some((start) => {
      const directed = duration >= 0 ? current.ts >= start.ts : current.ts <= start.ts;
      return directed && crosses(start.price + (current.ts - start.ts) * slope);
    });
  }
  if (drawing.type === "hline") price = drawing.price;
  else if (drawing.type === "hray" && current.ts >= drawing.ts) price = drawing.price;
  else if ((drawing.type === "trendline" || drawing.type === "info-line" || drawing.type === "trend-angle") && current.ts >= Math.min(drawing.ts1, drawing.ts2) && current.ts <= Math.max(drawing.ts1, drawing.ts2)) price = linePriceAt(drawing, current.ts);
  else if (drawing.type === "ray" && (drawing.ts2 >= drawing.ts1 ? current.ts >= drawing.ts1 : current.ts <= drawing.ts1)) price = linePriceAt(drawing, current.ts);
  else if (drawing.type === "extended-line") price = linePriceAt(drawing, current.ts);
  if (price !== null) return previous.close !== price && ((previous.close < price && current.high >= price) || (previous.close > price && current.low <= price));

  if (drawing.type === "crossline") {
    const crossedPrice = previous.close !== drawing.price && ((previous.close < drawing.price && current.high >= drawing.price) || (previous.close > drawing.price && current.low <= drawing.price));
    return crossedPrice || (previous.ts < drawing.ts && current.ts >= drawing.ts);
  }
  if (drawing.type === "vline") return previous.ts < drawing.ts && current.ts >= drawing.ts;
  return false;
}

export function isDrawingAlertExpired(alert: ChartDrawingAlert, nowSeconds: number): boolean {
  return alert.expirationAt !== null && alert.expirationAt <= nowSeconds;
}

export function hasDrawingAlertExpired(alert: ChartDrawingAlert, nowSeconds: number): boolean {
  return isDrawingAlertExpired(alert, nowSeconds);
}
