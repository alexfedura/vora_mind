use rusqlite::{params, Connection, Result};
use std::time::{SystemTime, UNIX_EPOCH};

const HEALING_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS worker_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL UNIQUE,
    project_root TEXT NOT NULL,
    intent TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 10,
    status TEXT NOT NULL DEFAULT 'PENDING',
    claimed_by TEXT,
    healing_attempts INTEGER NOT NULL DEFAULT 0,
    error_payload TEXT,
    created_at INTEGER NOT NULL
);
"#;

fn init_test_db() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(HEALING_SCHEMA)?;
    Ok(conn)
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[test]
fn test_healing_01_increment_attempts_transition_to_healing() -> Result<()> {
    let conn = init_test_db()?;
    let t_id = "task_heal_01";

    conn.execute(
        "INSERT INTO worker_tasks (task_id, project_root, intent, status, healing_attempts, created_at)
         VALUES (?1, '.', 'Self Healing Verification', 'TESTING', 0, ?2)",
        params![t_id, now_epoch()],
    )?;

    // شبیه‌سازی رخ دادن ارور و افزایش مرحله ترمیم
    conn.execute(
        "UPDATE worker_tasks SET healing_attempts = healing_attempts + 1, status = 'HEALING' WHERE task_id = ?1",
        params![t_id],
    )?;

    let (status, attempts): (String, i32) = conn.query_row(
        "SELECT status, healing_attempts FROM worker_tasks WHERE task_id = ?1",
        params![t_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    assert_eq!(status, "HEALING");
    assert_eq!(attempts, 1);
    Ok(())
}

#[test]
fn test_healing_02_max_attempts_transition_to_failed() -> Result<()> {
    let conn = init_test_db()?;
    let t_id = "task_heal_max";
    let max_attempts = 3;

    conn.execute(
        "INSERT INTO worker_tasks (task_id, project_root, intent, status, healing_attempts, created_at)
         VALUES (?1, '.', 'Max Healing Exceeded', 'HEALING', 3, ?2)",
        params![t_id, now_epoch()],
    )?;

    let attempts: i32 = conn.query_row(
        "SELECT healing_attempts FROM worker_tasks WHERE task_id = ?1",
        params![t_id],
        |r| r.get(0),
    )?;

    if attempts >= max_attempts {
        conn.execute(
            "UPDATE worker_tasks SET status = 'FAILED', error_payload = 'Exceeded maximum healing attempts' WHERE task_id = ?1",
            params![t_id],
        )?;
    }

    let status: String = conn.query_row(
        "SELECT status FROM worker_tasks WHERE task_id = ?1",
        params![t_id],
        |r| r.get(0),
    )?;

    assert_eq!(status, "FAILED");
    Ok(())
}
