# Vora Chart Dukascopy Lazy Loading

> **Status**: APPROVED
> **Author**: GitHub Copilot
> **Target Scope**: `market | chart`

## 1. Intent & System Goals

* **Objective**: Make Dukascopy the only source for chart price history and remove legacy local-file ingestion and local candle fallback.
* **Target Workflow**: `ChartPage.tsx` watches the visible logical range. When `from < 10`, it requests older candles through `mkt_fetch_historical_duka`. Rust fetches and caches provider results in process memory only. If Dukascopy is unavailable, the request fails without reading historical candles from SQLite. React prepends unique candles and preserves the visible range.
* **Legacy Removal**: Delete the checked-out Parquet market dataset, its CLI registration/aggregation/warmup path, market-source discovery backed by the local registry, and SQLite candle fallback for online chart requests. Keep SQLite candle tables only for existing M-Scale/box workflows; they are not a source for chart history. Keep the in-memory cache for provider results.
* **Non-Goals**: Remove SQLite-backed M-Scale/box workflows, migrate chart state to a frontend store, or refactor replay/timeframe loading.

## 2. Affected Architectural Boundaries

* `src/lib/market/marketApi.ts`: typed Tauri invoke wrapper.
* `src/pages/ChartPage.tsx`: older-candle request state, left-edge detection, and prepend.
* `src/lib/market/CandlestickChart.tsx`: setData and preserve/offset the logical viewport on a prepend.
* `src/pages/ChartPage.css`: older-history loading indicator.
* `src-tauri/Cargo.toml`: Dukascopy and Tokio feature dependencies.
* `src-tauri/src/market/dukascopy.rs`: Dukascopy request, mapping, and process-local candle cache.
* `src-tauri/src/market/ipc.rs`: request validation and `mkt_fetch_historical_duka`; no local fallback.
* `src-tauri/src/lib.rs`: register the online history command and retain unrelated market commands.
* `src-tauri/src/bin/vora_market.rs`, `src-tauri/src/market/parquet_io.rs`, `src-tauri/src/market/tick_reader.rs`, and Parquet-only aggregation code: remove the local-file ingest pipeline.
* `src/components/market/SymbolSelector.tsx`, `src/components/market/WatchlistPanel.tsx`, and `src/lib/market/marketApi.ts`: remove local market-source discovery and expose online instruments only.
* `data/market/`: remove the legacy local market dataset.

## 3. Data Contracts

The frontend command returns the existing market `CandleDto` shape: `ts`, `open`, `high`, `low`, `close`, `volume`, `tick_count`, and `first_extreme`. Dukascopy crate candles contain timestamp/OHLCV only. Convert Dukascopy millisecond timestamps to Vora Unix seconds; set tick count and first-extreme fields to explicit compatible defaults when the provider does not supply them.

Command arguments are `symbol: String`, `timeframe: String`, `end_timestamp: i64` (seconds, exclusive upper bound), and `limit: usize`. Results are ascending, unique candles strictly older than `end_timestamp`.

The process-local cache is keyed by `{symbol}_{timeframe}`. Cache hits must be filtered to the requested exclusive end timestamp and return at most the nearest `limit` older candles. Successful fetches merge by timestamp and remain sorted; failed or rate-limited requests are not cached.

## 4. Dukascopy API Constraints

`dukascopy_rust` 0.2.0 exposes an async batch `fetch(instrument, interval, offer_side, last_update, limit, client)` and a `stream` helper. Its `Candle.timestamp` is milliseconds. The helper's stream advances from `start` toward `end`, yields raw JSON rows, performs a fetch per yielded row, and does not surface fetch errors as stream items. For historical paging, prefer bounded batch fetches and convert/map rows using the crate's `Candle` model; do not issue one HTTP request per candle. Normalize Dukascopy instrument/timeframe identifiers at the backend boundary and return clear unsupported-input/provider errors.

Provider HTTP 429/rate-limit failures must return a stable `DUKASCOPY_RATE_LIMIT` error prefix. Connection failures return a stable `DUKASCOPY_CONNECTION_FAILED` prefix. No provider error may trigger a SQLite or local-file history read. Other provider and conversion errors must remain distinguishable and must not replace the existing chart dataset. Raw request URLs belong in backend logs, not user-visible chart errors.

## 5. Implementation Plan

1. Implement bounded historical fetches, timestamp conversion, mapping to `CandleDto`, and a process-local cache in the Dukascopy module.
2. Expose `mkt_fetch_historical_duka` through Tauri IPC and ensure provider errors return directly without consulting SQLite.
3. Remove local-file registration, candle aggregation/warmup commands, Parquet-only modules/dependencies, and local market-source browsing.
4. Replace local source discovery in chart selectors with the supported online instrument list.
5. Delete the legacy `data/market/` dataset. Keep SQLite candle tables only where separate M-Scale/box flows still require them.
6. Preserve the single-flight older-history request and chart viewport when prepending provider candles.
7. Bump the application version consistently and commit the focused online-only change.

## 6. Verification & Acceptance Criteria

* [ ] `pnpm tsc --noEmit` succeeds.
* [ ] `cargo check --manifest-path src-tauri/Cargo.toml` succeeds.
* [ ] Reaching the left threshold issues at most one request at a time.
* [ ] Successful older results prepend in ascending order without timestamp duplicates.
* [ ] The same visible candles stay at the same screen positions after prepend; `fitContent` is not called for this update.
* [ ] Empty results stop repeated requests at the oldest available history; provider errors leave chart data unchanged and clear loading state.
* [ ] Provider failures never read local SQLite candle history, and no UI/API path discovers market sources from the local file registry.