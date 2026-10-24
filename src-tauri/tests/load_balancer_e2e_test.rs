use rusqlite::{params, Connection, Result};
use std::time::{SystemTime, UNIX_EPOCH};

const LB_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS worker_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL UNIQUE,
    project_root TEXT NOT NULL,
    intent TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 10,
    status TEXT NOT NULL DEFAULT 'PENDING',
    claimed_by TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS worker_heartbeats (
    worker_id TEXT PRIMARY KEY,
    last_seen INTEGER NOT NULL,
    active_tasks INTEGER NOT NULL DEFAULT 0,
    max_concurrency INTEGER NOT NULL DEFAULT 2,
    status TEXT NOT NULL
);
"#;

fn init_test_db() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(LB_SCHEMA)?;
    Ok(conn)
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[test]
fn test_lb_01_least_loaded_node_selection() -> Result<()> {
    let conn = init_test_db()?;
    let now = now_epoch();

    // ثبت دو نود فعال با بارهای کاری مختلف
    conn.execute(
        "INSERT INTO worker_heartbeats (worker_id, last_seen, active_tasks, max_concurrency, status)
         VALUES ('node-busy', ?1, 2, 2, 'BUSY'),
                ('node-idle', ?1, 0, 2, 'IDLE')",
        params![now],
    )?;

    // کوئری پیدا کردن خلوت‌ترین نود زنده
    let selected_node: String = conn.query_row(
        "SELECT worker_id FROM worker_heartbeats 
         WHERE ?1 - last_seen <= 10 AND active_tasks < max_concurrency 
         ORDER BY (active_tasks * 1.0 / max_concurrency) ASC, last_seen DESC 
         LIMIT 1",
        params![now],
        |r| r.get(0),
    )?;

    assert_eq!(selected_node, "node-idle");
    Ok(())
}

#[test]
fn test_lb_02_concurrency_saturation_rejection() -> Result<()> {
    let conn = init_test_db()?;
    let now = now_epoch();

    // تمام نودها به سقف ظرفیت رسیده‌اند
    conn.execute(
        "INSERT INTO worker_heartbeats (worker_id, last_seen, active_tasks, max_concurrency, status)
         VALUES ('node-1', ?1, 2, 2, 'BUSY'),
                ('node-2', ?1, 4, 4, 'BUSY')",
        params![now],
    )?;

    let available_node: Option<String> = conn
        .query_row(
            "SELECT worker_id FROM worker_heartbeats 
         WHERE ?1 - last_seen <= 10 AND active_tasks < max_concurrency 
         LIMIT 1",
            params![now],
            |r| r.get(0),
        )
        .ok();

    assert_eq!(available_node, None);
    Ok(())
}
