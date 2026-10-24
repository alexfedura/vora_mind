use rusqlite::{params, Connection, Result};
use std::time::{SystemTime, UNIX_EPOCH};

const TEST_SCHEMA: &str = r#"
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
    error_payload TEXT,
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    finished_at INTEGER
);

CREATE TABLE IF NOT EXISTS worker_heartbeats (
    worker_id TEXT PRIMARY KEY,
    last_seen INTEGER NOT NULL,
    current_task_id TEXT,
    status TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_worker_tasks_status ON worker_tasks(status, priority DESC, created_at ASC);
"#;

fn init_test_db() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(TEST_SCHEMA)?;
    Ok(conn)
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[test]
fn test_w01_enqueue_and_priority_ordering() -> Result<()> {
    let conn = init_test_db()?;

    // ثبت دو تسک با اولویت‌های متفاوت
    conn.execute(
        "INSERT INTO worker_tasks (task_id, project_root, intent, priority, status, created_at)
         VALUES (?1, ?2, ?3, ?4, 'PENDING', ?5)",
        params!["task_low", ".", "Low Priority Intent", 5, now_epoch()],
    )?;

    conn.execute(
        "INSERT INTO worker_tasks (task_id, project_root, intent, priority, status, created_at)
         VALUES (?1, ?2, ?3, ?4, 'PENDING', ?5)",
        params![
            "task_high",
            ".",
            "High Priority Intent",
            20,
            now_epoch() + 1
        ],
    )?;

    // تسک با اولویت بالاتر باید اول برداشته شود
    let next_task: String = conn.query_row(
        "SELECT task_id FROM worker_tasks WHERE status = 'PENDING' ORDER BY priority DESC, created_at ASC LIMIT 1",
        [],
        |r| r.get(0),
    )?;

    assert_eq!(next_task, "task_high");
    Ok(())
}

#[test]
fn test_w02_atomic_claim_transition() -> Result<()> {
    let mut conn = init_test_db()?;
    let t_id = "task_claim_test";

    conn.execute(
        "INSERT INTO worker_tasks (task_id, project_root, intent, priority, status, created_at)
         VALUES (?1, ?2, ?3, 10, 'PENDING', ?4)",
        params![t_id, ".", "Claim Verification", now_epoch()],
    )?;

    // کلیم کردن تسک در تراکنش
    let tx = conn.transaction()?;
    let claimed_id: String = tx.query_row(
        "SELECT task_id FROM worker_tasks WHERE status = 'PENDING' LIMIT 1",
        [],
        |r| r.get(0),
    )?;

    tx.execute(
        "UPDATE worker_tasks SET status = 'CLAIMED', started_at = ?1 WHERE task_id = ?2",
        params![now_epoch(), claimed_id],
    )?;
    tx.commit()?;

    // وضعیت باید CLAIMED شده باشد و در صف PENDING دیده نشود
    let pending_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM worker_tasks WHERE status = 'PENDING'",
        [],
        |r| r.get(0),
    )?;
    assert_eq!(pending_count, 0);

    let status: String = conn.query_row(
        "SELECT status FROM worker_tasks WHERE task_id = ?1",
        params![t_id],
        |r| r.get(0),
    )?;
    assert_eq!(status, "CLAIMED");

    Ok(())
}

#[test]
fn test_w03_crash_recovery_resets_interrupted_tasks() -> Result<()> {
    let conn = init_test_db()?;

    // شبیه‌سازی وضعیت تسک‌هایی که وسط پردازش ورکر کرش کرده است
    conn.execute(
        "INSERT INTO worker_tasks (task_id, project_root, intent, status, created_at)
         VALUES ('t_claimed', '.', 'Intent 1', 'CLAIMED', ?1),
                ('t_planning', '.', 'Intent 2', 'PLANNING', ?1),
                ('t_simulating', '.', 'Intent 3', 'SIMULATING', ?1),
                ('t_testing', '.', 'Intent 4', 'TESTING', ?1),
                ('t_completed', '.', 'Intent 5', 'COMPLETED', ?1)",
        params![now_epoch()],
    )?;

    // اجرای تابع بازیابی
    let recovered_count = conn.execute(
        "UPDATE worker_tasks 
         SET status = 'PENDING', error_payload = 'Recovered from unexpected worker restart/crash' 
         WHERE status IN ('CLAIMED', 'PLANNING', 'SIMULATING', 'TESTING')",
        [],
    )?;

    assert_eq!(recovered_count, 4);

    let pending_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM worker_tasks WHERE status = 'PENDING'",
        [],
        |r| r.get(0),
    )?;
    assert_eq!(pending_count, 4);

    let completed_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM worker_tasks WHERE status = 'COMPLETED'",
        [],
        |r| r.get(0),
    )?;
    assert_eq!(completed_count, 1);

    Ok(())
}

#[test]
fn test_w04_heartbeat_upsert_and_telemetry() -> Result<()> {
    let conn = init_test_db()?;
    let worker_id = "worker-test-node";

    // ثبت اولین ضربان قلب
    conn.execute(
        "INSERT INTO worker_heartbeats (worker_id, last_seen, current_task_id, status)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(worker_id) DO UPDATE SET last_seen = ?2, current_task_id = ?3, status = ?4",
        params![worker_id, now_epoch(), Option::<String>::None, "IDLE"],
    )?;

    // به‌روزرسانی هنگام مشغول شدن
    conn.execute(
        "INSERT INTO worker_heartbeats (worker_id, last_seen, current_task_id, status)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(worker_id) DO UPDATE SET last_seen = ?2, current_task_id = ?3, status = ?4",
        params![worker_id, now_epoch() + 10, Some("task_123"), "BUSY"],
    )?;

    let (task_id, status): (Option<String>, String) = conn.query_row(
        "SELECT current_task_id, status FROM worker_heartbeats WHERE worker_id = ?1",
        params![worker_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    assert_eq!(task_id, Some("task_123".to_string()));
    assert_eq!(status, "BUSY");

    Ok(())
}
