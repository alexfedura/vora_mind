use super::model::*;
use rusqlite::{Connection, OptionalExtension};

pub fn init_schema(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch(include_str!("../../schema_market.sql"))?;
    conn.execute_batch("DROP TABLE IF EXISTS market_sources;")?;
    for (column, definition) in [
        ("notify_app", "INTEGER NOT NULL DEFAULT 1"),
        ("notify_toast", "INTEGER NOT NULL DEFAULT 1"),
        ("notify_email", "INTEGER NOT NULL DEFAULT 0"),
        ("notify_sound", "INTEGER NOT NULL DEFAULT 0"),
    ] {
        let exists = conn
            .prepare("PRAGMA table_info(chart_drawing_alerts)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(Result::ok)
            .any(|name| name == column);
        if !exists {
            conn.execute_batch(&format!(
                "ALTER TABLE chart_drawing_alerts ADD COLUMN {column} {definition}"
            ))?;
        }
    }
    // v5.1.10: ensure universal columns (fresh DBs come from schema file).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS market_boxes (
            address TEXT PRIMARY KEY, symbol TEXT NOT NULL, step INTEGER NOT NULL,
            pattern TEXT NOT NULL, zone TEXT NOT NULL, scale INTEGER NOT NULL,
            x INTEGER NOT NULL, y_abs INTEGER NOT NULL, z INTEGER NOT NULL, y_band INTEGER NOT NULL,
            x_idx INTEGER NOT NULL,
            ts_start INTEGER NOT NULL, ts_end INTEGER NOT NULL,
            high REAL NOT NULL, low REAL NOT NULL, origin_price REAL NOT NULL,
            engulf_ts INTEGER NULL,
            duration_sec INTEGER NOT NULL DEFAULT 0,
            scale_sec INTEGER NOT NULL DEFAULT 0,
            pivot_step INTEGER NULL,
            chain_id INTEGER NULL, chain_pos INTEGER NULL
         );
         CREATE INDEX IF NOT EXISTS idx_market_boxes_symbol_m ON market_boxes(symbol, step);
         CREATE INDEX IF NOT EXISTS idx_market_boxes_chain ON market_boxes(chain_id);",
    )?;
    // v6.5.0 migration: add duration_sec column if missing (idempotent).
    let has_duration: bool = {
        let mut stmt = conn.prepare("PRAGMA table_info(market_boxes)")?;
        let mut rows = stmt.query([])?;
        let mut found = false;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == "duration_sec" {
                found = true;
                break;
            }
        }
        found
    };
    if !has_duration {
        conn.execute(
            "ALTER TABLE market_boxes ADD COLUMN duration_sec INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    let has_scale_sec: bool = {
        let mut stmt = conn.prepare("PRAGMA table_info(market_boxes)")?;
        let mut rows = stmt.query([])?;
        let mut found = false;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == "scale_sec" {
                found = true;
                break;
            }
        }
        found
    };
    if !has_scale_sec {
        conn.execute(
            "ALTER TABLE market_boxes ADD COLUMN scale_sec INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    // Idempotent column drops for DBs created with legacy y_level/band_size.
    let _ = conn.execute_batch("ALTER TABLE market_boxes DROP COLUMN y_level;");
    let _ = conn.execute_batch("ALTER TABLE market_boxes DROP COLUMN band_size;");
    // v5.3.0: pivot_step on pre-existing tables (idempotent)
    let _ = conn.execute_batch(
        "ALTER TABLE market_boxes ADD COLUMN pivot_step INTEGER NOT NULL DEFAULT 3;",
    );
    // v5.1.9.1: engulf_ts on pre-existing tables (idempotent, ignore if exists)
    let _ = conn.execute_batch("ALTER TABLE market_boxes ADD COLUMN engulf_ts INTEGER;");
    // Idempotent column migration for DBs created before first_extreme.
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(market_candles)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .filter_map(Result::ok)
        .collect();
    if !cols.iter().any(|c| c == "first_extreme") {
        conn.execute(
            "ALTER TABLE market_candles ADD COLUMN first_extreme TEXT NOT NULL DEFAULT 'same'",
            [],
        )?;
    }
    // v5.1.8: scale_registry table (also created via schema_market.sql).
    // v6.4.9: global registry — PK = symbol only.
    migrate_global_scale_registry(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS scale_registry (
            symbol        TEXT    PRIMARY KEY,
            registry_json TEXT    NOT NULL,
            updated_at    INTEGER NOT NULL
          );",
    )?;
    // v5.1.10: meta + symbol_origin (BEFORE wipe check).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _vora_meta (
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
         );",
    )?;
    // v5.3.0: wipe market_boxes once (address now includes P{pivot_step}).
    for marker in ["v5_1_10_wipe_done", "v5_3_0_wipe_done"] {
        let done: bool = conn
            .query_row(
                "SELECT 1 FROM _vora_meta WHERE key = ?1",
                rusqlite::params![marker],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !done {
            match conn.execute("DELETE FROM market_boxes", []) {
                Ok(_) => {
                    let _ = conn.execute(
                        "INSERT OR REPLACE INTO _vora_meta (key, value)
                         VALUES (?1, ?2)",
                        rusqlite::params![marker, "1"],
                    );
                }
                Err(_) => { /* leave marker unset; retry next run */ }
            }
        }
    }
    // v6.3.0b: rename m_scale/pivot_m columns (idempotent, fresh + old DBs).
    const MIGRATION_KEY_V630B: &str = "v6_3_0b_column_rename";
    let done_b: Option<String> = conn
        .query_row(
            "SELECT value FROM _vora_meta WHERE key = ?1",
            [MIGRATION_KEY_V630B],
            |r| r.get(0),
        )
        .optional()
        .unwrap_or(None);
    if done_b.is_none() {
        let _ = conn.execute("ALTER TABLE market_boxes RENAME COLUMN m_scale TO step", []);
        let _ = conn.execute(
            "ALTER TABLE market_boxes RENAME COLUMN pivot_m TO pivot_step",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE symbol_origin RENAME COLUMN m_scale TO step",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE scale_registry RENAME COLUMN m_scale TO step",
            [],
        );
        conn.execute(
            "INSERT OR REPLACE INTO _vora_meta(key, value) VALUES(?1, '1')",
            [MIGRATION_KEY_V630B],
        )
        .ok();
        eprintln!("[v6.3.0b] DB: renamed m_scale/pivot_m columns.");
    }
    // v6.4.0: idempotent scale_registry m_scale → step rename (belt-and-braces).
    const KEY_V640: &str = "v6_4_0_scale_registry_rename";
    let done: Option<String> = conn
        .query_row(
            "SELECT value FROM _vora_meta WHERE key = ?1",
            [KEY_V640],
            |r| r.get(0),
        )
        .optional()
        .unwrap_or(None);
    if done.is_none() {
        let _ = conn.execute(
            "ALTER TABLE scale_registry RENAME COLUMN m_scale TO step",
            [],
        );
        conn.execute(
            "INSERT OR REPLACE INTO _vora_meta(key, value) VALUES(?1, '1')",
            [KEY_V640],
        )
        .ok();
    }
    // v6.3.0c: ×5 ladder — wipe stale 10^M M-level candles once (idempotent).
    const MIGRATION_KEY_V630C: &str = "v6_3_0c_ladder_math";
    let done_c: Option<String> = conn
        .query_row(
            "SELECT value FROM _vora_meta WHERE key = ?1",
            [MIGRATION_KEY_V630C],
            |r| r.get(0),
        )
        .optional()
        .unwrap_or(None);
    if done_c.is_none() {
        match conn.execute("DELETE FROM market_candles WHERE timeframe LIKE 'M%'", []) {
            Ok(n) => eprintln!("[v6.3.0c] wiped {} M-level candles", n),
            Err(e) => eprintln!("[v6.3.0c] candle wipe skipped: {e}"),
        }
        conn.execute(
            "INSERT OR REPLACE INTO _vora_meta(key, value) VALUES(?1, '1')",
            [MIGRATION_KEY_V630C],
        )
        .ok();
    }
    Ok(())
}

