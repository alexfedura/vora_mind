// src/lib/market/CandlestickChart.tsx
import React, { useEffect, useMemo, useRef } from "react";
import {
  createChart,
  ColorType,
  CrosshairMode,
  IChartApi,
  IPriceLine,
  ISeriesApi,
  LineStyle,
  CustomSeriesWhitespaceData,
  SeriesPartialOptions,
  Time,
  UTCTimestamp,
} from "lightweight-charts";
import { ZigzagSeries, type FirstExtreme, type ZigzagPoint, type ZigzagSeriesOptions } from "./ZigzagSeries";
import { TV_DARK } from "./palette";

export interface CandleDto {
  ts: number; // unix seconds
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  tick_count: number;
  first_extreme: "high_first" | "low_first" | "same";
}

export type ZigzagSeriesRef = ISeriesApi<
  "Custom",
  Time,
  ZigzagPoint | CustomSeriesWhitespaceData<Time>,
  ZigzagSeriesOptions,
  SeriesPartialOptions<ZigzagSeriesOptions>
>;

const EMPTY_CHART_BAR_COUNT = 500;
const EMPTY_CHART_VISIBLE_BAR_COUNT = 180;

function createDrawingScaffold(
  intervalSeconds: number,
  referencePrice: number,
): ZigzagPoint[] {
  const step = Number.isFinite(intervalSeconds) && intervalSeconds > 0
    ? Math.floor(intervalSeconds)
    : 60;
  const endTimestamp = Math.floor(Date.now() / step) * step;
  const center = Number.isFinite(referencePrice) && referencePrice > 0
    ? referencePrice
    : 100;
  const margin = Math.max(center * 0.02, 0.01);

  return Array.from({ length: EMPTY_CHART_BAR_COUNT }, (_, index) => {
    const price = center;
    return {
      time: (endTimestamp - (EMPTY_CHART_BAR_COUNT - index - 1) * step) as UTCTimestamp,
      open: price,
      high: price + margin,
      low: price - margin,
      close: price,
      firstExtreme: "same",
    };
  });
}

export interface CandlestickChartProps {
  candles: CandleDto[];
  emptyStateIntervalSeconds?: number;
  emptyStateReferencePrice?: number;
  replayCursorTime?: number | null;
  secondsVisible?: boolean;
  onCrosshairMove?: (candle: CandleDto | null) => void;
  onReady?: (chart: IChartApi, series: ZigzagSeriesRef) => void;
  /** v7.0.0: "special" = pivot-line, "candlestick" = OHLC. Non-destructive toggle. */
  displayMode?: "special" | "candlestick";
  /**
   * v0.83.4: when true, skip fitContent() after setData.
   * Used during snapshot restore so the restored viewport is preserved.
   */
  preserveViewport?: boolean;
  /**
   * v0.83.18: when true, crosshair snaps to candle OHLC
   * (CrosshairMode.MagnetOHLC). When false, free-floating
   * crosshair (CrosshairMode.Normal).
   */
  magnetActive?: boolean;
}

