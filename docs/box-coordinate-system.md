```markdown
# Vora Mind — Box Coordinate System

**Version:** 3.0.0
**Status:** ACTIVE
**Effective From:** 2026-10-01
**Owner:** Alex Fedura
**Supersedes:** v2.1.0

---

## 1. Overview

This document defines the **Universal Box Coordinate System** for Vora Mind
— a 3-dimensional, absolute, asset-independent coordinate system that
assigns every supply/demand box a globally unique, mathematically-reproducible
address.

**Design principles:**

1. **Absolute** — no arbitrary reference points. All axes anchored to
   universal constants (Unix epoch, unity price, zero scale).
2. **Universal** — SHIB ($10⁻⁷) and BTC ($10⁵) share the SAME coordinate
   space. No per-symbol tuning.
3. **Laser-precise** — Y-axis uses log₂ with 10⁻⁶ resolution, giving
   ~7-digit price accuracy.
4. **Deterministic** — same input → same address, bit-for-bit.
5. **Independent of asset range** — a SHIB box and a BTC box at the same
   time with the same scale differ ONLY in their Y coordinate.

---

## 2. The 3 Axes

### 2.1 X-Axis — Time (Absolute)

```
x = floor(ts_sec / period(M))
```

Where:
- `ts_sec` = box start time in Unix seconds
- `period(M) = 10^M` seconds
- Origin: 1970-01-01 00:00:00 UTC (Unix epoch)

**Box start time** (`ts_sec`) is the CANONICAL base start:

- RBR: `ts_sec = ts[H1_idx]` (start of base = top of Leg 1)
- DBD: `ts_sec = ts[L1_idx]` (start of base = bottom of Leg 1)

See `docs/box-standard.md §7.2` for the full box extent definition.

**Range:** x ≥ 0 (non-negative)
**Cell size:** exactly one M-period

**Examples:**

| M | period | ts_sec = 1,735,772,400 → x |
|---|---|---|
| M0 | 1 s | 1,735,772,400 |
| M3 | 1,000 s | 1,735,772 |
| M6 | 1,000,000 s | 1,735 |
| M9 | 1,000,000,000 s | 1 |

**Note:** Each M has a DIFFERENT x for the same time. This is intentional —
x is expressed in units of the current chart's period.

### 2.2 Y-Axis — Price (Reference-Based Log, Universal)

```
y_abs = floor( log2(price / price_ref) * 10^6 )

Where:
  price     = box center = (high + low) / 2
  price_ref = OPEN of first candle >= 2025-01-15 00:00:00 UTC
              (fallback: first available candle)

