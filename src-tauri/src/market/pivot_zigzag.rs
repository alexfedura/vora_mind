//! v6.0.0-POC: Zigzag (swing-based) pivot detection.
//!
//! Replaces the window-based `detect_pivots(candles, k)` with a
//! threshold-based swing detector. The threshold is a PERCENTAGE
//! of price, not a count of candles — so the same percentage
//! behaves identically on 1s candles and on 30d candles.

use crate::market::boxes::{Pivot, PivotKind};
use crate::market::model::Candle;

/// Detect pivots using a pure zigzag algorithm.
///
/// Algorithm:
///   1. Seed direction from the first candle's body.
///   2. Track the current leg's extreme price (highest high on an
///      up-leg, lowest low on a down-leg).
///   3. When price retraces from that extreme by at least
///      `threshold_pct` percent, EMIT the extreme as a pivot
///      and flip direction.
///   4. Continue to the end. The final (unconfirmed) leg is NOT
///      emitted.
///
/// `threshold_pct` is in PERCENT units (0.5 = 0.5%).
/// Returns an alternating sequence High, Low, High, Low, ...
pub fn detect_pivots_zigzag(candles: &[Candle], threshold_pct: f64) -> Vec<Pivot> {
    if candles.len() < 2 || threshold_pct <= 0.0 {
        return Vec::new();
    }
    let thresh = threshold_pct / 100.0;

    let mut pivots: Vec<Pivot> = Vec::new();

    // Seed direction from first candle's body.
    // (close >= open → up-leg; else down-leg).
    let mut dir: i8 = if candles[0].close >= candles[0].open {
        1
    } else {
        -1
    };

    let mut ext_idx: usize = 0;
    let mut ext_price: f64 = if dir == 1 {
        candles[0].high
    } else {
        candles[0].low
    };

    for i in 1..candles.len() {
        let c = &candles[i];
        if dir == 1 {
            // Up-leg: extend on a new high.
            if c.high > ext_price {
                ext_price = c.high;
                ext_idx = i;
                continue;
            }
            // Retrace check on the low side.
            if ext_price > 0.0 {
                let retrace = (ext_price - c.low) / ext_price;
                if retrace >= thresh {
                    pivots.push(Pivot {
                        idx: ext_idx,
                        price: ext_price,
                        kind: PivotKind::High,
                    });
                    dir = -1;
                    ext_price = c.low;
                    ext_idx = i;
                }
            }
        } else {
            // Down-leg: extend on a new low.
            if c.low < ext_price {
                ext_price = c.low;
                ext_idx = i;
                continue;
            }
            // Retrace check on the high side.
            if ext_price > 0.0 {
                let retrace = (c.high - ext_price) / ext_price;
                if retrace >= thresh {
                    pivots.push(Pivot {
                        idx: ext_idx,
                        price: ext_price,
                        kind: PivotKind::Low,
                    });
                    dir = 1;
                    ext_price = c.high;
                    ext_idx = i;
                }
            }
        }
    }

    pivots
}

// ─────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::model::ExtremeKind;

    fn mk(high: f64, low: f64) -> Candle {
        let mid = (high + low) / 2.0;
        Candle {
            symbol: "TEST".to_string(),
            timeframe: "1s".to_string(),
            ts: 0,
            open: mid,
            high,
            low,
            close: mid,
            volume: 0.0,
            tick_count: 0,
            first_extreme: ExtremeKind::Same,
        }
    }

    #[test]
    fn zigzag_empty_input() {
        assert!(detect_pivots_zigzag(&[], 0.5).is_empty());
    }

    #[test]
    fn zigzag_single_candle() {
        let c = vec![mk(100.0, 99.0)];
        assert!(detect_pivots_zigzag(&c, 0.5).is_empty());
    }

    #[test]
    fn zigzag_zero_threshold_returns_empty() {
        let c = vec![mk(100.0, 99.0), mk(101.0, 99.5)];
        assert!(detect_pivots_zigzag(&c, 0.0).is_empty());
    }

    #[test]
    fn zigzag_up_down_up() {
        let candles = vec![
            mk(100.0, 99.0),  // seed up (close==open → up)
            mk(105.0, 100.0), // ext high 105
            mk(105.0, 103.0), // still up
            mk(104.0, 100.0), // drop to 100 = ~4.7% retrace → High@105
            mk(103.0, 98.0),  // down ext 98
            mk(102.0, 98.5),  // still down
            mk(104.0, 100.0), // rally to 104 = ~6.1% retrace → Low@98
            mk(106.0, 103.0), // ext high 106
            mk(107.0, 105.0), // ext high 107
        ];
        let p = detect_pivots_zigzag(&candles, 3.0);
        assert!(p.len() >= 2, "expected ≥2 pivots, got {:?}", p);
        assert_eq!(p[0].kind, PivotKind::High);
        assert!((p[0].price - 105.0).abs() < 1e-9);
        assert_eq!(p[1].kind, PivotKind::Low);
        assert!((p[1].price - 98.0).abs() < 1e-9);
    }

    #[test]
    fn zigzag_high_threshold_no_pivots() {
        let candles = vec![
            mk(100.0, 99.9),
            mk(100.1, 99.9),
            mk(100.05, 99.95),
            mk(100.1, 99.9),
            mk(100.0, 99.9),
        ];
        let p = detect_pivots_zigzag(&candles, 5.0);
        assert!(p.is_empty(), "expected no pivots, got {:?}", p);
    }

    #[test]
    fn zigzag_alternates_kinds() {
        let mut candles = Vec::new();
        let mut price = 100.0;
        for i in 0..50 {
            let d = if (i / 5) % 2 == 0 { 1.0 } else { -1.0 };
            price += d * 1.0;
            candles.push(mk(price + 0.3, price - 0.3));
        }
        let p = detect_pivots_zigzag(&candles, 1.0);
        for w in p.windows(2) {
            assert_ne!(
                w[0].kind, w[1].kind,
                "consecutive pivots must alternate: {:?}",
                p
            );
        }
    }
}
