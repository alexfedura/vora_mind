import React, { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { Link, useSearchParams } from "react-router-dom";
import type { IChartApi, MouseEventParams, UTCTimestamp, Time } from "lightweight-charts";
import {
  mktFetchHistoricalDuka,
  detectBoxes,
  cancelRequest,
  getScaleRegistry,
  setScaleRegistry as persistScaleRegistry,
  listPivots,
  type Scale,
  type ScaleRegistry,
  type PivotDto,
  CandleDto,
  BoxDto,
  ChainDto,
} from "@/lib/market/marketApi";
import { CandlestickChart, type ZigzagSeriesRef } from "@/lib/market/CandlestickChart";
import { useMarqueeSelection, type SelectionBox } from "@/lib/market/useMarqueeSelection";
import { BoxOverlay } from "@/lib/market/BoxOverlay";
import { MeasureOverlay } from "@/lib/market/MeasureOverlay";
import { GradedRectangleOverlay } from "@/lib/market/GradedRectangleOverlay";
import { GradedRectangleControls } from "@/components/market/GradedRectangleControls";
import { useGradedRectStore } from "@/lib/market/gradedRectStore";
import { useGradedRectTool, type GradedRectContextMenu } from "@/lib/market/useGradedRectTool";
import {
  RectangleOverlay,
  type DrawingRect,
  type TrendPreview,
} from "@/lib/market/RectangleOverlay";
import { isGradedRectangle, type DrawingAny, type GradedRectangle, type LineToolId } from "@/lib/market/lineDrawings";
import { HLINE_WIDTH_KEY, HRAY_WIDTH_KEY, LINE_COLOR_KEY, RECT_WIDTH_KEY, TREND_WIDTH_KEY, VLINE_WIDTH_KEY, isAnnotationToolId, migrateDrawingAny, readLineColor, readLineWidth } from "@/lib/market/lineDrawings";
import { isDrawingVisible, type DrawingIconSelection } from "@/lib/market/drawingTypes";
import type { ChartDrawingAlert } from "@/lib/market/drawingTypes";
import { deleteDrawingAlert, deleteDrawingAlertsForDrawings, listDrawingAlerts, updateDrawingAlert } from "@/lib/market/marketApi";
import { drawingAlertTriggered, hasDrawingAlertExpired } from "@/lib/market/drawingAlerts";
import { LineOverlay, type LineHover } from "@/lib/market/LineOverlay";
import { registerLineTool, pruneNullDrawings } from "@/lib/market/useLineTool";
import {
  type RectDrag,
  type DrawingPreview,
  hasCandleAtCoordinate,
  snapPriceToOhlc,
  snapStrong,
} from "@/lib/market/rectInteraction";
import { useRectTool } from "@/lib/market/useRectTool";
import {
  applyTemplateAtr,
  applyZoomPreservingCenter,
  calculateATR,
  DEFAULT_ZOOM,
  DEFAULT_VISIBLE_CANDLE_COUNT,
  ZoomState,
  offsetToCandleCount,
  offsetToYMilli,
} from "@/lib/market/chartTemplate";
import {
  AlertCircle,
  Camera,
  House,
  Menu,
  BookOpen,
  RotateCcw,
  Sliders,
  Video,
} from "lucide-react";
import {
  PolicyPanel,
  DEFAULT_POLICY,
  M_ROLE_DEFAULTS,
  type PolicyDefaults,
} from "@/components/market/PolicyPanel";
import { FloatingToolbar } from "@/components/market/FloatingToolbar";
import { DrawingToolbarHost } from "@/components/market/DrawingToolbarHost";
import { SnapshotCaptureModal } from "@/components/market/SnapshotCaptureModal";
import { NarrativePlayback } from "@/components/market/NarrativePlayback";
import { SymbolSelector } from "@/components/market/SymbolSelector";
import { snapshotToDrawings, parseSnapshotInterval } from "@/lib/snapshot/restoreConvert";
import { snapshotToCapturedViewport } from "@/lib/snapshot/restoreViewport";
import type { ActiveScale, NarrativeEvent, NarrativeMode, SnapshotPolicy, SnapshotViewport, VoraSnapshot } from "@/lib/snapshot/types";
import { deriveCompactSteps } from "@/lib/snapshot/narrative";
import { deserialize } from "@/lib/snapshot/serialize";
import { captureViewport } from "@/lib/snapshot/capture";
import { captureChartState, captureDrawings, capturePolicy, captureTheme } from "@/lib/snapshot/captureState";
import { buildSnapshot } from "@/lib/snapshot/builder";
import { saveSnapshot, loadSnapshot } from "@/lib/snapshot/storage";
import { useDebounced } from "@/lib/hooks/useDebounced";
import { migrateDrawingRect, type RectHover } from "@/lib/market/RectangleOverlay";
import { isFibGannTool } from "@/lib/market/fibGannGeometry";
import { isGeometryToolId } from "@/lib/market/geometryToolGeometry";
import { isValidHex } from "@/lib/market/colorUtils";
import { history as undoHistory } from "@/lib/market/history";
import { getDefaultRoute, setDefaultRoute, type DefaultRoute } from "@/lib/settings";
import { acquireInteractionLock, useInteractionLocked } from "@/lib/ui/interactionLock";
import { ScrollToLatestButton } from "@/components/market/ScrollToLatestButton";
import { useMarketSymbol } from "@/lib/market/useMarketSymbol";
import { ChartRightDock, type ChartRightPanelId } from "@/components/market/ChartRightDock";
import { ChartDrawingToolbar } from "@/components/market/ChartDrawingToolbar";
import { ChartIntervalSelector } from "@/components/market/ChartIntervalSelector";
import { ChartReplayFooter, ChartReplayToolbar, type ReplayRange, type ReplayUpdateInterval } from "@/components/market/ChartReplayToolbar";
import { intervalWindowSeconds, isChartIntervalKey, nearestMScaleStep } from "@/lib/market/chartIntervals";
import { deleteSetting, getSetting, setSetting } from "@/shared/store/useSettingsStore";
import "./ChartPage.css";
import {
  captureViewport as captureTfViewport,
  applyViewportVerified as applyTfViewportVerified,
  type CapturedViewport,
} from "@/lib/chart/viewportPreserve";

// v0.83.1: heavy per-timeframe scale/box recalculation is DISABLED.
// The RBR/DBD scan (detectBoxes IPC → Rust scan_rbr/scan_dbd) no longer
// runs on timeframe change. Box coloring (registry palette keyed by
// geometry-derived scale index) stays active.
// Global kill switch for scale/box recalculation.
// Set to true ONLY if the user later decides to re-enable.
export const ENABLE_SCALE_RECALC = false;

// v6.4.11: canonical TF grid (seconds) — user_step snapping source.
// Module-level constant; indexed by debouncedM.
const TF_SECONDS_TS = [1, 5, 25, 125, 625, 3125, 15625, 78125, 390625, 1953125, 9765625];
const OFFLINE_REFERENCE_PRICE_PREFIX = "vora.chart.offline-reference.";
const LOCAL_PROXY_DEFAULT_MIGRATION_KEY = "vora.policy.local_proxy_default_v1";

function readOfflineReferencePrice(symbol: string): number | undefined {
  try {
    const value = Number(getSetting(`${OFFLINE_REFERENCE_PRICE_PREFIX}${symbol}`));
    return Number.isFinite(value) && value > 0 ? value : undefined;
  } catch {
    return undefined;
  }
}

function isLineToolId(id: string): id is LineToolId {
  return id === "icon" || isGeometryToolId(id) || isAnnotationToolId(id) || id === "vertline" || id === "horzline" || id === "horzray" || id === "trendline" || id === "ray" || id === "info-line" || id === "extended-line" || id === "trend-angle" || id === "crossline" || id === "parallel-channel" || id === "pitchfork" || id === "schiff-pitchfork" || id === "modified-schiff-pitchfork" || isFibGannTool(id);
}

/**
 * v0.83.20 — Read real axis dimensions from the LC API instead
 * of hardcoded pixel values. Falls back to conservative defaults
 * if the API is not ready.
 */
function readAxisDims(chart: IChartApi | null): {
  rightW: number;
  bottomH: number;
} {
  try {
    const ps = chart?.priceScale("right");
    const ts = chart?.timeScale();
    const rightW =
      ps && typeof ps.width === "function"
        ? Number(ps.width()) || 60
        : 60;
    const bottomH =
      ts && typeof ts.height === "function"
        ? Number(ts.height()) || 28
        : 28;
    return { rightW, bottomH };
  } catch {
    return { rightW: 60, bottomH: 28 };
  }
}

/**
 * v0.83.20 — Classify the pointer position relative to the
 * chart's plot vs its chrome (price axis / time axis).
 * Adds a small margin so the boundary is stable (no flicker).
 */
function computeOverChrome(
  chart: IChartApi | null,
  wrapperEl: HTMLElement | null,
  clientX: number,
  clientY: number,
): boolean {
  if (!chart || !wrapperEl) return false;
  const el = chart.chartElement?.();
  if (!el) return false;
  const rect = el.getBoundingClientRect();
  if (rect.width === 0 || rect.height === 0) return false;

  const x = clientX - rect.left;
  const y = clientY - rect.top;
  const { rightW, bottomH } = readAxisDims(chart);

  // 2 px safety margin outside the axis so that the boundary
  // is not jittery between adjacent pixels.
  const MARGIN = 2;

  const overRightAxis = x >= rect.width - rightW + MARGIN;
  const overBottomAxis = y >= rect.height - bottomH + MARGIN;
  return overRightAxis || overBottomAxis;
}

function nearestReplayIndex(candles: CandleDto[], timestamp: number): number | null {
  if (candles.length === 0) return null;
  let low = 0;
  let high = candles.length;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    if ((candles[middle]?.ts ?? Number.POSITIVE_INFINITY) < timestamp) low = middle + 1;
    else high = middle;
  }
  if (low === 0) return 0;
  if (low >= candles.length) return candles.length - 1;
  const before = candles[low - 1];
  const after = candles[low];
  if (!before || !after) return null;
  return timestamp - before.ts <= after.ts - timestamp ? low - 1 : low;
}

function replayIndexAtOrAfter(candles: CandleDto[], timestamp: number): number | null {
  if (candles.length === 0) return null;
  let low = 0;
  let high = candles.length;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    if ((candles[middle]?.ts ?? Number.POSITIVE_INFINITY) < timestamp) low = middle + 1;
    else high = middle;
  }
  return Math.min(low, candles.length - 1);
}

function replayTimestampToCoordinate(chart: IChartApi, candles: CandleDto[], timestamp: number): number | null {
  const timeScale = chart.timeScale();
  const exactCoordinate = timeScale.timeToCoordinate(timestamp as UTCTimestamp);
  if (exactCoordinate !== null) return exactCoordinate as unknown as number;

  const afterIndex = replayIndexAtOrAfter(candles, timestamp);
  if (afterIndex === null) return null;
  const after = candles[afterIndex];
  if (!after) return null;
  const afterCoordinate = timeScale.timeToCoordinate(after.ts as UTCTimestamp);
  if (afterCoordinate === null) return null;
  if (afterIndex === 0) return afterCoordinate as unknown as number;

  const before = candles[afterIndex - 1];
  if (!before || after.ts <= before.ts) return afterCoordinate as unknown as number;
  const beforeCoordinate = timeScale.timeToCoordinate(before.ts as UTCTimestamp);
  if (beforeCoordinate === null) return null;
  const fraction = Math.max(0, Math.min(1, (timestamp - before.ts) / (after.ts - before.ts)));
  return (beforeCoordinate + (afterCoordinate - beforeCoordinate) * fraction) as unknown as number;
}

function mergeSnapshotScaleRegistry(
  registry: ScaleRegistry,
  snapshotScales: SnapshotPolicy["registry"],
): ScaleRegistry {
  const scalesById = new Map(snapshotScales.map((scale) => [scale.id, scale]));
  return {
    ...registry,
    scales: registry.scales.map((scale, index) => {
      const digits = String(scale.id).replace(/[^0-9]/g, "");
      const parsedId = digits.length > 0 ? parseInt(digits, 10) : NaN;
      const id = Number.isFinite(parsedId) ? parsedId : index;
      const savedScale = scalesById.get(id);
      return savedScale
        ? { ...scale, color: savedScale.color, start_pct: savedScale.min, end_pct: savedScale.max }
        : scale;
    }),
  };
}

function advanceReplayByBar(candles: CandleDto[], index: number): number {
  return Math.min(Math.max(0, candles.length - 1), index + 1);
}

function advanceReplayBySecond(candles: CandleDto[], index: number): number {
  const current = candles[index];
  if (!current || index >= candles.length - 1) return Math.max(0, candles.length - 1);
  const nextOffset = nearestReplayIndex(candles.slice(index + 1), current.ts + 1);
  return nextOffset === null ? candles.length - 1 : index + 1 + nextOffset;
}

function readSavedReplaySpeed(): number {
  try {
    const savedSpeed = Number(getSetting("vora.chart.replay.speed"));
    return [0.25, 0.5, 1, 2, 3, 5, 10].includes(savedSpeed) ? savedSpeed : 1;
  } catch {
    return 1;
  }
}

const SCALE_PALETTE = [
  '#4caf50','#8bc34a','#cddc39','#ffeb3b','#ffc107',
  '#ff9800','#ff5722','#f44336','#e91e63','#9c27b0',
  '#673ab7','#3f51b5','#2196f3','#03a9f4','#00bcd4',
  '#009688','#795548','#607d8b','#9e9e9e','#bdbdbd',
];

// v6.4.4: re-classify a box by registry ranges → scale index (or null).
function reclassifyScale(
  basePct: number,
  scales: { start_pct: number; end_pct: number }[],
): number | null {
  for (let i = 0; i < scales.length; i++) {
    const s = scales[i];
    if (s.start_pct <= basePct && basePct < s.end_pct) return i;
  }
  return null;
}

function RestoreOverlay({ visible, name }: { visible: boolean; name: string }): React.ReactElement | null {
  // v0.83.2.1 — hold the global interaction lock while visible so chart
  // pointer/keyboard handlers stay disabled behind the overlay.
  useEffect(() => {
    if (!visible) return;
    const release = acquireInteractionLock();
    return release;
  }, [visible]);
  if (!visible) return null;
  return (
    <div
      className="vora-floating-ui"
      style={{
        position: "fixed", inset: 0, zIndex: 9999,
        background: "rgba(0,0,0,0.65)",
        display: "flex", alignItems: "center", justifyContent: "center",
        pointerEvents: "auto",
      }}
    >
      <div
        style={{
          background: "#181818", border: "1px solid #2a2a2a", borderRadius: 4,
          padding: "16px 24px", textAlign: "center",
        }}
      >
        <div style={{ color: "#d6dbe2", fontSize: 13, fontWeight: 600 }}>Restoring snapshot...</div>
        {name ? <div style={{ color: "#e8ad55", fontSize: 12, marginTop: 4 }}>{name}</div> : null}
      </div>
    </div>
  );
}

