```markdown
# Vora Mind — Box Detection Standard

**Version:** 1.5.2
**Status:** ACTIVE
**Effective From:** 2026-09-30
**Owner:** Alex Fedura

---

## 1. Overview

This document defines the **Box Detection System** for Vora Mind — an algorithm that automatically identifies **Rally-Base-Rally (Demand)** and **Drop-Base-Drop (Supply)** patterns on tick-aggregated candles, classifies them by importance (**S-Scale**), gives each box a **unique virtual address**, and renders them as hollow rectangles in a **chain layout** (overlapping).

**Goals:**
- Strict 4-pivot pattern detection (RBR / DBD)
- Wick-based invalidation (tick-level precision)
- Mathematical rigor (reproducible, testable)
- Multi-timeframe support (M0–M9)
- Every box has an **immutable address** (searchable, reproducible)
- Empirical tuning (policy configurable at runtime)

**Non-goals (v1):**
- Cross-timeframe merging
- Predictive signals (only visualization)
- Order-flow analysis

---

## 2. Pattern Definitions

A box is defined by **three legs** and **four pivots**:

- **Leg 1** — directional move from the first pivot to the second
- **Leg 2 (Base)** — retracement from the second pivot to the third
- **Leg 3** — directional move from the third pivot to the fourth

### 2.1 Notation

Let a candle be `c[i] = { ts, open, high, low, close }` for `i ∈ [0, N)`.

Let `ref_price` be the first pivot price (`L1` for RBR, `H1` for DBD).

Let `M` be the M-scale of the current chart (0–9), where `period(M) = 10^M` seconds.

**Pivot detection** uses `high` and `low` **only** (never `open` or `close`):

- **Pivot-High at `i`**: `high[i]` is the strict maximum of `high` within `[i-K, i+K]`
- **Pivot-Low at `i`**: `low[i]` is the strict minimum of `low` within `[i-K, i+K]`

Where `K` is the pivot lookback window (default: `2`).

### 2.2 The Two Primary Patterns

#### RBR — Rally-Base-Rally (Demand)

Four pivots in strict chronological order: **L1 → H1 → HL → HH**

| Pivot | Role | Definition |
|---|---|---|
| **L1** | Leg 1 low | The starting pivot-low |
| **H1** | Leg 1 high | The pivot-high ending Leg 1 |
| **HL** | Base low (Higher Low) | The pivot-low ending the base |
| **HH** | Leg 3 high (Higher High) | The pivot-high ending Leg 3 |

**Strict mathematical conditions:**

```
(A)  HL > L1    — Higher Low  (a single pip at or below L1 → invalid)
(B)  HH > H1    — Higher High (a single pip at or below H1 → invalid)
```

Where "pip" is the smallest tick increment of the symbol. Invalidation is
**wick-based**: any candle whose `low` reaches or breaks `L1` fails condition (A);
any candle whose `high` fails to reach `H1` fails condition (B).

**Leg magnitudes:**

```
Leg 1 (Rally 1):  H1 - L1
Leg 2 (Base):     H1 - HL       ← BASE
Leg 3 (Rally 2):  HH - HL
```

**Base Range (used for BOTH box extent AND S-Scale):**

```
base_height = H1 - HL
base_top    = H1
base_bottom = HL
```

#### DBD — Drop-Base-Drop (Supply)

Four pivots in strict chronological order: **H1 → L1 → LH → LL**

| Pivot | Role | Definition |
|---|---|---|
| **H1** | Leg 1 high | The starting pivot-high |
| **L1** | Leg 1 low | The pivot-low ending Leg 1 |
| **LH** | Base high (Lower High) | The pivot-high ending the base |
| **LL** | Leg 3 low (Lower Low) | The pivot-low ending Leg 3 |

**Strict mathematical conditions:**

```
(A)  LH < H1    — Lower High (a single pip at or above H1 → invalid)
(B)  LL < L1    — Lower Low  (a single pip at or above L1 → invalid)
```

Invalidation is **wick-based**: any candle whose `high` reaches or breaks `H1`
fails condition (A); any candle whose `low` fails to reach `L1` fails condition (B).

**Leg magnitudes:**

```
Leg 1 (Drop 1): H1 - L1
Leg 2 (Base):   LH - L1       ← BASE
Leg 3 (Drop 2): LH - LL
```

**Base Range (used for BOTH box extent AND S-Scale):**

```
base_height = LH - L1
base_top    = LH
base_bottom = L1
```

### 2.3 Engulf (Invalidation Trigger)

The box remains valid until the pattern's **first pivot** is pierced by a wick:

- **RBR invalidated** when any candle's `low < L1`
- **DBD invalidated** when any candle's `high > H1`

The candle that triggers invalidation is called the **engulf candle**.

```
engulf_rbr_index = first j > HL_idx where candles[j].low  < L1
engulf_dbd_index = first j > LH_idx where candles[j].high > H1
```

`open` and `close` play **no role** in engulf detection. Only wicks matter.

**Note:** The engulf candle is used to determine box VALIDITY (whether the
pattern still holds), but it does NOT extend the box rectangle. The box
renders only the base segment (see §7.2).

### 2.4 Detection Algorithm (RBR)

```
1. Scan all i in [0, N):
     if not pivot_low(i): continue
     L1_idx = i
     L1     = candles[i].low