/// Public schema entrypoint: run CREATE TABLE IF NOT EXISTS + idempotent
/// migrations. Safe to call on every open (existing or fresh DB).
pub fn ensure_schema(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    init_schema(conn)
}

fn extreme_to_str(fe: ExtremeKind) -> &'static str {
    match fe {
        ExtremeKind::HighFirst => "high_first",
        ExtremeKind::LowFirst => "low_first",
        ExtremeKind::Same => "same",
    }
}

fn extreme_from_str(s: &str) -> ExtremeKind {
    match s {
        "high_first" => ExtremeKind::HighFirst,
        "low_first" => ExtremeKind::LowFirst,
        _ => ExtremeKind::Same,
    }
}

pub fn save_boxes(
    conn: &Connection,
    symbol: &str,
    step: u8,
    boxes: &[super::boxes::Box],
    _chains: &[super::chain::Chain],
) -> Result<(), Box<dyn std::error::Error>> {
    use super::boxes::{BoxPattern, BoxZone};
    conn.execute(
        "DELETE FROM market_boxes WHERE symbol = ?1 AND step = ?2",
        rusqlite::params![symbol, step as i64],
    )?;
    let mut stmt = conn.prepare(
        "INSERT OR REPLACE INTO market_boxes
         (address, symbol, step, pivot_step, pattern, zone, scale, x, y_abs, z, y_band,
          x_idx, ts_start, ts_end, high, low, origin_price, engulf_ts, duration_sec, scale_sec,
          chain_id, chain_pos)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",
    )?;
    for b in boxes {
        let pat = match b.pattern {
            BoxPattern::RBR => "RBR",
            BoxPattern::DBD => "DBD",
            BoxPattern::RBD => "RBD",
            BoxPattern::DBR => "DBR",
        };
        let zone = match b.zone {
            BoxZone::Supply => "supply",
            BoxZone::Demand => "demand",
        };
        stmt.execute(rusqlite::params![
            b.address,
            b.symbol,
            step as i64,
            b.pivot_step as i64,
            pat,
            zone,
            b.scale as i64,
            b.x,
            b.y_abs,
            b.z as i64,
            b.y_band,
            b.x_idx,
            b.ts_start,
            b.ts_end,
            b.high,
            b.low,
            b.origin_price,
            b.engulf_ts,
            b.duration_sec as i64,
            b.scale_sec as i64,
            b.chain_id,
            b.chain_pos.map(|v| v as i64),
        ])?;
    }
    Ok(())
}

