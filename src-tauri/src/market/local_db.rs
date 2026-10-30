use std::path::PathBuf;

use duckdb::{params, Connection};
use tauri::Manager;

use super::ipc::CandleDto;

const MAX_LIMIT: usize = 5_000;

fn build_candle_query(interval_minutes: u32) -> String {
    format!(
        "WITH ticks AS (
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
        "
    )
}

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
    if interval_minutes == 0 || interval_minutes > 527_040 {
        return Err("Invalid interval_minutes: expected 1..=527040".into());
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
    let parquet_glob = symbol_dir.join("**").join("*.parquet");
    let connection = Connection::open_in_memory()
        .map_err(|error| format!("Could not open in-memory DuckDB: {error}"))?;
    let mut statement = connection
        .prepare(&build_candle_query(interval_minutes))
        .map_err(|error| format!("Could not prepare local candle query: {error}"))?;
    let rows = statement
        .query_map(
            params![
                parquet_glob.to_string_lossy().replace('\\', "/"),
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
    if candles.is_empty() {
        return Err(format!(
            "No data found for the requested date range. Check if 2020_normalized.parquet exists in data/ticks/{}/2020/ (resolved symbol directory: {}). Requested Unix seconds: {start_ts}..{end_ts}.",
            symbol.to_ascii_uppercase(),
            symbol_dir.display(),
        ));
    }
    candles.reverse();
    Ok(candles)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use duckdb::Connection;

    use super::fetch_local_candles;
    use crate::market::ipc::CandleDto;

    struct TestDirectory(std::path::PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vora-local-candle-test-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("temporary test directory should be created");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn reads_and_aggregates_parquet_ticks() {
        let temporary = TestDirectory::new();
        let symbol_dir = temporary.0.join("ticks").join("XAUUSD").join("2020");
        std::fs::create_dir_all(&symbol_dir).expect("partition directory should be created");
        let parquet_path = symbol_dir.join("2020_normalized.parquet");
        let sql_path = parquet_path
            .to_string_lossy()
            .replace('\\', "/")
            .replace('\'', "''");

        let writer = Connection::open_in_memory().expect("writer connection should open");
        writer
            .execute_batch(
                "CREATE TABLE ticks (ts BIGINT, price DOUBLE, volume DOUBLE);
                 INSERT INTO ticks VALUES
                    (1577836800, 10, 1),
                    (1577836810, 13, 2),
                    (1577836820, 8, 3),
                    (1577836860, 9, 2),
                    (1577836870, 9, 4);",
            )
            .expect("test ticks should be inserted");
        writer
            .execute_batch(&format!("COPY ticks TO '{sql_path}' (FORMAT PARQUET);"))
            .expect("test Parquet file should be written");

        let candles = fetch_local_candles(
            temporary.0.join("ticks"),
            "XAUUSD".to_string(),
            1,
            1577836800,
            1577836920,
            10,
        )
        .expect("test Parquet file should aggregate");

        assert_eq!(
            candles,
            vec![
                CandleDto {
                    ts: 1577836800,
                    open: 10.0,
                    high: 13.0,
                    low: 8.0,
                    close: 8.0,
                    volume: 6.0,
                    tick_count: 3,
                    first_extreme: "high_first".to_string(),
                },
                CandleDto {
                    ts: 1577836860,
                    open: 9.0,
                    high: 9.0,
                    low: 9.0,
                    close: 9.0,
                    volume: 6.0,
                    tick_count: 2,
                    first_extreme: "same".to_string(),
                },
            ]
        );
    }

    #[test]
    fn empty_date_range_returns_a_path_hint() {
        let temporary = TestDirectory::new();
        let symbol_dir = temporary.0.join("ticks").join("XAUUSD").join("2020");
        std::fs::create_dir_all(&symbol_dir).expect("partition directory should be created");
        let parquet_path = symbol_dir.join("2020_normalized.parquet");
        let sql_path = parquet_path
            .to_string_lossy()
            .replace('\\', "/")
            .replace('\'', "''");
        let writer = Connection::open_in_memory().expect("writer connection should open");
        writer
            .execute_batch(
                "CREATE TABLE ticks (ts BIGINT, price DOUBLE, volume DOUBLE);
                 INSERT INTO ticks VALUES (1577836800, 10, 1);",
            )
            .expect("test tick should be inserted");
        writer
            .execute_batch(&format!("COPY ticks TO '{sql_path}' (FORMAT PARQUET);"))
            .expect("test Parquet file should be written");

        let error = fetch_local_candles(
            temporary.0.join("ticks"),
            "XAUUSD".to_string(),
            1,
            0,
            60,
            10,
        )
        .expect_err("an out-of-range request should fail clearly");

        assert!(error.contains("No data found for the requested date range"));
        assert!(error.contains("data/ticks/XAUUSD/2020/"));
        assert!(error.contains("2020_normalized.parquet"));
    }

    #[test]
    fn validates_request_bounds_before_querying() {
        let temporary = TestDirectory::new();
        let data_dir = temporary.0.join("ticks");
        std::fs::create_dir_all(data_dir.join("XAUUSD"))
            .expect("symbol directory should be created");

        assert!(
            fetch_local_candles(data_dir.clone(), "../XAUUSD".to_string(), 1, 0, 60, 10,).is_err()
        );
        assert!(fetch_local_candles(data_dir, "XAUUSD".to_string(), 0, 0, 60, 10).is_err());
    }
}
