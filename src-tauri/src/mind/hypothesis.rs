use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

use super::canvas_db::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Hypothesis {
    pub id: String,
    pub inquiry_id: String,
    pub statement_h1: String,
    pub statement_h0: String,
    pub independent_var: String,
    pub dependent_var: String,
    pub metric: String,
    pub success_threshold: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub evidence_notes: Option<String>,
    pub verdict: Option<String>,
    pub evaluated_at: Option<i64>,
}

const HYPOTHESIS_COLUMNS: &str = "id, inquiry_id, statement_h1, statement_h0, \
    independent_var, dependent_var, metric, success_threshold, status, created_at, updated_at, \
    evidence_notes, verdict, evaluated_at";

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS hypotheses (
            id TEXT PRIMARY KEY,
            inquiry_id TEXT NOT NULL,
            statement_h1 TEXT NOT NULL,
            statement_h0 TEXT NOT NULL,
            independent_var TEXT NOT NULL,
            dependent_var TEXT NOT NULL,
            metric TEXT NOT NULL,
            success_threshold TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'draft'
                CHECK(status IN ('draft', 'preregistered', 'testing', 'supported', 'rejected')),
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_hypotheses_inquiry ON hypotheses(inquiry_id);
        CREATE INDEX IF NOT EXISTS idx_hypotheses_status
            ON hypotheses(status, updated_at DESC);",
    )?;

    let columns = {
        let mut statement = conn.prepare("PRAGMA table_info(hypotheses)")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
        rows.collect::<Result<Vec<_>>>()?
    };
    for (name, definition) in [
        ("evidence_notes", "TEXT NULL"),
        ("verdict", "TEXT NULL"),
        ("evaluated_at", "INTEGER NULL"),
    ] {
        if !columns.iter().any(|column| column == name) {
            conn.execute_batch(&format!(
                "ALTER TABLE hypotheses ADD COLUMN {name} {definition}"
            ))?;
        }
    }
    Ok(())
}

pub fn promote_inquiry(
    conn: &mut Connection,
    id: &str,
    inquiry_id: &str,
    statement_h1: &str,
    statement_h0: &str,
    independent_var: &str,
    dependent_var: &str,
    metric: &str,
    success_threshold: &str,
    now: i64,
) -> Result<Hypothesis> {
    let transaction = conn.transaction()?;
    let inquiry_state = transaction
        .query_row(
            "SELECT status, is_selected_for_promotion FROM questions WHERE id = ?1",
            [inquiry_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?)),
        )
        .optional()?;
    if !matches!(inquiry_state, Some((ref status, true)) if status == "triaged") {
        return Err(rusqlite::Error::InvalidQuery);
    }

    transaction.execute(
        "INSERT INTO hypotheses (
            id, inquiry_id, statement_h1, statement_h0, independent_var,
            dependent_var, metric, success_threshold, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![
            id,
            inquiry_id,
            statement_h1,
            statement_h0,
            independent_var,
            dependent_var,
            metric,
            success_threshold,
            now
        ],
    )?;
    let changed = transaction.execute(
        "UPDATE questions
         SET status = 'promoted', promoted_to_hypothesis_id = ?1,
             is_selected_for_promotion = 0, updated_at = ?2
         WHERE id = ?3 AND status = 'triaged' AND is_selected_for_promotion = 1",
        params![id, now, inquiry_id],
    )?;
    if changed != 1 {
        return Err(rusqlite::Error::InvalidQuery);
    }

    let hypothesis = query_hypothesis(&transaction, id)?;
    transaction.commit()?;
    Ok(hypothesis)
}

pub fn list_hypotheses(conn: &Connection) -> Result<Vec<Hypothesis>> {
    let mut statement = conn.prepare(&format!(
        "SELECT {HYPOTHESIS_COLUMNS} FROM hypotheses ORDER BY created_at DESC"
    ))?;
    let rows = statement.query_map([], map_hypothesis)?;
    rows.collect()
}

