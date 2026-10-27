# Local Parquet Market Data

## Goal

Replace the chart's online Dukascopy candle fetch path with local Parquet tick
data queried by DuckDB, while preserving the existing `CandleDto` shape and
chart load/older-history workflows.

## Input data contract

- Files are stored below `data/ticks/{SYMBOL}/` and may be nested by year or
  other partition directories.
- Each Parquet row has `ts`, `price`, and `volume` columns.
- `ts` is a numeric Unix timestamp in seconds; `price` and `volume` are
  numeric.
- Symbols are restricted to ASCII letters, digits, `_`, and `-` before they
  are used in a filesystem path.
- Candle timestamps are Unix seconds at the start of fixed-duration buckets.

## Backend behavior

- Use bundled DuckDB with Parquet support and an in-memory connection per read.
- Resolve the dataset from the source checkout in development and the Tauri
  resource directory in a packaged build.
- Execute blocking DuckDB work on Tokio's blocking thread pool.
- Accept only positive whole-minute intervals and bounded request sizes/ranges.
- Use `[start_ts, end_ts)` tick bounds, return no more than `limit` candles in
  chronological order, and return explicit errors for missing files, bad input,
  and query/schema failures.
- Compute OHLC, summed volume, tick count, and the first high/low occurrence
  from the tick rows. A flat bucket has `first_extreme = "same"`.
- Package the root `data/ticks` tree as Tauri resources. Users may populate the
  checked-in directory with local datasets before building the application.

## Frontend behavior

- Use a typed Tauri invoke wrapper that returns the existing `CandleDto[]`.
- Convert chart intervals into exact whole-minute buckets; reject intervals
  that cannot be represented without rounding rather than silently changing
  their meaning.
- Initial, restore, older-history, and market-page fetches use the local
  command. Request windows are derived from the requested interval and candle
  count.
- Candles remain page-local React state; no new frontend persistence is added.

## Verification

- Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- Run `pnpm tsc --noEmit`.
- Verify DuckDB SQL returns chronological `CandleDto` rows for the stated
  Parquet schema and inspect the exact query used by the backend.
