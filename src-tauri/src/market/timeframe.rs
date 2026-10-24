//! v6.3.0a: Timeframe ladder with ×5 ratio.
//!
//! This module is PURE — no integration with detection yet.
//! Steps are INDEXED 0..=10. Each step's period in seconds
//! is `TF_SECONDS[step]`.
//!
//!   step : 0     1     2     3      4      5
//!   secs : 1     5     25    125    625    3125
//!
//!   step : 6        7         8          9           10
//!   secs : 15625    78125     390625     1953125     9765625
//!
//! Replaces the legacy ×10 M-scale (M0..M9).

/// 11 timeframe steps (seconds). Ratio ×5 per step.
pub const TF_SECONDS: [u64; 11] = [
    1,         // step 0  — 1s
    5,         // step 1
    25,        // step 2
    125,       // step 3
    625,       // step 4
    3_125,     // step 5
    15_625,    // step 6
    78_125,    // step 7
    390_625,   // step 8  — ~4.52 days
    1_953_125, // step 9  — ~22.6 days
    9_765_625, // step 10 — ~113 days
];

pub const MAX_STEP: u8 = 10;
pub const N_STEPS: usize = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartInterval {
    MScale(u8),
    FixedSeconds(i64),
    CalendarMonths(u32),
}

pub fn parse_chart_interval(key: &str) -> Option<ChartInterval> {
    let trimmed = key.trim();
    if let Some(step) = trimmed
        .strip_prefix('M')
        .and_then(|value| value.parse::<u8>().ok())
    {
        return (step <= MAX_STEP).then_some(ChartInterval::MScale(step));
    }
    if let Some(timeframe) = super::model::Timeframe::from_str(trimmed) {
        return Some(ChartInterval::FixedSeconds(timeframe.seconds()));
    }
    if trimmed.is_empty() || !trimmed.bytes().all(|byte| byte.is_ascii()) {
        return None;
    }
    let (amount_text, multiplier) = if let Some(value) = trimmed.strip_suffix("mo") {
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let amount = value.parse::<u32>().ok()?;
        if !(1..=12).contains(&amount) {
            return None;
        }
        return Some(ChartInterval::CalendarMonths(amount));
    } else if let Some(value) = trimmed.strip_suffix('s') {
        (value, 1_i64)
    } else if let Some(value) = trimmed.strip_suffix('m') {
        (value, 60_i64)
    } else if let Some(value) = trimmed.strip_suffix('h') {
        (value, 3_600_i64)
    } else if let Some(value) = trimmed.strip_suffix('d') {
        (value, 86_400_i64)
    } else if let Some(value) = trimmed.strip_suffix('w') {
        (value, 604_800_i64)
    } else {
        return None;
    };
    if amount_text.is_empty() || !amount_text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let amount = amount_text.parse::<i64>().ok()?;
    if amount < 1 {
        return None;
    }
    let seconds = amount.checked_mul(multiplier)?;
    (seconds <= 366 * 86_400).then_some(ChartInterval::FixedSeconds(seconds))
}

pub fn calendar_month_bucket(timestamp: i64, months: u32) -> i64 {
    if months == 0 {
        return timestamp;
    }
    let Some(date_time) = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0) else {
        return timestamp;
    };
    use chrono::Datelike;
    let month_index = i64::from(date_time.year()) * 12 + i64::from(date_time.month0());
    let group = i64::from(months);
    let bucket_index = month_index.div_euclid(group) * group;
    let year = bucket_index.div_euclid(12) as i32;
    let month = (bucket_index.rem_euclid(12) + 1) as u32;
    chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|date_time| date_time.and_utc().timestamp())
        .unwrap_or(timestamp)
}

/// Seconds for a step. Returns 1 for out-of-range (safe default).
#[inline]
pub fn tf_seconds(step: u8) -> u64 {
    match TF_SECONDS.get(step as usize) {
        Some(&s) => s,
        None => 1,
    }
}

