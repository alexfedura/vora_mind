//! Tauri IPC layer for the market module.
//!
//! Exposes local historical candles and market analysis commands to the frontend.

use crate::market::timeframe::MAX_STEP;
use crate::market::{db, model::*};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── DTOs ─────────────────────────────────────────────────

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CandleDto {
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub tick_count: i64,
    pub first_extreme: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct DetectBoxesRequest {
    pub symbol: String,
    #[serde(alias = "mScale", alias = "m_scale")]
    pub step: u8,
    pub params: Option<crate::market::model::BoxParamsDto>,
}

// ── Helper ────────────────────────────────────────────────
fn err<E: std::fmt::Display>(e: E) -> String {
    format!("{}", e)
}

// ── Implementation functions (_impl) ──────────────────────

// ── Commands ──────────────────────────────────────────────

#[tauri::command]
pub async fn mkt_fetch_local_candles(
    app: tauri::AppHandle,
    symbol: String,
    interval_minutes: u32,
    start_ts: i64,
    end_ts: i64,
    limit: usize,
) -> Result<Vec<CandleDto>, String> {
    let data_dir = super::local_db::resolve_tick_data_dir(&app)?;
    tokio::task::spawn_blocking(move || {
        super::local_db::fetch_local_candles(
            data_dir,
            symbol,
            interval_minutes,
            start_ts,
            end_ts,
            limit,
        )
    })
    .await
    .map_err(|error| format!("Local candle query task failed: {error}"))?
}

/// 7. Detect boxes for (symbol, step). v5.1.10 returns { boxes, chains }
/// with universal Y-axis (price_ref from symbol_origin / REF_TS).
#[derive(Serialize, Clone, Debug)]
pub struct DetectBoxesResult {
    pub boxes: Vec<crate::market::model::BoxDto>,
    pub chains: Vec<crate::market::model::ChainDto>,
}

#[tauri::command]
pub fn mkt_cancel_request(request_id: u64) -> Result<(), String> {
    crate::market::cancel::global().cancel(request_id);
    Ok(())
}

#[tauri::command]
pub async fn mkt_detect_boxes(
    req: DetectBoxesRequest,
    request_id: Option<u64>,
    cancel_previous: Option<u64>,
) -> Result<DetectBoxesResult, String> {
    // v6.3.0g single-flight + latest-wins. ORDER IS CRITICAL:
    // set_latest → cancel_all → lock → latest-wins check → register.
    let registry = crate::market::cancel::registry();
    let my_id = request_id.unwrap_or(0);

    // 1. Mark me as the latest request.
    if my_id > 0 {
        crate::market::cancel::set_latest(my_id);
    }

    // 2. Cancel all currently registered tokens (previous work).
    registry.cancel_all();

    // 3. Explicitly cancel the previous request_id (defensive).
    if let Some(prev) = cancel_previous {
        registry.cancel(prev);
    }

    // 4. Single-flight: only one detect runs at a time.
    let _guard = crate::market::cancel::detect_lock().lock().await;

    // 5. Latest-wins: if a newer request arrived while we queued,
    //    bail instantly WITHOUT doing any work.
    if my_id > 0 && crate::market::cancel::latest() != my_id {
        return Err("cancelled".to_string());
    }

    // 6. Register my cancel token now (we are the running one).
    let token = registry.register(my_id);
    let token_opt: Option<Arc<std::sync::atomic::AtomicBool>> = Some(token.clone());

    // Heavy SYNC work runs in spawn_blocking (DB + detection loop).
    // Connection is opened INSIDE the closure (rusqlite is not Send across awaits).
    let result = tokio::task::spawn_blocking(move || detect_boxes_sync(req, my_id, token_opt))
        .await
        .map_err(|e| format!("join error: {e}"))?;

    // Cleanup in the outer async ctx.
    registry.remove(my_id);
    result
}

/// v6.5.1: choose the finest TF (M0..M4) whose candle count over the window
/// stays ≤ 5_000_000.
fn pick_base_tf(window_sec: i64) -> &'static str {
    ["M0", "M1", "M2", "M3", "M4"]
        .iter()
        .find(|&&tf| {
            let sec = crate::market::timeframe::step_from_key(tf)
                .map(|s| crate::market::timeframe::tf_seconds(s))
                .unwrap_or(1) as i64;
            window_sec / sec <= 5_000_000
        })
        .copied()
        .unwrap_or("M4")
}

