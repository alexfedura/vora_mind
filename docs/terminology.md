```markdown
# 📖 Vora Mind — Terminology Reference

> **Version:** 2.3.0  
> **Last Updated:** 2026-10-13  
> **Status:** Living document — extend as new terms are introduced.  
>
> **Purpose:** Single source of truth for all domain terminology used across
> backend, frontend, and documentation. Every term has a precise definition,
> an example, and references to code that implements it.

---

## Document Structure

```
Vora Mind Terminology
│
├── Part I — Domain Terms (Trading / Chart)
│   ├── §1–§5    Dimensions, Ranges & Time Frames
│   ├── §6–§7    Scales
│   ├── §8–§10   Bars & Pivots
│   ├── §11–§15  Pivot Labels & Base
│   ├── §16–§17  Patterns (RBR / DBD)
│   ├── §18–§19  Engulf & Pending
│   ├── §20–§22  Box, Duration, Pattern
│   ├── §23–§26  Zone, Chain, Detection
│   ├── §27–§28  Scale List, Address
│   └── §29–§30  Origin (price), Epoch (time)
│
├── Part II — Spatial & Coordinate Concepts
│   ├── §31      Data
│   ├── §32      Data Space
│   ├── §33      Screen Space
│   ├── §34      Region (chained, 2D)
│   ├── §35      Analysis Zone
│   ├── §36      Execution Zone
│   └── §37      Transition
│
├── Part III — Interaction
│   ├── §38      Zoom
│   └── §39      Pan
│
└── Part IV — Physical & System Concepts
    ├── Hardware Layer
    │   ├── §40  Screen
    │   └── §41  Display
    │
    ├── OS Layer
    │   ├── §42  Desktop
    │   └── §43  Taskbar
    │
    └── Application Layer
        ├── §44  Window
        ├── §45  Window State
        ├── §46  Title Bar
        ├── §47  Controls
        ├── §48  Menu Bar
        ├── §49  Toolbar
        ├── §50  Content Area
        ├── §51  Viewport
        ├── §52  Sidebar / Panel
        ├── §53  Status Bar
        ├── §54  Scrollbar
        ├── §55  Focus
        ├── §56  Z-Order
        └── §57  Modal / Dialog
```

---

## Table of Contents

**Part I — Domain Terms (Trading / Chart)**
1. [Dimension](#1-dimension)
2. [Range (Generic)](#2-range-generic)
3. [Time Range](#3-time-range)
4. [Price Range](#4-price-range)
5. [Time Frame](#5-time-frame)
6. [Scale](#6-scale)
7. [scale_sec](#7-scale_sec)
8. [Bar / Candlestick](#8-bar--candlestick)
9. [Aggregated Bar](#9-aggregated-bar)
10. [Pivot](#10-pivot)
11. [H1, L1](#11-h1-l1)
12. [HL, HH](#12-hl-hh)
13. [LH, LL](#13-lh-ll)
14. [P1](#14-p1)
15. [Base](#15-base)
16. [RBR (Rally-Base-Rally)](#16-rbr-rally-base-rally)
17. [DBD (Drop-Base-Drop)](#17-dbd-drop-base-drop)
18. [Engulf](#18-engulf)
19. [Pending](#19-pending)
20. [Box](#20-box)
21. [Duration](#21-duration)
22. [Pattern](#22-pattern)
23. [Zone](#23-zone)
24. [Chain](#24-chain)
25. [Detection Pass](#25-detection-pass)
26. [Multi-Scale Detection](#26-multi-scale-detection)
27. [Scale List](#27-scale-list)
28. [Address](#28-address)
29. [Origin](#29-origin)
30. [Epoch](#30-epoch)

**Part II — Spatial & Coordinate Concepts**
31. [Data](#31-data)
32. [Data Space](#32-data-space)
33. [Screen Space](#33-screen-space)
34. [Region](#34-region)
35. [Analysis Zone](#35-analysis-zone)
36. [Execution Zone](#36-execution-zone)
37. [Transition](#37-transition)

**Part III — Interaction**
38. [Zoom](#38-zoom)
39. [Pan](#39-pan)

**Part IV — Physical & System Concepts**

*Hardware Layer*
40. [Screen](#40-screen)
41. [Display](#41-display)

*OS Layer (Desktop)*
42. [Desktop](#42-desktop)
43. [Taskbar](#43-taskbar)

*Application Layer (Window)*
44. [Window](#44-window)
45. [Window State](#45-window-state)
46. [Title Bar](#46-title-bar)
47. [Controls](#47-controls)
48. [Menu Bar](#48-menu-bar)
49. [Toolbar](#49-toolbar)
50. [Content Area](#50-content-area)
51. [Viewport](#51-viewport)
52. [Sidebar / Panel](#52-sidebar--panel)
53. [Status Bar](#53-status-bar)
54. [Scrollbar](#54-scrollbar)
55. [Focus](#55-focus)
56. [Z-Order](#56-z-order)
57. [Modal / Dialog](#57-modal--dialog)

---

# Part I — Domain Terms (Trading / Chart)

## 1. Dimension

**English:** Dimension  
**Persian:** بعد  
**Type:** Concept

### Definition
A measurable axis or attribute along which data points are positioned and measured. In this system, the primary dimensions are Time and Price. All structural concepts are defined relative to these dimensions.

### Types
- **Time Dimension (بعد زمان):** The horizontal axis (X) representing chronological order.
- **Price Dimension (بعد قیمت):** The vertical axis (Y) representing asset value.
- **2D Space (فضای دوبعدی):** The combination of Time and Price (Time × Price).

### Related Terms
- [Range (Generic)](#2-range-generic)
- [Data Space](#32-data-space)

---

## 2. Range (Generic)

**English:** Range  
**Persian:** بازه (عمومی)  
**Type:** Concept

### Definition
A continuous span between two boundaries along one or more dimensions. A Range can be 1D or 2D (or higher). It answers "what is the span?" along a defined dimension.

### Types
- **1D Range (بازه یک‌بعدی):** Measured along a single dimension. Examples include [Time Range](#3-time-range) and [Price Range](#4-price-range).
- **2D Range (بازه دوبعدی):** The Cartesian product of two 1D ranges on two different dimensions. In this system, this is called a [Region](#34-region).

### Rationale for "Range" over "Interval"
- **Rust idiomatic:** `Range<T>` is a standard library type.
- **Trading convention:** `Trading Range`, `Range Bar`, `Time Range` are industry-standard terms.
- **Codebase consistency:** Existing helpers use `range` (`range_candles`, `visibleRange`).
- **Flexibility:** Supports inclusive (`a..=b`), exclusive (`a..b`), and open-ended (`a..`) forms.

### Related Terms
- [Dimension](#1-dimension)
- [Time Range](#3-time-range)
- [Price Range](#4-price-range)
- [Region](#34-region)

### References
- `src-tauri/src/market/db.rs` — `range_candles()`
- `src/pages/ScaleLabPage.tsx` — `visibleRange`

---

## 3. Time Range

**English:** Time Range  
**Persian:** بازه زمانی  
**Type:** Tuple (1D Range)

### Definition
A pair of timestamps `(start, end)` describing a continuous interval on the time axis. Both endpoints are inclusive.

### Formal Notation
```
Time Range = [start, end]
Length     = end - start     (in seconds)
```

### Example
```
Time Range = [1738027380, 1738027680]   (Unix seconds)
Length     = 300 seconds                (= 5 minutes)
```

### Note
Not to be confused with [Time Frame](#5-time-frame) — a Time Frame is a *label* (M5), while a Time Range is a *concrete interval*.

### Related Terms
- [Price Range](#4-price-range)
- [Region](#34-region)

### References
- `src-tauri/src/market/boxes.rs` — `Box.ts_start`, `Box.ts_end`

---

## 4. Price Range

**English:** Price Range  
**Persian:** بازه قیمتی  
**Type:** Tuple (1D Range)

### Definition
A pair of prices `(price_low, price_high)` describing a continuous interval on the price axis. Both endpoints are inclusive.

### Formal Notation
```
Price Range = [price_low, price_high]
Height      = price_high - price_low     (in quote currency)
```

### Example
```
Price Range = [2734.74, 2745.07]     (XAUUSD)
Height      = 10.33 USD
Percentage  = (10.33 / 2745.07) × 100 = 0.376%
```

### Related Terms
- [Time Range](#3-time-range)
- [Scale](#6-scale)
- [Region](#34-region)

### References
- `src-tauri/src/market/boxes.rs` — `Box.low`, `Box.high`

---

## 5. Time Frame

**English:** Time Frame (TF)  
**Persian:** تایم فریم  
**Type:** Label

### Definition
A **label** or **standardized category** describing the length of a bar (e.g., `M5` = 5-minute bars). Used for display and grouping.

### Common Values
| Label | Seconds | Description |
|---|---|---|
| M0 | 1 | 1-second |
| M1 | 5 | 5-second |
| M5 | 300 | 5-minute |
| H1 | 3600 | 1-hour |
| D1 | 86400 | 1-day |

### Note
Time Frame is a **label**, NOT a concrete interval. A Time Frame answers "what kind of bar?" — a [Time Range](#3-time-range) answers "which bars exactly?"

### Related Terms
- [Time Range](#3-time-range)
- [scale_sec](#7-scale_sec)

### References
- `src-tauri/src/market/timeframe.rs` — `TF_SECONDS`, `step_from_key()`

---

## 6. Scale

**English:** Scale  
**Persian:** اسکیل  
**Type:** Index (u8, 0..15)

### Definition
A **price-based classification index** `S0..S15` categorizing boxes by their percentage height relative to the origin price.

### Classification Rule
```
pct = (high - low) / origin_price × 100
Scale = smallest S such that scale_start_pct(S) ≤ pct < scale_start_pct(S+1)
```

Where `scale_start_pct(S) = 0.1 × 1.25^S`.

### Table (Excerpt)
| S | Start % | End % |
|---|---|---|
| S0 | 0.100 | 0.125 |
| S5 | 0.305 | 0.382 |
| S10 | 0.999 | 1.252 |

### Note
Scale is **independent of time**. Do not confuse with [scale_sec](#7-scale_sec).

### Related Terms
- [scale_sec](#7-scale_sec)
- [Price Range](#4-price-range)
- [Origin](#29-origin)

### References
- `src-tauri/src/market/boxes.rs` — `classify_scale_from_base()`, `scale_start_pct()`

---

## 7. scale_sec

**English:** scale_sec  
**Persian:** طول اسکیل زمانی  
**Type:** Scalar (seconds, i64)

### Definition
The **length in seconds** of the [Region](#34-region) from which a box was detected. Captures the temporal scale at which a structure was identified.

### Formal Notation
```
scale_sec = Region.length = Region.end - Region.start
```

### Example
| scale_sec | Meaning |
|---|---|
| 1 | Detected at 1-second regions |
| 60 | Detected at 1-minute regions |
| 300 | Detected at 5-minute regions |
| 3600 | Detected at 1-hour regions |
| 86400 | Detected at 1-day regions |

### Note
Different from [Scale](#6-scale): scale_sec is **time**, Scale is **price %**.

### Related Terms
- [Scale](#6-scale)
- [Region](#34-region)

### References
- `src-tauri/src/market/boxes.rs` — `Box.scale_sec`

---

## 8. Bar / Candlestick

**English:** Bar, Candlestick  
**Persian:** کندل  
**Type:** Data structure

### Definition
An OHLC record over a fixed time interval `[start, start + L]`. Contains `open`, `high`, `low`, `close`.

### Fields
```rust
pub struct Candle {
    pub symbol: String,
    pub timeframe: String,
    pub ts: i64,           // Bar start timestamp
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub tick_count: i64,
    pub first_extreme: ExtremeKind,
}
```

### Related Terms
- [Aggregated Bar](#9-aggregated-bar)

### References
- `src-tauri/src/market/model.rs` — `Candle`

---

## 9. Aggregated Bar

**English:** Aggregated Bar  
**Persian:** کندل تجمیعی  
**Type:** Data structure

### Definition
A bar synthesized by combining multiple finer-grained bars into one coarser bar. Used to detect structures at scales not natively stored in the DB.

### Aggregation Rule
Given input bars with timestamps and OHLC, and a target length `L`:
```
bucket(ts) = floor(ts / L) × L
Aggregated Bar = {
    ts:     bucket (of first input bar)
    open:   first input bar's open
    high:   max of all input bars' high
    low:    min of all input bars' low
    close:  last input bar's close
}
```

### Related Terms
- [Bar](#8-bar--candlestick)
- [Region](#34-region)

### References
- `src-tauri/src/market/boxes.rs` — `aggregate_to_seconds()`

---

## 10. Pivot

**English:** Pivot  
**Persian:** پیوت  
**Type:** Detection primitive

### Definition
A local extremum (High or Low) identified by the ZigZag algorithm using a threshold-based retracement rule.

### Fields
```rust
pub struct Pivot {
    pub idx: usize,     // Candle index in the region
    pub price: f64,     // High or Low price
    pub kind: PivotKind, // High | Low
}
```

### Detection Rule
A pivot is emitted when the price retraces by at least `threshold_pct` from the running extreme, then flips direction.

### Related Terms
- [H1, L1](#11-h1-l1)
- [HL, HH](#12-hl-hh)

### References
- `src-tauri/src/market/pivot_zigzag.rs` — `detect_pivots_zigzag()`

---

## 11. H1, L1

**English:** H1, L1  
**Persian:** پیوت اول سقف / کف  
**Type:** Pivot labels

### Definition
- **H1:** The High pivot that marks the **top** of an RBR or the **start** of a DBD.
- **L1:** The Low pivot that marks the **bottom** of a DBD or the **start** of an RBR.

### In RBR
```
P1 (low) → H1 (high) → HL (low) → HH (high)
          ↑ Top of box
