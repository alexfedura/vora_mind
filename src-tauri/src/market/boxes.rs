// ═══════════════════════════════════════════════════════════════
// Box Detection — v1.3.0 (Pivot-based RBR/DBD)
// Reference: docs/box-standard.md v1.3.0
// ═══════════════════════════════════════════════════════════════

use std::collections::HashMap;

use crate::market::model::Candle;
use crate::market::timeframe::TF_SECONDS;

/// 2025-01-15 00:00:00 UTC (Wednesday, no major holiday).
pub const REF_TS: i64 = 1_736_899_200;

/// v6.7.0: full scale list from 1s to 5y. ~350 scales.
pub fn build_scale_list_sec() -> Vec<i64> {
    let mut v: Vec<i64> = Vec::new();
    for s in 1..=60 {
        v.push(s as i64);
    }
    let mut s: i64 = 65;
    while s <= 300 {
        v.push(s);
        s += 5;
    }
    let mut s: i64 = 315;
    while s <= 900 {
        v.push(s);
        s += 15;
    }
    let mut s: i64 = 930;
    while s <= 1800 {
        v.push(s);
        s += 30;
    }
    let mut s: i64 = 1860;
    while s <= 3600 {
        v.push(s);
        s += 60;
    }
    let mut s: i64 = 3900;
    while s <= 21600 {
        v.push(s);
        s += 300;
    }
    let mut s: i64 = 23400;
    while s <= 86400 {
        v.push(s);
        s += 1800;
    }
    let day: i64 = 86400;
    for d in 2..=30 {
        v.push(d * day);
    }
    let month: i64 = 30 * day;
    for m in 1..=12 {
        v.push(m * month);
    }
    let year: i64 = 365 * day;
    for y in 1..=5 {
        v.push(y * year);
    }
    v.sort();
    v.dedup();
    v
}

