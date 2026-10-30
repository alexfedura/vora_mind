# Market Data & Charting Recon

**Scope:** Focused, read-only audit of the Rust `market/` module and frontend candle-fetching paths. **FACT** denotes repository evidence; **INFERENCE** denotes conclusions or recommendations. All source claims include file/line citations.

## 1. DUCKDB ENGINE & INITIALIZATION

| Question | Finding |
|---|---|
| Initialization | **FACT:** `fetch_local_candles` creates DuckDB with `Connection::open_in_memory()` for each request. It prepares the query on that connection, executes it, collects the result, and then returns; this is not a persistent DuckDB file or shared connection. [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L81-L129) |
| Connection flow | **FACT:** `mkt_fetch_local_candles` resolves a tick-data `PathBuf`, then uses `spawn_blocking` to pass the directory and request values to `fetch_local_candles`. It does not pass a DuckDB connection; the latter opens its own in-memory connection. [src-tauri/src/market/ipc.rs](../../src-tauri/src/market/ipc.rs#L42-L60), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L81-L119) |
| Dependency | **FACT:** DuckDB and Parquet support are enabled on the same crate dependency; there is no separate `parquet` crate dependency in this manifest line. [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml#L34) |

Exact dependency:

```toml
duckdb = { version = "1.0", features = ["bundled", "parquet"] }
```

## 2. CURRENT CANDLE FETCHING LOGIC

**FACT: IPC command and response DTO** ([src-tauri/src/market/ipc.rs](../../src-tauri/src/market/ipc.rs#L13-L59)):

```rust
pub struct CandleDto {
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub tick_count: i64,
    pub first_extreme: String,
}

pub async fn mkt_fetch_local_candles(
    app: tauri::AppHandle,
    symbol: String,
    interval_minutes: u32,
    start_ts: i64,
    end_ts: i64,
    limit: usize,
) -> Result<Vec<CandleDto>, String>
```

**FACT: Production query template.** `interval_minutes` is interpolated into the interval literal; the Parquet glob, start, end, and limit are bound parameters. [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L10-L59), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L113-L129)

```sql
WITH ticks AS (
    SELECT
        time_bucket(
            INTERVAL '{interval_minutes} minutes',
            TIMESTAMP 'epoch' + ts * INTERVAL '1 second'
        ) AS bucket,
        ts,
        price,
        volume
    FROM read_parquet(?)
    WHERE ts >= ? AND ts < ?
),
aggregated AS (
    SELECT
        bucket,
        first(price ORDER BY ts) AS open,
        max(price) AS high,
        min(price) AS low,
        last(price ORDER BY ts) AS close,
        arg_max(ts, price ORDER BY ts) AS high_ts,
        arg_min(ts, price ORDER BY ts) AS low_ts,
        sum(volume) AS volume,
        count(*) AS tick_count
    FROM ticks
    GROUP BY bucket
),
candles AS (
    SELECT
        epoch(aggregated.bucket)::BIGINT AS ts,
        aggregated.open,
        aggregated.high,
        aggregated.low,
        aggregated.close,
        aggregated.volume,
        aggregated.tick_count,
        CASE
            WHEN aggregated.high = aggregated.low
                OR aggregated.high_ts = aggregated.low_ts THEN 'same'
            WHEN aggregated.high_ts < aggregated.low_ts THEN 'high_first'
            ELSE 'low_first'
        END AS first_extreme
    FROM aggregated
)
SELECT ts, open, high, low, close, volume, tick_count, first_extreme
FROM candles
ORDER BY ts DESC
LIMIT ?
```

- **FACT:** The production read is `FROM read_parquet(?)`; the only `COPY ... FORMAT PARQUET` found in this path is test setup that writes a fixture, not a production import command. [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L21), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L181-L205)
- **FACT:** Development resolves the root to `<repo>/data/ticks`; packaged execution resolves it to `<resource_dir>/data/ticks`. The Tauri bundle includes `../data/ticks/**/*`. [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L63-L78), [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json#L34-L37)
- **FACT:** The query then requires a directory named by the uppercased symbol and recursively globs `*.parquet`: `<resolved-root>/<UPPERCASE_SYMBOL>/**/*.parquet`. The repository README documents `data/ticks/XAUUSD` and optional year subdirectories. [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L106-L123), [data/ticks/XAUUSD/README.md](../../data/ticks/XAUUSD/README.md#L1-L5)
- **INFERENCE:** The supplied `Root/01/tick/Forex/Metals/XAU-USD/...` layout will not match the current resolver/glob without an import/normalization step: `01` is encountered where the app expects the symbol directory, and `XAU-USD` also differs from the app's `XAUUSD` instrument name. The expected hierarchy is evidenced above; the mismatch follows from the supplied path and that code.
- **FACT:** Rust receives `interval_minutes: u32`, not a timeframe string. The Chart frontend converts an interval key to minutes and rejects intervals that are not whole minutes. For example, minute/hour units are represented in the selector and converted through seconds-per-unit before division by 60; tick, seconds, and M-scale options are disabled. [src/lib/market/chartIntervals.ts](../../src/lib/market/chartIntervals.ts#L12-L33), [src/lib/market/chartIntervals.ts](../../src/lib/market/chartIntervals.ts#L54-L73), [src/pages/ChartPage.tsx](../../src/pages/ChartPage.tsx#L1991-L2007)

## 3. EXPECTED SCHEMA

| Input field | Evidence / expected meaning |
|---|---|
| `ts` | **FACT:** The folder README requires numeric Unix seconds; the test fixture uses DuckDB `BIGINT`. The query multiplies `ts` by an interval of one second and returns bucket epoch as `BIGINT`. [data/ticks/XAUUSD/README.md](../../data/ticks/XAUUSD/README.md#L3-L5), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L14-L17), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L194-L195), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L39-L41) |
| `price` | **FACT:** The README requires a `price` column; the test fixture declares it `DOUBLE`. The query uses it as the OHLC source. The production reader does not cast or declare the Parquet type. [data/ticks/XAUUSD/README.md](../../data/ticks/XAUUSD/README.md#L3-L5), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L27-L32), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L194-L195) |
| `volume` | **FACT:** The README requires a `volume` column; the test fixture declares it `DOUBLE`. Production sums it per bucket. The production reader does not cast or declare the Parquet type. [data/ticks/XAUUSD/README.md](../../data/ticks/XAUUSD/README.md#L3-L5), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L33-L34), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L194-L195) |
| Raw Dukascopy columns | **FACT:** The query reads only `ts`, `price`, and `volume`; it does not refer to `time`, `ask`, `bid`, `ask_volume`, or `bid_volume`. [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L12-L23) **INFERENCE:** Such raw columns need an explicit conversion or a changed query; their physical types and time encoding have not been established by the current contract. |

**FACT: Candle DTO.** Rust serializes `ts`, OHLC, `volume`, `tick_count`, and `first_extreme`; the frontend interface mirrors those fields as JavaScript `number`s and a three-value string union. [src-tauri/src/market/ipc.rs](../../src-tauri/src/market/ipc.rs#L13-L22), [src/lib/market/marketApi.ts](../../src/lib/market/marketApi.ts#L56-L74)

## 4. FRONTEND CHART INTEGRATION

- **FACT:** `marketApi.ts` defines the typed request as `{ symbol, intervalMinutes, startTs, endTs, limit }` and invokes `mkt_fetch_local_candles`. It defines the candle response interface. [src/lib/market/marketApi.ts](../../src/lib/market/marketApi.ts#L56-L74)
- **FACT:** `ChartPage.tsx` calls `chartIntervalMinutes(interval)`, then requests up to 5,000 intervals ending at the current time or just after a viewport anchor. Its pagination path requests 500 older intervals. [src/pages/ChartPage.tsx](../../src/pages/ChartPage.tsx#L1991-L2007), [src/pages/ChartPage.tsx](../../src/pages/ChartPage.tsx#L2866-L2888)
- **FACT:** The separate `MarketPage` caller also sends `symbol`, `intervalSeconds / 60`, `startTs`, `endTs`, and `limit: 5000`. [src/pages/MarketPage.tsx](../../src/pages/MarketPage.tsx#L7-L45)
- **FACT:** The current static local instrument list includes `XAUUSD`, `XAGUSD`, and several FX pairs; the frontend labels them `Local Parquet`. [src/lib/market/marketApi.ts](../../src/lib/market/marketApi.ts#L44-L53)

## 5. EXISTING DATA IMPORT / MANAGEMENT

- **FACT:** The market IPC surface exposes `mkt_fetch_local_candles` and market analysis/configuration commands; the fetch command resolves the fixed tick-data root. The command registration includes fetch and analysis commands, with no tick-file import command in this audited market command list. [src-tauri/src/market/ipc.rs](../../src-tauri/src/market/ipc.rs#L42-L60), [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs#L1503-L1519)
- **FACT:** `MarketPage` loads from the static `LOCAL_MARKET_INSTRUMENTS` list and calls the fetch API; no file chooser/import action appears in that component. [src/pages/MarketPage.tsx](../../src/pages/MarketPage.tsx#L1-L64), [src/lib/market/marketApi.ts](../../src/lib/market/marketApi.ts#L44-L74)
- **FACT:** The app's â€œSourcesâ€� page lists and adds Vora source records through `srcListSources`, `srcGetSourceBundle`, and `AddSourceModal`; it is not wired to market Parquet files. [src/pages/SourcesPage.tsx](../../src/pages/SourcesPage.tsx#L1-L10), [src/pages/SourcesPage.tsx](../../src/pages/SourcesPage.tsx#L28-L48), [src/pages/SourcesPage.tsx](../../src/pages/SourcesPage.tsx#L159-L165)
- **FACT:** `DatabaseSettings` can import a `.vora` database backup or `.json` settings file and displays the current SQLite database path. It does not expose a tick-data folder setting. [src/components/DatabaseSettings.tsx](../../src/components/DatabaseSettings.tsx#L16-L29), [src/components/DatabaseSettings.tsx](../../src/components/DatabaseSettings.tsx#L55-L69), [src/components/DatabaseSettings.tsx](../../src/components/DatabaseSettings.tsx#L97-L108), [src/components/DatabaseSettings.tsx](../../src/components/DatabaseSettings.tsx#L175-L176)
- **FACT:** Tick files are expected to be placed under the repository/package `data/ticks` resource tree; the XAUUSD README describes placement there and nested Parquet files. [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L63-L78), [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json#L34-L37), [data/ticks/XAUUSD/README.md](../../data/ticks/XAUUSD/README.md#L1-L5)

## 6. RECOMMENDED INTEGRATION POINT (INFERENCE)

- **INFERENCE:** Put the file-import command in `src-tauri/src/market/ipc.rs`, its blocking DuckDB transform in `src-tauri/src/market/local_db.rs`, and register the command in `src-tauri/src/lib.rs` beside the existing candle-fetch command. This reuses the established IPC-to-`spawn_blocking`-to-market-module flow and existing DuckDB `parquet` feature; no new library appears necessary. [src-tauri/src/market/ipc.rs](../../src-tauri/src/market/ipc.rs#L42-L60), [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs#L1503-L1519), [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml#L34)
- **INFERENCE:** Add the user-facing source-folder selection/status to the market workspace, not `SourcesPage` or `DatabaseSettings`; those currently represent Vora source records and SQLite backup/settings management respectively. [src/pages/MarketWorkspace.tsx](../../src/pages/MarketWorkspace.tsx#L100-L120), [src/pages/SourcesPage.tsx](../../src/pages/SourcesPage.tsx#L1-L10), [src/components/DatabaseSettings.tsx](../../src/components/DatabaseSettings.tsx#L55-L69)
- **INFERENCE:** Discover monthly files recursively from the selected root, e.g. a glob conceptually equivalent to `<selected-root>/**/tick/Forex/Metals/XAU-USD/*.parquet`, then write normalized output under the app's symbol-first convention (`<writable-tick-root>/XAUUSD/<year>/*.parquet`). The current release resolver points inside `resource_dir`, which may not be a writable install location; an import destination should be writable and the resolver should read the same destination. The existing resolver/glob and bundle root are documented at [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L63-L78), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L106-L123), and [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json#L34-L37).
- **INFERENCE:** Normalize each raw row to the current contract before writing: convert the verified `time` representation to Unix-second `ts`, choose and document a scalar `price` policy (midpoint `(ask + bid) / 2` is a reasonable starting convention), and map the two side volumes to scalar `volume` only after confirming their units/semantics. The existing consumer requires `ts`, `price`, and `volume`; it does not consume ask/bid columns. [data/ticks/XAUUSD/README.md](../../data/ticks/XAUUSD/README.md#L3-L5), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L12-L23), [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L33-L34)
- **INFERENCE:** Dynamic timeframe aggregation is already implemented, not pre-aggregated: the request's minute value drives DuckDB `time_bucket` over tick rows read from Parquet. Keep normalized tick-level Parquet as the import output; no separate pre-aggregation job is required for supported whole-minute intervals. To enable seconds, tick-count, or non-whole-minute M-scale candles, the UI validation and backend query/API contract would need extension; the current UI explicitly disables those intervals. [src-tauri/src/market/local_db.rs](../../src-tauri/src/market/local_db.rs#L10-L37), [src/lib/market/chartIntervals.ts](../../src/lib/market/chartIntervals.ts#L12-L33), [src/lib/market/chartIntervals.ts](../../src/lib/market/chartIntervals.ts#L68-L73)
