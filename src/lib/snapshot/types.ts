import type { DrawingCandleSample, DrawingIconCategory, DrawingTextAlign, DrawingTextVerticalAlign, FibGannToolId, GeometryToolId } from "@/lib/market/drawingTypes";
import type { DrawingAny } from "@/lib/market/lineDrawings";

export interface ActiveScale {
  x: number;
  y: number;
  s: number;
}

export type NarrativeMode = "FULL_VIDEO" | "COMPACT_STEPS";

export interface NarrativeEvent {
  timestamp: number;
  drawings: DrawingAny[];
  activeScale: ActiveScale;
}

export interface CompactStep {
  stepId: number;
  committedDrawing: DrawingAny;
  activeScale: ActiveScale;
}

export const SNAPSHOT_FORMAT = "vora-snapshot" as const;
export const SNAPSHOT_SCHEMA_VERSION = 2;

export interface SnapshotViewport {
  fromTs: number;
  toTs: number;
  fromLogical: number;
  toLogical: number;
  barSpacing: number;
  rightOffset: number;
  crosshair?: { ts: number; price: number };
  windowW: number;
  windowH: number;
  dpr: number;

  // v0.83.4 — vertical price range + autoScale state
  priceFrom?: number;
  priceTo?: number;
  autoScale?: boolean;

  // v0.83.6 — center anchor point (chart center at capture time)
  anchorTs?: number; // seconds (LC native)
  anchorPrice?: number;
}

export interface SnapshotChartState {
  symbol: string;
  exchange: string;
  interval: string;
  dataType: "candles" | "heikin" | "line" | "bar";
  displayMode: "candlestick" | "special";
  priceScale: "linear" | "logarithmic" | "percent";
  timezone: string;
  currency: string;
}

interface SnapshotDrawingBase {
  id: string;
  name?: string;
  points: Array<{ ts: number; price: number }>;
  color: string;
  lineWidth: number;
  style: "solid" | "dashed" | "dotted";
  bgColor?: string;
  text?: string;
  textColor?: string;
  textOpacity?: number;
  textSize?: number;
  textBold?: boolean;
  textItalic?: boolean;
  textAlignHorizontal?: DrawingTextAlign;
  textAlignVertical?: DrawingTextVerticalAlign;
  locked?: boolean;
  hidden?: boolean;
  visibleOnIntervals?: number[];
  colorOpacity?: number;
  sourceBars?: DrawingCandleSample[];
}

export type SnapshotDrawing = SnapshotDrawingBase & (
  | { type: "vline" | "hline" | "hray" | "trendline" | "rect" | "ray" | "info-line" | "extended-line" | "trend-angle" | "crossline" | "regression-trend" }
  | { type: "parallel-channel" | "pitchfork" }
  | { type: "icon"; category: DrawingIconCategory; symbol: string; size: number }
  | { type: "fib-gann"; tool: FibGannToolId }
  | { type: "geometry"; tool: GeometryToolId }
  | {
      type: "graded-rectangle";
      structureType: "RBR" | "DBD";
      anchorPrice: number;
      anchorTime: number;
      baselineSide: "top" | "bottom";
      correctionStartPrice?: number;
      correctionStartTime?: number;
    }
);

export interface SnapshotPolicy {
  scaleBase: number;
  roles: { small: number; medium: number; large: number };
  registry: Array<{ id: number; color: string; min: number; max: number }>;
}

export interface SnapshotTheme {
  background: string;
  gridColor: string;
  upColor: string;
  downColor: string;
  textColor: string;
}

export interface VoraSnapshot {
  format: typeof SNAPSHOT_FORMAT;
  schemaVersion: number;
  id: string;
  name: string;
  notes: string;
  tags: string[];
  createdAt: number;
  updatedAt: number;

  chart: SnapshotChartState;
  viewport: SnapshotViewport;
  drawings: SnapshotDrawing[];
  policy: SnapshotPolicy;
  theme: SnapshotTheme;

  narrativeMode?: NarrativeMode;
  narrativeLog?: NarrativeEvent[] | CompactStep[];

  checksum: string;
}

export interface SnapshotMetadata {
  id: string;
  name: string;
  symbol: string;
  interval: string;
  createdAt: number;
  updatedAt: number;
  drawingCount: number;
  tags: string[];
}