2. From L1_idx, scan forward for the next pivot-high:
     H1_idx = first j > L1_idx where pivot_high(j)
     if H1_idx is None: continue
     H1 = candles[H1_idx].high

3. From H1_idx, scan forward for the next pivot-low:
     HL_idx = first k > H1_idx where pivot_low(k)
     if HL_idx is None: continue
     HL = candles[HL_idx].low

4. Strict condition (A): HL > L1
     if not: continue

5. From HL_idx, scan forward for the next pivot-high:
     HH_idx = first m > HL_idx where pivot_high(m)
     if HH_idx is None: continue
     HH = candles[HH_idx].high

6. Strict condition (B): HH > H1
     if not: continue

7. Scan forward from HH_idx for engulf:
     engulf_idx = first p > HH_idx where candles[p].low < L1
     if engulf_idx is None: continue

8. Emit RBR box.
```

**DBD** is the exact mirror (swap HIGH ↔ LOW and flip inequalities).

### 2.5 Four-Pattern Enum (RBD / DBR reserved)

| Pattern | Pivot sequence | Zone |
|---|---|---|
| **RBR** | L1 → H1 → HL → HH | Demand |
| **DBD** | H1 → L1 → LH → LL | Supply |
| **RBD** | L1 → H1 → HL → LL | Supply *(reserved)* |
| **DBR** | H1 → L1 → LH → HH | Demand *(reserved)* |

Only **RBR** and **DBD** are implemented. RBD and DBR are reserved.

---

## 3. Virtual Y-Axis

### 3.1 Rationale

Because time is linear, two boxes with the **same S-Scale at the exact same
timestamp** cannot both exist in a 2D chart. To give every box a **unique,
reproducible label**, we introduce a **virtual Y-axis** that quantizes price
into discrete bands.

### 3.2 Band Size

```
band_size = ref_price × 0.001
```

Where `ref_price` is the first pivot price (`L1` for RBR, `H1` for DBD).
This equals 0.1% of the reference price — the S0 unit (see §5).

### 3.3 Base Price

```
base_price = per-symbol constant (configurable)
```

| Symbol | base_price |
|---|---|
| XAUUSD | 2000.0 |
| BTCUSD | 10000.0 |
| EURUSD | 1.0 |

Stored in SQLite table `market_symbol_config(symbol, base_price, updated_at)`.

### 3.4 Y-Level Computation

```
box_center = (box.high + box.low) / 2.0
y_level    = floor((box_center - base_price) / band_size)
```

`y_level` is a signed integer.

**Example (XAUUSD, M3, box_center = 2650, ref_price = 2650):**
```
band_size = 2650 × 0.001 = 2.65
y_level = floor((2650 - 2000) / 2.65) = 245
```

### 3.5 Address Format

```
{symbol}~M{m}~X{x_idx}~Y{y_level}~{pattern}
```

Where:

- `symbol` — e.g., `XAUUSD`
- `m` — M-scale (0–9)
- `x_idx = floor(ts_start / period(M))` — ts_start is the base start (H1 for RBR, L1 for DBD)
- `y_level` — signed integer
- `pattern` — `RBR` | `DBD` | `RBD` | `DBR`

**Examples:**

```
XAUUSD~M3~X1741242~Y150~RBR
XAUUSD~M3~X1741242~Y151~DBD
XAUUSD~M5~X348248~Y62~RBR
```

**Guarantees:** unique, stable, searchable, human-readable.

---

## 4. Parameter Specification

| Parameter | Symbol | Default | Unit | Meaning |
|---|---|---|---|---|
| Pivot lookback | `K` | **2** | candles | Window on each side for pivot detection |
| Min Leg 1 (%) | `min_leg1_pct` | **0.5** | % | Minimum `H1 - L1` / ref_price |
| Min Leg 3 (%) | `min_leg3_pct` | **0.5** | % | Minimum `HH - HL` / ref_price |
| Min base candles | `min_base` | **2** | count | Fewest candles between H1 and HL |
| Max base candles | `max_base` | **12** | count | Most candles between H1 and HL |
| Max base (%) | `max_base_pct` | **1.0** | % | Max base_height / ref_price |
| Box extend | `extend` | **0** | candles | Shift both X-edges by N candles. Range: -20..+20 |

**Notes:**

- All percentage thresholds are relative to the first pivot price
  (`L1` for RBR, `H1` for DBD), matching the S-Scale reference (see §5).
- Per-M overrides stored in `m_scale_policies`.
- `extend` is exposed live in the Policy Panel and applies to the
  overlay only (no re-detection required).

---

## 5. S-Scale Classification

S-Scale measures the **base height** relative to the **first pivot price**.

### 5.1 Formula

**RBR (base at H1):**
$$\text{pct} = \frac{H1 - HL}{H1} \times 100$$

**DBD (base at L1):**
$$\text{pct} = \frac{LH - L1}{L1} \times 100$$

**Level:**
$$S = \left\lfloor \frac{\ln(\text{pct} / 0.1)}{\ln(1.25)} \right\rfloor, \quad S \in [0, 9]$$

### 5.2 Lookup Table

| S | Range (% of price) |
|----|--------------------|
| S0 | [0.100, 0.125)     |
| S1 | [0.125, 0.156)     |
| S2 | [0.156, 0.195)     |
| S3 | [0.195, 0.244)     |
| S4 | [0.244, 0.305)     |
| S5 | [0.305, 0.382)     |
| S6 | [0.382, 0.477)     |
| S7 | [0.477, 0.596)     |
| S8 | [0.596, 0.746)     |
| S9 | [0.746, ∞)         |

**Determinism:** same input → same S.

### 5.3 Why Base Height (Not Full Pattern)

The S-Scale is measured on the **base** (the retracement) so that two
patterns with different leg sizes can be compared by the size of their
consolidation zone. Normalizing by the first pivot price makes the metric
scale-invariant and directly comparable across symbols and M-scales.

**Consistency:** §2.2 defines `base_height = H1 - HL` (RBR) or `LH - L1`
(DBD). §7.2 sets the box's vertical extent to the SAME range. This
guarantees that what you see (box height) matches what is classified
(S-scale).

---

## 6. Three-Color Palette

Regardless of M-scale, **each chart displays exactly three S-levels**: one
**large**, one **medium**, one **small**.

### 6.1 Color Assignment

| Role | Hex | Name |
|---|---|---|
| **S_large** (biggest) | `#9c27b0` | Purple |
| **S_medium** (middle) | `#ff9800` | Orange |
| **S_small** (smallest) | `#4caf50` | Green |

