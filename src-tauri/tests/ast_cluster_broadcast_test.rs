use rusqlite::{params, Connection, Result};
use std::time::{SystemTime, UNIX_EPOCH};

const AST_EVENT_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS cluster_ast_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    source_node TEXT NOT NULL,
    file_path TEXT NOT NULL,
    symbol_id TEXT,
    action TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);
"#;

fn init_test_db() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(AST_EVENT_SCHEMA)?;
    Ok(conn)
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[test]
fn test_ast_broadcast_01_event_publishing() -> Result<()> {
    let conn = init_test_db()?;
    let now = now_epoch();

    conn.execute(
        "INSERT INTO cluster_ast_events (event_id, source_node, file_path, symbol_id, action, timestamp)
         VALUES ('evt_101', 'node-alpha', 'src/calc.rs', 'calc_tax', 'SURGICAL_PATCH', ?1)",
        params![now],
    )?;

    let (node, action): (String, String) = conn.query_row(
        "SELECT source_node, action FROM cluster_ast_events WHERE event_id = 'evt_101'",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    assert_eq!(node, "node-alpha");
    assert_eq!(action, "SURGICAL_PATCH");
    Ok(())
}

#[test]
fn test_ast_broadcast_02_recent_events_tail() -> Result<()> {
    let conn = init_test_db()?;
    let now = now_epoch();

    for i in 1..=5 {
        conn.execute(
            "INSERT INTO cluster_ast_events (event_id, source_node, file_path, symbol_id, action, timestamp)
             VALUES (?1, 'node-alpha', 'src/lib.rs', 'entry', 'MUTATION', ?2)",
            params![format!("evt_{}", i), now + i],
        )?;
    }

    let mut stmt =
        conn.prepare("SELECT event_id FROM cluster_ast_events ORDER BY timestamp DESC LIMIT 3")?;
    let event_ids: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .filter_map(Result::ok)
        .collect();

    assert_eq!(event_ids.len(), 3);
    assert_eq!(event_ids[0], "evt_5");
    assert_eq!(event_ids[1], "evt_4");
    assert_eq!(event_ids[2], "evt_3");

    Ok(())
}
