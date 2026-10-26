use rusqlite::{params, Connection, Result, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskFlowItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub position: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

const TASK_COLUMNS: &str = "id, title, description, completed, position, created_at, updated_at";

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_flow_items (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 200),
            description TEXT NOT NULL DEFAULT '',
            completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0, 1)),
            position INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_task_flow_items_position
            ON task_flow_items(position, created_at, id);",
    )?;
    let has_description = {
        let mut statement = conn.prepare("PRAGMA table_info(task_flow_items)")?;
        let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
        columns
            .collect::<Result<Vec<_>>>()?
            .iter()
            .any(|name| name == "description")
    };
    if !has_description {
        conn.execute(
            "ALTER TABLE task_flow_items ADD COLUMN description TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    Ok(())
}

pub fn list_tasks(conn: &Connection) -> Result<Vec<TaskFlowItem>> {
    let sql =
        format!("SELECT {TASK_COLUMNS} FROM task_flow_items ORDER BY position, created_at, id");
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map([], map_task)?;
    rows.collect()
}

pub fn create_task(conn: &mut Connection, id: &str, title: &str, now: i64) -> Result<TaskFlowItem> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let transaction = conn.transaction()?;
    let position: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(position) + 1, 0) FROM task_flow_items",
        [],
        |row| row.get(0),
    )?;
    transaction.execute(
        "INSERT INTO task_flow_items (id, title, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, title, position, now],
    )?;
    let task = query_task(&transaction, id)?;
    transaction.commit()?;
    Ok(task)
}

pub fn update_task(
    conn: &Connection,
    id: &str,
    title: &str,
    description: &str,
    now: i64,
) -> Result<TaskFlowItem> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let changed = conn.execute(
        "UPDATE task_flow_items
         SET title = ?1, description = ?2, updated_at = ?3
         WHERE id = ?4",
        params![title, description, now, id],
    )?;
    if changed == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    query_task(conn, id)
}

pub fn set_completed(
    conn: &Connection,
    id: &str,
    completed: bool,
    now: i64,
) -> Result<TaskFlowItem> {
    let changed = conn.execute(
        "UPDATE task_flow_items SET completed = ?1, updated_at = ?2 WHERE id = ?3",
        params![completed, now, id],
    )?;
    if changed == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    query_task(conn, id)
}

pub fn reorder_tasks(conn: &mut Connection, ids: &[String], now: i64) -> Result<Vec<TaskFlowItem>> {
    let transaction = conn.transaction()?;
    let existing_ids = {
        let mut statement = transaction.prepare("SELECT id FROM task_flow_items")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<HashSet<_>>>()?
    };
    let ordered_ids: HashSet<&str> = ids.iter().map(String::as_str).collect();
    if ordered_ids.len() != ids.len()
        || existing_ids.len() != ids.len()
        || existing_ids
            .iter()
            .any(|id| !ordered_ids.contains(id.as_str()))
    {
        return Err(rusqlite::Error::InvalidQuery);
    }

    for (index, id) in ids.iter().enumerate() {
        let position = i64::try_from(index).map_err(|_| rusqlite::Error::InvalidQuery)?;
        transaction.execute(
            "UPDATE task_flow_items SET position = ?1, updated_at = ?2 WHERE id = ?3",
            params![position, now, id],
        )?;
    }
    let tasks = list_tasks_in_transaction(&transaction)?;
    transaction.commit()?;
    Ok(tasks)
}

pub fn delete_task(conn: &Connection, id: &str) -> Result<()> {
    let changed = conn.execute("DELETE FROM task_flow_items WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

fn query_task(conn: &Connection, id: &str) -> Result<TaskFlowItem> {
    let sql = format!("SELECT {TASK_COLUMNS} FROM task_flow_items WHERE id = ?1");
    conn.query_row(&sql, [id], map_task)
}

fn list_tasks_in_transaction(transaction: &Transaction<'_>) -> Result<Vec<TaskFlowItem>> {
    let sql =
        format!("SELECT {TASK_COLUMNS} FROM task_flow_items ORDER BY position, created_at, id");
    let mut statement = transaction.prepare(&sql)?;
    let rows = statement.query_map([], map_task)?;
    rows.collect()
}

fn map_task(row: &rusqlite::Row<'_>) -> Result<TaskFlowItem> {
    Ok(TaskFlowItem {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        completed: row.get(3)?,
        position: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        create_task, delete_task, init_schema, list_tasks, reorder_tasks, set_completed,
        update_task,
    };
    use rusqlite::Connection;

    #[test]
    fn task_creation_completion_deletion_and_order_are_persisted() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&connection).expect("initialize Task Flow schema");

        create_task(&mut connection, "task-a", "First task", 1).expect("create first task");
        create_task(&mut connection, "task-b", "Second task", 2).expect("create second task");
        update_task(
            &connection,
            "task-a",
            "Updated task",
            "Full task details",
            3,
        )
        .expect("update task details");
        set_completed(&connection, "task-a", true, 4).expect("complete first task");
        let reordered = reorder_tasks(
            &mut connection,
            &["task-b".to_string(), "task-a".to_string()],
            5,
        )
        .expect("reorder tasks");

        assert_eq!(
            reordered
                .iter()
                .map(|task| task.id.as_str())
                .collect::<Vec<_>>(),
            vec!["task-b", "task-a"]
        );
        assert!(reordered[1].completed);
        assert_eq!(reordered[1].title, "Updated task");
        assert_eq!(reordered[1].description, "Full task details");

        delete_task(&connection, "task-b").expect("delete second task");
        let remaining = list_tasks(&connection).expect("list remaining tasks");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "task-a");
    }

    #[test]
    fn reorder_rejects_missing_or_duplicate_task_ids() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&connection).expect("initialize Task Flow schema");
        create_task(&mut connection, "task-a", "First task", 1).expect("create task");

        assert!(reorder_tasks(&mut connection, &[], 2).is_err());
        assert!(reorder_tasks(
            &mut connection,
            &["task-a".to_string(), "task-a".to_string()],
            2,
        )
        .is_err());
    }

    #[test]
    fn schema_migration_adds_description_to_existing_tasks() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch(
                "CREATE TABLE task_flow_items (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    completed INTEGER NOT NULL DEFAULT 0,
                    position INTEGER NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                INSERT INTO task_flow_items (id, title, position, created_at, updated_at)
                VALUES ('legacy-task', 'Legacy task', 0, 1, 1);",
            )
            .expect("create legacy Task Flow schema");

        init_schema(&connection).expect("migrate Task Flow schema");
        let tasks = list_tasks(&connection).expect("list migrated tasks");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].description, "");
    }
}
