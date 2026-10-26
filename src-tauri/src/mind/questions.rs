use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuestionItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub created_at: i64,
    pub updated_at: i64,
}

const QUESTION_COLUMNS: &str = "id, title, description, created_at, updated_at";

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS questions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 200),
            description TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_questions_updated_at
            ON questions(updated_at DESC, created_at DESC, id);",
    )
}

pub fn list_questions(conn: &Connection) -> Result<Vec<QuestionItem>> {
    let sql = format!(
        "SELECT {QUESTION_COLUMNS} FROM questions ORDER BY updated_at DESC, created_at DESC, id"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map([], map_question)?;
    rows.collect()
}

pub fn create_question(
    conn: &Connection,
    id: &str,
    title: &str,
    description: &str,
    now: i64,
) -> Result<QuestionItem> {
    let title = validate_title(title)?;
    conn.execute(
        "INSERT INTO questions (id, title, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, title, description, now],
    )?;
    query_question(conn, id)
}

pub fn save_question(
    conn: &Connection,
    id: &str,
    title: &str,
    description: &str,
    now: i64,
) -> Result<QuestionItem> {
    let title = validate_title(title)?;
    let changed = conn.execute(
        "UPDATE questions SET title = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
        params![title, description, now, id],
    )?;
    if changed == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    query_question(conn, id)
}

pub fn delete_question(conn: &Connection, id: &str) -> Result<()> {
    let changed = conn.execute("DELETE FROM questions WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

fn validate_title(title: &str) -> Result<&str> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(title)
}

fn query_question(conn: &Connection, id: &str) -> Result<QuestionItem> {
    let sql = format!("SELECT {QUESTION_COLUMNS} FROM questions WHERE id = ?1");
    conn.query_row(&sql, [id], map_question)
}

fn map_question(row: &rusqlite::Row<'_>) -> Result<QuestionItem> {
    Ok(QuestionItem {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        create_question, delete_question, init_schema, list_questions, save_question,
    };
    use rusqlite::Connection;

    #[test]
    fn questions_support_create_update_list_and_delete() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&connection).expect("initialize Questions Board schema");

        let long_description = "How does this work? ".repeat(10_000);
        let created = create_question(
            &connection,
            "question-1",
            "  A question  ",
            &long_description,
            1,
        )
        .expect("create question");
        create_question(&connection, "question-2", "Second question", "", 2)
            .expect("create empty-description question");

        assert_eq!(created.title, "A question");
        assert_eq!(created.description, long_description);
        assert_eq!(created.created_at, 1);
        assert_eq!(created.updated_at, 1);

        let updated = save_question(&connection, "question-1", "Updated", "Updated details", 3)
            .expect("update question");
        assert_eq!(updated.title, "Updated");
        assert_eq!(updated.description, "Updated details");
        assert_eq!(updated.created_at, 1);
        assert_eq!(updated.updated_at, 3);

        let questions = list_questions(&connection).expect("list questions");
        assert_eq!(questions.len(), 2);
        assert_eq!(questions[0], updated);

        delete_question(&connection, "question-1").expect("delete question");
        assert_eq!(
            list_questions(&connection).expect("list remaining questions").len(),
            1
        );
        assert!(delete_question(&connection, "question-1").is_err());
    }

    #[test]
    fn questions_reject_invalid_titles_and_missing_updates() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&connection).expect("initialize Questions Board schema");

        assert!(create_question(&connection, "empty", "  ", "", 1).is_err());
        assert!(create_question(&connection, "long", &"x".repeat(201), "", 1).is_err());
        assert!(save_question(&connection, "missing", "Valid", "", 1).is_err());
    }
}
