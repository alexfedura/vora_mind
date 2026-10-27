use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuestionItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub impact: Option<i64>,
    pub feasibility: Option<i64>,
    pub urgency: Option<i64>,
    pub priority_score: Option<i64>,
    pub triage_notes: Option<String>,
    pub triaged_at: Option<i64>,
    pub is_selected_for_promotion: bool,
    pub promoted_to_hypothesis_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

const QUESTION_COLUMNS: &str = "id, title, description, status, impact, feasibility, urgency, \
    priority_score, triage_notes, triaged_at, is_selected_for_promotion, \
    promoted_to_hypothesis_id, created_at, updated_at";

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
    )?;
    let columns = {
        let mut statement = conn.prepare("PRAGMA table_info(questions)")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
        rows.collect::<Result<Vec<_>>>()?
    };
    for (column, alter_sql) in [
        (
            "status",
            "ALTER TABLE questions ADD COLUMN status TEXT NOT NULL DEFAULT 'parked' CHECK(status IN ('parked','triaged','archived','promoted'))",
        ),
        (
            "impact",
            "ALTER TABLE questions ADD COLUMN impact INTEGER NULL",
        ),
        (
            "feasibility",
            "ALTER TABLE questions ADD COLUMN feasibility INTEGER NULL",
        ),
        (
            "urgency",
            "ALTER TABLE questions ADD COLUMN urgency INTEGER NULL",
        ),
        (
            "priority_score",
            "ALTER TABLE questions ADD COLUMN priority_score INTEGER NULL",
        ),
        (
            "triage_notes",
            "ALTER TABLE questions ADD COLUMN triage_notes TEXT NULL",
        ),
        (
            "triaged_at",
            "ALTER TABLE questions ADD COLUMN triaged_at INTEGER NULL",
        ),
        (
            "is_selected_for_promotion",
            "ALTER TABLE questions ADD COLUMN is_selected_for_promotion INTEGER NOT NULL DEFAULT 0 CHECK(is_selected_for_promotion IN (0,1))",
        ),
        (
            "promoted_to_hypothesis_id",
            "ALTER TABLE questions ADD COLUMN promoted_to_hypothesis_id TEXT NULL",
        ),
    ] {
        if !columns.iter().any(|existing| existing == column) {
            conn.execute(alter_sql, [])?;
        }
    }
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_questions_status_score
            ON questions(status, priority_score DESC, updated_at DESC);",
    )?;
    Ok(())
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

pub fn triage_question(
    conn: &Connection,
    id: &str,
    impact: i64,
    feasibility: i64,
    urgency: i64,
    triage_notes: Option<&str>,
    now: i64,
) -> Result<QuestionItem> {
    if !(1..=5).contains(&impact) || !(1..=5).contains(&feasibility) || !(1..=5).contains(&urgency)
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let status: String =
        conn.query_row("SELECT status FROM questions WHERE id = ?1", [id], |row| {
            row.get(0)
        })?;
    if status != "parked" && status != "triaged" {
        return Err(rusqlite::Error::InvalidQuery);
    }
    conn.execute(
        "UPDATE questions
         SET status = 'triaged', impact = ?1, feasibility = ?2, urgency = ?3,
             priority_score = ?4, triage_notes = ?5, triaged_at = ?6, updated_at = ?6
         WHERE id = ?7",
        params![
            impact,
            feasibility,
            urgency,
            impact * feasibility * urgency,
            triage_notes,
            now,
            id
        ],
    )?;
    query_question(conn, id)
}

pub fn untriage_question(conn: &Connection, id: &str, now: i64) -> Result<QuestionItem> {
    let changed = conn.execute(
        "UPDATE questions
         SET status = 'parked', impact = NULL, feasibility = NULL, urgency = NULL,
             priority_score = NULL, triage_notes = NULL, triaged_at = NULL,
             is_selected_for_promotion = 0, updated_at = ?1
         WHERE id = ?2 AND status = 'triaged'",
        params![now, id],
    )?;
    if changed == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    query_question(conn, id)
}

pub fn archive_question(conn: &Connection, id: &str, now: i64) -> Result<QuestionItem> {
    let changed = conn.execute(
        "UPDATE questions
         SET status = 'archived', impact = NULL, feasibility = NULL, urgency = NULL,
             priority_score = NULL, triage_notes = NULL, triaged_at = NULL,
             is_selected_for_promotion = 0, updated_at = ?1
         WHERE id = ?2 AND status IN ('parked', 'triaged')",
        params![now, id],
    )?;
    if changed == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    query_question(conn, id)
}