export const ChartPage: React.FC<{ isVisible?: boolean }> = ({ isVisible = true }) => {
  const [searchParams, setSearchParams] = useSearchParams();
  const snapshotIdFromUrl = searchParams.get("snapshot");
  const playNarrativeFromUrl = searchParams.get("playNarrative") === "1";
  const [currentM, setCurrentM] = useState<number>(() => {
    try {
      const raw = getSetting("vora.policy.M");
      if (raw) {
        const p = JSON.parse(raw) as { user_step?: unknown; user_m?: unknown };
        const v = p.user_step ?? p.user_m ?? DEFAULT_POLICY.user_step;
        if (typeof v === "number" && Number.isFinite(v)) {
          return Math.max(0, Math.min(10, Math.round(v)));
        }
      }
    } catch { /* ignore */ }
    return 3;
  });
  const [activeInterval, setActiveInterval] = useState<string>(() => {
    try {
      const saved = getSetting("vora.chart.interval");
      if (saved && isChartIntervalKey(saved)) return saved;
    } catch { /* ignore */ }
    return "1m";
  });
  const [replayOpen, setReplayOpen] = useState(false);
  const [replaySelecting, setReplaySelecting] = useState(false);
  const [replayStartIndex, setReplayStartIndex] = useState<number | null>(null);
  const [replayEndIndex, setReplayEndIndex] = useState<number | null>(null);
  const [replayPlaying, setReplayPlaying] = useState(false);
  const replayTimestampOverridesRef = useRef<{ start: number | null; end: number | null } | null>(null);
  const replayTimeframeRestoreRef = useRef<{
    startTimestamp: number | null;
    endTimestamp: number | null;
    wasPlaying: boolean;
  } | null>(null);
  const resumeReplayAfterNavigationRef = useRef(false);
  const [replaySpeed, setReplaySpeed] = useState(readSavedReplaySpeed);
  const [replayUpdateInterval, setReplayUpdateInterval] = useState<ReplayUpdateInterval>("chart");
  const [replayAutoUpdateInterval, setReplayAutoUpdateInterval] = useState(true);
  const [replayRange, setReplayRange] = useState<ReplayRange>("All");
  const [replayCursorX, setReplayCursorX] = useState<number | null>(null);
  const [replayHoverTimestamp, setReplayHoverTimestamp] = useState<number | null>(null);
  const replayCursorRef = useRef<number | null>(replayEndIndex);
  replayCursorRef.current = replayEndIndex;
  const replaySkipViewportFollowRef = useRef(false);
  useEffect(() => {
    if (!isVisible) {
      resumeReplayAfterNavigationRef.current = replayPlaying;
      if (replayPlaying) setReplayPlaying(false);
      return;
    }
    if (resumeReplayAfterNavigationRef.current) {
      resumeReplayAfterNavigationRef.current = false;
      setReplayPlaying(true);
    }
  }, [isVisible]);

  useEffect(() => {
    try { setSetting("vora.chart.replay.speed", String(replaySpeed)); } catch { /* ignore */ }
  }, [replaySpeed]);

  const selectInterval = (interval: string, preserveReplay = true): void => {
    if (!isChartIntervalKey(interval)) return;
    if (preserveReplay && replayOpen) {
      const timestamps = replayTimestampOverridesRef.current;
      replayTimeframeRestoreRef.current = {
        startTimestamp: timestamps?.start ?? (replayStartIndex === null ? null : candles[replayStartIndex]?.ts ?? null),
        endTimestamp: timestamps?.end ?? (replayEndIndex === null ? null : candles[replayEndIndex]?.ts ?? null),
        wasPlaying: replayPlaying,
      };
    } else {
      replayTimestampOverridesRef.current = null;
      replayTimeframeRestoreRef.current = null;
      setReplayOpen(false);
      setReplaySelecting(false);
      setReplayStartIndex(null);
      setReplayEndIndex(null);
    }
    setReplayPlaying(false);
    setActiveInterval(interval);
    setCurrentM(nearestMScaleStep(interval));
    try { setSetting("vora.chart.interval", interval); } catch { /* ignore */ }
  };
  // MKT-6b-6.3.0d: debounce slider drags → detectBoxes fires once after 300ms
  const debouncedM = useDebounced(currentM, 300);
  const debouncedInterval = useDebounced(activeInterval, 300);
  const [candles, setCandles] = useState<CandleDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [isFetchingOlder, setIsFetchingOlder] = useState(false);
  const isFetchingOlderRef = useRef(false);
  const blockedOlderBoundaryRef = useRef<string | null>(null);
  const olderRetryAfterRef = useRef<{ boundary: string; timestamp: number } | null>(null);
  const suppressTemplateForPrependRef = useRef(false);
  const [error, setError] = useState<string | null>(null);
  const [rightPanel, setRightPanel] = useState<ChartRightPanelId | null>(null);
  const [drawingAlerts, setDrawingAlerts] = useState<ChartDrawingAlert[]>([]);

  useEffect(() => {
    if (!rightPanel) return;
    const closeOnEscape = (event: KeyboardEvent): void => {
      if (event.key === "Escape") setRightPanel(null);
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [rightPanel]);

  const [zoomState, setZoomState] = useState<ZoomState>(DEFAULT_ZOOM);
  const [activeScale, setActiveScale] = useState<ActiveScale>(() => ({
    x: DEFAULT_ZOOM.x_off_milli,
    y: DEFAULT_ZOOM.y_off_milli,
    s: currentM,
  }));
  const activeScaleRef = useRef(activeScale);
  activeScaleRef.current = activeScale;
  useEffect(() => {
    setActiveScale((scale) => scale.s === currentM ? scale : { ...scale, s: currentM });
  }, [currentM]);
  useEffect(() => {
    setActiveScale((scale) => ({
      ...scale,
      x: zoomState.x_off_milli,
      y: zoomState.y_off_milli,
    }));
  }, [zoomState.x_off_milli, zoomState.y_off_milli]);
  const initialChartViewResetRef = useRef(false);
  const [inputX, setInputX] = useState<string>("0.0000");
  const [inputY, setInputY] = useState<string>("0.0000");
  const zoomStateRef = useRef(zoomState);
  zoomStateRef.current = zoomState;
  const inputXRef = useRef<HTMLInputElement | null>(null);
  const inputYRef = useRef<HTMLInputElement | null>(null);

  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ZigzagSeriesRef | null>(null);
  const chartContainerRef = useRef<HTMLDivElement | null>(null);
  const spacePanHeldRef = useRef(false);
  const spacePanUsedRef = useRef(false);
  const interactionLockedRef = useRef(false);
  const atrValueRef = useRef(0);
  const spacePanSyncSuppressedRef = useRef(false);
  const spacePanSyncReleaseFrameRef = useRef<number | null>(null);
  const spacePanDragRef = useRef<{
    chart: IChartApi;
    startX: number;
    startY: number;
    logicalRange: { from: number; to: number };
    priceRange: { from: number; to: number };
    plotWidth: number;
    plotHeight: number;
    barSpacing: number;
  } | null>(null);
  useEffect(() => {
    const releasePanZoomSync = (): void => {
      if (spacePanSyncReleaseFrameRef.current !== null) {
        cancelAnimationFrame(spacePanSyncReleaseFrameRef.current);
      }
      spacePanSyncReleaseFrameRef.current = requestAnimationFrame(() => {
        spacePanSyncSuppressedRef.current = false;
        spacePanSyncReleaseFrameRef.current = null;
      });
    };
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.code !== "Space") return;
      const target = event.target;
      if (target instanceof HTMLElement && (
        target.isContentEditable || target.closest("input, textarea, select, button, [role='menu']")
      )) return;
      if (event.repeat) {
        event.preventDefault();
        return;
      }
      spacePanHeldRef.current = true;
      spacePanUsedRef.current = false;
      event.preventDefault();
    };
    const onKeyUp = (event: KeyboardEvent): void => {
      if (event.code !== "Space") return;
      spacePanHeldRef.current = false;
      spacePanDragRef.current = null;
      releasePanZoomSync();
    };
    const onBlur = (): void => {
      spacePanHeldRef.current = false;
      spacePanDragRef.current = null;
      releasePanZoomSync();
    };
    const onMouseDown = (event: MouseEvent): void => {
      const container = chartContainerRef.current;
      const chart = chartRef.current;
      if (!spacePanHeldRef.current || event.button !== 0 || !container || !chart) return;
      if (!(event.target instanceof Node) || !container.contains(event.target)) return;
      if (event.target instanceof Element && event.target.closest(".vora-floating-ui")) return;
      if (computeOverChrome(chart, container, event.clientX, event.clientY)) return;

      const logicalRange = chart.timeScale().getVisibleLogicalRange();
      const priceRange = chart.priceScale("right").getVisibleRange();
      const barSpacing = chart.timeScale().options().barSpacing;
      const chartBounds = chart.chartElement().getBoundingClientRect();
      const { bottomH } = readAxisDims(chart);
      const plotWidth = chart.timeScale().width();
      const plotHeight = chartBounds.height - bottomH;
      if (
        !logicalRange || !priceRange || !Number.isFinite(barSpacing) || barSpacing <= 0 ||
        plotWidth <= 0 || plotHeight <= 0
      ) return;

      spacePanUsedRef.current = true;
      spacePanSyncSuppressedRef.current = true;
      spacePanDragRef.current = {
        chart,
        startX: event.clientX,
        startY: event.clientY,
        logicalRange,
        priceRange,
        plotWidth,
        plotHeight,
        barSpacing,
      };
      setSelectionBox(null);
      event.preventDefault();
      event.stopImmediatePropagation();
    };
    const onMouseMove = (event: MouseEvent): void => {
      const drag = spacePanDragRef.current;
      if (!drag) return;
      const deltaX = event.clientX - drag.startX;
      const deltaY = event.clientY - drag.startY;
      const logicalShift = -deltaX / drag.barSpacing;
      const priceShift = deltaY * (drag.priceRange.to - drag.priceRange.from) / drag.plotHeight;
      drag.chart.timeScale().setVisibleLogicalRange({
        from: drag.logicalRange.from + logicalShift,
        to: drag.logicalRange.to + logicalShift,
      });
      drag.chart.priceScale("right").setVisibleRange({
        from: drag.priceRange.from + priceShift,
        to: drag.priceRange.to + priceShift,
      });
      event.preventDefault();
      event.stopImmediatePropagation();
    };
    const onMouseUp = (event: MouseEvent): void => {
      if (!spacePanDragRef.current || event.button !== 0) return;
      spacePanDragRef.current = null;
      releasePanZoomSync();
      event.preventDefault();
      event.stopImmediatePropagation();
    };
    const onWheel = (event: WheelEvent): void => {
      const container = chartContainerRef.current;
      const chart = chartRef.current;
      if (!container || !chart || !(event.target instanceof Node) || !container.contains(event.target)) return;
      if (event.target instanceof Element && event.target.closest(".vora-floating-ui")) return;

      event.preventDefault();
      event.stopImmediatePropagation();
      if (event.ctrlKey || interactionLockedRef.current || replaySelecting) return;

      const series = seriesRef.current;
      const chartElement = chart.chartElement();
      if (!series || !chartElement) return;
      const chartBounds = chartElement.getBoundingClientRect();
      const plotWidth = chart.timeScale().width();
      const plotHeight = chartBounds.height - readAxisDims(chart).bottomH;
      const timeScale = chart.timeScale();
      const logicalRange = timeScale.getVisibleLogicalRange();
      const priceRange = chart.priceScale("right").getVisibleRange();
      if (
        !logicalRange || !priceRange || plotWidth <= 0 || plotHeight <= 0 ||
        !Number.isFinite(event.deltaY) || event.deltaY === 0
      ) return;
      const pointerX = Math.max(0, Math.min(plotWidth, event.clientX - chartBounds.left));
      const pointerY = Math.max(0, Math.min(plotHeight, event.clientY - chartBounds.top));
      const logicalAnchor = timeScale.coordinateToLogical(pointerX)
        ?? (logicalRange.from + logicalRange.to) / 2;
      const priceAnchor = series.coordinateToPrice(pointerY)
        ?? (priceRange.from + priceRange.to) / 2;
      const logicalSpan = logicalRange.to - logicalRange.from;
      const nextPriceSpan = priceRange.to - priceRange.from;
      if (logicalSpan <= 0 || nextPriceSpan <= 0) return;

      const deltaMultiplier = event.deltaMode === WheelEvent.DOM_DELTA_LINE
        ? 16
        : event.deltaMode === WheelEvent.DOM_DELTA_PAGE
          ? Math.max(1, chartBounds.height - readAxisDims(chart).bottomH)
          : 1;
      const scaleFactor = Math.exp(Math.max(-4, Math.min(4, event.deltaY * deltaMultiplier * 0.0015)));
      const timeScaleOptions = timeScale.options();
      const currentBarSpacing = timeScaleOptions.barSpacing;
      const nextBarSpacing = Math.max(
        timeScaleOptions.minBarSpacing,
        Math.min(
          timeScaleOptions.maxBarSpacing,
          currentBarSpacing / scaleFactor,
        ),
      );
      if (Math.abs(nextBarSpacing - currentBarSpacing) >= 0.001) {
        timeScale.applyOptions({ barSpacing: nextBarSpacing });
      }
      const updatedLogicalRange = timeScale.getVisibleLogicalRange();
      if (!updatedLogicalRange) return;
      const nextLogicalSpan = updatedLogicalRange.to - updatedLogicalRange.from;
      const logicalAnchorRatio = Math.max(
        0,
        Math.min(1, (logicalAnchor - logicalRange.from) / logicalSpan),
      );
      const nextLogicalFrom = logicalAnchor - logicalAnchorRatio * nextLogicalSpan;
      timeScale.setVisibleLogicalRange({
        from: nextLogicalFrom,
        to: nextLogicalFrom + nextLogicalSpan,
      });
      const appliedLogicalRange = timeScale.getVisibleLogicalRange();
      const appliedLogicalSpan = appliedLogicalRange
        ? appliedLogicalRange.to - appliedLogicalRange.from
        : nextLogicalSpan;
      const priceCenter = (priceRange.from + priceRange.to) / 2;
      const candleCount = candlesRef.current.length;
      const atr = atrValueRef.current;
      const defaultHalfRange = atr > 0
        ? atr * 4
        : Math.abs(priceCenter) * (candleCount < 14 ? 0.05 : 0.01);
      const minPriceSpan = Math.max(defaultHalfRange * 0.0005, Number.EPSILON);
      const maxPriceSpan = Math.max(defaultHalfRange * 500, minPriceSpan);
      const clampedPriceSpan = Math.max(minPriceSpan, Math.min(maxPriceSpan, nextPriceSpan * scaleFactor));
      const priceAnchorRatio = (priceAnchor - priceRange.from) / nextPriceSpan;
      const nextPriceFrom = priceAnchor - priceAnchorRatio * clampedPriceSpan;
      const nextZoom: ZoomState = {
        x_off_milli: Math.round(-Math.log2(appliedLogicalSpan / DEFAULT_VISIBLE_CANDLE_COUNT) * 1000),
        y_off_milli: defaultHalfRange > 0
          ? Math.round(-Math.log2((clampedPriceSpan / 2) / defaultHalfRange) * 1000)
          : zoomStateRef.current.y_off_milli,
      };
      const zoomChanged = nextZoom.x_off_milli !== zoomStateRef.current.x_off_milli
        || nextZoom.y_off_milli !== zoomStateRef.current.y_off_milli;
      lastAppliedRef.current = {
        cc: appliedLogicalSpan,
        ym: defaultHalfRange > 0 ? Math.round((clampedPriceSpan / 2 / defaultHalfRange) * 4000) : 4000,
      };
      if (zoomChanged) {
        skipNextApplyRef.current = true;
        zoomStateRef.current = nextZoom;
        setZoomState(nextZoom);
        setInputX((nextZoom.x_off_milli / 1000).toFixed(4));
        setInputY((nextZoom.y_off_milli / 1000).toFixed(4));
      }

      chart.priceScale("right").setVisibleRange({
        from: nextPriceFrom,
        to: nextPriceFrom + clampedPriceSpan,
      });
    };
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("keyup", onKeyUp, true);
    window.addEventListener("blur", onBlur);
    window.addEventListener("mousedown", onMouseDown, true);
    window.addEventListener("mousemove", onMouseMove, true);
    window.addEventListener("mouseup", onMouseUp, true);
    window.addEventListener("wheel", onWheel, { capture: true, passive: false });
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("keyup", onKeyUp, true);
      window.removeEventListener("blur", onBlur);
      window.removeEventListener("mousedown", onMouseDown, true);
      window.removeEventListener("mousemove", onMouseMove, true);
      window.removeEventListener("mouseup", onMouseUp, true);
      window.removeEventListener("wheel", onWheel, true);
      if (spacePanSyncReleaseFrameRef.current !== null) {
        cancelAnimationFrame(spacePanSyncReleaseFrameRef.current);
        spacePanSyncReleaseFrameRef.current = null;
      }
    };
  }, [replaySelecting]);
  const longPressTimerRef = useRef<number | null>(null);
  const longPressStartRef = useRef<{ pointerId: number; x: number; y: number } | null>(null);
  const [longPressTooltip, setLongPressTooltip] = useState<{
    x: number;
    y: number;
    timestamp: number | null;
    price: number;
    candle: CandleDto | null;
  } | null>(null);
  useEffect(() => () => {
    if (longPressTimerRef.current !== null) window.clearTimeout(longPressTimerRef.current);
  }, []);
  const pivotBadgeRef = useRef<HTMLDivElement | null>(null);
  const pivotsRef = useRef<PivotDto[]>([]);
  const boxOverlayRef = useRef<BoxOverlay | null>(null);
  const measureRef = useRef<MeasureOverlay | null>(null);
  const rectOverlayRef = useRef<RectangleOverlay | null>(null);
  const gradedRectOverlayRef = useRef<GradedRectangleOverlay | null>(null);
  const gradedRectConfig = useGradedRectStore((state) => state.gradedRectConfig);
  const setGradedRectConfig = useGradedRectStore((state) => state.setGradedRectConfig);
  const gradedRectConfigRef = useRef(gradedRectConfig);
  gradedRectConfigRef.current = gradedRectConfig;
  const hydrateGradedRectConfig = useGradedRectStore((state) => state.hydrateGradedRectConfig);
  const [gradedRectContextMenu, setGradedRectContextMenu] = useState<GradedRectContextMenu | null>(null);
  useEffect(() => { hydrateGradedRectConfig(); }, [hydrateGradedRectConfig]);
  // v7.2.0 Part E — per-symbol persistence key: vora.drawings.<symbol>.
  // v0.86.2 (C2) — Symbol is now reactive (hook-backed).
  // Default is "XAUUSD" until C3 adds the selector.
  const { symbol, setSymbol } = useMarketSymbol();
  useEffect(() => {
    void listDrawingAlerts(symbol).then(setDrawingAlerts).catch(() => setDrawingAlerts([]));
  }, [symbol]);
  const DRAW_KEY = `vora.drawings.${symbol}`;
  const previousSymbolRef = useRef<string>(symbol);
  const skipSymbolDrawingPersistRef = useRef(false);
  const skipSnapshotDrawingHydrateRef = useRef(false);
  const [drawings, setDrawings] = useState<DrawingAny[]>(() => {
    try {
      // Read current + legacy hardcoded key, then migrate legacy fields.
      const currentDrawings = getSetting(`vora.drawings.${symbol}`);
      const legacyDrawings = symbol === "XAUUSD" ? getSetting("vora.drawings.XAUUSD") : null;
      const raw = currentDrawings ?? legacyDrawings;
      if (!raw) return [];
      const parsed: unknown = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      // v7.4.5 — sanitize hydrate: drop null/non-object/missing id|type (read path).
      // v0.83.14 — pruneNullDrawings first so no null reaches the typed filter.
      const arr = pruneNullDrawings(parsed).filter((d: unknown): d is DrawingAny =>
        typeof d === "object" &&
        typeof (d as { id?: unknown }).id === "string" &&
        ((d as { id: string }).id.length > 0) &&
        typeof (d as { type?: unknown }).type === "string");
      // v7.2.0 Part E — migrate legacy { color, fill } → { borderColor, fillColor, fillOpacity }.
      // v7.4.4 — keep rectangle filter for legacy? No: keep ALL types now.
      return arr
        .filter((d) => d && (
          d.type === "rectangle" ||
          d.type === "vline" ||
          d.type === "hline" ||
          d.type === "hray" ||
          d.type === "trendline" ||
          d.type === "ray" ||
          d.type === "info-line" ||
          d.type === "extended-line" ||
          d.type === "trend-angle" ||
          d.type === "crossline" ||
          d.type === "parallel-channel" ||
          d.type === "pitchfork" ||
          d.type === "regression-trend" ||
          d.type === "icon" ||
          d.type === "geometry" ||
          d.type === "fib-gann" ||
          d.type === "graded-rectangle"
        ))
        .filter((d) => d.type !== "graded-rectangle" || isGradedRectangle(d))
        .map((d) => {
          if (d.type === "graded-rectangle") return { ...d, selected: false };
          if (d.type === "rectangle") {
            const r = d as DrawingRect;
            if (r.borderColor && r.fillColor !== undefined && r.fillOpacity !== undefined) return r;
            return migrateDrawingRect(r);
          }
          return migrateDrawingAny(d);
        });
    } catch { return []; }
  });
  // Save the previous instrument before the following effect hydrates the next one.
  useEffect(() => {
    const previousSymbol = previousSymbolRef.current;
    if (previousSymbol === symbol) return;
    replayTimestampOverridesRef.current = null;
    replayTimeframeRestoreRef.current = null;
    setReplayPlaying(false);
    setReplayOpen(false);
    setReplaySelecting(false);
    setReplayStartIndex(null);
    setReplayEndIndex(null);
    replaySkipViewportFollowRef.current = false;
    try {
      setSetting(`vora.drawings.${previousSymbol}`, JSON.stringify(drawings));
    } catch { /* ignore */ }
    previousSymbolRef.current = symbol;
    skipSymbolDrawingPersistRef.current = true;
    setDrawings([]);
  }, [symbol, drawings]);
  // v7.4.4 — rectangle-only projection for the rect overlay / rect tool.
  // v7.2.0 Part E — reload drawings when the symbol changes.
  // v7.4.4 — keep ALL drawing types (rectangle + 4 line tools).
  useEffect(() => {
    if (skipSnapshotDrawingHydrateRef.current) {
      skipSnapshotDrawingHydrateRef.current = false;
      return;
    }
    try {
      const currentDrawings = getSetting(`vora.drawings.${symbol}`);
      const legacyDrawings = symbol === "XAUUSD" ? getSetting("vora.drawings.XAUUSD") : null;
      const raw = currentDrawings ?? legacyDrawings;
      if (!raw) { setDrawings([]); return; }
      const parsed: unknown = JSON.parse(raw);
      if (!Array.isArray(parsed)) { setDrawings([]); return; }
      // v7.4.5 — sanitize hydrate: drop null/non-object/missing id|type (read path).
      // v0.83.14 — pruneNullDrawings first so no null reaches the typed filter.
      const arr = pruneNullDrawings(parsed).filter((d: unknown): d is DrawingAny =>
        typeof d === "object" &&
        typeof (d as { id?: unknown }).id === "string" &&
        ((d as { id: string }).id.length > 0) &&
        typeof (d as { type?: unknown }).type === "string");
      setDrawings(arr
        .filter((d) => d && (
          d.type === "rectangle" ||
          d.type === "vline" ||
          d.type === "hline" ||
          d.type === "hray" ||
          d.type === "trendline" ||
          d.type === "ray" ||
          d.type === "info-line" ||
          d.type === "extended-line" ||
          d.type === "trend-angle" ||
          d.type === "crossline" ||
          d.type === "parallel-channel" ||
          d.type === "pitchfork" ||
          d.type === "regression-trend" ||
          d.type === "icon" ||
          d.type === "geometry" ||
          d.type === "fib-gann" ||
          d.type === "graded-rectangle"
        ))
        .filter((d) => d.type !== "graded-rectangle" || isGradedRectangle(d))
        .map((d) => {
          if (d.type === "graded-rectangle") return { ...d, selected: false };
          if (d.type === "rectangle") {
            const r = d as DrawingRect;
            return (r.borderColor && r.fillColor !== undefined && r.fillOpacity !== undefined) ? r : migrateDrawingRect(r);
          }
          const migrated = migrateDrawingAny(d);
          return migrated.type === "geometry" && migrated.tool === "path"
            ? { ...migrated, selectedSegments: undefined }
            : migrated;
        }));
    } catch { setDrawings([]); }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [symbol]);
  // v0.83.14 — DEV-only one-shot repair of persisted drawings (prune nulls).
  useEffect(() => {
    if (!import.meta.env.DEV) return;
    try {
      const keys = [DRAW_KEY];
      for (const k of keys) {
        const raw = getSetting(k);
        if (!raw) continue;
        const parsed: unknown = JSON.parse(raw);
        if (!Array.isArray(parsed)) continue;
        const hadNull = parsed.some((x) => x == null);
        if (!hadNull) continue;
        const pruned = pruneNullDrawings(parsed as DrawingAny[]);
        setSetting(k, JSON.stringify(pruned));
        // eslint-disable-next-line no-console
        console.warn(
          "[drawings] pruned null lines from localStorage:",
          parsed.length - pruned.length,
        );
      }
    } catch {
      /* ignore */
    }
  }, [DRAW_KEY]);
  // v7.2.0 Part A — master switch for the auto RBR/DBD detection pipeline.
  const [autoDetect, setAutoDetect] = useState<boolean>(() => {
    try {
      const raw = getSetting("vora.policy.autoDetect");
      return raw == null ? true : raw === "1";
    } catch { return true; }
  });
  useEffect(() => {
    try { setSetting("vora.policy.autoDetect", autoDetect ? "1" : "0"); } catch { /* ignore */ }
  }, [autoDetect]);
  // v7.2.0 C2/C3 — hover state for refined-handle visibility.
  const [rectHover, setRectHover] = useState<RectHover | null>(null);
  const rectHoverRef = useRef(rectHover);
  rectHoverRef.current = rectHover;
  const [rectToolActive, setRectToolActive] = useState(false);
  const [selectionBox, setSelectionBox] = useState<SelectionBox | null>(null);
  const [drawingsHidden, setDrawingsHidden] = useState(false);
  const [alwaysRemoveLockedDrawings, setAlwaysRemoveLockedDrawings] = useState<boolean>(() => {
    try { return getSetting("vora.chart.always_remove_locked_drawings") !== "false"; }
    catch { return true; }
  });
  useEffect(() => {
    try { setSetting("vora.chart.always_remove_locked_drawings", String(alwaysRemoveLockedDrawings)); }
    catch { /* ignore */ }
  }, [alwaysRemoveLockedDrawings]);
  const drawingsRef = useRef<DrawingAny[]>(drawings);
  drawingsRef.current = drawings;
  const interactionDrawingsRef = useRef<DrawingAny[]>(drawings);
  interactionDrawingsRef.current = drawingsHidden ? [] : drawings.filter(isDrawingVisible);
  const rectToolRef = useRef(false);
  rectToolRef.current = rectToolActive;
  // v7.4.2 — suppress drawings persist once after Remove-objects clear
  // (keeps vora.drawings.* keys intact while visible state is cleared).
  const suppressDrawingsPersistRef = useRef(false);
  const clearedDrawingsSnapshotRef = useRef<{ key: string; raw: string | null }[]>([]);
  // v7.3.0 Part A — snapshot BEFORE any drawings mutation (draw, resize,
  // move, delete, clear-all). Callers pass the CURRENT snapshot explicitly.
  const pushDrawingsHistory = useCallback((snap: DrawingAny[]): void => {
    undoHistory.push({ drawings: pruneNullDrawings(snap) });
    setHistoryTick((t) => t + 1);
  }, []);
  const undoDrawings = useCallback((): void => {
    const prev = undoHistory.undo({ drawings: pruneNullDrawings(drawingsRef.current) });
    if (prev) {
      const restored = pruneNullDrawings(prev.drawings).map((d) => ({ ...d }));
      drawingsRef.current = restored;
      setDrawings(restored);
      setHistoryTick((t) => t + 1);
    }
  }, []);
  const redoDrawings = useCallback((): void => {
    const next = undoHistory.redo({ drawings: pruneNullDrawings(drawingsRef.current) });
    if (next) {
      const restored = pruneNullDrawings(next.drawings).map((d) => ({ ...d }));
      drawingsRef.current = restored;
      setDrawings(restored);
      setHistoryTick((t) => t + 1);
    }
  }, []);
  const debouncedMRef = useRef(debouncedM);
  debouncedMRef.current = debouncedM;
  // v0.83.13 — tracks which M the current `candles` array was fetched for.
  // Effect A short-circuits while currentM !== lastFetchedM (stale-M guard).
  const lastFetchedIntervalRef = useRef<string>("");
  const rectDragRef = useRef<RectDrag | null>(null);
  // v7.1.1 — live click-click preview + rect snap marker.
  const [rectPreview, setRectPreview] = useState<DrawingPreview | null>(null);
  const previewRef = useRef<DrawingPreview | null>(null);
  previewRef.current = rectPreview;
  const [rectSnap, setRectSnap] = useState<{ t: number; p: number } | null>(null);
  const [measureActive, setMeasureActive] = useState(false);
  const [overChrome, setOverChrome] = useState(false);
  // Cursor cascade class. LC's canvas children set `cursor: auto`,
  // so the chart cursor must be forced through a CSS class (see App.css).
  const chartCursorClass = replaySelecting
    ? "vora-chart-cursor-crosshair"
    : rectToolActive
    ? "vora-chart-cursor-crosshair"
    : overChrome
      ? "vora-chart-cursor-default"
      : "vora-chart-cursor-none";
  const [measurePending, setMeasurePending] = useState<{ t: number; p: number } | null>(null);
  const [measureComplete, setMeasureComplete] = useState<{ tA: number; pA: number; tB: number; pB: number } | null>(null);
  const [boxes, setBoxes] = useState<BoxDto[]>([]);
  // v7.1.2: STATS panel removed — chains state kept (detect flow still writes it).
  const [chains, setChains] = useState<ChainDto[]>([]);
  void chains;
  const [pivots, setPivots] = useState<PivotDto[]>([]);
  const [scaleRegistry, setScaleRegistry] =
    useState<ScaleRegistry | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [saveOk, setSaveOk] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const previousAnalysisSymbolRef = useRef(symbol);
  useEffect(() => {
    if (previousAnalysisSymbolRef.current === symbol) return;
    previousAnalysisSymbolRef.current = symbol;
    setBoxes([]);
    setChains([]);
    setPivots([]);
    setScaleRegistry(null);
    setSaveOk(false);
    setSaveError(null);
  }, [symbol]);
  const [registryRefreshKey, setRegistryRefreshKey] = useState(0);
  const [detectRefreshKey, setDetectRefreshKey] = useState(0);
  const [chartReady, setChartReady] = useState<boolean>(false);
  const [panelOpen, setPanelOpen] = useState<boolean>(false);
  // v7.3.0 Part A — undo/redo stack is in-memory only (clears on restart).
  // historyTick forces the toolbar Undo/Redo buttons to re-render.
  // v7.3.0 Part B — lastTool + ctxMenu drive the right-click context menu.
  const [historyTick, setHistoryTick] = useState(0);
  void historyTick;
  const [lastTool, setLastTool] = useState<"rect" | null>(null);
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number; priceText: string | null } | null>(null);
  const [toolbarToast82, setToolbarToast82] = useState<string | null>(null);
  // v0.83.0 — Live Snapshot capture UI state.
  const [snapshotModalOpen, setSnapshotModalOpen] = useState(false);
  const [isRecording, setIsRecording] = useState(false);
  const [fullNarrativeLog, setFullNarrativeLog] = useState<NarrativeEvent[]>([]);
  const lastNarrativeSignatureRef = useRef<string | null>(null);
  const [narrativePlaybackSnapshot, setNarrativePlaybackSnapshot] = useState<VoraSnapshot | null>(null);
  const [narrativePlaybackBaseDrawings, setNarrativePlaybackBaseDrawings] = useState<DrawingAny[]>([]);
  // v0.83.2 — snapshot restore overlay + idempotency guard.
  const [restoreOverlay, setRestoreOverlay] = useState<{ visible: boolean; name: string }>({ visible: false, name: "" });
  const restoredRef = useRef<string | null>(null);
  const snapshotRestoreRequestRef = useRef(0);
  const snapshotFetchSuppressionRef = useRef<{ symbol: string; interval: string } | null>(null);
  const pendingSnapshotRegistryRef = useRef<{ symbol: string; scales: SnapshotPolicy["registry"] } | null>(null);
  // v0.83.4 — when true, CandlestickChart skips fitContent() after setData
  // so the restored viewport survives. True only during snapshot restore.
  const [preserveViewport, setPreserveViewport] = useState(false);
  // v0.83.4 — holds the last restored viewport + expiry so the debounced
  // candle refetch (setCurrentM -> fetchCandles -> Effect A template) does
  // not clobber the exact restored zoom.
  const restoreHoldRef = useRef<{ viewport: SnapshotViewport; expiry: number } | null>(null);
  // v0.83.5 — viewport captured BEFORE a timeframe fetch so zoom (X+Y)
  // survives timeframe switches. Applied after setCandles + double rAF.
  const viewportToRestoreRef = useRef<CapturedViewport | null>(null);
  // v0.83.5 — suppress Effect A template while a timeframe viewport hold
  // is active (prevents applyTemplateAtr from clobbering preserved zoom).
  const timeframeHoldRef = useRef<{ viewport: CapturedViewport; expiry: number } | null>(null);
  // v0.83.7 Part E — primary in-flight guard: suppress
  // applyTemplateAtr fully during the timeframe-change window.
  const timeframeChangeInFlightRef = useRef(false);
  // v0.83.5 — "Scroll to most recent bar" affordance.
  const [scrollToLatestVisible, setScrollToLatestVisible] = useState(false);
  // v0.83.2.1 — global interaction lock (modal/overlay open → chart inert).
  const interactionLocked = useInteractionLocked();
  interactionLockedRef.current = interactionLocked;
  const showSnapshotToast = useCallback((m: string) => {
    setToolbarToast82(m);
    window.setTimeout(() => setToolbarToast82(null), 2200);
  }, []);
  const captureNarrativeEvent = useCallback((): void => {
    const drawings = drawingsRef.current.map((drawing) => ({
      ...structuredClone(drawing),
      selected: false,
      hovered: false,
    }));
    const activeScale = { ...activeScaleRef.current };
    const signature = JSON.stringify({ drawings, activeScale });
    if (signature === lastNarrativeSignatureRef.current) return;
    lastNarrativeSignatureRef.current = signature;
    const event: NarrativeEvent = {
      timestamp: Date.now(),
      drawings,
      activeScale,
    };
    setFullNarrativeLog((previous) => [...previous, event]);
  }, []);

  const applyNarrativeDrawings = useCallback((next: DrawingAny[]): void => {
    const restored = next.map((drawing) => ({ ...drawing }));
    drawingsRef.current = restored;
    setDrawings(restored);
  }, []);
  const applyNarrativeScale = useCallback((scale: ActiveScale): void => {
    setActiveScale({ ...scale });
    setCurrentM(Math.max(0, Math.min(10, Math.round(scale.s))));
    setZoomState({ x_off_milli: Math.round(scale.x), y_off_milli: Math.round(scale.y) });
  }, []);
  const closeNarrativePlayback = useCallback((): void => {
    setNarrativePlaybackSnapshot(null);
    setSearchParams((current) => {
      const next = new URLSearchParams(current);
      next.delete("playNarrative");
      return next;
    }, { replace: true });
  }, [setSearchParams]);

  useEffect(() => {
    if (!isRecording) return;
    const timer = window.setInterval(captureNarrativeEvent, 50);
    return () => window.clearInterval(timer);
  }, [captureNarrativeEvent, isRecording]);

  const handleStartRecording = useCallback((): void => {
    lastNarrativeSignatureRef.current = null;
    setFullNarrativeLog([]);
    setIsRecording(true);
    captureNarrativeEvent();
  }, [captureNarrativeEvent]);

  const handleRecordingButtonClick = useCallback((): void => {
    if (!isRecording) {
      handleStartRecording();
      return;
    }
    captureNarrativeEvent();
    setIsRecording(false);
    setSnapshotModalOpen(true);
  }, [captureNarrativeEvent, handleStartRecording, isRecording]);

  // v7.3.2 Part 2D — clear history ONLY on symbol change (never on drawings change).
  useEffect(() => {
    undoHistory.clear();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [symbol]);
  // v7.3.2 Part 2E — persist drawings after every mutation (incl. undo/redo).
  // v7.4.2 — skip once after Remove-objects clear, restoring vora.drawings.*.
  useEffect(() => {
    if (suppressDrawingsPersistRef.current) {
      suppressDrawingsPersistRef.current = false;
      try {
        for (const { key, raw } of clearedDrawingsSnapshotRef.current) {
          if (raw !== null && raw !== undefined) setSetting(key, raw);
        }
      } catch { /* ignore */ }
      clearedDrawingsSnapshotRef.current = [];
      return;
    }
    if (skipSymbolDrawingPersistRef.current) {
      skipSymbolDrawingPersistRef.current = false;
      return;
    }
    try {
      if (drawings.length === 0) deleteSetting(DRAW_KEY);
      else setSetting(DRAW_KEY, JSON.stringify(drawings));
    } catch { /* ignore */ }
  }, [drawings, DRAW_KEY]);
  // v7.3.0 Part C — clicking outside the chart canvas deselects.
  // (Clicks INSIDE the canvas are resolved by the rect tool: rect → select,
  // empty → deselect. Toolbars/axes/palette live outside the canvas.)
  const hasSelection = pruneNullDrawings(drawings).some((d) => d.selected);
  useEffect(() => {
    if (!hasSelection) return;
    const onDocDown = (e: MouseEvent): void => {
      // v0.83.2.1 — locked (modal/overlay open): never deselect/move drawings.
      if (interactionLocked) return;
      // v7.3.0 Part C rule 4 — palette/toolbar/axes live outside the chart
      // canvas, so a mousedown outside the canvas only deselects.
      const t = e.target as HTMLElement | null;
      const container = chartContainerRef.current;
      if (container && t && container.contains(t)) return;
      setDrawings((prev) => pruneNullDrawings(prev).some((d) => d.selected)
        ? pruneNullDrawings(prev).map((d) => ({ ...d, selected: false }))
        : pruneNullDrawings(prev));
    };
    document.addEventListener("mousedown", onDocDown, true);
    return () => document.removeEventListener("mousedown", onDocDown, true);
  }, [hasSelection, interactionLocked]);

  useEffect(() => {
    if (candles.length < 2 || drawingAlerts.length === 0) return;
    const current = candles[candles.length - 1];
    const previous = candles[candles.length - 2];
    const now = Math.floor(Date.now() / 1000);
    const drawingsById = new Map(drawings.map((drawing) => [drawing.id, drawing]));
    const pendingUpdates: { id: string; enabled: boolean; lastEvaluatedAt: number }[] = [];
    for (const alert of drawingAlerts) {
      if (!alert.enabled || alert.symbol !== symbol || alert.timeframe !== currentM || current.ts <= (alert.lastEvaluatedAt ?? 0)) continue;
      if (hasDrawingAlertExpired(alert, now)) {
        pendingUpdates.push({ id: alert.id, enabled: false, lastEvaluatedAt: current.ts });
        continue;
      }
      const drawing = drawingsById.get(alert.drawingId);
      if (!drawing) {
        pendingUpdates.push({ id: alert.id, enabled: false, lastEvaluatedAt: current.ts });
        continue;
      }
      const triggered = drawingAlertTriggered(alert, drawing, previous, current, candles);
      pendingUpdates.push({ id: alert.id, enabled: triggered && alert.triggerOnce ? false : alert.enabled, lastEvaluatedAt: current.ts });
      if (!triggered) continue;
      if (alert.notifyToast) {
        setToolbarToast82(alert.message);
        window.setTimeout(() => setToolbarToast82(null), 4000);
      }
      if (alert.notifyApp && "Notification" in window && Notification.permission === "granted") {
        new Notification("VORA chart alert", { body: alert.message });
      }
      if (alert.notifySound) {
        try {
          const audio = new AudioContext();
          const oscillator = audio.createOscillator();
          const gain = audio.createGain();
          oscillator.frequency.value = 880;
          gain.gain.setValueAtTime(0.08, audio.currentTime);
          gain.gain.exponentialRampToValueAtTime(0.001, audio.currentTime + 0.25);
          oscillator.connect(gain);
          gain.connect(audio.destination);
          oscillator.start();
          oscillator.stop(audio.currentTime + 0.25);
          oscillator.onended = () => { void audio.close(); };
        } catch { /* audio may be unavailable or blocked */ }
      }
    }
    if (pendingUpdates.length === 0) return;
    for (const update of pendingUpdates) void updateDrawingAlert(update.id, update.enabled, update.lastEvaluatedAt).catch(() => undefined);
    setDrawingAlerts((previousAlerts) => previousAlerts.map((alert) => {
      const update = pendingUpdates.find((item) => item.id === alert.id);
      return update ? { ...alert, enabled: update.enabled, lastEvaluatedAt: update.lastEvaluatedAt } : alert;
    }));
  }, [candles, currentM, drawings, drawingAlerts, symbol]);
  // v7.0.0 Part A: chart display mode defaults to candlesticks for each live chart session.
  type ChartDisplayMode = "special" | "candlestick";
  const [chartDisplayMode, setChartDisplayMode] = useState<ChartDisplayMode>("candlestick");
  // v7.0.0 Part B: strong magnet for drawing tools (persisted)
  type MagnetMode = "off" | "weak" | "strong";
  const [magnetMode, setMagnetMode] = useState<MagnetMode>(() => {
    try {
      const saved = getSetting("vora.magnet_mode");
      return saved === "weak" || saved === "strong" ? saved : "off";
    } catch { return "off"; }
  });
  const [lastMagnetMode, setLastMagnetMode] = useState<"weak" | "strong">(() => {
    try {
      const savedLast = getSetting("vora.magnet_last_mode");
      if (savedLast === "weak" || savedLast === "strong") return savedLast;
      return getSetting("vora.magnet_mode") === "strong" ? "strong" : "weak";
    } catch { return "weak"; }
  });
  const selectMagnetMode = (nextMode: MagnetMode): void => {
    setMagnetMode(nextMode);
    try { setSetting("vora.magnet_mode", nextMode); } catch { /* ignore */ }
    if (nextMode !== "off") {
      setLastMagnetMode(nextMode);
      try { setSetting("vora.magnet_last_mode", nextMode); } catch { /* ignore */ }
    }
    const effectiveNextMode = ctrlHeldRef.current ? (nextMode === "off" ? "strong" : "off") : nextMode;
    if (effectiveNextMode === "off") {
      snapPointRef.current = null;
      measureRef.current?.setSnap(null);
      setRectSnap(null);
    }
  };
  const [keepDrawing, setKeepDrawing] = useState(false);
  const keepDrawingRef = useRef(keepDrawing);
  keepDrawingRef.current = keepDrawing;
  // v7.0.0 Part B5: snap marker feedback (snapped OHLC point in data coords)
  const [snapPoint, setSnapPoint] = useState<{ t: number; p: number } | null>(null);
  const snapPointRef = useRef<{ t: number; p: number } | null>(null);
  // v7.0.0 Part C4: default-route setting (persisted, default scale-lab)
  const [defaultRouteSetting, setDefaultRouteState] = useState<DefaultRoute>(() => getDefaultRoute());
  const setDefaultRouteSetting = (r: DefaultRoute) => {
    setDefaultRoute(r);
    setDefaultRouteState(r);
  };
  // v7.4.0 — floating drawing toolbar visibility (persisted).
  const [floatingToolbarVisible, setFloatingToolbarVisible] = useState<boolean>(() => {
    try {
      const v = getSetting("vora.floating_toolbar.visible");
      return v === null ? true : v === "true";
    } catch { return true; }
  });
  useEffect(() => {
    try { setSetting("vora.floating_toolbar.visible", String(floatingToolbarVisible)); } catch { /* ignore */ }
  }, [floatingToolbarVisible]);
  const [cursorFavorites, setCursorFavorites] = useState<string[]>(() => {
    try {
      const raw = getSetting("vora.chart.cursor_favorites");
      if (!raw) return [];
      const parsed: unknown = JSON.parse(raw);
      const allowed = new Set(["cursor", "cursor-dot", "cursor-arrow", "cursor-demo", "cursor-magic", "cursor-eraser"]);
      return Array.isArray(parsed) ? parsed.filter((id): id is string => typeof id === "string" && allowed.has(id)) : [];
    } catch { return []; }
  });
  useEffect(() => {
    try { setSetting("vora.chart.cursor_favorites", JSON.stringify(cursorFavorites)); } catch { /* ignore */ }
  }, [cursorFavorites]);
  const [drawingToolFavorites, setDrawingToolFavorites] = useState<string[]>(() => {
    try {
      const stored: unknown = JSON.parse(getSetting("vora.chart.drawing_tool_favorites") ?? "[]");
      return Array.isArray(stored) ? stored.filter((id): id is string => typeof id === "string") : [];
    } catch { return []; }
  });
  useEffect(() => {
    try { setSetting("vora.chart.drawing_tool_favorites", JSON.stringify(drawingToolFavorites)); } catch { /* ignore */ }
  }, [drawingToolFavorites]);
  const [valuesTooltipOnLongPress, setValuesTooltipOnLongPress] = useState<boolean>(() => {
    try {
      const stored = getSetting("vora.chart.values_tooltip.long_press");
      return stored === null ? true : stored === "true";
    } catch { return true; }
  });
  const [isHomeMenuOpen, setIsHomeMenuOpen] = useState(false);
  useEffect(() => {
    try { setSetting("vora.chart.values_tooltip.long_press", String(valuesTooltipOnLongPress)); } catch { /* ignore */ }
  }, [valuesTooltipOnLongPress]);
  // v7.4.3 — 2s delayed custom tooltips for display-toggle + remove-objects.
  const [showDisplayTip, setShowDisplayTip] = useState(false);
  const displayHoverTimer = useRef<number | null>(null);
  useEffect(() => {
    return () => {
      if (displayHoverTimer.current !== null) { window.clearTimeout(displayHoverTimer.current); displayHoverTimer.current = null; }
    };
  }, []);
  // v7.4.0 — active tool id for the floating toolbar (rect/measure are functional).
  const [activeTool, setActiveTool] = useState<string>("cursor");
  const gradedActiveToolRef = useRef(activeTool);
  gradedActiveToolRef.current = activeTool;
  // v0.83.0 — Live Snapshot capture: builds the full snapshot from live refs.
  const handleSaveSnapshot = useCallback(async (input: { name: string; notes: string; tags: string[]; saveMode: "STANDARD" | NarrativeMode }) => {
    const chart = chartRef.current;
    if (!chart) throw new Error("Chart not ready yet.");
    const narrativeMode = input.saveMode === "STANDARD" ? undefined : input.saveMode;
    const narrativeLog = input.saveMode === "FULL_VIDEO"
      ? fullNarrativeLog
      : input.saveMode === "COMPACT_STEPS"
        ? deriveCompactSteps(fullNarrativeLog)
        : undefined;
    const initialDrawings = fullNarrativeLog[0]?.drawings ?? drawingsRef.current;
    const viewport = captureViewport(chart);
    const snap = await buildSnapshot({
      name: input.name,
      notes: input.notes,
      tags: input.tags,
      chart: captureChartState({ symbol, timeframe: activeInterval, displayMode: chartDisplayMode }),
      viewport,
      drawings: captureDrawings((input.saveMode === "STANDARD" ? drawingsRef.current : initialDrawings).filter(Boolean)),
      policy: capturePolicy(policyRef.current, scaleRegistry),
      theme: captureTheme(chart),
      ...(narrativeMode && narrativeLog ? { narrativeMode, narrativeLog } : {}),
    });
    await saveSnapshot(snap);
    setFullNarrativeLog([]);
    showSnapshotToast(`Snapshot saved: ${snap.name}`);
    setSnapshotModalOpen(false);
  }, [symbol, activeInterval, chartDisplayMode, scaleRegistry, showSnapshotToast, fullNarrativeLog]);
  // v7.4.4 — active line tool (null when rect/cursor/etc).
  const lineTool: LineToolId | null = isLineToolId(activeTool) ? activeTool : null;
  const lineToolRef = useRef<LineToolId | null>(lineTool);
  lineToolRef.current = lineTool;
  const iconSelectionRef = useRef<DrawingIconSelection | null>(null);
  const floatingActiveTool: string = rectToolActive ? "rect" : measureActive ? "measure" : (lineTool ?? activeTool);
  const handleDrawingToolSelect = (id: string): void => {
    setDrawingsHidden(false);
    const isLineTool = isLineToolId(id);
    if (id === "cursor") {
      setActiveTool("cursor");
      setGradedRectContextMenu(null);
      setRectToolActive(false);
      setRectPreview(null);
      rectDragRef.current = null;
      setRectSnap(null);
      setMeasureActive(false);
      setMeasurePending(null);
      return;
    }
    if (id === "graded-rectangle") {
      setRectToolActive(false);
      setRectPreview(null);
      rectDragRef.current = null;
      setRectSnap(null);
      setMeasureActive(false);
      setMeasurePending(null);
      setTrendPreview(null);
      setGradedRectContextMenu(null);
      setActiveTool((previous) => previous === "graded-rectangle" ? "cursor" : "graded-rectangle");
      return;
    }
    if (id === "rect") {
      setLastTool("rect");
      setActiveTool("cursor");
      setTrendPreview(null);
      setRectToolActive((previous) => {
        const next = !previous;
        if (!next) {
          setRectPreview(null);
          rectDragRef.current = null;
          setRectSnap(null);
        }
        return next;
      });
      if (measureActive) {
        if (measurePending) setMeasurePending(null);
        setMeasureActive(false);
      }
      return;
    }
    if (id === "measure") {
      if (measureActive && measurePending) setMeasurePending(null);
      setMeasureActive((previous) => !previous);
      if (rectToolActive) {
        setRectToolActive(false);
        setRectPreview(null);
        rectDragRef.current = null;
        setRectSnap(null);
      }
      if (lineTool) {
        setActiveTool("cursor");
        setTrendPreview(null);
        setRectSnap(null);
      }
      return;
    }
    if (id === "icon") {
      if (rectToolActive) {
        setRectToolActive(false);
        setRectPreview(null);
        rectDragRef.current = null;
        setRectSnap(null);
      }
      if (measureActive) {
        setMeasureActive(false);
        setMeasurePending(null);
      }
      setTrendPreview(null);
      setActiveTool("icon");
      return;
    }
    if (isLineTool) {
      if (rectToolActive) {
        setRectToolActive(false);
        setRectPreview(null);
        rectDragRef.current = null;
        setRectSnap(null);
      }
      if (measureActive) {
        if (measurePending) setMeasurePending(null);
        setMeasureActive(false);
      }
      setTrendPreview(null);
      setActiveTool((previous) => (previous === id ? "cursor" : id));
    }
  };
  const stopChartToolsForReplay = (): void => {
    setActiveTool("cursor");
    setRectToolActive(false);
    setRectPreview(null);
    rectDragRef.current = null;
    setRectSnap(null);
    setTrendPreview(null);
    setMeasureActive(false);
    setMeasurePending(null);
    setMeasureComplete(null);
  };
  const closeBarReplay = (): void => {
    replayTimeframeRestoreRef.current = null;
    replayTimestampOverridesRef.current = null;
    setReplayPlaying(false);
    setReplayOpen(false);
    setReplaySelecting(false);
    setReplayStartIndex(null);
    setReplayEndIndex(null);
    replaySkipViewportFollowRef.current = false;
  };
  const selectReplayStart = (index: number): void => {
    replayTimestampOverridesRef.current = null;
    setReplayPlaying(false);
    setReplaySelecting(false);
    setReplayStartIndex(index);
    replaySkipViewportFollowRef.current = true;
    replayCursorRef.current = index;
    setReplayEndIndex(index);
  };
  const beginReplayBarSelection = (): void => {
    if (candles.length === 0) return;
    stopChartToolsForReplay();
    setReplayPlaying(false);
    replaySkipViewportFollowRef.current = false;
    setReplayCursorX(null);
    setReplayHoverTimestamp(null);
    setReplaySelecting(true);
  };
  const toggleBarReplay = (): void => {
    if (replayOpen) {
      closeBarReplay();
      return;
    }
    setReplayOpen(true);
    beginReplayBarSelection();
  };
  const stepReplayForward = (): void => {
    if (replayEndIndex === null) return;
    if (replayTimestampOverridesRef.current) replayTimestampOverridesRef.current.end = null;
    const updateInterval = replayAutoUpdateInterval ? "chart" : replayUpdateInterval;
    const nextIndex = updateInterval === "second"
      ? advanceReplayBySecond(candles, replayEndIndex)
      : advanceReplayByBar(candles, replayEndIndex);
    replayCursorRef.current = nextIndex;
    setReplayEndIndex(nextIndex);
    if (nextIndex >= candles.length - 1) setReplayPlaying(false);
  };
  const jumpReplayToLatest = (): void => {
    if (candles.length === 0) return;
    if (replayTimestampOverridesRef.current) replayTimestampOverridesRef.current.end = null;
    const lastIndex = candles.length - 1;
    replayCursorRef.current = lastIndex;
    setReplayEndIndex(lastIndex);
    setReplayPlaying(false);
    setReplaySelecting(false);
  };
  const toggleReplayPlayback = (): void => {
    if (replayPlaying) {
      setReplayPlaying(false);
      return;
    }
    if (replayEndIndex !== null && replayEndIndex < candles.length - 1) setReplayPlaying(true);
  };

  useEffect(() => {
    if (!replayOpen || !replayPlaying || replayStartIndex === null) return;
    const timer = window.setInterval(() => {
      const currentIndex = replayCursorRef.current;
      if (currentIndex === null) return;
      const updateInterval = replayAutoUpdateInterval ? "chart" : replayUpdateInterval;
      const nextIndex = updateInterval === "second"
        ? advanceReplayBySecond(candles, currentIndex)
        : advanceReplayByBar(candles, currentIndex);
      if (replayTimestampOverridesRef.current) replayTimestampOverridesRef.current.end = null;
      replayCursorRef.current = nextIndex;
      setReplayEndIndex(nextIndex);
      if (nextIndex >= candles.length - 1) setReplayPlaying(false);
    }, Math.max(50, 1000 / replaySpeed));
    return () => window.clearInterval(timer);
  }, [replayOpen, replayPlaying, replayStartIndex, replaySpeed, replayUpdateInterval, replayAutoUpdateInterval, candles]);

  useEffect(() => {
    if (!replayOpen || !replaySelecting || !chartReady || interactionLocked) return;
    const chart = chartRef.current;
    if (!chart) return;
    const onReplaySelect = (param: MouseEventParams): void => {
      if (!param.point) return;
      const chartTime = typeof param.time === "number"
        ? param.time
        : chart.timeScale().coordinateToTime(param.point.x);
      if (typeof chartTime !== "number") return;
      const index = nearestReplayIndex(candles, chartTime);
      if (index !== null) selectReplayStart(index);
    };
    chart.subscribeClick(onReplaySelect);
    return () => {
      try { chart.unsubscribeClick(onReplaySelect); } catch { /* noop */ }
    };
  }, [replayOpen, replaySelecting, chartReady, interactionLocked, candles]);

  useEffect(() => {
    if (!replayOpen || replayEndIndex === null || !chartReady) return;
    const frame = window.requestAnimationFrame(() => {
      const chart = chartRef.current;
      if (!chart) return;
      if (replaySkipViewportFollowRef.current) {
        replaySkipViewportFollowRef.current = false;
        return;
      }
      const range = chart.timeScale().getVisibleLogicalRange();
      if (range && replayEndIndex > range.to - 2) {
        const span = Math.max(10, range.to - range.from);
        const from = Math.max(0, replayEndIndex - span * 0.65);
        chart.timeScale().setVisibleLogicalRange({ from, to: Math.min(candles.length - 1, from + span) });
      }
    });
    return () => window.cancelAnimationFrame(frame);
  }, [replayOpen, replayEndIndex, chartReady]);

  useEffect(() => {
    if (!replayOpen || !replaySelecting || !chartReady) {
      setReplayCursorX(null);
      setReplayHoverTimestamp(null);
      return;
    }
    const chart = chartRef.current;
    const container = chartContainerRef.current;
    if (!chart || !container) return;

    const updateCursorPosition = (param: MouseEventParams): void => {
      if (!param.point) {
        setReplayCursorX(null);
        setReplayHoverTimestamp(null);
        return;
      }
      const chartTime = typeof param.time === "number"
        ? param.time
        : chart.timeScale().coordinateToTime(param.point.x);
      if (typeof chartTime !== "number") {
        setReplayCursorX(null);
        setReplayHoverTimestamp(null);
        return;
      }
      const candleIndex = nearestReplayIndex(candles, chartTime);
      const timestamp = candleIndex === null ? undefined : candles[candleIndex]?.ts;
      if (timestamp === undefined) {
        setReplayCursorX(null);
        setReplayHoverTimestamp(null);
        return;
      }
      const coordinate = chart.timeScale().timeToCoordinate(timestamp as UTCTimestamp as Time);
      const chartElement = chart.chartElement();
      if (coordinate === null || !chartElement) {
        setReplayCursorX(null);
        setReplayHoverTimestamp(null);
        return;
      }
      const chartRect = chartElement.getBoundingClientRect();
      const containerRect = container.getBoundingClientRect();
      setReplayCursorX(chartRect.left - containerRect.left + coordinate);
      setReplayHoverTimestamp(timestamp);
    };
    chart.subscribeCrosshairMove(updateCursorPosition);
    return () => {
      chart.unsubscribeCrosshairMove(updateCursorPosition);
    };
  }, [replayOpen, replaySelecting, chartReady, candles]);

  const selectReplayRange = (range: ReplayRange): void => {
    setReplayRange(range);
    const chart = chartRef.current;
    const endIndex = replayEndIndex ?? candles.length - 1;
    const endCandle = candles[endIndex];
    const endTimestamp = replayTimestampOverridesRef.current?.end ?? endCandle?.ts;
    if (!chart || !endCandle || endTimestamp === undefined) return;
    if (range === "All") {
      chart.timeScale().fitContent();
      return;
    }

    const endDate = new Date(endTimestamp * 1000);
    const startDate = new Date(endDate);
    if (range === "1D") startDate.setUTCDate(startDate.getUTCDate() - 1);
    else if (range === "5D") startDate.setUTCDate(startDate.getUTCDate() - 5);
    else if (range === "1M") startDate.setUTCMonth(startDate.getUTCMonth() - 1);
    else if (range === "3M") startDate.setUTCMonth(startDate.getUTCMonth() - 3);
    else if (range === "6M") startDate.setUTCMonth(startDate.getUTCMonth() - 6);
    else if (range === "YTD") startDate.setTime(Date.UTC(endDate.getUTCFullYear(), 0, 1));
    else if (range === "1Y") startDate.setUTCFullYear(startDate.getUTCFullYear() - 1);
    else startDate.setUTCFullYear(startDate.getUTCFullYear() - 5);
    const startIndex = nearestReplayIndex(candles, Math.floor(startDate.getTime() / 1000)) ?? 0;
    const span = Math.max(1, endIndex - startIndex);
    chart.timeScale().setVisibleLogicalRange({ from: startIndex, to: Math.min(candles.length - 1, endIndex + span * 0.12) });
  };

  useEffect(() => {
    if (!replayOpen) return;
    const onReplayKeyDown = (event: KeyboardEvent): void => {
      const target = event.target;
      if (target instanceof HTMLElement && (target.isContentEditable || target.closest("input, textarea, select, button, [role='menu']"))) return;
      if (event.key === "Escape" && replaySelecting) {
        event.preventDefault();
        event.stopPropagation();
        setReplaySelecting(false);
        return;
      }
      if (event.key === " ") {
        event.preventDefault();
        event.stopPropagation();
        if (!spacePanHeldRef.current) toggleReplayPlayback();
      } else if (event.key === "ArrowRight") {
        event.preventDefault();
        event.stopPropagation();
        stepReplayForward();
      } else if (event.key === "ArrowLeft" && replayEndIndex !== null) {
        event.preventDefault();
        event.stopPropagation();
        const startIndex = replayStartIndex ?? 0;
        const previousIndex = Math.max(startIndex, replayEndIndex - 1);
        if (replayTimestampOverridesRef.current) replayTimestampOverridesRef.current.end = null;
        replayCursorRef.current = previousIndex;
        setReplayEndIndex(previousIndex);
        setReplayPlaying(false);
      }
    };
    const onReplayKeyUp = (event: KeyboardEvent): void => {
      if (event.code !== "Space" || spacePanUsedRef.current) return;
      const target = event.target;
      if (target instanceof HTMLElement && (
        target.isContentEditable || target.closest("input, textarea, select, button, [role='menu']")
      )) return;
      event.preventDefault();
      event.stopPropagation();
      toggleReplayPlayback();
    };
    window.addEventListener("keydown", onReplayKeyDown, true);
    window.addEventListener("keyup", onReplayKeyUp, true);
    return () => {
      window.removeEventListener("keydown", onReplayKeyDown, true);
      window.removeEventListener("keyup", onReplayKeyUp, true);
    };
  }, [replayOpen, replaySelecting, replayEndIndex, replayStartIndex, toggleReplayPlayback, stepReplayForward]);

  const toggleDrawingsHidden = (): void => {
    const nextHidden = !drawingsHidden;
    setDrawingsHidden(nextHidden);
    if (nextHidden) {
      setRectToolActive(false);
      setActiveTool("cursor");
      setMeasureActive(false);
      setMeasurePending(null);
      setRectPreview(null);
      setRectSnap(null);
      setTrendPreview(null);
      rectDragRef.current = null;
    }
  };
  const hideDrawings = (): void => {
    if (!drawingsHidden) toggleDrawingsHidden();
  };
  const removeDrawings = async (): Promise<void> => {
    const current = pruneNullDrawings(drawingsRef.current);
    const toRemove = current.filter((drawing) => alwaysRemoveLockedDrawings || drawing.locked !== true);
    if (toRemove.length === 0) return;
    try {
      await deleteDrawingAlertsForDrawings(symbol, toRemove.map((drawing) => drawing.id));
    } catch {
      setToolbarToast82("Could not remove drawing alerts");
      return;
    }
    pushDrawingsHistory(current);
    const removedIds = new Set(toRemove.map((drawing) => drawing.id));
    setDrawingAlerts((previous) => previous.filter((alert) => !removedIds.has(alert.drawingId)));
    setDrawings((previous) => pruneNullDrawings(previous).filter((drawing) => !removedIds.has(drawing.id)));
    showSnapshotToast(`${toRemove.length} drawing${toRemove.length === 1 ? "" : "s"} removed`);
  };
  // Ctrl temporarily forces strong magnet while a drawing tool is armed.
  const [ctrlHeld, setCtrlHeld] = useState(false);
  const ctrlHeldRef = useRef(false);
  ctrlHeldRef.current = ctrlHeld;
  const effectiveMagnetMode: MagnetMode = ctrlHeld
    ? magnetMode === "off" ? "strong" : "off"
    : magnetMode;
  useEffect(() => {
    const down = (e: KeyboardEvent): void => { if (e.key === "Control") setCtrlHeld(true); };
    const up = (): void => { setCtrlHeld(false); };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    window.addEventListener("blur", up);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
      window.removeEventListener("blur", up);
    };
  }, []);
  const [trendPreview, setTrendPreview] = useState<TrendPreview | null>(null);
  const trendPreviewRef = useRef<TrendPreview | null>(null);
  trendPreviewRef.current = trendPreview;
  const [lineHover, setLineHover] = useState<LineHover | null>(null);
  const lineHoverRef = useRef(lineHover);
  lineHoverRef.current = lineHover;
  const lineOverlayRef = useRef<LineOverlay | null>(null);
  const deselectDrawingsIfCanvasBlank = useCallback((): boolean => {
    if (rectHoverRef.current || lineHoverRef.current) return false;
    setDrawings((previous) => {
      const current = pruneNullDrawings(previous);
      return current.some((drawing) => drawing.selected)
        ? current.map((drawing) => ({ ...drawing, selected: false }))
        : current;
    });
    return true;
  }, [setDrawings]);

  // v7.4.4 — Policy Panel drawing prefs (color + widths).
  const [lineColor, setLineColor] = useState<string>(() => readLineColor());
  const [rectWidth, setRectWidth] = useState<number>(() => readLineWidth(RECT_WIDTH_KEY, 2));
  const [hlineWidth, setHlineWidth] = useState<number>(() => readLineWidth(HLINE_WIDTH_KEY, 1));
  const [vlineWidth, setVlineWidth] = useState<number>(() => readLineWidth(VLINE_WIDTH_KEY, 1));
  const [hrayWidth, setHrayWidth] = useState<number>(() => readLineWidth(HRAY_WIDTH_KEY, 1));
  const [trendWidth, setTrendWidth] = useState<number>(() => readLineWidth(TREND_WIDTH_KEY, 2));
  const persistLinePrefs = useCallback((patch: { color?: string; rectW?: number; hW?: number; vW?: number; rW?: number; tW?: number }): void => {
    try {
      if (patch.color !== undefined) { setSetting(LINE_COLOR_KEY, patch.color); setLineColor(patch.color); }
      if (patch.rectW !== undefined) { setSetting(RECT_WIDTH_KEY, String(patch.rectW)); setRectWidth(patch.rectW); }
      if (patch.hW !== undefined) { setSetting(HLINE_WIDTH_KEY, String(patch.hW)); setHlineWidth(patch.hW); }
      if (patch.vW !== undefined) { setSetting(VLINE_WIDTH_KEY, String(patch.vW)); setVlineWidth(patch.vW); }
      if (patch.rW !== undefined) { setSetting(HRAY_WIDTH_KEY, String(patch.rW)); setHrayWidth(patch.rW); }
      if (patch.tW !== undefined) { setSetting(TREND_WIDTH_KEY, String(patch.tW)); setTrendWidth(patch.tW); }
    } catch { /* ignore */ }
  }, []);
  const [policy, setPolicy] = useState<PolicyDefaults>(() => {
    (() => {
      const raw = getSetting("vora.policy.M");
      if (!raw) return;
      try {
        const p = JSON.parse(raw) as Record<string, unknown>;
        if (p.user_m != null && p.user_step == null) {
          p.user_step = p.user_m;
          delete p.user_m;
          setSetting("vora.policy.M", JSON.stringify(p));
        }
      } catch { /* ignore */ }
    })();
    try {
      const raw = getSetting("vora.policy.M");
      if (raw) {
        const saved = JSON.parse(raw) as Record<string, unknown>;
        const merged = { ...DEFAULT_POLICY, ...saved } as unknown as Record<string, unknown>;
        delete merged.use_dukascopy_proxy;
        if (getSetting(LOCAL_PROXY_DEFAULT_MIGRATION_KEY) !== "1") {
          merged.use_local_proxy = true;
          setSetting(LOCAL_PROXY_DEFAULT_MIGRATION_KEY, "1");
        }
        if (typeof merged.use_local_proxy !== "boolean") merged.use_local_proxy = true;
        if (merged.proxy_protocol !== "socks5" && merged.proxy_protocol !== "http") {
          merged.proxy_protocol = "socks5";
        }
        if (typeof merged.proxy_host !== "string") merged.proxy_host = "127.0.0.1";
        if (
          typeof merged.proxy_port !== "number" ||
          !Number.isInteger(merged.proxy_port) ||
          merged.proxy_port < 1 ||
          merged.proxy_port > 65535
        ) {
          merged.proxy_port = 10808;
        }
        if ("chain_extend" in merged) delete merged.chain_extend;
        if ("box_extend_candles" in merged && !("box_extend" in merged)) {
          merged.box_extend = merged.box_extend_candles;
          delete merged.box_extend_candles;
        }
        // v6.2.1: migrate box_extend number → {left,right}
        {
          const s = saved as Record<string, unknown>;
          void s;
          const be = merged.box_extend;
          if (typeof be === "number") {
            merged.box_extend = { left: be, right: be };
          } else if (be && typeof be === "object") {
            const o = be as { left?: unknown; right?: unknown };
            merged.box_extend = {
              left:  typeof o.left  === "number" ? o.left  : 0,
              right: typeof o.right === "number" ? o.right : 0,
            };
          } else {
            merged.box_extend = { left: 0, right: 0 };
          }
          const clampBE = merged.box_extend as { left: number; right: number };
          clampBE.left = Math.max(-20, Math.min(20, clampBE.left));
          clampBE.right = Math.max(-20, Math.min(20, clampBE.right));
          if (typeof merged.show_engulf_zone !== "boolean") {
            merged.show_engulf_zone = true;
          }
          {
            const s = saved as Record<string, unknown>;
            if (typeof s.extend_to_engulf !== "boolean") {
              s.extend_to_engulf = false;
            }
            if ("show_all_scales" in merged) delete merged.show_all_scales;
            merged.extend_to_engulf = s.extend_to_engulf;
          }
          {
            const s = saved as Record<string, unknown>;
            if (typeof s.z0_scale !== "number") {
              s.z0_scale = 5;
              merged.z0_scale = 5;
            } else {
              const v = Math.max(0, Math.min(9, Math.round(s.z0_scale as number)));
              merged.z0_scale = v;
            }
          }
          // v6.3.0d: clamp persisted user_step to 0..10 (x5 ladder, 11 steps)
          {
            const s = saved as Record<string, unknown>;
            const rawStep = (s.user_step ?? s.user_m ?? DEFAULT_POLICY.user_step) as unknown;
            if (typeof rawStep === "number" && Number.isFinite(rawStep)) {
              merged.user_step = Math.max(0, Math.min(10, Math.round(rawStep)));
            } else {
              merged.user_step = DEFAULT_POLICY.user_step;
            }
            if ("user_m" in merged) delete merged.user_m;
          }
        }
        // v6.2.1: drop all 7 obsolete detection keys + legacy aliases.
        for (const k of ["base_k", "pivot_k", "min_leg1_pct", "min_leg3_pct",
                         "min_base", "max_base", "max_base_pct",
                         "min_leg1_atr", "min_leg3_atr", "max_base_atr", "band_k"]) {
          if (k in merged) delete merged[k];
        }
        return merged as unknown as PolicyDefaults;
      }
    } catch (e) {
      // eslint-disable-next-line no-console
      console.warn("[policy] load failed:", e);
    }
    return { ...DEFAULT_POLICY };
  });
  const policyRef = useRef(policy);
  policyRef.current = policy;
  const lastAppliedRef = useRef<{ cc: number; ym: number }>({ cc: DEFAULT_VISIBLE_CANDLE_COUNT, ym: 4000 });
  const recentlyAppliedRef = useRef<number>(0);
  const skipNextApplyRef = useRef(false);
  // MKT-6b-6.3.0e: race guard — only latest candle fetch applies
  const reqIdRef = useRef<number>(0);
  // MKT-6b-6.3.0f: separate monotonic id for detect (backend cancel token)
  const detectReqIdRef = useRef<number>(0);

  const atrValue = useMemo(() => calculateATR(candles, 14), [candles]);
  atrValueRef.current = atrValue ?? 0;
  const handleChartReady = useCallback(
    (chart: IChartApi, series: ZigzagSeriesRef) => {
      chartRef.current = chart;
      seriesRef.current = series;

      // Attach the box overlay primitive (MKT-6b-3.4)
      const overlay = new BoxOverlay(chart, series);
      overlay.setExtend(policyRef.current?.box_extend ?? { left: 0, right: 0 });
      overlay.setShowEngulfZone(policyRef.current?.show_engulf_zone ?? true);
      boxOverlayRef.current = overlay;
      series.attachPrimitive(overlay);
      const m = new MeasureOverlay(chart, series as unknown as import("lightweight-charts").ISeriesApi<import("lightweight-charts").SeriesType>);
      series.attachPrimitive(m as unknown as Parameters<import("@/lib/market/CandlestickChart").ZigzagSeriesRef["attachPrimitive"]>[0]);
      measureRef.current = m;
      const r = new RectangleOverlay(chart, series as unknown as import("lightweight-charts").ISeriesApi<import("lightweight-charts").SeriesType>);
      series.attachPrimitive(r as unknown as Parameters<import("@/lib/market/CandlestickChart").ZigzagSeriesRef["attachPrimitive"]>[0]);
      rectOverlayRef.current = r;
      r.setDrawings(drawingsRef.current.filter(Boolean).filter((d): d is DrawingRect => d.type === "rectangle"));
      r.setPreview(null);
      r.setSnap(null);
      const graded = new GradedRectangleOverlay(
        chart,
        series as unknown as import("lightweight-charts").ISeriesApi<import("lightweight-charts").SeriesType>,
      );
      series.attachPrimitive(graded as unknown as Parameters<import("@/lib/market/CandlestickChart").ZigzagSeriesRef["attachPrimitive"]>[0]);
      gradedRectOverlayRef.current = graded;
      graded.setData(
        drawingsRef.current.filter((drawing): drawing is GradedRectangle => drawing.type === "graded-rectangle"),
        candlesRef.current,
      );
      // v7.4.4 — line overlay alongside the rect overlay.
      const lo = new LineOverlay(chart, series as unknown as import("lightweight-charts").ISeriesApi<import("lightweight-charts").SeriesType>);
      series.attachPrimitive(lo as unknown as Parameters<import("@/lib/market/CandlestickChart").ZigzagSeriesRef["attachPrimitive"]>[0]);
      lineOverlayRef.current = lo;
      lo.setDrawings(drawingsRef.current);
      lo.setPreview(null);
      lo.setHover(null);
      // v0.83.8 Part E (Suspect #5): apply the visual right offset AFTER
      // mount. Creation uses rightOffset: 0 (see CandlestickChart) so
      // LC's internal setData clamp has no margin bias; restores handle
      // 0 symmetrically via temp-zero. This fires once per chart mount,
      // before the initial template/verified apply, so all captures
      // record rightOffset 5 and restores stay symmetric.
      try {
        chart.timeScale().applyOptions({ rightOffset: 5 });
      } catch { /* ignore */ }
      setChartReady(true);
    },
    [],
  );

  // MKT-6b-6.3.0e: candle fetch on debouncedM + race guard (fixes rapid-arrow hang)
  // v0.83.5: capture the viewport BEFORE setCandles and apply it AFTER so
  // the timeframe switch preserves X+Y zoom (no per-timeframe default zoom,
  // no jump to the last candle).
  const fetchCandles = useCallback(async (interval: string, myId: number, anchorTsSec?: number) => {
    // v0.83.7 Part E — manual-gesture unsubscriber (outer scope so both
    // the success and error paths can clean up).
    let unsubManual: (() => void) | null = null;
    // v0.83.5 — capture BEFORE the fetch resolves; the chart still shows
    // the previous timeframe's candles so the viewport is meaningful.
    const chartBeforeFetch = chartRef.current;
    const replayCutTimestamp = replayTimeframeRestoreRef.current?.endTimestamp;
    const replayCutIndex = replayCutTimestamp == null
      ? null
      : replayIndexAtOrAfter(candlesRef.current, replayCutTimestamp);
    const replayVisibleAnchor = replayCutIndex !== null && replayCutIndex > 0
      ? candlesRef.current[replayCutIndex - 1]
      : undefined;
    const replayAnchorTimestamp = replayVisibleAnchor?.ts;
    const replayAnchorCoordinate = chartBeforeFetch && replayAnchorTimestamp != null
      ? replayTimestampToCoordinate(chartBeforeFetch, candlesRef.current, replayAnchorTimestamp)
      : null;
    const plotWidth = chartBeforeFetch?.timeScale().width() ?? 0;
    const replayAnchorXRatio = replayAnchorCoordinate !== null && plotWidth > 0
      ? Math.max(0, Math.min(1, replayAnchorCoordinate / plotWidth))
      : undefined;
    const captured = chartBeforeFetch
      ? captureTfViewport(chartBeforeFetch, replayAnchorXRatio)
      : null;
    if (captured && replayAnchorTimestamp != null) captured.anchorTs = replayAnchorTimestamp;
    if (captured) viewportToRestoreRef.current = captured;
    setPreserveViewport(true);
    setLoading(true);
    setError(null);
    // Safety unlock — if fetch stalls > 20s, re-enable arrows
    const safety = setTimeout(() => {
      if (myId === reqIdRef.current) setLoading(false);
    }, 20_000);
    try {
      // Fetch the latest online candles, or end just after the restore anchor.
      const LIMIT = 5000;
      const halfWindowSec = Math.floor((LIMIT / 2) * intervalWindowSeconds(interval));
      const endTimestamp = typeof anchorTsSec === "number" && anchorTsSec > 0
        ? anchorTsSec + halfWindowSec
        : Math.floor(Date.now() / 1000);
      const res = await mktFetchHistoricalDuka({
        symbol,
        timeframe: interval,
        endTimestamp,
        limit: LIMIT,
      });
      const lastCandle = res[res.length - 1];
      if (lastCandle && Number.isFinite(lastCandle.close) && lastCandle.close > 0) {
        try {
          setSetting(
            `${OFFLINE_REFERENCE_PRICE_PREFIX}${symbol}`,
            String(lastCandle.close),
          );
        } catch (storageError) {
          console.warn("[chart] could not save offline drawing reference price:", storageError);
        }
      }
      // v0.83.9 Part C — DEV-only anchor-in-window assertion.
      if (import.meta.env.DEV && typeof anchorTsSec === "number") {
        const firstTs = res[0]?.ts;
        const lastTs = res[res.length - 1]?.ts;
        if (
          typeof firstTs === "number" &&
          typeof lastTs === "number" &&
          (anchorTsSec < firstTs || anchorTsSec > lastTs)
        ) {
          console.warn(
            "[fetch] anchor not in returned window",
            { anchorTsSec, firstTs, lastTs, count: res.length },
          );
        }
      }
      if (myId !== reqIdRef.current) return;
      // Record the full interval key before setCandles so stale results are ignored.
      // setCandles so Effect A's stale-M guard can validate the match.
      lastFetchedIntervalRef.current = interval;
      const replayRestore = replayTimeframeRestoreRef.current;
      if (replayRestore) {
        const startIndex = replayRestore.startTimestamp === null
          ? null
          : replayIndexAtOrAfter(res, replayRestore.startTimestamp);
        const endIndex = replayRestore.endTimestamp === null
          ? null
          : replayIndexAtOrAfter(res, replayRestore.endTimestamp);
        replayTimestampOverridesRef.current = {
          start: replayRestore.startTimestamp,
          end: replayRestore.endTimestamp,
        };
        const remappedVisibleAnchor = endIndex !== null && endIndex > 0 ? res[endIndex - 1] : undefined;
        if (remappedVisibleAnchor && viewportToRestoreRef.current) {
          viewportToRestoreRef.current.anchorTs = remappedVisibleAnchor.ts;
        }
        replaySkipViewportFollowRef.current = true;
        replayCursorRef.current = endIndex;
        setReplayStartIndex(startIndex);
        setReplayEndIndex(endIndex);
        setReplayPlaying(replayRestore.wasPlaying && endIndex !== null && endIndex < res.length - 1);
        replayTimeframeRestoreRef.current = null;
      }
      // v0.83.7 Part E — mark the timeframe change in flight BEFORE
      // setCandles so the template effect skips applyTemplateAtr.
      // If the user manually zooms/pans during the window, release
      // immediately so their gesture wins.
      timeframeChangeInFlightRef.current = true;
      // v0.83.7 Part E — programmatic applies run synchronously inside
      // this window; only a USER gesture (flagged by LC via a visible
      // range change outside applyViewportVerified) releases early.
      // Track programmatic depth so our own setVisibleRange calls do
      // not cancel the guard.
      let programmaticDepth = 0;
      try {
        const c = chartRef.current;
        if (c) {
          const release = (): void => {
            if (programmaticDepth === 0) {
              timeframeChangeInFlightRef.current = false;
            }
          };
          try {
            c.timeScale().subscribeVisibleLogicalRangeChange(release);
            unsubManual = () => {
              try {
                c.timeScale().unsubscribeVisibleLogicalRangeChange(release);
              } catch { /* ignore */ }
            };
          } catch { /* ignore */ }
        }
      } catch { /* ignore */ }
      // v0.83.8 Part A (Suspect #1) + Part D (Suspect #4): bracket
      // setCandles AND the verified apply inside programmaticDepth so
      // the setData sync event cannot release the in-flight guard, and
      // stamp recentlyAppliedRef so the zoom-sync echo stays quiet.
      programmaticDepth++;
      try {
        recentlyAppliedRef.current = Date.now();
        setCandles(res);
        // v0.83.7 Part D — verified viewport application (apply, settle
        // two frames, verify anchor at center, re-apply up to 3x).
        // Arm the hold BEFORE applying so Effect A re-applies instead of
        // applyTemplateAtr. Do NOT reorder the fetch / setCandles logic.
        await new Promise<void>((resolve) =>
          requestAnimationFrame(() =>
            requestAnimationFrame(() => resolve()),
          ),
        );
        const vp = viewportToRestoreRef.current;
        if (vp && chartRef.current) {
          timeframeHoldRef.current = { viewport: vp, expiry: Date.now() + 500 };
          await applyTfViewportVerified(chartRef.current, vp);
          viewportToRestoreRef.current = null;
        }
      } finally {
        programmaticDepth--;
      }
      // v0.83.7 Part E — release the in-flight guard one frame after
      // the verified apply so synchronously-queued followers also skip.
      try { unsubManual?.(); } catch { /* ignore */ }
      requestAnimationFrame(() => {
        timeframeChangeInFlightRef.current = false;
        setPreserveViewport(false);
      });
    } catch (e) {
      if (myId !== reqIdRef.current) return;
      replayTimeframeRestoreRef.current = null;
      replayTimestampOverridesRef.current = null;
      setError(String(e));
      setCandles([]);
      viewportToRestoreRef.current = null;
      timeframeChangeInFlightRef.current = false;
      try { unsubManual?.(); } catch { /* ignore */ }
      requestAnimationFrame(() => setPreserveViewport(false));
      // zoom reset happens via [currentM] effect above
    } finally {
      clearTimeout(safety);
      if (myId === reqIdRef.current) {
        setLoading(false);
      }
    }
  }, [symbol]);

  useEffect(() => {
    const pendingSnapshotFetch = snapshotFetchSuppressionRef.current;
    if (pendingSnapshotFetch && pendingSnapshotFetch.symbol === symbol) {
      if (pendingSnapshotFetch.interval === debouncedInterval) {
        snapshotFetchSuppressionRef.current = null;
      }
      return;
    }
    const myId = ++reqIdRef.current;
    let cancelled = false;
    // v0.83.9 B4 — capture the current viewport BEFORE the fetch and pass
    // its anchorTs so fetchCandles can center from_ts/to_ts on it.
    // (fetchCandles still re-captures for the restore apply; this capture
    // only feeds the window computation.)
    const captured = chartRef.current
      ? captureTfViewport(chartRef.current)
      : null;
    const anchorTsSec =
      captured && typeof captured.anchorTs === "number"
        ? captured.anchorTs
        : undefined;
    void (async () => {
      if (cancelled) return;
      await fetchCandles(debouncedInterval, myId, anchorTsSec);
    })();
    return () => {
      cancelled = true;
    };
  }, [debouncedInterval, fetchCandles]);

  // v0.83.2 — restore on mount when ?snapshot=<id>.
  useEffect(() => {
    const sid = snapshotIdFromUrl;
    if (!sid) {
      restoredRef.current = null;
      snapshotRestoreRequestRef.current += 1;
      snapshotFetchSuppressionRef.current = null;
      setRestoreOverlay({ visible: false, name: "" });
      setPreserveViewport(false);
      restoreHoldRef.current = null;
      return;
    }
    if (restoredRef.current === sid || !chartReady || !chartRef.current) return;
    restoredRef.current = sid;
    const restoreRequestId = ++snapshotRestoreRequestRef.current;
    const isCurrentRestore = (): boolean => snapshotRestoreRequestRef.current === restoreRequestId;
    const chart = chartRef.current;
    void (async () => {
      setNarrativePlaybackSnapshot(null);
      setRestoreOverlay({ visible: true, name: "" });
      // v0.83.4: block fitContent before setCandles so the child's
      // [candles] effect doesn't reset the viewport we're restoring.
      setPreserveViewport(true);
      try {
        const snap = await loadSnapshot(sid);
        if (!isCurrentRestore()) return;
        if (!snap) { showSnapshotToast("Snapshot not found"); restoredRef.current = null; return; }
        try { deserialize(JSON.stringify(snap)); }
        catch { showSnapshotToast("Snapshot incompatible"); return; }
        setRestoreOverlay({ visible: true, name: snap.name });
        const targetSymbol = snap.chart.symbol.trim();
        if (!targetSymbol) { showSnapshotToast("Restore failed: unsupported symbol"); return; }
        const targetInterval = parseSnapshotInterval(snap.chart.interval);
        if (targetInterval === null) { showSnapshotToast("Restore failed: unsupported interval"); return; }
        const targetM = nearestMScaleStep(targetInterval);
        const rows = await mktFetchHistoricalDuka({
          symbol: targetSymbol,
          timeframe: targetInterval,
          endTimestamp: Math.floor(Date.now() / 1000),
          limit: 5000,
        });
        if (!isCurrentRestore()) return;
        if (!rows || rows.length === 0) {
          showSnapshotToast("Restore failed: No candles");
          return;
        }
        pendingSnapshotRegistryRef.current = { symbol: targetSymbol, scales: snap.policy.registry };
        snapshotFetchSuppressionRef.current = targetSymbol !== symbol || targetInterval !== activeInterval
          ? { symbol: targetSymbol, interval: targetInterval }
          : null;
        if (targetSymbol !== symbol) {
          skipSnapshotDrawingHydrateRef.current = true;
          setSymbol(targetSymbol);
        }
        selectInterval(targetInterval, false);
        if (snap.chart.displayMode === "candlestick" || snap.chart.displayMode === "special") setChartDisplayMode(snap.chart.displayMode);
        // v0.83.8 Part D (Suspect #4): stamp before setCandles so the
        // zoom-sync subscriber ignores the setData-induced event.
        recentlyAppliedRef.current = Date.now();
        setCandles(rows);
        const conv = snapshotToDrawings(snap.drawings || []);
        drawingsRef.current = conv;
        setDrawings(conv);
        const roles = snap.policy.roles;
        setPolicy((p) => ({ ...p, user_step: targetM, s_small: roles.small, s_medium: roles.medium, s_large: roles.large }));
        const regSnap = snap.policy.registry;
        setScaleRegistry((prev) => {
          if (!prev) return prev;
          return mergeSnapshotScaleRegistry(prev, regSnap);
        });
        try {
          chart.applyOptions({ layout: { background: { color: snap.theme.background }, textColor: snap.theme.textColor }, grid: { vertLines: { color: snap.theme.gridColor }, horzLines: { color: snap.theme.gridColor } } } as unknown as Parameters<IChartApi["applyOptions"]>[0]);
        } catch { /* ignore */ }
        // Wait for React to commit the new candles AND for
        // CandlestickChart's own effect to run series.setData(...).
        // Two frames is the safest.
        await new Promise<void>((resolve) =>
          requestAnimationFrame(() =>
            requestAnimationFrame(() => resolve()),
          ),
        );
        if (!isCurrentRestore()) return;
        // v0.83.4 Part G: skip apply when no viewport was saved.
        const v = snap.viewport;
        const hasViewport =
          (typeof v.fromLogical === "number" && typeof v.toLogical === "number" &&
            (v.fromLogical !== 0 || v.toLogical !== 0)) ||
          (typeof v.fromTs === "number" && typeof v.toTs === "number" &&
            v.fromTs > 0 && v.toTs > 0);
        if (hasViewport && chartRef.current) {
          // Arm the hold BEFORE applying so the debounced refetch's
          // Effect A re-applies the restored viewport instead of the
          // template default.
          // v0.83.7 Part D — verified restore: applyViewportVerified
          // (up to 3x with anchor verification); keep the other
          // concerns (drawings, policy, theme) unchanged.
          restoreHoldRef.current = { viewport: v, expiry: Date.now() + 500 };
          recentlyAppliedRef.current = Date.now();
          await applyTfViewportVerified(
            chartRef.current,
            snapshotToCapturedViewport(v),
          );
          if (!isCurrentRestore()) return;
        }
        if (playNarrativeFromUrl && snap.narrativeMode && snap.narrativeLog) {
          setNarrativePlaybackBaseDrawings(conv.map((drawing) => ({ ...drawing })));
          setNarrativePlaybackSnapshot(snap);
        }
        try { setSetting("vora.snapshots.last", snap.id); } catch { /* ignore */ }
        showSnapshotToast("Restored: " + snap.name);
        // One more frame, then release preserveViewport so future
        // timeframe changes go back to fitContent.
        requestAnimationFrame(() => {
          setPreserveViewport(false);
        });
      } catch (e: unknown) {
        if (!isCurrentRestore()) return;
        snapshotFetchSuppressionRef.current = null;
        showSnapshotToast("Restore error: " + (e instanceof Error ? e.message : String(e)));
        setPreserveViewport(false);
        restoreHoldRef.current = null;
      } finally {
        if (isCurrentRestore()) {
          setRestoreOverlay({ visible: false, name: "" });
          setPreserveViewport(false);
          setSearchParams((current) => {
            const next = new URLSearchParams(current);
            next.delete("snapshot");
            return next;
          }, { replace: true });
        }
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [chartReady, snapshotIdFromUrl, symbol, playNarrativeFromUrl, setSearchParams]);

  // v6.4.18: pivot inspector — fetch engine pivots for current M.
  // DISABLED in v0.83.1: heavy per-timeframe scale scan.
  // Do not re-enable without a hardware upgrade plan.
  useEffect(() => {
    if (!ENABLE_SCALE_RECALC && activeTool !== "graded-rectangle") {
      return;
    }
    let cancelled = false;
    const mVal = Math.max(0, Math.min(10, Math.round(debouncedM)));
    void (async () => {
      try {
        const ps = await listPivots(symbol, mVal);
        if (cancelled) return;
        setPivots(ps);
        const w = window as unknown as { __vora?: Record<string, unknown> };
        w.__vora = { ...(w.__vora ?? {}), pivots: ps };
      } catch (error: unknown) {
        if (!cancelled) {
          setPivots([]);
          console.error("Could not load swing pivots for the Graded Rectangle tool:", error);
        }
      }
    })();
    return () => { cancelled = true; };
  }, [activeTool, debouncedM, symbol]);

  useEffect(() => { pivotsRef.current = pivots; }, [pivots]);

  // ─── Box detection (MKT-6b-3.4) + policy tuning (MKT-6b-3.6) ───

  useEffect(() => {
    try {
      setSetting("vora.policy.M", JSON.stringify(policy));
    } catch (e) {
      // eslint-disable-next-line no-console
      console.warn("[policy] save failed:", e);
    }
  }, [policy]);

  // Sync S-scale roles when M changes (v6.3.0g: identity-preserving)
  useEffect(() => {
    setPolicy((p) => {
      const [s, md, l] = M_ROLE_DEFAULTS[currentM] ?? [4, 5, 6];
      if (
        p.s_small === s &&
        p.s_medium === md &&
        p.s_large === l &&
        p.user_step === currentM
      ) {
        return p; // same ref → React bails out
      }
      return { ...p, s_small: s, s_medium: md, s_large: l, user_step: currentM };
    });
  }, [currentM]);

  // MKT-6b-6.3.0d: slider (policy.user_step) → single source of truth (currentM)
  useEffect(() => {
    const v = Math.max(0, Math.min(10, Math.round(policy.user_step)));
    if (v !== currentM) selectInterval(`M${v}`);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [policy.user_step]);

  // v6.3.0g: composite detect key — debounce keyed to this single string
  // so ONLY this dep triggers re-detect.
  const detectKey = useMemo(
    () =>
      JSON.stringify({
        m: debouncedM,
        s_small: policy.s_small,
        s_medium: policy.s_medium,
        s_large: policy.s_large,
        z0_scale: policy.z0_scale,
        extend_to_engulf: policy.extend_to_engulf,
      }),
    [
      debouncedM,
      policy.s_small,
      policy.s_medium,
      policy.s_large,
      policy.z0_scale,
      policy.extend_to_engulf,
    ],
  );

  const debouncedDetectKey = useDebounced(detectKey, 300);

  // Fetch boxes on composite key change (v6.3.0g: single dep)
  // NOTE: box_extend_candles is frontend-only render shift — must NOT trigger IPC
  // v7.2.0 Part A — when autoDetect is OFF, skip the pipeline entirely + clear.
  // DISABLED in v0.83.1: heavy per-timeframe scale scan.
  // Do not re-enable without a hardware upgrade plan.
  useEffect(() => {
    if (!ENABLE_SCALE_RECALC) {
      // v0.83.1: timeframe change must NOT trigger RBR/DBD/scale scan.
      // Keep existing boxes (coloring still applies via visibleBoxes memo).
      // Candle fetch (above) and drawings load are untouched.
      return;
    }
    if (!autoDetect) {
      setBoxes([]);
      setChains([]);
      return;
    }
    const cfg = JSON.parse(debouncedDetectKey) as {
      m: number;
      s_small: number;
      s_medium: number;
      s_large: number;
      z0_scale: number;
      extend_to_engulf: boolean;
    };
    const prevId = detectReqIdRef.current;
    const myId = prevId + 1;
    detectReqIdRef.current = myId;
    let cancelled = false;
    (async () => {
      try {
        const res = await detectBoxes(symbol, cfg.m, {
          user_step:       cfg.m,
          s_small:      cfg.s_small,
          s_medium:     cfg.s_medium,
          s_large:      cfg.s_large,
          z0_scale:     cfg.z0_scale,
          extend_to_engulf: cfg.extend_to_engulf,
        }, {
          requestId: myId,
          cancelPrevious: prevId > 0 ? prevId : undefined,
        });
        if (cancelled) return;
        setBoxes(res.boxes);
        setChains(res.chains);
        if (import.meta.env.DEV) {
          console.log(
            `[boxes] M${cfg.m}: ${res.boxes.length} boxes, ${res.chains.length} chains`
          );
          (window as unknown as { __vora?: Record<string, unknown> }).__vora = {
            ...((window as unknown as { __vora?: Record<string, unknown> }).__vora as Record<string, unknown>),
            boxes: res.boxes,
            chains: res.chains,
            userStep: cfg.m,
            ts: Date.now(),
          };
        }
      } catch (e) {
        const msg = String(e);
        // v6.3.0f: cancellation is expected — silence it.
        if (!msg.includes("cancelled")) {
          console.warn("[boxes] detect failed:", e);
        }
        if (!cancelled && !msg.includes("cancelled")) {
          setBoxes([]);
          setChains([]);
        }
      }
    })();

    return () => {
      cancelled = true;
      // v6.3.0f: also cancel backend-side when effect tears down.
      void cancelRequest(myId);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [debouncedDetectKey, detectRefreshKey, autoDetect, symbol]);

  // v6.4.4: re-classify boxes by registry ranges + filter by `enabled`.
  // Chains keep RAW addresses (setChains unchanged); overlay skips chains
  // whose boxes were filtered (BoxOverlay.setBoxes precomputes roots only
  // from visible boxes, no crash on missing members).
  const visibleBoxes = useMemo(() => {
    if (!scaleRegistry) return boxes;
    const scales = scaleRegistry.scales;
    // v6.4.11: snap boxes to the debouncedM (user_step) grid BEFORE
    // reclassifyScale + enabled filtering. PRESERVES existing logic below.
    const gridSec = TF_SECONDS_TS[debouncedM] ?? 1;
    const snapped = boxes.map((b) => {
      const tsStart = b.ts_start ?? 0;
      const s = Math.floor(tsStart / gridSec) * gridSec;
      const xIdx = Math.floor(s / gridSec);
      return { ...b, ts_start: s, x_idx: xIdx, x: xIdx };
    });
    const out: typeof boxes = [];
    for (const b of snapped) {
      // v6.4.9: base_pct = (high - low) / origin_price * 100 (matches backend base_h).
      const basePct =
        b.origin_price > 0 ? ((b.high - b.low) / b.origin_price) * 100 : 0;
      const idx = reclassifyScale(basePct, scales);
      if (idx == null) continue;
      if (!scales[idx].enabled) continue;
      out.push({ ...b, scale: idx });
    }
    return out;
  }, [boxes, scaleRegistry, debouncedM]);

  // v6.4.8-diag: richer debug helper — dump raw boxes in [min,max] base_pct range.
  // v6.4.9: SAME base_pct formula as visibleBoxes: (high - low) / origin_price * 100.
  useEffect(() => {
    const w = window as unknown as { __vora?: Record<string, unknown> };
    const existing = w.__vora ?? {};
    w.__vora = {
      ...existing,
      debugScales: (minPct?: number, maxPct?: number) => {
        if (!scaleRegistry) {
          console.log("[debugScales] registry not loaded");
          return;
        }
        const scales = scaleRegistry.scales;
        const minP = minPct ?? -Infinity;
        const maxP = maxPct ?? Infinity;

        const stepSecs = [
          1, 5, 25, 125, 625, 3125, 15625, 78125,
          390625, 1953125, 9765625,
        ];

        // --- SCALES ---
        console.log("=== SCALES ===");
        scales.forEach((s, i) => {
          console.log(
            `  [${i}] ${s.label} [${s.start_pct} .. ${s.end_pct}) ` +
            `enabled=${s.enabled} color=${s.color}`,
          );
        });

        // --- helper (v6.4.9: SAME formula as visibleBoxes) ---
        const basePctOf = (b: BoxDto): number =>
          b.origin_price > 0
            ? ((b.high - b.low) / b.origin_price) * 100
            : 0;

        // --- raw vs visible counts ---
        console.log(`=== RAW boxes: ${boxes.length} ===`);
        console.log(`=== VISIBLE boxes: ${visibleBoxes.length} ===`);
        const rawByStep: Record<number, number> = {};
        const visByStep: Record<number, number> = {};
        for (const b of boxes) {
          const s = b.pivot_step ?? -1;
          rawByStep[s] = (rawByStep[s] || 0) + 1;
        }
        for (const b of visibleBoxes) {
          const s = b.pivot_step ?? -1;
          visByStep[s] = (visByStep[s] || 0) + 1;
        }
        console.log("raw per pivot_step:", rawByStep);
        console.log("visible per pivot_step:", visByStep);

        // --- filtered dump ---
        console.log(`=== BOXES in [${minP} .. ${maxP}] ===`);
        let matched = 0, unmatched = 0, disabled = 0;
        const scaleCounts: Record<string, number> = {};

        for (const b of boxes) {
          const bp = basePctOf(b);
          if (bp < minP || bp > maxP) continue;

          const idx = reclassifyScale(bp, scales);
          const label = idx == null ? "UNMATCHED" : scales[idx].label;
          const enabled = idx == null ? "-" : String(scales[idx].enabled);

          const sIdx = b.pivot_step ?? 0;
          const secs = stepSecs[sIdx] ?? 1;
          const tsStart = b.ts_start ?? 0;
          const tsEnd = b.ts_end ?? 0;
          const boxWidthCandles = Math.round((tsEnd - tsStart) / secs);
          const xIdx = b.x_idx ?? "n/a";

          console.log(
            `  addr=${b.address} x_idx=${xIdx} ` +
            `basePct=${bp.toFixed(4)}% matched=${label} ` +
            `enabled=${enabled} step=${sIdx} ` +
            `ts=${tsStart}..${tsEnd} ` +
            `box_candles=${boxWidthCandles} ` +
            `high=${b.high} low=${b.low} ` +
            `leg2=${b.leg2_magnitude} ` +
            `origin=${b.origin_price} ` +
            `backend_scale=${b.scale}`,
          );

          scaleCounts[label] = (scaleCounts[label] || 0) + 1;
          if (idx == null) unmatched++;
          else if (!scales[idx].enabled) disabled++;
          else matched++;
        }

        console.log("=== SUMMARY ===");
        console.log("matched (rendered):", matched);
        console.log("disabled (skipped):", disabled);
        console.log("unmatched (no scale):", unmatched);
        console.log("count per scale:", scaleCounts);
        console.log("=== END debugScales ===");
      },
    };
  }, [scaleRegistry, boxes, visibleBoxes]);

  // Push boxes to overlay (waits for chartReady)
  useEffect(() => {
    const overlay = boxOverlayRef.current;
    if (!overlay || !chartReady) return;
    overlay.setBoxes(visibleBoxes);
  }, [visibleBoxes, chartReady]);

  // Push box extend (fractional per-edge) to overlay — frontend-only, no IPC (MKT-6b-5.1.9.1)
  useEffect(() => {
    const overlay = boxOverlayRef.current;
    if (!overlay || !chartReady) return;
    overlay.setExtend(policy.box_extend ?? { left: 0, right: 0 });
  }, [policy.box_extend.left, policy.box_extend.right, chartReady]);

  useEffect(() => {
    const overlay = boxOverlayRef.current;
    if (!overlay || !chartReady) return;
    overlay.setShowEngulfZone(policy.show_engulf_zone ?? true);
  }, [policy.show_engulf_zone, chartReady]);

  // Load scale registry once per symbol (v6.4.9: global registry, PK=symbol).
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const reg = await getScaleRegistry(symbol);
        if (!cancelled) {
          const pendingSnapshotRegistry = pendingSnapshotRegistryRef.current;
          const restoredRegistry = pendingSnapshotRegistry?.symbol === symbol
            ? mergeSnapshotScaleRegistry(reg, pendingSnapshotRegistry.scales)
            : reg;
          if (pendingSnapshotRegistry?.symbol === symbol) pendingSnapshotRegistryRef.current = null;
          setScaleRegistry(restoredRegistry);
          setSaveOk(false);
          setSaveError(null);
        }
      } catch (e) {
        // eslint-disable-next-line no-console
        console.warn("[scales] load failed:", e);
        if (!cancelled) {
          if (pendingSnapshotRegistryRef.current?.symbol === symbol) pendingSnapshotRegistryRef.current = null;
          setScaleRegistry(null);
        }
      }
    })();
    return () => { cancelled = true; };
  }, [registryRefreshKey, symbol]);

  const onScaleChange = useCallback((i: number, patch: Partial<Scale>) => {
    setScaleRegistry((r) => {
      if (!r) return r;
      const scales = r.scales.slice();
      scales[i] = { ...scales[i], ...patch };
      return { ...r, scales };
    });
    setSaveOk(false);
  }, []);

  const onAddScale = useCallback(() => {
    setScaleRegistry((r) => {
      if (!r || r.scales.length >= 20) return r;
      const n = r.scales.length;
      const last = r.scales[n - 1];
      const step = Math.max(0.001, last.end_pct - last.start_pct);
      const start = last.end_pct;
      const end = start + step;
      const scales = r.scales.slice();
      scales.push({
        id: `S${n}`,
        label: `S${n}`,
        start_pct: +start.toFixed(3),
        end_pct: +end.toFixed(3),
        color: SCALE_PALETTE[n % SCALE_PALETTE.length],
        enabled: true,
        sort_order: n,
      });
      return { ...r, scales };
    });
    setSaveOk(false);
  }, []);

  const onRemoveScale = useCallback((i: number) => {
    setScaleRegistry((r) => {
      if (!r || r.scales.length <= 1) return r;
      const scales = r.scales.slice();
      scales.splice(i, 1);
      // RELABEL — S0..S{n-1} sequential. Existing saved boxes with old IDs
      // will be stale until re-detect (user must Save + change step to re-run detection).
      const relabeled = scales.map((s, idx) => ({
        ...s,
        id: `S${idx}`,
        label: `S${idx}`,
        sort_order: idx,
      }));
      return { ...r, scales: relabeled };
    });
    setSaveOk(false);
    // eslint-disable-next-line no-console
    console.warn("[scales] removed — Save Registry + re-detect to refresh boxes");
  }, []);

  const validationError = useMemo(() => {
    if (!scaleRegistry) return null;
    const enabled = scaleRegistry.scales.filter((s) => s.enabled);

    // Per-scale: end must be > start
    for (const s of enabled) {
      if (s.end_pct <= s.start_pct) {
        return (
          `${s.label} [${s.start_pct} .. ${s.end_pct}]: ` +
          `end_pct must be > start_pct`
        );
      }
    }

    // Cross-scale: no overlap (sorted by start)
    const sorted = [...enabled].sort(
      (a, b) => a.start_pct - b.start_pct,
    );
    for (let i = 1; i < sorted.length; i++) {
      const prev = sorted[i - 1];
      const cur = sorted[i];
      if (cur.start_pct < prev.end_pct) {
        return (
          `Overlap: ${prev.label} ` +
          `[${prev.start_pct} .. ${prev.end_pct}) ` +
          `overlaps ${cur.label} ` +
          `[${cur.start_pct} .. ${cur.end_pct})`
        );
      }
    }
    return null;
  }, [scaleRegistry]);

  const onSaveRegistry = useCallback(async () => {
    if (!scaleRegistry || scaleRegistry.symbol !== symbol || isSaving || validationError) return;
    setIsSaving(true);
    setSaveError(null);
    try {
      await persistScaleRegistry(scaleRegistry);
      setSaveOk(true);
      setRegistryRefreshKey((k) => k + 1);
      setDetectRefreshKey((k) => k + 1);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("[scale-save]", e);
      setSaveError(String(e));
    } finally {
      setIsSaving(false);
    }
  }, [scaleRegistry, isSaving, validationError, symbol]);

  // v6.4.1: registry → overlay colors (key = sort_order else index)
  const registryColors = useMemo(() => {
    const m = new Map<number, string>();
    if (!scaleRegistry) return m;
    scaleRegistry.scales.forEach((s, i) => {
      if (!s.enabled) return;
      if (!isValidHex(s.color)) return;
      const key = typeof s.sort_order === "number" ? s.sort_order : i;
      m.set(key, s.color);
    });
    return m;
  }, [scaleRegistry]);

  // Push registry colors to overlay (repaint via _requestUpdate)
  useEffect(() => {
    const overlay = boxOverlayRef.current;
    if (!overlay || !chartReady) return;
    overlay.setScaleColors(registryColors);
  }, [registryColors, chartReady]);

  // Expose scales to window.__vora for debug (v5.1.8)
  useEffect(() => {
    const w = window as unknown as { __vora?: Record<string, unknown> };
    if (!w.__vora) w.__vora = {};
    w.__vora.scales = scaleRegistry;
  }, [scaleRegistry]);

  // v6.4.7: measure v2 — time-based pending/complete, survives zoom/pan
  useEffect(() => {
    measureRef.current?.setState({
      active: measureActive,
      pending: measurePending,
      complete: measureComplete,
    });
  }, [measureActive, measurePending, measureComplete]);

  // v7.1.0: push drawings to overlay + persist to localStorage
  // v7.4.4 — rect overlay gets rects only; line overlay gets the full union.
  useEffect(() => {
    const visibleDrawings = drawingsHidden ? [] : drawings.filter((drawing) => drawing && isDrawingVisible(drawing));
    rectOverlayRef.current?.setDrawings(visibleDrawings.filter((drawing): drawing is DrawingRect => drawing.type === "rectangle"));
    gradedRectOverlayRef.current?.setData(
      visibleDrawings.filter((drawing): drawing is GradedRectangle => drawing.type === "graded-rectangle"),
      candlesRef.current,
    );
    lineOverlayRef.current?.setDrawings(visibleDrawings);
    // v7.4.4 — mid-price hint for vline handle Y (middle of visible range).
    try {
      const s = seriesRef.current as unknown as { coordinateToPrice?: (y: number) => number | null } | null;
      const h = chartContainerRef.current?.clientHeight ?? 400;
      const mid = s?.coordinateToPrice?.(h / 2);
      if (typeof mid === "number" && Number.isFinite(mid)) lineOverlayRef.current?.setMidPrice(mid);
    } catch { /* ignore */ }
    try {
      setSetting(DRAW_KEY, JSON.stringify(drawings));
    } catch { /* ignore */ }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [drawings, drawingsHidden]);

  useEffect(() => {
    lineOverlayRef.current?.setCandles(candles.map(({ ts, open, high, low, close, volume }) => ({ ts, open, high, low, close, volume })));
    gradedRectOverlayRef.current?.setData(
      drawingsRef.current.filter((drawing): drawing is GradedRectangle => drawing.type === "graded-rectangle"),
      candles,
    );
  }, [candles, chartReady]);

  // v7.1.1 — push live preview + snap marker into the rect overlay.
  useEffect(() => {
    rectOverlayRef.current?.setPreview(rectPreview ? {
      anchorTs: rectPreview.anchorTs,
      anchorPrice: rectPreview.anchorPrice,
      currentTs: rectPreview.currentTs,
      currentPrice: rectPreview.currentPrice,
    } : null);
  }, [rectPreview]);
  useEffect(() => {
    rectOverlayRef.current?.setSnap(effectiveMagnetMode !== "off" ? rectSnap : null);
  }, [rectSnap, effectiveMagnetMode]);
  // v7.2.0 C2/C3 — sync hover state into the overlay for handle visibility.
  useEffect(() => {
    rectOverlayRef.current?.setHover(rectHover);
  }, [rectHover, chartReady]);
  // v7.4.4 — sync trend preview + line hover into the line overlay.
  useEffect(() => {
    lineOverlayRef.current?.setPreview(trendPreview);
  }, [trendPreview, chartReady]);
  useEffect(() => {
    lineOverlayRef.current?.setHover(lineHover);
  }, [lineHover, chartReady]);
  // Clear the previous snap marker when Ctrl starts forcing strong magnet.
  useEffect(() => {
    if (ctrlHeld) setRectSnap(null);
  }, [ctrlHeld]);

  // v7.0.0 B: strong-magnet OHLC snap helper (30px screen-space threshold)
  const candlesRef = useRef(candles);
  candlesRef.current = candles;
  const olderFetchContextRef = useRef(`${symbol}:${debouncedInterval}`);
  olderFetchContextRef.current = `${symbol}:${debouncedInterval}`;
  const fetchOlderCandles = useCallback(async (): Promise<void> => {
    if (isFetchingOlderRef.current || loading || replayOpen) return;
    const context = `${symbol}:${debouncedInterval}`;
    const currentCandles = candlesRef.current;
    const endTimestamp = currentCandles[0]?.ts;
    if (endTimestamp === undefined || !Number.isFinite(endTimestamp)) return;
    const boundaryKey = `${context}:${endTimestamp}`;
    if (blockedOlderBoundaryRef.current === boundaryKey) return;
    const retryAfter = olderRetryAfterRef.current;
    if (retryAfter?.boundary === boundaryKey && Date.now() < retryAfter.timestamp) return;

    isFetchingOlderRef.current = true;
    setIsFetchingOlder(true);
    try {
      const olderCandles = await mktFetchHistoricalDuka({
        symbol,
        timeframe: debouncedInterval,
        endTimestamp,
        limit: 500,
      });
      if (olderFetchContextRef.current !== context) return;
      const uniqueOlder = olderCandles.filter((candle) => candle.ts < endTimestamp);
      if (uniqueOlder.length === 0) {
        blockedOlderBoundaryRef.current = boundaryKey;
        return;
      }
      const existingTimestamps = new Set(candlesRef.current.map((candle) => candle.ts));
      if (!uniqueOlder.some((candle) => !existingTimestamps.has(candle.ts))) {
        blockedOlderBoundaryRef.current = boundaryKey;
        return;
      }
      suppressTemplateForPrependRef.current = true;
      setCandles((current) => {
        if (olderFetchContextRef.current !== context) return current;
        const byTimestamp = new Map<number, CandleDto>();
        for (const candle of [...uniqueOlder, ...current]) {
          if (!byTimestamp.has(candle.ts)) byTimestamp.set(candle.ts, candle);
        }
        return [...byTimestamp.values()].sort((a, b) => a.ts - b.ts);
      });
      blockedOlderBoundaryRef.current = null;
      olderRetryAfterRef.current = null;
      setError(null);
    } catch (fetchError) {
      if (olderFetchContextRef.current === context) {
        const message = fetchError instanceof Error ? fetchError.message : String(fetchError);
        olderRetryAfterRef.current = {
          boundary: boundaryKey,
          timestamp: Date.now() + (message.startsWith("DUKASCOPY_RATE_LIMIT") ? 30_000 : 10_000),
        };
        setError(message.startsWith("DUKASCOPY_CONNECTION_FAILED")
          ? "Dukascopy is unreachable. Older online candles could not be loaded."
          : message.startsWith("DUKASCOPY_RATE_LIMIT")
            ? "Dukascopy is temporarily rate-limiting requests. Try again shortly."
            : message);
      }
    } finally {
      isFetchingOlderRef.current = false;
      setIsFetchingOlder(false);
    }
  }, [debouncedInterval, loading, replayOpen, symbol]);

  useEffect(() => {
    const chart = chartRef.current;
    if (!chart || !chartReady) return;
    const loadAtLeftEdge = (): void => {
      const logicalRange = chart.timeScale().getVisibleLogicalRange();
      if (logicalRange && logicalRange.from < 10) void fetchOlderCandles();
    };
    chart.timeScale().subscribeVisibleLogicalRangeChange(loadAtLeftEdge);
    return () => chart.timeScale().unsubscribeVisibleLogicalRangeChange(loadAtLeftEdge);
  }, [chartReady, fetchOlderCandles]);
  const magnetRef = useRef(magnetMode);
  magnetRef.current = magnetMode;
  // v7.1.1 — hide rect snap marker when magnet turns off.
  useEffect(() => {
    if (effectiveMagnetMode === "off") setRectSnap(null);
  }, [effectiveMagnetMode]);

  // v7.2.0 Part B — live registry scales for auto-coloring finished rects.
  const scalesRef = useRef<{ color: string; start_pct: number; end_pct: number }[] | null>(null);
  scalesRef.current = scaleRegistry
    ? scaleRegistry.scales.map((s) => ({ color: s.color, start_pct: s.start_pct, end_pct: s.end_pct }))
    : null;

  // v7.1.0: rectangle tool wiring (stable ctx object)
  const rectCtx = useMemo(() => {
    if (!chartReady || !chartRef.current || !seriesRef.current || !chartContainerRef.current) return null;
    return {
      container: chartContainerRef.current,
      chart: chartRef.current,
      series: seriesRef.current as unknown as object,
      drawingsRef: interactionDrawingsRef, rectToolRef, rectDragRef, magnetRef, ctrlHeldRef, candlesRef,
      setDrawings,
      previewRef,
      setPreview: (p: DrawingPreview | null) => setRectPreview(p),
      setSnap: (s: { t: number; p: number } | null) => setRectSnap(s),
      // v7.2.0 C2/C3 + Part B
      setHover: (h: RectHover | null) => setRectHover(h),
      scalesRef,
      // v7.3.0 Parts A+B — history snapshots + auto-deactivate on finalize.
      pushHistorySnapshot: (snap: DrawingAny[]) => pushDrawingsHistory(snap),
      onFinalizeRect: () => { if (!keepDrawingRef.current) setRectToolActive(false); },
    };
  }, [chartReady, pushDrawingsHistory]);

  const marqueeCtx = useMemo(() => {
    if (!chartReady || !chartRef.current || !seriesRef.current || !chartContainerRef.current) return null;
    return {
      container: chartContainerRef.current,
      chart: chartRef.current,
      series: seriesRef.current as unknown as object,
      drawingsRef: interactionDrawingsRef,
      candlesRef,
      activeToolRef: lineToolRef,
      rectToolRef,
      interactionLocked,
      setDrawings,
      setSelectionBox,
      isOverChrome: (clientX: number, clientY: number): boolean =>
        computeOverChrome(chartRef.current, chartContainerRef.current, clientX, clientY),
    };
  }, [chartReady, interactionLocked]);
  useMarqueeSelection(marqueeCtx);
  useRectTool(rectCtx);
  const gradedRectCtx = useMemo(() => {
    if (!chartReady || !chartRef.current || !seriesRef.current || !chartContainerRef.current) return null;
    return {
      container: chartContainerRef.current,
      chart: chartRef.current,
      series: seriesRef.current as unknown as object,
      activeToolRef: gradedActiveToolRef,
      drawingsRef: interactionDrawingsRef,
      candlesRef,
      pivotsRef,
      configRef: gradedRectConfigRef,
      setConfig: setGradedRectConfig,
      setDrawings,
      pushHistory: pushDrawingsHistory,
      setHoverPivot: (pivot: PivotDto | null) => gradedRectOverlayRef.current?.setHoverPivot(pivot),
      onContextMenu: setGradedRectContextMenu,
    };
  }, [chartReady, pushDrawingsHistory, setGradedRectConfig]);
  useGradedRectTool(gradedRectCtx);

  // v7.4.4 — line tools wiring (registers alongside rect tool; ignores
  // events when no line tool is active, except line select/drag).
  useEffect(() => {
    if (!chartReady || !chartRef.current || !seriesRef.current || !chartContainerRef.current) return;
    return registerLineTool({
      container: chartContainerRef.current,
      chart: chartRef.current,
      series: seriesRef.current as unknown as object,
      drawingsRef: interactionDrawingsRef,
      setDrawings,
      activeToolRef: lineToolRef,
      magnetRef,
      ctrlHeldRef,
      candlesRef,
      iconSelectionRef,
      pushHistorySnapshot: (snap: DrawingAny[]) => pushDrawingsHistory(snap),
      onFinalizeLine: () => { if (!keepDrawingRef.current) setActiveTool("cursor"); },
      trendPreviewRef,
      setTrendPreview,
      setLineHover,
      setSnap: (s: { t: number; p: number } | null) => setRectSnap(s),
      onBlankContextMenu: deselectDrawingsIfCanvasBlank,
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [chartReady, pushDrawingsHistory, deselectDrawingsIfCanvasBlank]);

  // v7.1.0 + v7.3.0 Part A + v7.3.1 Part 2C + v7.3.2 Part 2C/2F:
  // Keep history mutations outside state updaters so StrictMode cannot consume
  // multiple undo/redo entries while validating an updater.
  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      const target = e.target;
      if (target instanceof HTMLElement && (
        target.isContentEditable || target.closest("input, textarea, select")
      )) return;
      const mod = e.ctrlKey || e.metaKey;
      const debug = (() => { try { return getSetting("vora.debug") === "1"; } catch { return false; } })();
      if (mod) {
        const key = e.key.toLowerCase();
        const isZKey = key === "z" || e.key === "ز" || e.code === "KeyZ";
        const isYKey = key === "y" || e.code === "KeyY";
        if (isZKey && !e.shiftKey) {
          e.preventDefault();
          e.stopPropagation();
          if (debug) console.log("[undo] fired", key, "shift:", e.shiftKey, "stack:", undoHistory.size());
          undoDrawings();
          return;
        }
        if ((isZKey && e.shiftKey) || isYKey) {
          e.preventDefault();
          e.stopPropagation();
          if (debug) console.log("[undo] fired", key, "shift:", e.shiftKey, "stack:", undoHistory.size());
          redoDrawings();
          return;
        }
        return;
      }
      if (e.key === "Delete" || e.code === "Delete" || e.key === "Backspace") {
        const current = pruneNullDrawings(drawingsRef.current);
        if (!current.some((drawing) => drawing.selected)) return;
        e.preventDefault();
        e.stopPropagation();
        undoHistory.push({ drawings: current });
        setHistoryTick((n) => n + 1);
        const remaining = current.filter((drawing) => !drawing.selected);
        drawingsRef.current = remaining;
        setDrawings(remaining);
      } else if (e.key === "Escape") {
        setCtxMenu(null);
        setDrawings((prev) => prev.filter(Boolean).some((d) => d.selected)
          ? prev.filter(Boolean).map((d) => ({ ...d, selected: false }))
          : prev.filter(Boolean));
      }
    };
    window.addEventListener("keydown", onKey, true); // capture phase
    return () => window.removeEventListener("keydown", onKey, true);
  }, [undoDrawings, redoDrawings]);

  useEffect(() => {
    measureRef.current?.setSnap(snapPoint);
  }, [snapPoint]);

  useEffect(() => {
    const chart = chartRef.current;
    const series = seriesRef.current;
    if (!chart || !series || !chartReady) return;
    const srs = series as unknown as {
      coordinateToPrice: (y: number) => number | null;
      priceToCoordinate: (p: number) => number | null;
    };
    const snapAt = (px: number, py: number): { t: number; p: number } | null => {
      const mode = ctrlHeldRef.current
        ? magnetRef.current === "off" ? "strong" : "off"
        : magnetRef.current;
      if (mode === "off") return null;
      const list = candlesRef.current;
      if (!hasCandleAtCoordinate(chart, list, px)) return null;
      const t = chart.timeScale().coordinateToTime(px);
      if (t == null) return null;
      const time = t as number;
      const rawPrice = srs.coordinateToPrice(py);
      if (rawPrice == null) return null;
      if (mode === "strong") {
        const snapped = snapPriceToOhlc(time, rawPrice, list);
        return { t: snapped.ts, p: snapped.price };
      }
      return snapStrong(chart, srs, list, px, py);
    };
    type ClickParam = { point?: { x: number; y: number } };
    const handler = (param: ClickParam) => {
      if (!param?.point) return;
      const px = param.point.x as number;
      const py = param.point.y as number;
      const m = measureRef.current;
      const xr = m?.getXButtonRect?.();
      if (measureComplete && xr &&
          px >= xr.x && px <= xr.x + xr.w &&
          py >= xr.y && py <= xr.y + xr.h) {
        setMeasureComplete(null);
        setMeasurePending(null);
        setMeasureActive(false);
        setSnapPoint(null);
        return;
      }
      if (!measureActive) return;
      const t = chart.timeScale().coordinateToTime(px);
      if (t == null) return;
      const time = t as number;
      const price = srs.coordinateToPrice(py);
      if (price == null) return;
      const snapped = snapAt(px, py);
      const pt = snapped ?? { t: time, p: price };
      if (!measurePending) {
        setMeasurePending(pt);
        setSnapPoint(null);
      } else {
        setMeasureComplete({ tA: measurePending.t, pA: measurePending.p, tB: pt.t, pB: pt.p });
        setMeasurePending(null);
        setMeasureActive(false);
        setSnapPoint(null);
      }
    };
    // B5: live snap feedback on crosshair move (marker follows cursor within 30px)
    type MoveParam = { point?: { x: number; y: number } };
    const moveHandler = (param: MoveParam) => {
      if (!measureActive || !param?.point) {
        if (snapPointRef.current) { snapPointRef.current = null; measureRef.current?.setSnap(null); }
        return;
      }
      const s = snapAt(param.point.x, param.point.y);
      const prev = snapPointRef.current;
      if ((s?.t !== prev?.t) || (s?.p !== prev?.p)) {
        snapPointRef.current = s;
        measureRef.current?.setSnap(s);
      }
    };
    chart.subscribeClick(handler as unknown as Parameters<typeof chart.subscribeClick>[0]);
    chart.subscribeCrosshairMove(moveHandler as unknown as Parameters<typeof chart.subscribeCrosshairMove>[0]);
    return () => {
      try { chart.unsubscribeClick(handler as unknown as Parameters<typeof chart.subscribeClick>[0]); } catch { /* noop */ }
      try { chart.unsubscribeCrosshairMove(moveHandler as unknown as Parameters<typeof chart.subscribeCrosshairMove>[0]); } catch { /* noop */ }
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [measureActive, measurePending, measureComplete, chartReady]);

  useEffect(() => {
    setMeasurePending(null);
    setMeasureComplete(null);
  }, [debouncedM]);

  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) return;
    chart.applyOptions({
      crosshair: {
        vertLine: { visible: !overChrome },
        horzLine: { visible: !overChrome },
      },
    });
  }, [overChrome]);

  // v6.4.18: pivot inspector click handler (additive — separate from measure-v2).
  useEffect(() => {
    if (!chartReady) return;
    const chart = chartRef.current;
    const series = seriesRef.current;
    if (!chart || !series) return;

    const onClick = (param: MouseEventParams) => {
      const badge = pivotBadgeRef.current;
      if (!badge) return;
      const ps = pivotsRef.current;
      if (ps.length === 0) { badge.style.display = "none"; return; }

      const cx = param.point?.x, cy = param.point?.y;
      if (cx == null || cy == null) { badge.style.display = "none"; return; }

      const tsScale = chart.timeScale();
      let best: { x: number; y: number; p: PivotDto; d2: number } | null = null;
      for (const p of ps) {
        const x = tsScale.timeToCoordinate(p.ts as UTCTimestamp as Time);
        const y = series.priceToCoordinate(p.price);
        if (x == null || y == null) continue;
        const dx = x - cx, dy = y - cy;
        const d2 = dx * dx + dy * dy;
        if (d2 > 144) continue;
        if (!best || d2 < best.d2) best = { x, y, p, d2 };
      }

      if (!best) { badge.style.display = "none"; return; }

      const pad = (n: number) => String(n).padStart(2, "0");
      const t = new Date(best.p.ts * 1000);
      const line1 = `${best.p.kind}  ${best.p.price.toFixed(2)}  (idx ${best.p.idx})`;
      const line2 = `${t.getUTCFullYear()}-${pad(t.getUTCMonth()+1)}-${pad(t.getUTCDate())} ${pad(t.getUTCHours())}:${pad(t.getUTCMinutes())} UTC`;
      badge.textContent = `${line1}\n${line2}`;

      const container = chartContainerRef.current;
      const cw = container?.clientWidth ?? 0;
      const ch = container?.clientHeight ?? 0;
      badge.style.display = "block";
      const bw = badge.offsetWidth || 160;
      const bh = badge.offsetHeight || 36;
      let bx = best.x + 16, by = best.y - 16;
      if (bx + bw > cw - 8) bx = best.x - bw - 16;
      if (by + bh > ch - 8) by = best.y + 8;
      if (bx < 8) bx = 8;
      if (by < 8) by = 8;
      badge.style.left = `${bx}px`;
      badge.style.top = `${by}px`;
    };

    chart.subscribeClick(onClick);
    return () => { try { chart.unsubscribeClick(onClick); } catch { /* noop */ } };
  }, [chartReady]);

  // v6.4.18: hide pivot badge on step change.
  useEffect(() => {
    const b = pivotBadgeRef.current;
    if (b) b.style.display = "none";
  }, [debouncedM]);

  // v7.1.1 B5 — Escape cancels rect preview (in addition to tool-level key handler).
  useEffect(() => {
    const onKey = (ev: KeyboardEvent): void => {
      if (ev.key === "Escape" && previewRef.current) {
        setRectPreview(null);
        rectDragRef.current = null;
        setRectSnap(null);
        try {
          chartRef.current?.applyOptions({
            handleScroll: { pressedMouseMove: true },
          } as unknown as Parameters<NonNullable<typeof chartRef.current.applyOptions>>[0]);
        } catch { /* ignore */ }
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      const overlay = boxOverlayRef.current;
      const mOverlay = measureRef.current;
      const series  = seriesRef.current;
      if (overlay && series) {
        try { series.detachPrimitive(overlay as unknown as Parameters<typeof series.detachPrimitive>[0]); } catch { /* chart may be removed */ }
      }
      if (mOverlay && series) {
        try { series.detachPrimitive(mOverlay as unknown as Parameters<typeof series.detachPrimitive>[0]); } catch { /* noop */ }
      }
      boxOverlayRef.current = null;
      measureRef.current = null;
    };
  }, []);

  // ═══════════════════════════════════════════════════════════════
  // ZOOM APPLICATION MODES
  // ───────────────────────────────────────────────────────────────
  // Effect A → M change / initial load  → applyTemplateAtr (padding)
  // Effect B → user zoom (wheel / input) → applyZoomPreservingCenter
  // skipNextApplyRef prevents B from firing after A on M change.
  // ═══════════════════════════════════════════════════════════════

  // Effect A (M change / initial candles load)
  // v0.83.5: per-timeframe default zoom DISABLED — while a timeframe
  // viewport hold is active, re-apply the preserved viewport instead of
  // applyTemplateAtr. Template runs only on initial load (no hold armed).
  useEffect(() => {
    if (chartRef.current && candles.length > 0) {
      if (suppressTemplateForPrependRef.current) {
        suppressTemplateForPrependRef.current = false;
        return;
      }
      if (!initialChartViewResetRef.current) return;
      // v0.83.13: do not run template when the user just clicked a
      // new M — wait until the matching candles arrive.
      if (debouncedInterval !== lastFetchedIntervalRef.current) {
        return;
      }
      // v0.83.7 Part E — primary guard: suppress applyTemplateAtr fully
      // while a timeframe change is in flight.
      if (timeframeChangeInFlightRef.current) return;
      // v0.83.4: suppress template while a restore hold is active so the
      // refetch triggered by the restore itself doesn't clobber exact zoom.
      const hold = restoreHoldRef.current;
      if (hold && Date.now() < hold.expiry && chartRef.current) {
        void applyTfViewportVerified(
          chartRef.current,
          snapshotToCapturedViewport(hold.viewport),
        );
        return;
      }
      // v0.83.5: suppress template while a timeframe hold is active so a
      // timeframe switch preserves X+Y zoom (no per-timeframe default).
      const tfHold = timeframeHoldRef.current;
      if (tfHold && Date.now() < tfHold.expiry && chartRef.current) {
        void applyTfViewportVerified(chartRef.current, tfHold.viewport);
        return;
      }
      skipNextApplyRef.current = true; // suppress Effect B
      applyTemplateAtr(
        chartRef.current,
        candles,
        atrValue ?? 0,
        zoomState,
      );
    }
  }, [currentM, debouncedInterval, candles.length]);

  // Effect B (user-driven zoomState change)
  useEffect(() => {
    if (!chartRef.current || candles.length === 0) return;
    // v0.83.4: suppress user-zoom re-apply while a restore hold is active.
    const hold = restoreHoldRef.current;
    if (hold && Date.now() < hold.expiry && chartRef.current) {
      void applyTfViewportVerified(
        chartRef.current,
        snapshotToCapturedViewport(hold.viewport),
      );
      return;
    }
    // v0.83.5: suppress user-zoom re-apply while a timeframe hold is active.
    const tfHoldB = timeframeHoldRef.current;
    if (tfHoldB && Date.now() < tfHoldB.expiry && chartRef.current) {
      void applyTfViewportVerified(chartRef.current, tfHoldB.viewport);
      return;
    }
    if (skipNextApplyRef.current) {
      skipNextApplyRef.current = false;
      return; // skip once after Effect A
    }
    applyZoomPreservingCenter(
      chartRef.current,
      candles,
      atrValue ?? 0,
      zoomState,
    );
  }, [zoomState]);

  // Keep zoom controls synchronized with time-scale and price-scale gestures.
  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) return;

    let rafId: number | null = null;

    const sync = () => {
      if (spacePanDragRef.current || spacePanSyncSuppressedRef.current) return;
      const rangeVal = chart.timeScale().getVisibleLogicalRange();
      const priceRange = chart.priceScale("right").getVisibleRange();
      if (!rangeVal || !priceRange) return;

      const visible = Math.max(1, rangeVal.to - rangeVal.from);
      const halfRange = (priceRange.to - priceRange.from) / 2;
      const candleCount = candlesRef.current.length;
      const centerPrice = (priceRange.from + priceRange.to) / 2;
      const defaultHalfRange = atrValue && atrValue > 0
        ? atrValue * 4
        : Math.abs(centerPrice) * (candleCount < 14 ? 0.05 : 0.01);
      const nextZoom: ZoomState = {
        x_off_milli: Math.round(
          -Math.log2(visible / DEFAULT_VISIBLE_CANDLE_COUNT) * 1000,
        ),
        y_off_milli: defaultHalfRange > 0 && halfRange > 0
          ? Math.round(-Math.log2(halfRange / defaultHalfRange) * 1000)
          : zoomStateRef.current.y_off_milli,
      };
      const previousZoom = zoomStateRef.current;
      const xChanged = nextZoom.x_off_milli !== previousZoom.x_off_milli;
      const yChanged = nextZoom.y_off_milli !== previousZoom.y_off_milli;
      if (!xChanged && !yChanged) return;

      zoomStateRef.current = nextZoom;
      skipNextApplyRef.current = true;
      lastAppliedRef.current = {
        cc: visible,
        ym: defaultHalfRange > 0 ? Math.round((halfRange / defaultHalfRange) * 4000) : 4000,
      };
      setZoomState(nextZoom);
      if (document.activeElement !== inputXRef.current) {
        setInputX((nextZoom.x_off_milli / 1000).toFixed(4));
      }
      if (document.activeElement !== inputYRef.current) {
        setInputY((nextZoom.y_off_milli / 1000).toFixed(4));
      }
    };

    const schedule = () => {
      if (rafId !== null) return;
      rafId = requestAnimationFrame(() => {
        rafId = null;
        sync();
      });
    };

    chart.timeScale().subscribeVisibleLogicalRangeChange(schedule);
    const container = chartContainerRef.current;
    container?.addEventListener("pointerdown", schedule, true);
    container?.addEventListener("pointermove", schedule, true);
    container?.addEventListener("pointerup", schedule, true);
    container?.addEventListener("pointercancel", schedule, true);

    return () => {
      chart.timeScale().unsubscribeVisibleLogicalRangeChange(schedule);
      container?.removeEventListener("pointerdown", schedule, true);
      container?.removeEventListener("pointermove", schedule, true);
      container?.removeEventListener("pointerup", schedule, true);
      container?.removeEventListener("pointercancel", schedule, true);
      if (rafId !== null) cancelAnimationFrame(rafId);
    };
  }, [atrValue]);

  // Mirror zoomState -> inputs
  useEffect(() => {
    zoomStateRef.current = zoomState;
    if (document.activeElement !== inputXRef.current) {
      setInputX((zoomState.x_off_milli / 1000).toFixed(4));
    }
    if (document.activeElement !== inputYRef.current) {
      setInputY((zoomState.y_off_milli / 1000).toFixed(4));
    }
  }, [zoomState.x_off_milli, zoomState.y_off_milli]);

  const commitX = () => {
    const val = parseFloat(inputX);
    if (!isFinite(val)) {
      setInputX((zoomState.x_off_milli / 1000).toFixed(4));
      return;
    }
    const milli = Math.round(val * 1000);
    setZoomState((s) => ({ ...s, x_off_milli: milli }));
  };

  const commitY = () => {
    const val = parseFloat(inputY);
    if (!isFinite(val)) {
      setInputY((zoomState.y_off_milli / 1000).toFixed(4));
      return;
    }
    const milli = Math.round(val * 1000);
    setZoomState((s) => ({ ...s, y_off_milli: milli }));
  };

  const handleResetZoom = useCallback(() => {
    // Reset to the current data window, not the previously panned anchor.
    const chart = chartRef.current;
    if (!chart) return;
    restoreHoldRef.current = null;
    timeframeHoldRef.current = null;
    viewportToRestoreRef.current = null;
    timeframeChangeInFlightRef.current = false;
    skipNextApplyRef.current = true;
    setZoomState(DEFAULT_ZOOM);
    setInputX((DEFAULT_ZOOM.x_off_milli / 1000).toFixed(4));
    setInputY((DEFAULT_ZOOM.y_off_milli / 1000).toFixed(4));
    lastAppliedRef.current = {
      cc: offsetToCandleCount(DEFAULT_ZOOM.x_off_milli),
      ym: offsetToYMilli(DEFAULT_ZOOM.y_off_milli),
    };
    recentlyAppliedRef.current = Date.now();
    const activeCandles = candlesRef.current;
    if (activeCandles.length > 0) {
      applyTemplateAtr(chart, activeCandles, atrValueRef.current, DEFAULT_ZOOM);
    } else {
      const visibleRange = chart.timeScale().getVisibleLogicalRange();
      if (visibleRange) {
        chart.timeScale().setVisibleLogicalRange({
          from: Math.max(0, visibleRange.to - DEFAULT_VISIBLE_CANDLE_COUNT),
          to: visibleRange.to,
        });
      } else {
        chart.timeScale().fitContent();
      }
      chart.priceScale("right").applyOptions({ autoScale: true });
    }
  }, []);

  useEffect(() => {
    if (!chartReady || candles.length === 0 || initialChartViewResetRef.current) return;
    initialChartViewResetRef.current = true;
    void handleResetZoom();
  }, [chartReady, candles.length, handleResetZoom]);

  const handleCopyContextPrice = useCallback(async () => {
    const priceText = ctxMenu?.priceText;
    if (!priceText) return;

    let copied = false;
    try {
      if (!navigator.clipboard?.writeText) throw new Error("Clipboard API unavailable");
      await navigator.clipboard.writeText(priceText);
      copied = true;
    } catch {
      const input = document.createElement("textarea");
      input.value = priceText;
      input.style.position = "fixed";
      input.style.opacity = "0";
      document.body.appendChild(input);
      input.select();
      copied = document.execCommand("copy");
      input.remove();
    }

    showSnapshotToast(copied ? `Copied price ${priceText}` : "Could not copy price");
    setCtxMenu(null);
  }, [ctxMenu, showSnapshotToast]);

  // v0.83.5 — "Scroll to most recent bar" (TradingView-style).
  const handleScrollToLatest = useCallback(() => {
    const chart = chartRef.current;
    if (!chart) return;
    const timeScale = chart.timeScale();
    const anyTs = timeScale as unknown as {
      scrollToRealTime?: () => void;
    };
    if (typeof anyTs.scrollToRealTime === "function") {
      anyTs.scrollToRealTime();
    } else {
      const len = candlesRef.current.length;
      if (len === 0) return;
      const chartWidth =
        chartContainerRef.current?.clientWidth ?? 800;
      const barSpacing = timeScale.options().barSpacing ?? 10;
      const visibleBars = Math.max(
        5,
        Math.floor(chartWidth / barSpacing),
      );
      timeScale.setVisibleLogicalRange({
        from: Math.max(0, len - visibleBars),
        to: len - 1,
      });
    }
    try {
      (chart as unknown as IChartApi)
        .priceScale("right")
        ?.applyOptions?.({ autoScale: true });
    } catch {
      /* ignore */
    }
  }, []);

  // v0.83.5 — auto-hide: hidden at the latest bar, shown when scrolled away.
  useEffect(() => {
    const chart = chartRef.current;
    if (!chart || !chartReady) return;
    const timeScale = chart.timeScale();
    const update = () => {
      const lr = timeScale.getVisibleLogicalRange();
      const len = candlesRef.current.length;
      if (!lr || len === 0) {
        setScrollToLatestVisible(false);
        return;
      }
      const isAtLatest = lr.to >= len - 3;
      setScrollToLatestVisible(!isAtLatest);
    };
    timeScale.subscribeVisibleLogicalRangeChange(update);
    update();
    return () => {
      try {
        timeScale.unsubscribeVisibleLogicalRangeChange(update);
      } catch {
        /* ignore */
      }
    };
  }, [chartReady]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // v0.83.2.1 — modal/overlay open: no chart shortcuts (arrows/Esc/Delete).
      if (interactionLocked) return;
      // v0.83.5 — scroll to the most recent bar.
      if (e.altKey && e.shiftKey && e.key === "ArrowRight") {
        e.preventDefault();
        handleScrollToLatest();
        return;
      }
      if (e.target instanceof HTMLInputElement) return;
      if (e.altKey && e.key.toLowerCase() === "r") {
        e.preventDefault();
        void handleResetZoom();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        selectInterval(`M${Math.min(currentM + 1, 10)}`);
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        selectInterval(`M${Math.max(currentM - 1, 0)}`);
      } else if (e.key === "Escape") {
        handleResetZoom();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleResetZoom, interactionLocked]);

  const clearLongPressTimer = (): void => {
    if (longPressTimerRef.current !== null) {
      window.clearTimeout(longPressTimerRef.current);
      longPressTimerRef.current = null;
    }
  };

  const handleChartPointerDown = (event: React.PointerEvent<HTMLDivElement>): void => {
    if (replaySelecting || !valuesTooltipOnLongPress || interactionLocked || (event.pointerType === "mouse" && event.button !== 0)) return;
    if (event.target instanceof Element && event.target.closest(".vora-floating-ui")) return;
    clearLongPressTimer();
    setLongPressTooltip(null);
    const start = { pointerId: event.pointerId, x: event.clientX, y: event.clientY };
    longPressStartRef.current = start;
    longPressTimerRef.current = window.setTimeout(() => {
      const chart = chartRef.current;
      const series = seriesRef.current;
      const container = chartContainerRef.current;
      if (!chart || !series || !container) return;
      const chartRect = chart.chartElement().getBoundingClientRect();
      const pointX = start.x - chartRect.left;
      const pointY = start.y - chartRect.top;
      const time = chart.timeScale().coordinateToTime(pointX);
      const price = series.coordinateToPrice(pointY);
      if (price === null) return;
      const timestamp = typeof time === "number" ? time : null;
      const candle = timestamp === null ? null : candles.reduce<CandleDto | null>((nearest, candidate) => {
        if (!nearest || Math.abs(candidate.ts - timestamp) < Math.abs(nearest.ts - timestamp)) return candidate;
        return nearest;
      }, null);
      const containerRect = container.getBoundingClientRect();
      setLongPressTooltip({
        x: Math.max(8, Math.min(container.clientWidth - 220, start.x - containerRect.left + 12)),
        y: Math.max(8, Math.min(container.clientHeight - 112, start.y - containerRect.top + 12)),
        timestamp,
        price: Number(price),
        candle,
      });
      longPressTimerRef.current = null;
    }, 550);
  };

  const handleChartPointerMove = (event: React.PointerEvent<HTMLDivElement>): void => {
    const start = longPressStartRef.current;
    if (!start || start.pointerId !== event.pointerId) return;
    if (Math.hypot(event.clientX - start.x, event.clientY - start.y) > 8) {
      clearLongPressTimer();
      longPressStartRef.current = null;
      setLongPressTooltip(null);
    }
  };

  const endChartPointer = (event: React.PointerEvent<HTMLDivElement>): void => {
    if (longPressStartRef.current?.pointerId !== event.pointerId) return;
    clearLongPressTimer();
    longPressStartRef.current = null;
  };

  return (
    <div className="chart-shell">
      <header className="chart-shell__topbar">
        <div
          className="h-full min-w-0 flex-1 flex items-center gap-3 px-3 flex-nowrap overflow-x-auto overflow-y-hidden bg-[#0f0f0f] border-b border-[#2a2a2a]"
          style={{ cursor: "default" }}
          onMouseEnter={() => setOverChrome(true)}
          onMouseLeave={() => setOverChrome(false)}
        >
          <SymbolSelector />

          <ChartIntervalSelector value={activeInterval} onChange={selectInterval} />

          {/* v7.4.0: Rect + Measure moved to floating toolbar */}
          {/* v7.4.3: single display-mode toggle (icon-only, 2s tooltip) */}
          <div style={{ position: "relative" }}>
            <button
              type="button"
              onClick={() => {
                const next: ChartDisplayMode = chartDisplayMode === "candlestick" ? "special" : "candlestick";
                setChartDisplayMode(next);
              }}
              className="px-2 h-7 text-[13px] font-medium transition-colors cursor-pointer bg-[#ff9800] text-black rounded border border-[#383838] shrink-0 flex items-center justify-center"
              aria-label={chartDisplayMode === "candlestick" ? "Candlestick" : "Special"}
              onMouseEnter={() => {
                if (displayHoverTimer.current !== null) window.clearTimeout(displayHoverTimer.current);
                displayHoverTimer.current = window.setTimeout(() => setShowDisplayTip(true), 2000);
              }}
              onMouseLeave={() => {
                if (displayHoverTimer.current !== null) { window.clearTimeout(displayHoverTimer.current); displayHoverTimer.current = null; }
                setShowDisplayTip(false);
              }}
            >
              {chartDisplayMode === "candlestick" ? (
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" fill="currentColor">
                  <path d="M17 11v6h3v-6h-3zm-.5-1h4a.5.5 0 0 1 .5.5v7a.5.5 0 0 1-.5.5h-4a.5.5 0 0 1-.5-.5v-7a.5.5 0 0 1 .5-.5z"></path>
                  <path d="M18 7h1v3.5h-1zm0 10.5h1V21h-1z"></path>
                  <path d="M9 8v12h3V8H9zm-.5-1h4a.5.5 0 0 1 .5.5v13a.5.5 0 0 1-.5.5h-4a.5.5 0 0 1-.5-.5v-13a.5.5 0 0 1 .5-.5z"></path>
                  <path d="M10 4h1v3.5h-1zm0 16.5h1V24h-1z"></path>
                </svg>
              ) : (
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 28" width="28" height="28" fill="none" aria-hidden="true">
                  <g stroke="currentColor" strokeWidth="1.35" strokeLinecap="round" strokeLinejoin="round">
                    <path d="m14 6 6.5 3.75v7.5L14 21l-6.5-3.75v-7.5L14 6Z" />
                    <path d="m7.5 9.75 6.5 3.8 6.5-3.8M14 13.55V21" />
                    <path d="M14 21 23.5 26.2M14 21 4.5 26.2M14 21V2.2" strokeWidth="1.55" />
                    <path d="m20.9 24.8 2.6 1.4-.2-2.9M7.1 24.8l-2.6 1.4.2-2.9M12.2 4l1.8-1.8L15.8 4" strokeWidth="1.55" />
                  </g>
                  <circle cx="14" cy="21" r="1.15" fill="currentColor" />
                </svg>
              )}
            </button>
            {showDisplayTip && (
              <span style={{
                position: "absolute", top: "calc(100% + 6px)", left: "50%", transform: "translateX(-50%)",
                background: "#181818", color: "#d6dbe2", padding: "2px 6px", border: "1px solid #2a2a2a",
                borderRadius: 3, fontSize: 11, whiteSpace: "nowrap", pointerEvents: "none", zIndex: 30,
              }}>{chartDisplayMode === "candlestick" ? "Candlestick" : "Special"}</span>
            )}
          </div>

          {/* v7.3.0 Part A — Undo/Redo (in-memory, drawings only). */}
          <button
            type="button"
            onClick={undoDrawings}
            disabled={!undoHistory.canUndo()}
            className="w-7 h-7 shrink-0 flex items-center justify-center rounded hover:bg-[#2a2a2a] text-[#787b86] hover:text-[#d6dbe2] disabled:opacity-30 disabled:cursor-not-allowed"
            title="Undo drawing change (Ctrl+Z)"
            aria-label="Undo drawing change"
          >
            <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <path d="M6 3 3 6l3 3" />
              <path d="M3 6h6a4 4 0 0 1 0 8h-2" transform="translate(0 -2)" />
            </svg>
          </button>
          <button
            type="button"
            onClick={redoDrawings}
            disabled={!undoHistory.canRedo()}
            className="w-7 h-7 shrink-0 flex items-center justify-center rounded hover:bg-[#2a2a2a] text-[#787b86] hover:text-[#d6dbe2] disabled:opacity-30 disabled:cursor-not-allowed"
            title="Redo drawing change (Ctrl+Shift+Z)"
            aria-label="Redo drawing change"
          >
            <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <path d="m10 3 3 3-3 3" />
              <path d="M13 6H7a4 4 0 0 0 0 8h2" transform="translate(0 -2)" />
            </svg>
          </button>
          <button
            type="button"
            className={`chart-replay-toggle${replayOpen ? " is-active" : ""}`}
            aria-label={replayOpen ? "Exit Bar Replay" : "Bar Replay"}
            aria-pressed={replayOpen}
            title={replayOpen ? "Exit Bar Replay" : "Bar Replay"}
            disabled={candles.length === 0}
            onClick={toggleBarReplay}
          >
            <svg viewBox="0 0 28 28" width="28" height="28" aria-hidden="true">
              <path fill="none" stroke="currentColor" d="M13.5 20V9l-6 5.5 6 5.5zM21.5 20V9l-6 5.5 6 5.5z" />
            </svg>
          </button>
          {/* v7.3.1 Part 2E — debug stack readout (VORA_DEBUG in localStorage only). */}
          {(() => {
            try {
              if (!getSetting("VORA_DEBUG")) return null;
            } catch { return null; }
            const s = undoHistory.size();
            return (
              <span className="text-[11px] font-mono text-[#787b86]" title="Undo/Redo stack depth">
                past:{s.past} future:{s.future}
              </span>
            );
          })()}
          {/* Divider */}
          <div className="w-px h-5 bg-[#383838] shrink-0" />

          {/* Center: X + Y inputs */}
          <span className="font-mono tabular-nums text-[#d1d4dc] text-xs shrink-0">
            X
          </span>
          <input
            ref={inputXRef}
            type="text"
            value={inputX}
            onChange={(e) => {
              const value = e.target.value;
              setInputX(value);
              if (value.trim() === "") return;
              const parsed = Number(value);
              if (!Number.isFinite(parsed)) return;
              const xOffMilli = Math.round(parsed * 1000);
              setZoomState((previous) => previous.x_off_milli === xOffMilli
                ? previous
                : { ...previous, x_off_milli: xOffMilli });
            }}
            onBlur={commitX}
            onKeyDown={(e) => e.key === "Enter" && commitX()}
            className="w-20 px-2 py-0.5 text-center font-mono tabular-nums text-xs bg-[#0f0f0f] border border-[#383838] rounded text-[#ff9800] whitespace-nowrap shrink-0 focus:outline-none focus:border-[#ff9800]"
          />

          <span className="font-mono tabular-nums text-[#d1d4dc] text-xs shrink-0">
            Y
          </span>
          <input
            ref={inputYRef}
            type="text"
            value={inputY}
            onChange={(e) => {
              const value = e.target.value;
              setInputY(value);
              if (value.trim() === "") return;
              const parsed = Number(value);
              if (!Number.isFinite(parsed)) return;
              const yOffMilli = Math.round(parsed * 1000);
              setZoomState((previous) => previous.y_off_milli === yOffMilli
                ? previous
                : { ...previous, y_off_milli: yOffMilli });
            }}
            onBlur={commitY}
            onKeyDown={(e) => e.key === "Enter" && commitY()}
            className="w-20 px-2 py-0.5 text-center font-mono tabular-nums text-xs bg-[#0f0f0f] border border-[#383838] rounded text-[#ff9800] whitespace-nowrap shrink-0 focus:outline-none focus:border-[#ff9800]"
          />

          <div
            className="group relative z-40 shrink-0"
            onMouseEnter={() => setIsHomeMenuOpen(true)}
            onMouseLeave={() => setIsHomeMenuOpen(false)}
            onFocusCapture={() => setIsHomeMenuOpen(true)}
            onBlurCapture={(event) => {
              if (!event.currentTarget.contains(event.relatedTarget)) {
                setIsHomeMenuOpen(false);
              }
            }}
          >
            <button
              type="button"
              onClick={() => setIsHomeMenuOpen(true)}
              aria-label="Open Home and Journal menu"
              aria-haspopup="menu"
              aria-expanded={isHomeMenuOpen}
              title="Home and Journal"
              className="w-7 h-7 shrink-0 flex items-center justify-center rounded hover:bg-[#2a2a2a] text-[#787b86] hover:text-[#ff9800]"
            >
              <Menu size={16} />
            </button>
            <div
              role="menu"
              aria-label="Home and Journal navigation"
              className={`absolute left-0 top-full z-50 mt-1 flex h-14 w-7 flex-col overflow-hidden rounded border border-[#2a2a2a] bg-[#0f0f0f]/95 p-0 shadow-lg backdrop-blur transition-all duration-200 ${
                isHomeMenuOpen ? "visible translate-y-0 opacity-100" : "invisible -translate-y-1 opacity-0"
              }`}
            >
              <Link
                to="/home"
                role="menuitem"
                aria-label="Home"
                title="Home"
                onClick={() => setIsHomeMenuOpen(false)}
                className="flex h-7 w-full shrink-0 items-center justify-center text-[#787b86] transition hover:bg-[#2a2a2a] hover:text-[#ff9800]"
              >
                <House size={16} aria-hidden="true" />
              </Link>
              <Link
                to="/home"
                state={{ openJournal: true }}
                role="menuitem"
                aria-label="Journal"
                title="Journal"
                onClick={() => setIsHomeMenuOpen(false)}
                className="flex h-7 w-full shrink-0 items-center justify-center text-[#787b86] transition hover:bg-[#2a2a2a] hover:text-[#ff9800]"
              >
                <BookOpen size={16} aria-hidden="true" />
              </Link>
            </div>
          </div>

          {/* v0.83.0 — Take Snapshot + Snapshots list */}
          <Link
            to="/snapshots"
            className="w-7 h-7 shrink-0 flex items-center justify-center rounded hover:bg-[#2a2a2a] text-[#787b86] hover:text-[#ff9800]"
            aria-label="Open saved snapshots"
            data-name="open-snapshots"
          >
            <Camera size={20} />
          </Link>
          <button
            type="button"
            onClick={handleRecordingButtonClick}
            className={`w-7 h-7 shrink-0 flex items-center justify-center rounded ${isRecording ? "bg-[#ef5350]/10 text-[#ef5350]" : "text-[#787b86] hover:bg-[#2a2a2a] hover:text-[#ff9800]"}`}
            title={isRecording ? "Stop recording and save" : "Start narrative recording"}
            aria-label={isRecording ? "Stop recording and save" : "Start narrative recording"}
            aria-pressed={isRecording}
            data-name="narrative-recording"
          >
            <Video size={20} />
          </button>
          {/* Policy panel toggle (MKT-6b-3.6) */}
          <button
            onClick={() => setPanelOpen((v) => !v)}
            className="w-7 h-7 shrink-0 flex items-center justify-center rounded hover:bg-[#2a2a2a] text-[#787b86] hover:text-[#d6dbe2]"
            title={panelOpen ? "Hide policy panel" : "Show policy panel"}
          >
            <Sliders size={14} />
          </button>

          {/* Spacer */}
          <div className="flex-1" />
        </div>
      </header>
      <main className={`chart-shell__main${rightPanel ? " chart-shell__main--panel-open" : ""}`}>
        <ChartDrawingToolbar
          activeTool={floatingActiveTool || activeTool}
          magnetMode={lastMagnetMode}
          magnetEnabled={effectiveMagnetMode !== "off"}
          keepDrawing={keepDrawing}
          drawingsLocked={drawings.length > 0 && drawings.every((drawing) => drawing.locked === true)}
          drawingsHidden={drawingsHidden}
          removableDrawingCount={drawings.filter((drawing) => alwaysRemoveLockedDrawings || drawing.locked !== true).length}
          indicatorCount={0}
          alwaysRemoveLockedDrawings={alwaysRemoveLockedDrawings}
          favoriteToolbarVisible={floatingToolbarVisible}
          onSelectTool={handleDrawingToolSelect}
          onSelectIcon={(category, symbol) => {
            iconSelectionRef.current = { category, symbol };
            handleDrawingToolSelect("icon");
          }}
          onToggleFavoriteToolbar={() => setFloatingToolbarVisible((visible) => !visible)}
          onToggleMagnet={() => selectMagnetMode(magnetMode === "off" ? lastMagnetMode : "off")}
          onSetMagnetMode={selectMagnetMode}
          onToggleKeepDrawing={() => setKeepDrawing((value) => !value)}
          onToggleLock={() => {
            if (drawings.length === 0) return;
            const shouldLock = !drawings.every((drawing) => drawing.locked === true);
            pushDrawingsHistory(drawingsRef.current);
            setDrawings((previous) => previous.map((drawing) => ({ ...drawing, locked: shouldLock })));
          }}
          onLockDrawings={() => {
            if (drawings.length === 0 || drawings.every((drawing) => drawing.locked === true)) return;
            pushDrawingsHistory(drawingsRef.current);
            setDrawings((previous) => previous.map((drawing) => ({ ...drawing, locked: true })));
          }}
          onLockIndicators={() => undefined}
          onLockAll={() => {
            if (drawings.length === 0 || drawings.every((drawing) => drawing.locked === true)) return;
            pushDrawingsHistory(drawingsRef.current);
            setDrawings((previous) => previous.map((drawing) => ({ ...drawing, locked: true })));
          }}
          onHideDrawings={hideDrawings}
          onHideIndicators={() => undefined}
          onHidePositionsAndOrders={() => undefined}
          onHideAllObjects={hideDrawings}
          onRemoveDrawings={() => void removeDrawings()}
          onRemoveIndicators={() => undefined}
          onRemoveAllObjects={() => void removeDrawings()}
          onToggleAlwaysRemoveLockedDrawings={() => setAlwaysRemoveLockedDrawings((value) => !value)}
          cursorFavorites={cursorFavorites}
          toolFavorites={drawingToolFavorites}
          onToggleToolFavorite={(id) => setDrawingToolFavorites((previous) => previous.includes(id) ? previous.filter((favorite) => favorite !== id) : [...previous, id])}
          onToggleCursorFavorite={(id) => setCursorFavorites((previous) => previous.includes(id) ? previous.filter((favorite) => favorite !== id) : [...previous, id])}
          valuesTooltipOnLongPress={valuesTooltipOnLongPress}
          onToggleValuesTooltip={() => {
            setValuesTooltipOnLongPress((enabled) => !enabled);
            clearLongPressTimer();
            longPressStartRef.current = null;
            setLongPressTooltip(null);
          }}
          onZoomIn={() => {
            const chart = chartRef.current;
            if (!chart) return;
            const timeScale = chart.timeScale();
            const options = timeScale.options() as unknown as { barSpacing?: number; maxBarSpacing?: number };
            timeScale.applyOptions({ barSpacing: Math.min(options.maxBarSpacing ?? 60, (options.barSpacing ?? 10) * 1.18) });
          }}
        />
        <div className="chart-shell__stage">
            <div className="h-full w-full flex flex-col overflow-hidden bg-[#0f0f0f] text-[#d6dbe2] select-none relative">
            {/* Chart area — FULL SCREEN with overlay */}
            {/* v7.1.3 Part D — cursor states: only the inner chart drawing canvas
                gets crosshair (when Rect tool is active); toolbar + price/time axes
                (children inheriting this container) stay default. The rect tool
                hover handler overrides the drawing canvas cursor per-state. */}
            <div
              ref={chartContainerRef}
              className={`flex-1 min-h-0 overflow-hidden relative ${chartCursorClass}`}
              style={{
                pointerEvents: interactionLocked ? "none" : "auto",
              }}
              onMouseEnter={(e) => {
                const chart = chartRef.current;
                const wrap = chartContainerRef.current;
                if (!chart || !wrap) return;
                const onAxis = computeOverChrome(
                  chart,
                  wrap,
                  e.clientX,
                  e.clientY,
                );
                setOverChrome(onAxis);
              }}
              onMouseMove={(e) => {
                const chart = chartRef.current;
                const wrap = chartContainerRef.current;
                if (!chart || !wrap) return;
                const onAxis = computeOverChrome(
                  chart,
                  wrap,
                  e.clientX,
                  e.clientY,
                );
                if (onAxis !== overChrome) setOverChrome(onAxis);
              }}
              onMouseLeave={() => {
                if (overChrome) setOverChrome(false);
              }}
              onPointerDown={handleChartPointerDown}
              onPointerMove={handleChartPointerMove}
              onPointerUp={endChartPointer}
              onPointerCancel={endChartPointer}
              onPointerLeave={() => {
                clearLongPressTimer();
                longPressStartRef.current = null;
              }}
              onContextMenu={(e) => {
                if (replaySelecting) { e.preventDefault(); return; }
                // v0.83.2.1 — locked: never open the drawing context menu.
                if (interactionLocked) { e.preventDefault(); return; }
                e.preventDefault();
                if (rectPreview?.active || measurePending) {
                  setRectToolActive(false);
                  setRectPreview(null);
                  rectDragRef.current = null;
                  setRectSnap(null);
                  setMeasureActive(false);
                  setMeasurePending(null);
                  setSnapPoint(null);
                  measureRef.current?.setSnap(null);
                  try {
                    chartRef.current?.applyOptions({
                      handleScroll: { pressedMouseMove: true },
                    } as unknown as Parameters<NonNullable<typeof chartRef.current.applyOptions>>[0]);
                  } catch { /* ignore */ }
                  deselectDrawingsIfCanvasBlank();
                  setCtxMenu(null);
                  return;
                }
                const isCanvasBlank = deselectDrawingsIfCanvasBlank();
                if (isCanvasBlank && rectToolActive) {
                  setRectToolActive(false);
                  setRectPreview(null);
                  rectDragRef.current = null;
                  setRectSnap(null);
                  try {
                    chartRef.current?.applyOptions({
                      handleScroll: { pressedMouseMove: true },
                    } as unknown as Parameters<NonNullable<typeof chartRef.current.applyOptions>>[0]);
                  } catch { /* ignore */ }
                }
                const chart = chartRef.current;
                const series = seriesRef.current;
                const chartRect = chart?.chartElement().getBoundingClientRect();
                const price = chartRect && series
                  ? series.coordinateToPrice(e.clientY - chartRect.top)
                  : null;
                const priceText = price === null || !series
                  ? null
                  : series.priceFormatter().format(price);
                setCtxMenu({ x: e.clientX, y: e.clientY, priceText });
              }}
            >
              {selectionBox && (
                <div
                  className="chart-marquee-selection vora-floating-ui"
                  style={{
                    left: selectionBox.left,
                    top: selectionBox.top,
                    width: selectionBox.width,
                    height: selectionBox.height,
                  }}
                />
              )}
              {/* Error overlay (only when error) */}
              {error && (
                <div className="vora-floating-ui pointer-events-none absolute top-2 left-2 z-10 px-3 py-1.5 rounded bg-[#ef5350]/15 border border-[#ef5350]/40 text-[#ef5350] text-xs flex items-center gap-2">
                  <AlertCircle size={13} />
                  <span>{error}</span>
                </div>
              )}

              {/* Loading overlay */}
              {loading && (
                <div className="vora-floating-ui pointer-events-none absolute top-2 left-2 z-10 px-3 py-1.5 rounded bg-[#1e1e1e] border border-[#383838] text-[#d6dbe2] text-xs">
                  Loading {activeInterval}...
                </div>
              )}
              {!loading && candles.length === 0 && (
                <div
                  className="vora-floating-ui pointer-events-none absolute inset-0 z-[1] flex items-center justify-center"
                  role="status"
                >
                  <span className="rounded border border-[#383838] bg-[#111]/85 px-3 py-2 text-xs text-[#9ca3af]">
                    Market data unavailable · drawing tools are ready
                  </span>
                </div>
              )}
              {isFetchingOlder && (
                <div className="vora-floating-ui chart-older-loading" role="status" aria-label="Loading older candles">
                  <span className="chart-older-loading__spinner" aria-hidden="true" />
                </div>
              )}
              {activeTool === "graded-rectangle" && (
                <GradedRectangleControls config={gradedRectConfig} onChange={setGradedRectConfig} />
              )}

              {/* Chart */}
              <CandlestickChart
                candles={candles}
                emptyStateIntervalSeconds={intervalWindowSeconds(activeInterval)}
                emptyStateReferencePrice={readOfflineReferencePrice(symbol)}
                secondsVisible={intervalWindowSeconds(activeInterval) < 60}
                replayCursorTime={replayOpen && replayEndIndex !== null
                  ? replayTimestampOverridesRef.current?.end ?? candles[replayEndIndex]?.ts ?? null
                  : null}
                onReady={handleChartReady}
                displayMode={chartDisplayMode}
                preserveViewport={preserveViewport || replayOpen}
                magnetActive={activeTool !== "cursor" && effectiveMagnetMode !== "off"}
              />
              {replayOpen && replaySelecting && replayCursorX !== null && <div className="chart-replay-cursor" style={{ left: replayCursorX }} aria-hidden="true">
                <span>{new Date((replayHoverTimestamp ?? 0) * 1000).toLocaleString("en-GB", { timeZone: "UTC", day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit", hour12: false })}</span>
              </div>}
              {replaySelecting && <div className="chart-replay-selection-prompt" role="status">Click a candle to set the replay start</div>}
              {/* v0.83.5 — "Scroll to most recent bar" affordance (bottom-right hotspot). */}
              <div className="vora-floating-ui scroll-to-latest-hotspot">
                <ScrollToLatestButton
                  visible={scrollToLatestVisible}
                  onClick={handleScrollToLatest}
                />
              </div>
              {longPressTooltip && (
                <div
                  role="status"
                  className="chart-long-press-values vora-floating-ui"
                  style={{ left: longPressTooltip.x, top: longPressTooltip.y }}
                >
                  <span className="chart-long-press-values__time">
                    {longPressTooltip.timestamp === null ? "Chart value" : new Date(longPressTooltip.timestamp * 1000).toLocaleString()}
                  </span>
                  <span>Price <strong>{longPressTooltip.price.toFixed(2)}</strong></span>
                  {longPressTooltip.candle && (
                    <span className="chart-long-press-values__ohlc">
                      O {longPressTooltip.candle.open.toFixed(2)} · H {longPressTooltip.candle.high.toFixed(2)} · L {longPressTooltip.candle.low.toFixed(2)} · C {longPressTooltip.candle.close.toFixed(2)}
                    </span>
                  )}
                </div>
              )}
              <RestoreOverlay visible={restoreOverlay.visible} name={restoreOverlay.name} />
              {/* v7.4.0 — floating drawing toolbar (TradingView-style) */}
              <FloatingToolbar
                visible={floatingToolbarVisible}
                onToggle={() => setFloatingToolbarVisible((v) => !v)}
                activeTool={floatingActiveTool}
                onSelectTool={handleDrawingToolSelect}
                cursorFavorites={cursorFavorites}
                drawingToolFavorites={drawingToolFavorites}
              />
              <div
                ref={pivotBadgeRef}
                className="vora-floating-ui"
                style={{
                  position: "absolute",
                  pointerEvents: "none",
                  zIndex: 30,
                  padding: "4px 8px",
                  background: "#181818",
                  color: "#d6dbe2",
                  font: "12px/1.4 monospace",
                  border: "1px solid #383838",
                  borderRadius: 4,
                  display: "none",
                  whiteSpace: "pre",
                }}
              />

              {/* Policy panel (MKT-6b-3.6) */}
              {panelOpen && (
                <PolicyPanel
                  values={policy}
                  onChange={setPolicy}
                  onClose={() => setPanelOpen(false)}
                  registry={scaleRegistry}
                  onScaleChange={onScaleChange}
                  onAddScale={onAddScale}
                  onRemoveScale={onRemoveScale}
                  onSaveRegistry={onSaveRegistry}
                  isSaving={isSaving}
                  saveOk={saveOk}
                  validationError={saveError ?? validationError}
                  autoDetect={autoDetect}
                  onAutoDetectChange={setAutoDetect}
                  defaultRouteSetting={defaultRouteSetting}
                  onDefaultRouteChange={setDefaultRouteSetting}
                  drawings={{ color: lineColor, rectW: rectWidth, hW: hlineWidth, vW: vlineWidth, rW: hrayWidth, tW: trendWidth, onPatch: persistLinePrefs }}
                />
              )}
              {toolbarToast82 && (
                <div className="vora-floating-ui" style={{ position: "absolute", bottom: 90, left: "50%", transform: "translateX(-50%)", background: "#181818", color: "#d6dbe2", border: "1px solid #2a2a2a", padding: "6px 12px", borderRadius: 4, fontSize: 12, zIndex: 60 }}>{toolbarToast82}</div>
              )}
              <DrawingToolbarHost drawings={drawings} drawingsRef={drawingsRef} setDrawings={setDrawings} setToast={setToolbarToast82} pushHist={pushDrawingsHistory} persistPrefs={persistLinePrefs} symbol={symbol} timeframe={currentM} latestCandleTs={candles.length > 0 ? candles[candles.length - 1].ts : null} onAlertCreated={(alert) => { setDrawingAlerts((previous) => [alert, ...previous]); setToolbarToast82("Drawing alert created"); window.setTimeout(() => setToolbarToast82(null), 2200); }} />
              <SnapshotCaptureModal
                open={snapshotModalOpen}
                defaultName={`${symbol} ${activeInterval} setup`}
                frameCount={fullNarrativeLog.length}
                onClose={() => setSnapshotModalOpen(false)}
                onSave={handleSaveSnapshot}
              />

              {/* v7.3.0 Part B1 — right-click context menu (position:fixed, viewport coords). */}
              {gradedRectContextMenu && (
                <>
                  <div
                    className="vora-floating-ui fixed inset-0 z-40"
                    onMouseDown={() => setGradedRectContextMenu(null)}
                    onContextMenu={(event) => { event.preventDefault(); setGradedRectContextMenu(null); }}
                  />
                  <div
                    role="menu"
                    aria-label="Graded Rectangle options"
                    className="vora-floating-ui fixed z-50 min-w-[210px] rounded border border-[#63358d] bg-[#18151d] py-1 text-[13px] shadow-xl"
                    style={{
                      left: Math.max(8, Math.min(gradedRectContextMenu.x, window.innerWidth - 226)),
                      top: Math.max(8, Math.min(gradedRectContextMenu.y, window.innerHeight - 92)),
                    }}
                    onMouseDown={(event) => event.stopPropagation()}
                  >
                    {(["top", "bottom"] as const).map((baseline) => (
                      <button
                        key={baseline}
                        type="button"
                        role="menuitem"
                        className="block w-full px-3 py-2 text-left text-[#e9d5ff] hover:bg-[#33223f]"
                        onClick={() => {
                          const target = drawingsRef.current.find(
                            (drawing): drawing is GradedRectangle =>
                              drawing.type === "graded-rectangle" && drawing.id === gradedRectContextMenu.drawingId,
                          );
                          if (!target || target.baselineSide === baseline) {
                            setGradedRectContextMenu(null);
                            return;
                          }
                          pushDrawingsHistory(drawingsRef.current);
                          setDrawings((current) => current.map((drawing) =>
                            drawing.type === "graded-rectangle" && drawing.id === target.id
                              ? { ...drawing, baselineSide: baseline, selected: true }
                              : { ...drawing, selected: false },
                          ));
                          setGradedRectContextMenu(null);
                        }}
                      >
                        {baseline === "top" ? "Set Top as Baseline" : "Set Bottom as Baseline"}
                      </button>
                    ))}
                  </div>
                </>
              )}
              {ctxMenu && (
                <>
                  <div
                    className="vora-floating-ui fixed inset-0 z-40"
                    style={{ cursor: "default" }}
                    onMouseDown={() => setCtxMenu(null)}
                    onContextMenu={(e) => { e.preventDefault(); setCtxMenu(null); }}
                  />
                  <div
                    role="menu"
                    className="vora-floating-ui fixed z-50 min-w-[250px] rounded-[3px] py-1 text-[13px]"
                    style={{
                      left: Math.max(8, Math.min(ctxMenu.x, window.innerWidth - 266)),
                      top: Math.max(8, Math.min(ctxMenu.y, window.innerHeight - 220)),
                      background: "#181818",
                      border: "1px solid #383838",
                      boxShadow: "0 8px 24px rgba(0,0,0,0.45)",
                    }}
                    onMouseDown={(e) => e.stopPropagation()}
                  >
                    <button
                      type="button"
                      role="menuitem"
                      onClick={() => {
                        setCtxMenu(null);
                        void handleResetZoom();
                      }}
                      className="flex h-10 w-full items-center gap-3 px-3 text-left text-[#d6dbe2] hover:bg-[#2a2a2a]"
                    >
                      <RotateCcw size={16} strokeWidth={1.7} />
                      <span className="flex-1">Reset chart view</span>
                      <span className="text-xs text-[#787b86]">Alt + R</span>
                    </button>
                    <button
                      type="button"
                      role="menuitem"
                      disabled={ctxMenu.priceText === null}
                      onClick={() => void handleCopyContextPrice()}
                      className="flex h-10 w-full items-center gap-3 px-3 text-left text-[#d6dbe2] hover:bg-[#2a2a2a] disabled:cursor-not-allowed disabled:opacity-40"
                    >
                      <span className="w-4 shrink-0" />
                      <span className="flex-1">Copy price {ctxMenu.priceText ?? ""}</span>
                    </button>
                    <div role="separator" className="my-1 h-px bg-[#383838]" />
                    <button
                      type="button"
                      role="menuitem"
                      disabled={lastTool == null}
                      onClick={() => {
                        if (lastTool === "rect") {
                          setRectToolActive(true);
                        }
                        setCtxMenu(null);
                      }}
                      className="w-full text-left px-3 py-1.5 hover:bg-[#2a2a2a] disabled:opacity-40 disabled:cursor-not-allowed text-[#d6dbe2]"
                    >
                      Repeat last tool: Rect
                    </button>
                    <button
                      type="button"
                      role="menuitem"
                      disabled={!drawings.filter(Boolean).some((d) => d.selected)}
                      onClick={() => {
                        const cur = drawingsRef.current.filter(Boolean);
                        if (!cur.some((d) => d.selected)) { setCtxMenu(null); return; }
                        undoHistory.push({ drawings: cur });
                        setHistoryTick((t) => t + 1);
                        setDrawings(cur.filter(Boolean).filter((d) => !d.selected));
                        setCtxMenu(null);
                      }}
                      className="w-full text-left px-3 py-1.5 hover:bg-[#2a2a2a] disabled:opacity-40 disabled:cursor-not-allowed text-[#d6dbe2]"
                    >
                      Clear this drawing
                    </button>
                  </div>
                </>
              )}
            </div>
            {replayOpen && <div className="chart-replay-dock">
              <ChartReplayFooter
                currentTimestamp={replayEndIndex === null
                  ? null
                  : replayTimestampOverridesRef.current?.end ?? candles[replayEndIndex]?.ts ?? null}
                selectedRange={replayRange}
                onSelectRange={selectReplayRange}
                onSelectDate={(timestamp) => {
                  const index = nearestReplayIndex(candles, timestamp);
                  if (index !== null) selectReplayStart(index);
                }}
              />
              <ChartReplayToolbar
                canSelectStart={candles.length > 0}
                canPlay={replayEndIndex !== null && replayEndIndex < candles.length - 1}
                canStepForward={replayEndIndex !== null && replayEndIndex < candles.length - 1}
                canJumpToLatest={replayEndIndex !== null && replayEndIndex < candles.length - 1}
                isSelecting={replaySelecting}
                isPlaying={replayPlaying}
                selectedTimestamp={replayStartIndex === null
                  ? null
                  : replayTimestampOverridesRef.current?.start ?? candles[replayStartIndex]?.ts ?? null}
                speed={replaySpeed}
                activeInterval={activeInterval}
                updateInterval={replayUpdateInterval}
                autoSelectUpdateInterval={replayAutoUpdateInterval}
                onSelectBarMode={beginReplayBarSelection}
                onSelectDateTime={(timestamp) => {
                  const index = nearestReplayIndex(candles, timestamp);
                  if (index !== null) selectReplayStart(index);
                }}
                onSelectFirstAvailable={() => { if (candles.length > 0) selectReplayStart(0); }}
                onSelectRandomBar={() => { if (candles.length > 0) selectReplayStart(Math.floor(Math.random() * candles.length)); }}
                onTogglePlayback={toggleReplayPlayback}
                onStepForward={stepReplayForward}
                onSetSpeed={setReplaySpeed}
                onSetUpdateInterval={setReplayUpdateInterval}
                onToggleAutoSelectUpdateInterval={() => setReplayAutoUpdateInterval((selected) => !selected)}
                onJumpToLatest={jumpReplayToLatest}
                onClose={closeBarReplay}
              />
            </div>}
            {narrativePlaybackSnapshot && (
              <NarrativePlayback
                snapshot={narrativePlaybackSnapshot}
                baseDrawings={narrativePlaybackBaseDrawings}
                speedFontSize={policy.narrative_speed_font_size}
                countFontSize={policy.narrative_count_font_size}
                onApplyDrawings={applyNarrativeDrawings}
                onApplyScale={applyNarrativeScale}
                onClose={closeNarrativePlayback}
              />
            )}

          </div>
        </div>
        <ChartRightDock
          activePanel={rightPanel}
          onPanelChange={setRightPanel}
          drawings={drawings}
          alerts={drawingAlerts}
          onToggleAlert={(id, enabled) => { const currentAlert = drawingAlerts.find((alert) => alert.id === id); const baseline = currentAlert?.lastEvaluatedAt ?? (candles.length > 0 ? candles[candles.length - 1].ts : null); void updateDrawingAlert(id, enabled, baseline).then(() => setDrawingAlerts((previous) => previous.map((alert) => alert.id === id ? { ...alert, enabled, lastEvaluatedAt: baseline } : alert))).catch(() => setToolbarToast82("Could not update drawing alert")); }}
          onDeleteAlert={(id) => { void deleteDrawingAlert(id).then(() => setDrawingAlerts((previous) => previous.filter((alert) => alert.id !== id))).catch(() => setToolbarToast82("Could not delete drawing alert")); }}
          onSelectDrawing={(id) => {
            setDrawings((previous) => previous.map((drawing) => ({
              ...drawing,
              hidden: drawing.id === id ? false : drawing.hidden,
              selected: drawing.id === id,
            })));
          }}
          onToggleDrawingHidden={(id, hidden) => {
            pushDrawingsHistory(drawingsRef.current);
            setDrawings((previous) => previous.map((drawing) => drawing.id === id ? { ...drawing, hidden, selected: hidden ? false : drawing.selected } : drawing));
          }}
        />
      </main>
    </div>
  );
};