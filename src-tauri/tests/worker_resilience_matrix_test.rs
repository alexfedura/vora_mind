use rusqlite::{params, Connection, Result};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn init_test_queue_db(db_path: &str) -> Result<Connection> {
    if Path::new(db_path).exists() {
        let _ = fs::remove_file(db_path);
    }
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;

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
             healing_attempts INTEGER NOT NULL DEFAULT 0,
             last_error_stage TEXT,
             error_payload TEXT,
             created_at INTEGER NOT NULL,
             started_at INTEGER,
             finished_at INTEGER
         );

         CREATE TABLE IF NOT EXISTS cluster_file_locks (
             file_path TEXT PRIMARY KEY,
             locked_by_task TEXT NOT NULL,
             locked_by_node TEXT NOT NULL,
             locked_at INTEGER NOT NULL
         );",
    )?;
    Ok(conn)
}

#[test]
fn test_stale_task_recovery_mechanism() -> Result<()> {
    let db_path = "test_worker_stale_recovery.db";
    let conn = init_test_queue_db(db_path)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // ثبت یک تسک که ۴۰ ثانیه پیش Claim شده و نود آن کرش کرده است
    let past_epoch = now - 40;
    conn.execute(
        "INSERT INTO worker_tasks (task_id, project_root, intent, status, claimed_by, started_at, created_at)
         VALUES ('task_crashed_01', '.', 'Dead node test', 'SIMULATING', 'node-crashed', ?1, ?1)",
        params![past_epoch],
    )?;

    // ثبت یک قفل فایل رها شده
    conn.execute(
        "INSERT INTO cluster_file_locks (file_path, locked_by_task, locked_by_node, locked_at)
         VALUES ('src/lib.rs', 'task_crashed_01', 'node-crashed', ?1)",
        params![past_epoch],
    )?;

    // اجرای روتین بازیابی کرش
    let recovered_count = conn.execute(
        "UPDATE worker_tasks 
         SET status = 'PENDING', claimed_by = NULL, error_payload = 'Recovered from dead node or crash' 
         WHERE status IN ('CLAIMED', 'PLANNING', 'SIMULATING', 'TESTING', 'HEALING')
           AND (started_at IS NULL OR ?1 - started_at > 30)",
        params![now],
    )?;

    let released_locks = conn.execute(
        "DELETE FROM cluster_file_locks WHERE ?1 - locked_at > 30",
        params![now],
    )?;

    assert_eq!(
        recovered_count, 1,
        "Crashed task must be recovered to PENDING"
    );
    assert_eq!(released_locks, 1, "Stale lock must be released");

    let new_status: String = conn.query_row(
        "SELECT status FROM worker_tasks WHERE task_id = 'task_crashed_01'",
        [],
        |r| r.get(0),
    )?;
    assert_eq!(new_status, "PENDING");

    let lock_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cluster_file_locks WHERE file_path = 'src/lib.rs'",
        [],
        |r| r.get(0),
    )?;
    assert_eq!(lock_count, 0, "No orphaned locks should remain");

    let _ = fs::remove_file(db_path);
    Ok(())
}

#[test]
fn test_exclusive_lock_conflict_matrix() -> Result<()> {
    let db_path = "test_worker_lock_matrix.db";
    let conn = init_test_queue_db(db_path)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // نود ۱ فایل را قفل می‌کند
    let lock_1 = conn.execute(
        "INSERT INTO cluster_file_locks (file_path, locked_by_task, locked_by_node, locked_at)
         VALUES ('src/main.rs', 'task_node_1', 'node-01', ?1)",
        params![now],
    );
    assert!(lock_1.is_ok(), "Node 1 must acquire lock successfully");

    // تلاش نود ۲ برای قفل همان فایل باید با شکست مواجه شود
    let lock_2 = conn.execute(
        "INSERT INTO cluster_file_locks (file_path, locked_by_task, locked_by_node, locked_at)
         VALUES ('src/main.rs', 'task_node_2', 'node-02', ?1)",
        params![now],
    );
    assert!(
        lock_2.is_err(),
        "Duplicate lock on the same path must be rejected by Primary Key invariant"
    );

    let _ = fs::remove_file(db_path);
    Ok(())
}