These three colors are **fixed across all M-scales**.

### 6.2 Why Three Colors

- Cognitive load — humans distinguish 3 colors instantly
- Hierarchy — big/medium/small maps to purple/orange/green
- Universal — the same three colors appear on every M

---

## 7. Box Rendering

### 7.1 Visual Specification

| Property | Value | Notes |
|---|---|---|
| **Border width** | **2 px (uniform)** | All S-levels use the same width |
| **Fill** | **None (transparent)** | Never draw a background |
| **Line style** | Solid | No dashes |
| **Border color** | Per §6 (3-color palette) | Only the color changes |
| **Z-order** | Above candles, above grid | Boxes must stay visible |
| **Layout** | Chain (overlapping) | Adjacent boxes touch or overlap by `extend` |

### 7.2 Box Extent

The box spans horizontally from base start to
base end:
  RBR: ts[H1_idx] → ts[HL_idx]
  DBD: ts[L1_idx] → ts[LH_idx]
This matches §2.2 (base definition). Vertically it spans ONLY
the base segment (Leg 2).

**RBR:**

```
high     = H1                         (top of base)
low      = HL                         (bottom of base)
ts_start = ts[H1_idx]                 (start of base)
ts_end   = ts[HL_idx]                 (end of base)
```

**DBD:**