pub fn select_for_promotion(conn: &mut Connection, id: &str, now: i64) -> Result<QuestionItem> {
    let transaction = conn.transaction()?;
    let status: String =
        transaction.query_row("SELECT status FROM questions WHERE id = ?1", [id], |row| {
            row.get(0)
        })?;
    if status != "triaged" {
        return Err(rusqlite::Error::InvalidQuery);
    }
    transaction.execute(
        "UPDATE questions SET is_selected_for_promotion = 0 WHERE is_selected_for_promotion = 1",
        [],
    )?;
    transaction.execute(
        "UPDATE questions
         SET is_selected_for_promotion = 1, updated_at = ?1
         WHERE id = ?2 AND status = 'triaged'",
        params![now, id],
    )?;
    let question = query_question(&transaction, id)?;
    transaction.commit()?;
    Ok(question)
}

pub fn unselect_for_promotion(conn: &Connection, id: &str, now: i64) -> Result<QuestionItem> {
    let changed = conn.execute(
        "UPDATE questions
         SET is_selected_for_promotion = 0, updated_at = ?1
         WHERE id = ?2 AND is_selected_for_promotion = 1",
        params![now, id],
    )?;
    if changed == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    query_question(conn, id)
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
        status: row.get(3)?,
        impact: row.get(4)?,
        feasibility: row.get(5)?,
        urgency: row.get(6)?,
        priority_score: row.get(7)?,
        triage_notes: row.get(8)?,
        triaged_at: row.get(9)?,
        is_selected_for_promotion: row.get(10)?,
        promoted_to_hypothesis_id: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        archive_question, create_question, delete_question, init_schema, list_questions,
        query_question, save_question, select_for_promotion, triage_question, untriage_question,
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
            list_questions(&connection)
                .expect("list remaining questions")
                .len(),
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

    #[test]
    fn fresh_schema_has_all_columns_and_is_idempotent() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&connection).expect("initialize schema");
        init_schema(&connection).expect("initialize schema a second time");

        let columns = {
            let mut statement = connection
                .prepare("PRAGMA table_info(questions)")
                .expect("prepare table info");
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                })
                .expect("query table info")
                .collect::<rusqlite::Result<Vec<_>>>()
                .expect("collect table info")
        };
        println!("Final PRAGMA table_info(questions): {columns:?}");
        assert_eq!(
            columns
                .iter()
                .map(|column| column.1.as_str())
                .collect::<Vec<_>>(),
            vec![
                "id",
                "title",
                "description",
                "created_at",
                "updated_at",
                "status",
                "impact",
                "feasibility",
                "urgency",
                "priority_score",
                "triage_notes",
                "triaged_at",
                "is_selected_for_promotion",
                "promoted_to_hypothesis_id",
            ]
        );
        let question = create_question(&connection, "fresh", "Fresh", "", 10)
            .expect("create question with migration defaults");
        assert_eq!(question.status, "parked");
        assert!(!question.is_selected_for_promotion);
        assert_eq!(question.impact, None);
        assert_eq!(question.promoted_to_hypothesis_id, None);
    }

    #[test]
    fn existing_questions_receive_defaults_and_checked_constraints() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch(
                "CREATE TABLE questions (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    description TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                INSERT INTO questions VALUES ('legacy', 'Legacy', '', 1, 2);",
            )
            .expect("create old schema");

        init_schema(&connection).expect("migrate old schema");
        let question = query_question(&connection, "legacy").expect("read migrated question");
        assert_eq!(question.status, "parked");
        assert_eq!(question.is_selected_for_promotion, false);
        assert_eq!(question.impact, None);
        assert_eq!(question.promoted_to_hypothesis_id, None);

        assert!(connection
            .execute(
                "UPDATE questions SET status = 'invalid' WHERE id = 'legacy'",
                [],
            )
            .is_err());
        assert!(connection
            .execute(
                "UPDATE questions SET is_selected_for_promotion = 2 WHERE id = 'legacy'",
                [],
            )
            .is_err());
    }

    #[test]
    fn triage_sets_score_notes_and_timestamps() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&connection).expect("initialize schema");
        create_question(&connection, "triage", "Triage me", "", 1).expect("create question");

        let question = triage_question(&connection, "triage", 4, 3, 2, Some("Worth a look"), 9)
            .expect("triage question");
        assert_eq!(question.status, "triaged");
        assert_eq!(question.impact, Some(4));
        assert_eq!(question.feasibility, Some(3));
        assert_eq!(question.urgency, Some(2));
        assert_eq!(question.priority_score, Some(24));
        assert_eq!(question.triage_notes.as_deref(), Some("Worth a look"));
        assert_eq!(question.triaged_at, Some(9));
        assert_eq!(question.updated_at, 9);
        assert_eq!(question.promoted_to_hypothesis_id, None);
    }

    #[test]
    fn triage_rejects_out_of_range_values_and_archived_questions() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&connection).expect("initialize schema");
        create_question(&connection, "candidate", "Candidate", "", 1).expect("create candidate");

        for invalid in [0, 6] {
            assert!(triage_question(&connection, "candidate", invalid, 3, 3, None, 2).is_err());
            assert!(triage_question(&connection, "candidate", 3, invalid, 3, None, 2).is_err());
            assert!(triage_question(&connection, "candidate", 3, 3, invalid, None, 2).is_err());
        }
        archive_question(&connection, "candidate", 3).expect("archive candidate");
        assert!(triage_question(&connection, "candidate", 3, 3, 3, None, 4).is_err());
    }

    #[test]
    fn untriage_clears_triage_values_and_selection() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&connection).expect("initialize schema");
        create_question(&connection, "selected", "Selected", "", 1).expect("create question");
        triage_question(&connection, "selected", 3, 4, 5, Some("Notes"), 2)
            .expect("triage question");
        select_for_promotion(&mut connection, "selected", 3).expect("select question");

        let question = untriage_question(&connection, "selected", 4).expect("untriage question");
        assert_eq!(question.status, "parked");
        assert_eq!(question.impact, None);
        assert_eq!(question.feasibility, None);
        assert_eq!(question.urgency, None);
        assert_eq!(question.priority_score, None);
        assert_eq!(question.triage_notes, None);
        assert_eq!(question.triaged_at, None);
        assert!(!question.is_selected_for_promotion);
        assert!(untriage_question(&connection, "selected", 5).is_err());
    }

    #[test]
    fn archive_is_allowed_from_parked_or_triaged_and_is_terminal() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&connection).expect("initialize schema");
        create_question(&connection, "parked", "Parked", "", 1).expect("create parked");
        create_question(&connection, "triaged", "Triaged", "", 1).expect("create triaged");
        triage_question(&connection, "triaged", 2, 3, 4, Some("Notes"), 2)
            .expect("triage question");
        select_for_promotion(&mut connection, "triaged", 3).expect("select triaged question");

        let archived_parked =
            archive_question(&connection, "parked", 3).expect("archive parked question");
        let archived_triaged =
            archive_question(&connection, "triaged", 4).expect("archive selected triaged question");
        assert_eq!(archived_parked.status, "archived");
        assert_eq!(archived_triaged.status, "archived");
        assert_eq!(archived_triaged.priority_score, None);
        assert_eq!(archived_triaged.triaged_at, None);
        assert!(!archived_triaged.is_selected_for_promotion);
        assert!(archive_question(&connection, "parked", 5).is_err());
        assert!(triage_question(&connection, "triaged", 2, 3, 4, None, 5).is_err());
    }

    #[test]
    fn selection_requires_triage_and_replaces_previous_selection_atomically() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&connection).expect("initialize schema");
        create_question(&connection, "parked", "Parked", "", 1).expect("create parked");
        create_question(&connection, "first", "First", "", 1).expect("create first");
        create_question(&connection, "second", "Second", "", 1).expect("create second");
        assert!(select_for_promotion(&mut connection, "parked", 2).is_err());
        triage_question(&connection, "first", 2, 2, 2, None, 3).expect("triage first");
        triage_question(&connection, "second", 3, 3, 3, None, 3).expect("triage second");

        select_for_promotion(&mut connection, "first", 4).expect("select first");
        let second = select_for_promotion(&mut connection, "second", 5).expect("select second");
        assert!(second.is_selected_for_promotion);
        assert!(
            !query_question(&connection, "first")
                .expect("read first question")
                .is_selected_for_promotion
        );
        let selected_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM questions WHERE is_selected_for_promotion = 1",
                [],
                |row| row.get(0),
            )
            .expect("count selected questions");
        assert_eq!(selected_count, 1);
        assert!(super::unselect_for_promotion(&connection, "first", 6).is_err());
    }

    #[test]
    fn unselect_requires_current_selection_and_lifecycle_commands_never_promote() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        init_schema(&connection).expect("initialize schema");
        create_question(&connection, "one", "One", "", 1).expect("create question");
        triage_question(&connection, "one", 1, 2, 3, None, 2).expect("triage question");
        assert!(super::unselect_for_promotion(&connection, "one", 3).is_err());
        select_for_promotion(&mut connection, "one", 4).expect("select question");
        super::unselect_for_promotion(&connection, "one", 5).expect("unselect question");
        triage_question(&connection, "one", 2, 3, 4, None, 6).expect("update triage");
        untriage_question(&connection, "one", 7).expect("untriage question");
        archive_question(&connection, "one", 8).expect("archive question");
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM questions WHERE status = 'promoted'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .expect("count promoted questions"),
            0
        );
    }
}
