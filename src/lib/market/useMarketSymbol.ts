/**
 * v0.86.1 (C1) — Foundation hook for the Option C WIP merge.
 *
 * Provides a single reactive source of truth for the currently
 * selected market symbol. All future consumers (candle fetches,
 * drawings storage, symbol selector UI, watchlist) must read
 * from this hook instead of the legacy hardcoded `SYMBOL`
 * constant in ChartPage.
 *
 * No side effects on import. No network calls. No chart
 * mutations. localStorage read happens only on first mount of a
 * consuming component.
 *
 * Persistence:
 *   - Key: `vora.current_symbol`
 *   - Default: `"XAUUSD"`
 *   - Validation: any non-empty string is accepted; a corrupted
 *     value falls back to the default.
 *
 * Legacy compatibility:
 *   - The legacy `SYMBOL = "XAUUSD"` constant in ChartPage is
 *     untouched by this chunk. C2 will replace its consumers.
 *   - Per-symbol localStorage drawers like
 *     `vora.drawings.${symbol}` are read/written by their own
 *     subsystems; this hook only exposes the symbol string.
 */

import { useSyncExternalStore } from "react";
import { getSetting, setSetting } from "@/shared/store/useSettingsStore";

const STORAGE_KEY = "vora.current_symbol";
const DEFAULT_SYMBOL = "XAUUSD";
const EVENT_NAME = "vora:symbol-change";

// ─── In-memory state ───────────────────────────────────────────
let current: string = DEFAULT_SYMBOL;
let hydrated = false;
const subscribers = new Set<() => void>();

// ─── Internal helpers ──────────────────────────────────────────
function safeReadStorage(): string | null {
  try {
    const raw = getSetting(STORAGE_KEY);
    if (typeof raw !== "string") return null;
    const trimmed = raw.trim();
    return trimmed.length > 0 ? trimmed : null;
  } catch {
    return null;
  }
}

function safeWriteStorage(value: string): void {
  try {
    setSetting(STORAGE_KEY, value);
  } catch {
    /* storage unavailable — ignore */
  }
}

function hydrateOnce(): void {
  if (hydrated) return;
  hydrated = true;
  const stored = safeReadStorage();
  if (stored && stored !== current) {
    current = stored;
  }
}

function notify(): void {
  for (const fn of subscribers) {
    try {
      fn();
    } catch {
      /* a bad subscriber must not break others */
    }
  }
}

function subscribe(fn: () => void): () => void {
  subscribers.add(fn);
  return () => {
    subscribers.delete(fn);
  };
}

function getSnapshot(): string {
  hydrateOnce();
  return current;
}

// Handle cross-tab updates (another tab writing to localStorage).
if (typeof window !== "undefined") {
  window.addEventListener("storage", (e) => {
    if (e.key !== STORAGE_KEY) return;
    const next = e.newValue;
    const normalized = next && next.trim() ? next.trim() : DEFAULT_SYMBOL;
    if (normalized !== current) {
      current = normalized;
      notify();
    }
  });
}

// ─── Public API ────────────────────────────────────────────────
export function getMarketSymbol(): string {
  hydrateOnce();
  return current;
}

export function setMarketSymbol(next: string): void {
  hydrateOnce();
  const normalized =
    typeof next === "string" && next.trim() ? next.trim() : DEFAULT_SYMBOL;
  if (normalized === current) return;
  current = normalized;
  safeWriteStorage(normalized);
  notify();
  if (typeof window !== "undefined") {
    try {
      window.dispatchEvent(new CustomEvent(EVENT_NAME, { detail: normalized }));
    } catch {
      /* ignore */
    }
  }
}

/**
 * React hook — returns the current symbol and a setter.
 *
 * Must be called from a component. The very first call site will
 * trigger the one-time localStorage hydration; every subsequent
 * call in the same tab shares the same in-memory value.
 */
export function useMarketSymbol(): {
  symbol: string;
  setSymbol: (next: string) => void;
} {
  const symbol = useSyncExternalStore(
    subscribe,
    getSnapshot,
    getSnapshot, // server snapshot — same value
  );
  return { symbol, setSymbol: setMarketSymbol };
}

/**
 * Non-React escape hatch for imperative code paths (async fetch
 * callbacks, event handlers created outside React). Reads only —
 * writes should go through `setMarketSymbol`.
 */
export const marketSymbol = {
  get: getMarketSymbol,
  set: setMarketSymbol,
  subscribe,
  EVENT_NAME,
  STORAGE_KEY,
  DEFAULT_SYMBOL,
} as const;