y_abs MAY BE NEGATIVE for data before price_ref_ts.
```

### 2.3 Z-Axis — S-Scale

```
z = S-scale (0..9)
```

**Range:** [0, 9]
**Origin:** S0 (smallest scale)

**Why no negative z:** S-Scale formula (`docs/box-standard.md §5`)
produces 0..9 by construction. There is no "smaller than smallest".

---

## 3. Full Address Format

### 3.1 Structure

```
{symbol}~M{m}~X{x}~Y{y_abs}~S{z}
```

**Example:**
```
XAUUSD~M3~X1735772~Y12135700~S5
```

### 3.2 Box Center Price

For each box, `price` in §2.2 is the box center:

```
box_center = (box.high + box.low) / 2.0
```

Using center (not high, not low) is a design choice:
- Stable under retracements
- Matches the "representative price" of the box
- Independent of `first_extreme` ordering

**Note:** `box.high` and `box.low` are the CANONICAL base bounds:

- RBR: `high = H1`, `low = HL`
- DBD: `high = LH`, `low = L1`

See `docs/box-standard.md §2.2` and `§7.2`.

### 3.3 Address Stability

An address is stable **iff**:
- Candle data unchanged
- Detection parameters unchanged
- `box.high` and `box.low` unchanged

Given these, the 5-tuple `(symbol, M, x, y_abs, z)` is deterministic.

---

## 4. Bands (for Chain Grouping)

### 4.1 Band Definition

Bands are **log-spaced** price slices for grouping boxes into streams:

```
BAND_GRID = 10^5
y_band = floor(y_abs / BAND_GRID)
```

Each band spans:
```
Δ log₂(price) = 0.1
Δ price/price ≈ 6.9%   (uniform across all assets)
```

**Example (XAUUSD at $4,500):**

| Price | y_abs | y_band |
|---|---|---|
| $4,400 | 12,104,000 | 121 |
| $4,500 | 12,136,000 | 121 |
| $4,600 | 12,167,000 | 121 |
| $4,700 | 12,198,000 | 121 |
| $4,800 | 12,228,000 | 122 |

### 4.2 Band Universality

The band width is **7% of price for every asset**. This is the KEY
property that makes chains comparable across symbols.

- XAUUSD band = ~$315 (at $4,500)
- BTCUSD band = ~$5,600 (at $80,000)
- SHIB band = ~$7 × 10⁻⁹ (at $10⁻⁷)

All these bands represent the same **relative** move.

### 4.3 Streams

A **stream** is:

```
stream = (symbol, M, y_band)
```

All boxes in the same stream are grouped together. Within a stream,
boxes are ordered by `x` (time). Chains are formed by consecutive
boxes in the same stream (see §5).

**Note:** In v2.1.0, the stream key no longer includes `z`. Size
compatibility between adjacent boxes is enforced at chain-construction
time via a ±20% tolerance on `base_height` (see §5.2). This change
was motivated by the observation that z is a coarse quantization of
base_height, and two boxes of similar size can land in different z
buckets.

---

## 5. Chains

### 5.1 Definition

A **chain** is a maximal subsequence of a stream where consecutive boxes
satisfy:

```
box[i+1].pattern ≠ box[i].pattern       (alternating RBR ↔ DBD)
AND
box[i+1] engulfs box[i]                 (wick-based, see §11)
AND
|h1 − h2| / max(h1, h2) ≤ 0.20          (size tolerance ±20%)
```

Where `h1 = box[i].high − box[i].low` and `h2 = box[i+1].high − box[i+1].low`.

**Time is NOT part of the chain condition.** A chain may span any
gap in `x`. This is intentional: swap chains represent structural
price action, not bounded time windows.

### 5.2 Size Tolerance

Two boxes are considered "same scale" iff their base heights are within
±20% of each other, relative to the larger of the two:

```
same_scale = |h1 − h2| / max(h1, h2) ≤ 0.20
```

**Rationale:** `z` (S-scale) is a discrete quantization of base height.
Two boxes with nearly identical base heights can fall into different
z-buckets. The ±20% tolerance preserves the semantic intent of "same
scale" while accommodating quantization boundaries.

### 5.3 Chain ID and Position

For each chain:

```
chain_id = box[0].x        (x of first box in chain)
chain_pos = i              (0-based index in chain)
```

### 5.4 Extended Address (with Chain)

```
{symbol}~M{m}~X{x}~Y{y_abs}~S{z}~C{chain_id}~P{chain_pos}
```

**Example:**
```
XAUUSD~M3~X1735800~Y12136000~S5~C1735772~P3
```

The 5-tuple (§3.1) remains valid as an absolute coordinate.
The 7-tuple adds chain context.

---

## 6. Coordinate Reference Constants

```
X_ORIGIN   = Unix epoch (ts_sec = 0)
X_UNIT     = period(M) = 10^M seconds
Y_ORIGIN   = price_ref per (symbol, M) (y_abs = 0 at price_ref)
Y_UNIT     = 10^-6 in log₂ space
Z_ORIGIN   = S0
BAND_GRID  = 10^5 (y_abs units)
SIZE_TOL   = 0.20 (±20% base-height tolerance for chain grouping)
REF_TS = 1_736_899_200  // 2025-01-15 00:00:00 UTC
```

**No per-symbol constants except cached price_ref.**

### 6.1 symbol_origin cache

```
symbol_origin table caches price_ref per (symbol, m_scale).
Symbol list is extensible; each symbol may have a DIFFERENT
price_ref_ts if 2025-01-15 is unavailable.
```

---

## 7. Comparison with v1.0.0

| Aspect | v1.0.0 (deprecated) | v2.0.0 / v2.1.0 |
|---|---|---|
| x formula | relative to ref_ts | absolute (Unix epoch) |
| y formula | linear: (p − base)/band | log₂: floor(log₂(p) × 10⁶) |
| y range | per-symbol | universal |
| base_price | per-symbol (2000 for XAUUSD) | **removed** |
| Comparability | per-symbol only | across all assets |
| Precision | 1 ATR | 7 × 10⁻⁷ relative |
| Bands | per-symbol ATR | universal log-spaced |
| Stream key | (symbol, M, S, y_band) | (symbol, M, y_band) — v2.1.0 |
| Chain rule | gap + alternation | alternation + engulf + size ±20% — v2.1.0 |

---

## 8. Worked Examples

### 8.1 XAUUSD M3 — RBR

Box: RBR, H1=$4,510, HL=$4,490, ts_start=1,735,772,400

```
box_center = (4510 + 4490) / 2 = 4500.0
y_abs      = floor(log₂(4500.0) × 10^6)
           = floor(12.1357 × 10^6)
           = 12,135,700
