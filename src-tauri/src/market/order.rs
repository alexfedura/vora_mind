// ═══════════════════════════════════════════════════════════════
// order.rs — temporal order preservation for OHLC extremes
// ═══════════════════════════════════════════════════════════════

use crate::market::model::ExtremeKind;

pub fn from_ticks(ticks: &[(i64, f64)]) -> ExtremeKind {
    if ticks.is_empty() {
        return ExtremeKind::Same;
    }
    let mut final_high = f64::NEG_INFINITY;
    let mut final_low = f64::INFINITY;
    for (_, p) in ticks {
        if *p > final_high {
            final_high = *p;
        }
        if *p < final_low {
            final_low = *p;
        }
    }
    let tol = 1e-6;
    if (final_high - final_low).abs() < tol {
        return ExtremeKind::Same;
    }
    let mut t_high: Option<usize> = None;
    let mut t_low: Option<usize> = None;
    for (i, (_, p)) in ticks.iter().enumerate() {
        if t_high.is_none() && (p - final_high).abs() < tol {
            t_high = Some(i);
        }
        if t_low.is_none() && (p - final_low).abs() < tol {
            t_low = Some(i);
        }
        if t_high.is_some() && t_low.is_some() {
            break;
        }
    }
    match (t_high, t_low) {
        (Some(h), Some(l)) if h < l => ExtremeKind::HighFirst,
        (Some(_), Some(_)) => ExtremeKind::LowFirst,
        _ => ExtremeKind::Same,
    }
}

pub fn from_source_candles(src: &[crate::market::model::Candle]) -> ExtremeKind {
    if src.is_empty() {
        return ExtremeKind::Same;
    }
    let mut final_high = f64::NEG_INFINITY;
    let mut final_low = f64::INFINITY;
    for c in src {
        if c.high > final_high {
            final_high = c.high;
        }
        if c.low < final_low {
            final_low = c.low;
        }
    }
    let tol = 1e-6;
    if (final_high - final_low).abs() < tol {
        return ExtremeKind::Same;
    }
    let h_idx = src.iter().position(|c| (c.high - final_high).abs() < tol);
    let l_idx = src.iter().position(|c| (c.low - final_low).abs() < tol);
    match (h_idx, l_idx) {
        (Some(h), Some(l)) if h < l => ExtremeKind::HighFirst,
        (Some(h), Some(l)) if l < h => ExtremeKind::LowFirst,
        (Some(h), Some(_)) => src[h].first_extreme,
        _ => ExtremeKind::Same,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::model::{Candle, ExtremeKind};

    fn tick(ts: i64, p: f64) -> (i64, f64) {
        (ts, p)
    }
    fn mk(ts: i64, o: f64, h: f64, l: f64, c: f64, fe: ExtremeKind) -> Candle {
        Candle {
            ts,
            symbol: "X".into(),
            timeframe: "M0".into(),
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 0.0,
            tick_count: 0,
            first_extreme: fe,
        }
    }

    #[test]
    fn ticks_empty() {
        assert_eq!(from_ticks(&[]), ExtremeKind::Same);
    }
    #[test]
    fn ticks_all_same() {
        let t = vec![tick(1, 100.0), tick(2, 100.0), tick(3, 100.0)];
        assert_eq!(from_ticks(&t), ExtremeKind::Same);
    }
    #[test]
    fn ticks_high_first() {
        let t = vec![
            tick(1, 100.0),
            tick(2, 102.0),
            tick(3, 105.0),
            tick(4, 99.0),
        ];
        assert_eq!(from_ticks(&t), ExtremeKind::HighFirst);
    }
    #[test]
    fn ticks_low_first() {
        let t = vec![
            tick(1, 100.0),
            tick(2, 102.0),
            tick(3, 99.0),
            tick(4, 105.0),
        ];
        assert_eq!(from_ticks(&t), ExtremeKind::LowFirst);
    }
    #[test]
    fn candles_high_first_across_two() {
        let c = vec![
            mk(1, 99.0, 100.0, 98.0, 99.5, ExtremeKind::Same),
            mk(2, 99.5, 105.0, 99.0, 104.0, ExtremeKind::Same),
        ];
        assert_eq!(from_source_candles(&c), ExtremeKind::LowFirst);
    }
    #[test]
    fn candles_low_first_across_two() {
        let c = vec![
            mk(1, 100.0, 105.0, 99.0, 100.0, ExtremeKind::Same),
            mk(2, 100.0, 100.0, 98.0, 99.0, ExtremeKind::Same),
        ];
        assert_eq!(from_source_candles(&c), ExtremeKind::HighFirst);
    }
    #[test]
    fn candles_same_candle_inherits() {
        let c = vec![mk(1, 100.0, 105.0, 95.0, 102.0, ExtremeKind::LowFirst)];
        assert_eq!(from_source_candles(&c), ExtremeKind::LowFirst);
    }
    #[test]
    fn candles_same_candle_inherits_high_first() {
        let c = vec![mk(1, 100.0, 105.0, 95.0, 102.0, ExtremeKind::HighFirst)];
        assert_eq!(from_source_candles(&c), ExtremeKind::HighFirst);
    }
    #[test]
    fn candles_high_in_later_candle_same_low_in_first() {
        let c = vec![
            mk(1, 95.0, 100.0, 90.0, 98.0, ExtremeKind::LowFirst),
            mk(2, 98.0, 105.0, 95.0, 100.0, ExtremeKind::HighFirst),
        ];
        assert_eq!(from_source_candles(&c), ExtremeKind::LowFirst);
    }
    #[test]
    fn candles_empty() {
        assert_eq!(from_source_candles(&[]), ExtremeKind::Same);
    }
}
