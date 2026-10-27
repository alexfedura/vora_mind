import { invoke } from "@tauri-apps/api/core";
import { getSetting } from "@/shared/store/useSettingsStore";
import type { ChartDrawingAlert, DrawingStyleTemplate } from "@/lib/market/drawingTypes";

export async function listDrawingTemplates(): Promise<DrawingStyleTemplate[]> {
  return invoke<DrawingStyleTemplate[]>("mkt_list_drawing_templates");
}

export async function saveDrawingTemplate(template: DrawingStyleTemplate): Promise<void> {
  await invoke("mkt_save_drawing_template", { template });
}

export async function deleteDrawingTemplate(id: string): Promise<void> {
  await invoke("mkt_delete_drawing_template", { id });
}

export async function listDrawingAlerts(symbol: string): Promise<ChartDrawingAlert[]> {
  return invoke<ChartDrawingAlert[]>("mkt_list_drawing_alerts", { symbol });
}

export async function saveDrawingAlert(alert: ChartDrawingAlert): Promise<void> {
  await invoke("mkt_save_drawing_alert", { alert });
}

export async function updateDrawingAlert(id: string, enabled: boolean, lastEvaluatedAt: number | null): Promise<void> {
  await invoke("mkt_update_drawing_alert", { id, enabled, lastEvaluatedAt });
}

export async function deleteDrawingAlert(id: string): Promise<void> {
  await invoke("mkt_delete_drawing_alert", { id });
}

export async function deleteDrawingAlertsForDrawings(symbol: string, drawingIds: string[]): Promise<void> {
  if (drawingIds.length === 0) return;
  await invoke("mkt_delete_drawing_alerts_for_drawings", { symbol, drawingIds });
}

export interface MarketInstrument {
  id: string;
  symbol: string;
  kind: string;
  row_count: number;
}

export const LOCAL_MARKET_INSTRUMENTS: MarketInstrument[] = [
  { id: "local-XAUUSD", symbol: "XAUUSD", kind: "Local Parquet", row_count: 0 },
  { id: "local-XAGUSD", symbol: "XAGUSD", kind: "Local Parquet", row_count: 0 },
  { id: "local-EURUSD", symbol: "EURUSD", kind: "Local Parquet", row_count: 0 },
  { id: "local-GBPUSD", symbol: "GBPUSD", kind: "Local Parquet", row_count: 0 },
  { id: "local-USDJPY", symbol: "USDJPY", kind: "Local Parquet", row_count: 0 },
  { id: "local-AUDUSD", symbol: "AUDUSD", kind: "Local Parquet", row_count: 0 },
  { id: "local-USDCAD", symbol: "USDCAD", kind: "Local Parquet", row_count: 0 },
  { id: "local-USDCHF", symbol: "USDCHF", kind: "Local Parquet", row_count: 0 },
  { id: "local-NZDUSD", symbol: "NZDUSD", kind: "Local Parquet", row_count: 0 },
];

export interface CandleDto {
  ts: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  tick_count: number;
  first_extreme: "high_first" | "low_first" | "same";
}

export async function mktFetchLocalCandles(req: {
  symbol: string;
  intervalMinutes: number;
  startTs: number;
  endTs: number;
  limit: number;
}): Promise<CandleDto[]> {
  return invoke<CandleDto[]>("mkt_fetch_local_candles", req);
}

// M-Scale helpers
export function stepLabel(step: number): string {
  const labels: Record<number, string> = {
    [-3]: "1 ms", [-2]: "10 ms", [-1]: "100 ms",
    0: "1 sec", 1: "10 sec", 2: "1.67 min", 3: "16.67 min",
    4: "2.78 hr", 5: "27.78 hr", 6: "11.57 days",
    7: "115.7 days", 8: "3.17 years", 9: "31.7 years",
  };
  return labels[step] ?? `M${step}`;
}

export function stepKey(step: number): string {
  return `M${step}`;
}