```

### In DBD
```
H0 (high) → L1 (low) → LH (high) → LL (low)
            ↑ Bottom of box
```

### Related Terms
- [HL, HH](#12-hl-hh)
- [LH, LL](#13-lh-ll)

---

## 12. HL, HH

**English:** HL (Higher Low), HH (Higher High)  
**Persian:** کف بالاتر، سقف بالاتر  
**Type:** Pivot labels

### Definition
- **HL:** The lowest low that forms the **base** of an RBR. Marks the bottom of the box.
- **HH:** The first high that breaks above H1, confirming the RBR.

### Formal (v6.4.17)
```
HL = deepest candle.low in (H1.idx, HH.idx)
HH = first candle with high > H1.price after HL
```

### Related Terms
- [RBR](#16-rbr-rally-base-rally)

---

## 13. LH, LL

**English:** LH (Lower High), LL (Lower Low)  
**Persian:** سقف پایین‌تر، کف پایین‌تر  
**Type:** Pivot labels

### Definition
- **LH:** The highest high that forms the **base** of a DBD. Marks the top of the box.
- **LL:** The first low that breaks below L1, confirming the DBD.

### Formal (v6.4.17)
```
LH = highest candle.high in (L1.idx, LL.idx)
LL = first candle with low < L1.price after LH
```

### Related Terms
- [DBD](#17-dbd-drop-base-drop)

---

## 14. P1

**English:** P1  
**Persian:** پی ۱  
**Type:** Origin point

### Definition
The **origin low** of an RBR — the starting point of the initial rally. Marks the beginning of the RBR structure. Not always a pivot; can be any low at or below HL.

### In RBR
```
P1 → H1 → HL → HH
↑ Start
```

### Related Terms
- [RBR](#16-rbr-rally-base-rally)

---

## 15. Base

**English:** Base  
**Persian:** بیس  
**Type:** Concept

### Definition
The **consolidation zone** between two impulsive legs (rally-then-rally for RBR, drop-then-drop for DBD). In an RBR, the base is the price range `[HL.price, H1.price]`.

### Related Terms
- [RBR](#16-rbr-rally-base-rally)
- [DBD](#17-dbd-drop-base-drop)

---

## 16. RBR (Rally-Base-Rally)

**English:** RBR — Rally-Base-Rally  
**Persian:** رالی بیس رالی  
**Type:** Pattern

### Definition
A three-phase pattern in a demand zone: (1) an initial rally, (2) a consolidation "base", and (3) a second rally that breaks above the first rally's top.

### Structure
```
    HH ←── Break of H1
    │
H1 ─┤     ← Top of base
    │
HL ─┘     ← Bottom of base
│
P1        ← Origin of first rally
```

### Detection (v6.4.17)
1. `H1` = a High pivot
2. `HL` = first Low pivot after H1
3. `HH` = first candle after HL with `high > H1.price`
4. `HL_price` = deepest candle.low in `(H1.idx, HH.idx)`
5. Box = `[HL_price, H1.price]`

### Related Terms
- [DBD](#17-dbd-drop-base-drop)
- [HL, HH](#12-hl-hh)
- [H1, L1](#11-h1-l1)

---

## 17. DBD (Drop-Base-Drop)

**English:** DBD — Drop-Base-Drop  
**Persian:** دراپ بیس دراپ  
**Type:** Pattern

### Definition
The mirror of RBR in a supply zone: (1) an initial drop, (2) a consolidation "base", and (3) a second drop that breaks below the first drop's bottom.

### Structure
```
LH ─┐     ← Top of base
    │
L1 ─┤     ← Bottom of base
    │
LL ←── Break of L1
```

### Detection (v6.4.17)
Mirror of RBR:
1. `L1` = a Low pivot
2. `LH` = first High pivot after L1
3. `LL` = first candle after LH with `low < L1.price`
4. `LH_price` = highest candle.high in `(L1.idx, LL.idx)`
5. Box = `[L1.price, LH_price]`

### Related Terms
- [RBR](#16-rbr-rally-base-rally)

---

## 18. Engulf

**English:** Engulf  
**Persian:** پوشش  
**Type:** Confirmation event

### Definition
For an RBR: the **first candle** after HH whose `low` penetrates below the HL price. For a DBD: the first candle after LL whose `high` penetrates above the LH price. Wick-based (not close-based).

### Detection
```
RBR: find_engulf_rbr(candles, hh_idx, hl_price)
     → first j > hh_idx with candles[j].low < hl_price

DBD: find_engulf_dbd(candles, ll_idx, lh_price)
     → first j > ll_idx with candles[j].high > lh_price