```
high     = LH                         (top of base)
low      = L1                         (bottom of base)
ts_start = ts[L1_idx]                 (start of base)
ts_end   = ts[LH_idx]                 (end of base)
```

Where:
- `P = period(M)` (seconds per candle at current M-scale)
- `extend` is a configurable candle offset (see §4)
  - Default: `0`
  - Range: `-20 .. +20`
  - Positive → shifts both edges FORWARD (right) by N candles
  - Negative → shifts both edges BACKWARD (left) by N candles
  - The width (`ts_end - ts_start`) is preserved; only position shifts.

**Invariants (must hold for every emitted box):**

```
high > low
ts_start < ts_end
(high - low) == base_height       (§2.2)
```

**Notes:**

- `H1_idx`, `HL_idx`, `L1_idx`, `LH_idx` are candle indices
  of the respective pivots (see §2.2, §2.4). `engulf_idx` (§2.4
  step 7) is a validity gate only — it does NOT set box extent.
- The box spans horizontally from base start to base end:
    RBR: ts[H1_idx] → ts[HL_idx]
    DBD: ts[L1_idx] → ts[LH_idx]
  This matches §2.2 (base definition).
- The `extend` shift is render-only (BoxOverlay, §7.3).
- The `extend` shift is a RENDERING-ONLY concern — it does not affect
  the underlying `Box` data model, address, or chain assignment.
- This definition is CONSISTENT with §2.2 (base_height) and §5
  (S-Scale uses the same segment).

### 7.3 Chain Effect

Consecutive boxes in a chain are visually adjacent. With `extend = 0`
(default), box `n` and box `n+1` touch edge-to-edge at base boundaries.

- `extend > 0` shifts all boxes forward in time, creating a visual
  gap between the last box and current price.
- `extend < 0` shifts all boxes backward, increasing overlap between
  chain members.

The `extend` parameter is exposed in the Policy Panel and applies
live without re-running detection.

### 7.4 Rendering Pseudocode

```
FOR box IN visible_boxes:
  draw_rectangle(
    x1        = timeToCoordinate(box.ts_start),
    x2        = timeToCoordinate(box.ts_end),
    y1        = priceToCoordinate(box.high),
    y2        = priceToCoordinate(box.low),
    color     = palette_color(box.role),
    lineWidth = 2,
    fill      = transparent
  )
```

The overlay reads `box.ts_start`, `box.ts_end`, `box.high`, `box.low`
directly — no padding, no margin, no additional offset.

### 7.5 Interaction (v1: minimal)

| Action | Behavior |
|---|---|
| Hover on box | Tooltip: `RBR · S5 · Y150 · 2612.40–2624.60` |
| Click on box | Toggle highlight (thicker border) |
| Drag | Read-only |

---

## 8. Overlap & Nesting

### 8.1 Overlap Computation

For two boxes `A`, `B`:

```
overlap_height = max(0, min(A.high, B.high) - max(A.low, B.low))
min_height     = min(A.high - A.low, B.high - B.low)
overlap_ratio  = overlap_height / min_height
```

