# Vora Mind — Box Anatomy

**Version:** 1.0.0
**Status:** ACTIVE
**Effective From:** 2026-10-01
**Owner:** Alex Fedura

---

## 1. Overview

Every box (RBR / DBD) is defined by **4 corners** and **4 edges**.
This document fixes the canonical naming used across:

- Rust: `Box` struct, `boxes.rs`, `db.rs`, `ipc.rs`
- TypeScript: `BoxDto`, `BoxOverlay.ts`
- UI: Policy Panel labels
- Docs: all `docs/*.md`
- Conversation: issue reports, prompts, reviews

Do NOT invent alternate names. Use these exact tokens.

---

## 2. The 4 Corners

```
      A ●━━━━━━━━━━━━━━━━━● B
        ┃                 ┃
        ┃                 ┃
        ┃                 ┃
        ┃                 ┃
        ┃                 ┃
      C ●━━━━━━━━━━━━━━━━━● D
```

| Letter | Name        | Position      | Coordinates           |
|--------|-------------|---------------|-----------------------|
| **A**  | `TopLeft`   | بالا-چپ       | `(ts_start, high)`    |
| **B**  | `TopRight`  | بالا-راست     | `(ts_end,   high)`    |
| **C**  | `BottomLeft`| پایین-چپ      | `(ts_start, low)`     |
| **D**  | `BottomRight`| پایین-راست   | `(ts_end,   low)`     |

**Invariants:**
- `A.x == C.x == ts_start`
- `B.x == D.x == ts_end`
- `A.y == B.y == high`
- `C.y == D.y == low`

---

## 3. The 4 Edges

| Edge   | Name       | From → To | Field                    | Mutable? |
|--------|------------|-----------|--------------------------|----------|
| **AB** | `Top`      | A → B     | `high` (price)           | NO       |
| **BD** | `Right`    | B → D     | `ts_end` (time)          | **YES**  |
| **CD** | `Bottom`   | D → C     | `low` (price)            | NO       |
| **AC** | `Left`     | C → A     | `ts_start` (time)        | **YES**  |

**Direction:**
- `Top`    : left → right (A → B)
- `Right`  : top → bottom (B → D)
- `Bottom` : right → left (D → C)
- `Left`   : bottom → top (C → A)

Direction matters for rendering (stroke order) and any future animation.

**Mutability:**
- `Left`  edge moves with `box_extend.left`
- `Right` edge moves with `box_extend.right`
- `Top` and `Bottom` are FIXED to `high` / `low` of the base
  and are never extended by policy.

---

## 4. Diagonals (Reserved)

Two diagonals exist geometrically but are NOT used:

| Diag   | From → To | Purpose |
|--------|-----------|---------|
| **AD** | A → D     | Reserved (unused) |
| **BC** | B → C     | Reserved (unused) |

Do NOT reference diagonals in code, docs, or UI.

---

## 5. Relation to Box Struct Fields

| Anatomy token     | Rust field  | TS field   |
|-------------------|-------------|------------|
| `A.x` / `C.x`     | `ts_start`  | `ts_start` |
| `B.x` / `D.x`     | `ts_end`    | `ts_end`   |
| `A.y` / `B.y`     | `high`      | `high`     |
| `C.y` / `D.y`     | `low`       | `low`      |
| `Left` edge       | `ts_start`  | `ts_start` |
| `Right` edge      | `ts_end`    | `ts_end`   |
| `Top` edge        | `high`      | `high`     |
| `Bottom` edge     | `low`       | `low`      |

No field is renamed. This document defines the SEMANTIC aliases.

---

## 6. Relation to Patterns

| Pattern | A / C x       | B / D x      | Top (high)  | Bottom (low) |
|---------|---------------|--------------|-------------|--------------|
| **RBR** | `H1_idx`      | `HL_idx`     | `H1` price  | `HL` price   |
| **DBD** | `L1_idx`      | `LH_idx`     | `LH` price  | `L1` price   |

Note: `A` is the START of the base and `B` is the END of the base.
For RBR, base = H1 → HL. For DBD, base = L1 → LH.

---

## 7. Relation to Extend Policy

```
Policy field             Anatomy target   Affects
───────────────────────  ──────────────   ────────────────────────
box_extend.left          Left edge (AC)   ts_start (fractional candles)
box_extend.right         Right edge (BD)  ts_end   (fractional candles)
```

- Both fields accept fractional candles (`-1.5`, `+0.25`, ...).
- Range: `-20.0 .. +20.0`.
- Default: `0.0` (both edges exactly on base).
- Extend does NOT modify `Top` or `Bottom`.

---

## 8. Relation to Engulf Zone

The `engulf_ts` field (if present on a Box) is the timestamp of
the invalidation candle. It is rendered as an extension of the
Right edge:

```
      A ●━━━━━━━━━● B ┄┄┄┄┄┄┄┄┄┄┄┄┄● E
        ┃         ┃              ┆
        ┃         ┃              ┆   ← dashed (engulf zone)
        ┃         ┃              ┆
      C ●━━━━━━━━━● D ┄┄┄┄┄┄┄┄┄┄┄┄┄● F
```

- `E` = `(engulf_ts, high)`
- `F` = `(engulf_ts, low)`
- Segment `B → E` and `D → F` are drawn dashed, same color,
  lower opacity (0.4).
- The segment `A-B-D-C-A` remains solid.
- Toggle: `policy.show_engulf_zone` (default: true).

The engulf zone is READ-ONLY. It is not extended by `box_extend`.

---

## 9. Examples

### RBR
```
A = (ts[H1_idx], H1.price)
B = (ts[HL_idx], H1.price)
C = (ts[H1_idx], HL.price)
D = (ts[HL_idx], HL.price)
```

### DBD
```
A = (ts[L1_idx], LH.price)
B = (ts[LH_idx], LH.price)
C = (ts[L1_idx], L1.price)
D = (ts[LH_idx], L1.price)
```

### With extend.left = -1.5, extend.right = +2.0
```
A.x = ts[H1_idx] - 1.5 * period(M)
B.x = ts[HL_idx] + 2.0 * period(M)
C.x = A.x
D.x = B.x
```

---

## 10. Change Log

| Version | Date       | Changes |
|---------|------------|---------|
| 1.0.0   | 2026-10-01 | Initial anatomy spec: 4 corners (A/B/C/D), 4 edges (Top/Right/Bottom/Left), 2 reserved diagonals. |

---

**End of Box Anatomy v1.0.0**
