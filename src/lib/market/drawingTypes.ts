export type LineStyle = "solid" | "dashed" | "dotted";
export type DrawingTextAlign = "left" | "center" | "right";
export type DrawingTextVerticalAlign = "top" | "middle" | "bottom";
export type DrawingIconCategory = "emoji" | "sticker" | "icon";
export interface DrawingIconSelection { category: DrawingIconCategory; symbol: string; }
export type GeometryToolId = "brush" | "highlighter" | "arrow-marker" | "arrow" | "arrow-mark-up" | "arrow-mark-down" | "rotated-rectangle" | "path" | "circle" | "ellipse" | "polyline" | "triangle-shape" | "arc" | "curve" | "double-curve" | "rr_long" | "rr_short" | "position-forecast" | "bars-pattern" | "ghost-feed" | "sector" | "anchored-vwap" | "fixed-range-volume-profile" | "anchored-volume-profile" | "price_range" | "date_range" | "date_price_range";
export const GEOMETRY_TOOL_LABELS: Record<GeometryToolId, string> = {
  brush: "Brush",
  highlighter: "Highlighter",
  "arrow-marker": "Arrow marker",
  arrow: "Arrow",
  "arrow-mark-up": "Arrow mark up",
  "arrow-mark-down": "Arrow mark down",
  "rotated-rectangle": "Rotated rectangle",
  path: "Path",
  circle: "Circle",
  ellipse: "Ellipse",
  polyline: "Polyline",
  "triangle-shape": "Triangle",
  arc: "Arc",
  curve: "Curve",
  "double-curve": "Double curve",
  rr_long: "Long position",
  rr_short: "Short position",
  "position-forecast": "Position forecast",
  "bars-pattern": "Bars pattern",
  "ghost-feed": "Ghost feed",
  sector: "Sector",
  "anchored-vwap": "Anchored VWAP",
  "fixed-range-volume-profile": "Fixed range volume profile",
  "anchored-volume-profile": "Anchored volume profile",
  price_range: "Price range",
  date_range: "Date range",
  date_price_range: "Date and price range",
};
export interface DrawingAnchor { ts: number; price: number; }
export interface DrawingCandleSample { ts: number; open: number; high: number; low: number; close: number; volume: number; }
export type FibGannToolId =
  | "fib-retracement"
  | "fib-extension"
  | "fib-channel"
  | "fib-time-zone"
  | "fib-speed-resistance-fan"
  | "trend-based-fib-time"
  | "fib-circles"
  | "fib-spiral"
  | "fib-speed-resistance-arcs"
  | "fib-wedge"
  | "pitchfan"
  | "gann-box"
  | "gann-square-fixed"
  | "gann-square"
  | "gann-fan";

export const FIB_GANN_TOOL_LABELS: Record<FibGannToolId, string> = {
  "fib-retracement": "Fib retracement",
  "fib-extension": "Trend-based fib extension",
  "fib-channel": "Fib channel",
  "fib-time-zone": "Fib time zone",
  "fib-speed-resistance-fan": "Fib speed resistance fan",
  "trend-based-fib-time": "Trend-based fib time",
  "fib-circles": "Fib circles",
  "fib-spiral": "Fib spiral",
  "fib-speed-resistance-arcs": "Fib speed resistance arcs",
  "fib-wedge": "Fib wedge",
  pitchfan: "Pitchfan",
  "gann-box": "Gann box",
  "gann-square-fixed": "Gann square fixed",
  "gann-square": "Gann square",
  "gann-fan": "Gann fan",
};

export interface DrawingPresentation {
  colorOpacity?: number;
  hidden?: boolean;
  visibleOnIntervals?: number[];
  name?: string;
  style?: LineStyle;
  text?: string;
  textColor?: string;
  textOpacity?: number;
  textSize?: number;
  textBold?: boolean;
  textItalic?: boolean;
  textAlignHorizontal?: DrawingTextAlign;
  textAlignVertical?: DrawingTextVerticalAlign;
}

export function isDrawingVisible(drawing: DrawingPresentation): boolean {
  return drawing.hidden !== true;
}

export type DrawingKind = "rectangle" | "graded-rectangle" | "vline" | "hline" | "hray" | "trendline" | "ray" | "info-line" | "extended-line" | "trend-angle" | "crossline" | "parallel-channel" | "pitchfork" | "regression-trend" | "fib-gann" | "icon" | "geometry";
export type DrawingAlertCondition = "entering_drawing" | "crossing";

export interface DrawingStyleTemplateSettings {
  color: string;
  colorOpacity: number;
  lineWidth: number;
  style: LineStyle;
  fillColor: string | null;
  fillOpacity: number;
  text: string;
  textColor: string;
  textOpacity: number;
  textSize: number;
  textBold: boolean;
  textItalic: boolean;
  textAlignHorizontal: DrawingTextAlign;
  textAlignVertical: DrawingTextVerticalAlign;
}

export interface DrawingStyleTemplate {
  id: string;
  name: string;
  drawingType: DrawingKind;
  settings: DrawingStyleTemplateSettings;
  createdAt: number;
}

export interface ChartDrawingAlert {
  id: string;
  symbol: string;
  timeframe: number;
  drawingId: string;
  condition: DrawingAlertCondition;
  message: string;
  triggerOnce: boolean;
  notifyApp: boolean;
  notifyToast: boolean;
  notifyEmail: boolean;
  notifySound: boolean;
  expirationAt: number | null;
  enabled: boolean;
  createdAt: number;
  lastEvaluatedAt: number | null;
}