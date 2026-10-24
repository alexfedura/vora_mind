use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use dukascopy_rust::models::Candle as DukascopyCandle;
use reqwest::{Client, Url};
use serde_json::Value;
use tokio::sync::Mutex;

use super::ipc::CandleDto;
use super::timeframe::{self, ChartInterval};

const MAX_FETCH_LIMIT: usize = 5_000;
const MAX_PROVIDER_LIMIT: usize = 50_000;
const WINDOW_FACTORS: [usize; 3] = [2, 4, 8];
const DUKASCOPY_CHART_URL: &str = "https://freeserv.dukascopy.com/2.0/index.php";

static JSONP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct DukaInterval {
    name: &'static str,
    source_seconds: i64,
    target_seconds: i64,
    calendar_months: Option<u32>,
}

static CANDLE_CACHE: OnceLock<Mutex<HashMap<String, Vec<CandleDto>>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<String, Vec<CandleDto>>> {
    CANDLE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_key(symbol: &str, timeframe: &str) -> String {
    format!(
        "{}_{}",
        symbol.trim().to_ascii_uppercase(),
        timeframe.trim()
    )
}

fn duka_instrument(symbol: &str) -> Result<String, String> {
    let normalized = symbol
        .trim()
        .to_ascii_uppercase()
        .replace(['/', '-', '_'], "");
    if normalized.is_empty() {
        return Err("DUKASCOPY_INVALID_SYMBOL: symbol cannot be empty".to_string());
    }
    if normalized.len() == 6 && normalized.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Ok(format!("{}/{}", &normalized[..3], &normalized[3..]));
    }
    if normalized == "XAUUSD" {
        return Ok("XAU/USD".to_string());
    }
    Err(format!("DUKASCOPY_UNSUPPORTED_SYMBOL: {symbol}"))
}

fn duka_interval(timeframe: &str) -> Result<DukaInterval, String> {
    let parsed = timeframe::parse_chart_interval(timeframe)
        .ok_or_else(|| format!("DUKASCOPY_UNSUPPORTED_TIMEFRAME: {timeframe}"))?;
    let (target_seconds, calendar_months) = match parsed {
        ChartInterval::MScale(step) => (timeframe::tf_seconds(step) as i64, None),
        ChartInterval::FixedSeconds(seconds) => (seconds, None),
        ChartInterval::CalendarMonths(months) => (i64::from(months) * 30 * 86_400, Some(months)),
    };
    if target_seconds < 60 {
        return Err(format!(
            "DUKASCOPY_UNSUPPORTED_TIMEFRAME: {timeframe} requires tick aggregation"
        ));
    }
    let sources = [
        ("1MIN", 60),
        ("5MIN", 5 * 60),
        ("10MIN", 10 * 60),
        ("15MIN", 15 * 60),
        ("30MIN", 30 * 60),
        ("1HOUR", 60 * 60),
        ("4HOUR", 4 * 60 * 60),
        ("1DAY", 24 * 60 * 60),
        ("1WEEK", 7 * 24 * 60 * 60),
        ("1MONTH", 30 * 24 * 60 * 60),
    ];
    let (name, source_seconds) = sources
        .iter()
        .rev()
        .find(|(_, seconds)| *seconds <= target_seconds)
        .copied()
        .unwrap_or(sources[0]);
    Ok(DukaInterval {
        name,
        source_seconds,
        target_seconds,
        calendar_months,
    })
}

fn rate_limit_error(message: &str) -> String {
    let lowercase = message.to_ascii_lowercase();
    eprintln!("[dukascopy] historical request failed: {message}");
    if message.contains("429")
        || lowercase.contains("rate limit")
        || lowercase.contains("too many requests")
    {
        "DUKASCOPY_RATE_LIMIT: historical data provider temporarily rejected the request"
            .to_string()
    } else if lowercase.contains("connection") || lowercase.contains("error sending request") {
        "DUKASCOPY_CONNECTION_FAILED: historical data provider could not be reached".to_string()
    } else {
        "DUKASCOPY_FETCH_FAILED: historical data provider request failed".to_string()
    }
}

fn strip_jsonp(body: &str) -> &str {
    match (body.find('('), body.rfind(')')) {
        (Some(start), Some(end)) if start < end => &body[start + 1..end],
        _ => body,
    }
}

async fn fetch_duka_rows(
    client: &Client,
    request_url: &Url,
    instrument: &str,
    interval: &str,
    last_update: i64,
    limit: usize,
) -> anyhow::Result<Vec<Vec<Value>>> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = JSONP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let callback = format!("_callbacks____{timestamp}{sequence}");
    let mut url = request_url.clone();
    url.query_pairs_mut()
        .append_pair("path", "chart/json3")
        .append_pair("splits", "true")
        .append_pair("stocks", "true")
        .append_pair("time_direction", "N")
        .append_pair("jsonp", &callback)
        .append_pair("last_update", &last_update.to_string())
        .append_pair("offer_side", "B")
        .append_pair("instrument", instrument)
        .append_pair("interval", interval)
        .append_pair("limit", &limit.to_string());
    let body = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    anyhow::ensure!(!body.is_empty(), "empty Dukascopy chart response");

    serde_json::from_str(strip_jsonp(&body)).context("invalid Dukascopy JSONP response")
}

