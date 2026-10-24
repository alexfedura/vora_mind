use rusqlite::{params, Connection, Result};
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS cluster_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL,
    task_id TEXT,
    level TEXT NOT NULL,
    message TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_cluster_logs_ts ON cluster_logs(timestamp DESC);
"#;

fn init_test_db() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(LOG_SCHEMA)?;
    Ok(conn)
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[test]
fn test_log_01_structured_log_ingestion() -> Result<()> {
    let conn = init_test_db()?;
    let now = now_epoch();

    conn.execute(
        "INSERT INTO cluster_logs (node_id, task_id, level, message, timestamp)
         VALUES ('node-1', 'task_100', 'INFO', 'Baseline test passed 100% green.', ?1)",
        params![now],
    )?;

    let (node, level, msg): (String, String, String) = conn.query_row(
        "SELECT node_id, level, message FROM cluster_logs WHERE task_id = 'task_100'",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;

    assert_eq!(node, "node-1");
    assert_eq!(level, "INFO");
    assert_eq!(msg, "Baseline test passed 100% green.");
    Ok(())
}

#[test]
fn test_log_02_recent_logs_ordering() -> Result<()> {
    let conn = init_test_db()?;
    let now = now_epoch();

    for i in 1..=5 {
        conn.execute(
            "INSERT INTO cluster_logs (node_id, task_id, level, message, timestamp)
             VALUES ('node-1', ?1, 'DEBUG', 'Telemetry cycle', ?2)",
            params![format!("task_{}", i), now + i],
        )?;
    }

    let mut stmt =
        conn.prepare("SELECT task_id FROM cluster_logs ORDER BY timestamp DESC LIMIT 3")?;
    let task_ids: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .filter_map(Result::ok)
        .collect();

    assert_eq!(task_ids.len(), 3);
    assert_eq!(task_ids[0], "task_5");
    assert_eq!(task_ids[1], "task_4");
    assert_eq!(task_ids[2], "task_3");
    Ok(())
}
