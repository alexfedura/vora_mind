use rusqlite::{params, Connection, Result};
use std::time::{SystemTime, UNIX_EPOCH};

const LOCK_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS cluster_file_locks (
    file_path TEXT PRIMARY KEY,
    locked_by_task TEXT NOT NULL,
    locked_by_node TEXT NOT NULL,
    locked_at INTEGER NOT NULL
);
"#;

fn init_test_db() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(LOCK_SCHEMA)?;
    Ok(conn)
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[test]
fn test_lock_01_exclusive_file_locking() -> Result<()> {
    let conn = init_test_db()?;
    let now = now_epoch();

    // قفل شدن فایل توسط نود اول
    conn.execute(
        "INSERT INTO cluster_file_locks (file_path, locked_by_task, locked_by_node, locked_at)
         VALUES ('src/main.rs', 'task_1', 'node-alpha', ?1)",
        params![now],
    )?;

    // تلاش نود دوم برای قفل کردن همان فایل با شکست مواجه می‌شود
    let conflict = conn.execute(
        "INSERT INTO cluster_file_locks (file_path, locked_by_task, locked_by_node, locked_at)
         VALUES ('src/main.rs', 'task_2', 'node-beta', ?1)",
        params![now],
    );

    assert!(conflict.is_err());
    Ok(())
}

#[test]
fn test_lock_02_lock_release_and_reacquire() -> Result<()> {
    let conn = init_test_db()?;
    let now = now_epoch();

    conn.execute(
        "INSERT INTO cluster_file_locks (file_path, locked_by_task, locked_by_node, locked_at)
         VALUES ('src/lib.rs', 'task_1', 'node-alpha', ?1)",
        params![now],
    )?;

    // آزادسازی قفل
    conn.execute(
        "DELETE FROM cluster_file_locks WHERE file_path = 'src/lib.rs' AND locked_by_task = 'task_1'",
        [],
    )?;

    // قفل شدن توسط نود دوم با موفقیت انجام می‌شود
    let acquired = conn.execute(
        "INSERT INTO cluster_file_locks (file_path, locked_by_task, locked_by_node, locked_at)
         VALUES ('src/lib.rs', 'task_2', 'node-beta', ?1)",
        params![now],
    )?;

    assert_eq!(acquired, 1);
    Ok(())
}
