use std::fs;
use std::path::{Path, PathBuf};

use duckdb::{params, Connection};
use tauri::Manager;

use super::ipc::CandleDto;

const MAX_LIMIT: usize = 5_000;

pub fn resolve_tick_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    #[cfg(debug_assertions)]
    {
        let source_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("data")
            .join("ticks");
        if source_dir.is_dir() {
            return Ok(source_dir);
        }
    }

    app.path()
        .resource_dir()
        .map(|path| path.join("data").join("ticks"))
        .map_err(|error| format!("Could not resolve bundled tick-data directory: {error}"))
}

fn contains_parquet_file(directory: &Path) -> Result<bool, String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("Could not read tick-data directory: {error}"))?;

    for entry in entries {
        let entry = entry.map_err(|error| format!("Could not inspect tick-data entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            if contains_parquet_file(&path)? {
                return Ok(true);
            }
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("parquet"))
        {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn fetch_local_candles(
    data_dir: PathBuf,
    symbol: String,
    interval_minutes: u32,
    start_ts: i64,
    end_ts: i64,
    limit: usize,
) -> Result<Vec<CandleDto>, String> {
    if symbol.is_empty()
        || !symbol
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err("Invalid symbol: only ASCII letters, digits, '_' and '-' are allowed".into());
    }
    if interval_minutes == 0 || interval_minutes > 525_600 {
        return Err("Invalid interval_minutes: expected 1..=525600".into());
    }
    if start_ts >= end_ts {
        return Err("Invalid time range: start_ts must be less than end_ts".into());
    }
    if limit == 0 || limit > MAX_LIMIT {
        return Err(format!("Invalid limit: expected 1..={MAX_LIMIT}"));
    }

    let symbol_dir = data_dir.join(symbol.to_ascii_uppercase());
    if !symbol_dir.is_dir() {
        return Err(format!(
            "No local tick-data directory found for symbol {}",
            symbol.to_ascii_uppercase()
        ));
    }
    if !contains_parquet_file(&symbol_dir)? {
        return Err(format!(
            "No Parquet tick files found for symbol {}",
            symbol.to_ascii_uppercase()
        ));
    }

    let parquet_glob = symbol_dir.join("**").join("*.parquet");
    let connection = Connection::open_in_memory()
        .map_err(|error| format!("Could not open in-memory DuckDB: {error}"))?;
    let query = format!(
        "WITH ticks AS (
            SELECT
                time_bucket(INTERVAL '{interval_minutes} minutes', to_timestamp(ts)) AS bucket,
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
                sum(volume) AS volume,
                count(*) AS tick_count
            FROM ticks
            GROUP BY bucket
        ),
        extreme_times AS (
            SELECT
                aggregated.bucket,
                min(ticks.ts) FILTER (WHERE ticks.price = aggregated.high) AS high_ts,
                min(ticks.ts) FILTER (WHERE ticks.price = aggregated.low) AS low_ts
            FROM aggregated
            JOIN ticks USING (bucket)
            GROUP BY aggregated.bucket, aggregated.high, aggregated.low
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
                        OR extreme_times.high_ts = extreme_times.low_ts THEN 'same'
                    WHEN extreme_times.high_ts < extreme_times.low_ts THEN 'high_first'
                    ELSE 'low_first'
                END AS first_extreme
            FROM aggregated
            JOIN extreme_times USING (bucket)
        )
        SELECT ts, open, high, low, close, volume, tick_count, first_extreme
        FROM candles
        ORDER BY ts DESC
        LIMIT ?
        "
    );

    let mut statement = connection
        .prepare(&query)
        .map_err(|error| format!("Could not prepare local candle query: {error}"))?;
    let rows = statement
        .query_map(
            params![
                parquet_glob.to_string_lossy().as_ref(),
                start_ts,
                end_ts,
                limit as i64
            ],
            |row| {
                Ok(CandleDto {
                    ts: row.get(0)?,
                    open: row.get(1)?,
                    high: row.get(2)?,
                    low: row.get(3)?,
                    close: row.get(4)?,
                    volume: row.get(5)?,
                    tick_count: row.get(6)?,
                    first_extreme: row.get(7)?,
                })
            },
        )
        .map_err(|error| format!("Local candle query failed: {error}"))?;
    let mut candles = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not read local candle rows: {error}"))?;
    candles.reverse();
    Ok(candles)
}