```

### Related Terms
- [Pending](#19-pending)

### References
- `src-tauri/src/market/boxes.rs` — `find_engulf_rbr()`, `find_engulf_dbd()`

---

## 19. Pending

**English:** Pending  
**Persian:** در انتظار  
**Type:** State

### Definition
A box whose [Engulf](#18-engulf) has not yet occurred. The box's right edge is extended to the last available candle until engulf fires.

### Formal
```
if engulf_ts is None:
    ts_end = candles[candles.len() - 1].ts
    engulf_ts = None
```

### Related Terms
- [Engulf](#18-engulf)
- [Duration](#21-duration)

---

## 20. Box

**English:** Box  
**Persian:** باکس  
**Type:** Data structure

### Definition
A rectangular region on the chart defined by the Cartesian product of a [Time Range](#3-time-range) and a [Price Range](#4-price-range). Either an [RBR](#16-rbr-rally-base-rally) or a [DBD](#17-dbd-drop-base-drop).

### Formal
```
Box = Time Range × Price Range
    = [ts_start, ts_end] × [low, high]
```

### Key Fields
| Field | Type | Description |
|---|---|---|
| `ts_start` | i64 | Start of Time Range |
| `ts_end` | i64 | End of Time Range |
| `low` | f64 | Bottom of Price Range |
| `high` | f64 | Top of Price Range |
| `origin_price` | f64 | Reference price for `scale` — see [Origin](#29-origin) |
| `pattern` | enum | RBR \| DBD |
| `scale` | u8 | Price-based S0..S15 |
| `scale_sec` | i64 | Time-based length |
| `duration_sec` | i64 | ts_end - ts_start (pre-snap) |

### Related Terms
- [Scale](#6-scale)
- [scale_sec](#7-scale_sec)
- [Duration](#21-duration)
- [Origin](#29-origin)

### References
- `src-tauri/src/market/boxes.rs` — `struct Box`

---

## 21. Duration

**English:** Duration  
**Persian:** مدت  
**Type:** Scalar (seconds, i64)

### Definition
The **actual length** of a box in seconds, from `ts_start` to `ts_end`. Recorded **pre-snap** (before v6.4.11 grid snapping) to preserve true structure.

### Formal
```
duration_sec = original_ts_end - original_ts_start
```

### Note
Different from [scale_sec](#7-scale_sec): scale_sec is the **region length** used during detection; duration is the **actual box length** after engulf resolution.

### Related Terms
- [scale_sec](#7-scale_sec)
- [Pending](#19-pending)

### References
- `src-tauri/src/market/boxes.rs` — `Box.duration_sec`

---

## 22. Pattern

**English:** Pattern  
**Persian:** الگو  
**Type:** Enum

### Definition
The structural classification of a box: either `RBR` or `DBD`.

### Values
| Value | Meaning |
|---|---|
| `RBR` | Rally-Base-Rally |
| `DBD` | Drop-Base-Drop |

### Related Terms
- [RBR](#16-rbr-rally-base-rally)
- [DBD](#17-dbd-drop-base-drop)

---

## 23. Zone

**English:** Zone  
**Persian:** ناحیه  
**Type:** Enum

### Definition
The market-structure interpretation: `Demand` (for RBR) or `Supply` (for DBD).

### Values
| Value | Pattern | Meaning |
|---|---|---|
| `Demand` | RBR | Buy-side zone |
| `Supply` | DBD | Sell-side zone |

### Related Terms
- [Pattern](#22-pattern)

---

## 24. Chain

**English:** Chain  
**Persian:** زنجیره  
**Type:** Structure

### Definition
A sequence of boxes sharing the same Y-band, with alternating RBR/DBD patterns and satisfying wick-invalidation + size-tolerance constraints.

### Constraints
1. Same `y_band`
2. Strictly alternating `RBR ↔ DBD`
3. Wick invalidation: `RBR→DBD: curr.low < prev.low`; `DBD→RBR: curr.high > prev.high`
4. Size tolerance: `|h1 - h2| / max(h1, h2) ≤ 0.20`

### Related Terms
- [Box](#20-box)

### References
- `src-tauri/src/market/chain.rs` — `assign_chains()`

---

## 25. Detection Pass

**English:** Detection Pass  
**Persian:** پاس تشخیص  
**Type:** Procedure

### Definition
A single execution of the full detection pipeline over a given [Region](#34-region):
1. Run ZigZag on the region's bars → pivots
2. Iterate pivots → run `scan_rbr` → RBR candidates
3. Iterate pivots → run `scan_dbd` → DBD candidates
4. Post-process (dedup, sort, chain, y_lane)

### Related Terms
- [Region](#34-region)
- [Multi-Scale Detection](#26-multi-scale-detection)

---

## 26. Multi-Scale Detection

**English:** Multi-Scale Detection  
**Persian:** تشخیص چند-اسکیلی  
**Type:** Procedure

### Definition
Running the full detection pipeline over a **chain of Regions**, where each Region has its own length (see [Length](#34-region)). Regions are chained: `Region_i.start = Region_{i-1}.end`.

### Algorithm
```
prev_end = Epoch
while prev_end < data_end:
    region = [prev_end, next_end] × [price_low, price_high]
    boxes = detect_boxes(region)
    for b in boxes:
        b.scale_sec = region.end - region.start
    all += boxes
    prev_end = region.end
dedup(all) by address
```

### Related Terms
- [Region](#34-region)
- [Scale List](#27-scale-list)
- [Epoch](#30-epoch)
- [Aggregated Bar](#9-aggregated-bar)

### References
- `src-tauri/src/market/boxes.rs` — `detect_boxes_multiscale()`

---

## 27. Scale List

**English:** Scale List  
**Persian:** لیست اسکیل‌ها  
**Type:** Configuration

### Definition
The ordered list of region lengths (in seconds) used for multi-scale detection. Currently ~350 entries from 1s to 5y.

### Structure
| Band | Range | Step | Count |
|---|---|---|---|
| 1 | 1s..60s | 1s | 60 |
| 2 | 65s..300s | 5s | 48 |
| 3 | 315s..900s | 15s | 40 |
| 4 | 930s..1800s | 30s | 30 |
| 5 | 1860s..3600s | 60s | 30 |
| 6 | 3900s..21600s | 300s | 60 |
| 7 | 23400s..86400s | 1800s | 36 |
| 8 | 2d..30d | 1d | 29 |
| 9 | 1mo..12mo | ~30d | 12 |
| 10 | 1y..5y | ~365d | 5 |
| **Total** | | | **~349** |

### References
- `src-tauri/src/market/boxes.rs` — `build_scale_list_sec()`

---

## 28. Address

**English:** Address  
**Persian:** آدرس  
**Type:** Identifier (String)

### Definition
A unique string key identifying a box across the system, encoding symbol, detection parameters, and location.

### Format (pre-v6.7.0)
```
{symbol}~M{step}~P{pivot_step}~X{x_idx}~Y{y_abs}~{pattern}
```

### Format (v6.7.0+)
```
{symbol}~L{scale_sec}~M{step}~P{pivot_step}~X{x_idx}~Y{y_abs}~{pattern}
```

### Example
```
XAUUSD~L300~M5~P0~X556167~Y28450~RBR
```

### Purpose
- Unique primary key in DB (`address TEXT PRIMARY KEY`)
- Deduplication key across multi-scale detection
- Stable identifier for parent/child relationships

### References
- `src-tauri/src/market/boxes.rs` — `format!("{}~L{}~{}", ...)`

---

## 29. Origin

**English:** Origin (Origin Price)  
**Persian:** مبدأ / مبدأ قیمتی  
**Type:** Scalar (price, f64)

### Definition
The **reference price** of a [Box](#20-box), used as the denominator when classifying its percentage height (and thus its [Scale](#6-scale)). It is a per-box property stored in the `origin_price` field.

### Formal Notation
```
Origin = origin_price
pct    = (high - low) / origin_price × 100
Scale  = classify_scale_from_base(high - low, origin_price)
```

### Value by Pattern
| Pattern | origin_price |
|---|---|
| [RBR](#16-rbr-rally-base-rally) | `H1.price` (top of base) |
| [DBD](#17-dbd-drop-base-drop) | `L1.price` (bottom of base) |

### Example
```
RBR at Jan 28:
  high         = 2745.07
  low          = 2734.74
  origin_price = 2745.07          ← Origin = H1.price
  pct          = (2745.07 - 2734.74) / 2745.07 × 100 = 0.376%
  Scale        = S5
```

### Distinction from Epoch
| Term | Meaning | Example |
|---|---|---|
| **Origin** (this) | Price reference for `scale` | `H1.price` |
| [Epoch](#30-epoch) | Time reference for chaining | `2026-01-01 00:00:00` |

### Related Terms
- [Box](#20-box)
- [Scale](#6-scale)
- [Price Range](#4-price-range)
- [Epoch](#30-epoch)

### References
- `src-tauri/src/market/boxes.rs` — `Box.origin_price`

---

## 30. Epoch

**English:** Epoch (Origin Time)  
**Persian:** مبدأ زمانی  
**Type:** Scalar (timestamp, i64)

### Definition
The **reference timestamp** from which a chain of [Regions](#34-region) begins. It is the anchor of the region chain on the time axis.

### Formal Notation
```
Region_0.start = Epoch
Region_i.start = Region_{i-1}.end   (for i > 0)
```

Where:
- `Epoch` = reference timestamp
- `Region_i` = i-th region in the chain
- `i` = 0, 1, 2, ...

### Example
```
Epoch = 2026-01-01 00:00:00   (Unix: 1767225600)