fn map_candle(candle: DukascopyCandle) -> CandleDto {
    CandleDto {
        ts: candle.timestamp.div_euclid(1_000),
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
        volume: candle.volume,
        tick_count: 0,
        first_extreme: "same".to_string(),
    }
}

fn normalize_candles(candles: &mut Vec<CandleDto>) {
    candles.sort_by_key(|candle| candle.ts);
    candles.dedup_by_key(|candle| candle.ts);
}

fn aggregate_candles(candles: Vec<CandleDto>, interval: &DukaInterval) -> Vec<CandleDto> {
    let mut aggregated: Vec<CandleDto> = Vec::new();
    for candle in candles {
        let timestamp = interval
            .calendar_months
            .map(|months| timeframe::calendar_month_bucket(candle.ts, months))
            .unwrap_or_else(|| {
                candle.ts.div_euclid(interval.target_seconds) * interval.target_seconds
            });
        if let Some(current) = aggregated
            .last_mut()
            .filter(|current| current.ts == timestamp)
        {
            current.high = current.high.max(candle.high);
            current.low = current.low.min(candle.low);
            current.close = candle.close;
            current.volume += candle.volume;
            current.tick_count += candle.tick_count;
        } else {
            aggregated.push(CandleDto {
                ts: timestamp,
                open: candle.open,
                high: candle.high,
                low: candle.low,
                close: candle.close,
                volume: candle.volume,
                tick_count: candle.tick_count,
                first_extreme: candle.first_extreme,
            });
        }
    }
    aggregated
}

pub async fn cached_candles(
    symbol: &str,
    timeframe: &str,
    end_timestamp: i64,
    limit: usize,
) -> Option<Vec<CandleDto>> {
    let cache = cache().lock().await;
    let candles = cache.get(&cache_key(symbol, timeframe))?;
    let older: Vec<CandleDto> = candles
        .iter()
        .filter(|candle| candle.ts < end_timestamp)
        .cloned()
        .collect();
    if older.len() < limit {
        return None;
    }
    Some(
        older
            .into_iter()
            .rev()
            .take(limit)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect(),
    )
}

pub async fn fetch_duka_candles(
    symbol: &str,
    timeframe: &str,
    end_timestamp: i64,
    limit: usize,
    policy: &super::http_client::PolicySettings,
) -> Result<Vec<CandleDto>, String> {
    if limit == 0 || limit > MAX_FETCH_LIMIT {
        return Err(format!(
            "DUKASCOPY_INVALID_LIMIT: expected 1..={MAX_FETCH_LIMIT}"
        ));
    }
    let instrument = duka_instrument(symbol)?;
    let interval = duka_interval(timeframe)?;
    let client = super::http_client::build_http_client(policy)
        .map_err(|error| format!("DUKASCOPY_CLIENT_CONFIG_FAILED: {error:#}"))?;
    let request_url = Url::parse(DUKASCOPY_CHART_URL)
        .map_err(|error| format!("DUKASCOPY_URL_CONFIG_FAILED: {error}"))?;
    let end_ms = end_timestamp.saturating_mul(1_000);
    let mut historical = Vec::new();

    for factor in WINDOW_FACTORS {
        let source_ratio = ((interval.target_seconds + interval.source_seconds - 1)
            / interval.source_seconds) as usize;
        let query_limit = limit
            .saturating_mul(source_ratio.max(1))
            .saturating_mul(factor)
            .min(MAX_PROVIDER_LIMIT);
        let window_seconds = interval.source_seconds.saturating_mul(query_limit as i64);
        let start_ms = end_ms.saturating_sub(window_seconds.saturating_mul(1_000));
        let rows = fetch_duka_rows(
            &client,
            &request_url,
            &instrument,
            interval.name,
            start_ms,
            query_limit,
        )
        .await
        .map_err(|error| rate_limit_error(&error.to_string()))?;

        let mut source_candles: Vec<CandleDto> = rows
            .into_iter()
            .map(DukascopyCandle::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("DUKASCOPY_INVALID_CANDLE: {error}"))?
            .into_iter()
            .map(map_candle)
            .filter(|candle| candle.ts < end_timestamp)
            .collect();
        normalize_candles(&mut source_candles);
        historical = aggregate_candles(source_candles, &interval);
        if historical.len() >= limit {
            break;
        }
    }

    Ok(historical)
}

pub async fn cache_and_get_older(
    symbol: &str,
    timeframe: &str,
    end_timestamp: i64,
    limit: usize,
    fetched: Vec<CandleDto>,
) -> Vec<CandleDto> {
    let key = cache_key(symbol, timeframe);
    let mut cache = cache().lock().await;
    let candles = cache.entry(key).or_default();
    candles.extend(fetched);
    normalize_candles(candles);
    candles
        .iter()
        .filter(|candle| candle.ts < end_timestamp)
        .rev()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}
