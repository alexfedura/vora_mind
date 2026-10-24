use rusqlite::{params, Connection, Result};
use std::time::{SystemTime, UNIX_EPOCH};

const DIAG_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS worker_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL UNIQUE,
    project_root TEXT NOT NULL,
    intent TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 10,
    status TEXT NOT NULL DEFAULT 'PENDING',
    claimed_by TEXT,
    healing_attempts INTEGER NOT NULL DEFAULT 0,
    last_error_stage TEXT,
    error_payload TEXT,
    created_at INTEGER NOT NULL
);
"#;

fn init_test_db() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(DIAG_SCHEMA)?;
    Ok(conn)
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[test]
fn test_diag_01_error_stage_recording() -> Result<()> {
    let conn = init_test_db()?;
    let t_id = "task_diag_01";

    conn.execute(
        "INSERT INTO worker_tasks (task_id, project_root, intent, status, healing_attempts, last_error_stage, error_payload, created_at)
         VALUES (?1, '.', 'Diagnostic Test', 'HEALING', 1, 'SYNTAX_INVALID', 'AST parser rejected token', ?2)",
        params![t_id, now_epoch()],
    )?;

    let (stage, payload): (Option<String>, Option<String>) = conn.query_row(
        "SELECT last_error_stage, error_payload FROM worker_tasks WHERE task_id = ?1",
        params![t_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    assert_eq!(stage, Some("SYNTAX_INVALID".to_string()));
    assert_eq!(payload, Some("AST parser rejected token".to_string()));
    Ok(())
}

#[test]
fn test_diag_02_diagnostics_retrieval_query() -> Result<()> {
    let conn = init_test_db()?;
    let t_id = "task_diag_02";

    conn.execute(
        "INSERT INTO worker_tasks (task_id, project_root, intent, status, healing_attempts, last_error_stage, error_payload, created_at)
         VALUES (?1, '.', 'Stage Recovery Query', 'COMPLETED', 2, 'CANDIDATE_TEST_FAILED', 'Resolved after healing', ?2)",
        params![t_id, now_epoch()],
    )?;

    let (status, attempts, stage): (String, i32, Option<String>) = conn.query_row(
        "SELECT status, healing_attempts, last_error_stage FROM worker_tasks WHERE task_id = ?1",
        params![t_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;

    assert_eq!(status, "COMPLETED");
    assert_eq!(attempts, 2);
    assert_eq!(stage, Some("CANDIDATE_TEST_FAILED".to_string()));
    Ok(())
}