Region_0 = [00:00:00, 00:00:10]
Region_1 = [00:00:10, 00:00:40]
Region_2 = [00:00:40, 00:01:00]
...
```

### Common Choices
| Choice | Value | Use Case |
|---|---|---|
| Data start | `candles[0].ts` | Full-history scan |
| Day boundary | `00:00:00 UTC` | Daily analysis |
| User selection | From UI | Custom range |
| Symbol listing date | Metadata | From beginning |

### Why It Matters
If the Epoch shifts, the entire chain of Regions shifts. Epoch must be chosen so that the union of all Regions covers every candle of interest.

### Distinction from Origin
| Term | Meaning | Example |
|---|---|---|
| [Origin](#29-origin) | Price reference for `scale` | `H1.price` |
| **Epoch** (this) | Time reference for chaining | `2026-01-01 00:00:00` |

### Related Terms
- [Region](#34-region)
- [Multi-Scale Detection](#26-multi-scale-detection)
- [Origin](#29-origin)

---

# Part II — Spatial & Coordinate Concepts

## 31. Data

**English:** Data  
**Persian:** داده  
**Type:** Concept

### Definition
The **raw domain values** stored and processed by the system — timestamps, prices, OHLC values, volumes. Data exists **independently of any visualization** or coordinate system.

### Examples
```
Data points:
  (ts=1738027380, open=2745.07, high=2745.87, low=2734.74, close=2740.99)
  (ts=1738027680, open=2740.99, high=2742.30, low=2735.20, close=2736.50)
```

### Characteristics
- **Absolute** — does not change with zoom, pan, or timeframe
- **Persistent** — survives across sessions (stored in DB)
- **Independent** — no reference to screen, window, or viewport

### Related Terms
- [Data Space](#32-data-space)
- [Screen Space](#33-screen-space)

---

## 32. Data Space

**English:** Data Space  
**Persian:** فضای داده  
**Type:** Coordinate system

### Definition
The **coordinate system of domain values** where all chart elements are defined. Timestamps and prices are the axes. **Independent** of zoom, pan, timeframe, or any visual transformation.

### Formal Notation
```
Data Space = (ts, price)
  ts     ∈ [0, ∞) seconds (Unix time)
  price  ∈ ℝ         (quote currency)
```

### Example
```
Data Space of the Jan 28 RBR:
  ts_start   = 1738027380
  ts_end     = 1738286580
  price_low  = 2734.74
  price_high = 2745.07
```

### Stability Under Interactions
| Interaction | Effect on Data Space |
|---|---|
| Zoom | ❌ Unchanged |
| Pan | ❌ Unchanged |
| Timeframe change | ❌ Unchanged |
| Resize window | ❌ Unchanged |

### Relationship to Other Terms
```
Data Space (source of truth)
    ↓ projection
Screen Space (rendered output)
```

### Why It Matters
Data Space is the **reference system**. All Regions, Viewports, and Boxes are defined here. Any storage, logic, or interaction decision should be made in Data Space, not Screen Space.

### Related Terms
- [Data](#31-data)
- [Screen Space](#33-screen-space)
- [Region](#34-region)
- [Viewport](#51-viewport)

---

## 33. Screen Space

**English:** Screen Space  
**Persian:** فضای صفحه  
**Type:** Coordinate system

### Definition
The **2D pixel coordinate system** used by the rendering engine to decide **where to draw** each visual element on the screen. Changes with every zoom, pan, or timeframe change.

### Formal Notation
```
Screen Space = (x_pixel, y_pixel)
  x_pixel ∈ [0, canvas_width]
  y_pixel ∈ [0, canvas_height]
```

### Origin Convention
```
(0, 0) ────────────→ x
  │
  │
  ↓
  y
```

### Example
```
A 1920×1080 canvas:
  (0, 0)       = top-left
  (1920, 0)    = top-right
  (0, 1080)    = bottom-left
  (1920, 1080) = bottom-right
```

### Instability Under Interactions
| Interaction | Effect on Screen Space of a Fixed Point |
|---|---|
| Zoom In | x/y changes (point moves) |
| Pan | x/y changes (point moves) |
| Timeframe change | x/y changes (bar width changes) |

### Projection from Data Space
```
x_pixel = timeToCoordinate(ts)
y_pixel = priceToCoordinate(price)
```

### Rule of Thumb
**Screen Space is a RESULT, not a SOURCE.** Never store data in Screen Space coordinates. Only use it at the moment of rendering.

### Related Terms
- [Data Space](#32-data-space)
- [Data](#31-data)

---

## 34. Region

**English:** Region  
**Persian:** ناحیه  
**Type:** Concept

### Definition
A **rectangular 2D window** on the data plane, defined by the Cartesian product of a [Time Range](#3-time-range) and a [Price Range](#4-price-range). Regions are **chained** along the time axis: each Region starts where the previous one ended. Each Region is internally split into an [Analysis Zone](#35-analysis-zone) and an [Execution Zone](#36-execution-zone) by a [Transition](#37-transition) point.

### Formal Notation
```
Region = [start, end] × [price_low, price_high]

Chaining Rule:
  Region_0.start = Epoch
  Region_i.start = Region_{i-1}.end      for i > 0

Length (per-Region):
  Region_i.length = Region_i.end - Region_i.start
  (each Region has its OWN length; not a global parameter)

Internal Split:
  Region = Analysis Zone ∪ Execution Zone
  Analysis Zone  = [region.start, transition]
  Execution Zone = [transition, region.end]
```

### Five Components
| Component | Symbol | Meaning |
|---|---|---|
| **Start** | `region.start` | Start timestamp (chained from previous) |
| **End** | `region.end` | End timestamp (arbitrary or rule-based) |
| **Price Low** | `region.price_low` | Bottom of price window |
| **Price High** | `region.price_high` | Top of price window |
| **Transition** | `region.transition` | Boundary between Analysis and Execution |

### Example
```
Epoch = 2026-01-01 00:00:00

Region_0 = [00:00:00, 00:00:10] × [2740, 2750]
  transition = 00:00:06
  Analysis Zone  = [00:00:00, 00:00:06]
  Execution Zone = [00:00:06, 00:00:10]

Region_1 = [00:00:10, 00:00:40] × [2735, 2755]
  transition = 00:00:25
  Analysis Zone  = [00:00:10, 00:00:25]
  Execution Zone = [00:00:25, 00:00:40]
...
```

### Chaining Properties
| Property | Value |
|---|---|
| **Gap** | None — `Region_i.start == Region_{i-1}.end` |
| **Overlap** | None — no shared intervals |
| **Coverage** | Continuous from Epoch to data end |
| **Length** | Varies per Region |
| **Price Range** | May vary per Region |

### Why Chained
- **No missed intervals:** every candle belongs to exactly one Region.
- **No duplicate intervals:** no candle is scanned twice.
- **Per-Region length:** allows variable scales within one scan.

### Relationship to Other Terms
```
Epoch (fixed anchor)
  ↓
Region_0 → Region_1 → Region_2 → ... (chain)
  ↓           ↓           ↓
Analysis    Analysis    Analysis
Transition  Transition  Transition
Execution   Execution   Execution
  ↓           ↓           ↓