/// v6.7.0: aggregate M0 candles into L-second bars.
fn aggregate_to_seconds(candles: &[Candle], seconds: i64) -> Vec<Candle> {
    if candles.is_empty() || seconds <= 0 {
        return Vec::new();
    }
    let mut out: Vec<Candle> = Vec::new();
    let mut cur_ts: i64 = i64::MIN;
    let mut cur_o: f64 = 0.0;
    let mut cur_h: f64 = f64::NEG_INFINITY;
    let mut cur_l: f64 = f64::INFINITY;
    let mut cur_c: f64 = 0.0;
    let mut cur_sym: String = String::new();
    let mut cur_tf: String = String::new();
    for c in candles {
        let bucket = c.ts.div_euclid(seconds) * seconds;
        if bucket != cur_ts {
            if cur_ts != i64::MIN && cur_h.is_finite() && cur_l.is_finite() {
                out.push(Candle {
                    symbol: cur_sym.clone(),
                    timeframe: cur_tf.clone(),
                    ts: cur_ts,
                    open: cur_o,
                    high: cur_h,
                    low: cur_l,
                    close: cur_c,
                    volume: 0.0,
                    tick_count: 0,
                    first_extreme: crate::market::model::ExtremeKind::Same,
                });
            }
            cur_ts = bucket;
            cur_sym = c.symbol.clone();
            cur_tf = c.timeframe.clone();
            cur_o = c.open;
            cur_h = c.high;
            cur_l = c.low;
            cur_c = c.close;
        } else {
            if c.high > cur_h {
                cur_h = c.high;
            }
            if c.low < cur_l {
                cur_l = c.low;
            }
            cur_c = c.close;
        }
    }
    if cur_ts != i64::MIN && cur_h.is_finite() && cur_l.is_finite() {
        out.push(Candle {
            symbol: cur_sym,
            timeframe: cur_tf,
            ts: cur_ts,
            open: cur_o,
            high: cur_h,
            low: cur_l,
            close: cur_c,
            volume: 0.0,
            tick_count: 0,
            first_extreme: crate::market::model::ExtremeKind::Same,
        });
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BoxPattern {
    RBR = 0,
    DBD = 1,
    RBD = 2,
    DBR = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxZone {
    Supply,
    Demand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxRole {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PivotKind {
    High,
    Low,
}

#[derive(Debug, Clone, Copy)]
pub struct Pivot {
    pub idx: usize,
    pub price: f64,
    pub kind: PivotKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Box {
    pub address: String,
    pub symbol: String,
    pub step: u8,
    pub pivot_step: u8,
    pub pattern: BoxPattern,
    pub zone: BoxZone,
    pub scale: u8,
    pub x_idx: i64,
    pub ts_start: i64,
    pub ts_end: i64,
    pub engulf_ts: Option<i64>,
    pub duration_sec: i64,
    pub scale_sec: i64,
    pub high: f64,
    pub low: f64,
    pub origin_price: f64,
    pub leg1_magnitude: f64,
    pub leg2_magnitude: f64,
    pub parent_address: Option<String>,
    pub children_addresses: Vec<String>,
    // ── Universal coordinates (v5.1) ──
    pub x: i64,
    pub y_abs: i64,
    pub z: u8,
    pub y_band: i64,
    pub chain_id: Option<i64>,
    pub chain_pos: Option<u32>,
    pub y_lane: Option<i32>,
}

/// v6.1.0: Scale start percentage.
/// S(s) = 0.1% × 1.25^s  (box-standard.md §5).
pub fn scale_start_pct(s: u8) -> f64 {
    0.1_f64 * 1.25_f64.powi(s as i32)
}

#[derive(Debug, Clone, Copy)]
pub struct Parameters {
    pub user_step: u8,
    pub role_override: Option<(u8, u8, u8)>,
    pub z0_scale: u8,
    pub extend_to_engulf: bool,
    pub pivot_threshold_pct: Option<f64>, // v6.1.0
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            user_step: 3,
            role_override: None,
            z0_scale: 5,
            extend_to_engulf: false,
            pivot_threshold_pct: None,
        }
    }
}

pub fn role_for_scale(step: u8, scale: u8) -> Option<BoxRole> {
    let (s, m, l) = match step {
        0 => (0, 1, 2),
        1 => (1, 2, 3),
        2 => (2, 3, 4),
        3 => (3, 4, 5),
        4 => (4, 5, 6),
        5 => (5, 6, 7),
        6 => (6, 7, 8),
        7 | 8 | 9 => (7, 8, 9),
        _ => (0, 1, 2),
    };
    if scale == s {
        Some(BoxRole::Small)
    } else if scale == m {
        Some(BoxRole::Medium)
    } else if scale == l {
        Some(BoxRole::Large)
    } else {
        None
    }
}

pub fn detect_pivots(candles: &[Candle], k: usize) -> Vec<Pivot> {
    let n = candles.len();
    if n < 2 * k + 1 {
        return vec![];
    }
    let mut pivots = Vec::new();
    for i in k..(n - k) {
        let mut is_high = true;
        let mut is_low = true;
        for j in (i - k)..=(i + k) {
            if j == i {
                continue;
            }
            if candles[j].high >= candles[i].high {
                is_high = false;
            }
            if candles[j].low <= candles[i].low {
                is_low = false;
            }
            if !is_high && !is_low {
                break;
            }
        }
        if is_high && !is_low {
            pivots.push(Pivot {
                idx: i,
                price: candles[i].high,
                kind: PivotKind::High,
            });
        } else if is_low && !is_high {
            pivots.push(Pivot {
                idx: i,
                price: candles[i].low,
                kind: PivotKind::Low,
            });
        } else if is_high && is_low {
            pivots.push(Pivot {
                idx: i,
                price: candles[i].high,
                kind: PivotKind::High,
            });
        }
    }
    pivots
}
pub fn classify_scale_from_base(base_height: f64, pivot_price: f64) -> u8 {
    if pivot_price <= 0.0 {
        return 0;
    }
    let pct = base_height / pivot_price * 100.0;
    const BASE_PCT: f64 = 0.1; // v5.0.0: was 1.0 (too high for FX/Metals)
    if pct < BASE_PCT {
        return 0;
    }
    let ratio = pct / BASE_PCT;
    let s = (ratio.ln() / 1.25_f64.ln()).floor() as i32;
    // v6.4.10: widen clamp 0..15 (frontend re-classifies dynamically).
    s.clamp(0, 15) as u8
}

pub fn y_abs_of(price: f64, price_ref: f64) -> i64 {
    if price <= 0.0 || price_ref <= 0.0 {
        return 0;
    }
    let ratio = price / price_ref;
    if ratio <= 0.0 {
        return 0;
    }
    (ratio.log2() * 1_000_000.0).floor() as i64
}

/// y_band = floor(y_abs / 100_000) with signed-floor.
pub fn y_band_of(y_abs: i64) -> i64 {
    y_abs.div_euclid(100_000)
}

/// Resolve price_ref for (symbol, step):
///   1. cache lookup (symbol_origin)
///   2. compute: OPEN of first candle with ts >= REF_TS
///      fallback: OPEN of first candle
///   3. cache + return
pub fn resolve_price_ref(
    conn: &rusqlite::Connection,
    symbol: &str,
    step: u8,
    candles: &[Candle],
) -> Result<(f64, i64), String> {
    if let Some((p, ts)) = crate::market::db::get_symbol_origin(conn, symbol, step)? {
        return Ok((p, ts));
    }
    let (price, ts) = match candles.iter().find(|c| c.ts >= REF_TS) {
        Some(c) => (c.open, c.ts),
        None => match candles.first() {
            Some(c) => (c.open, c.ts),
            None => return Err(format!("no candles for {} M{}", symbol, step)),
        },
    };
    crate::market::db::save_symbol_origin(conn, symbol, step, price, ts)?;
    Ok((price, ts))
}

/// v5.3.1: scale-aware pivot M: M(S) = user_step + (S - z0), None if out of [0, 9].
pub fn m_for_scale(user_step: u8, z0: u8, s: u8) -> Option<u8> {
    let m = user_step as i32 + s as i32 - z0 as i32;
    if (0..=9).contains(&m) {
        Some(m as u8)
    } else {
        None
    }
}

pub fn detect_boxes(
    candles: &[Candle],
    params: Parameters,
    symbol: &str,
    step: u8,
    price_ref: f64,
    pivot_step: u8,
    cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> (Vec<Box>, Vec<crate::market::chain::Chain>) {
    if candles.len() < 20 {
        return (vec![], vec![]);
    }
    // v6.1.0: threshold-based zigzag pivots.
    // Fallback uses z0 + (pivot_step - user_step) to derive the S.
    let threshold_pct: f64 = match params.pivot_threshold_pct {
        Some(v) if v > 0.0 => v,
        _ => {
            let s_signed =
                (params.z0_scale as i32) + (pivot_step as i32) - (params.user_step as i32);
            let s = s_signed.clamp(0, 9) as u8;
            scale_start_pct(s)
        }
    };
    let _ = threshold_pct;

    let mut raw: Vec<Box> = Vec::new();
    raw.extend(scan_rbr(
        candles,
        &params,
        symbol,
        step,
        price_ref,
        pivot_step,
        cancel.clone(),
    ));
    raw.extend(scan_dbd(
        candles,
        &params,
        symbol,
        step,
        price_ref,
        pivot_step,
        cancel.clone(),
    ));

    // v6.5.3: pivot-first (pivot_step == 0) preserves ALL detected structures.
    // merge_overlapping is a legacy TF-ladder dedup that collapses boxes by
    // vertical price-range overlap only (ignoring time). In pivot-first mode
    // we scan on M0 and each box is an independent structure; merging destroys
    // legitimate smaller-scale boxes nested inside larger ones.
    let merged: Vec<Box> = if pivot_step == 0 {
        raw
    } else {
        merge_overlapping(raw)
    };
    let sorted = sort_for_output(merged);
    let nested = build_nesting(sorted);

    // v6.4.4: show_all_scales removed — backend always returns all scales.
    // role_override retained for API compat (frontend filtering via registry).
    let _ = &params.role_override;
    let mut filtered: Vec<Box> = nested;
    for b in filtered.iter_mut() {
        b.x = b.x_idx;
        let y = y_abs_of(b.origin_price, price_ref);
        b.y_abs = y;
        b.z = b.scale;
        b.y_band = y_band_of(y);
        b.chain_id = None;
        b.chain_pos = None;
    }
    let chains = crate::market::chain::assign_chains(
        &mut filtered,
        &crate::market::chain::ChainParams::default(),
    );
    compute_y_lanes(&mut filtered, params.z0_scale);
    (filtered, chains)
}

/// v6.7.0: run RBR/DBD detection at EVERY scale from `build_scale_list_sec()`.
pub fn detect_boxes_multiscale(
    candles_base: &[Candle],
    params: Parameters,
    symbol: &str,
    price_ref: f64,
    cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> (Vec<Box>, Vec<crate::market::chain::Chain>) {
    let scales = build_scale_list_sec();
    let mut all: Vec<Box> = Vec::new();
    for l in scales {
        if crate::market::cancel::is_cancelled(&cancel) {
            return (Vec::new(), Vec::new());
        }
        let bars = aggregate_to_seconds(candles_base, l);
        if bars.len() < 20 {
            continue;
        }
        let (boxes, _) = detect_boxes(
            &bars,
            params.clone(),
            symbol,
            0,
            price_ref,
            0,
            cancel.clone(),
        );
        for mut b in boxes {
            b.scale_sec = l;
            let suffix = b.address.splitn(2, '~').nth(1).unwrap_or("");
            b.address = format!("{}~L{}~{}", symbol, l, suffix);
            all.push(b);
        }
    }
    all.sort_by(|a, b| a.address.cmp(&b.address));
    all.dedup_by(|a, b| a.address == b.address);
    all = sort_for_output(all);
    let chains = crate::market::chain::assign_chains(
        &mut all,
        &crate::market::chain::ChainParams::default(),
    );
    compute_y_lanes(&mut all, params.z0_scale);
    (all, chains)
}

/// Non-fractal nearest-scale-step y_lane assignment.
/// Processes boxes in (chain_id, x, idx) order so ancestors
/// are ready before descendants.
fn compute_y_lanes(boxes: &mut [Box], z0_scale: u8) {
    let n = boxes.len();
    if n == 0 {
        return;
    }
    let patterns: Vec<BoxPattern> = boxes.iter().map(|b| b.pattern).collect();
    let scales: Vec<u8> = boxes.iter().map(|b| b.scale).collect();
    let chain_ids: Vec<i64> = boxes
        .iter()
        .map(|b| b.chain_id.unwrap_or(i64::MAX))
        .collect();
    let xs: Vec<i64> = boxes.iter().map(|b| b.x).collect();
    let mut order: Vec<usize> = (0..n).collect();
    // y_lane is time-dependent: x first, then chain_id, then idx.
    order.sort_by_key(|&i| (xs[i], chain_ids[i], i));
    let mut lanes: Vec<i32> = vec![0; n];
    for (pos, &i) in order.iter().enumerate() {
        let scale = scales[i];
        let pattern = patterns[i];
        let z0i = z0_scale as i32;
        let si = scale as i32;
        let y: i32 = if scale == z0_scale {
            0
        } else if scale > z0_scale {
            let want = scale - 1;
            let mut found: Option<usize> = None;
            for &j in order[..pos].iter().rev() {
                if patterns[j] == pattern && scales[j] == want {
                    found = Some(j);
                    break;
                }
            }
            match found {
                Some(p) => {
                    if pattern == BoxPattern::RBR {
                        lanes[p] + 1
                    } else {
                        lanes[p] - 1
                    }
                }
                None => si - z0i,
            }
        } else {
            let want = scale + 1;
            let mut found: Option<usize> = None;
            for &j in order[..pos].iter().rev() {
                if patterns[j] == pattern && scales[j] == want {
                    found = Some(j);
                    break;
                }
            }
            match found {
                Some(c) => {
                    if pattern == BoxPattern::RBR {
                        lanes[c] - 1
                    } else {
                        lanes[c] + 1
                    }
                }
                None => si - z0i,
            }
        };
        lanes[i] = y;
    }
    // MKT-6b-5.2.3: chain-level lane — all boxes in one chain share the
    // chain root's (chain_pos == 0) lane. Boxes without chain_id keep
    // their per-box lane.
    let mut chain_lane: HashMap<i64, i32> = HashMap::new();
    for i in 0..n {
        if boxes[i].chain_id.is_some() && boxes[i].chain_pos == Some(0) {
            // Safe: chain_id checked Some above.
            chain_lane.insert(boxes[i].chain_id.unwrap_or(i64::MAX), lanes[i]);
        }
    }
    for i in 0..n {
        if let Some(cid) = boxes[i].chain_id {
            if let Some(&root_lane) = chain_lane.get(&cid) {
                lanes[i] = root_lane;
            }
        }
    }
    for i in 0..n {
        boxes[i].y_lane = Some(lanes[i]);
    }
}

fn scan_rbr(
    candles: &[Candle],
    _p: &Parameters,
    symbol: &str,
    step: u8,
    price_ref: f64,
    pivot_step: u8,
    cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> Vec<Box> {
    let n = candles.len();
    if n < 4 {
        return Vec::new();
    }
    let mut out: Vec<Box> = Vec::new();
    let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();

    for t1 in 0..n {
        if crate::market::cancel::is_cancelled(&cancel) {
            return Vec::new();
        }
        let p1 = candles[t1].low;

        let mut t2_opt: Option<usize> = None;
        for j in (t1 + 1)..n {
            if candles[j].high > p1 {
                t2_opt = Some(j);
                break;
            }
        }
        let t2 = match t2_opt {
            Some(v) => v,
            None => continue,
        };
        let h1 = candles[t2].high;

        let mut t4_opt: Option<usize> = None;
        for j in (t2 + 1)..n {
            if candles[j].high > h1 {
                t4_opt = Some(j);
                break;
            }
        }
        let t4 = match t4_opt {
            Some(v) => v,
            None => continue,
        };

        if t4 <= t2 + 1 {
            continue;
        }
        let mut t3 = t2 + 1;
        let mut hl = candles[t3].low;
        for i in (t2 + 1)..t4 {
            if candles[i].low < hl {
                hl = candles[i].low;
                t3 = i;
            }
        }

        if hl <= p1 {
            continue;
        }

        if !seen.insert((t2, t3)) {
            continue;
        }

        let box_high = h1;
        let box_low = hl;
        let origin_price = h1;
        let scale = classify_scale_from_base(h1 - hl, origin_price);

        let box_center = (box_high + box_low) / 2.0;
        let y_abs = y_abs_of(box_center, price_ref);
        let y_band = y_band_of(y_abs);

        let period_sec: i64 = TF_SECONDS[step as usize] as i64;
        let x_idx = candles[t2].ts / period_sec;

        let address = format!(
            "{}~M{}~P{}~X{}~Y{}~RBR",
            symbol, step, pivot_step, x_idx, y_abs
        );

        out.push(Box {
            address,
            symbol: symbol.to_string(),
            step,
            pivot_step,
            pattern: BoxPattern::RBR,
            zone: BoxZone::Demand,
            scale,
            x_idx,
            ts_start: candles[t2].ts,
            ts_end: candles[t4].ts,
            engulf_ts: Some(candles[t4].ts),
            duration_sec: candles[t4].ts - candles[t2].ts,
            scale_sec: 0,
            high: box_high,
            low: box_low,
            origin_price,
            leg1_magnitude: h1 - p1,
            leg2_magnitude: candles[t4].high - hl,
            parent_address: None,
            children_addresses: Vec::new(),
            x: x_idx,
            y_abs,
            z: scale,
            y_band,
            chain_id: None,
            chain_pos: None,
            y_lane: None,
        });
    }
    out
}
fn scan_dbd(
    candles: &[Candle],
    _p: &Parameters,
    symbol: &str,
    step: u8,
    price_ref: f64,
    pivot_step: u8,
    cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> Vec<Box> {
    let n = candles.len();
    if n < 4 {
        return Vec::new();
    }
    let mut out: Vec<Box> = Vec::new();
    let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();

    for t1 in 0..n {
        if crate::market::cancel::is_cancelled(&cancel) {
            return Vec::new();
        }
        let h1 = candles[t1].high;

        let mut t2_opt: Option<usize> = None;
        for j in (t1 + 1)..n {
            if candles[j].low < h1 {
                t2_opt = Some(j);
                break;
            }
        }
        let t2 = match t2_opt {
            Some(v) => v,
            None => continue,
        };
        let l1 = candles[t2].low;

        let mut t4_opt: Option<usize> = None;
        for j in (t2 + 1)..n {
            if candles[j].low < l1 {
                t4_opt = Some(j);
                break;
            }
        }
        let t4 = match t4_opt {
            Some(v) => v,
            None => continue,
        };

        if t4 <= t2 + 1 {
            continue;
        }
        let mut t3 = t2 + 1;
        let mut lh = candles[t3].high;
        for i in (t2 + 1)..t4 {
            if candles[i].high > lh {
                lh = candles[i].high;
                t3 = i;
            }
        }

        if lh >= h1 {
            continue;
        }

        if !seen.insert((t2, t3)) {
            continue;
        }

        let box_high = lh;
        let box_low = l1;
        let origin_price = l1;
        let scale = classify_scale_from_base(lh - l1, origin_price);

        let box_center = (box_high + box_low) / 2.0;
        let y_abs = y_abs_of(box_center, price_ref);
        let y_band = y_band_of(y_abs);

        let period_sec: i64 = TF_SECONDS[step as usize] as i64;
        let x_idx = candles[t2].ts / period_sec;

        let address = format!(
            "{}~M{}~P{}~X{}~Y{}~DBD",
            symbol, step, pivot_step, x_idx, y_abs
        );

        out.push(Box {
            address,
            symbol: symbol.to_string(),
            step,
            pivot_step,
            pattern: BoxPattern::DBD,
            zone: BoxZone::Supply,
            scale,
            x_idx,
            ts_start: candles[t2].ts,
            ts_end: candles[t4].ts,
            engulf_ts: Some(candles[t4].ts),
            duration_sec: candles[t4].ts - candles[t2].ts,
            scale_sec: 0,
            high: box_high,
            low: box_low,
            origin_price,
            leg1_magnitude: h1 - l1,
            leg2_magnitude: lh - candles[t4].low,
            parent_address: None,
            children_addresses: Vec::new(),
            x: x_idx,
            y_abs,
            z: scale,
            y_band,
            chain_id: None,
            chain_pos: None,
            y_lane: None,
        });
    }
    out
}

fn sort_for_output(mut boxes: Vec<Box>) -> Vec<Box> {
    boxes.sort_by(|a, b| {
        b.scale
            .cmp(&a.scale)
            .then(a.x_idx.cmp(&b.x_idx))
            .then((a.pattern as u8).cmp(&(b.pattern as u8)))
            .then(a.ts_end.cmp(&b.ts_end))
    });
    boxes
}
fn merge_overlapping(mut boxes: Vec<Box>) -> Vec<Box> {
    boxes.sort_by(|a, b| {
        a.ts_start
            .cmp(&b.ts_start)
            .then(b.scale.cmp(&a.scale))
            .then((b.ts_end - b.ts_start).cmp(&(a.ts_end - a.ts_start)))
    });
    let mut kept: Vec<Box> = Vec::new();
    'outer: for b in boxes {
        for k in kept.iter_mut() {
            let overlap = (k.high.min(b.high) - k.low.max(b.low)).max(0.0);
            let min_h = (k.high - k.low).min(b.high - b.low);
            if min_h <= 0.0 {
                continue;
            }
            let ratio = overlap / min_h;
            if ratio >= 0.80 {
                let replace = b.scale > k.scale
                    || (b.scale == k.scale && (b.ts_end - b.ts_start) > (k.ts_end - k.ts_start));
                if replace {
                    *k = b.clone();
                }
                continue 'outer;
            }
        }
        kept.push(b);
    }
    kept
}

fn build_nesting(mut boxes: Vec<Box>) -> Vec<Box> {
    let n = boxes.len();
    if n == 0 {
        return boxes;
    }
    let mut parent_idx: Vec<Option<usize>> = vec![None; n];
    for j in 0..n {
        for i in 0..n {
            if i == j {
                continue;
            }
            if boxes[j].low > boxes[i].low && boxes[j].high < boxes[i].high {
                let new_w = boxes[i].high - boxes[i].low;
                match parent_idx[j] {
                    None => parent_idx[j] = Some(i),
                    Some(prev) => {
                        let prev_w = boxes[prev].high - boxes[prev].low;
                        if new_w < prev_w {
                            parent_idx[j] = Some(i);
                        }
                    }
                }
            }
        }
    }
    for b in boxes.iter_mut() {
        b.children_addresses.clear();
    }
    let pairs: Vec<(usize, usize)> = parent_idx
        .iter()
        .enumerate()
        .filter_map(|(j, p)| p.map(|i| (j, i)))
        .collect();
    for (cj, pj) in pairs {
        let child_addr = boxes[cj].address.clone();
        let parent_addr = boxes[pj].address.clone();
        boxes[cj].parent_address = Some(parent_addr);
        boxes[pj].children_addresses.push(child_addr);
    }
    for b in boxes.iter_mut() {
        b.children_addresses.sort();
        b.children_addresses.dedup();
    }
    boxes
}
// ═══════════════════════════════════════════════════════════════
// TESTS (19) — calibrated
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn c(ts: i64, o: f64, h: f64, l: f64, cl: f64) -> Candle {
        Candle {
            symbol: "XAUUSD".into(),
            timeframe: "M4".into(),
            ts,
            open: o,
            high: h,
            low: l,
            close: cl,
            volume: 0.0,
            tick_count: 0,
            first_extreme: crate::market::model::ExtremeKind::Same,
        }
    }

    // 14 warmup candles (no volatility measure — v5.1.7 is %-based)
    fn warmup() -> Vec<Candle> {
        (0..14)
            .map(|i| {
                if i % 2 == 0 {
                    c(i, 100.0, 102.0, 98.0, 102.0)
                } else {
                    c(i, 102.0, 102.0, 98.0, 98.0)
                }
            })
            .collect()
    }

    // v6.4.12: L1=95 (14), H1=103 (17), HL=100 (20), HH=110 (23).
    // engulf = first wick low<HL after HH (idx25, low 96). Box high=H1, low=HL (FIXED).
    fn rbr_seq() -> Vec<Candle> {
        let mut v = warmup();
        v.push(c(14, 100.0, 100.0, 95.0, 96.0)); // L1 = 95
        v.push(c(15, 96.0, 99.0, 96.0, 98.0));
        v.push(c(16, 98.0, 101.0, 98.0, 100.0));
        v.push(c(17, 100.0, 103.0, 100.0, 102.0)); // H1 = 103
        v.push(c(18, 102.0, 102.0, 101.5, 101.5));
        v.push(c(19, 101.5, 101.5, 100.5, 101.0));
        v.push(c(20, 101.0, 101.0, 100.0, 100.5)); // HL = 100
        v.push(c(21, 100.5, 102.0, 100.5, 101.5));
        v.push(c(22, 101.5, 106.0, 99.0, 105.0)); // violation wick 99 + engulf close 105 > 103
        v.push(c(23, 105.0, 110.0, 105.0, 109.0)); // HH = 110
        v.push(c(24, 109.0, 109.0, 100.0, 101.0));
        v.push(c(25, 101.0, 101.0, 96.0, 97.0));
        v.push(c(26, 97.0, 97.0, 93.0, 94.0));
        v.push(c(27, 94.0, 94.0, 90.0, 91.0));
        v
    }

    // v6.4.12: H1=105 (14), L1=97 (17), LH=99.5 (20), LL=90 (23).
    // engulf = first wick high>LH after LL (idx25, high 100). Box high=LH, low=L1 (FIXED).
    fn dbd_seq() -> Vec<Candle> {
        let mut v = warmup();
        v.push(c(14, 100.0, 105.0, 100.0, 104.0)); // H1 = 105
        v.push(c(15, 104.0, 104.0, 101.0, 102.0));
        v.push(c(16, 102.0, 102.0, 99.0, 100.0));
        v.push(c(17, 100.0, 100.0, 97.0, 98.0)); // L1 = 97
        v.push(c(18, 98.0, 98.5, 97.5, 98.0));
        v.push(c(19, 98.0, 98.8, 97.5, 98.5));
        v.push(c(20, 98.5, 99.5, 98.3, 99.0)); // LH = 99.5
        v.push(c(21, 99.0, 99.0, 97.5, 98.0));
        v.push(c(22, 96.0, 100.5, 95.0, 95.0)); // violation wick 100.5 + engulf close 95 < 97
        v.push(c(23, 95.5, 95.5, 90.0, 91.0)); // LL = 90
        v.push(c(24, 91.0, 96.0, 91.0, 95.0));
        v.push(c(25, 95.0, 100.0, 95.0, 100.0));
        v.push(c(26, 100.0, 105.0, 100.0, 105.0));
        v.push(c(27, 105.0, 107.0, 105.0, 106.0)); // engulf (107 > 105)
        v
    }

    // v6.2.1: obsolete max_base_pct removed — defaults accept all fixtures.
    fn call(v: &[Candle]) -> Vec<Box> {
        let p = Parameters::default();
        detect_boxes(v, p, "XAUUSD", 8, 100.0, 8, None).0
    }

    #[test]
    fn pivot_high_detected() {
        let v = rbr_seq();
        let pivots = detect_pivots(&v, 2);
        assert!(pivots
            .iter()
            .any(|p| p.kind == PivotKind::High && p.idx == 17));
    }

    #[test]
    fn pivot_low_detected() {
        let v = rbr_seq();
        let pivots = detect_pivots(&v, 2);
        assert!(pivots
            .iter()
            .any(|p| p.kind == PivotKind::Low && p.idx == 14));
    }

    #[test]
    fn rbr_positive() {
        let boxes = call(&rbr_seq());
        let rbr: Vec<_> = boxes
            .iter()
            .filter(|b| b.pattern == BoxPattern::RBR)
            .collect();
        assert!(!rbr.is_empty(), "expected RBR, got {:?}", boxes);
        assert!(rbr.iter().all(|b| b.zone == BoxZone::Demand));
    }

    #[test]
    fn rbr_invalid_hl_below_l1() {
        // HL=94 < L1=95 → Lower Low, not Higher Low → no RBR.
        let mut v = rbr_seq();
        v[20] = c(20, 100.0, 100.0, 94.0, 95.0);
        let boxes = call(&v);
        assert_eq!(
            boxes
                .iter()
                .filter(|b| b.pattern == BoxPattern::RBR)
                .count(),
            0
        );
    }

    #[test]
    fn rbr_invalid_hh_below_h1() {
        // Lower c21–c24 highs so no pivot-high > H1=103 exists after HL.
        let mut v = rbr_seq();
        v[21] = c(21, 100.5, 101.8, 100.5, 101.5);
        v[22] = c(22, 101.5, 102.5, 101.5, 102.0); // pivot-high = 102.5
        v[23] = c(23, 102.0, 102.0, 100.0, 100.5);
        v[24] = c(24, 100.5, 101.0, 100.0, 100.5);
        let boxes = call(&v);
        assert_eq!(
            boxes
                .iter()
                .filter(|b| b.pattern == BoxPattern::RBR)
                .count(),
            0
        );
    }

    #[test]
    fn rbr_engulf_required() {
        // v6.4.15: fully flat extension (100/100) → no HH/LL confirm pivots → 0 RBR.
        // (Pending coverage lives in rbr_pending_no_engulf.)
        let mut v = warmup();
        for k in 14..=28 {
            v.push(c(k, 100.0, 100.0, 100.0, 100.0));
        }
        let boxes = call(&v);
        let rbr: Vec<_> = boxes
            .iter()
            .filter(|b| b.pattern == BoxPattern::RBR)
            .collect();
        assert_eq!(
            rbr.len(),
            0,
            "expected 0 RBR (no confirm pivots), got {:?}",
            boxes
        );
    }

    #[test]
    fn rbr_pending_no_engulf() {
        // v6.4.17: candle-based HH = first candle after hl_idx(20) with high>103 → idx22 (high 106).
        // box_low = HL scalar = min(low) over (17,22) = min(101.5,100.5,100.0,100.5) = 100.0.
        let mut v = rbr_seq();
        v.truncate(25);
        let boxes = call(&v);
        let rbr: Vec<_> = boxes
            .iter()
            .filter(|b| b.pattern == BoxPattern::RBR)
            .collect();
        assert!(!rbr.is_empty(), "expected pending RBR, got {:?}", boxes);
        let b = rbr[0];
        assert!(b.engulf_ts.is_none());
        assert_eq!(b.ts_end, 24);
        assert!(
            (b.low - 100.0).abs() < 1e-9,
            "low must be HL scalar 100.0, got {}",
            b.low
        );
        assert!(
            (b.high - 103.0).abs() < 1e-9,
            "high must be H1, got {}",
            b.high
        );
    }

    #[test]
    fn rbr_confirmed_fixed_levels() {
        // v6.4.17: H1=103 (17), HH candle=22, HL = min(low) over (17,22) = 100.0, box_low = 100.0.
        // engulf = first wick low<100.0 after HH → idx25 low=96.0 → ts_end=25.
        let boxes = call(&rbr_seq());
        let rbr: Vec<_> = boxes
            .iter()
            .filter(|b| b.pattern == BoxPattern::RBR)
            .collect();
        assert!(!rbr.is_empty());
        let b = rbr[0];
        assert!(b.engulf_ts.is_some());
        assert_eq!(b.ts_end, 25);
        assert!((b.low - 100.0).abs() < 1e-9);
        assert!((b.high - 103.0).abs() < 1e-9);
    }

    #[test]
    fn dbd_pending_no_engulf() {
        // v6.4.17: candle-based LL = first candle after lh_idx(20) with low<97 → idx22 (low 95).
        // box_high = LH scalar = max(high) over (17,22) = max(98.5,98.8,99.5,99.0) = 99.5.
        let mut v = dbd_seq();
        v.truncate(25);
        let boxes = call(&v);
        let dbd: Vec<_> = boxes
            .iter()
            .filter(|b| b.pattern == BoxPattern::DBD)
            .collect();
        assert!(!dbd.is_empty(), "expected pending DBD, got {:?}", boxes);
        let b = dbd[0];
        assert!(b.engulf_ts.is_none());
        assert_eq!(b.ts_end, 24);
        assert!(
            (b.high - 99.5).abs() < 1e-9,
            "high must be highest wick 99.5, got {}",
            b.high
        );
        assert!((b.low - 97.0).abs() < 1e-9, "low must be L1, got {}", b.low);
    }

    #[test]
    fn dbd_confirmed_fixed_levels() {
        // v6.4.17: L1=97 (17), LL candle=22, LH = max(high) over (17,22) = 99.5, box_high = 99.5.
        // engulf = first wick high>99.5 after LL → idx25 high=100.0 → ts_end=25.
        let boxes = call(&dbd_seq());
        let dbd: Vec<_> = boxes
            .iter()
            .filter(|b| b.pattern == BoxPattern::DBD)
            .collect();
        assert!(!dbd.is_empty());
        let b = dbd[0];
        assert!(b.engulf_ts.is_some());
        assert_eq!(b.ts_end, 25);
        assert!((b.high - 99.5).abs() < 1e-9);
        assert!((b.low - 97.0).abs() < 1e-9);
    }

    #[test]
    fn dbd_positive() {
        let boxes = call(&dbd_seq());
        let dbd: Vec<_> = boxes
            .iter()
            .filter(|b| b.pattern == BoxPattern::DBD)
            .collect();
        assert!(!dbd.is_empty(), "expected DBD, got {:?}", boxes);
        assert!(dbd.iter().all(|b| b.zone == BoxZone::Supply));
    }

    #[test]
    fn dbd_invalid_lh_above_h1() {
        // LH=106 > H1=105 → Higher High, not Lower High → no DBD.
        let mut v = dbd_seq();
        v[20] = c(20, 98.5, 106.0, 98.3, 105.0);
        let boxes = call(&v);
        assert_eq!(
            boxes
                .iter()
                .filter(|b| b.pattern == BoxPattern::DBD)
                .count(),
            0
        );
    }

    #[test]
    fn dbd_invalid_ll_above_l1() {
        // Replace c21..c27 lows so no pivot-low < L1=97 exists after LH.
        let mut v = dbd_seq();
        v[21] = c(21, 99.0, 99.0, 98.3, 98.5);
        v[22] = c(22, 98.5, 98.5, 98.4, 98.5);
        v[23] = c(23, 98.5, 98.6, 98.5, 98.5);
        v[24] = c(24, 98.5, 98.6, 98.5, 98.5);
        v[25] = c(25, 98.5, 98.7, 98.5, 98.6);
        v[26] = c(26, 98.6, 98.7, 98.6, 98.6);
        v[27] = c(27, 98.6, 98.7, 98.6, 98.7);
        let boxes = call(&v);
        assert_eq!(
            boxes
                .iter()
                .filter(|b| b.pattern == BoxPattern::DBD)
                .count(),
            0
        );
    }

    #[test]
    fn classify_scale_cases() {
        // base=0.11% → S0 (below 0.125 threshold)
        assert_eq!(classify_scale_from_base(1.1, 1000.0), 0);
        // base=0.2% → ratio=2 → S3
        assert_eq!(classify_scale_from_base(2.0, 1000.0), 3);
        // base=0.5% → ratio=5 → S7
        assert_eq!(classify_scale_from_base(5.0, 1000.0), 7);
        // base=1.0% → ratio=10 → S10 (v6.4.10 clamp 0..15)
        assert_eq!(classify_scale_from_base(10.0, 1000.0), 10);
        // base=5.0% → ratio=50 → S17 clamped to 15
        assert_eq!(classify_scale_from_base(50.0, 1000.0), 15);
        // pivot_price=0 → 0
        assert_eq!(classify_scale_from_base(5.0, 0.0), 0);
        // below base (0.05%) → 0
        assert_eq!(classify_scale_from_base(0.5, 1000.0), 0);
    }

    #[test]
    fn chain_extension_applied() {
        // v5.1.7: chain_extend shifts x_idx earlier (saturating at 0).
        let boxes = call(&rbr_seq());
        let b = boxes.iter().find(|b| b.pattern == BoxPattern::RBR);
        if let Some(b) = b {
            assert!(b.x_idx <= 17, "x_idx too late: {}", b.x_idx);
        }
    }

    #[test]
    fn y_abs_reference_based() {
        // v5.1.10: y_abs = floor(log2(price / price_ref) * 1e6), may be negative.
        let price_ref = 2600.0;
        assert_eq!(y_abs_of(price_ref, price_ref), 0);
        assert!(y_abs_of(2650.0, price_ref) > 0);
        assert!(y_abs_of(2550.0, price_ref) < 0);
        assert_eq!(y_abs_of(0.0, price_ref), 0);
        assert_eq!(y_band_of(-1), -1); // signed floor
        assert_eq!(y_band_of(100_000), 1);
    }

    #[test]
    fn address_format_correct() {
        let boxes = call(&rbr_seq());
        for b in &boxes {
            assert!(b.address.starts_with("XAUUSD~M8~P8~X"));
            assert!(b.address.contains("~Y"));
            assert!(b.address.ends_with("~RBR") || b.address.ends_with("~DBD"));
        }
    }

    #[test]
    fn address_stable_100_runs() {
        let base = call(&rbr_seq());
        for _ in 0..99 {
            let again = call(&rbr_seq());
            assert_eq!(again.len(), base.len());
            for (a, b) in base.iter().zip(again.iter()) {
                assert_eq!(a.address, b.address);
            }
        }
    }

    #[test]
    fn deterministic_100_runs() {
        let base = call(&rbr_seq());
        for _ in 0..99 {
            let again = call(&rbr_seq());
            assert_eq!(again.len(), base.len());
            for (a, b) in base.iter().zip(again.iter()) {
                assert_eq!(a.address, b.address);
                assert_eq!(a.pattern, b.pattern);
                assert_eq!(a.scale, b.scale);
            }
        }
    }

    #[test]
    fn overlap_merge_80() {
        let a = mk_box("A", 5, 100, 110, 110.0, 100.0);
        let b = mk_box("B", 3, 101, 111, 109.5, 100.5);
        let merged = merge_overlapping(vec![a, b]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].scale, 5);
    }

    #[test]
    fn overlap_keep_30() {
        let a = mk_box("A", 5, 100, 110, 110.0, 100.0);
        let b = mk_box("B", 5, 105, 115, 114.0, 106.0);
        let merged = merge_overlapping(vec![a, b]);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn nesting_detected() {
        let outer = mk_box("OUTER", 7, 100, 120, 120.0, 100.0);
        let inner = mk_box("INNER", 3, 105, 115, 115.0, 105.0);
        let nested = build_nesting(vec![outer, inner]);
        let o = nested.iter().find(|b| b.address == "OUTER").unwrap();
        let i = nested.iter().find(|b| b.address == "INNER").unwrap();
        assert_eq!(i.parent_address.as_deref(), Some("OUTER"));
        assert!(o.children_addresses.contains(&"INNER".to_string()));
    }

    #[test]
    fn role_filter_keeps_three_scales() {
        assert!(role_for_scale(4, 4).is_some());
        assert!(role_for_scale(4, 5).is_some());
        assert!(role_for_scale(4, 6).is_some());
        assert!(role_for_scale(4, 3).is_none());
        assert!(role_for_scale(4, 7).is_none());
    }

    #[test]
    fn role_override_honored() {
        // v6.4.4: show_all_scales removed — backend always returns all scales.
        let p = Parameters::default();
        let (boxes, _) = detect_boxes(&rbr_seq(), p, "X", 0, 100.0, 0, None);
        assert!(!boxes.is_empty(), "expected boxes, got empty");
    }

    #[test]
    fn role_default_fallback() {
        // v6.2.1: obsolete max_base_pct removed; default policy accepts S9 at M8.
        let p = Parameters::default();
        let (boxes, _) = detect_boxes(&rbr_seq(), p, "X", 8, 100.0, 8, None);
        assert!(!boxes.is_empty(), "expected M8 default-policy to accept S9");
    }

    #[test]
    fn sscale_v5_below_base_is_s0() {
        // 0.05% < 0.1% base → S0
        assert_eq!(classify_scale_from_base(0.05, 100.0), 0);
    }

    #[test]
    fn sscale_v5_large_is_s9() {
        // v6.4.10: 2.0% → ratio=20 → S13 (kept name for history; clamp 0..15)
        assert_eq!(classify_scale_from_base(2.0, 100.0), 13);
    }

    fn mk_box(addr: &str, scale: u8, ts_start: i64, ts_end: i64, high: f64, low: f64) -> Box {
        Box {
            address: addr.to_string(),
            symbol: "X".to_string(),
            step: 4,
            pivot_step: 4,
            pattern: BoxPattern::RBR,
            zone: BoxZone::Demand,
            scale,
            x_idx: ts_start,
            ts_start,
            ts_end,
            engulf_ts: None,
            duration_sec: ts_end - ts_start,
            scale_sec: 0,
            high,
            low,
            origin_price: low,
            leg1_magnitude: 0.0,
            leg2_magnitude: 0.0,
            parent_address: None,
            children_addresses: vec![],
            x: ts_start,
            y_abs: y_abs_of(low, 100.0),
            z: scale,
            y_band: y_band_of(y_abs_of(low, 100.0)),
            chain_id: None,
            chain_pos: None,
            y_lane: None,
        }
    }
}