x          = floor(1,735,772,400 / 1000) = 1,735,772
z          = S-scale (from base_height / H1)

Address = XAUUSD~M3~X1735772~Y12135700~S5
```

### 8.2 BTCUSD M3 — RBR

Box: RBR, H1=$81,000, HL=$79,000, ts_start=1,735,772,400

```
box_center = (81000 + 79000) / 2 = 80000.0
y_abs      = floor(log₂(80000.0) × 10^6)
           = floor(16.2877 × 10^6)
           = 16,287,700
x          = floor(1,735,772,400 / 1000) = 1,735,772
z          = 5

Address = BTCUSD~M3~X1735772~Y16287700~S5
```

### 8.3 SHIBUSD M3 — RBR

Box: RBR, H1=$1.01×10⁻⁷, HL=$0.99×10⁻⁷, same time

```
box_center = 1.0 × 10⁻⁷
y_abs      = floor(log₂(1e-7) × 10^6)
           = floor(-23.2535 × 10^6)
           = -23,253,500
x          = 1,735,772
z          = 5

Address = SHIBUSD~M3~X1735772~Y-23253500~S5
```

### 8.4 Interpretation

All three boxes:
- **Same x** → same time
- **Same z** → same S-scale
- **Different y** → different price levels, but all in the SAME
  universal log-space

**Cross-asset distance:**

```
BTC.y − XAUUSD.y = 16,287,700 − 12,135,700 = 4,152,000 units
XAUUSD.y − SHIB.y = 12,135,700 − (−23,253,500) = 35,389,200 units
```

These distances are CONSTANTS, independent of currency, exchange,
or time. They reflect the pure multiplicative ratio in log-space.

---

## 9. Data Model

### 9.1 Rust Struct

```rust
pub struct Box {
    pub address: String,        // "XAUUSD~M3~X1735772~Y12135700~S5"

    pub symbol: String,
    pub m_scale: u8,
    pub pattern: BoxPattern,
    pub zone: BoxZone,
    pub scale: u8,

    // Universal coordinates
    pub x: i64,                 // time cell (from base start)
    pub y_abs: i64,             // log₂(center_price) × 10^6
    pub z: u8,                  // = scale

    // Chain
    pub chain_id: Option<i64>,  // x of first box in chain; None if unassigned
    pub chain_pos: Option<u32>, // 0-based position in chain; None if unassigned

    // Band (derived from y_abs)
    pub y_band: i64,            // floor(y_abs / BAND_GRID)

    // Geometry (canonical — see box-standard.md §7.2)
    pub ts_start: i64,          // RBR: ts[H1_idx]  |  DBD: ts[L1_idx]
    pub ts_end: i64,            // RBR: ts[HL_idx]  |  DBD: ts[LH_idx]
    pub high: f64,              // RBR: H1          |  DBD: LH
    pub low: f64,               // RBR: HL          |  DBD: L1
    pub origin_price: f64,      // pivot reference for S-scale (H1 for RBR, L1 for DBD)
    pub leg1_magnitude: f64,
    pub leg2_magnitude: f64,
    pub parent_address: Option<String>,
    pub children_addresses: Vec<String>,
}
```

**Persistence note:** `ts_start`, `ts_end`, `high`, `low` store CANONICAL
(unshifted) values. The `extend` shift (Policy Panel, `box-standard.md §7.2`)
is applied only at render time by the overlay and does NOT modify
persisted data.

### 9.2 SQLite

```sql
CREATE TABLE market_boxes (
  address         TEXT PRIMARY KEY,
  symbol          TEXT NOT NULL,
  m_scale         INTEGER NOT NULL,
  pattern         TEXT NOT NULL,
  zone            TEXT NOT NULL,
  scale           INTEGER NOT NULL,

  x               INTEGER NOT NULL,
  y_abs           INTEGER NOT NULL,
  z               INTEGER NOT NULL,
  y_band          INTEGER NOT NULL,

  chain_id        INTEGER,               -- NULL if unassigned
  chain_pos       INTEGER,               -- NULL if unassigned

  ts_start        INTEGER NOT NULL,
  ts_end          INTEGER NOT NULL,
  high            REAL NOT NULL,
  low             REAL NOT NULL,
  origin_price    REAL NOT NULL,
  leg1_magnitude  REAL NOT NULL,
  leg2_magnitude  REAL NOT NULL,
  parent_address  TEXT,
  computed_at     INTEGER NOT NULL
);

