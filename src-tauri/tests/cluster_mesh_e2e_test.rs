use rusqlite::{params, Connection, Result};
use std::time::{SystemTime, UNIX_EPOCH};

const CLUSTER_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS worker_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL UNIQUE,
    project_root TEXT NOT NULL,
    intent TEXT NOT NULL,
    target_file TEXT,
    target_symbol TEXT,
    patch_code TEXT,
    priority INTEGER NOT NULL DEFAULT 10,
    status TEXT NOT NULL DEFAULT 'PENDING',
    claimed_by TEXT,
    error_payload TEXT,
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    finished_at INTEGER
);

CREATE TABLE IF NOT EXISTS worker_heartbeats (
    worker_id TEXT PRIMARY KEY,
    last_seen INTEGER NOT NULL,
    current_task_id TEXT,
    status TEXT NOT NULL,
    version TEXT NOT NULL DEFAULT '0.51.0',
    port INTEGER NOT NULL DEFAULT 4040
);

CREATE INDEX IF NOT EXISTS idx_worker_tasks_status ON worker_tasks(status, priority DESC, created_at ASC);
"#;

fn init_test_db() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(CLUSTER_SCHEMA)?;
    Ok(conn)
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[test]
fn test_mesh_01_multi_node_heartbeat_registration() -> Result<()> {
    let conn = init_test_db()?;

    // ثبت دو نود در کلاستر
    conn.execute(
        "INSERT INTO worker_heartbeats (worker_id, last_seen, current_task_id, status, version, port)
         VALUES ('node-alpha', ?1, NULL, 'IDLE', '0.51.0', 4040),
                ('node-beta', ?1, NULL, 'BUSY', '0.51.0', 4041)",
        params![now_epoch()],
    )?;

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM worker_heartbeats", [], |r| r.get(0))?;
    assert_eq!(count, 2);

    let (status_beta, port_beta): (String, u16) = conn.query_row(
        "SELECT status, port FROM worker_heartbeats WHERE worker_id = 'node-beta'",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    assert_eq!(status_beta, "BUSY");
    assert_eq!(port_beta, 4041);

    Ok(())
}

#[test]
fn test_mesh_02_task_claiming_with_node_ownership() -> Result<()> {
    let mut conn = init_test_db()?;
    let t_id = "task_mesh_01";
    let node_id = "node-alpha";

    conn.execute(
        "INSERT INTO worker_tasks (task_id, project_root, intent, priority, status, created_at)
         VALUES (?1, '.', 'Cluster Mesh Claim Verification', 10, 'PENDING', ?2)",
        params![t_id, now_epoch()],
    )?;

    // کلیم کردن توسط node-alpha
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE worker_tasks SET status = 'CLAIMED', claimed_by = ?1, started_at = ?2 WHERE task_id = ?3",
        params![node_id, now_epoch(), t_id],
    )?;
    tx.commit()?;

    let (status, claimed_by): (String, Option<String>) = conn.query_row(
        "SELECT status, claimed_by FROM worker_tasks WHERE task_id = ?1",
        params![t_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    assert_eq!(status, "CLAIMED");
    assert_eq!(claimed_by, Some("node-alpha".to_string()));

    Ok(())
}

#[test]
fn test_mesh_03_stale_node_task_auto_recovery() -> Result<()> {
    let conn = init_test_db()?;
    let now = now_epoch();

    // ثبت یک تسک که توسط نودی ۴۰ ثانیه پیش کلیم شده و نود غیرفعال شده است
    conn.execute(
        "INSERT INTO worker_tasks (task_id, project_root, intent, status, claimed_by, started_at, created_at)
         VALUES ('task_dead_node', '.', 'Dead Node Task', 'CLAIMED', 'node-dead', ?1, ?1)",
        params![now - 40],
    )?;

    // اجرای تابع بازیابی
    let recovered = conn.execute(
        "UPDATE worker_tasks 
         SET status = 'PENDING', claimed_by = NULL, error_payload = 'Recovered from dead node' 
         WHERE status IN ('CLAIMED', 'PLANNING', 'SIMULATING', 'TESTING')
           AND (started_at IS NULL OR ?1 - started_at > 30)",
        params![now],
    )?;

    assert_eq!(recovered, 1);

    let (status, claimed_by): (String, Option<String>) = conn.query_row(
        "SELECT status, claimed_by FROM worker_tasks WHERE task_id = 'task_dead_node'",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    assert_eq!(status, "PENDING");
    assert_eq!(claimed_by, None);

    Ok(())
}