// ─── Box Detection DTOs ───

export type BoxPattern = "RBR" | "DBD" | "RBD" | "DBR";
export type BoxZone = "supply" | "demand";
export type BoxRole = "small" | "medium" | "large";

export interface BoxDto {
  address: string;
  symbol: string;
  step: number;
  pivot_step: number;
  pattern: BoxPattern;
  zone: BoxZone;
  scale: number;
  role: BoxRole;
  x_idx: number;
  ts_start: number;
  ts_end: number;
  engulf_ts: number | null;
  duration_sec: number;
  scale_sec: number;
  high: number;
  low: number;
  origin_price: number;
  leg1_magnitude: number;
  leg2_magnitude: number;
  parent_address: string | null;
  children_addresses: string[];
  x: number;
  y_abs: number;
  z: number;
  y_band: number;
  chain_id: number | null;
  chain_pos: number | null;
  y_lane: number | null;
}

export interface BoxExtend {
  left:  number;   // fractional candles, -20..+20
  right: number;   // fractional candles, -20..+20
}

export interface ChainDto {
  id: number;
  y_band: number;
  z: number;
  boxes: string[];
  kind_seq: string[];
}

export interface BoxParams {
  user_step?:       number;
  s_small?:      number;
  s_medium?:     number;
  s_large?:      number;
  z0_scale?:    number;
  extend_to_engulf?: boolean;
}

export async function detectBoxes(
  symbol: string,
  step: number,
  params?: BoxParams,
  opts?: { requestId: number; cancelPrevious?: number },
): Promise<{ boxes: BoxDto[]; chains: ChainDto[] }> {
  return invoke<{ boxes: BoxDto[]; chains: ChainDto[] }>("mkt_detect_boxes", {
    req: { symbol, step, params: params ?? null },
    requestId: opts?.requestId ?? null,
    cancelPrevious: opts?.cancelPrevious ?? null,
  });
}

export async function cancelRequest(requestId: number): Promise<void> {
  try {
    await invoke("mkt_cancel_request", { requestId });
  } catch {
    /* best-effort */
  }
}

export interface SymbolOriginDto {
  symbol:       string;
  step:      number;
  price_ref:    number;
  price_ref_ts: number;
}

export async function getSymbolOrigin(
  symbol: string, step: number,
): Promise<SymbolOriginDto | null> {
  return invoke<SymbolOriginDto | null>(
    "mkt_get_symbol_origin", { symbol, step });
}

export async function refreshSymbolOrigin(
  symbol: string, step: number,
): Promise<SymbolOriginDto> {
  return invoke<SymbolOriginDto>(
    "mkt_refresh_symbol_origin", { symbol, step });
}

// ─── ScaleRegistry (v5.1.8 — data model only, no UI) ───

export interface Scale {
  id: string;
  label: string;
  start_pct: number;
  end_pct: number;
  tolerance_pct?: number; // hidden from UI
  color: string;
  enabled: boolean;
  sort_order: number;
}

export interface ScaleRegistry {
  symbol: string;
  step?: number;
  scales: Scale[];
}

export async function getScaleRegistry(
  symbol: string,
  step?: number,
): Promise<ScaleRegistry> {
  return invoke<ScaleRegistry>("mkt_get_scale_registry", {
    symbol,
    step: step ?? null,
  });
}

export async function setScaleRegistry(
  registry: ScaleRegistry,
): Promise<void> {
  return invoke<void>("mkt_set_scale_registry", { registry });
}

export async function listScaleRegistries(): Promise<Array<[string, number | null, number]>> {
  return invoke<Array<[string, number | null, number]>>("mkt_list_scale_registries");
}

export interface PivotDto {
  idx: number;
  ts: number;
  price: number;
  kind: "H" | "L";
}

export async function listPivots(symbol: string, m: number): Promise<PivotDto[]> {
  return invoke<PivotDto[]>("mkt_list_pivots", { symbol, m });
}