Boxes      Boxes      Boxes
```

### Related Terms
- [Epoch](#30-epoch)
- [Time Range](#3-time-range)
- [Price Range](#4-price-range)
- [Analysis Zone](#35-analysis-zone)
- [Execution Zone](#36-execution-zone)
- [Transition](#37-transition)
- [Multi-Scale Detection](#26-multi-scale-detection)
- [Viewport](#51-viewport) (independent concept — the visible window)
- [Data Space](#32-data-space)

---

## 35. Analysis Zone

**English:** Analysis Zone  
**Persian:** ناحیه تحلیل  
**Type:** Region sub-area

### Definition
The portion of a [Region](#34-region) dedicated to **cognitive work**: analyzing structure ([RBR](#16-rbr-rally-base-rally) / [DBD](#17-dbd-drop-base-drop)), drawing boxes, evaluating confluence, and preparing the execution plan. The trader is **actively reasoning** — not passively watching — under mental load, deciding whether and how to act.

### Formal Notation
```
Region = Analysis Zone ∪ Execution Zone
Analysis Zone = [region.start, region.transition] × [region.price_low, region.price_high]
```

Where:
- `Analysis Zone.start` = `region.start`
- `Analysis Zone.end`   = `region.transition` (see [Transition](#37-transition))

### Properties
| Property | Value |
|---|---|
| **Role** | Cognitive / decision-making |
| **Active?** | Yes — analysis, drawing, deciding |
| **Duration** | Typically longer than Execution Zone |
| **Ends at** | [Transition](#37-transition) point |

### Why "Analysis" (not "Watch")
- **Watch** implies passivity — just observing.
- **Analysis** conveys active reasoning, drawing, decision-making.
- In trading: `Analysis` is industry-standard for the pre-execution phase.

### Related Terms
- [Region](#34-region)
- [Execution Zone](#36-execution-zone)
- [Transition](#37-transition)
- [RBR](#16-rbr-rally-base-rally)
- [DBD](#17-dbd-drop-base-drop)

---

## 36. Execution Zone

**English:** Execution Zone  
**Persian:** ناحیه اجرا  
**Type:** Region sub-area

### Definition
The portion of a [Region](#34-region) dedicated to **physical action**: detecting a trigger (e.g., break), placing orders, and recording positions. Fast, decisive, without further deliberation. The trader **reacts** to the prepared plan.

### Formal Notation
```
Region = Analysis Zone ∪ Execution Zone
Execution Zone = [region.transition, region.end] × [region.price_low, region.price_high]
```

Where:
- `Execution Zone.start` = `region.transition` (see [Transition](#37-transition))
- `Execution Zone.end`   = `region.end`

### Properties
| Property | Value |
|---|---|
| **Role** | Physical / action-taking |
| **Active?** | Yes — placing orders, executing |
| **Duration** | Typically shorter than Analysis Zone |
| **Starts at** | [Transition](#37-transition) point |

### Why "Execution" (not "Action" or "Reaction")
- **Execution** matches `Analysis` symmetrically (both abstract nouns).
- **Execution** is standard in trading: `Analysis → Execution`.
- **Action** is too generic; **Reaction** implies passivity.

### Related Terms
- [Region](#34-region)
- [Analysis Zone](#35-analysis-zone)
- [Transition](#37-transition)
- [Engulf](#18-engulf)
- [Box](#20-box)

---

## 37. Transition

**English:** Transition  
**Persian:** گذار  
**Type:** Point (timestamp)

### Definition
The **boundary point** within a [Region](#34-region) that marks the separation between the [Analysis Zone](#35-analysis-zone) and the [Execution Zone](#36-execution-zone). It belongs to **both** zones: the end of Analysis is the start of Execution.

The Transition point is **user-defined via the UI**, marking the moment when the trader shifts from cognitive work (Analysis) to physical action (Execution).

### Formal Notation
```
Transition = region.transition   (timestamp)

Region = [region.start, region.end] × [region.price_low, region.price_high]
  Analysis Zone  = [region.start,       region.transition]
  Execution Zone = [region.transition,  region.end]

Invariant:
  Analysis Zone.end == Execution Zone.start == region.transition
```

### Structure Diagram
```
region.start ─────────── transition ─────────── region.end
     │                        │                        │
     │────── Analysis ────────│                        │
     │                        │                        │
     │                        │────── Execution ───────│
     │                        │                        │
     │                        │                        │
```

### Example
```
Region     = [00:00:00, 00:03:00]
Transition = 00:01:30

Analysis Zone  = [00:00:00, 00:01:30]   ← 90 seconds
Execution Zone = [00:01:30, 00:03:00]   ← 90 seconds
```

### Why "Transition" (not "Boundary")
- **Boundary** is neutral but only describes a *separator*.
- **Transition** describes the *moment of passing* from one phase to another.
- **Transition** belongs to both zones — end of Analysis, start of Execution.
- **Transition** is standard in physics, mathematics, and state machines (`Phase Transition`, `State Transition`).

### Why User-Defined (not Automatic)
- The trader knows the psychological moment of shift.
- The algorithm cannot infer when the trader transitions from analyzing to executing.
- Different traders have different transition points for the same structure.

### Related Terms
- [Region](#34-region)
- [Analysis Zone](#35-analysis-zone)
- [Execution Zone](#36-execution-zone)

### References
- `src-tauri/src/market/region.rs` — `Region.transition`

---

# Part III — Interaction

## 38. Zoom

**English:** Zoom  
**Persian:** زوم  
**Type:** Interaction

### Definition
An interaction that **changes the extent** of the [Viewport](#51-viewport) along one or both axes, making content appear larger or smaller. Zoom does **not** change the underlying [Data](#31-data) — only the visible window.

### Direction
| Action | Viewport Effect | Content Effect |
|---|---|---|
| **Zoom In** | Viewport shrinks | Content appears larger |
| **Zoom Out** | Viewport expands | Content appears smaller |

### Example
```
Before Zoom In:
  Viewport = [Jun 2025, Oct 2025] × [3,750, 4,800]

After Zoom In (2×):
  Viewport = [Jul 2025, Sep 2025] × [4,000, 4,700]
```

### Effect on Spatial Spaces
| Space | Changed? |
|---|---|
| [Data Space](#32-data-space) | ❌ Unchanged |
| [Viewport](#51-viewport) | ✅ Changed |
| [Screen Space](#33-screen-space) | ✅ Changed (projection differs) |
| [Region](#34-region) | ❌ Unchanged (its identity persists) |

### Trigger Mechanisms
- Mouse wheel (default in most charts)
- Pinch gesture (trackpad / touchscreen)
- Zoom In/Out buttons in the Toolbar
- Keyboard shortcuts (`+` / `-`)

### Related Terms
- [Pan](#39-pan)
- [Viewport](#51-viewport)
- [Region](#34-region)

---

## 39. Pan

**English:** Pan  
**Persian:** پن / جابجایی  
**Type:** Interaction

### Definition
An interaction that **shifts the position** of the [Viewport](#51-viewport) along one or both axes without changing its **extent** (size). The user "slides" the visible window over the underlying [Data](#31-data).

### Example
```
Before Pan:
  Viewport = [Jun 2025, Oct 2025] × [3,750, 4,800]

After Pan (right by 1 month):
  Viewport = [Jul 2025, Nov 2025] × [3,750, 4,800]
```

### Effect on Spatial Spaces
| Space | Changed? |
|---|---|
| [Data Space](#32-data-space) | ❌ Unchanged |
| [Viewport](#51-viewport) | ✅ Shifted (same extent) |
| [Screen Space](#33-screen-space) | ✅ Changed (projection differs) |
| [Region](#34-region) | ❌ Unchanged |

### Difference from Zoom
| Aspect | Zoom | Pan |
|---|---|---|
| **Extent** | Changes | Unchanged |
| **Position** | May change | Changes |
| **Content size** | Changes | Unchanged |
| **Content position** | May change | Changes |

### Trigger Mechanisms
- Click-and-drag on the chart
- Arrow keys
- Scrollbar thumb drag
- Touch drag

### Related Terms
- [Zoom](#38-zoom)
- [Viewport](#51-viewport)
- [Scrollbar](#54-scrollbar)

---

# Part IV — Physical & System Concepts

## Hierarchy Overview

```
📺 Hardware Layer
  ├── Screen (§40)
  └── Display (§41)

🖥️ OS Layer (Desktop)
  ├── Desktop (§42)
  └── Taskbar (§43)

🪟 Application Layer (Window)
  ├── Window (§44)
  │     ├── Window State (§45)
  │     │     ├── Normal / Restored
  │     │     ├── Maximized
  │     │     ├── Fullscreen
  │     │     └── Minimized
  │     ├── Title Bar (§46)
  │     ├── Controls (§47)
  │     ├── Menu Bar (§48)
  │     ├── Toolbar (§49)
  │     └── Content Area (§50)
  │           └── Viewport (§51)
  ├── Sidebar / Panel (§52)
  ├── Status Bar (§53)
  ├── Scrollbar (§54)
  ├── Focus (§55)
  ├── Z-Order (§56)
  └── Modal / Dialog (§57)
```

---

## Hardware Layer

### 40. Screen

**English:** Screen  
**Persian:** نمایشگر  
**Type:** Hardware concept

#### Definition
The **physical display device** of a computer — the entire screen surface that can be physically powered on or off, independent of any operating system or software state.

#### Characteristics
- **Hardware-level** concept (exists even when the OS is not running)
- Can be **on** (emitting light) or **off** (dark)
- Measured in physical units (inches, pixels, DPI)
- Does not depend on Windows, Linux, or macOS

#### Example
```
Laptop screen = 15.6" IPS, 1920×1080 resolution
  → Turned off: no light emitted
  → Turned on with Windows: shows Desktop
  → Turned on with BIOS: shows firmware screen
```

#### Related Terms
- [Display](#41-display)
- [Desktop](#42-desktop)
- [Window](#44-window)
- [Viewport](#51-viewport)

---

### 41. Display

**English:** Display  
**Persian:** نمایش  
**Type:** Hardware/OS concept

#### Definition
Synonym for [Screen](#40-screen) in most contexts, but with a subtle nuance: **Display** often refers to the **active rendering surface** — the screen when it is powered on and showing content.

#### Difference from Screen
| Aspect | Screen | Display |
|---|---|---|
| Emphasis | Physical device | Active rendering |
| State | Can be off | Implies on |
| Context | Hardware spec | OS/UI context |

#### Common Usage
- "Turn off the **screen**" (hardware power)
- "The **display** shows..." (rendering)

#### Related Terms
- [Screen](#40-screen)
- [Desktop](#42-desktop)

---

## OS Layer (Desktop)

### 42. Desktop

**English:** Desktop  
**Persian:** دسکتاپ  
**Type:** OS concept

#### Definition
The **working area of the operating system** that appears after login, before any application window is opened. It is the OS-level "surface" on which application windows are placed.

#### Components
```
Desktop = 
    Wallpaper (background image)
  + Icons (files, shortcuts)
  + Taskbar (start menu, running apps, clock)
  + Windows (application frames, drawn on top)