/// Floor-divide a Unix-seconds timestamp into the step's bucket
/// index. Uses `div_euclid` for correct behavior with negative ts.
#[inline]
pub fn tf_bucket(ts: i64, step: u8) -> i64 {
    let secs = tf_seconds(step) as i64;
    ts.div_euclid(secs)
}

/// Canonical string key for a step, e.g. `"s3"`.
/// Used by future chunks for candle cache keys.
#[inline]
pub fn step_key(step: u8) -> String {
    format!("s{}", step)
}

/// Parse a step key (`"s3"` or legacy `"M3"` / `"m3"`) back to a
/// step index. Returns None for unparseable input or out-of-range.
#[inline]
pub fn step_from_key(key: &str) -> Option<u8> {
    let k = key.trim();
    let rest: &str = if k.starts_with('s') || k.starts_with('S') {
        &k[1..]
    } else if k.starts_with('M') || k.starts_with('m') {
        &k[1..]
    } else {
        k
    };
    rest.parse::<u8>().ok().filter(|&s| s <= MAX_STEP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratios_are_5x() {
        for i in 1..N_STEPS {
            assert_eq!(
                TF_SECONDS[i],
                TF_SECONDS[i - 1] * 5,
                "step {i} must be ×5 of step {}",
                i - 1
            );
        }
    }

    #[test]
    fn endpoints() {
        assert_eq!(tf_seconds(0), 1);
        assert_eq!(tf_seconds(10), 9_765_625);
    }

    #[test]
    fn out_of_range_is_safe() {
        assert_eq!(tf_seconds(11), 1);
        assert_eq!(tf_seconds(200), 1);
    }

    #[test]
    fn bucket_floors_correctly() {
        assert_eq!(tf_bucket(12_345, 0), 12_345);
        assert_eq!(tf_bucket(12_345, 1), 2_469);
        assert_eq!(tf_bucket(12_345, 2), 493);
    }

    #[test]
    fn step_key_roundtrip() {
        for s in 0..=MAX_STEP {
            let k = step_key(s);
            assert_eq!(step_from_key(&k), Some(s));
        }
    }

    #[test]
    fn legacy_m_key_accepted() {
        assert_eq!(step_from_key("M3"), Some(3));
        assert_eq!(step_from_key("m7"), Some(7));
    }

    #[test]
    fn step_from_key_rejects_bad() {
        assert_eq!(step_from_key(""), None);
        assert_eq!(step_from_key("sX"), None);
        assert_eq!(step_from_key("s99"), None); // > MAX_STEP
        assert_eq!(step_from_key("abc"), None);
    }

    #[test]
    fn parses_chart_intervals_and_rejects_unsupported_modes() {
        assert_eq!(parse_chart_interval("M3"), Some(ChartInterval::MScale(3)));
        assert_eq!(
            parse_chart_interval("4h"),
            Some(ChartInterval::FixedSeconds(14_400))
        );
        assert_eq!(
            parse_chart_interval("45m"),
            Some(ChartInterval::FixedSeconds(2_700))
        );
        assert_eq!(
            parse_chart_interval("3mo"),
            Some(ChartInterval::CalendarMonths(3))
        );
        assert_eq!(parse_chart_interval("0s"), None);
        assert_eq!(parse_chart_interval("1000T"), None);
        assert_eq!(parse_chart_interval("100R"), None);
        assert_eq!(parse_chart_interval("13mo"), None);
    }

    #[test]
    fn calendar_months_align_to_utc_calendar_boundaries() {
        let feb_29 = chrono::NaiveDate::from_ymd_opt(2024, 2, 29)
            .unwrap()
            .and_hms_opt(13, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        let jan_1 = chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        let jul_1 = chrono::NaiveDate::from_ymd_opt(2024, 7, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        assert_eq!(
            calendar_month_bucket(feb_29, 1),
            chrono::NaiveDate::from_ymd_opt(2024, 2, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp()
        );
        assert_eq!(calendar_month_bucket(feb_29, 3), jan_1);
        assert_eq!(
            calendar_month_bucket(jul_1, 6),
            chrono::NaiveDate::from_ymd_opt(2024, 7, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp()
        );
    }
}