### 8.2 Resolution Rules

| Overlap ratio | Action |
|---|---|
| **≥ 0.80** | Merge → keep higher S |
| **[0.30, 0.80)** | Keep both (partial overlap, chain) |
| **(0, 0.30)** | Keep both (independent) |
| **= 0** | Keep both (disjoint) |

### 8.3 Nesting

If `A.low ≤ B.low` and `A.high ≥ B.high` and `A ≠ B`:

```
→ Keep both
→ B.parent_address = A.address
→ A.children_addresses.push(B.address)
→ Render B with 2px border, own role color
```

### 8.4 Merge Rules

When merging two boxes with `overlap_ratio ≥ 0.80`:

- Keep the box with **higher S**
- Tie → keep **larger time span**
- Tie → keep **earlier** (lower `ts_start`)

---

## 9. Visibility Policy

### 9.1 Default Policy

| M | S_small | S_medium | S_large | Max boxes |
|---|---|---|---|---|
| M0 | S0 | S1 | S2 | 20 |
| M1 | S1 | S2 | S3 | 20 |
| M2 | S2 | S3 | S4 | 20 |
| M3 | S3 | S4 | S5 | 20 |
| M4 | S4 | S5 | S6 | 20 |
| M5 | S5 | S6 | S7 | 20 |
| M6 | S6 | S7 | S8 | 20 |
| M7 | S7 | S8 | S9 | 15 |
| M8 | S7 | S8 | S9 | 15 |
| M9 | S7 | S8 | S9 | 10 |

### 9.2 Policy Data Model

```
MScalePolicy {
  m_scale: u8,
  s_small: u8,
  s_medium: u8,
  s_large: u8,
  max_boxes: u16,
  parameters: Parameters,
  base_price: f64,
  extend: i32,          // NEW in v1.4.0 — candle shift for X-edges
}
```

### 9.3 Filtering Logic

```
fn filter_by_policy(boxes, policy):
  filtered = boxes.filter(|b| b.scale in {policy.s_small, policy.s_medium, policy.s_large})
  filtered.sort_by(|a, b| b.scale.cmp(a.scale))
  filtered.truncate(policy.max_boxes)
  return filtered
```

### 9.4 Empirically Tunable

Policy Panel exposes:

- S-levels for small/medium/large per M
- `max_boxes`
- RBR/DBD parameters (§4)
- `extend` (range -20..+20, default 0)
- `base_price` per symbol
- Save / reset to default

---

## 10. Data Model