CREATE INDEX idx_boxes_xyz   ON market_boxes(symbol, m_scale, x, y_band, z);
CREATE INDEX idx_boxes_chain ON market_boxes(symbol, m_scale, chain_id);
```

---

## 10. Chain Assignment Algorithm

```
Input:  Vec<Box> with (x, y_band, z, high, low, pattern) computed
Output: Vec<Box> with (chain_id, chain_pos) filled

size_tol = 0.20   (±20% base-height tolerance)

1. Group boxes by (symbol, m_scale, y_band) → streams

2. For each stream:
     a. Sort boxes by x ascending.
        Ties broken by (pattern ASC, ts_end ASC).

     b. Initialize an empty current_chain buffer.

     c. For each box b in sorted order:
          if buffer is empty:
              buffer.push(b)
              continue

          prev = buffer.last()

          alt   = (prev.pattern ≠ b.pattern)
          inval = wick_engulf(prev, b)      // see §11
          h1    = prev.high − prev.low
          h2    = b.high    − b.low
          max_h = max(h1, h2)
          size_ok = (max_h > 0) AND (|h1 − h2| / max_h ≤ size_tol)

          if alt AND inval AND size_ok:
              buffer.push(b)
          else:
              flush(buffer)                 // assign chain_id/chain_pos
              buffer = [b]

     d. flush(buffer) at end of stream

3. flush(buffer):
     if buffer.len < min_chain_len (default 2):
         return                            // buffer discarded; boxes stay unassigned
     chain_id = buffer[0].x
     for i, b in buffer:
         b.chain_id  = chain_id
         b.chain_pos = i
```

**Determinism:** ties on `x` broken by `(pattern ASC, ts_end ASC)`.

**Notes:**

- `min_chain_len` default = 2.
- Boxes that fail all conditions remain with `chain_id = NULL`.
- The condition does NOT include a time-gap term (removed in v2.1.0).

---

## 11. Engulf (Revisited)

In v2.1.0, engulf is wick-based and coordinate-agnostic:

```
wick_engulf(prev, curr) =
    (prev.pattern == RBR  AND curr.pattern == DBD  AND curr.low  < prev.low)
 OR (prev.pattern == DBD  AND curr.pattern == RBR  AND curr.high > prev.high)
```

Only wicks matter. `open` and `close` play no role.

**No band-distance requirement** (the earlier `±1 y_band` clause is
removed). Two boxes can belong to the same stream (same y_band) and
still form a valid engulf pair.

---

## 12. Rendering Implication

Boxes in the SAME `(y_band)` stream can be drawn on the SAME horizontal
line `y = y_band × BAND_GRID + offset` — creating the "stream"
visualization.

Chain members (same `chain_id`) share:

- The same y-level line (they are in the same y_band)
- Temporal adjacency (sorted by x)

This is the foundation for MKT-6b-5.x (chain-aware rendering).

---

## 13. Determinism & Reproducibility

Given:
- Same candle data
- Same detection parameters
- Same BAND_GRID
- Same `size_tol`

Then:
- x, y_abs, z, y_band are deterministic
- chain_id, chain_pos are deterministic
- address is deterministic (5-tuple + chain suffix)

No randomness. No floating-point fragility for `x` and `y_abs`
(both are integers). The `size_ok` comparison uses a fixed
tolerance and a fixed denominator (`max(h1, h2)`), so it is
stable under re-runs.

---

## 14. Change Log

| Version | Date | Changes |
|---|---|---|
| 1.0.0 | 2026-09-29 | Initial chain-based coordinates (relative to ref_ts, per-symbol base_price) |
| 2.0.0 | 2026-09-29 | **Universal absolute coordinates**: Unix-epoch x, log₂ universal y, no per-symbol constants, universal bands |
| **2.1.0** | **2026-09-30** | **Chain rules rewritten.** Stream key = `(symbol, M, y_band)` — dropped `z`. Chain condition = `alternating pattern + wick engulf + size ±20%` — dropped time-gap. Aligns with `box-standard.md v1.4.0` (base-only box extent). `x` now derived from base start (H1 for RBR, L1 for DBD). Engulf check no longer requires adjacent y_band. `chain_id` and `chain_pos` are now `Option<>` (NULL when unassigned). |
| 3.0.0 | 2026-10-01 | **Universal Y-axis (2025 origin).** `y_abs = floor(log2(price / price_ref) * 10^6)` where `price_ref` = OPEN of first candle ≥ 2025-01-15 00:00 UTC. New `symbol_origin` table caches `price_ref` per (symbol, M). Removed `y_level` and `band_size`. `y_band` uses signed floor (`div_euclid`). `market_boxes` wiped once (addresses changed). |

---

**End of Box Coordinate System v3.0.0**

---