/// Sync body of detect (runs inside spawn_blocking).
fn detect_boxes_sync(
    req: DetectBoxesRequest,
    request_id: u64,
    tok_opt: Option<Arc<std::sync::atomic::AtomicBool>>,
) -> Result<DetectBoxesResult, String> {
    // v6.3.0f: atomic cancel-prev + register-new.
    let finish = |r: Result<DetectBoxesResult, String>| -> Result<DetectBoxesResult, String> {
        crate::market::cancel::global().remove(request_id);
        r
    };
    let mut p = crate::market::boxes::Parameters::default();

    if let Some(op) = req.params {
        if let Some(v) = op.user_step {
            p.user_step = v.min(MAX_STEP);
        }
        if let Some(v) = op.z0_scale {
            p.z0_scale = v.min(15);
        }
        if let Some(v) = op.extend_to_engulf {
            p.extend_to_engulf = v;
        }

        if let (Some(s), Some(m), Some(l)) = (op.s_small, op.s_medium, op.s_large) {
            p.role_override = Some((s.min(15), m.min(15), l.min(15)));
        }
    }

    let conn = match crate::open_default_db_connection() {
        Ok(c) => c,
        Err(e) => return finish(Err(err(e))),
    };
    // v5.3.0: price_ref computed ONCE at user_step (chart M).
    // v5.3.2: high limit for user_step candles (was 100_000).
    let tf_user = crate::market::mscale::m_key(p.user_step as i32);
    let candles_user = match db::range_candles(&conn, &req.symbol, &tf_user, 0, i64::MAX, 500_000) {
        Ok(v) => v,
        Err(e) => return finish(Err(err(e))),
    };
    if candles_user.is_empty() {
        return finish(Ok(DetectBoxesResult {
            boxes: Vec::new(),
            chains: Vec::new(),
        }));
    }
    // v5.3.2: shared time window derived from user_step.
    let from_ts: i64 = candles_user.first().map(|c| c.ts).unwrap_or(0);
    let to_ts: i64 = candles_user.last().map(|c| c.ts).unwrap_or(i64::MAX);
    let (price_ref, _price_ref_ts) = match crate::market::boxes::resolve_price_ref(
        &conn,
        &req.symbol,
        p.user_step,
        &candles_user,
    ) {
        Ok(v) => v,
        Err(e) => return finish(Err(err(e))),
    };

    // v6.5.0: pivot-first — pick finest available TF (M0..M4) with ≤ 5M candles
    // over the shared window, load ONCE, detect ONCE.
    let window_sec: i64 = to_ts.saturating_sub(from_ts).max(1);

    let base_tf: &str = pick_base_tf(window_sec);
    if std::env::var("VORA_DEBUG_BOXES").is_ok() {
        eprintln!(
            "[pivot-first] base_tf={} window_sec={}",
            base_tf, window_sec
        );
    }

    let candles_base =
        match db::range_candles(&conn, &req.symbol, base_tf, from_ts, to_ts, 5_000_000) {
            Ok(v) => v,
            Err(e) => return finish(Err(err(e))),
        };
    if std::env::var("VORA_DEBUG_BOXES").is_ok() {
        eprintln!("[pivot-first] candles_base.len()={}", candles_base.len());
    }

    if candles_base.len() < 50 {
        return finish(Ok(DetectBoxesResult {
            boxes: Vec::new(),
            chains: Vec::new(),
        }));
    }

    if crate::market::cancel::is_cancelled(&tok_opt) {
        return finish(Err("cancelled".to_string()));
    }

    let threshold_pct = crate::market::boxes::scale_start_pct(0);
    let mut p_with = p;
    p_with.pivot_threshold_pct = Some(threshold_pct);

    let (mut all_boxes, _) = crate::market::boxes::detect_boxes_multiscale(
        &candles_base,
        p_with,
        &req.symbol,
        price_ref,
        tok_opt.clone(),
    );

    if crate::market::cancel::is_cancelled(&tok_opt) {
        return finish(Err("cancelled".to_string()));
    }

    if std::env::var("VORA_DEBUG_BOXES").is_ok() {
        eprintln!(
            "[pivot-first] raw boxes BEFORE snap/dedup = {}",
            all_boxes.len()
        );
    }

    // v6.4.11 snap preserved: snap ts + x_idx + address to user_step grid.
    {
        let user_sec = crate::market::timeframe::tf_seconds(p.user_step) as i64;
        for b in all_boxes.iter_mut() {
            let s_start = b.ts_start.div_euclid(user_sec) * user_sec;
            let s_end = b.ts_end.div_euclid(user_sec) * user_sec;
            b.ts_start = s_start;
            b.ts_end = s_end;
            if let Some(e) = b.engulf_ts {
                b.engulf_ts = Some(e.div_euclid(user_sec) * user_sec);
            }
            let x_idx = s_start.div_euclid(user_sec);
            b.x_idx = x_idx;
            b.x = x_idx;
            let pat = match b.pattern {
                crate::market::boxes::BoxPattern::RBR => "RBR",
                crate::market::boxes::BoxPattern::DBD => "DBD",
                crate::market::boxes::BoxPattern::RBD => "RBD",
                crate::market::boxes::BoxPattern::DBR => "DBR",
            };
            b.address = format!(
                "{}~M{}~P{}~X{}~Y{}~{}",
                b.symbol, b.step, b.pivot_step, b.x_idx, b.y_abs, pat
            );
        }
    }

    // Dedup (v5.3.2 preserved).
    all_boxes.sort_by(|a, b| {
        a.pivot_step
            .cmp(&b.pivot_step)
            .then(a.x.cmp(&b.x))
            .then(a.y_abs.cmp(&b.y_abs))
            .then(a.scale.cmp(&b.scale))
            .then(a.pattern.cmp(&b.pattern))
    });
    {
        let mut seen: std::collections::HashSet<(i64, i64, u8, u8)> =
            std::collections::HashSet::new();
        all_boxes.retain(|b| seen.insert((b.x, b.y_abs, b.scale, b.pattern as u8)));
    }
    if std::env::var("VORA_DEBUG_BOXES").is_ok() {
        eprintln!("[pivot-first] boxes AFTER dedup = {}", all_boxes.len());
    }

    let all_chains: Vec<crate::market::chain::Chain> = crate::market::chain::assign_chains(
        &mut all_boxes,
        &crate::market::chain::ChainParams::default(),
    );

    if let Err(e) = db::save_boxes(&conn, &req.symbol, p.user_step, &all_boxes, &all_chains) {
        return finish(Err(err(e)));
    }
    finish(Ok(DetectBoxesResult {
        boxes: all_boxes.into_iter().map(Into::into).collect(),
        chains: all_chains.iter().map(Into::into).collect(),
    }))
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SymbolOriginDto {
    pub symbol: String,
    #[serde(alias = "mScale", alias = "m_scale")]
    pub step: u8,
    pub price_ref: f64,
    pub price_ref_ts: i64,
}

#[tauri::command]
pub fn mkt_get_symbol_origin(symbol: String, step: u8) -> Result<Option<SymbolOriginDto>, String> {
    let conn = crate::open_default_db_connection().map_err(|e| e.to_string())?;
    match crate::market::db::get_symbol_origin(&conn, &symbol, step) {
        Ok(Some((p, ts))) => Ok(Some(SymbolOriginDto {
            symbol,
            step,
            price_ref: p,
            price_ref_ts: ts,
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn mkt_refresh_symbol_origin(symbol: String, step: u8) -> Result<SymbolOriginDto, String> {
    let conn = crate::open_default_db_connection().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM symbol_origin WHERE symbol = ?1 AND step = ?2",
        rusqlite::params![symbol, step as i64],
    )
    .map_err(|e| e.to_string())?;
    let tf = crate::market::mscale::m_key(step as i32);
    let candles = crate::market::db::range_candles(&conn, &symbol, &tf, 0, i64::MAX, 100_000)
        .map_err(|e| e.to_string())?;
    let (p, ts) = crate::market::boxes::resolve_price_ref(&conn, &symbol, step, &candles)?;
    Ok(SymbolOriginDto {
        symbol,
        step,
        price_ref: p,
        price_ref_ts: ts,
    })
}

// ── ScaleRegistry (v5.1.8; v6.4.9: global per-symbol) ──

pub fn get_scale_registry_impl(
    conn: &rusqlite::Connection,
    symbol: &str,
    _step: Option<u8>,
) -> Result<crate::market::scales::ScaleRegistry, String> {
    match db::get_scale_registry(conn, symbol).map_err(err)? {
        Some(reg) => Ok(reg),
        None => Ok(crate::market::scales::ScaleRegistry::default_for(symbol)),
    }
}

pub fn set_scale_registry_impl(
    conn: &rusqlite::Connection,
    registry: crate::market::scales::ScaleRegistry,
) -> Result<(), String> {
    let errors = registry.validate();
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    db::save_scale_registry(conn, &registry).map_err(err)
}

pub fn list_scale_registries_impl(
    conn: &rusqlite::Connection,
) -> Result<Vec<(String, Option<u8>, i64)>, String> {
    db::list_scale_registries(conn).map_err(err)
}

/// Get registry for symbol (v6.4.9: global; step kept for back-compat, ignored); falls back to default S0..S9.
#[tauri::command]
pub fn mkt_get_scale_registry(
    symbol: String,
    step: Option<u8>,
) -> Result<crate::market::scales::ScaleRegistry, String> {
    let conn = crate::open_default_db_connection().map_err(err)?;
    get_scale_registry_impl(&conn, &symbol, step)
}

/// Validate + persist registry.
#[tauri::command]
pub fn mkt_set_scale_registry(
    registry: crate::market::scales::ScaleRegistry,
) -> Result<(), String> {
    let errors = registry.validate();
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    let conn = crate::open_default_db_connection().map_err(err)?;
    db::save_scale_registry(&conn, &registry).map_err(err)
}

/// List all persisted (symbol, updated_at).
#[tauri::command]
pub fn mkt_list_scale_registries() -> Result<Vec<(String, Option<u8>, i64)>, String> {
    let conn = crate::open_default_db_connection().map_err(err)?;
    list_scale_registries_impl(&conn)
}

#[tauri::command]
pub fn mkt_list_drawing_templates() -> Result<Vec<DrawingStyleTemplate>, String> {
    let conn = crate::open_default_db_connection().map_err(err)?;
    db::list_drawing_templates(&conn).map_err(err)
}

#[tauri::command]
pub fn mkt_save_drawing_template(template: DrawingStyleTemplate) -> Result<(), String> {
    if template.name.trim().is_empty() {
        return Err("Drawing template name cannot be empty".into());
    }
    if !(1.0..=10.0).contains(&template.settings.line_width)
        || !(0.0..=1.0).contains(&template.settings.color_opacity)
        || !(0.0..=1.0).contains(&template.settings.fill_opacity)
        || !(0.0..=1.0).contains(&template.settings.text_opacity)
    {
        return Err("Drawing template settings are out of range".into());
    }
    let conn = crate::open_default_db_connection().map_err(err)?;
    db::save_drawing_template(&conn, &template).map_err(err)
}

#[tauri::command]
pub fn mkt_delete_drawing_template(id: String) -> Result<(), String> {
    let conn = crate::open_default_db_connection().map_err(err)?;
    db::delete_drawing_template(&conn, &id).map_err(err)
}

#[tauri::command]
pub fn mkt_list_drawing_alerts(symbol: String) -> Result<Vec<ChartDrawingAlert>, String> {
    let conn = crate::open_default_db_connection().map_err(err)?;
    db::list_chart_drawing_alerts(&conn, &symbol).map_err(err)
}

#[tauri::command]
pub fn mkt_save_drawing_alert(alert: ChartDrawingAlert) -> Result<(), String> {
    if alert.symbol.trim().is_empty()
        || alert.drawing_id.trim().is_empty()
        || alert.message.trim().is_empty()
    {
        return Err("Alert symbol, drawing, and message are required".into());
    }
    let conn = crate::open_default_db_connection().map_err(err)?;
    db::save_chart_drawing_alert(&conn, &alert).map_err(err)
}

#[tauri::command]
pub fn mkt_update_drawing_alert(
    id: String,
    enabled: bool,
    last_evaluated_at: Option<i64>,
) -> Result<(), String> {
    let conn = crate::open_default_db_connection().map_err(err)?;
    db::update_chart_drawing_alert(&conn, &id, enabled, last_evaluated_at).map_err(err)
}

#[tauri::command]
pub fn mkt_delete_drawing_alert(id: String) -> Result<(), String> {
    let conn = crate::open_default_db_connection().map_err(err)?;
    db::delete_chart_drawing_alert(&conn, &id).map_err(err)
}

#[tauri::command]
pub fn mkt_delete_drawing_alerts_for_drawings(
    symbol: String,
    drawing_ids: Vec<String>,
) -> Result<usize, String> {
    let mut conn = crate::open_default_db_connection().map_err(err)?;
    db::delete_chart_drawing_alerts_for_drawings(&mut conn, &symbol, &drawing_ids).map_err(err)
}

/// v6.4.18: diagnostic — return pivots exactly as the detection engine sees them.
/// Threshold is constant S0 (0.100%) to match detect_boxes_sync.
#[tauri::command]
pub fn mkt_list_pivots(
    symbol: String,
    m: u8,
) -> Result<Vec<crate::market::model::PivotDto>, String> {
    let _ = m; // kept for back-compat; base_tf is chosen dynamically.
    let conn = crate::open_default_db_connection().map_err(err)?;

    let tf_user = crate::market::mscale::m_key(m as i32);
    let candles_user = db::range_candles(&conn, &symbol, &tf_user, 0, i64::MAX, 500_000)
        .map_err(|e| format!("range_candles (user): {e}"))?;
    if candles_user.is_empty() {
        return Ok(Vec::new());
    }
    let from_ts: i64 = candles_user.first().map(|c| c.ts).unwrap_or(0);
    let to_ts: i64 = candles_user.last().map(|c| c.ts).unwrap_or(i64::MAX);
    let window_sec: i64 = to_ts.saturating_sub(from_ts).max(1);

    let base_tf: &str = pick_base_tf(window_sec);

    if std::env::var("VORA_DEBUG_PIVOTS").is_ok() {
        eprintln!(
            "[list_pivots] base_tf={} window_sec={}",
            base_tf, window_sec
        );
    }

    let candles = db::range_candles(&conn, &symbol, base_tf, from_ts, to_ts, 5_000_000)
        .map_err(|e| format!("range_candles (base): {e}"))?;
    if candles.is_empty() {
        return Ok(Vec::new());
    }

    let threshold_pct = crate::market::boxes::scale_start_pct(0);
    let pivots = crate::market::pivot_zigzag::detect_pivots_zigzag(&candles, threshold_pct);

    if std::env::var("VORA_DEBUG_PIVOTS").is_ok() {
        eprintln!("[list_pivots] pivots.len()={}", pivots.len());
    }

    let mut out: Vec<crate::market::model::PivotDto> = Vec::with_capacity(pivots.len());
    for p in pivots {
        let ts = match candles.get(p.idx) {
            Some(c) => c.ts,
            None => continue,
        };
        out.push(crate::market::model::PivotDto {
            idx: p.idx,
            ts,
            price: p.price,
            kind: match p.kind {
                crate::market::boxes::PivotKind::High => "H".to_string(),
                crate::market::boxes::PivotKind::Low => "L".to_string(),
            },
        });
    }
    Ok(out)
}

// ── Tests ─────────────────────────────────────────────────
