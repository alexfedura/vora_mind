//! Swap-chain algorithm: alternating RBR/DBD boxes that invalidate
//! each other (wick-based engulf) on the same (y_band, z, size_bucket).
//! v5.1.2: group by y_band only; size tolerance ±20% at chain time.

use crate::market::boxes::{Box, BoxPattern};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainParams {
    pub min_chain_len: usize,
    pub size_quantum: f64, // DEPRECATED v5.1.1 (unused)
    pub time_gap_max: i64, // DEPRECATED v5.1.1 (unused)
}

impl Default for ChainParams {
    fn default() -> Self {
        Self {
            min_chain_len: 2,
            size_quantum: 0.001,
            time_gap_max: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    pub id: i64,
    pub y_band: i64,
    pub z: u8,
    pub boxes: Vec<String>,
    pub kind_seq: Vec<String>,
    pub size_min: f64,
    pub size_max: f64,
}

pub fn pattern_str(p: &BoxPattern) -> &'static str {
    match p {
        BoxPattern::RBR => "RBR",
        BoxPattern::DBD => "DBD",
        BoxPattern::RBD => "RBD",
        BoxPattern::DBR => "DBR",
    }
}

#[allow(dead_code)]
fn size_bucket(b: &Box, quantum: f64) -> i64 {
    let denom = if b.origin_price > 0.0 {
        b.origin_price
    } else {
        1.0
    };
    let base_height = b.high - b.low;
    ((base_height / denom) / quantum).round() as i64
}

fn flush_run(
    cur: &mut Vec<usize>,
    y_band: i64,
    min_len: usize,
    boxes: &mut [Box],
    chains: &mut Vec<Chain>,
    next_id: &mut i64,
) {
    if cur.len() < min_len {
        cur.clear();
        return;
    }
    let cid = *next_id;
    *next_id += 1;
    let ids: Vec<String> = cur.iter().map(|&k| boxes[k].address.clone()).collect();
    let kinds: Vec<String> = cur
        .iter()
        .map(|&k| pattern_str(&boxes[k].pattern).to_string())
        .collect();
    // v5.1.2: Chain.z = first box's z (informational only)
    let z_first = boxes[cur[0]].z;
    // v5.1.2: compute size_min / size_max from base heights
    let mut size_min = f64::INFINITY;
    let mut size_max = f64::NEG_INFINITY;
    for &k in cur.iter() {
        let h = boxes[k].high - boxes[k].low;
        if h < size_min {
            size_min = h;
        }
        if h > size_max {
            size_max = h;
        }
    }
    for (pos, &k) in cur.iter().enumerate() {
        boxes[k].chain_id = Some(cid);
        boxes[k].chain_pos = Some(pos as u32);
    }
    chains.push(Chain {
        id: cid,
        y_band,
        z: z_first,
        boxes: ids,
        kind_seq: kinds,
        size_min,
        size_max,
    });
    cur.clear();
}

/// Assign swap chains over `boxes` (mutated in place: chain_id/chain_pos).
/// Group key: (y_band, z, size_bucket). Within a group sort by `x` and
/// extend the run while: time gap <= max AND strict RBR<->DBD alternation
/// AND wick-based invalidation (RBR->DBD: curr.low < prev.low;
/// DBD->RBR: curr.high > prev.high). Time overlap allowed.
pub fn assign_chains(boxes: &mut [Box], p: &ChainParams) -> Vec<Chain> {
    // v5.1.2: group by y_band only; size checked at chain time (±20%)
    let mut groups: HashMap<i64, Vec<usize>> = HashMap::new();
    for (i, b) in boxes.iter().enumerate() {
        groups.entry(b.y_band).or_default().push(i);
    }
    let mut chains: Vec<Chain> = Vec::new();
    let mut next_id: i64 = 1;
    for (y_band, mut idxs) in groups {
        idxs.sort_by_key(|&i| boxes[i].x);
        let mut cur: Vec<usize> = Vec::new();
        for &i in &idxs {
            if cur.is_empty() {
                cur.push(i);
                continue;
            }
            let prev_i = *cur.last().expect("non-empty");
            let (plow, phigh, ppat) =
                (boxes[prev_i].low, boxes[prev_i].high, boxes[prev_i].pattern);
            let (clow, chigh, cpat) = (boxes[i].low, boxes[i].high, boxes[i].pattern);
            // v5.1.1: time is NOT part of chain condition.
            let alt = matches!(
                (ppat, cpat),
                (BoxPattern::RBR, BoxPattern::DBD) | (BoxPattern::DBD, BoxPattern::RBR)
            );
            let inval = match (ppat, cpat) {
                (BoxPattern::RBR, BoxPattern::DBD) => clow < plow,
                (BoxPattern::DBD, BoxPattern::RBR) => chigh > phigh,
                _ => false,
            };
            // v5.1.2: size tolerance ±20% (relative to larger of the two)
            let h1 = boxes[prev_i].high - boxes[prev_i].low;
            let h2 = boxes[i].high - boxes[i].low;
            let max_h = if h1 > h2 { h1 } else { h2 };
            let size_ok = max_h > 0.0 && (h1 - h2).abs() / max_h <= 0.20;

            if alt && inval && size_ok {
                cur.push(i);
            } else {
                flush_run(
                    &mut cur,
                    y_band,
                    p.min_chain_len,
                    boxes,
                    &mut chains,
                    &mut next_id,
                );
                cur.push(i);
            }
        }
        flush_run(
            &mut cur,
            y_band,
            p.min_chain_len,
            boxes,
            &mut chains,
            &mut next_id,
        );
    }
    chains
}
