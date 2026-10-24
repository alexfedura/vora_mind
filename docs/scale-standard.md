# Vora Mind — M-Scale Standard

**Version:** 1.0.0
**Status:** Active
**Effective From:** v0.61.0 (MKT-6a)

---

## 1. Definition

**M-Scale** is a mathematical coordinate system for time-based data, based on the SI unit of time (the second).

It **replaces the traditional timeframe concept** (1m, 5m, 1h, 1d) with a continuous, infinite axis.

### Core Formula

```
period(M) = 10^M  seconds

M ∈ ℤ  (any integer, positive, zero, or negative)
```

### Origin

```
M0 = 1 second
```

M0 is the **fundamental reference**. All other M-levels are defined relative to M0 by powers of 10.

---

## 2. Reference Table

| M | Period | Human Label |
|---|---|---|
| M-3 | 0.001 s | 1 millisecond |
| M-2 | 0.01 s | 10 milliseconds |
| M-1 | 0.1 s | 100 milliseconds |
| **M0** | **1 s** | **1 second (ORIGIN)** |
| M1 | 10 s | 10 seconds |
| M2 | 100 s | 1.67 minutes |
| M3 | 1,000 s | 16.67 minutes |
| M4 | 10,000 s | 2.78 hours |
| M5 | 100,000 s | 27.78 hours |
| M6 | 1,000,000 s | 11.57 days |
| M7 | 10^7 s | 115.7 days |
| M8 | 10^8 s | 3.17 years |
| M9 | 10^9 s | 31.7 years |

### Practical Range

- **Minimum:** M-3 (1 millisecond) — physical limit of tick data
- **Maximum:** M9 (31.7 years) — practical limit for human analysis
- **Outside range:** warning "out of data range"

Range can be extended if data permits (e.g. M-4 for nano-second precision in the future, or M10 for century-scale analysis).

---

## 3. Navigation

### Keyboard

| Key | Action |
|---|---|
| `ArrowUp` | M → M+1 (coarser aggregation) |
| `ArrowDown` | M → M-1 (finer aggregation) |

**No reset. No boundaries within [M-3, M9].**

Example: M3 → M4 → M5 → ... → M9 (max)
         M3 → M2 → M1 → M0 → M-1 → M-2 → M-3 (min)

### Mouse Wheel

**Visual zoom only.** Does NOT change M-level or data.

- Wheel up: zoom in (fewer candles visible)
- Wheel down: zoom out (more candles visible)
- Used for precision drawing

### Drag

Pan horizontally. M-level stays constant. Data stays constant. Only the visible window changes.

---

## 4. Chart Template

Every chart, at every M-level, follows a **fixed template**:

```typescript
interface ChartTemplate {
     candle_count: 180,
     candle_width: 5,
  price_range_pct: 2,
}
```

**Implication:**
- Changing M → changes the DATA (re-aggregation)
- Changing M → does NOT change the VISUAL STRUCTURE
- Default to 180 visible candles with TradingView-style width and consistent vertical range

### Why 300?

- Fits comfortably on a 1920px screen with margins
- Enough context for pattern recognition
- Not too many to overwhelm

### Why Fixed Vertical Range?

- Makes assets comparable: SHIB and BTC look the same at same M
- Eliminates auto-scale jitter
- Enables visual comparison across charts

---

## 5. Three Independent Layers

The chart system has three orthogonal layers:

### Layer 1: TEMPLATE (structural)
- 180 candles
- 5-pixel default body width
- Fixed vertical range
- **Immutable, applies everywhere**

### Layer 2: SCALE (semantic)
- M-level (i32)
- Determines aggregation period
- **Changes data, keeps template**
- Controls density of information

### Layer 3: FREE ZOOM (visual)
- Mouse wheel only
- Temporarily adjusts visible range
- **Does not affect M, template, or data**
- Used for precise drawing

**These three are fully orthogonal.** Changing one does not affect the others.

---

## 6. Comparison with Traditional Timeframes

### Old System (TF)
```
7 fixed timeframes: 1m, 5m, 15m, 30m, 1h, 4h, 1d
- Non-mathematical (human conventions)
- Gaps between levels (1m → 5m is 5×; 1h → 4h is 4×)
- Cannot go beyond 1d without new TF
- Different feel per TF
- Zoom is uncontrolled
```

### New System (M-Scale)
```
Infinite continuous M-axis
- Mathematical (SI-based)
- Uniform: 10× between levels
- Extends from milliseconds to centuries
- Same visual feel at all levels
- Zoom is locked to template
```

### Mapping (for migration reference)

| Old TF | Period (s) | Closest M |
|---|---|---|
| 1m | 60 | M1.78 |
| 5m | 300 | M2.48 |
| 15m | 900 | M2.95 |
| 30m | 1800 | M3.26 |
| 1h | 3600 | M3.56 |
| 4h | 14400 | M4.16 |
| 1d | 86400 | M4.94 |

**Note:** TF does NOT map cleanly. This is why TF must be removed entirely in MKT-6c and MKT-9.

---

## 7. Examples

### Example 1: XAUUSD (Gold)

