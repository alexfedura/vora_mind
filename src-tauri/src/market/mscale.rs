//! M-Scale — SI-based time coordinate system (v6.3.0c: ×5 TF_SECONDS ladder).
//!
//! Formula: period(M) = TF_SECONDS[M] seconds
//! Origin:  M0 = 1 second
//! Range:   M0 (1s) to M10 (~113 days)
//!
//! This REPLACES the traditional timeframe concept (1m, 5m, 1h, 1d).
//! See docs/scale-standard.md for full specification.

use super::timeframe;

/// Convert M level to period in seconds.
/// period(M) = TF_SECONDS[M] seconds (v6.3.0c ×5 ladder).
/// Returns None if M is out of practical range or sub-second.
pub fn period_seconds(m: i32) -> Option<i64> {
    if m < 0 {
        return None; // sub-second not supported in v1 (MKT-6a)
    }
    if m > 10 {
        return None; // beyond practical range
    }
    Some(timeframe::tf_seconds(m as u8) as i64)
}

/// Human-readable label for an M level.
pub fn m_label(m: i32) -> String {
    match m {
        -3 => "1 ms".to_string(),
        -2 => "10 ms".to_string(),
        -1 => "100 ms".to_string(),
        0 => "1 sec".to_string(),
        1 => "10 sec".to_string(),
        2 => "1.67 min".to_string(),
        3 => "16.67 min".to_string(),
        4 => "2.78 hr".to_string(),
        5 => "27.78 hr".to_string(),
        6 => "11.57 days".to_string(),
        7 => "115.7 days".to_string(),
        8 => "3.17 years".to_string(),
        9 => "31.7 years".to_string(),
        n => format!("M{}", n),
    }
}

/// Format a timeframe key from an M level (for DB storage).
/// Returns "M{n}" e.g. "M3", "M-1".
pub fn m_key(m: i32) -> String {
    format!("M{}", m)
}

/// Parse an M key back to i32.
/// Returns None if the string is not "M<n>".
pub fn m_from_key(key: &str) -> Option<i32> {
    if let Some(rest) = key.strip_prefix('M') {
        rest.parse::<i32>().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_mapping() {
        assert_eq!(period_seconds(0), Some(1));
        assert_eq!(period_seconds(1), Some(5));
        assert_eq!(period_seconds(2), Some(25));
        assert_eq!(period_seconds(3), Some(125));
        assert_eq!(period_seconds(10), Some(9_765_625));
        assert_eq!(period_seconds(-1), None); // sub-second
        assert_eq!(period_seconds(11), None); // out of range
    }

    #[test]
    fn labels() {
        assert_eq!(m_label(0), "1 sec");
        assert_eq!(m_label(3), "16.67 min");
        assert_eq!(m_label(6), "11.57 days");
    }

    #[test]
    fn key_roundtrip() {
        for m in [-3, -1, 0, 3, 6, 9] {
            let key = m_key(m);
            assert_eq!(m_from_key(&key), Some(m));
        }
        assert_eq!(m_from_key("bogus"), None);
        assert_eq!(m_from_key("1h"), None);
    }
}