// ─── v5.1.10: symbol_origin cache ───

pub fn get_symbol_origin(
    conn: &Connection,
    symbol: &str,
    step: u8,
) -> Result<Option<(f64, i64)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT price_ref, price_ref_ts FROM symbol_origin
                  WHERE symbol = ?1 AND step = ?2",
        )
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query(rusqlite::params![symbol, step as i64])
        .map_err(|e| e.to_string())?;
    if let Some(r) = rows.next().map_err(|e| e.to_string())? {
        let p: f64 = r.get(0).map_err(|e| e.to_string())?;
        let ts: i64 = r.get(1).map_err(|e| e.to_string())?;
        Ok(Some((p, ts)))
    } else {
        Ok(None)
    }
}

pub fn save_symbol_origin(
    conn: &Connection,
    symbol: &str,
    step: u8,
    price_ref: f64,
    price_ref_ts: i64,
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    conn.execute(
        "INSERT OR REPLACE INTO symbol_origin
           (symbol, step, price_ref, price_ref_ts, computed_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![symbol, step as i64, price_ref, price_ref_ts, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// v6.4.9: migrate scale_registry from PK (symbol, step) to PK (symbol).
// SAFE: PRAGMA-checks for the old `step` column; handles fresh DB, old DB,
// and already-migrated DB; idempotent via _vora_meta marker; keeps the
// lowest-step row per symbol.
fn migrate_global_scale_registry(conn: &Connection) {
    const KEY: &str = "v6_4_9_global_scale_registry";
    let done: Option<String> = conn
        .query_row("SELECT value FROM _vora_meta WHERE key = ?1", [KEY], |r| {
            r.get::<_, String>(0)
        })
        .optional()
        .unwrap_or(None);
    if done.is_some() {
        return;
    }
    // Ensure _vora_meta exists so the marker can be written on a fresh DB
    // where the schema file hasn't created it yet.
    let _ = conn
        .execute_batch("CREATE TABLE IF NOT EXISTS _vora_meta (key TEXT PRIMARY KEY, value TEXT);");

    // Check if scale_registry exists at all.
    let exists: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='scale_registry'",
            [],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .unwrap_or(None);
    if exists.is_none() {
        conn.execute(
            "INSERT OR REPLACE INTO _vora_meta(key, value) VALUES(?1, '1')",
            [KEY],
        )
        .ok();
        return;
    }

    // Check if it still has the OLD `step` column via PRAGMA.
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(scale_registry)")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_map([], |r| r.get::<_, String>(1))
                .ok()
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();
    let has_step = cols.iter().any(|c| c == "step");

    if has_step {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS scale_registry_new (
                symbol TEXT PRIMARY KEY,
                registry_json TEXT NOT NULL,
                updated_at INTEGER NOT NULL
             );
             INSERT OR IGNORE INTO scale_registry_new
                 (symbol, registry_json, updated_at)
             SELECT symbol, registry_json, updated_at
             FROM scale_registry
             ORDER BY step ASC;
             DROP TABLE scale_registry;
             ALTER TABLE scale_registry_new
                 RENAME TO scale_registry;",
        )
        .ok();
        eprintln!("[v6.4.9] migrated scale_registry → PK=symbol");
    }

    conn.execute(
        "INSERT OR REPLACE INTO _vora_meta(key, value) VALUES(?1, '1')",
        [KEY],
    )
    .ok();
}

// ─── ScaleRegistry (v5.1.8; v6.4.9: global per-symbol) ───

pub fn get_scale_registry(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<super::scales::ScaleRegistry>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare("SELECT registry_json FROM scale_registry WHERE symbol = ?1")?;
    let mut rows = stmt.query(rusqlite::params![symbol])?;
    if let Some(row) = rows.next()? {
        let json: String = row.get(0)?;
        let reg: super::scales::ScaleRegistry = serde_json::from_str(&json)?;
        Ok(Some(reg))
    } else {
        Ok(None)
    }
}

pub fn save_scale_registry(
    conn: &Connection,
    reg: &super::scales::ScaleRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(reg)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    conn.execute(
        "INSERT OR REPLACE INTO scale_registry
           (symbol, registry_json, updated_at)
         VALUES (?1, ?2, ?3)",
        rusqlite::params![reg.symbol, json, now],
    )?;
    Ok(())
}

pub fn list_scale_registries(
    conn: &Connection,
) -> Result<Vec<(String, Option<u8>, i64)>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT symbol, updated_at FROM scale_registry
         ORDER BY symbol",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, None, r.get::<_, i64>(1)?))
    })?;
    let mut out: Vec<(String, Option<u8>, i64)> = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn save_drawing_template(
    conn: &Connection,
    template: &DrawingStyleTemplate,
) -> Result<(), Box<dyn std::error::Error>> {
    let template_json = serde_json::to_string(template)?;
    conn.execute(
                "INSERT INTO chart_drawing_templates (id, name, drawing_type, template_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(name) DO UPDATE SET id = excluded.id, drawing_type = excluded.drawing_type,
                     template_json = excluded.template_json, created_at = excluded.created_at",
        rusqlite::params![template.id, template.name, template.drawing_type.as_str(), template_json, template.created_at],
    )?;
    Ok(())
}

pub fn list_drawing_templates(
    conn: &Connection,
) -> Result<Vec<DrawingStyleTemplate>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT template_json FROM chart_drawing_templates ORDER BY name COLLATE NOCASE",
    )?;
    let json_rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut templates = Vec::new();
    for json in json_rows {
        templates.push(serde_json::from_str(&json?)?);
    }
    Ok(templates)
}

