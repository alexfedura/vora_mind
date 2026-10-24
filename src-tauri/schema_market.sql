-- ═══════════════════════════════════════════════════════════
-- VORA MARKET -- derived OHLCV candles and market analysis tables
-- Timestamps are unix epoch SECONDS (INTEGER), UTC.
-- ═══════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS market_candles (
    symbol          TEXT NOT NULL,                  -- XAUUSD
    timeframe       TEXT NOT NULL,                  -- M1|M5|M15|H1|H4|D1
    ts              INTEGER NOT NULL,               -- candle OPEN time (unix sec)
    open            REAL NOT NULL,
    high            REAL NOT NULL,
    low             REAL NOT NULL,
    close           REAL NOT NULL,
    volume          REAL NOT NULL,
    tick_count      INTEGER NOT NULL,
    first_extreme   TEXT NOT NULL DEFAULT 'same',   -- high_first|low_first|same
    PRIMARY KEY (symbol, timeframe, ts)
);

-- Provenance lookup: every slice of a symbol, chronological.
-- Range scans across all timeframes for one symbol.
CREATE INDEX IF NOT EXISTS idx_market_candles_symbol_ts
    ON market_candles(symbol, ts);

-- Timeframe-wide scans (cross-symbol research / backtests).
-- The PK (symbol, timeframe, ts) already indexes the exact-tf path.
CREATE INDEX IF NOT EXISTS idx_market_candles_timeframe_ts
    ON market_candles(timeframe, ts);

-- ── v5.1.10: detected boxes + swap chains (universal Y-axis, wiped once) ──
-- ── v5.3.0: scale-aware detection (pivot_step) ──
CREATE TABLE IF NOT EXISTS market_boxes (
    address     TEXT PRIMARY KEY,
    symbol      TEXT NOT NULL,
    step     INTEGER NOT NULL,
    pivot_step     INTEGER NOT NULL DEFAULT 3,
    pattern     TEXT NOT NULL,
    zone        TEXT NOT NULL,
    scale       INTEGER NOT NULL,
    x           INTEGER NOT NULL,
    y_abs       INTEGER NOT NULL,
    z           INTEGER NOT NULL,
    y_band      INTEGER NOT NULL,
    x_idx       INTEGER NOT NULL,
    ts_start    INTEGER NOT NULL,
    ts_end      INTEGER NOT NULL,
    high        REAL NOT NULL,
    low         REAL NOT NULL,
    origin_price REAL NOT NULL,
    engulf_ts INTEGER NULL,
    duration_sec INTEGER NOT NULL DEFAULT 0,
    scale_sec INTEGER NOT NULL DEFAULT 0,
    chain_id    INTEGER NULL,
    chain_pos   INTEGER NULL
);
CREATE INDEX IF NOT EXISTS idx_market_boxes_symbol_m
    ON market_boxes(symbol, step);
CREATE INDEX IF NOT EXISTS idx_market_boxes_chain
    ON market_boxes(chain_id);

-- ── v5.1.10: universal Y-axis reference + one-time wipe marker ──
CREATE TABLE IF NOT EXISTS _vora_meta (
    key   TEXT PRIMARY KEY,
    value TEXT
);
CREATE TABLE IF NOT EXISTS symbol_origin (
    symbol        TEXT    NOT NULL,
    step       INTEGER NOT NULL,
    price_ref     REAL    NOT NULL,
    price_ref_ts  INTEGER NOT NULL,
    computed_at   INTEGER NOT NULL,
    PRIMARY KEY (symbol, step)
);

-- ── v5.1.8: scale registry (v6.4.9: global JSON blob per symbol) ──
CREATE TABLE IF NOT EXISTS scale_registry (
    symbol        TEXT    PRIMARY KEY,
    registry_json TEXT    NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS chart_drawing_templates (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE COLLATE NOCASE,
    drawing_type    TEXT NOT NULL,
    template_json   TEXT NOT NULL,
    created_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS chart_drawing_alerts (
    id                  TEXT PRIMARY KEY,
    symbol              TEXT NOT NULL,
    timeframe           INTEGER NOT NULL,
    drawing_id          TEXT NOT NULL,
    condition           TEXT NOT NULL,
    message             TEXT NOT NULL,
    trigger_once        INTEGER NOT NULL,
    notify_app          INTEGER NOT NULL DEFAULT 1,
    notify_toast        INTEGER NOT NULL DEFAULT 1,
    notify_email        INTEGER NOT NULL DEFAULT 0,
    notify_sound        INTEGER NOT NULL DEFAULT 0,
    expiration_at       INTEGER,
    enabled             INTEGER NOT NULL DEFAULT 1,
    created_at          INTEGER NOT NULL,
    last_evaluated_at   INTEGER
);
CREATE INDEX IF NOT EXISTS idx_chart_drawing_alerts_symbol_enabled
    ON chart_drawing_alerts(symbol, enabled, created_at DESC);