```

#### Key Characteristics
- **Software-level** concept (needs an OS)
- **Persistent** — stays there whether windows are open or not
- **Visual base layer** — application windows are drawn ON TOP of it
- **Visible only where no window covers it**

#### Visibility Rules
| Window state | Desktop visible? |
|---|---|
| No windows open | ✅ Fully visible |
| Small window centered | ✅ Partially visible (around window) |
| Window maximized | ⚠️ Only Taskbar visible |
| Window fullscreen (F11) | ❌ Completely hidden |

#### Analogy
**Desktop = a physical desk**
- Wallpaper = the tablecloth
- Icons = items placed on the desk
- Taskbar = the drawer (always accessible)
- A window = a large sheet of paper spread on the desk (hides what's under it)

#### Related Terms
- [Screen](#40-screen)
- [Display](#41-display)
- [Taskbar](#43-taskbar)
- [Window](#44-window)

---

### 43. Taskbar

**English:** Taskbar  
**Persian:** نوار وظیفه  
**Type:** OS component

#### Definition
A **horizontal bar**, typically at the bottom of the [Desktop](#42-desktop), containing:
- **Start Button** (Windows logo)
- **Pinned App icons**
- **Running App icons**
- **System Tray** (clock, network, volume, notifications)

#### Characteristics
- Managed by the **operating system** (not by applications)
- **Always visible** — even when a window is maximized
- **Hidden** when a window enters Fullscreen state (F11)
- Clicking a running app icon toggles Minimize/Restore

#### Platform Variants
| OS | Position | Name |
|---|---|---|
| Windows | Bottom | Taskbar |
| macOS | Top | Menu Bar + Dock (bottom) |
| Linux (GNOME) | Top | Top Bar |
| Linux (KDE) | Bottom | Panel |

#### Note
A user can move the Taskbar (usually only to left/right/top edges on Windows). Its contents are OS-controlled, not app-controlled.

#### Related Terms
- [Desktop](#42-desktop)
- [Window](#44-window)
- [Window State](#45-window-state)

---

## Application Layer (Window)

### 44. Window

**English:** Window  
**Persian:** پنجره  
**Type:** OS/UI concept

#### Definition
A **framed container** for an application's content, managed by the operating system's window manager. Windows are drawn **on top of the [Desktop](#42-desktop)**.

#### Anatomy
```
┌─────────────────────────────────────────┐
│ [Icon] Title          ─  □  ✕          │ ← Title Bar + Controls
├─────────────────────────────────────────┤
│ File  Edit  View  Help                  │ ← Menu Bar
├─────────────────────────────────────────┤
│ [🔍] [📁] [✏️] [🗑️] ...              │ ← Toolbar
├─────────────────────────────────────────┤
│                                         │
│         محتوای پنجره                    │ ← Content Area
│                                         │
├─────────────────────────────────────────┤
│ Ready                                    │ ← Status Bar
└─────────────────────────────────────────┘
```

#### Window States
| State | Taskbar | Title bar | Content area |
|---|---|---|---|
| Restored | ✅ visible | ✅ visible | Partial screen |
| Maximized | ✅ visible | ✅ visible | Full screen minus taskbar |
| Fullscreen (F11) | ❌ hidden | ❌ hidden | Full screen |
| Minimized | ✅ visible | ❌ hidden | Not rendered |

#### Relationship to Other Terms
```
Desktop
  └── Window (e.g., Chrome)
        └── Content Area
              └── Viewport (rendered content)
                    └── Chart (in Scale Lab)
```

#### Related Terms
- [Desktop](#42-desktop)
- [Window State](#45-window-state)
- [Title Bar](#46-title-bar)
- [Content Area](#50-content-area)
- [Viewport](#51-viewport)

---

### 45. Window State

**English:** Window State  
**Persian:** حالت پنجره  
**Type:** Enum (4 values)

#### Definition
The **current presentation mode** of a [Window](#44-window). At any moment, a Window is in exactly **one** of these four states.

#### Values
| Value | Persian | Taskbar | Title Bar | Content Area | Desktop |
|---|---|---|---|---|---|
| **Normal / Restored** | عادی | ✅ | ✅ | Partial | ✅ visible |
| **Maximized** | بزرگ | ✅ | ✅ | Full (minus taskbar) | ⚠️ only Taskbar |
| **Fullscreen** | تمام‌صفحه | ❌ | ❌ | Full screen | ❌ hidden |
| **Minimized** | کوچیک | ✅ | ❌ | Not rendered | ✅ visible |

#### Transitions
```
Normal ──maximize───→ Maximized
Normal ──F11────────→ Fullscreen
Normal ──minimize───→ Minimized
Maximized ──restore─→ Normal
Fullscreen ──F11/Esc→ Normal
Minimized ──click───→ Normal (previous state)
```

#### Platform References
- **Windows API:** `ShowWindow(hwnd, SW_NORMAL | SW_MAXIMIZE | SW_MINIMIZE)`
- **Electron:** `BrowserWindow.getState()` returns `'normal' | 'maximized' | 'minimized' | 'fullscreen'`
- **Tauri:** `window.is_maximized()`, `window.is_minimized()`, `window.is_fullscreen()`
- **Microsoft .NET:** `Form.WindowState` enum = `Normal | Minimized | Maximized`

#### Related Terms
- [Window](#44-window)
- [Title Bar](#46-title-bar)
- [Controls](#47-controls)

---

### 46. Title Bar

**English:** Title Bar  
**Persian:** نوار عنوان  
**Type:** Window component

#### Definition
The **horizontal bar at the very top of a [Window](#44-window)**, displaying:
- Window icon (left)
- Window title (center-left)
- [Controls](#47-controls) (right: minimize, maximize, close)

#### Behavior
- **Dragging** the Title Bar moves the window
- **Double-click** toggles between Normal and Maximized
- **Right-click** opens the system menu (Move, Size, Minimize, Close)
- **Hidden** in Fullscreen state

#### Platform Variants
| OS | Controls position | Style |
|---|---|---|
| Windows | Right | `─ □ ✕` |
| macOS | Left | `● ● ●` (traffic lights) |
| Linux (GNOME) | Right | `─ □ ✕` |
| Linux (KDE) | Right (configurable) | `─ □ ✕` |

#### Related Terms
- [Window](#44-window)
- [Controls](#47-controls)

---

### 47. Controls

**English:** Controls (Window Controls)  
**Persian:** دکمه‌های کنترل پنجره  
**Type:** Window component

#### Definition
The **three buttons** at the top-right (Windows) or top-left (macOS) of the [Title Bar](#46-title-bar) that manage the [Window State](#45-window-state).

#### Buttons (Windows convention)
| Symbol | Name | Action |
|---|---|---|
| `─` | Minimize | → Minimized state |
| `□` / `❐` | Maximize / Restore | Toggles Maximized ↔ Normal |
| `✕` | Close | Terminates the window (or app) |

#### Buttons (macOS convention)
| Symbol | Name | Action |
|---|---|---|
| 🔴 Red | Close | Terminates the window |
| 🟡 Yellow | Minimize | → Minimized state |
| 🟢 Green | Maximize/Fullscreen | Toggles Fullscreen |

#### Note
- In Fullscreen state, Controls are **hidden**
- Hovering `✕` typically highlights it red (Windows)
- Some apps disable Maximize (e.g., dialogs)

#### Related Terms
- [Title Bar](#46-title-bar)
- [Window State](#45-window-state)

---

### 48. Menu Bar

**English:** Menu Bar  
**Persian:** نوار منو  
**Type:** Window component

#### Definition
A **horizontal strip** of textual menu items (File, Edit, View, Help, …) placed directly below the [Title Bar](#46-title-bar). Clicking an item opens a **dropdown menu** of commands.

#### Typical Items
```
File   Edit   View   Insert   Format   Tools   Help
```

#### Behavior
- Always **visible** by default (not to be confused with a hidden menu)
- **Keyboard shortcut:** `Alt + <letter>` (Windows); differs on macOS
- **macOS:** Menu Bar is at the top of the [Screen](#40-screen), not per-window

#### Related Terms
- [Toolbar](#49-toolbar)
- [Window](#44-window)

---

### 49. Toolbar

**English:** Toolbar  
**Persian:** نوار ابزار  
**Type:** Window component

#### Definition
A **horizontal strip of icon buttons** providing quick access to frequently-used commands. Often placed below the [Menu Bar](#48-menu-bar) or docked on the sides of the window.

#### Common Contents
- New / Open / Save
- Undo / Redo
- Cut / Copy / Paste
- Zoom In / Zoom Out
- View mode toggles

#### Example (Scale Lab)
```
[← Canvas]  [M-Scale Lab]  [+5] [↑] [↓]  [Measure]  [👁] [⚙] [⛶]
```

#### Note
Many modern apps (e.g., browsers, VS Code) merge Menu Bar and Toolbar into a single **Command Bar** or **Ribbon**.

#### Related Terms
- [Menu Bar](#48-menu-bar)
- [Window](#44-window)

---

### 50. Content Area

**English:** Content Area  
**Persian:** ناحیه محتوا  
**Type:** Window component

#### Definition
The **inner region of a [Window](#44-window)** where the application's main content is rendered. It is bounded by the [Title Bar](#46-title-bar), [Menu Bar](#48-menu-bar), [Toolbar](#49-toolbar), [Status Bar](#53-status-bar), and [Sidebar](#52-sidebar--panel).

#### Relationship to Viewport
The Content Area is the **frame**, the [Viewport](#51-viewport) is the **visible subset** of the content that is currently rendered inside it. For a chart, the Content Area hosts the Viewport.

#### Related Terms
- [Viewport](#51-viewport)
- [Sidebar / Panel](#52-sidebar--panel)

---

### 51. Viewport

**English:** Viewport  
**Persian:** ناحیه نمایش  
**Type:** UI concept

#### Definition
The **rectangular visible area** of an application's content, defined by the four boundaries of what is currently rendered. Any content outside the viewport is clipped (not drawn).

#### Formal Notation (for a chart)
```
Viewport = [visible_ts_start, visible_ts_end] × [visible_price_low, visible_price_high]
```

#### Example (Scale Lab chart)
```
visible_ts_start   = 2025-01-27 12:00
visible_ts_end     = 2025-01-29 18:00
visible_price_low  = 2720.00
visible_price_high = 2790.00
```

Zoom/Pan = changing these four numbers.

#### Stability
A Viewport's **values** change with [Zoom](#38-zoom) and [Pan](#39-pan). Unlike a [Region](#34-region), a Viewport is **not** a stable identity — it is a moving window over the data.

#### Note
- In web browsers: viewport = the content area inside the browser chrome.
- In charts: viewport = the visible time/price range.
- In Vora Mind: `viewport` refers to the **Chart Viewport** — the visible time and price range in Scale Lab.

#### Related Terms
- [Content Area](#50-content-area)
- [Window](#44-window)
- [Region](#34-region)
- [Zoom](#38-zoom)
- [Pan](#39-pan)

#### References
- `src/pages/ScaleLabPage.tsx` — `visibleRange` / `getVisibleLogicalRange()`

---

### 52. Sidebar / Panel

**English:** Sidebar, Panel  
**Persian:** نوار کناری / پنل  
**Type:** Window component

#### Definition
A **vertical (or horizontal) region** docked to one side of the [Content Area](#50-content-area), containing auxiliary controls, settings, or lists.

#### Common Sidebars
- Left: file tree, tool palette
- Right: properties inspector, settings panel

#### Example (Scale Lab)
The **Policy Panel** on the right is a Sidebar.

#### Behavior
- Can often be **collapsed/expanded** (toggled by a toolbar button)
- **Resizable** by dragging its edge
- May **overlay** the content or **push** it (adjusting the content area width)

#### Related Terms
- [Content Area](#50-content-area)
- [Toolbar](#49-toolbar)

---

### 53. Status Bar

**English:** Status Bar  
**Persian:** نوار وضعیت  
**Type:** Window component

#### Definition
A **horizontal strip at the bottom of a [Window](#44-window)** that displays **read-only information** about the current state: coordinates, mode, file size, progress, etc.

#### Example (Scale Lab)
```
ATR(14)   $6.31   |   Candles   429   |   Range   $85.67   |   Chains   0
```

#### Common Contents
- Cursor position (row/column, time/price)
- Word count, file size
- Active mode (Insert/Overwrite, Layer name)
- Progress bars for background tasks
- Network/connection status

#### Behavior
- Usually **always visible**
- May contain **clickable segments** (e.g., click "Ln 1, Col 1" to jump)
- Not to be confused with the OS-level Taskbar

#### Related Terms
- [Taskbar](#43-taskbar) (OS-level)
- [Content Area](#50-content-area)

---

### 54. Scrollbar

**English:** Scrollbar  
**Persian:** نوار اسکرول  
**Type:** UI component

#### Definition
A **vertical or horizontal bar** on the edge of a scrollable region, allowing the user to **pan** through content larger than the visible area.

#### Anatomy
```
       ▲  ← Up arrow
       │
     ┌─┐
     │ │  ← Thumb (draggable)
     └─┘
       │
       ▼  ← Down arrow