pub fn delete_drawing_template(conn: &Connection, id: &str) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM chart_drawing_templates WHERE id = ?1", [id])?;
    Ok(())
}

pub fn save_chart_drawing_alert(
    conn: &Connection,
    alert: &ChartDrawingAlert,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO chart_drawing_alerts
         (id, symbol, timeframe, drawing_id, condition, message, trigger_once, notify_app, notify_toast, notify_email, notify_sound, expiration_at, enabled, created_at, last_evaluated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        rusqlite::params![
            alert.id,
            alert.symbol,
            alert.timeframe,
            alert.drawing_id,
            alert.condition.as_str(),
            alert.message,
            i64::from(alert.trigger_once),
            i64::from(alert.notify_app),
            i64::from(alert.notify_toast),
            i64::from(alert.notify_email),
            i64::from(alert.notify_sound),
            alert.expiration_at,
            i64::from(alert.enabled),
            alert.created_at,
            alert.last_evaluated_at,
        ],
    )?;
    Ok(())
}

pub fn list_chart_drawing_alerts(
    conn: &Connection,
    symbol: &str,
) -> Result<Vec<ChartDrawingAlert>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT id, symbol, timeframe, drawing_id, condition, message, trigger_once, notify_app, notify_toast, notify_email, notify_sound, expiration_at, enabled, created_at, last_evaluated_at
         FROM chart_drawing_alerts WHERE symbol = ?1 ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([symbol], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, i64>(6)? != 0,
            row.get::<_, i64>(7)? != 0,
            row.get::<_, i64>(8)? != 0,
            row.get::<_, i64>(9)? != 0,
            row.get::<_, i64>(10)? != 0,
            row.get::<_, Option<i64>>(11)?,
            row.get::<_, i64>(12)? != 0,
            row.get::<_, i64>(13)?,
            row.get::<_, Option<i64>>(14)?,
        ))
    })?;
    let mut alerts = Vec::new();
    for row in rows {
        let (
            id,
            symbol,
            timeframe,
            drawing_id,
            condition,
            message,
            trigger_once,
            notify_app,
            notify_toast,
            notify_email,
            notify_sound,
            expiration_at,
            enabled,
            created_at,
            last_evaluated_at,
        ) = row?;
        let condition = match condition.as_str() {
            "entering_drawing" => DrawingAlertCondition::EnteringDrawing,
            "crossing" => DrawingAlertCondition::Crossing,
            other => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown drawing alert condition: {other}"),
                )
                .into())
            }
        };
        alerts.push(ChartDrawingAlert {
            id,
            symbol,
            timeframe,
            drawing_id,
            condition,
            message,
            trigger_once,
            notify_app,
            notify_toast,
            notify_email,
            notify_sound,
            expiration_at,
            enabled,
            created_at,
            last_evaluated_at,
        });
    }
    Ok(alerts)
}