export const CandlestickChart: React.FC<CandlestickChartProps> = ({
  candles,
  emptyStateIntervalSeconds = 60,
  emptyStateReferencePrice = 100,
  replayCursorTime = null,
  secondsVisible = false,
  onCrosshairMove,
  onReady,
  displayMode = "special",
  preserveViewport = false,
  magnetActive = false,
}) => {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ZigzagSeriesRef | null>(null);
  const candlesRef = useRef<CandleDto[]>(candles);
  const renderedCandlesRef = useRef<CandleDto[] | null>(null);
  const renderedScaffoldRef = useRef<ZigzagPoint[] | null>(null);
  const renderedCountRef = useRef<number | null>(null);
  const priceLineRef = useRef<IPriceLine | null>(null);
  const magnetActiveRef = useRef(magnetActive);
  const normalizedCandles = useMemo(() => {
    const sorted = [...candles].sort((a, b) => a.ts - b.ts);
    return sorted.filter((candle, index) => index === 0 || candle.ts !== sorted[index - 1]?.ts);
  }, [candles]);
  const drawingScaffold = useMemo(
    () => normalizedCandles.length === 0
      ? createDrawingScaffold(emptyStateIntervalSeconds, emptyStateReferencePrice)
      : null,
    [emptyStateIntervalSeconds, emptyStateReferencePrice, normalizedCandles.length],
  );

  // Stable refs so the mount effect can safely have [] deps
  const onReadyRef = useRef(onReady);
  const onCrosshairMoveRef = useRef(onCrosshairMove);
  const preserveViewportRef = useRef(preserveViewport);

  candlesRef.current = candles;
  onReadyRef.current = onReady;
  onCrosshairMoveRef.current = onCrosshairMove;
  preserveViewportRef.current = preserveViewport;
  magnetActiveRef.current = magnetActive;

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    // StrictMode safety: nuke any orphan DOM from a prior mount
    // (LC v5's chart.remove() may leave canvases behind)
    while (container.firstChild) {
      container.removeChild(container.firstChild);
    }

    // --- ONLY sizing mechanism: autoSize:true ---
    // Do NOT pass width/height. Do NOT add a manual ResizeObserver.
    const chart = createChart(container, {
      autoSize: true,
      layout: {
        attributionLogo: false,
        background: { type: ColorType.Solid, color: TV_DARK.chartBg },
        textColor: TV_DARK.axisText,
        fontSize: 12,
        fontFamily:
          "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, " +
          "'Helvetica Neue', Arial, sans-serif",
      },
      grid: {
        vertLines: { color: TV_DARK.gridMajor, style: 0, visible: true },
        horzLines: { color: TV_DARK.gridMajor, style: 0, visible: true },
      },
      timeScale: {
        borderColor: TV_DARK.axisLine,
        visible: true,
        borderVisible: false,
        timeVisible: true,
        secondsVisible,
        ticksVisible: false,
        rightOffset: 0, // v0.83.8 Part E (Suspect #5): create with 0 so LC's
        // internal setData clamp has no right-margin bias; the visual
        // offset (5) is applied post-ready in handleChartReady.
        // 180-candle default range is paired with 10px TradingView-style bars.
        barSpacing: 10,
        minBarSpacing: 0.5,
        // v0.83.16 — hard upper zoom bound (fixes barSpacing exploding to 700+).
        maxBarSpacing: 60,
        fixLeftEdge: false,
        fixRightEdge: false,
        lockVisibleTimeRangeOnResize: true,
        // v0.83.7 Part A — disable auto-shift: stops LC from
        // auto-shifting the visible range when setData() lands
        // (root cause of timeframe-change / restore drift).
        shiftVisibleRangeOnNewBar: false,
        rightBarStaysOnScroll: false,
      },
      rightPriceScale: {
        borderColor: TV_DARK.axisLine,
        autoScale: false, // CRITICAL for Y zoom — preserved (v0.83.16)
        visible: true,
        borderVisible: false,
        scaleMargins: { top: 0.08, bottom: 0.08 },
        entireTextOnly: true,
      },
      crosshair: {
        // v0.83.23 — TradingView "Cross" mode visual parity.
        mode: magnetActiveRef.current ? CrosshairMode.MagnetOHLC : CrosshairMode.Normal,
        vertLine: {
          color: "#75849A",
          width: 1,
          style: 2, // LineStyle.Dashed
          visible: true,
          labelVisible: true,
          labelBackgroundColor: TV_DARK.crosshairLabelBg,
        },
        horzLine: {
          color: "#75849A",
          width: 1,
          style: 2,
          visible: true,
          labelVisible: true,
          labelBackgroundColor: TV_DARK.crosshairLabelBg,
        },
      },
      handleScroll: {
        mouseWheel: false,
        pressedMouseMove: true,
        horzTouchDrag: true,
        vertTouchDrag: false,
      },
      handleScale: {
        mouseWheel: false,
        pinch: true,
        axisPressedMouseMove: {
          time: true,
          price: true,
        },
        axisDoubleClickReset: {
          time: true,
          price: true,
        },
      },
      kineticScroll: {
        mouse: false,
        touch: true,
      },
    });

    const series = chart.addCustomSeries(new ZigzagSeries(), {
      lineColor: "#2962ff",
      lineWidth: 2,
      dotColor: "#ef5350",
      dotRadius: 3,
      displayMode: "special",
      scaffoldOnly: false,
    });
    chartRef.current = chart;
    seriesRef.current = series;

    if (onReadyRef.current) {
      onReadyRef.current(chart, series);
    }

    if (onCrosshairMoveRef.current) {
      chart.subscribeCrosshairMove((param) => {
        const cb = onCrosshairMoveRef.current;
        if (!cb) return;
        if (!param || !param.time || param.point === undefined) {
          cb(null);
          return;
        }
        const timeVal =
          typeof param.time === "number" ? param.time : null;
        if (timeVal !== null) {
          const match =
            candlesRef.current.find((c) => c.ts === timeVal) || null;
          cb(match);
        } else {
          cb(null);
        }
      });
    }

    return () => {
      try {
        chart.remove();
      } catch {
        // LC v5 may throw if already disposed; still clean DOM below
      }
      // StrictMode safety: double-tap to guarantee clean state
      while (container.firstChild) {
        container.removeChild(container.firstChild);
      }
      chartRef.current = null;
      seriesRef.current = null;
      priceLineRef.current = null;
    };
  }, []);

  useEffect(() => {
    chartRef.current?.timeScale().applyOptions({ secondsVisible });
  }, [secondsVisible]);

  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) return;
    chart.applyOptions({
      crosshair: {
        mode: magnetActive
          ? CrosshairMode.MagnetOHLC
          : CrosshairMode.Normal,
      },
    });
  }, [magnetActive]);

  // v7.0.0: display-mode switch — non-destructive, no data reload, no viewport reset
  useEffect(() => {
    const series = seriesRef.current;
    if (!series) return;
    series.applyOptions({ displayMode } as Record<string, unknown>);
  }, [displayMode]);

  // Update all candles on source changes and append only newly revealed replay bars.
  useEffect(() => {
    const series = seriesRef.current;
    const chart = chartRef.current;
    if (!series || !chart) return;

    const sourceChanged = renderedCandlesRef.current !== candles
      || renderedScaffoldRef.current !== drawingScaffold;
    const isScaffold = normalizedCandles.length === 0;
    const seriesData: ZigzagPoint[] = drawingScaffold ?? normalizedCandles.map((candle) => ({
      time: candle.ts as UTCTimestamp,
      open: candle.open,
      high: candle.high,
      low: candle.low,
      close: candle.close,
      firstExtreme: (candle.first_extreme ?? "same") as FirstExtreme,
    }));
    const previousCandles = renderedCandlesRef.current;
    const previousFirstTimestamp = previousCandles?.[0]?.ts;
    const previousFirstIndex = previousFirstTimestamp === undefined
      ? -1
      : normalizedCandles.findIndex((candle) => candle.ts === previousFirstTimestamp);
    const isPrepended = previousFirstIndex > 0
      && previousCandles !== null
      && previousCandles.every((candle, index) => normalizedCandles[previousFirstIndex + index]?.ts === candle.ts);
    const prependedCount = isPrepended ? previousFirstIndex : 0;
    const cursorIndex = replayCursorTime === null
      ? normalizedCandles.length
      : normalizedCandles.findIndex((candle) => candle.ts >= replayCursorTime);
    const nextCount = cursorIndex < 0 ? normalizedCandles.length : cursorIndex;
    const previousCount = renderedCountRef.current;
    const preserveReplayViewport = !sourceChanged
      && replayCursorTime !== null
      && previousCount !== null
      && previousCount !== nextCount;
    const visibleRange = preserveReplayViewport || prependedCount > 0
      ? chart.timeScale().getVisibleLogicalRange()
      : null;

    series.applyOptions({ scaffoldOnly: isScaffold });
    chart.priceScale("right").applyOptions({ autoScale: isScaffold });

    if (sourceChanged) {
      series.setData(isScaffold ? seriesData : seriesData.slice(0, nextCount));
      renderedCandlesRef.current = candles;
      renderedScaffoldRef.current = drawingScaffold;
      renderedCountRef.current = nextCount;
      if (isScaffold) {
        chart.timeScale().setVisibleLogicalRange({
          from: Math.max(0, EMPTY_CHART_BAR_COUNT - EMPTY_CHART_VISIBLE_BAR_COUNT),
          to: EMPTY_CHART_BAR_COUNT - 1,
        });
      } else if (nextCount > 0 && !preserveViewportRef.current && prependedCount === 0) {
        chart.timeScale().fitContent();
      }
    } else if (renderedCountRef.current !== nextCount) {
      const renderedCount = renderedCountRef.current ?? 0;
      if (nextCount > renderedCount) {
        for (const candle of seriesData.slice(renderedCount, nextCount)) {
          series.update(candle);
        }
      } else {
        series.setData(seriesData.slice(0, nextCount));
      }
      renderedCountRef.current = nextCount;
    }
    if (visibleRange) {
      chart.timeScale().setVisibleLogicalRange(prependedCount > 0
        ? { from: visibleRange.from + prependedCount, to: visibleRange.to + prependedCount }
        : visibleRange);
    }
    if (priceLineRef.current) series.removePriceLine(priceLineRef.current);
    const currentCandle = normalizedCandles[nextCount - 1];
    priceLineRef.current = currentCandle ? series.createPriceLine({
      price: currentCandle.close,
      color: currentCandle.close >= currentCandle.open ? TV_DARK.upColor : TV_DARK.downColor,
      lineWidth: 1,
      lineStyle: LineStyle.Dashed,
      lineVisible: false,
      axisLabelVisible: true,
      title: "",
    }) : null;
  }, [candles, drawingScaffold, normalizedCandles, replayCursorTime]);

  return (
    <div
      ref={containerRef}
      className="w-full h-full relative overflow-hidden bg-[#0f0f0f]"
    />
  );
};