#[tauri::command]
pub fn promote_to_hypothesis(
    state: State<AppState>,
    inquiry_id: String,
    statement_h1: String,
    statement_h0: String,
    independent_var: String,
    dependent_var: String,
    metric: String,
    success_threshold: String,
) -> Result<Hypothesis, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|error| error.to_string())?;
    let id = format!("hypothesis_{}", uuid::Uuid::now_v7());
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    promote_inquiry(
        &mut conn,
        &id,
        &inquiry_id,
        &statement_h1,
        &statement_h0,
        &independent_var,
        &dependent_var,
        &metric,
        &success_threshold,
        now,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_hypotheses(state: State<AppState>) -> Result<Vec<Hypothesis>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    list_hypotheses(&conn).map_err(|error| error.to_string())
}

fn record_verdict_in_db(
    conn: &Connection,
    id: &str,
    evidence_notes: &str,
    verdict: &str,
    now: i64,
) -> std::result::Result<Hypothesis, String> {
    if !matches!(verdict, "supported" | "rejected" | "inconclusive") {
        return Err(format!("Invalid hypothesis verdict: {verdict}"));
    }
    let changed = conn
        .execute(
            "UPDATE hypotheses
             SET evidence_notes = ?1, verdict = ?2, evaluated_at = ?3, updated_at = ?3
             WHERE id = ?4",
            params![evidence_notes, verdict, now, id],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("Hypothesis not found: {id}"));
    }
    query_hypothesis(conn, id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn record_verdict(
    state: State<AppState>,
    id: String,
    evidence_notes: String,
    verdict: String,
) -> std::result::Result<Hypothesis, String> {
    if !matches!(verdict.as_str(), "supported" | "rejected" | "inconclusive") {
        return Err(format!("Invalid hypothesis verdict: {verdict}"));
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|error| error.to_string())?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    record_verdict_in_db(&conn, &id, &evidence_notes, &verdict, now)
}

fn query_hypothesis(conn: &Connection, id: &str) -> Result<Hypothesis> {
    let sql = format!("SELECT {HYPOTHESIS_COLUMNS} FROM hypotheses WHERE id = ?1");
    conn.query_row(&sql, [id], map_hypothesis)
}

fn map_hypothesis(row: &rusqlite::Row<'_>) -> Result<Hypothesis> {
    Ok(Hypothesis {
        id: row.get(0)?,
        inquiry_id: row.get(1)?,
        statement_h1: row.get(2)?,
        statement_h0: row.get(3)?,
        independent_var: row.get(4)?,
        dependent_var: row.get(5)?,
        metric: row.get(6)?,
        success_threshold: row.get(7)?,
        status: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        evidence_notes: row.get(11)?,
        verdict: row.get(12)?,
        evaluated_at: row.get(13)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{init_schema, list_hypotheses, promote_inquiry, record_verdict_in_db};
    use crate::mind::questions::{
        create_question, init_schema as init_questions_schema, select_for_promotion,
        triage_question,
    };
    use rusqlite::Connection;

    fn setup() -> Connection {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        init_questions_schema(&connection).expect("initialize questions schema");
        init_schema(&connection).expect("initialize hypotheses schema");
        init_schema(&connection).expect("reinitialize hypotheses schema");
        connection
    }

    fn promote(
        conn: &mut Connection,
        inquiry_id: &str,
        id: &str,
        now: i64,
    ) -> rusqlite::Result<super::Hypothesis> {
        promote_inquiry(
            conn,
            id,
            inquiry_id,
            "If X then Y",
            "X does not affect Y",
            "X",
            "Y",
            "Y per day",
            "Y increases by 10%",
            now,
        )
    }

    fn select_inquiry(conn: &mut Connection, inquiry_id: &str) {
        create_question(conn, inquiry_id, "A question", "", 1).expect("create inquiry");
        triage_question(conn, inquiry_id, 3, 3, 3, None, 2).expect("triage inquiry");
        select_for_promotion(conn, inquiry_id, 3).expect("select inquiry");
    }

    #[test]
    fn promotion_creates_hypothesis_and_updates_inquiry() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");

        let hypothesis =
            promote(&mut connection, "inquiry-1", "hypothesis-1", 10).expect("promote inquiry");
        assert_eq!(hypothesis.inquiry_id, "inquiry-1");
        assert_eq!(hypothesis.statement_h1, "If X then Y");
        assert_eq!(hypothesis.status, "draft");
        assert_eq!(hypothesis.created_at, 10);
        assert_eq!(hypothesis.updated_at, 10);

        let inquiry_status: String = connection
            .query_row(
                "SELECT status FROM questions WHERE id = 'inquiry-1'",
                [],
                |row| row.get(0),
            )
            .expect("read inquiry status");
        let promoted_id: Option<String> = connection
            .query_row(
                "SELECT promoted_to_hypothesis_id FROM questions WHERE id = 'inquiry-1'",
                [],
                |row| row.get(0),
            )
            .expect("read linked hypothesis ID");
        let selected: bool = connection
            .query_row(
                "SELECT is_selected_for_promotion FROM questions WHERE id = 'inquiry-1'",
                [],
                |row| row.get(0),
            )
            .expect("read promotion selection");
        assert_eq!(inquiry_status, "promoted");
        assert_eq!(promoted_id.as_deref(), Some("hypothesis-1"));
        assert!(!selected);
        assert!(promote(&mut connection, "inquiry-1", "hypothesis-2", 11).is_err());
        assert_eq!(
            list_hypotheses(&connection).expect("list hypotheses").len(),
            1
        );
    }

    #[test]
    fn promotion_rejects_an_inquiry_that_is_not_selected() {
        let mut connection = setup();
        create_question(&connection, "inquiry-1", "A question", "", 1).expect("create inquiry");
        triage_question(&connection, "inquiry-1", 3, 3, 3, None, 2).expect("triage inquiry");

        assert!(promote(&mut connection, "inquiry-1", "hypothesis-1", 10).is_err());
        assert!(list_hypotheses(&connection)
            .expect("list hypotheses")
            .is_empty());
        let status: String = connection
            .query_row(
                "SELECT status FROM questions WHERE id = 'inquiry-1'",
                [],
                |row| row.get(0),
            )
            .expect("read inquiry status");
        assert_eq!(status, "triaged");
    }

    #[test]
    fn promotion_rejects_a_missing_inquiry() {
        let mut connection = setup();

        assert!(promote(&mut connection, "missing", "hypothesis-1", 10).is_err());
        assert!(list_hypotheses(&connection)
            .expect("list hypotheses")
            .is_empty());
    }

    #[test]
    fn hypotheses_are_returned_newest_first() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        promote(&mut connection, "inquiry-1", "hypothesis-1", 10).expect("promote inquiry");
        select_inquiry(&mut connection, "inquiry-2");
        promote(&mut connection, "inquiry-2", "hypothesis-2", 20).expect("promote inquiry");
        select_inquiry(&mut connection, "inquiry-3");
        promote(&mut connection, "inquiry-3", "hypothesis-3", 15).expect("promote inquiry");

        let hypotheses = list_hypotheses(&connection).expect("list hypotheses");
        assert_eq!(
            hypotheses
                .iter()
                .map(|hypothesis| hypothesis.id.as_str())
                .collect::<Vec<_>>(),
            vec!["hypothesis-2", "hypothesis-3", "hypothesis-1"]
        );
    }

    #[test]
    fn record_verdict_saves_evidence_and_rejects_invalid_verdicts() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        promote(&mut connection, "inquiry-1", "hypothesis-1", 10).expect("promote inquiry");

        let hypothesis = record_verdict_in_db(
            &connection,
            "hypothesis-1",
            "Observed results matched H1",
            "supported",
            20,
        )
        .expect("record verdict");
        assert_eq!(
            hypothesis.evidence_notes.as_deref(),
            Some("Observed results matched H1")
        );
        assert_eq!(hypothesis.verdict.as_deref(), Some("supported"));
        assert_eq!(hypothesis.evaluated_at, Some(20));
        assert_eq!(hypothesis.updated_at, 20);

        assert!(record_verdict_in_db(
            &connection,
            "hypothesis-1",
            "Invalid verdict attempt",
            "maybe",
            30,
        )
        .is_err());
    }
}