```

#### Interaction
- **Drag thumb** → continuous pan
- **Click track** → page up/down
- **Click arrows** → line up/down
- **Mouse wheel** → smooth scroll

#### Note
- Modern UIs (macOS, mobile) often **auto-hide** scrollbars
- In charts, scrollbars are sometimes replaced by **pan gestures**

#### Related Terms
- [Content Area](#50-content-area)
- [Viewport](#51-viewport)
- [Pan](#39-pan)

---

### 55. Focus

**English:** Focus  
**Persian:** فوکوس / تمرکز  
**Type:** UI state

#### Definition
The state in which a specific **[Window](#44-window)** or **UI element** (button, input, chart) is the **target of keyboard and mouse events**. At any moment, **exactly one window** has focus (per desktop).

#### Visual Indications
- **Window focus:** Title Bar is highlighted (colored), while unfocused windows have a muted Title Bar
- **Element focus:** Border highlight, blinking cursor, or outline
- **Web/Electron:** `document.activeElement`

#### Behavior
- Clicking a window gives it focus
- `Alt + Tab` cycles focus between windows
- In Fullscreen, focus is exclusive (no way to focus another app without exiting)

#### Related Terms
- [Window](#44-window)
- [Z-Order](#56-z-order)

---

### 56. Z-Order

**English:** Z-Order  
**Persian:** ترتیب لایه  
**Type:** UI concept

#### Definition
The **stacking order** of [Windows](#44-window) along the imaginary **Z-axis** (perpendicular to the screen). Determines which window is drawn **on top** of which when they overlap.

#### Rules
- **Focused window** is at the **top** of the Z-order (fully visible)
- **Unfocused windows** are stacked below
- **Modal dialogs** are always above their parent window
- **Always-on-top** windows (e.g., task managers) stay above everything

#### Example
```
Z-order (top to bottom):
  1. Modal dialog (child of Chrome)
  2. Chrome (focused)
  3. VS Code
  4. Notepad
  5. Desktop
```

#### Related Terms
- [Window](#44-window)
- [Focus](#55-focus)
- [Modal / Dialog](#57-modal--dialog)

---

### 57. Modal / Dialog

**English:** Modal, Dialog  
**Persian:** مودال / پنجره گفتگو  
**Type:** Window subtype

#### Definition
A **special Window** that requires the user's **immediate attention** and **blocks interaction** with the rest of the application (or the parent window) until it is closed.

#### Two Subtypes
| Type | Blocks | Example |
|---|---|---|
| **Modal** | Entire app | Save dialog, Preferences |
| **Modeless** | Nothing (stays alongside) | Find & Replace |

#### Characteristics
- **Always on top** of its parent window
- **Cannot be moved behind** its parent
- Usually **smaller** than the main window
- Often has **no minimize button**
- Closing it returns focus to the parent

#### Common Dialogs
- **Message Box** — OK / Cancel
- **File Picker** — Open / Save
- **Confirmation** — "Are you sure?"
- **Settings / Preferences**

#### Related Terms
- [Window](#44-window)
- [Focus](#55-focus)
- [Z-Order](#56-z-order)

---

## Appendix A — Naming Conventions

| Prefer | Avoid | Reason |
|---|---|---|
| Dimension | Axis, Vector | "Dimension" is the mathematical standard for measurable axes |
| Range | Interval | Rust/Trading standard |
| Time Range | Time Interval | Consistency with "Range" |
| Price Range | Price Interval | Consistency with "Range" |
| scale_sec | duration | scale_sec = region length; duration = box length |
| Origin | Base Price | "Origin" is standard for scale reference |
| Epoch | Start Time, Anchor | "Epoch" is standard for time reference |
| Region | Frame, Window, Segment | Region is chained 2D; no conflict with OS Window |
| Analysis Zone | Watch Zone, Setup Zone | "Analysis" conveys active work |
| Execution Zone | Action Zone, Reaction Zone | "Execution" is the standard trading phase |
| Transition | Boundary, Split | "Transition" conveys passing from one to another |
| Screen | Monitor | Screen is more general (includes laptops) |
| Desktop | Homescreen | "Desktop" is the Windows/Linux/macOS standard term |
| Window | Frame | Reserved for OS window concept |
| Viewport | Canvas | Viewport implies visible subset, not the full canvas |
| Window State | Window Mode | "State" matches Windows API / .NET / Tauri |
| Content Area | Canvas | Content Area is the frame; Viewport is the visible part |
| Data Space | Domain Space | "Data" pairs with "Screen" |
| Screen Space | Pixel Space | "Screen" pairs with "Data" |

---

## Appendix B — Data Space vs Screen Space (Reference System)

```
┌───────────────────────────────────────────────────────────┐
│ DATA SPACE (Source of Truth)                              │
│   Values: (ts, price)                                     │
│   Used by: Region, Viewport, Box, Chain                   │
│   Stability: STABLE — unchanged by zoom, pan, timeframe   │
├───────────────────────────────────────────────────────────┤
│ PROJECTION                                                │
│   x_pixel = timeToCoordinate(ts)                          │
│   y_pixel = priceToCoordinate(price)                      │
├───────────────────────────────────────────────────────────┤
│ SCREEN SPACE (Rendering Output)                           │
│   Values: (x_pixel, y_pixel)                              │
│   Used by: Rendering engine only                          │
│   Stability: UNSTABLE — changes with every interaction    │
└───────────────────────────────────────────────────────────┘

