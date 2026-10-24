# VORA Chart Offline Drawing Canvas

> **Status**: APPROVED
> **Target Scope**: `chart`

## Intent

Keep chart drawing tools usable when historical market data cannot be loaded.
An empty chart must still provide time and price coordinates for creating,
editing, selecting, and removing drawings.

## Behavior

- When the active chart has no candles, provide an invisible coordinate scaffold
  to Lightweight Charts. It supplies a usable time scale and price scale but
  must not render as market data or enter candle state, snapshots, or requests.
- Keep existing drawing overlays, tool interactions, undo/redo, and per-symbol
  drawing persistence active on the empty chart.
- Display a non-interactive status message that market data is unavailable and
  drawing tools remain available.
- Keep the most recently loaded close per symbol in local storage as a single
  reference value for the temporary empty-chart price scale. Do not cache or
  fall back to local candle history.
- When online candle data loads, replace the scaffold with the real dataset and
  restore the existing chart behavior.

## Non-Goals

- Offline market-data history or SQLite/provider fallback.
- Persisting scaffold points or treating them as candles.
- Changing drawing serialization or backend contracts.

## Verification

- `pnpm tsc --noEmit`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Manually verify line, rectangle, and measure tools on an empty chart, including
  selecting and undoing a drawing, then verify real candles replace the scaffold.