pub fn update_chart_drawing_alert(
    conn: &Connection,
    id: &str,
    enabled: bool,
    last_evaluated_at: Option<i64>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE chart_drawing_alerts SET enabled = ?2, last_evaluated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, i64::from(enabled), last_evaluated_at],
    )?;
    Ok(())
}

pub fn delete_chart_drawing_alert(conn: &Connection, id: &str) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM chart_drawing_alerts WHERE id = ?1", [id])?;
    Ok(())
}

pub fn delete_chart_drawing_alerts_for_drawings(
    conn: &mut Connection,
    symbol: &str,
    drawing_ids: &[String],
) -> Result<usize, rusqlite::Error> {
    if drawing_ids.is_empty() {
        return Ok(0);
    }
    let transaction = conn.transaction()?;
    let mut deleted = 0;
    {
        let mut statement = transaction.prepare_cached(
            "DELETE FROM chart_drawing_alerts WHERE drawing_id = ?1 AND symbol = ?2",
        )?;
        for drawing_id in drawing_ids {
            deleted += statement.execute(rusqlite::params![drawing_id, symbol])?;
        }
    }
    transaction.commit()?;
    Ok(deleted)
}

pub fn upsert_candles(
    conn: &Connection,
    candles: &[Candle],
) -> Result<usize, Box<dyn std::error::Error>> {
    if candles.is_empty() {
        return Ok(0);
    }

    let mut stmt = conn.prepare_cached(
        "INSERT INTO market_candles
           (symbol, timeframe, ts, open, high, low, close, volume, tick_count, first_extreme)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(symbol, timeframe, ts) DO UPDATE SET
           open = excluded.open,
           high = excluded.high,
           low = excluded.low,
           close = excluded.close,
           volume = excluded.volume,
           tick_count = excluded.tick_count,
           first_extreme = excluded.first_extreme",
    )?;

    let mut count = 0;
    for c in candles {
        stmt.execute(rusqlite::params![
            c.symbol,
            c.timeframe,
            c.ts,
            c.open,
            c.high,
            c.low,
            c.close,
            c.volume,
            c.tick_count,
            extreme_to_str(c.first_extreme),
        ])?;
        count += 1;
    }

    Ok(count)
}