Rule: Store in Data Space. Project to Screen Space only at render time.
```

---

## Appendix C — Origin vs Epoch (Reference Points)

```
┌───────────────────────────────────────────────────────────┐
│ ORIGIN (Price Reference)                                  │
│   Field:  Box.origin_price                                │
│   Value:  H1.price (RBR)  |  L1.price (DBD)               │
│   Used:   classify_scale_from_base()                      │
│   Role:   Denominator for computing pct and Scale         │
├───────────────────────────────────────────────────────────┤
│ EPOCH (Time Reference)                                    │
│   Field:  scan_epoch (parameter of Multi-Scale Detection) │
│   Value:  Any anchor timestamp                            │
│   Used:   Region_0.start = Epoch                          │
│   Role:   Anchor point for Region chain                   │
└───────────────────────────────────────────────────────────┘
```

---

## Appendix D — Region Chain (Multi-Scale Detection)

```
Epoch ─────────► end_0 ─────────► end_1 ─────────► end_2 ────► data_end
      │              │                │                │
      ├── Region 0 ──┤                │                │
      │ ┌──────────┬──────────┐       │                │
      │ │ Analysis │Execution │       │                │
      │ └──────────┴──────────┘       │                │
      │     ↑           ↑             │                │
      │   start     transition        │                │
      │              │                │                │
      │              ├── Region 1 ────┤                │
      │              │ ┌──────────┬──────────┐          │
      │              │ │ Analysis │Execution │          │
      │              │ └──────────┴──────────┘          │
      │              │     ↑           ↑               │
      │              │   start     transition          │
      │              │                │                │
      │              │                ├── Region 2 ────┤
      │              │                │ ┌──────────┬──────────┐
      │              │                │ │ Analysis │Execution │
      │              │                │ └──────────┴──────────┘
      │              │                │     ↑           ↑
      │              │                │   start     transition
      │              │                │                │
      │              │                │                ├── ...

Properties:
  • Region_i.start = Region_{i-1}.end   (no gap, no overlap)
  • Region_i.length = Region_i.end - Region_i.start   (variable per Region)
  • Region_i.price_range = [price_low, price_high]     (may vary per Region)

Sub-areas (per Region):
  • Analysis Zone   = [region.start,        region.transition]
  • Execution Zone  = [region.transition,   region.end]

Coverage:
  • Union of all Regions = [Epoch, data_end]
  • Every candle belongs to exactly one Region
```

---

## Appendix E — Interaction Effects on Spaces

| Interaction | Data Space | Viewport | Screen Space | Region |
|---|---|---|---|---|
| **Zoom In** | unchanged | smaller | changes | unchanged |
| **Zoom Out** | unchanged | larger | changes | unchanged |
| **Pan Left/Right** | unchanged | shifted | changes | unchanged |
| **Pan Up/Down** | unchanged | shifted | changes | unchanged |
| **Timeframe Change** | unchanged | changed | changes | unchanged |
| **Resize Window** | unchanged | same range | changes | unchanged |

**Key insight:** Data Space and Region are **stable**. Viewport, Screen Space are **volatile**.

---

## Appendix F — Physical vs Software Layering

```
┌───────────────────────────────────────────────────────────┐
│ HARDWARE LAYER                                            │
│   Screen / Display — the physical monitor                 │
├───────────────────────────────────────────────────────────┤
│ OS LAYER                                                  │
│   Desktop — wallpaper + icons + Taskbar                   │
│     └── Windows — application frames                      │
│           ├── Title Bar + Controls + Menu Bar + Toolbar   │
│           ├── Content Area — main region                  │
│           │     ├── Sidebar / Panel                       │
│           │     └── Viewport — visible content            │
│           ├── Status Bar                                  │
│           └── Modal / Dialog — special child windows      │
├───────────────────────────────────────────────────────────┤
│ APPLICATION LAYER (Vora Mind)                             │
│   Chart Viewport (Data Space) — visible time/price range  │
│     ├── Region — chained 2D detection window              │
│     │     ├── Analysis Zone  — cognitive work             │
│     │     ├── Transition     — boundary point             │
│     │     └── Execution Zone — physical action            │
│     └── Box    — detected RBR/DBD                         │
├───────────────────────────────────────────────────────────┤
│ SPATIAL LAYER (Orthogonal)                                │
│   Data Space  ←→  Screen Space  (via projection)          │
└───────────────────────────────────────────────────────────┘
```

---

## Appendix G — Window Anatomy Diagram

```
┌───────────────────────────────────────────────────────┐
│ 🖼️  Title                    ─  □  ✕                │ ← Title Bar + Controls
├───────────────────────────────────────────────────────┤
│ File   Edit   View   Help                            │ ← Menu Bar
├───────────────────────────────────────────────────────┤
│ [🔍] [📁] [✏️] [🗑️]                                │ ← Toolbar
├───────────────┬───────────────────────────────────────┤
│               │                                       │
│   Sidebar     │         Content Area                  │
│   (Panel)     │           └── Viewport                │
│               │                                       │
├───────────────┴───────────────────────────────────────┤
│ Ready                                     Ln 1, Col 1 │ ← Status Bar
└───────────────────────────────────────────────────────┘
```

---

## Appendix H — Region Anatomy Diagram

```
┌───────────────────────────────────────────────────────────┐
│ Region_i                                                  │
│                                                           │
│  ┌─────────────────────────┬────────────────────────────┐ │
│  │                         │                            │ │
│  │    Analysis Zone        │     Execution Zone         │ │
│  │    (ناحیه تحلیل)        │     (ناحیه اجرا)           │ │
│  │                         │                            │ │
│  │  • Analyze structure    │  • Detect trigger          │ │
│  │  • Draw boxes           │  • Place orders            │ │
│  │  • Evaluate confluence  │  • Record positions        │ │
│  │  • Prepare plan         │  • Act fast, no rethink    │ │
│  │                         │                            │ │
│  │  COGNITIVE WORK         │  PHYSICAL ACTION           │ │
│  └─────────────────────────┴────────────────────────────┘ │
│            ↑                          ↑                    │
│            │                          │                    │
│        region.start              region.end                │
│                      ↑                                     │
│                  transition                                │
│                                                           │
└───────────────────────────────────────────────────────────┘

Sub-area boundaries:
  • analysis.start   = region.start
  • analysis.end     = region.transition
  • transition       = region.transition
  • execution.start  = region.transition
  • execution.end    = region.end
```

---

## Appendix I — Change Log

| Version | Date | Changes |
|---|---|---|
| 1.0.0 | 2026-10-06 | Initial terminology reference (29 domain terms). |
| 1.1.0 | 2026-10-06 | Added Screen, Display, Desktop, Window, Viewport. |
| 1.2.0 | 2026-10-06 | Added Window State, Title Bar, Controls, Taskbar, Menu Bar, Toolbar, Content Area, Sidebar/Panel, Status Bar, Scrollbar, Focus, Z-Order, Modal/Dialog. Reorganized Physical & System Concepts into 3 layers. |
| 1.3.0 | 2026-10-06 | Added Part II (Spatial & Coordinate: Data, Data Space, Screen Space, Region) and Part III (Interaction: Zoom, Pan). Full renumbering. Added Document Structure tree, Data-vs-Screen reference appendix, interaction effect matrix. |
| 1.4.0 | 2026-10-07 | Added Origin (§30) and Epoch (§31) at end of Part I. Full renumbering of §32–§55. Updated Frame and Step to reference Epoch. Added Appendix C (Origin vs Epoch reference points). |
| 2.0.0 | 2026-10-07 | **BREAKING:** Removed Frame and Step. Region redefined as chained 2D window: `[start, end] × [price_low, price_high]` with `Region_i.start = Region_{i-1}.end`. Length now per-Region (not global). Multi-Scale Detection updated to chain-based algorithm. Full renumbering of §5–§53. Added Appendix D (Region Chain diagram). |
| 2.1.0 | 2026-10-07 | Added Analysis Zone (§34) and Execution Zone (§35) as Region sub-areas. Full renumbering of §36–§55. Added Appendix H (Region Anatomy Diagram). Updated Appendix D to show sub-areas. |
| 2.2.0 | 2026-10-07 | Added Transition (§36) as the boundary point between Analysis Zone and Execution Zone. Full renumbering of §37–§56. Updated Region (§33) to reference Transition. Updated Analysis Zone and Execution Zone to reference Transition. Updated Appendices D, F, H to show Transition. |
| 2.3.0 | 2026-10-13 | Added Dimension (§1) and redefined Range (§2) to explicitly support 1D and 2D abstractions. Full renumbering of all sections to accommodate the new foundational terms. Updated Document Structure, Table of Contents, and all cross-references. |
```