### 10.1 Rust Struct

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Box {
    pub address: String,
    pub symbol: String,
    pub m_scale: u8,

    pub pattern: BoxPattern,
    pub zone: BoxZone,
    pub scale: u8,

    pub x_idx: i64,
    pub y_level: i32,
    pub band_size: f64,

    pub ts_start: i64,      // = ts[H1_idx] (RBR) / ts[L1_idx] (DBD), before extend
    pub ts_end: i64,        // = ts[HL_idx] (RBR) / ts[LH_idx] (DBD), before extend
    pub high: f64,          // = H1 (RBR) / LH (DBD)
    pub low: f64,           // = HL (RBR) / L1 (DBD)
    pub origin_price: f64,

    pub leg1_magnitude: f64,
    pub leg2_magnitude: f64,
    pub parent_address: Option<String>,
    pub children_addresses: Vec<String>,
}
```

**Persistence note:** `ts_start`, `ts_end`, `high`, `low` store the
CANONICAL (unshifted) values. The `extend` shift is applied only
at render time by `BoxOverlay`.

### 10.2 SQLite Schema

```sql
CREATE TABLE IF NOT EXISTS market_boxes (
  address         TEXT PRIMARY KEY,
  symbol          TEXT NOT NULL,
  m_scale         INTEGER NOT NULL,
  pattern         TEXT NOT NULL,
  zone            TEXT NOT NULL,
  scale           INTEGER NOT NULL,
  x_idx           INTEGER NOT NULL,
  y_level         INTEGER NOT NULL,
  band_size       REAL NOT NULL,
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

CREATE INDEX idx_boxes_lookup
  ON market_boxes(symbol, m_scale, scale);

CREATE INDEX idx_boxes_xy
  ON market_boxes(symbol, m_scale, x_idx, y_level);

CREATE TABLE IF NOT EXISTS m_scale_policies (
  m_scale          INTEGER PRIMARY KEY,
  s_small          INTEGER NOT NULL,
  s_medium         INTEGER NOT NULL,
  s_large          INTEGER NOT NULL,
  max_boxes        INTEGER NOT NULL,
  pivot_k          INTEGER NOT NULL DEFAULT 2,
  min_leg1_pct     REAL NOT NULL,
  min_leg3_pct     REAL NOT NULL,
  min_base         INTEGER NOT NULL,
  max_base         INTEGER NOT NULL,
  max_base_pct     REAL NOT NULL,
  extend           INTEGER NOT NULL DEFAULT 0,
  updated_at       INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS market_symbol_config (
  symbol      TEXT PRIMARY KEY,
  base_price  REAL NOT NULL,
  updated_at  INTEGER NOT NULL
);
```

### 10.3 IPC Commands

```
mkt_detect_boxes(symbol, m_scale)        -> Vec<BoxDto>
mkt_get_policy(m_scale)                  -> PolicyDto
mkt_set_policy(policy: PolicyDto)        -> ()
mkt_clear_boxes(symbol, m_scale)         -> u64
mkt_get_box_by_address(address)          -> Option<BoxDto>
mkt_get_symbol_config(symbol)            -> SymbolConfigDto
mkt_set_symbol_config(cfg)               -> ()
```

---

## 11. Testing Strategy

### 11.1 Unit Tests (Rust)

| Test | Input | Expected |
|---|---|---|
| `rbr_positive` | L1=100, H1=150, HL=120, HH=180 | 1 box, zone=Demand |
| `rbr_invalid_hl_below_l1` | HL=99 (< L1) | 0 boxes |
| `rbr_invalid_hh_below_h1` | HH=145 (< H1) | 0 boxes |
| `rbr_engulf_triggers` | Candle with `low < L1` after HH | engulf found |
| `dbd_positive` | H1=150, L1=100, LH=125, LL=80 | 1 box, zone=Supply |
| `dbd_invalid_lh_above_h1` | LH=151 (> H1) | 0 boxes |
| `dbd_invalid_ll_above_l1` | LL=101 (> L1) | 0 boxes |
| `dbd_engulf_triggers` | Candle with `high > H1` after LL | engulf found |
| `pivot_detection_high` | Known sequence | Correct pivots |
| `pivot_detection_low` | Known sequence | Correct pivots |
| `scale_rbr_formula` | H1=150, HL=120 | pct=20% → clamp S to 9 |
| `scale_dbd_formula` | L1=100, LH=125 | pct=25% → clamp S to 9 |
| `address_format_correct` | Known box | `XAUUSD~M3~X...~Y...~RBR` |
| `box_high_low_rbr` | RBR-positive | high=H1, low=HL |
| `box_high_low_dbd` | DBD-positive | high=LH, low=L1 |
| `box_ts_rbr_exact` | RBR-positive, extend=0 | ts_start=H1_ts, ts_end=HL_ts |
| `box_ts_dbd_exact` | DBD-positive, extend=0 | ts_start=L1_ts, ts_end=LH_ts |
| `extend_positive` | extend=+2 | ts_start = H1_ts + 2*P |
| `extend_negative` | extend=-2 | ts_start = H1_ts - 2*P |
| `extend_preserves_width` | extend=±5 | (ts_end - ts_start) unchanged |
| `y_level_computed` | center=2650, base=2000, band=2.65 | y_level = 245 |
| `deterministic_100_runs` | Same input × 100 | Identical output |
| `overlap_merge_80` | 85% overlap | 1 box (higher S) |
| `overlap_keep_30` | 40% overlap | 2 boxes |
| `nesting_detected` | A contains B | parent/child set |
| `role_filter_keeps_three` | M3 filter | Only S3/S4/S5 |

### 11.2 Integration Tests

- Run on 1 month of real XAUUSD M0 data
- Assert: box count ∈ expected range
- Assert: all addresses unique
- Assert: nested boxes have valid parent_address
- Assert: **every box satisfies `high - low == base_height`** (§2.2 invariant)

### 11.3 Empirical Tuning

Field-observation exercise via the Policy Panel: adjust `K`, `min_leg1_pct`,
`max_base_pct`, `extend`, `base_price` in real time and observe the
visualization.

---

## 12. Determinism & Reproducibility

1. Same `candles[]` + `parameters` → bit-identical output.
2. No random seeds, no time-dependent logic.
3. Addresses are deterministic from `(symbol, m_scale, ts_start, y_level, pattern)`.
4. `extend` affects RENDER only — detection output is invariant.
5. Final ordering: `(scale DESC, x_idx ASC, pattern ASC, ts_end ASC)`.

---

## 13. Performance Targets

| Metric | Target |
|---|---|
| Detection (5000 candles, single M) | < 200 ms |
| Detection (5000 candles, all M) | < 1.5 s |
| Address computation | O(1) per box |
| Rendering (20 boxes, 60 FPS) | Smooth |
| DB query by address | < 5 ms |
| DB query by (symbol, m, scale) | < 20 ms |

---

## 14. Open Questions (Deferred)

- Cross-M merging
- Box invalidation tracking
- RBD / DBR pattern implementation
- Box hit-count (how many times price revisits)
- Multi-symbol address semantics

---

## 15. Change Log

| Version | Date | Changes |
|---|---|---|
| 1.0.0 | 2026-09-28 | Initial release |
| 1.1.0 | 2026-09-29 | Extended box geometry |
| 1.2.0 | 2026-09-29 | 3-color palette, chain, Y-axis, address |
| 1.3.0 | 2026-09-29 | Strict 4-pivot RBR/DBD detection. Pivot detection uses `high`/`low` only. Engulf is wick-based (not body). S-Scale from base height relative to first pivot. `open`/`close` play no role. |
| 1.3.1 | 2026-09-29 | S-scale base: 1.0% → 0.1% (FX/Metals calibration) |
| **1.4.0** | **2026-09-30** | **Box extent now consistent with §2.2 base definition.** RBR: `high = H1`, `low = HL`, `ts_start = ts[H1_idx]`, `ts_end = ts[HL_idx]`. DBD: `high = LH`, `low = L1`, `ts_start = ts[L1_idx]`, `ts_end = ts[LH_idx]`. Resolves §7.2 ↔ §2.2 contradiction. Replaced `CHAIN_EXTEND` (default 3, fixed) with `extend` (default 0, range -20..+20, exposed in Policy Panel). `extend` is render-only — canonical Box data unchanged. Added `box_high_low_rbr`, `box_high_low_dbd`, `box_ts_rbr_exact`, `box_ts_dbd_exact`, `extend_positive`, `extend_negative`, `extend_preserves_width` tests. |
| 1.5.0 | 2026-09-30 | **Removed detection volatility measure entirely.** Detection thresholds migrated to percentage-of-reference-price: `min_leg1_atr` → `min_leg1_pct`, `min_leg3_atr` → `min_leg3_pct`, `max_base_atr` → `max_base_pct`. Removed `band_k`; `band_size` is now `ref_price × 0.001` (0.1%, matching S0 unit). Makes detection deterministic, M-independent, and symbol-independent. Consistent with S-Scale. |
| 1.5.1 | 2026-09-30 | Box horizontal extent now spans base start → engulf candle (`ts_end = ts[engulf_idx]`). Previously ended at base end (HL/LH). Matches user intent: box shows the FULL life of the pattern. `extend` is render-only (BoxOverlay), does not modify `ts_start`/`ts_end`. |
| 1.5.2 | 2026-09-30 | Reverted box ts_end to base boundary (ts[HL_idx] / ts[LH_idx]). v5.1.8.2's engulf-based extent was incorrect; it extended the box beyond base. Box now represents ONLY the base segment. |

---

**End of Box Detection Standard v1.5.2**

---