```
M3 → 1000-second candles (16.67 min)
     January 2025: ~2,678 candles total
     Visible: 300 (last 83 hours)

M4 → 10,000-second candles (2.78 hours)
     January 2025: ~268 candles total
     Visible: 268 (all of January)

M5 → 100,000-second candles (27.78 hours)
     January 2025: ~27 candles total
     Visible: 27 (whole month)

M6 → 1,000,000-second candles (11.57 days)
     January 2025: ~3 candles total
     Visible: 3
```

### Example 2: BTC vs SHIB

Both at M4 (2.78-hour candles):

| Asset | Price | ATR(M4) | Visible Range |
|---|---|---|---|
| BTC | $80,000 | ~$1,500 | ±2% = ±$1,600 |
| SHIB | $0.000012 | ~$0.0000002 | ±2% = ±$0.00000024 |

**Both show identical visual structure:** 180 candles, ±2% vertical range.

---

## 8. Data Layer Architecture

Chart price history is fetched directly from Dukascopy through the Tauri command
`mkt_fetch_historical_duka`. The Rust process keeps a temporary in-memory cache
for paging; it does not read local files or SQLite candles for chart history.
SQLite candle rows remain available to existing M-Scale/box analysis workflows,
but the local Parquet ingest CLI and dataset are removed.

---

## 9. Implementation Notes
Market history paging uses `mkt_fetch_historical_duka(symbol, timeframe,
end_timestamp, limit)`. Provider errors are returned to the chart; they never
trigger a local file or SQLite-history fallback. The `market_sources` registry
is dropped when an existing database is opened.

---

## 10. Future Extensions

### ATR-based Vertical Range (MKT-7)
Replace fixed ±2% with dynamic ±MY × ATR:
- M0-M1: MY = 5.0
- M2-M3: MY = 3.0
- M4-M5: MY = 2.0
- M6+: MY = 1.5

This keeps visual consistency even during high-volatility periods.

### Logarithmic Y-axis (future)
For assets with exponential growth (BTC), log scale becomes preferable. M-Scale works with both linear and log Y-axes.

### Sub-second M-levels (MKT-10+)
M-4 (100μs), M-5 (10μs), M-6 (1μs) — for HFT research.

### Extended M-levels
M10 (317 years), M11 (3,170 years) — for long historical analysis.

---

## 11. Why This Standard Matters

### Mathematical Purity
Based on SI unit (second), not human convention.

### Universal
Works for any time-series: stocks, crypto, forex, futures, physics data.

### Comparable
Same M → same visual experience across all assets.

### Scalable
Infinite axis, from nanoseconds to millennia.

### Separation of Concerns
- Template (visual structure)
- Scale (semantic density)
- Zoom (temporary visual adjustment)

Three orthogonal layers.

---

## 12. S-Scale (Skill Scale) — Link to M-Scale

Every M-Scale displays exactly **3 skills** (S-levels).

### 12.1 Formula

```
percent(S) = 1% × 1.25^S

S0  = 1.000%
S1  = 1.250%
S2  = 1.5625%
S3  = 1.953%
S4  = 2.441%
S5  = 3.052%
S6  = 3.815%
S7  = 4.768%
S8  = 5.960%
S9  = 7.451%
S10 = 9.313%
S15 = 28.42%
S20 = 86.74%
```

### 12.2 Link Table

| M | Skills Displayed |
|---|---|
| M-3 | S-9, S-8, S-7 |
| M-2 | S-6, S-5, S-4 |
| M-1 | S-3, S-2, S-1 |
| **M0** | **S0, S1, S2** |
| M1 | S3, S4, S5 |
| M2 | S6, S7, S8 |
| M3 | S9, S10, S11 |
| M4 | S12, S13, S14 |
| M5 | S15, S16, S17 |
| M6 | S18, S19, S20 |
| M7 | S21, S22, S23 |
| M8 | S24, S25, S26 |
| M9 | S27, S28, S29 |

**Formula:**
```
S_start(M) = 3 × M
S_end(M)   = 3 × M + 2
```

### 12.3 Consequence

Changing M changes BOTH:
- The box detection threshold (recompute at new skill levels)
- The data displayed (re-aggregation at new period)

**Boxes are recomputed for each M-level.** They are NOT visually scaled across M-levels.

### 12.4 Relation to Boxes

S-Scale defines the **height** of a box. Combined with M-Scale (which defines the time window), a box has both a precise time span and a precise percentage height.

See `docs/box-standard.md` for full box specification (RBR / DBD patterns, chain rules, engulf mechanics).

---

## 13. Summary

| Axis | Unit | Origin | Range | Purpose |
|---|---|---|---|---|
| **M-Scale** | Time | M0 = 1 second | M-3 to M9 | Aggregation period |
| **S-Scale** | Price | S0 = 1% | S-9 to S29 | Box height |

**Together:** Every chart state = `(M, S_range)` where `S_range = [3M, 3M+2]`.

**Three orthogonal layers:**
1. Template — visual structure (180 candles, locked zoom)
2. M-Scale — time density
3. S-Scale — price density

---

**End of M-Scale Standard v1.0.0**

Next: see `docs/box-standard.md` for box mechanics.
Next: see `docs/mkt-chunks.md` for implementation phases.