/// Batched insert/replace of candles inside ONE transaction.
/// Use for bulk rebuilds (reaggregate) to avoid per-row disk thrash.
pub fn replace_candles(
    conn: &mut Connection,
    symbol: &str,
    tf: &str,
    candles: &[Candle],
) -> Result<usize, Box<dyn std::error::Error>> {
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM market_candles WHERE symbol = ?1 AND timeframe = ?2",
        rusqlite::params![symbol, tf],
    )?;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO market_candles
               (symbol, timeframe, ts, open, high, low, close, volume, tick_count, first_extreme)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )?;
        for c in candles {
            stmt.execute(rusqlite::params![
                c.symbol,
                c.timeframe,
                c.ts,
                c.open,
                c.high,
                c.low,
                c.close,
                c.volume,
                c.tick_count,
                extreme_to_str(c.first_extreme),
            ])?;
        }
    }
    tx.commit()?;
    Ok(candles.len())
}

pub fn count_candles(
    conn: &Connection,
    symbol: &str,
    tf: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM market_candles WHERE symbol = ?1 AND timeframe = ?2",
        rusqlite::params![symbol, tf],
        |r| r.get(0),
    )?;
    Ok(count)
}

pub fn range_candles(
    conn: &Connection,
    symbol: &str,
    tf: &str,
    from_ts: i64,
    to_ts: i64,
    limit: i64,
) -> Result<Vec<Candle>, Box<dyn std::error::Error>> {
    // v0.83.9 — root-cause fix: `ORDER BY ts ASC LIMIT` returned the OLDEST
    // N candles, so every TF window anchored at the first-ever candle.
    // - from_ts == 0 means "give me recent data": return the LATEST N
    //   (DESC + reverse to keep chronological output).
    // - from_ts > 0 means a specific window: if the window holds more than
    //   `limit` rows, return the MOST RECENT part (DESC + reverse),
    //   else the fast ASC path.
    let map_row = |r: &rusqlite::Row| {
        let fe_str: String = r.get(9).unwrap_or_else(|_| "same".to_string());
        Ok(Candle {
            symbol: r.get(0)?,
            timeframe: r.get(1)?,
            ts: r.get(2)?,
            open: r.get(3)?,
            high: r.get(4)?,
            low: r.get(5)?,
            close: r.get(6)?,
            volume: r.get(7)?,
            tick_count: r.get(8)?,
            first_extreme: extreme_from_str(&fe_str),
        })
    };

    const SQL_DESC: &str =
        "SELECT symbol, timeframe, ts, open, high, low, close, volume, tick_count, first_extreme
         FROM market_candles
         WHERE symbol = ?1 AND timeframe = ?2 AND ts >= ?3 AND ts <= ?4
         ORDER BY ts DESC
         LIMIT ?5";
    const SQL_ASC: &str =
        "SELECT symbol, timeframe, ts, open, high, low, close, volume, tick_count, first_extreme
         FROM market_candles
         WHERE symbol = ?1 AND timeframe = ?2 AND ts >= ?3 AND ts <= ?4
         ORDER BY ts ASC
         LIMIT ?5";
    // from_ts == 0 path: no lower bound; latest N at or below to_ts.
    const SQL_RECENT: &str =
        "SELECT symbol, timeframe, ts, open, high, low, close, volume, tick_count, first_extreme
         FROM market_candles
         WHERE symbol = ?1 AND timeframe = ?2 AND ts <= ?3
         ORDER BY ts DESC
         LIMIT ?4";

    if from_ts == 0 {
        let mut stmt = conn.prepare(SQL_RECENT)?;
        let rows = stmt.query_map(rusqlite::params![symbol, tf, to_ts, limit], map_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        out.reverse();
        return Ok(out);
    }

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM market_candles WHERE symbol = ?1 AND timeframe = ?2 AND ts >= ?3 AND ts <= ?4",
        rusqlite::params![symbol, tf, from_ts, to_ts],
        |r| r.get(0),
    )?;
    if count > limit {
        let mut stmt = conn.prepare(SQL_DESC)?;
        let rows = stmt.query_map(
            rusqlite::params![symbol, tf, from_ts, to_ts, limit],
            map_row,
        )?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        out.reverse();
        Ok(out)
    } else {
        let mut stmt = conn.prepare(SQL_ASC)?;
        let rows = stmt.query_map(
            rusqlite::params![symbol, tf, from_ts, to_ts, limit],
            map_row,
        )?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn init_schema_drops_legacy_market_sources_table() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE market_sources (id TEXT PRIMARY KEY);")
            .unwrap();

        init_schema(&conn).unwrap();

        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'market_sources')",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(!exists);
    }

    #[test]
    fn drawing_templates_and_alerts_round_trip() {
        let conn = mem_conn();
        let template = DrawingStyleTemplate {
            id: "template-1".into(),
            name: "Default".into(),
            drawing_type: DrawingKind::Rectangle,
            settings: DrawingStyleTemplateSettings {
                color: "#ffb74d".into(),
                color_opacity: 0.8,
                line_width: 2.0,
                style: DrawingLineStyle::Dashed,
                fill_color: Some("#9c27b0".into()),
                fill_opacity: 0.25,
                text: "zone".into(),
                text_color: "#ffffff".into(),
                text_opacity: 0.9,
                text_size: 14.0,
                text_bold: true,
                text_italic: false,
                text_align_horizontal: DrawingTextAlign::Center,
                text_align_vertical: DrawingTextVerticalAlign::Top,
            },
            created_at: 100,
        };
        save_drawing_template(&conn, &template).unwrap();
        let mut updated_template = template.clone();
        updated_template.id = "template-2".into();
        updated_template.settings.line_width = 4.0;
        save_drawing_template(&conn, &updated_template).unwrap();
        let templates = list_drawing_templates(&conn).unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].id, "template-2");
        assert_eq!(templates[0].settings.line_width, 4.0);

        let alert = ChartDrawingAlert {
            id: "alert-1".into(),
            symbol: "XAUUSD".into(),
            timeframe: 3,
            drawing_id: "drawing-1".into(),
            condition: DrawingAlertCondition::EnteringDrawing,
            message: "Price entered zone".into(),
            trigger_once: true,
            notify_app: true,
            notify_toast: true,
            notify_email: false,
            notify_sound: false,
            expiration_at: None,
            enabled: true,
            created_at: 100,
            last_evaluated_at: Some(90),
        };
        save_chart_drawing_alert(&conn, &alert).unwrap();
        let alerts = list_chart_drawing_alerts(&conn, "XAUUSD").unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].condition, DrawingAlertCondition::EnteringDrawing);
        assert_eq!(alerts[0].last_evaluated_at, Some(90));
        update_chart_drawing_alert(&conn, "alert-1", false, Some(110)).unwrap();
        let updated = list_chart_drawing_alerts(&conn, "XAUUSD").unwrap();
        assert!(!updated[0].enabled);
        assert_eq!(updated[0].last_evaluated_at, Some(110));
        delete_chart_drawing_alert(&conn, "alert-1").unwrap();
        assert!(list_chart_drawing_alerts(&conn, "XAUUSD")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn drawing_alert_schema_migration_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE chart_drawing_alerts (
                id TEXT PRIMARY KEY, symbol TEXT NOT NULL, timeframe INTEGER NOT NULL,
                drawing_id TEXT NOT NULL, condition TEXT NOT NULL, message TEXT NOT NULL,
                trigger_once INTEGER NOT NULL, expiration_at INTEGER, enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL, last_evaluated_at INTEGER
            );",
        ).unwrap();
        init_schema(&conn).unwrap();
        init_schema(&conn).unwrap();
        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(chart_drawing_alerts)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        for column in ["notify_app", "notify_toast", "notify_email", "notify_sound"] {
            assert!(columns.iter().any(|actual| actual == column));
        }
    }

    #[test]
    fn upsert_candles_is_idempotent() {
        let conn = mem_conn();
        let candles = vec![
            Candle {
                symbol: "XAUUSD".into(),
                timeframe: "1m".into(),
                ts: 1000,
                open: 2600.0,
                high: 2605.0,
                low: 2599.0,
                close: 2604.0,
                volume: 120.0,
                tick_count: 45,
                first_extreme: ExtremeKind::Same,
            },
            Candle {
                symbol: "XAUUSD".into(),
                timeframe: "1m".into(),
                ts: 1060,
                open: 2604.0,
                high: 2608.0,
                low: 2603.0,
                close: 2607.0,
                volume: 150.0,
                tick_count: 55,
                first_extreme: ExtremeKind::Same,
            },
        ];

        let inserted1 = upsert_candles(&conn, &candles).unwrap();
        assert_eq!(inserted1, 2);
        assert_eq!(count_candles(&conn, "XAUUSD", "1m").unwrap(), 2);

        let mut updated = candles.clone();
        updated[0].close = 2604.5;
        let inserted2 = upsert_candles(&conn, &updated).unwrap();
        assert_eq!(inserted2, 2);
        assert_eq!(count_candles(&conn, "XAUUSD", "1m").unwrap(), 2);

        let res = range_candles(&conn, "XAUUSD", "1m", 1000, 1000, 10).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].close, 2604.5);
    }

    #[test]
    fn range_query_orders_by_ts() {
        let conn = mem_conn();
        let candles = vec![
            Candle {
                symbol: "XAUUSD".into(),
                timeframe: "5m".into(),
                ts: 1300,
                open: 2610.0,
                high: 2615.0,
                low: 2609.0,
                close: 2614.0,
                volume: 200.0,
                tick_count: 70,
                first_extreme: ExtremeKind::Same,
            },
            Candle {
                symbol: "XAUUSD".into(),
                timeframe: "5m".into(),
                ts: 1000,
                open: 2600.0,
                high: 2605.0,
                low: 2599.0,
                close: 2604.0,
                volume: 120.0,
                tick_count: 45,
                first_extreme: ExtremeKind::Same,
            },
            Candle {
                symbol: "XAUUSD".into(),
                timeframe: "5m".into(),
                ts: 1600,
                open: 2614.0,
                high: 2620.0,
                low: 2612.0,
                close: 2618.0,
                volume: 250.0,
                tick_count: 85,
                first_extreme: ExtremeKind::Same,
            },
        ];

        upsert_candles(&conn, &candles).unwrap();

        let queried = range_candles(&conn, "XAUUSD", "5m", 1000, 2000, 10).unwrap();
        assert_eq!(queried.len(), 3);
        assert_eq!(queried[0].ts, 1000);
        assert_eq!(queried[1].ts, 1300);
        assert_eq!(queried[2].ts, 1600);

        let limited = range_candles(&conn, "XAUUSD", "5m", 1000, 2000, 2).unwrap();
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].ts, 1000);
        assert_eq!(limited[1].ts, 1300);
    }
}
